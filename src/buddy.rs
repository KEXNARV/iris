//! Who lives in the core: the original (the little ball with its arcs) or a character. It's
//! separate from the style, so any style can carry either. Picked with `/buddy` and saved in
//! `~/.config/iris/buddy` (on Windows, `%APPDATA%\\iris\\buddy`).

use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buddy {
    /// The usual core, with Omarchy's colors.
    Original,
    /// Baymax's face; its face color is saved in `~/.config/iris/colores-baymax.toml`.
    Baymax,
}

/// (buddy, id, name, what it's for)
pub const ALL: [(Buddy, &str, &str, &str); 2] = [
    (Buddy::Original, "original", "Original", "el núcleo de siempre, con sus arcos"),
    (Buddy::Baymax, "baymax", "Baymax", "su cara en el núcleo · /core para su color"),
];

impl Buddy {
    pub fn id(self) -> &'static str {
        ALL.iter().find(|b| b.0 == self).map_or("original", |b| b.1)
    }

    pub fn name(self) -> &'static str {
        ALL.iter().find(|b| b.0 == self).map_or("Original", |b| b.2)
    }

    /// The character's file (its face color), if it has one. The colors come from the theme.
    pub fn palette(self) -> Option<PathBuf> {
        match self {
            Buddy::Baymax => Some(path()?.with_file_name("colores-baymax.toml")),
            Buddy::Original => None,
        }
    }

    /// By id or by the start of the name, case-insensitive (`bay`, `Original`).
    pub fn find(q: &str) -> Option<Buddy> {
        // Chopper was the first character; anything saying "chopper" is now Baymax.
        let q = q.trim().to_lowercase().replace("chopper", "baymax");
        if q.is_empty() {
            return None;
        }
        ALL.iter().find(|b| b.1.starts_with(&q) || b.2.to_lowercase().starts_with(&q)).map(|b| b.0)
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

pub fn save(b: Buddy) -> std::io::Result<()> {
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
