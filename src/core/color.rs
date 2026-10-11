//! Color: the fixed ones (listening, transcribing, green, red, amber), the inks each dot is
//! painted with (the base ones, the same for all buddies) and the color math (mixing, HSV,
//! separating from the accent).

use ratatui::style::Color;

/// Fixed, like on the keyboard: listening and transcribing don't depend on the theme.
pub(crate) const YELLOW: [f64; 3] = [255.0, 210.0, 0.0];
pub(crate) const PURPLE: [f64; 3] = [150.0, 0.0, 255.0];
pub(crate) const GREEN: [f64; 3] = [40.0, 235.0, 90.0];
pub(crate) const RED: [f64; 3] = [255.0, 40.0, 40.0];
pub(crate) const WARM: [f64; 3] = [255.0, 170.0, 40.0];

pub(crate) const INK_MAIN: u8 = 0;
pub(crate) const INK_GREEN: u8 = 1;
pub(crate) const INK_RED: u8 = 2;
pub(crate) const INK_WARM: u8 = 3;
/// The theme accent (Aether): the surroundings, which don't change with the state.
pub(crate) const INK_ACCENT: u8 = 4;
/// The own ink of whatever goes in the center (Baymax's face: the theme color or the one
/// chosen with `/core`).
pub(crate) const INK_FACE: u8 = 5;
/// From here on, one ink per kid: each with the color of what it's doing.
pub(crate) const INK_KID: u8 = 6;
pub(crate) const MAX_KID_INKS: usize = 40;

/// If a fixed color lands too close to the theme accent (blue Thinking with a blue theme), its
/// hue is rotated until it separates: every state must still read differently from Idle.
pub(crate) fn apart(c: [f64; 3], accent: [f64; 3]) -> [f64; 3] {
    let (h, s, v) = hsv(c);
    let (ha, sa, _) = hsv(accent);
    if s < 0.15 || sa < 0.15 {
        return c; // grays: no hue to compare
    }
    let d = (h - ha + 540.0) % 360.0 - 180.0; // signed difference, in degrees
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

/// The four tones of an ink (empty, dim, mid, full), with the dim ones in the theme's
/// gray.
pub(crate) fn tones(c: [f64; 3]) -> [[f64; 3]; 4] {
    let dim = crate::theme::dim_rgb();
    [dim, dim, mix(c, dim, 0.38), c]
}

pub(crate) fn rgb(c: [f64; 3]) -> Color {
    Color::Rgb(c[0].round() as u8, c[1].round() as u8, c[2].round() as u8)
}
