//! Chopper: la cara que reemplaza al cuerpo del núcleo y las cabecitas de las esquinas, en los
//! estilos Chopper. Solo líneas, de un color: el del tema o uno elegido.
//!
//! La cara del núcleo son trazos (líneas y elipses) en coordenadas de -1 a 1, con y hacia
//! arriba: así queda nítida a cualquier tamaño. Las cabecitas, chicas, van dibujadas a mano y
//! rellenas, que a ese tamaño se leen mejor que las líneas.
//!
//! El color sale de `nucleo` y `esquinas` en la paleta (`colores-chopper.toml`), o de
//! `JARVIS_NUCLEO` y `JARVIS_ESQUINAS`, que mandan al arrancar: `auto` (el del tema) o un
//! `#rrggbb`. Las esquinas además pueden ir `no`, que es lo que va si no se pide nada.

use std::f64::consts::{PI, TAU};
use std::sync::Mutex;

enum Trazo {
    /// Una línea quebrada.
    Linea(&'static [(f64, f64)]),
    /// Arco de elipse: centro, radios y ángulos de inicio y fin.
    Arco((f64, f64), (f64, f64), f64, f64),
    /// Arco de superelipse (`n` > 2: más cuadrada), para la copa del sombrero.
    Super((f64, f64), (f64, f64), f64, f64, f64),
}

/// Lo que no cambia.
const CARA: &[Trazo] = &[
    // Copa del sombrero: redondeada y ancha.
    Trazo::Super((0.0, 0.10), (0.58, 0.72), 2.6, 0.0, PI),
    // Ala: banda ancha de puntas redondeadas.
    Trazo::Linea(&[(-0.62, 0.10), (0.62, 0.10)]),
    Trazo::Linea(&[
        (-0.62, 0.10),
        (-0.70, 0.08),
        (-0.74, 0.02),
        (-0.70, -0.04),
        (-0.56, -0.06),
        (0.56, -0.06),
        (0.70, -0.04),
        (0.74, 0.02),
        (0.70, 0.08),
        (0.62, 0.10),
    ]),
    // La cruz.
    Trazo::Linea(&[(-0.17, 0.62), (0.17, 0.28)]),
    Trazo::Linea(&[(0.17, 0.62), (-0.17, 0.28)]),
    // La cara, bajo el ala.
    Trazo::Arco((0.0, -0.06), (0.58, 0.72), PI, TAU),
    // La nariz.
    Trazo::Arco((0.0, -0.46), (0.07, 0.045), 0.0, TAU),
];

/// Las astas y las orejas: van a la izquierda y la derecha es su espejo.
const ESPEJADOS: &[Trazo] = &[
    // Astas: un tallo que sale del costado del sombrero y dos ramas.
    Trazo::Linea(&[(-0.52, 0.36), (-0.66, 0.52), (-0.80, 0.78), (-0.84, 0.94)]),
    Trazo::Linea(&[(-0.66, 0.52), (-0.90, 0.62), (-0.98, 0.74)]),
    Trazo::Linea(&[(-0.74, 0.70), (-0.64, 0.86)]),
    // Orejitas bajo el ala.
    Trazo::Linea(&[(-0.56, -0.10), (-0.80, -0.16), (-0.86, -0.24), (-0.72, -0.27), (-0.54, -0.24)]),
];

/// El ojo izquierdo (el derecho es su espejo): centro y radios.
pub const OJO: ((f64, f64), (f64, f64)) = ((-0.22, -0.30), (0.12, 0.14));
const PUPILA: f64 = 0.05;
const BOCA_Y: f64 = -0.55;

/// Lo que se mueve: ojos cerrados (parpadeo, sueño), hacia dónde miran (-1..1, y hacia arriba)
/// y cuánto se abre la boca (0 cerrada, 1 del todo).
pub struct Gesto {
    pub cerrados: bool,
    pub mirada: [f64; 2],
    pub boca: f64,
}

/// Recorre la cara punto por punto, cada ~`paso` (en las mismas unidades), y llama a `plot`.
pub fn trazar(paso: f64, g: &Gesto, plot: &mut impl FnMut(f64, f64)) {
    let paso = paso.max(1e-3);
    for t in CARA {
        recorrer(t, 1.0, paso, plot);
    }
    for t in ESPEJADOS {
        recorrer(t, 1.0, paso, plot);
        recorrer(t, -1.0, paso, plot);
    }
    let ((ox, oy), (rx, ry)) = OJO;
    for cx in [ox, -ox] {
        if g.cerrados {
            linea(&[(cx - rx, oy), (cx + rx, oy)], paso, plot);
            continue;
        }
        arco((cx, oy), (rx, ry), 0.0, TAU, paso, plot);
        // Pupila: un disco que se corre hacia donde mira, sin salirse del ojo.
        let (px, py) = (cx + g.mirada[0] * (rx - PUPILA * 1.4), oy - 0.02 + g.mirada[1] * (ry - PUPILA * 1.4));
        disco((px, py), PUPILA, paso, plot);
    }
    if g.boca > 0.15 {
        // Abierta: un óvalo que crece con la voz.
        let b = g.boca.min(1.0);
        arco((0.0, BOCA_Y - 0.03 * b), (0.07, 0.03 + 0.06 * b), 0.0, TAU, paso, plot);
    } else {
        // Cerrada: una «ω» chiquita.
        arco((-0.06, BOCA_Y), (0.06, 0.05), PI, TAU, paso, plot);
        arco((0.06, BOCA_Y), (0.06, 0.05), PI, TAU, paso, plot);
    }
}

/// Un trazo; `s` = -1 lo da vuelta en espejo.
fn recorrer(t: &Trazo, s: f64, paso: f64, plot: &mut impl FnMut(f64, f64)) {
    let mut plot = |x: f64, y: f64| plot(x * s, y);
    match t {
        Trazo::Linea(pts) => linea(pts, paso, &mut plot),
        Trazo::Arco(c, r, a0, a1) => arco(*c, *r, *a0, *a1, paso, &mut plot),
        Trazo::Super((cx, cy), (a, b), n, a0, a1) => {
            let k = muestras(a.max(*b) * (a1 - a0), paso);
            for i in 0..=k {
                let th = a0 + (a1 - a0) * i as f64 / k as f64;
                let (c, sn) = (th.cos(), th.sin());
                plot(cx + a * c.signum() * c.abs().powf(2.0 / n), cy + b * sn.signum() * sn.abs().powf(2.0 / n));
            }
        }
    }
}

fn linea(pts: &[(f64, f64)], paso: f64, plot: &mut impl FnMut(f64, f64)) {
    for w in pts.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        let k = muestras((x1 - x0).hypot(y1 - y0), paso);
        for i in 0..=k {
            let u = i as f64 / k as f64;
            plot(x0 + (x1 - x0) * u, y0 + (y1 - y0) * u);
        }
    }
}

fn arco((cx, cy): (f64, f64), (rx, ry): (f64, f64), a0: f64, a1: f64, paso: f64, plot: &mut impl FnMut(f64, f64)) {
    let k = muestras(rx.max(ry) * (a1 - a0).abs(), paso);
    for i in 0..=k {
        let a = a0 + (a1 - a0) * i as f64 / k as f64;
        plot(cx + rx * a.cos(), cy + ry * a.sin());
    }
}

fn disco((cx, cy): (f64, f64), r: f64, paso: f64, plot: &mut impl FnMut(f64, f64)) {
    let p = paso * 0.7;
    let n = (r / p).ceil() as i32;
    for j in -n..=n {
        for i in -n..=n {
            let (x, y) = (i as f64 * p, j as f64 * p);
            if x.hypot(y) <= r {
                plot(cx + x, cy + y);
            }
        }
    }
}

/// Cuántos puntos hacen falta para que un trazo de largo `largo` no quede cortado.
fn muestras(largo: f64, paso: f64) -> usize {
    ((largo / (paso * 0.5)).ceil() as usize).clamp(2, 4000)
}

/// La cabecita de las esquinas: 14×9 píxeles sólidos, con la cruz, los ojos y la boca huecos
/// (como la de la barra de la maqueta azul). Va en sextantes: 7×3 celdas.
pub const MINI: [&str; 9] = [
    ".a..######..a.",
    "aa.########.aa",
    ".a.##.##.##.a.",
    ".a.###..###.a.",
    "...##.##.##...",
    ".############.",
    "##.########.##",
    "...##.##.##...",
    "....##..##....",
];

pub fn mini(col: usize, fila: usize) -> bool {
    MINI.get(fila).and_then(|f| f.as_bytes().get(col)).is_some_and(|b| *b != b'.')
}

/// El sextante (bloque de 2×3) con estos píxeles prendidos: bit 0 arriba a la izquierda, 1 arriba
/// a la derecha, 2 y 3 al medio, 4 y 5 abajo. Unicode se salta las dos mitades y el bloque lleno,
/// que ya existían.
pub fn sextante(bits: u8) -> char {
    match bits & 63 {
        0 => ' ',
        21 => '▌',
        42 => '▐',
        63 => '█',
        v => {
            let i = v as u32 - 1 - (v > 21) as u32 - (v > 42) as u32;
            char::from_u32(0x1fb00 + i).unwrap_or('█')
        }
    }
}

/// De qué color va: el del tema o uno elegido.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Auto,
    Color(u32),
}

/// Lo que se ofrece en `/nucleo` y `/esquinas`: (id, nombre, modo).
pub const OPCIONES: [(&str, &str, Modo); 7] = [
    ("auto", "Auto (el del tema)", Modo::Auto),
    ("rosa", "Rosa del sombrero", Modo::Color(0xffa6c9)),
    ("celeste", "Celeste", Modo::Color(0x9ed8f5)),
    ("crema", "Crema de la cruz", Modo::Color(0xfff1d6)),
    ("cafe", "Café de las astas", Modo::Color(0xe2ba8c)),
    ("blanco", "Blanco", Modo::Color(0xffffff)),
    ("amarillo", "Amarillo", Modo::Color(0xffd84a)),
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

/// Lo de ahora: el núcleo y las esquinas (`None`: ocultas).
static NUCLEO: Mutex<Modo> = Mutex::new(Modo::Auto);
static ESQUINAS: Mutex<Option<Modo>> = Mutex::new(None);

pub fn nucleo() -> Modo {
    *NUCLEO.lock().unwrap()
}

pub fn esquinas() -> Option<Modo> {
    *ESQUINAS.lock().unwrap()
}

pub fn poner_nucleo(m: Modo) {
    *NUCLEO.lock().unwrap() = m;
}

pub fn poner_esquinas(m: Option<Modo>) {
    *ESQUINAS.lock().unwrap() = m;
}

/// Toma `nucleo` y `esquinas` de la paleta; las variables de entorno mandan sobre ella. Lo que
/// no se entiende queda en `auto`.
pub fn leer(toml: &str) {
    let get = |key: &str| {
        toml.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let nucleo = std::env::var("JARVIS_NUCLEO").ok().or_else(|| get("nucleo"));
    poner_nucleo(nucleo.and_then(|s| Modo::buscar(&s)).unwrap_or(Modo::Auto));
    let esq = std::env::var("JARVIS_ESQUINAS").ok().or_else(|| get("esquinas"));
    poner_esquinas(match esq.as_deref().map(str::trim) {
        Some("no") => None,
        Some(s) => Some(Modo::buscar(s).unwrap_or(Modo::Auto)),
        // Sin pedirlas, no van (la usuaria las dejó en pausa).
        None => None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn puntos(g: &Gesto) -> Vec<(f64, f64)> {
        let mut v = vec![];
        trazar(0.02, g, &mut |x, y| v.push((x, y)));
        v
    }

    #[test]
    fn la_cara_entra_en_su_caja_y_es_simetrica() {
        let v = puntos(&Gesto { cerrados: false, mirada: [0.0, 0.0], boca: 0.0 });
        assert!(v.len() > 500);
        assert!(v.iter().all(|(x, y)| x.abs() <= 1.0 && y.abs() <= 1.0));
        // Con la mirada al frente, lo de la izquierda tiene su espejo a la derecha.
        let (izq, der) = (v.iter().filter(|p| p.0 < -0.3).count(), v.iter().filter(|p| p.0 > 0.3).count());
        assert!((izq as f64 / der as f64 - 1.0).abs() < 0.05, "{izq} vs {der}");
    }

    #[test]
    fn las_pupilas_siguen_la_mirada_sin_salirse() {
        let ((ox, oy), (rx, ry)) = OJO;
        let dentro = |(x, y): &&(f64, f64)| ((x - ox) / rx).powi(2) + ((y - oy) / ry).powi(2) < 0.6;
        // Mirando a la derecha, la pupila del ojo izquierdo se corre a la derecha.
        let v = puntos(&Gesto { cerrados: false, mirada: [1.0, 0.0], boca: 0.0 });
        let pupila: Vec<_> = v.iter().filter(dentro).collect();
        assert!(pupila.iter().any(|p| p.0 > ox + 0.03));
        assert!(pupila.iter().all(|p| p.0 > ox - 0.02));
        // Con los ojos cerrados no hay pupila: solo la raya.
        let v = puntos(&Gesto { cerrados: true, mirada: [0.0, 0.0], boca: 0.0 });
        assert!(v.iter().filter(dentro).all(|p| (p.1 - oy).abs() < 1e-9));
    }

    #[test]
    fn la_cabecita_es_pareja_y_simetrica() {
        for f in MINI {
            assert_eq!(f.len(), 14);
            assert_eq!(f, f.chars().rev().collect::<String>());
        }
        assert!(mini(1, 0) && !mini(0, 0) && !mini(14, 0) && !mini(1, 9));
    }

    #[test]
    fn sextantes() {
        assert_eq!(sextante(0), ' ');
        assert_eq!(sextante(1), '\u{1fb00}');
        assert_eq!(sextante(20), '\u{1fb13}');
        assert_eq!(sextante(21), '▌');
        assert_eq!(sextante(22), '\u{1fb14}');
        assert_eq!(sextante(42), '▐');
        assert_eq!(sextante(62), '\u{1fb3b}');
        assert_eq!(sextante(63), '█');
    }

    #[test]
    fn busca_modos() {
        assert_eq!(Modo::buscar("auto"), Some(Modo::Auto));
        assert_eq!(Modo::buscar("café"), Some(Modo::Color(0xe2ba8c)));
        assert_eq!(Modo::buscar("#123456"), Some(Modo::Color(0x123456)));
        assert_eq!(Modo::buscar("chopper"), None);
        assert_eq!(Modo::Color(0x123456).texto(), "#123456");
        assert_eq!(Modo::Auto.color(0xabcdef), 0xabcdef);
    }
}
