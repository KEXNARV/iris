//! Screen styles: how everything around the core is laid out and drawn. The colors are still
//! those of the Omarchy theme; this is the composition. Chosen with `/theme` and saved in
//! `~/.config/iris/estilo`.

use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiStyle {
    /// Bordered panels: conversation on the left, core and activity on the right.
    Clasico,
    /// No boxes: the conversation as a script and the core up front with its telemetry.
    Propuesta,
    /// Instrument HUD: corners, timestamped log and the turn line.
    Cabina,
    /// Voice first: core in the center, history on the left and what it's doing on the right.
    Cine,
    /// A single-color CRT: double frame, oscilloscope graticule and sweep.
    Fosforo,
    /// A disc split into sectors: each tool shows up in its own.
    Radar,
    /// A newspaper: the state as headline, data column, reading and the core as a figure.
    Editorial,
    /// Almost nothing: a pulsing dot, the conversation below and the input line.
    Zen,
    /// Everything as a log: time, type, content and duration, line by line.
    Bitacora,
    /// Tiles: core, context, cost, tools and session.
    Tablero,
}

/// (style, id, name, what for)
pub const ALL: [(UiStyle, &str, &str, &str); 10] = [
    (UiStyle::Clasico, "clasico", "Clásico", "paneles con borde, como siempre"),
    (UiStyle::Propuesta, "propuesta", "Propuesta", "sin cajas, el núcleo al frente"),
    (UiStyle::Cabina, "cabina", "Cabina", "HUD denso: registro con hora y línea del turno"),
    (UiStyle::Cine, "cine", "Cine", "la voz primero: historial, núcleo y lo que hace"),
    (UiStyle::Fosforo, "fosforo", "Fósforo", "CRT de un solo color, con retícula y barrido"),
    (UiStyle::Radar, "radar", "Radar", "cada herramienta aparece en su sector"),
    (UiStyle::Editorial, "editorial", "Editorial", "titular grande, columna de lectura y figura"),
    (UiStyle::Zen, "zen", "Zen", "casi nada: un punto que late y la línea de escritura"),
    (UiStyle::Bitacora, "bitacora", "Bitácora", "todo en orden, línea por línea, con su hora"),
    (UiStyle::Tablero, "tablero", "Tablero", "mosaicos: contexto, costo, herramientas y sesión"),
];

impl UiStyle {
    pub fn id(self) -> &'static str {
        ALL.iter().find(|e| e.0 == self).map_or("clasico", |e| e.1)
    }

    pub fn name(self) -> &'static str {
        ALL.iter().find(|e| e.0 == self).map_or("Clásico", |e| e.2)
    }

    /// By id or by the start of the name, ignoring accents and case (`clas`, `Cine`).
    pub fn find(q: &str) -> Option<UiStyle> {
        // Baymax (and Chopper) used to be attached to the style; now it's a buddy (`/buddy`), so
        // something saved as `cine-baymax` is Cine.
        let q = strip_accents(&q.trim().to_lowercase());
        let q = q.replace("-baymax", "").replace(" baymax", "").replace("-chopper", "");
        if q.is_empty() {
            return None;
        }
        ALL.iter().find(|e| e.1.starts_with(&q) || strip_accents(&e.2.to_lowercase()).starts_with(&q)).map(|e| e.0)
    }
}

fn strip_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' => 'u',
            c => c,
        })
        .collect()
}

fn path() -> Option<PathBuf> {
    Some(crate::paths::config()?.join("estilo"))
}

/// `IRIS_ESTILO` overrides what's saved; with nothing, the classic one.
pub fn load() -> UiStyle {
    if let Some(e) = crate::paths::var("ESTILO").and_then(|s| UiStyle::find(&s)) {
        return e;
    }
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| UiStyle::find(&s))
        .unwrap_or(UiStyle::Clasico)
}

pub fn save(e: UiStyle) -> std::io::Result<()> {
    let Some(p) = path() else { return Ok(()) };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(p, format!("{}\n", e.id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_id_name_and_prefix() {
        assert_eq!(UiStyle::find("cine"), Some(UiStyle::Cine));
        assert_eq!(UiStyle::find("Clásico"), Some(UiStyle::Clasico));
        assert_eq!(UiStyle::find("clas"), Some(UiStyle::Clasico));
        assert_eq!(UiStyle::find(" CAB \n"), Some(UiStyle::Cabina));
        assert_eq!(UiStyle::find("pro"), Some(UiStyle::Propuesta));
        assert_eq!(UiStyle::find("cine-baymax"), Some(UiStyle::Cine));
        assert_eq!(UiStyle::find("Clásico Baymax"), Some(UiStyle::Clasico));
        assert_eq!(UiStyle::find("cine-chopper"), Some(UiStyle::Cine));
        assert_eq!(UiStyle::find("fósforo"), Some(UiStyle::Fosforo));
        assert_eq!(UiStyle::find("Bitácora"), Some(UiStyle::Bitacora));
        assert_eq!(UiStyle::find("ze"), Some(UiStyle::Zen));
        assert_eq!(UiStyle::find("x"), None);
        assert_eq!(UiStyle::find(""), None);
    }

    #[test]
    fn ids_and_find_round_trip() {
        for (e, id, _, _) in ALL {
            assert_eq!(e.id(), id);
            assert_eq!(UiStyle::find(id), Some(e));
        }
    }
}
