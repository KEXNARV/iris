//! Updates: at startup and every hour it checks the version published on iris.knarvaez.com
//! (`version.json`). If it's newer than this one, Iris offers it; on accepting, it's installed
//! and reopened in the same session.
//!
//! The new one is always installed with the website's installer, in binary-only mode and into
//! the same folder: the same path for everyone, including whoever built Iris from the repo.

use crate::AppEvent;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// What's published: `{"version": "0.2.0", "notas": ["…", "…"]}`.
const VERSION_URL: &str = "https://iris.knarvaez.com/version.json";
const INSTALLER_SH: &str = "https://iris.knarvaez.com/install.sh";
const INSTALLER_PS1: &str = "https://iris.knarvaez.com/install.ps1";
/// This version, the one in Cargo.toml.
const THIS_VERSION: &str = env!("CARGO_PKG_VERSION");

pub enum UpdateEvent {
    Available(Info),
    Installed,
    Failed(String),
}

#[derive(Clone)]
pub struct Info {
    /// The published version, `0.2.0`.
    pub version: String,
    /// What it brings, as written by whoever published it (may be empty).
    pub notes: Vec<String>,
}

/// Checks in the background: a few seconds after startup and then every hour.
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

/// Downloads and installs the new version; reports with `Installed` or `Failed`.
pub fn install(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        let ev = match install_now() {
            Ok(()) => UpdateEvent::Installed,
            Err(e) => UpdateEvent::Failed(e),
        };
        let _ = tx.send(AppEvent::Update(ev));
    });
}

/// The freshly installed binary. The running one has already been replaced, and Linux shows
/// it as "… (deleted)".
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
    if !is_newer(&version, THIS_VERSION) {
        return None;
    }
    let notes = v["notas"].as_array().map(|n| n.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    Some(Info { version, notes })
}

/// `a` is newer than `b` (`0.10.0` > `0.9.3`). Anything unparseable is not newer.
fn is_newer(a: &str, b: &str) -> bool {
    let parts = |s: &str| s.split(['.', '-']).take(3).map(|p| p.parse::<u64>().ok()).collect::<Option<Vec<_>>>();
    match (parts(a), parts(b)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn install_now() -> Result<(), String> {
    // The website's installer, binary only and into this one's folder: no system dependencies
    // (they'd ask for sudo without a terminal), no voice model and no Claude Code.
    let dir = exe().parent().map(Path::to_path_buf).ok_or("no sé dónde está instalado Iris")?;
    if cfg!(windows) {
        let script = format!("$env:IRIS_SOLO_BINARIO='1'; $env:IRIS_DESTINO='{}'; irm {INSTALLER_PS1} | iex", dir.display());
        run(Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]))?;
    } else {
        let script = format!("curl -fsSL {INSTALLER_SH} | sh -s -- --solo-binario");
        run(Command::new("sh").args(["-c", &script]).env("IRIS_DESTINO", &dir))?;
    }
    Ok(())
}

/// Runs and returns the output; on failure, the last line of the error.
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
    use super::is_newer;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.3"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("pronto", "0.1.0"));
    }
}
