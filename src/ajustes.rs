//! Lo que se elige en la bienvenida y se guarda, un archivo por ajuste en `~/.config/iris`
//! (como `estilo` y `buddy`): permisos, voz, modelo, esfuerzo y el acento sin Omarchy.

use std::path::PathBuf;

fn path(clave: &str) -> Option<PathBuf> {
    Some(crate::rutas::config()?.join(clave))
}

pub fn leer(clave: &str) -> Option<String> {
    let s = std::fs::read_to_string(path(clave)?).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

pub fn guardar(clave: &str, valor: &str) -> std::io::Result<()> {
    let Some(p) = path(clave) else { return Ok(()) };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(p, format!("{valor}\n"))
}

/// Cuánto deja hacer Iris sin preguntar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Permisos {
    /// `--dangerously-skip-permissions`: todo sin preguntar (lo de siempre).
    Todo,
    /// `acceptEdits`: edita archivos solo y pregunta antes de comandos que cambian algo.
    Editar,
    /// `manual`: pregunta antes de cualquier cosa que no sea solo leer.
    Preguntar,
}

/// (permisos, id, nombre, para qué)
pub const PERMISOS: [(Permisos, &str, &str, &str); 3] = [
    (Permisos::Todo, "todo", "Todo sin preguntar", "trabaja de corrido; confías en lo que hace (skip-permissions)"),
    (Permisos::Editar, "editar", "Edita solo", "cambia archivos sin preguntar; pide permiso para comandos"),
    (Permisos::Preguntar, "preguntar", "Pregunta todo", "pide permiso antes de cualquier cosa que no sea leer"),
];

impl Permisos {
    pub fn id(self) -> &'static str {
        PERMISOS.iter().find(|p| p.0 == self).map_or("todo", |p| p.1)
    }

    /// Sin nada guardado, todo sin preguntar: así trabajaba Iris antes de la bienvenida.
    pub fn cargar() -> Permisos {
        let s = leer("permisos").unwrap_or_default();
        PERMISOS.iter().find(|p| p.1 == s).map_or(Permisos::Todo, |p| p.0)
    }
}

/// Lo que el motor recibe según lo guardado. Lo que venga a mano en la línea de órdenes
/// (`iris --model x`, `--dangerously-skip-permissions`) manda.
pub fn args_motor(extra: &[String]) -> Vec<String> {
    let trae = |flags: &[&str]| extra.iter().any(|a| flags.iter().any(|f| a == f || a.starts_with(&format!("{f}="))));
    let mut out = vec![];
    if !trae(&["--dangerously-skip-permissions", "--permission-mode"]) {
        match Permisos::cargar() {
            Permisos::Todo => out.push("--dangerously-skip-permissions".into()),
            Permisos::Editar => out.extend(["--permission-mode".into(), "acceptEdits".into()]),
            Permisos::Preguntar => out.extend(["--permission-mode".into(), "manual".into()]),
        }
    }
    if !trae(&["--model"]) {
        if let Some(m) = leer("modelo") {
            out.extend(["--model".into(), m]);
        }
    }
    if !trae(&["--effort"]) {
        if let Some(e) = leer("esfuerzo").filter(|e| e != "auto") {
            out.extend(["--effort".into(), e]);
        }
    }
    out
}

/// La bienvenida sale una vez: si ya se hizo o si Iris ya se usaba (hay un estilo guardado).
pub fn primera_vez() -> bool {
    leer("bienvenida").is_none() && path("estilo").is_some_and(|p| !p.exists())
}

pub fn bienvenida_hecha() {
    let _ = guardar("bienvenida", "hecha");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_escrito_a_mano_manda() {
        let a = |v: &[&str]| args_motor(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let con = a(&["--dangerously-skip-permissions", "--model=sonnet", "--effort", "low"]);
        assert!(!con.iter().any(|x| x == "--permission-mode" || x == "--dangerously-skip-permissions"));
        assert!(!con.iter().any(|x| x == "--model" || x == "--effort"));
    }

    #[test]
    fn ids_unicos() {
        for (p, id, _, _) in PERMISOS {
            assert_eq!(p.id(), id);
        }
    }
}
