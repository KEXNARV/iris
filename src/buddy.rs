//! Who lives in the core: the original (the little ball with its arcs), Baymax, or any recipe:
//! the user's own (`~/.config/iris/buddies/<name>/recipe.toml`, also where Iris Studio's drafts
//! land) or one installed from the store. It's separate from the style, so any style can carry
//! any. Picked with `/buddy` and saved in `~/.config/iris/buddy` (on Windows,
//! `%APPDATA%\\iris\\buddy`).

use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Buddy {
    /// The usual core, with Omarchy's colors.
    Original,
    /// Baymax's face; its face color is saved in `~/.config/iris/colores-baymax.toml`.
    Baymax,
    /// A recipe, by name: the user's or the store's.
    Recipe(String),
}

/// A buddy as `/buddy` lists it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub buddy: Buddy,
    pub name: String,
    pub what: String,
}

/// Every buddy there is: the built-in ones, the user's recipes (and drafts) and the store's.
pub fn all() -> Vec<Entry> {
    let mut out: Vec<Entry> = ALL.iter().map(|(b, _, name, what)| Entry { buddy: b.clone(), name: name.to_string(), what: what.to_string() }).collect();
    if let Some(d) = crate::paths::config().map(|c| c.join("buddies")) {
        let mut mine: Vec<_> = std::fs::read_dir(d)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().join("recipe.toml").exists() && !e.file_name().to_string_lossy().starts_with('.'))
            .collect();
        mine.sort_by_key(|e| e.file_name());
        for e in mine {
            let name = e.file_name().to_string_lossy().to_string();
            let what = if e.path().join(".draft").exists() { "borrador de Iris Studio" } else { "tu receta" };
            out.push(Entry { buddy: Buddy::Recipe(name.clone()), name, what: what.into() });
        }
    }
    for (name, version, _, kind) in crate::store::installed() {
        if kind == "buddy" && !out.iter().any(|e| e.name == name) {
            out.push(Entry { buddy: Buddy::Recipe(name.clone()), name, what: format!("de la tienda · {version}") });
        }
    }
    out
}

/// Where a recipe buddy's files are: the user's folder first, then the store's newest version.
pub fn recipe_dir(name: &str) -> Option<PathBuf> {
    let mine = crate::paths::config()?.join("buddies").join(name);
    if mine.join("recipe.toml").exists() {
        return Some(mine);
    }
    crate::store::version_dir(name)
}

/// (buddy, id, name, what it's for)
pub const ALL: [(Buddy, &str, &str, &str); 2] = [
    (Buddy::Original, "original", "Original", "el núcleo de siempre, con sus arcos"),
    (Buddy::Baymax, "baymax", "Baymax", "su cara en el núcleo · /core para su color"),
];

impl Buddy {
    pub fn id(&self) -> String {
        match self {
            Buddy::Recipe(n) => n.clone(),
            b => ALL.iter().find(|x| x.0 == *b).map_or("original", |x| x.1).to_string(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Buddy::Recipe(n) => n.clone(),
            b => ALL.iter().find(|x| x.0 == *b).map_or("Original", |x| x.2).to_string(),
        }
    }

    /// The character's file (its face color), if it has one. The colors come from the theme.
    pub fn palette(&self) -> Option<PathBuf> {
        match self {
            Buddy::Baymax => Some(path()?.with_file_name("colores-baymax.toml")),
            _ => None,
        }
    }

    /// By id or by the start of the name, case-insensitive (`bay`, `Original`, `kirby`).
    pub fn find(q: &str) -> Option<Buddy> {
        // Chopper was the first character; anything saying "chopper" is now Baymax.
        let q = q.trim().to_lowercase().replace("chopper", "baymax");
        if q.is_empty() {
            return None;
        }
        if let Some(b) = ALL.iter().find(|b| b.1.starts_with(&q) || b.2.to_lowercase().starts_with(&q)) {
            return Some(b.0.clone());
        }
        let all = all();
        all.iter().find(|e| e.name == q).or_else(|| all.iter().find(|e| e.name.to_lowercase().starts_with(&q))).map(|e| e.buddy.clone())
    }
}

fn path() -> Option<PathBuf> {
    Some(crate::paths::config()?.join("buddy"))
}

/// `IRIS_BUDDY` overrides what's saved. With nothing saved, the one from the old style (when
/// Baymax came bundled with "Cine Baymax" and "Clásico Baymax"); otherwise, the original.
pub fn load() -> Buddy {
    if let Some(b) = crate::paths::var("BUDDY").and_then(|s| Buddy::find(&s)) {
        return b;
    }
    let read = |p: PathBuf| std::fs::read_to_string(p).ok();
    if let Some(s) = path().and_then(read) {
        return Buddy::find(&s).unwrap_or(Buddy::Original);
    }
    let style = path().and_then(|p| read(p.with_file_name("estilo"))).unwrap_or_default();
    from_old_style(&style)
}

fn from_old_style(style: &str) -> Buddy {
    if style.contains("baymax") || style.contains("chopper") { Buddy::Baymax } else { Buddy::Original }
}

pub fn save(b: &Buddy) -> std::io::Result<()> {
    let Some(p) = path() else { return Ok(()) };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(p, format!("{}\n", b.id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_id_name_and_prefix() {
        assert_eq!(Buddy::find("baymax"), Some(Buddy::Baymax));
        assert_eq!(Buddy::find(" Bay \n"), Some(Buddy::Baymax));
        assert_eq!(Buddy::find("chopper"), Some(Buddy::Baymax));
        assert_eq!(Buddy::find("orig"), Some(Buddy::Original));
        assert_eq!(Buddy::find("x"), None);
        assert_eq!(Buddy::find(""), None);
        for (b, id, _, _) in ALL {
            assert_eq!(b.id(), id);
            assert_eq!(Buddy::find(id), Some(b));
        }
        assert_eq!(Buddy::Recipe("kirby".into()).id(), "kirby");
    }

    #[test]
    fn only_baymax_has_palette() {
        assert!(Buddy::Original.palette().is_none());
        assert!(Buddy::Baymax.palette().unwrap().ends_with("iris/colores-baymax.toml"));
    }

    #[test]
    fn old_baymax_styles_bring_baymax() {
        assert_eq!(from_old_style("cine-baymax\n"), Buddy::Baymax);
        assert_eq!(from_old_style("cine-chopper\n"), Buddy::Baymax);
        assert_eq!(from_old_style("cabina\n"), Buddy::Original);
        assert_eq!(from_old_style(""), Buddy::Original);
    }
}
