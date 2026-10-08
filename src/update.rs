//! Actualizaciones: al arrancar y cada hora mira la versión publicada en iris.knarvaez.com
//! (`version.json`). Si es más nueva que esta, Iris lo ofrece; al aceptar se instala y se vuelve
//! a abrir en la misma sesión.
//!
//! La nueva se instala siempre con el instalador de la página, en modo solo binario y en la misma
//! carpeta: el mismo camino para todos, también para quien compiló Iris desde el repo.

use crate::AppEvent;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// Lo publicado: `{"version": "0.2.0", "notas": ["…", "…"]}`.
const VERSION_URL: &str = "https://iris.knarvaez.com/version.json";
const INSTALADOR_SH: &str = "https://iris.knarvaez.com/install.sh";
const INSTALADOR_PS1: &str = "https://iris.knarvaez.com/install.ps1";
/// Esta versión, la de Cargo.toml.
const ESTA: &str = env!("CARGO_PKG_VERSION");

pub enum UpdateEvent {
    Available(Info),
    Installed,
    Failed(String),
}

#[derive(Clone)]
pub struct Info {
    /// La versión publicada, `0.2.0`.
    pub version: String,
    /// Lo que trae, como lo escribió quien la publicó (puede venir vacío).
    pub notas: Vec<String>,
}

/// Revisa en segundo plano: unos segundos después de arrancar y luego cada hora.
pub fn spawn(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        let mut offered = String::new();
        loop {
            if let Some(info) = check() {
                if info.version != offered {
                    offered = info.version.clone();
                    if tx.send(AppEvent::Update(UpdateEvent::Available(info))).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_secs(3600));
        }
    });
}

/// Baja e instala la versión nueva; avisa con `Installed` o `Failed`.
pub fn install(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        let ev = match install_now() {
            Ok(()) => UpdateEvent::Installed,
            Err(e) => UpdateEvent::Failed(e),
        };
        let _ = tx.send(AppEvent::Update(ev));
    });
}

/// El binario recién instalado. El que corre ya fue reemplazado, y Linux lo muestra como
/// «… (deleted)».
pub fn exe() -> PathBuf {
    let p = std::env::current_exe().unwrap_or_else(|_| "iris".into());
    match p.to_str().and_then(|s| s.strip_suffix(" (deleted)")) {
        Some(s) => s.into(),
        None => p,
    }
}

fn check() -> Option<Info> {
    let v: serde_json::Value = serde_json::from_str(&run(Command::new("curl").args(["-fsSL", "--max-time", "15", VERSION_URL])).ok()?).ok()?;
    let version = v["version"].as_str()?.trim().trim_start_matches('v').to_string();
    if !mas_nueva(&version, ESTA) {
        return None;
    }
    let notas = v["notas"].as_array().map(|n| n.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    Some(Info { version, notas })
}

/// `a` es más nueva que `b` (`0.10.0` > `0.9.3`). Lo que no se entiende no es más nuevo.
fn mas_nueva(a: &str, b: &str) -> bool {
    let partes = |s: &str| s.split(['.', '-']).take(3).map(|p| p.parse::<u64>().ok()).collect::<Option<Vec<_>>>();
    match (partes(a), partes(b)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn install_now() -> Result<(), String> {
    // El instalador de la página, solo el binario y en la carpeta de este: sin dependencias del
    // sistema (pedirían sudo sin terminal), sin modelo de voz y sin Claude Code.
    let dir = exe().parent().map(Path::to_path_buf).ok_or("no sé dónde está instalado Iris")?;
    if cfg!(windows) {
        let script = format!("$env:IRIS_SOLO_BINARIO='1'; $env:IRIS_DESTINO='{}'; irm {INSTALADOR_PS1} | iex", dir.display());
        run(Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]))?;
    } else {
        let script = format!("curl -fsSL {INSTALADOR_SH} | sh -s -- --solo-binario");
        run(Command::new("sh").args(["-c", &script]).env("IRIS_DESTINO", &dir))?;
    }
    Ok(())
}

/// Corre y devuelve la salida; si falla, la última línea del error.
fn run(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    Err(err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("falló").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::mas_nueva;

    #[test]
    fn compara_versiones() {
        assert!(mas_nueva("0.2.0", "0.1.0"));
        assert!(mas_nueva("0.10.0", "0.9.3"));
        assert!(mas_nueva("1.0.0", "0.99.99"));
        assert!(!mas_nueva("0.1.0", "0.1.0"));
        assert!(!mas_nueva("0.1.0", "0.2.0"));
        assert!(!mas_nueva("pronto", "0.1.0"));
    }
}
