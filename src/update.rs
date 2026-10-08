//! Actualizaciones: al arrancar y cada hora mira la versión publicada en iris.knarvaez.com
//! (`version.json`). Si es más nueva que esta, Iris lo ofrece; al aceptar se instala y se vuelve
//! a abrir en la misma sesión.
//!
//! Dos formas de instalar la nueva: con el instalador de la página, en modo solo binario y en la
//! misma carpeta (lo normal), o desde el repo clonado (`git pull` + `cargo install --path .`, la
//! de Kevin, que compila lo suyo).

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
const SRC: &str = env!("CARGO_MANIFEST_DIR");

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

/// Clonado del repo de Iris (y no, por ejemplo, la copia temporal de `cargo install --git`).
fn is_repo() -> bool {
    // Los clones de antes del cambio de nombre siguen apuntando a KEXNARV/jarvis (GitHub redirige).
    git(&["remote", "get-url", "origin"]).is_ok_and(|url| {
        let url = url.trim().trim_end_matches(".git");
        url.ends_with("KEXNARV/iris") || url.ends_with("KEXNARV/jarvis")
    })
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
    if is_repo() {
        let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if branch != "master" {
            return Err(format!("el repo está en la rama {branch}, no en master"));
        }
        // --autostash: lo que estés editando en el repo se guarda y vuelve después.
        git(&["pull", "-q", "--ff-only", "--autostash", "origin", "master"])?;
        run(Command::new(cargo()).args(["install", "--path", SRC]))?;
        return Ok(());
    }
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

/// Lanzado desde Hyprland, el PATH no siempre trae ~/.cargo/bin.
fn cargo() -> PathBuf {
    let home = std::env::var("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        Path::new(&std::env::var("HOME").unwrap_or_default()).join(".cargo")
    });
    let p = home.join("bin/cargo");
    if p.exists() { p } else { "cargo".into() }
}

fn git(args: &[&str]) -> Result<String, String> {
    run(Command::new("git").arg("-C").arg(SRC).args(args))
}

/// Corre y devuelve la salida; si falla, la última línea del error.
fn run(cmd: &mut Command) -> Result<String, String> {
    // Sin terminal: si git quisiera pedir una contraseña, que falle en vez de quedarse esperando.
    let out = cmd.env("GIT_TERMINAL_PROMPT", "0").output().map_err(|e| e.to_string())?;
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
