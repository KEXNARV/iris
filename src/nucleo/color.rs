//! Color: los fijos (escuchar, transcribir, verde, rojo, ámbar), las tintas con que se
//! pinta cada punto (las de base, iguales para todos los buddies) y las cuentas de color
//! (mezclas, HSV, separar del acento).

use ratatui::style::Color;

/// Fijos, como en el teclado: escuchando y transcribiendo no dependen del tema.
pub(crate) const YELLOW: [f64; 3] = [255.0, 210.0, 0.0];
pub(crate) const PURPLE: [f64; 3] = [150.0, 0.0, 255.0];
pub(crate) const GREEN: [f64; 3] = [40.0, 235.0, 90.0];
pub(crate) const RED: [f64; 3] = [255.0, 40.0, 40.0];
pub(crate) const WARM: [f64; 3] = [255.0, 170.0, 40.0];

pub(crate) const INK_MAIN: u8 = 0;
pub(crate) const INK_GREEN: u8 = 1;
pub(crate) const INK_RED: u8 = 2;
pub(crate) const INK_WARM: u8 = 3;
/// El acento del tema (Aether): lo de alrededor, que no cambia con el estado.
pub(crate) const INK_ACCENT: u8 = 4;
/// La tinta propia de lo que va en el centro (la cara de Baymax: el color del tema o el que
/// se elija con `/core`).
pub(crate) const INK_CARA: u8 = 5;
/// Desde acá, una tinta por hijo: cada uno con el color de lo que está haciendo.
pub(crate) const INK_KID: u8 = 6;
pub(crate) const MAX_KID_INKS: usize = 40;

/// Si un color fijo cae demasiado cerca del acento del tema (Pensando azul con un tema azul),
/// se gira su tono hasta separarlo: cada estado tiene que seguir leyéndose distinto de En espera.
pub(crate) fn apart(c: [f64; 3], accent: [f64; 3]) -> [f64; 3] {
    let (h, s, v) = hsv(c);
    let (ha, sa, _) = hsv(accent);
    if s < 0.15 || sa < 0.15 {
        return c; // grises: no hay tono que comparar
    }
    let d = (h - ha + 540.0) % 360.0 - 180.0; // diferencia con signo, en grados
    if d.abs() >= 40.0 {
        return c;
    }
    let h = ha + if d >= 0.0 { 40.0 } else { -40.0 };
    from_hsv(h.rem_euclid(360.0), s, v)
}

pub(crate) fn hsv(c: [f64; 3]) -> (f64, f64, f64) {
    let [r, g, b] = c.map(|x| x / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn from_hsv(h: f64, s: f64, v: f64) -> [f64; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

pub(crate) fn mix(a: [f64; 3], b: [f64; 3], f: f64) -> [f64; 3] {
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f]
}

/// Los cuatro tonos de una tinta (vacío, apagado, medio, pleno), con los apagados en el gris
/// del tema.
pub(crate) fn tonos(c: [f64; 3]) -> [[f64; 3]; 4] {
    let dim = crate::theme::dim_rgb();
    [dim, dim, mix(c, dim, 0.38), c]
}

pub(crate) fn rgb(c: [f64; 3]) -> Color {
    Color::Rgb(c[0].round() as u8, c[1].round() as u8, c[2].round() as u8)
}
