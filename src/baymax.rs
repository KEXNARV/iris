//! Baymax: la cara que reemplaza al cuerpo del núcleo con el buddy Baymax. Un óvalo relleno de un color (el del tema o uno elegido) con los ojos huecos:
//! dos puntos unidos por una línea, por donde se ve el fondo, como sus ojos negros en la cara
//! blanca.
//!
//! El color sale de `nucleo` en la paleta (`colores-baymax.toml`), o de `JARVIS_NUCLEO`, que
//! manda al arrancar: `auto` (el del tema) o un `#rrggbb`.

use std::sync::Mutex;

/// La cabeza: semiejes del óvalo, en las unidades del núcleo (-1..1, y hacia arriba).
const CABEZA: (f64, f64) = (0.80, 0.52);
/// Los ojos: distancia del centro y radio.
const OJO_X: f64 = 0.38;
const OJO_R: f64 = 0.12;

/// Lo que se mueve: ojos cerrados (parpadeo, sueño) y hacia dónde miran (-1..1, y hacia arriba).
pub struct Gesto {
    pub cerrados: bool,
    pub mirada: [f64; 2],
}

/// Qué va en el punto (`x`, `y`) de la cara: 0 nada (fuera, o en los ojos), 1 la cara, 3 el
/// borde. `du` es lo que mide un punto, para que el borde y la línea no se pierdan.
pub fn cara(x: f64, y: f64, du: f64, g: &Gesto) -> u8 {
    let (rx, ry) = CABEZA;
    let e = ((x / rx).powi(2) + (y / ry).powi(2)).sqrt();
    if e > 1.0 {
        return 0;
    }
    // Los ojos se corren juntos hacia donde mira, sin salirse de la cara.
    let (mx, my) = (g.mirada[0] * 0.12, g.mirada[1] * 0.08);
    let (ex, ey) = (x - mx, y - my);
    let linea = ey.abs() < (1.3 * du).max(0.012) && ex.abs() < OJO_X;
    let ojo = if g.cerrados {
        // Cerrados: los puntos se aplastan en una raya un poco más gruesa que la línea.
        ey.abs() < (2.0 * du).max(0.02) && (ex.abs() - OJO_X).abs() < OJO_R
    } else {
        (ex.abs() - OJO_X).hypot(ey) < OJO_R
    };
    if ojo || linea {
        0
    } else if e > 1.0 - 2.6 * du / ry {
        3
    } else {
        1
    }
}

/// La paleta de Baymax si todavía no hay archivo: en blanco y negro, como él.
pub const PALETA: &str = "\
# Paleta de Baymax en JARVIS: blanco y negro, como él.
# Cada comentario va en su propia línea: JARVIS no entiende un comentario al lado del valor.

# Blanco: Baymax, bordes, lo principal.
accent = \"#FFFFFF\"
# Texto.
foreground = \"#F2F2F2\"
# Texto apagado.
light_foreground = \"#8C8C8C\"
# Fondo negro: sus ojos huecos se ven negros (cambia con /fondo).
background = \"#000000\"
";

/// De qué color va: el del tema o uno elegido.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Auto,
    Color(u32),
}

/// Lo que se ofrece en `/nucleo`: (id, nombre, modo).
pub const OPCIONES: [(&str, &str, Modo); 7] = [
    ("auto", "Auto (el del tema)", Modo::Auto),
    ("blanco", "Blanco", Modo::Color(0xffffff)),
    ("rosa", "Rosa", Modo::Color(0xffa6c9)),
    ("celeste", "Celeste", Modo::Color(0x9ed8f5)),
    ("crema", "Crema", Modo::Color(0xfff1d6)),
    ("cafe", "Café", Modo::Color(0xe2ba8c)),
    ("rojo", "Rojo de su armadura", Modo::Color(0xff5a5a)),
];

impl Modo {
    /// Por id, comienzo del nombre o `#rrggbb`.
    pub fn buscar(q: &str) -> Option<Modo> {
        let q = q.trim().to_lowercase().replace('é', "e");
        if let Some(h) = q.strip_prefix('#') {
            return (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok().map(Modo::Color))?;
        }
        if q.is_empty() {
            return None;
        }
        OPCIONES.iter().find(|o| o.0.starts_with(&q) || o.1.to_lowercase().replace('é', "e").starts_with(&q)).map(|o| o.2)
    }

    pub fn texto(self) -> String {
        match (OPCIONES.iter().find(|o| o.2 == self), self) {
            (Some(o), _) => o.0.to_string(),
            (None, Modo::Color(c)) => format!("#{c:06X}"),
            (None, Modo::Auto) => "auto".into(),
        }
    }

    pub fn nombre(self) -> String {
        OPCIONES.iter().find(|o| o.2 == self).map_or_else(|| self.texto(), |o| o.1.to_string())
    }

    /// El color, con el acento del tema para `Auto`.
    pub fn color(self, acento: u32) -> u32 {
        match self {
            Modo::Auto => acento,
            Modo::Color(c) => c,
        }
    }
}

/// Un color hacia el fondo: para los tonos apagados.
pub fn hacia(c: u32, fondo: u32, f: f64) -> [f64; 3] {
    let v = |x: u32, k: u32| (x >> k & 0xff) as f64;
    [16, 8, 0].map(|k| v(c, k) + (v(fondo, k) - v(c, k)) * f)
}

/// El color de ahora.
static NUCLEO: Mutex<Modo> = Mutex::new(Modo::Auto);

pub fn nucleo() -> Modo {
    *NUCLEO.lock().unwrap()
}

pub fn poner_nucleo(m: Modo) {
    *NUCLEO.lock().unwrap() = m;
}

/// Toma `nucleo` de la paleta; la variable de entorno manda sobre ella. Lo que no se entiende
/// queda en `auto`.
pub fn leer(toml: &str) {
    let get = |key: &str| {
        toml.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let nucleo = std::env::var("JARVIS_NUCLEO").ok().or_else(|| get("nucleo"));
    poner_nucleo(nucleo.and_then(|s| Modo::buscar(&s)).unwrap_or(Modo::Auto));
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABIERTOS: Gesto = Gesto { cerrados: false, mirada: [0.0, 0.0] };

    #[test]
    fn la_cara_tiene_borde_relleno_y_ojos_huecos() {
        let du = 0.02;
        assert_eq!(cara(0.0, 0.3, du, &ABIERTOS), 1, "la frente es cara");
        assert_eq!(cara(0.0, 0.515, du, &ABIERTOS), 3, "arriba de todo, el borde");
        assert_eq!(cara(0.0, 0.6, du, &ABIERTOS), 0, "fuera del óvalo, nada");
        assert_eq!(cara(-OJO_X, 0.0, du, &ABIERTOS), 0, "el ojo es hueco");
        assert_eq!(cara(0.0, 0.0, du, &ABIERTOS), 0, "la línea entre los ojos también");
        assert_eq!(cara(0.0, 0.06, du, &ABIERTOS), 1, "pero es fina");
    }

    #[test]
    fn los_ojos_miran_y_se_cierran() {
        let du = 0.02;
        let derecha = Gesto { cerrados: false, mirada: [1.0, 0.0] };
        // Mirando a la derecha, el ojo izquierdo se corre: donde estaba ya hay cara.
        assert_eq!(cara(-OJO_X - 0.08, 0.0, du, &derecha), 1);
        assert_eq!(cara(-OJO_X + 0.12, 0.0, du, &derecha), 0);
        let cerrados = Gesto { cerrados: true, mirada: [0.0, 0.0] };
        assert_eq!(cara(-OJO_X, 0.08, du, &cerrados), 1, "cerrado, el ojo es solo una raya");
        assert_eq!(cara(-OJO_X, 0.0, du, &cerrados), 0);
    }

    #[test]
    fn busca_modos() {
        assert_eq!(Modo::buscar("auto"), Some(Modo::Auto));
        assert_eq!(Modo::buscar("café"), Some(Modo::Color(0xe2ba8c)));
        assert_eq!(Modo::buscar("#123456"), Some(Modo::Color(0x123456)));
        assert_eq!(Modo::buscar("x"), None);
        assert_eq!(Modo::Color(0x123456).texto(), "#123456");
        assert_eq!(Modo::Auto.color(0xabcdef), 0xabcdef);
    }
}
