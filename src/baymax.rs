//! Baymax: the face that replaces the core's body with the Baymax buddy. An oval filled with one color (the theme's or a chosen one) with hollow eyes:
//! two dots joined by a line, through which the background shows, like his black eyes on his
//! white face.
//!
//! The color comes from `nucleo` in the palette (`colores-baymax.toml`), or from `IRIS_NUCLEO`,
//! which wins at startup: `auto` (the theme's) or a `#rrggbb`.

use std::sync::Mutex;

/// The head: semi-axes of the oval, in core units (-1..1, y pointing up).
const HEAD: (f64, f64) = (0.80, 0.52);
/// The eyes: distance from the center and radius.
const EYE_X: f64 = 0.38;
const EYE_R: f64 = 0.12;

/// What moves: eyes closed (blink, sleep) and where they look (-1..1, y pointing up).
pub struct Gesture {
    pub closed: bool,
    pub gaze: [f64; 2],
}

/// What goes at point (`x`, `y`) of the face: 0 nothing (outside, or in the eyes), 1 the face,
/// 3 the border. `du` is the size of a dot, so the border and the line don't get lost.
pub fn face(x: f64, y: f64, du: f64, g: &Gesture) -> u8 {
    let (rx, ry) = HEAD;
    let e = ((x / rx).powi(2) + (y / ry).powi(2)).sqrt();
    if e > 1.0 {
        return 0;
    }
    // The eyes shift together toward where it looks, without leaving the face.
    let (mx, my) = (g.gaze[0] * 0.12, g.gaze[1] * 0.08);
    let (ex, ey) = (x - mx, y - my);
    let line = ey.abs() < (1.3 * du).max(0.012) && ex.abs() < EYE_X;
    let eye = if g.closed {
        // Closed: the dots flatten into a stroke a bit thicker than the line.
        ey.abs() < (2.0 * du).max(0.02) && (ex.abs() - EYE_X).abs() < EYE_R
    } else {
        (ex.abs() - EYE_X).hypot(ey) < EYE_R
    };
    if eye || line {
        0
    } else if e > 1.0 - 2.6 * du / ry {
        3
    } else {
        1
    }
}

/// What color it is: the theme's or a chosen one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Auto,
    Color(u32),
}

/// What `/core` offers: (id, name, mode).
pub const OPTIONS: [(&str, &str, Mode); 7] = [
    ("auto", "Auto (el del tema)", Mode::Auto),
    ("blanco", "Blanco", Mode::Color(0xffffff)),
    ("rosa", "Rosa", Mode::Color(0xffa6c9)),
    ("celeste", "Celeste", Mode::Color(0x9ed8f5)),
    ("crema", "Crema", Mode::Color(0xfff1d6)),
    ("cafe", "Café", Mode::Color(0xe2ba8c)),
    ("rojo", "Rojo de su armadura", Mode::Color(0xff5a5a)),
];

impl Mode {
    /// By id, start of the name or `#rrggbb`.
    pub fn find(q: &str) -> Option<Mode> {
        let q = q.trim().to_lowercase().replace('é', "e");
        if let Some(h) = q.strip_prefix('#') {
            return (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok().map(Mode::Color))?;
        }
        if q.is_empty() {
            return None;
        }
        OPTIONS.iter().find(|o| o.0.starts_with(&q) || o.1.to_lowercase().replace('é', "e").starts_with(&q)).map(|o| o.2)
    }

    pub fn text(self) -> String {
        match (OPTIONS.iter().find(|o| o.2 == self), self) {
            (Some(o), _) => o.0.to_string(),
            (None, Mode::Color(c)) => format!("#{c:06X}"),
            (None, Mode::Auto) => "auto".into(),
        }
    }

    pub fn name(self) -> String {
        OPTIONS.iter().find(|o| o.2 == self).map_or_else(|| self.text(), |o| o.1.to_string())
    }

    /// The color, with the theme's accent for `Auto`.
    pub fn color(self, accent: u32) -> u32 {
        match self {
            Mode::Auto => accent,
            Mode::Color(c) => c,
        }
    }
}

/// A color toward the background: for the dim tones.
pub fn toward(c: u32, background: u32, f: f64) -> [f64; 3] {
    let v = |x: u32, k: u32| (x >> k & 0xff) as f64;
    [16, 8, 0].map(|k| v(c, k) + (v(background, k) - v(c, k)) * f)
}

/// The current color.
static CORE: Mutex<Mode> = Mutex::new(Mode::Auto);

pub fn core() -> Mode {
    *CORE.lock().unwrap()
}

pub fn set_core(m: Mode) {
    *CORE.lock().unwrap() = m;
}

/// Takes `nucleo` from the palette; the environment variable overrides it. Whatever isn't
/// understood stays `auto`.
pub fn read(toml: &str) {
    let get = |key: &str| {
        toml.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let core = crate::paths::var("NUCLEO").or_else(|| get("nucleo"));
    set_core(core.and_then(|s| Mode::find(&s)).unwrap_or(Mode::Auto));
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPEN: Gesture = Gesture { closed: false, gaze: [0.0, 0.0] };

    #[test]
    fn face_has_border_fill_and_hollow_eyes() {
        let du = 0.02;
        assert_eq!(face(0.0, 0.3, du, &OPEN), 1, "the forehead is face");
        assert_eq!(face(0.0, 0.515, du, &OPEN), 3, "at the very top, the border");
        assert_eq!(face(0.0, 0.6, du, &OPEN), 0, "outside the oval, nothing");
        assert_eq!(face(-EYE_X, 0.0, du, &OPEN), 0, "the eye is hollow");
        assert_eq!(face(0.0, 0.0, du, &OPEN), 0, "so is the line between the eyes");
        assert_eq!(face(0.0, 0.06, du, &OPEN), 1, "but it is thin");
    }

    #[test]
    fn eyes_look_and_close() {
        let du = 0.02;
        let right = Gesture { closed: false, gaze: [1.0, 0.0] };
        // Looking right, the left eye shifts: where it was there is now face.
        assert_eq!(face(-EYE_X - 0.08, 0.0, du, &right), 1);
        assert_eq!(face(-EYE_X + 0.12, 0.0, du, &right), 0);
        let closed = Gesture { closed: true, gaze: [0.0, 0.0] };
        assert_eq!(face(-EYE_X, 0.08, du, &closed), 1, "closed, the eye is just a stroke");
        assert_eq!(face(-EYE_X, 0.0, du, &closed), 0);
    }

    #[test]
    fn finds_modes() {
        assert_eq!(Mode::find("auto"), Some(Mode::Auto));
        assert_eq!(Mode::find("café"), Some(Mode::Color(0xe2ba8c)));
        assert_eq!(Mode::find("#123456"), Some(Mode::Color(0x123456)));
        assert_eq!(Mode::find("x"), None);
        assert_eq!(Mode::Color(0x123456).text(), "#123456");
        assert_eq!(Mode::Auto.color(0xabcdef), 0xabcdef);
    }
}
