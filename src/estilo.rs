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
    /// Un CRT de un solo color: marco doble, retícula de osciloscopio y barrido.
    Fosforo,
    /// Un disco por sectores: cada herramienta aparece en el suyo.
    Radar,
    /// Un diario: el estado como titular, columna de datos, lectura y el núcleo como figura.
    Editorial,
    /// Casi nada: un punto que late, la conversación abajo y la línea de escritura.
    Zen,
    /// Todo como un registro: hora, tipo, contenido y duración, línea por línea.
    Bitacora,
    /// Mosaicos: núcleo, contexto, costo, herramientas y sesión.
    Tablero,
}

/// (estilo, id, nombre, para qué)
pub const TODOS: [(Estilo, &str, &str, &str); 10] = [
    (Estilo::Clasico, "clasico", "Clásico", "paneles con borde, como siempre"),
    (Estilo::Propuesta, "propuesta", "Propuesta", "sin cajas, el núcleo al frente"),
    (Estilo::Cabina, "cabina", "Cabina", "HUD denso: registro con hora y línea del turno"),
    (Estilo::Cine, "cine", "Cine", "la voz primero: historial, núcleo y lo que hace"),
    (Estilo::Fosforo, "fosforo", "Fósforo", "CRT de un solo color, con retícula y barrido"),
    (Estilo::Radar, "radar", "Radar", "cada herramienta aparece en su sector"),
    (Estilo::Editorial, "editorial", "Editorial", "titular grande, columna de lectura y figura"),
    (Estilo::Zen, "zen", "Zen", "casi nada: un punto que late y la línea de escritura"),
    (Estilo::Bitacora, "bitacora", "Bitácora", "todo en orden, línea por línea, con su hora"),
    (Estilo::Tablero, "tablero", "Tablero", "mosaicos: contexto, costo, herramientas y sesión"),
];

impl Estilo {
    pub fn id(self) -> &'static str {
        TODOS.iter().find(|e| e.0 == self).map_or("clasico", |e| e.1)
    }

    pub fn nombre(self) -> &'static str {
        TODOS.iter().find(|e| e.0 == self).map_or("Clásico", |e| e.2)
    }

    /// Por id o por el comienzo del nombre, sin importar tildes ni mayúsculas (`clas`, `Cine`).
    pub fn buscar(q: &str) -> Option<Estilo> {
        // Antes Baymax (y Chopper) venía pegado al estilo; ahora es un buddy (`/buddy`), así que
        // lo guardado como `cine-baymax` es Cine.
        let q = sin_tildes(&q.trim().to_lowercase());
        let q = q.replace("-baymax", "").replace(" baymax", "").replace("-chopper", "");
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
        assert_eq!(Estilo::buscar("cine-baymax"), Some(Estilo::Cine));
        assert_eq!(Estilo::buscar("Clásico Baymax"), Some(Estilo::Clasico));
        assert_eq!(Estilo::buscar("cine-chopper"), Some(Estilo::Cine));
        assert_eq!(Estilo::buscar("fósforo"), Some(Estilo::Fosforo));
        assert_eq!(Estilo::buscar("Bitácora"), Some(Estilo::Bitacora));
        assert_eq!(Estilo::buscar("ze"), Some(Estilo::Zen));
        assert_eq!(Estilo::buscar("x"), None);
        assert_eq!(Estilo::buscar(""), None);
    }

    #[test]
    fn ids_y_buscar_van_de_ida_y_vuelta() {
        for (e, id, _, _) in TODOS {
            assert_eq!(e.id(), id);
            assert_eq!(Estilo::buscar(id), Some(e));
        }
    }
}
