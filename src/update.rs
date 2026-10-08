//! Actualizaciones desde GitHub: al arrancar y cada hora mira si `origin/master` tiene commits
//! que este binario no tiene. Si los hay, Iris lo ofrece; al aceptar, baja el código, lo
//! instala con cargo y se vuelve a abrir en la misma sesión.
//!
//! Dos formas de instalación: desde el repo clonado (`cargo install --path .`, la de Kevin) o
//! directo de GitHub (`cargo install --git`). En la primera el código se trae con `git pull`.

use crate::AppEvent;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

const REPO: &str = "https://github.com/KEXNARV/jarvis";
/// El commit con que se compiló (lo pone `build.rs`).
const BUILT: &str = env!("IRIS_COMMIT");
const SRC: &str = env!("CARGO_MANIFEST_DIR");

pub enum UpdateEvent {
    Available(Info),
    Installed,
    Failed(String),
}

#[derive(Clone)]
pub struct Info {
    /// El commit nuevo, corto.
    pub commit: String,
    /// Títulos de los commits nuevos, del más reciente al más viejo (vacío si no se sabe).
    pub commits: Vec<String>,
}

/// Revisa en segundo plano: unos segundos después de arrancar y luego cada hora.
pub fn spawn(tx: Sender<AppEvent>) {
    if BUILT.is_empty() {
        return; // compilado sin git: no hay con qué comparar
    }
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        let mut offered = String::new();
        loop {
            if let Some(info) = check() {
                if info.commit != offered {
                    offered = info.commit.clone();
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
    git(&["remote", "get-url", "origin"]).is_ok_and(|url| url.trim_end_matches(".git").ends_with("KEXNARV/jarvis"))
}

fn check() -> Option<Info> {
    if is_repo() {
        git(&["fetch", "-q", "origin", "master"]).ok()?;
        let remote = git(&["rev-parse", "origin/master"]).ok()?;
        // Igual, o con cambios locales que GitHub todavía no tiene: no hay nada que ofrecer.
        if remote == BUILT || git(&["merge-base", "--is-ancestor", BUILT, &remote]).is_err() {
            return None;
        }
        let log = git(&["log", "--format=%s", &format!("{BUILT}..{remote}")]).ok()?;
        Some(Info { commit: remote[..7].into(), commits: log.lines().map(String::from).collect() })
    } else {
        let out = run(Command::new("git").args(["ls-remote", REPO, "refs/heads/master"])).ok()?;
        let remote = out.split_whitespace().next()?.to_string();
        (remote.len() >= 7 && remote != BUILT).then(|| Info { commit: remote[..7].into(), commits: vec![] })
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
    } else {
        run(Command::new(cargo()).args(["install", "--git", REPO, "--force"]))?;
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
