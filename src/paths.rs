//! Where Iris keeps its things, the same on Linux and Windows, and the environment variables.
//!
//! Linux: `~/.config/iris` (choices), `~/.local/share/iris` (models and voice) and
//! `~/.local/state/iris` (history). Windows: `%APPDATA%\iris` and `%LOCALAPPDATA%\iris`.
//! It used to be called JARVIS: on startup the old folders are moved and the `JARVIS_*`
//! variables still apply if there is no `IRIS_*` one.

use std::path::{Path, PathBuf};

const NAME: &str = "iris";
const OLD_NAME: &str = "jarvis";

/// What is chosen and saved (style, buddy, colors, installed pieces).
pub fn config() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(NAME))
}

/// The heavy stuff that gets downloaded: the voice models and the speaking voice's environment.
pub fn data() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join(NAME))
}

/// What accumulates over time (the prompt history). Windows has no state folder.
pub fn state() -> Option<PathBuf> {
    Some(dirs::state_dir().or_else(dirs::data_local_dir)?.join(NAME))
}

/// The user's folder. On Windows there is no `HOME` (it's `%USERPROFILE%`): reading it by hand
/// left Iris without `~/.claude` and `/resume` found no session.
pub fn home() -> Option<PathBuf> {
    dirs::home_dir()
}

/// `IRIS_<name>`, or `JARVIS_<name>` if only the old one is set.
pub fn var(name: &str) -> Option<String> {
    std::env::var(format!("IRIS_{name}")).or_else(|_| std::env::var(format!("JARVIS_{name}"))).ok()
}

pub fn has_var(name: &str) -> bool {
    std::env::var_os(format!("IRIS_{name}")).or_else(|| std::env::var_os(format!("JARVIS_{name}"))).is_some()
}

/// Moves `…/jarvis` to `…/iris` in the three folders, once. The old one keeps a link to the
/// new one: an old Iris still open (or a shortcut that names it) isn't left without its things.
pub fn migrate() {
    let bases = [dirs::config_dir(), dirs::data_local_dir(), dirs::state_dir()];
    for base in bases.into_iter().flatten() {
        relocate(&base.join(OLD_NAME), &base.join(NAME));
    }
}

fn relocate(old_path: &Path, new_path: &Path) {
    let is_folder = std::fs::symlink_metadata(old_path).is_ok_and(|m| m.is_dir());
    if !is_folder || new_path.exists() {
        return;
    }
    if std::fs::rename(old_path, new_path).is_ok() {
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(new_path, old_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_once_and_leaves_a_link() {
        let base = std::env::temp_dir().join(format!("iris-rutas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (old_path, new_path) = (base.join(OLD_NAME), base.join(NAME));
        std::fs::create_dir_all(&old_path).unwrap();
        std::fs::write(old_path.join("estilo"), "cine\n").unwrap();
        relocate(&old_path, &new_path);
        assert_eq!(std::fs::read_to_string(new_path.join("estilo")).unwrap(), "cine\n");
        #[cfg(unix)]
        assert_eq!(std::fs::read_to_string(old_path.join("estilo")).unwrap(), "cine\n", "the old path still leads there");
        // Already moved, doing it again does nothing.
        relocate(&old_path, &new_path);
        assert!(new_path.join("estilo").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn new_variable_overrides_old() {
        // Names nobody else uses: tests run in parallel.
        unsafe {
            std::env::set_var("JARVIS_PRUEBA_RUTAS", "vieja");
        }
        assert_eq!(var("PRUEBA_RUTAS").as_deref(), Some("vieja"));
        unsafe {
            std::env::set_var("IRIS_PRUEBA_RUTAS", "nueva");
        }
        assert_eq!(var("PRUEBA_RUTAS").as_deref(), Some("nueva"));
    }
}
