//! Estilos de la pantalla: cómo se reparte y se dibuja todo alrededor del núcleo. Los colores
//! siguen siendo los del tema de Omarchy; esto es la composición. Se elige con `/theme` y se
//! guarda en `~/.config/jarvis/estilo`.

use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Estilo {
    /// Paneles con borde: conversación a la izquierda, núcleo y actividad a la derecha.
    Clasico,
    /// Sin cajas: la conversación como guion y el núcleo al frente con su telemetría.
    Propuesta,
    /// HUD de instrumentos: esquinas, registro con hora y la línea del turno.
    Cabina,
    /// La voz primero: núcleo al centro, historial a la izquierda y lo que hace a la derecha.
    Cine,
    /// El clásico con la paleta de Chopper (`~/.config/jarvis/colores-chopper.toml`).
    ClasicoChopper,
    /// Cine con la misma paleta de Chopper.
    CineChopper,
}

/// (estilo, id, nombre, para qué)
pub const TODOS: [(Estilo, &str, &str, &str); 6] = [
    (Estilo::Clasico, "clasico", "Clásico", "paneles con borde, como siempre"),
    (Estilo::Propuesta, "propuesta", "Propuesta", "sin cajas, el núcleo al frente"),
    (Estilo::Cabina, "cabina", "Cabina", "HUD denso: registro con hora y línea del turno"),
    (Estilo::Cine, "cine", "Cine", "la voz primero: historial, núcleo y lo que hace"),
    (Estilo::ClasicoChopper, "clasico-chopper", "Clásico Chopper", "el clásico con los colores de Chopper"),
    (Estilo::CineChopper, "cine-chopper", "Cine Chopper", "cine con los colores de Chopper"),
];

/// Fondos de los estilos Chopper para `/fondo`: (id, nombre, color).
pub const FONDOS: [(&str, &str, u32); 11] = [
    ("chocolate", "Chocolate", 0x1e120c),
    ("nariz", "Azul de su nariz", 0x10233f),
    ("drum", "Noche nevada de Drum", 0x1a2332),
    ("sombrero", "Morado del sombrero", 0x24142e),
    ("pelaje", "Pelaje café suave", 0x3a2418),
    ("rojizo", "Café rojizo", 0x2b1a16),
    ("sunny", "Mar del Sunny, de noche", 0x0f2a33),
    ("pizarra", "Pizarra con un toque rosa", 0x2a2228),
    // Los claros que todavía dejan leer el crema del texto.
    ("celeste", "Celeste", 0x235a75),
    ("hielo", "Azul hielo", 0x1e3a55),
    ("real", "Azul real", 0x1a2d66),
];

/// Un fondo por número (`3`), id o comienzo del nombre (`drum`, `noche`) o `#rrggbb`.
pub fn buscar_fondo(q: &str) -> Option<u32> {
    let q = sin_tildes(&q.trim().to_lowercase());
    if q.is_empty() {
        return None;
    }
    if let Some(h) = q.strip_prefix('#') {
        return (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok())?;
    }
    if let Ok(n) = q.parse::<usize>() {
        return FONDOS.get(n.checked_sub(1)?).map(|f| f.2);
    }
    FONDOS.iter().find(|f| f.0.starts_with(&q) || sin_tildes(&f.1.to_lowercase()).starts_with(&q)).map(|f| f.2)
}

impl Estilo {
    pub fn id(self) -> &'static str {
        TODOS.iter().find(|e| e.0 == self).map_or("clasico", |e| e.1)
    }

    pub fn nombre(self) -> &'static str {
        TODOS.iter().find(|e| e.0 == self).map_or("Clásico", |e| e.2)
    }

    /// Cómo se reparte la pantalla: los Chopper usan la de su original.
    pub fn base(self) -> Estilo {
        match self {
            Estilo::ClasicoChopper => Estilo::Clasico,
            Estilo::CineChopper => Estilo::Cine,
            e => e,
        }
    }

    /// La paleta propia del estilo, si tiene; si no, manda el tema de Omarchy. Los dos Chopper
    /// comparten la misma.
    pub fn paleta(self) -> Option<PathBuf> {
        match self {
            Estilo::ClasicoChopper | Estilo::CineChopper => Some(path()?.with_file_name("colores-chopper.toml")),
            _ => None,
        }
    }

    /// Por id o por el comienzo del nombre, sin importar tildes ni mayúsculas (`clas`, `Cine`).
    pub fn buscar(q: &str) -> Option<Estilo> {
        let q = sin_tildes(&q.trim().to_lowercase());
        if q.is_empty() {
            return None;
        }
        TODOS.iter().find(|e| e.1.starts_with(&q) || sin_tildes(&e.2.to_lowercase()).starts_with(&q)).map(|e| e.0)
    }
}

fn sin_tildes(s: &str) -> String {
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
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("jarvis").join("estilo"))
}

/// `JARVIS_ESTILO` manda sobre lo guardado; sin nada, el clásico.
pub fn cargar() -> Estilo {
    if let Some(e) = std::env::var("JARVIS_ESTILO").ok().and_then(|s| Estilo::buscar(&s)) {
        return e;
    }
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| Estilo::buscar(&s))
        .unwrap_or(Estilo::Clasico)
}

pub fn guardar(e: Estilo) -> std::io::Result<()> {
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
    fn busca_por_id_nombre_y_prefijo() {
        assert_eq!(Estilo::buscar("cine"), Some(Estilo::Cine));
        assert_eq!(Estilo::buscar("Clásico"), Some(Estilo::Clasico));
        assert_eq!(Estilo::buscar("clas"), Some(Estilo::Clasico));
        assert_eq!(Estilo::buscar(" CAB \n"), Some(Estilo::Cabina));
        assert_eq!(Estilo::buscar("pro"), Some(Estilo::Propuesta));
        assert_eq!(Estilo::buscar("cine-chop"), Some(Estilo::CineChopper));
        assert_eq!(Estilo::buscar("Clásico Chopper"), Some(Estilo::ClasicoChopper));
        assert_eq!(Estilo::buscar("x"), None);
        assert_eq!(Estilo::buscar(""), None);
    }

    #[test]
    fn los_chopper_usan_la_pantalla_de_su_original() {
        assert_eq!(Estilo::ClasicoChopper.base(), Estilo::Clasico);
        assert_eq!(Estilo::CineChopper.base(), Estilo::Cine);
        assert_eq!(Estilo::Cabina.base(), Estilo::Cabina);
        assert!(Estilo::Cine.paleta().is_none());
        assert!(Estilo::CineChopper.paleta().unwrap().ends_with("jarvis/colores-chopper.toml"));
        assert_eq!(Estilo::ClasicoChopper.paleta(), Estilo::CineChopper.paleta());
    }

    #[test]
    fn busca_fondos_por_numero_nombre_y_color() {
        assert_eq!(buscar_fondo("3"), Some(0x1a2332));
        assert_eq!(buscar_fondo("drum"), Some(0x1a2332));
        assert_eq!(buscar_fondo("Café"), Some(0x2b1a16));
        assert_eq!(buscar_fondo("#301830"), Some(0x301830));
        assert_eq!(buscar_fondo("0"), None);
        assert_eq!(buscar_fondo("9"), Some(0x235a75));
        assert_eq!(buscar_fondo("celeste"), Some(0x235a75));
        assert_eq!(buscar_fondo("12"), None);
        assert_eq!(buscar_fondo("#12"), None);
        assert_eq!(buscar_fondo("x"), None);
    }

    #[test]
    fn ids_y_buscar_van_de_ida_y_vuelta() {
        for (e, id, _, _) in TODOS {
            assert_eq!(e.id(), id);
            assert_eq!(Estilo::buscar(id), Some(e));
        }
    }
}
