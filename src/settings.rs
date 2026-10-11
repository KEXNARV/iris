//! What is chosen in the welcome and saved, one file per setting in `~/.config/iris`
//! (like `estilo` and `buddy`): permissions, voice, model, effort and the accent without Omarchy.

use std::path::PathBuf;

fn path(key: &str) -> Option<PathBuf> {
    Some(crate::paths::config()?.join(key))
}

pub fn read(key: &str) -> Option<String> {
    let s = std::fs::read_to_string(path(key)?).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

pub fn save(key: &str, value: &str) -> std::io::Result<()> {
    let Some(p) = path(key) else { return Ok(()) };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(p, format!("{value}\n"))
}

/// How much Iris does without asking.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Permissions {
    /// `--dangerously-skip-permissions`: everything without asking (the usual).
    All,
    /// `acceptEdits`: edits files on its own and asks before commands that change something.
    Edit,
    /// `manual`: asks before anything that isn't just reading.
    Ask,
}

/// (permissions, id, name, what it's for)
pub const PERMISSIONS: [(Permissions, &str, &str, &str); 3] = [
    (Permissions::All, "todo", "Todo sin preguntar", "trabaja de corrido; confías en lo que hace (skip-permissions)"),
    (Permissions::Edit, "editar", "Edita solo", "cambia archivos sin preguntar; pide permiso para comandos"),
    (Permissions::Ask, "preguntar", "Pregunta todo", "pide permiso antes de cualquier cosa que no sea leer"),
];

impl Permissions {
    pub fn id(self) -> &'static str {
        PERMISSIONS.iter().find(|p| p.0 == self).map_or("todo", |p| p.1)
    }

    /// With nothing saved, everything without asking: that's how Iris worked before the welcome.
    pub fn load() -> Permissions {
        let s = read("permisos").unwrap_or_default();
        PERMISSIONS.iter().find(|p| p.1 == s).map_or(Permissions::All, |p| p.0)
    }
}

/// What the engine receives based on what's saved. Anything passed by hand on the command line
/// (`iris --model x`, `--dangerously-skip-permissions`) wins.
pub fn engine_args(extra: &[String]) -> Vec<String> {
    let has_flag = |flags: &[&str]| extra.iter().any(|a| flags.iter().any(|f| a == f || a.starts_with(&format!("{f}="))));
    let mut out = vec![];
    if !has_flag(&["--dangerously-skip-permissions", "--permission-mode"]) {
        match Permissions::load() {
            Permissions::All => out.push("--dangerously-skip-permissions".into()),
            Permissions::Edit => out.extend(["--permission-mode".into(), "acceptEdits".into()]),
            Permissions::Ask => out.extend(["--permission-mode".into(), "manual".into()]),
        }
    }
    if !has_flag(&["--model"]) {
        if let Some(m) = read("modelo") {
            out.extend(["--model".into(), m]);
        }
    }
    if !has_flag(&["--effort"]) {
        if let Some(e) = read("esfuerzo").filter(|e| e != "auto") {
            out.extend(["--effort".into(), e]);
        }
    }
    out
}

/// The welcome shows once: if it was already done or if Iris was already in use (a style is saved).
pub fn first_run() -> bool {
    read("bienvenida").is_none() && path("estilo").is_some_and(|p| !p.exists())
}

pub fn welcome_done() {
    let _ = save("bienvenida", "hecha");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handwritten_flags_win() {
        let a = |v: &[&str]| engine_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let with = a(&["--dangerously-skip-permissions", "--model=sonnet", "--effort", "low"]);
        assert!(!with.iter().any(|x| x == "--permission-mode" || x == "--dangerously-skip-permissions"));
        assert!(!with.iter().any(|x| x == "--model" || x == "--effort"));
    }

    #[test]
    fn unique_ids() {
        for (p, id, _, _) in PERMISSIONS {
            assert_eq!(p.id(), id);
        }
    }
}
