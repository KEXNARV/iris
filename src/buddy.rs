//! Quién vive en el núcleo: el original (la bolita con sus arcos) o un personaje. Es aparte del
//! estilo, así que cualquier estilo puede llevar cualquiera. Se elige con `/buddy` y se guarda en
//! `~/.config/jarvis/buddy`.

use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buddy {
    /// El núcleo de siempre, con los colores de Omarchy.
    Original,
    /// La cara de Baymax; el color de su cara se guarda en `~/.config/jarvis/colores-baymax.toml`.
    Baymax,
}

/// (buddy, id, nombre, para qué)
pub const TODOS: [(Buddy, &str, &str, &str); 2] = [
    (Buddy::Original, "original", "Original", "el núcleo de siempre, con sus arcos"),
    (Buddy::Baymax, "baymax", "Baymax", "su cara en el núcleo · /core para su color"),
];

impl Buddy {
    pub fn id(self) -> &'static str {
        TODOS.iter().find(|b| b.0 == self).map_or("original", |b| b.1)
    }

    pub fn nombre(self) -> &'static str {
        TODOS.iter().find(|b| b.0 == self).map_or("Original", |b| b.2)
    }

    /// El archivo del personaje (el color de su cara), si tiene. Los colores salen del tema.
    pub fn paleta(self) -> Option<PathBuf> {
        match self {
            Buddy::Baymax => Some(path()?.with_file_name("colores-baymax.toml")),
            Buddy::Original => None,
        }
    }

    /// Por id o por el comienzo del nombre, sin importar mayúsculas (`bay`, `Original`).
    pub fn buscar(q: &str) -> Option<Buddy> {
        // Chopper fue el primer personaje; lo que diga «chopper» ahora es Baymax.
        let q = q.trim().to_lowercase().replace("chopper", "baymax");
        if q.is_empty() {
            return None;
        }
        TODOS.iter().find(|b| b.1.starts_with(&q) || b.2.to_lowercase().starts_with(&q)).map(|b| b.0)
    }
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("jarvis").join("buddy"))
}

/// `JARVIS_BUDDY` manda sobre lo guardado. Sin nada guardado, el del estilo de antes (cuando
/// Baymax venía pegado a «Cine Baymax» y «Clásico Baymax»); si no, el original.
pub fn cargar() -> Buddy {
    if let Some(b) = std::env::var("JARVIS_BUDDY").ok().and_then(|s| Buddy::buscar(&s)) {
        return b;
    }
    let leer = |p: PathBuf| std::fs::read_to_string(p).ok();
    if let Some(s) = path().and_then(leer) {
        return Buddy::buscar(&s).unwrap_or(Buddy::Original);
    }
    let estilo = path().and_then(|p| leer(p.with_file_name("estilo"))).unwrap_or_default();
    de_estilo_viejo(&estilo)
}

fn de_estilo_viejo(estilo: &str) -> Buddy {
    if estilo.contains("baymax") || estilo.contains("chopper") { Buddy::Baymax } else { Buddy::Original }
}

pub fn guardar(b: Buddy) -> std::io::Result<()> {
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
    fn busca_por_id_nombre_y_prefijo() {
        assert_eq!(Buddy::buscar("baymax"), Some(Buddy::Baymax));
        assert_eq!(Buddy::buscar(" Bay \n"), Some(Buddy::Baymax));
        assert_eq!(Buddy::buscar("chopper"), Some(Buddy::Baymax));
        assert_eq!(Buddy::buscar("orig"), Some(Buddy::Original));
        assert_eq!(Buddy::buscar("x"), None);
        assert_eq!(Buddy::buscar(""), None);
        for (b, id, _, _) in TODOS {
            assert_eq!(b.id(), id);
            assert_eq!(Buddy::buscar(id), Some(b));
        }
    }

    #[test]
    fn solo_baymax_trae_paleta() {
        assert!(Buddy::Original.paleta().is_none());
        assert!(Buddy::Baymax.paleta().unwrap().ends_with("jarvis/colores-baymax.toml"));
    }

    #[test]
    fn los_estilos_baymax_de_antes_traen_a_baymax() {
        assert_eq!(de_estilo_viejo("cine-baymax\n"), Buddy::Baymax);
        assert_eq!(de_estilo_viejo("cine-chopper\n"), Buddy::Baymax);
        assert_eq!(de_estilo_viejo("cabina\n"), Buddy::Original);
        assert_eq!(de_estilo_viejo(""), Buddy::Original);
    }
}
