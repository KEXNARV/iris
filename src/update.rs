//! Actualizaciones desde GitHub: al arrancar y cada hora mira si `origin/master` tiene commits
//! que este binario no tiene. Si los hay, Jarvis lo ofrece; al aceptar, Claude mezcla master
//! (resolviendo conflictos), lo instala con cargo, y Jarvis se vuelve a abrir en la misma sesión.
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
const BUILT: &str = env!("JARVIS_COMMIT");
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
    let p = std::env::current_exe().unwrap_or_else(|_| "jarvis".into());
    match p.to_str().and_then(|s| s.strip_suffix(" (deleted)")) {
        Some(s) => s.into(),
        None => p,
    }
}

/// Clonado del repo de Jarvis (y no, por ejemplo, la copia temporal de `cargo install --git`).
fn is_repo() -> bool {
    git(&["remote", "get-url", "origin"]).is_ok_and(|url| url.trim_end_matches(".git").ends_with("KEXNARV/jarvis"))
}

fn check() -> Option<Info> {
    if is_repo() {
        git(&["fetch", "-q", "origin", "master"]).ok()?;
        let remote = git(&["rev-parse", "origin/master"]).ok()?;
        // Lo que GitHub tiene y este binario no. Los commits propios de la rama no cuentan: se
        // mezclan con master al actualizar.
        let range = format!("{BUILT}..{remote}");
        if git(&["rev-list", "--count", &range]).ok()? == "0" {
            return None;
        }
        let log = git(&["log", "--format=%s", &range]).ok()?;
        Some(Info { commit: remote[..7].into(), commits: log.lines().map(String::from).collect() })
    } else {
        let out = run(Command::new("git").args(["ls-remote", REPO, "refs/heads/master"])).ok()?;
        let remote = out.split_whitespace().next()?.to_string();
        (remote.len() >= 7 && remote != BUILT).then(|| Info { commit: remote[..7].into(), commits: vec![] })
    }
}

/// El encargo para Claude: que traiga master, resuelva lo que choque e instale. Solo con el
/// repo clonado (sin repo no hay nada que mezclar: se usa `install`).
pub fn prompt(info: &Info) -> Option<String> {
    if !is_repo() {
        return None;
    }
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    let commits = match info.commits.len() {
        0 => String::new(),
        _ => format!("\nLo nuevo:\n{}\n", info.commits.iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n")),
    };
    Some(format!(
        "Actualiza Jarvis (este programa) a la versión de GitHub ({commit}). El repo está en {SRC}, en la rama {branch}.\n{commits}\n\
         1. `git -C {SRC} fetch origin master` y mezcla origin/master en {branch} con un merge (sin rebase, sin cambiar de rama, sin push). Si hay cambios sin commit, guárdalos con stash y devuélvelos al final.\n\
         2. Si hay conflictos, resuélvelos conservando lo de los dos lados. Si para eso hay que decidir algo importante, para y explícamelo sin instalar.\n\
         3. Corre `{cargo} test` en el repo; tiene que pasar.\n\
         4. Instala con `{cargo} install --locked --path {SRC} --root {root}`.\n\
         5. Termina con un resumen corto de lo que entró. No reinicies Jarvis: se reabre solo cuando termines.",
        commit = info.commit,
        cargo = cargo().display(),
        root = root().display(),
    ))
}

/// Cuándo se instaló el binario; si cambia, hay versión nueva que abrir.
pub fn stamp() -> Option<std::time::SystemTime> {
    std::fs::metadata(exe()).and_then(|m| m.modified()).ok()
}

/// Donde está instalado (`~/.local` para `~/.local/bin/jarvis`): ahí va el nuevo, que es el
/// que se vuelve a abrir.
fn root() -> PathBuf {
    let exe = exe();
    exe.parent().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_else(|| ".".into())
}

fn install_now() -> Result<(), String> {
    if is_repo() {
        // En cualquier rama: master se mezcla con lo que tengas (o solo la adelanta, si no hay
        // commits propios). Con conflictos se deshace todo y queda como estaba.
        // --autostash: lo que estés editando en el repo se guarda y vuelve después.
        let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if let Err(e) = git(&["pull", "-q", "--no-rebase", "--no-edit", "--autostash", "origin", "master"]) {
            let _ = git(&["merge", "--abort"]);
            return Err(format!("master no se pudo mezclar solo con la rama {branch}: {e}"));
        }
        run(Command::new(cargo()).args(["install", "--path", SRC, "--root"]).arg(root()))?;
    } else {
        run(Command::new(cargo()).args(["install", "--git", REPO, "--force", "--root"]).arg(root()))?;
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
    use super::*;

    #[test]
    fn el_encargo_dice_donde_instalar() {
        let info = Info { commit: "abc1234".into(), commits: vec!["Algo nuevo".into()] };
        let Some(p) = prompt(&info) else { return }; // sin el repo clonado no hay encargo
        println!("{p}");
        assert!(p.contains("abc1234") && p.contains("- Algo nuevo"));
        assert!(p.contains(&format!("--root {}", root().display())));
        assert!(p.contains(SRC));
    }
}
