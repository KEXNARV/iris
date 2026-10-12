//! Kirby as a recipe part: the body of Iris drawn as the pixel-art character, dot by dot, and
//! alive with every state. It goes in place of `builtin:blob` and reads where the body is and
//! how it moves (`body.*`, the gaze, the blink); Iris's arcs, motes and signs stay around it.
//!
//! Every dot of its box asks "what am I?" (a hat star, a sweat drop, an eye, the mouth, a
//! cheek, a foot, the outline, the body) and paints that, so the shapes come out crisp at any
//! size. The rest are particles: what it inhales while reading and the stars when it's done.

use core::f64::consts::{PI, TAU};
use iris_piece::export_part;
use iris_piece::part::*;

const PINK: u8 = own(0);
const CHEEK: u8 = own(1);
const FEET: u8 = own(2);
const DARK: u8 = own(3);
const BLUE: u8 = own(4);
const WHITE: u8 = own(5);
const MOUTH: u8 = own(6);
const STAR: u8 = own(7);
const BUBBLE: u8 = own(8);
/// The outlines: a darker pink for the body, a darker red for the feet.
const LINE: u8 = own(9);
const FEET_LINE: u8 = own(10);
const ORANGE: u8 = own(11);
const NIGHT: u8 = own(12);

// The inputs, in the order of `INPUTS`.
const BX: usize = 0;
const BY: usize = 1;
const BR: usize = 2;
const SX: usize = 3;
const SY: usize = 4;
const GX: usize = 5;
const GY: usize = 6;
const BLINK: usize = 7;
const RED: usize = 8;
const YAWN: usize = 9;
const VOICE: usize = 10;
const RIPPLE: usize = 11;
const BEAT: usize = 12;
const GROOVE: usize = 13;
const DEE: usize = 14;

/// Something it inhales, in its own units (1 = its radius, around its center).
#[derive(Clone, Copy)]
struct Mote {
    x: f64,
    y: f64,
    white: bool,
}

/// A star from the celebration, in its own units.
#[derive(Clone, Copy)]
struct Spark {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
}

struct Kirby {
    seed: u64,
    /// When each thing happened (seconds of `t`).
    error: f64,
    done: f64,
    copy: f64,
    key: f64,
    nope: f64,
    /// Which foot the last key tapped.
    left: bool,
    /// How much it's inhaling and how open its mouth is, eased.
    inhale: f64,
    open: f64,
    motes: Vec<Mote>,
    sparks: Vec<Spark>,
    /// When a returning subagent was slurped, and when its mouth got full (it then swallows).
    slurp: f64,
    full: f64,
}

/// How the eyes look this frame.
#[derive(Clone, Copy, PartialEq)]
enum Eyes {
    Open,
    /// A blink: a flat line.
    Shut,
    /// Asleep: a curve down.
    Asleep,
    /// Happy: ^ ^
    Happy,
    /// Inhaling: > <
    Squint,
    /// Hurt: x x
    Cross,
}

/// The mouth this frame.
#[derive(Clone, Copy, PartialEq)]
enum Mouth {
    /// The little v.
    Closed,
    /// Open by this much (0..1).
    Open(f64),
    /// A big happy smile.
    Smile,
    /// Hurt: a wavy line.
    Wavy,
    /// Its mouth full: pressed shut, a little wavy.
    Full,
}

/// The copy ability it wears, by state: a nightcap to sleep, fire to run, Hammer's headband
/// to edit.
#[derive(Clone, Copy, PartialEq)]
enum Cap {
    None,
    Night,
    Fire,
    Band,
}

/// Everything the dots need to know about this frame, in its own units.
struct Pose {
    t: f64,
    /// One outline wide, and the smallest feature that still reads (both in its units).
    w: f64,
    min: f64,
    arm_l: (f64, f64),
    arm_r: (f64, f64),
    foot_l: f64,
    foot_r: f64,
    cheek: f64,
    gaze: (f64, f64),
    eye_b: f64,
    eyes: Eyes,
    mouth: Mouth,
    /// The sleeping bubble's radius (0: none), the sweat drop's fall (none: no drop), the hat
    /// star's size (0: none).
    bubble: f64,
    sweat: Option<f64>,
    hat: f64,
    cap: Cap,
    /// Too many dots for one frame: the fills go checkered (outlines and features stay whole).
    hatch: bool,
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}

fn ease_out_back(u: f64) -> f64 {
    let c = 1.7;
    let u = u - 1.0;
    1.0 + (c + 1.0) * u * u * u + c * u * u
}

/// The length of (x, y): `sqrt` is one WASM instruction, `hypot` a whole function.
fn len(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// An ellipse's squared normalized radius at (u, v): below 1 inside. No square root: this runs
/// for every dot, and a big window has tens of thousands of them per frame.
fn q(u: f64, v: f64, cx: f64, cy: f64, a: f64, b: f64) -> f64 {
    let (x, y) = ((u - cx) / a, (v - cy) / b);
    x * x + y * y
}

/// Distance (roughly, in its units) from an ellipse's edge: negative inside.
fn ellipse(u: f64, v: f64, cx: f64, cy: f64, a: f64, b: f64) -> f64 {
    (len((u - cx) / a, (v - cy) / b) - 1.0) * a.min(b)
}

impl Kirby {
    fn rand(&mut self) -> f64 {
        // xorshift64*: its own chance, seeded by the buddy's when it was born.
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        (self.seed.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn burst(&mut self) {
        for k in 0..7 {
            let a = PI * (0.12 + 0.76 * (k as f64 + self.rand() * 0.6) / 7.0);
            let s = 1.3 + self.rand() * 0.7;
            self.sparks.push(Spark { x: 0.0, y: 0.35, vx: a.cos() * s, vy: a.sin() * s, age: 0.0 });
        }
    }

    fn pose(&self, io: &Io, r: f64) -> Pose {
        let t = io.t();
        let du = io.du();
        let state = io.state();
        let since = |at: f64, long: f64| ((t - at) / long).clamp(0.0, 1.0);
        let hurt = t - self.error < 1.4;
        let happy = t - self.done < 1.5;

        // Arms: resting at the sides; up while listening, waving to ask, cheering when done.
        let mut arm_l = (-0.86, -0.05);
        let mut arm_r = (0.86, 0.02);
        match state {
            State::Listening | State::Transcribing => {
                arm_l.1 += 0.14;
                arm_r.1 += 0.14;
            }
            State::Asking => arm_r = (0.8, 0.55 + 0.07 * (t * 9.0).sin()),
            State::Running | State::Testing => {
                arm_l.1 += 0.08 * (t * 14.0).sin();
                arm_r.1 -= 0.08 * (t * 14.0).sin();
            }
            _ => {}
        }
        if happy {
            let up = (PI * since(self.done, 0.7)).sin() * 0.4;
            arm_l.1 += up;
            arm_r.1 += up;
        }
        if hurt {
            let f = 1.0 - since(self.error, 1.4);
            arm_l.1 += 0.2 * f + 0.05 * (t * 30.0).sin() * f;
            arm_r.1 += 0.2 * f - 0.05 * (t * 30.0).sin() * f;
        }

        // Feet: a tap per key, running in place, stepping to the music.
        let tap = 0.1 * (1.0 - since(self.key, 0.18)) * if t - self.key < 0.18 { 1.0 } else { 0.0 };
        let (mut foot_l, mut foot_r) = if self.left { (tap, 0.0) } else { (0.0, tap) };
        if matches!(state, State::Running | State::Testing) {
            foot_l += (t * 14.0).sin().max(0.0) * 0.12;
            foot_r += (t * 14.0 + PI).sin().max(0.0) * 0.12;
        }
        let groove = io.input(GROOVE).clamp(0.0, 1.0);
        if groove > 0.05 {
            let beat = io.input(BEAT).clamp(0.0, 1.0) * groove * 0.1;
            if (t * 1.1).fract() < 0.5 { foot_l += beat } else { foot_r += beat }
        }

        // Where it looks, in its units: the gaze, and up and to the side while thinking.
        let lim = 0.07;
        let mut gaze = ((io.input(GX) / r * 0.5).clamp(-lim, lim), (io.input(GY) / r * 0.5).clamp(-lim, lim));
        if matches!(state, State::Thinking | State::Planning) {
            gaze = (gaze.0 * 0.5 - 0.05, gaze.1 * 0.5 + 0.05);
        }
        let gaze = (if gaze.0.is_finite() { gaze.0 } else { 0.0 }, if gaze.1.is_finite() { gaze.1 } else { 0.0 });

        let inhaling = self.inhale > 0.5;
        let eyes = if hurt {
            Eyes::Cross
        } else if happy {
            Eyes::Happy
        } else if inhaling {
            Eyes::Squint
        } else if matches!(state, State::Sleeping | State::Offline) {
            Eyes::Asleep
        } else if io.input(BLINK) > 0.5 || io.input(YAWN) > 0.6 {
            Eyes::Shut
        } else {
            Eyes::Open
        };
        let stuffed = t - self.full < 1.3;
        let mouth = if stuffed {
            Mouth::Full
        } else if hurt {
            Mouth::Wavy
        } else if happy {
            Mouth::Smile
        } else if self.open > 0.06 {
            Mouth::Open(self.open)
        } else {
            Mouth::Closed
        };
        let eye_b = 0.22 * if matches!(state, State::Listening | State::Asking) { 1.12 } else { 1.0 };

        let bubble = if state == State::Sleeping { 0.05 + 0.13 * (0.5 + 0.5 * (t * 1.1).sin()) } else { 0.0 };
        let sweat = (t - self.error < 1.6).then(|| since(self.error, 1.6));
        let hat = if t - self.copy < 2.6 {
            let u = t - self.copy;
            let pop = ease_out_back((u * 4.0).min(1.0));
            // It blinks out at the end.
            if u > 2.2 && (u * 16.0).fract() < 0.5 { 0.0 } else { pop }
        } else {
            0.0
        };

        let cap = match state {
            State::Sleeping => Cap::Night,
            State::Running | State::Testing => Cap::Fire,
            State::Editing => Cap::Band,
            _ => Cap::None,
        };
        Pose {
            hatch: false,
            cap,
            t,
            w: 1.15 * du / r,
            min: 1.3 * du / r,
            arm_l,
            arm_r,
            foot_l,
            foot_r,
            cheek: 1.0 + io.input(RED).clamp(0.0, 1.0) * 0.3 + self.inhale * 0.25 + if stuffed { 0.55 } else { 0.0 },
            gaze,
            eye_b,
            eyes,
            mouth,
            bubble,
            sweat,
            hat,
        }
    }
}

/// Where an ellipse crosses the row at `v`: its half width (negative: the row misses it).
fn span(v: f64, cy: f64, a: f64, b: f64) -> f64 {
    let y = (v - cy) / b;
    let h = 1.0 - y * y;
    if h > 0.0 { a * h.sqrt() } else { -1.0 }
}

/// An ellipse on one row: where it is, and where it's more than an outline inside.
#[derive(Clone, Copy)]
struct Cut {
    lo: f64,
    hi: f64,
    ilo: f64,
    ihi: f64,
}

impl Cut {
    fn new(v: f64, cx: f64, cy: f64, a: f64, b: f64, w: f64) -> Cut {
        let (h, ih) = (span(v, cy, a, b), span(v, cy, a - w, b - w));
        if h < 0.0 {
            return Cut { lo: 1.0, hi: -1.0, ilo: 1.0, ihi: -1.0 };
        }
        let ih = if ih < 0.0 { -1.0 } else { ih };
        Cut { lo: cx - h, hi: cx + h, ilo: cx - ih, ihi: cx + ih }
    }

    fn has(&self, u: f64) -> bool {
        u >= self.lo && u <= self.hi
    }

    fn deep(&self, u: f64) -> bool {
        u >= self.ilo && u <= self.ihi
    }
}

/// The hat, the copy star, the sweat drop and the bubble. Apart from `dot`: the WASM
/// interpreter charges for every local a function has when it's called, so the per-dot path
/// stays small and this one only runs near them.
#[inline(never)]
fn extras(p: &Pose, u: f64, v: f64) -> Option<(u8, u8)> {
    let w = p.w;
    // The copy star, above its head, turning.
    if p.hat > 0.0 && u.abs() < 0.3 && v > 1.0 && v < 1.65 {
        let (sx, sy) = (u, v - 1.32 - 0.03 * (p.t * 3.0).sin());
        let rr = len(sx, sy);
        if rr < 0.27 * p.hat {
            let th = sy.atan2(sx) - p.t * 1.5 - PI / 2.0;
            let f = (th * 5.0 / TAU).rem_euclid(1.0);
            let tip = (f * 2.0 - 1.0).abs();
            let edge = (0.11 + 0.15 * tip * tip) * p.hat;
            if rr < edge {
                return Some((3, if rr < edge * 0.45 { WHITE } else { STAR }));
            }
        }
    }

    if p.cap != Cap::None && v > 0.4 {
        if let Some(d) = cap(p, u, v) {
            return Some(d);
        }
    }

    // The sweat drop, sliding down its forehead.
    if let Some(f) = p.sweat.filter(|_| (u - 0.74).abs() < 0.2 && v > 0.3 && v < 1.1) {
        let (dx, dy, r) = (0.74, 0.74 - 0.25 * f, 0.075_f64.max(p.min));
        let top = dy + r * 2.4;
        if len(u - dx, v - dy) < r || (v > dy && v < top && (u - dx).abs() < (top - v) / 2.4) {
            return Some((3, if len(u - dx + r * 0.35, v - dy - r * 0.2) < r * 0.35 { WHITE } else { BLUE }));
        }
    }

    // The sleeping bubble, growing and shrinking at the mouth.
    if p.bubble > 0.0 && u > -0.1 && u < 0.6 && v > -0.6 && v < 0.2 {
        let rb = p.bubble.max(p.min);
        let (cx, cy) = (0.1 + rb * 0.85, -0.12 - rb * 0.25);
        let d = len(u - cx, v - cy);
        if len(u - cx + rb * 0.4, v - cy - rb * 0.4) < (rb * 0.22).max(p.min * 0.6) {
            return Some((3, WHITE));
        }
        if (d - rb).abs() < w * 0.8 {
            return Some((3, BUBBLE));
        }
        if d < rb {
            return Some((2, BUBBLE));
        }
    }

    None
}

/// The top of the body at `u` (where a hat sits).
fn crown(u: f64) -> f64 {
    0.02 + 0.93 * (1.0 - u * u).max(0.0).sqrt()
}

/// The copy ability's hat, if this dot is part of it.
#[inline(never)]
fn cap(p: &Pose, u: f64, v: f64) -> Option<(u8, u8)> {
    match p.cap {
        Cap::None => None,
        Cap::Night => {
            // A striped beanie over the head, a tail drooping to the right, a pompom at the end.
            if !(v > 0.5 && u > -1.1 && u < 1.3 && v < 1.3) {
                return None;
            }
            let sway = 0.04 * (p.t * 1.3).sin();
            let (px, py) = (1.05 + sway, 0.62 + sway * 0.5);
            if len(u - px, v - py) < 0.12_f64.max(p.min) {
                return Some((3, WHITE));
            }
            // The tail: from the top of the head to the pompom, thinning.
            for k in 0..=10 {
                let q = k as f64 / 10.0;
                let (a, b) = (1.0 - q, q);
                // A quadratic curve: the top, a bend up and right, the pompom.
                let (x, y) = (a * a * 0.15 + 2.0 * a * b * 0.75 + b * b * px, a * a * 1.02 + 2.0 * a * b * 1.2 + b * b * py);
                if len(u - x, v - y) < 0.2 * (1.0 - q * 0.75) {
                    return Some((3, if ((q * 6.0) as i32) % 2 == 0 { NIGHT } else { WHITE }));
                }
            }
            if v > 0.6 && ellipse(u, v, 0.0, 0.02, 1.04, 0.98) < 0.0 {
                let band = v < 0.69;
                return Some((3, if band || ((v * 9.0) as i32) % 2 == 0 { WHITE } else { NIGHT }));
            }
            None
        }
        Cap::Fire => {
            // Flames on its head, flickering: yellow at the root, orange on top.
            if u.abs() > 0.62 || v < 0.6 {
                return None;
            }
            let base = crown(u) - 0.06;
            let h = (0.28 + 0.12 * (u * 9.0 + p.t * 12.0).sin() + 0.08 * (u * 23.0 - p.t * 17.0).sin()) * (1.0 - (u / 0.62).powi(2));
            if v > base && v < base + h {
                return Some((3, if v < base + h * 0.5 { STAR } else { ORANGE }));
            }
            None
        }
        Cap::Band => {
            // Hammer's headband over the forehead, its knot's tails flapping on the right.
            let flap = (p.t * 9.0).sin();
            for (cy, k) in [(0.6, 1.0), (0.47, -0.7)] {
                if ellipse(u, v, 1.1 + 0.03 * flap, cy + 0.04 * flap * k, 0.17, 0.055_f64.max(p.min)) < 0.0 {
                    return Some((3, FEET));
                }
            }
            (v > 0.5 && v < 0.6 && ellipse(u, v, 0.0, 0.02, 1.02, 0.95) < 0.0).then_some((3, FEET))
        }
    }
}

/// The eyes, the mouth and the cheeks.
#[inline(never)]
fn face(p: &Pose, u: f64, v: f64) -> Option<(u8, u8)> {
    let w = p.w;
    let line = |d: f64| d.abs() < w * 0.75;

    for side in [-1.0, 1.0] {
        let (cx, cy) = (side * 0.21 + p.gaze.0, 0.24 + p.gaze.1);
        let (a, b) = (0.1_f64.max(p.min), p.eye_b.max(p.min * 2.0));
        let (x, y) = (u - cx, v - cy);
        if x.abs() > a * 1.5 || y.abs() > b * 1.3 {
            continue;
        }
        let hit = match p.eyes {
            Eyes::Open => {
                if ellipse(u, v, cx, cy, a, b) < 0.0 {
                    let ink = if len(x + a * 0.1, (y - b * 0.42) * 0.8) < a * 0.55 {
                        WHITE
                    } else if y < -b * 0.3 {
                        BLUE
                    } else {
                        DARK
                    };
                    return Some((3, ink));
                }
                false
            }
            Eyes::Shut => x.abs() < a * 1.2 && line(y + b * 0.2),
            Eyes::Asleep => x.abs() < a * 1.3 && line(y + b * 0.3 - 4.0 * x * x),
            Eyes::Happy => x.abs() < a * 1.3 && line(y - b * 0.1 + x.abs() * 1.4),
            Eyes::Squint => {
                // > on the left, < on the right: the tip points to the middle.
                let reach = a * 1.3;
                let k = -side * x;
                k.abs() < reach && line(y.abs() - (reach - k) * 0.55 - w * 0.2)
            }
            Eyes::Cross => x.abs() < a * 1.3 && (line(y - x) || line(y + x)) && y.abs() < a * 1.3,
        };
        if hit {
            return Some((3, DARK));
        }
    }

    let mouth_box = u.abs() < 0.32 && v < 0.05 && v > -0.5;
    match p.mouth {
        _ if !mouth_box => {}
        Mouth::Open(o) => {
            let (a, b) = ((0.05 + 0.14 * o).max(p.min), (0.03 + 0.24 * o).max(p.min));
            let cy = -0.08 - 0.05 * o;
            if ellipse(u, v, 0.0, cy, a, b) < 0.0 {
                return Some((3, if v < cy - b * 0.4 { CHEEK } else { MOUTH }));
            }
        }
        Mouth::Smile => {
            let (a, b) = (0.13_f64.max(p.min), 0.12_f64.max(p.min));
            if v < -0.04 && ellipse(u, v, 0.0, -0.04, a, b) < 0.0 {
                return Some((3, if v < -0.04 - b * 0.5 { CHEEK } else { MOUTH }));
            }
        }
        Mouth::Wavy => {
            if u.abs() < 0.11 && line(v + 0.07 - 0.025 * (u * 45.0).sin()) {
                return Some((3, DARK));
            }
        }
        Mouth::Full => {
            if u.abs() < 0.09 && line(v + 0.06 - 0.012 * (u * 50.0).cos()) {
                return Some((3, DARK));
            }
        }
        Mouth::Closed => {
            if u.abs() < 0.065 && line(v + 0.07 - u.abs() * 0.9) {
                return Some((3, DARK));
            }
        }
    }

    for side in [-1.0, 1.0] {
        if (u - side * 0.44).abs() < 0.3 && v.abs() < 0.16 && q(u, v, side * 0.44, 0.0, 0.15 * p.cheek, 0.075 * p.cheek) < 1.0 {
            return Some((3, CHEEK));
        }
    }
    None
}

impl Part for Kirby {
    const INPUTS: &'static [&'static str] =
        &["body.x", "body.y", "body.radius", "body.sx", "body.sy", "gaze.x", "gaze.y", "blink", "red", "yawn", "body.voice", "ripple", "beat", "groove", "dee.pull"];
    const SETTINGS: &'static [Setting] = &[Setting::new("size", 0.9, 0.4, 1.6)];
    const COLORS: &'static [&'static str] = &["#ffa3cf", "#ff6f9f", "#f0306a", "#120a24", "#6a7dff", "#ffffff", "#8a1640", "#ffe14a", "#b8ecff", "#c94882", "#a3173f", "#ff7a1a", "#4a5bd6"];

    fn new(_io: &Io, rand: &mut Rand) -> Self {
        let seed = (rand.rand() * 1e15) as u64 | 1;
        Kirby { seed, error: -99.0, done: -99.0, copy: -99.0, key: -99.0, nope: -99.0, left: false, inhale: 0.0, open: 0.0, motes: vec![], sparks: vec![], slurp: -99.0, full: -99.0 }
    }

    fn step(&mut self, io: &mut Io) {
        let (t, dt) = (io.t(), io.dt().clamp(0.0, 0.1));
        let state = io.state();
        let inhaling = matches!(state, State::Reading | State::Searching | State::Web) || t - self.slurp < 0.85 || io.input(DEE) > 0.5;
        // When it stops inhaling, whatever it caught stays in its mouth: it's full, then swallows.
        if !inhaling && self.inhale > 0.5 && t - self.full > 1.6 {
            self.full = t;
        }
        self.inhale += ((if inhaling { 1.0 } else { 0.0 }) - self.inhale) * (dt * 7.0).min(1.0);

        let voice = io.input(VOICE).max(0.0);
        let target = match state {
            _ if self.inhale > 0.5 => 0.85 + 0.12 * (t * 7.0).sin(),
            State::Speaking => (voice * 2.5 + io.input(RIPPLE).max(0.0) * 0.4 * (0.5 + 0.5 * (t * 13.0).sin())).clamp(0.15, 1.0),
            State::Listening | State::Transcribing => 0.14 + voice.min(0.3),
            State::Thinking | State::Planning => 0.12,
            State::Sleeping => 0.0,
            _ => 0.0,
        };
        self.open += (target - self.open) * (dt * 14.0).min(1.0);

        // What it inhales: specks from both sides, faster as they get close, gone in the mouth.
        if self.inhale > 0.3 && self.motes.len() < 120 {
            let n = (self.inhale * 45.0 * dt + self.rand()) as usize;
            for _ in 0..n {
                let side = if self.rand() < 0.5 { -1.0 } else { 1.0 };
                let (x, y) = (side * (1.5 + self.rand() * 0.7), -0.7 + self.rand() * 1.4);
                let white = self.rand() < 0.5;
                self.motes.push(Mote { x, y, white });
            }
        }
        let mouth = (0.0, -0.13);
        for m in &mut self.motes {
            let (dx, dy) = (mouth.0 - m.x, mouth.1 - m.y);
            let d = len(dx, dy).max(1e-6);
            let speed = 0.5 + 1.6 / (d + 0.25);
            m.x += dx / d * speed * dt;
            m.y += dy / d * speed * dt;
        }
        self.motes.retain(|m| len(m.x - mouth.0, m.y - mouth.1) > 0.12);
        if self.inhale < 0.3 {
            self.motes.clear();
        }

        for s in &mut self.sparks {
            s.vy -= 2.6 * dt;
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.age += dt;
        }
        self.sparks.retain(|s| s.age < 1.0);
    }

    fn event(&mut self, h: Happening, io: &mut Io) {
        let t = io.t();
        match h {
            Happening::Event(Event::Error) => self.error = t,
            Happening::Event(Event::Done | Event::Pass) => {
                self.done = t;
                self.burst();
            }
            Happening::Event(Event::Copy) => self.copy = t,
            Happening::Event(Event::Key) => {
                self.key = t;
                self.left = !self.left;
            }
            Happening::Event(Event::Nope | Event::Cancel) => self.nope = t,
            // A subagent comes back (or finishes well and flies back to it): it slurps it up.
            Happening::Event(Event::Merge) | Happening::Finish(_, true) => self.slurp = t,
            _ => {}
        }
    }

    fn paint(&self, g: &mut Grid, io: &Io) {
        let v = |i: usize| io.input(i);
        let r = v(BR) * io.setting(0);
        let du = io.du();
        if !(r > du * 2.0) || !v(BX).is_finite() || !v(BY).is_finite() {
            return;
        }
        let t = io.t();
        let p = self.pose(io, r);

        // Where it is: the body, plus its own hop when done, a shake when hurt or saying no, and
        // a sway to the music.
        let hop = {
            let u = (t - self.done) / 0.7;
            if (0.0..1.0).contains(&u) { (PI * u).sin() * 0.35 } else { 0.0 }
        };
        let land = {
            let u = (t - self.done - 0.7) / 0.2;
            if (0.0..1.0).contains(&u) { (PI * u).sin() * 0.1 } else { 0.0 }
        };
        let shake = {
            let e = 1.0 - ((t - self.error) / 0.5).clamp(0.0, 1.0);
            let n = 1.0 - ((t - self.nope) / 0.45).clamp(0.0, 1.0);
            0.05 * (t * 60.0).sin() * e + 0.06 * (t * 28.0).sin() * n
        };
        let sway = v(GROOVE).clamp(0.0, 1.0) * 0.06 * (t * TAU * 0.55).sin();
        let ok = |x: f64| if x.is_finite() && x > 0.1 { x } else { 1.0 };
        // Compacting: squished flat and back, like Stone.
        // Full: puffed up round. Then the gulp: a squash down and back.
        let since_full = t - self.full;
        let puff = if since_full < 1.3 { 0.14 * ((since_full / 0.12).min(1.0)) } else { 0.0 };
        let gulp = if (1.3..1.6).contains(&since_full) { (PI * (since_full - 1.3) / 0.3).sin() } else { 0.0 };
        let press = if io.state() == State::Compacting { 0.5 + 0.5 * (t * 3.0).sin() } else { 0.0 };
        let (sx, sy) = (ok(v(SX)) * (1.0 + land + self.inhale * 0.06 + press * 0.12 + puff + gulp * 0.08), ok(v(SY)) * (1.0 - land - press * 0.22 + puff - gulp * 0.16));
        // Booting: it arrives from above (on the warp star Dream Land draws under it).
        let drop = if io.state() == State::Booting { 1.0 - ease_out((io.st() / 1.2).min(1.0)) } else { 0.0 };
        let (cx, cy) = (v(BX) + (shake + sway) * r, v(BY) + (hop + drop * 3.0 - press * 0.2) * r);
        let (kx, ky) = (r * sx, r * sy);

        // Its box, in dots: the inverse of `io.center`.
        let (dw, dh) = io.grid_size();
        if dw < 2 || dh < 2 {
            return;
        }
        let (x0, y0) = io.center(0, 0);
        let (x1, y1) = io.center(1, 1);
        let (di, dj) = (x1 - x0, y1 - y0);
        if di == 0.0 || dj == 0.0 {
            return;
        }
        let col = |x: f64| ((x - x0) / di).round();
        let row = |y: f64| ((y - y0) / dj).round();
        let (a, b) = (col(cx - 1.4 * kx), col(cx + 1.4 * kx));
        let (c, d) = (row(cy + 1.7 * ky), row(cy - 1.25 * ky));
        let clamp = |n: f64, m: usize| n.clamp(0.0, m as f64 - 1.0) as usize;
        let (i0, i1) = (clamp(a.min(b), dw), clamp(a.max(b), dw));
        // A whole body of dots is about π·kx·ky / (dot area); past what one frame can send,
        // the fills go checkered.
        let mut p = p;
        p.hatch = PI * kx * ky / (di * dj).abs() > 12_000.0;
        let (j0, j1) = (clamp(c.min(d), dh), clamp(c.max(d), dh));
        // Row by row: where the body, the arms and the feet cross it (two square roots each),
        // and then each dot only compares numbers. The interpreter charges for every call (and
        // every local of the function called), so the face, the hats and the rest are only
        // called inside their own boxes.
        let w = p.w;
        let (gx, gy) = p.gaze;
        let over = |u: f64, v: f64| {
            (p.hat > 0.0 || p.cap != Cap::None) && v > 0.4 || p.sweat.is_some() && u > 0.5 && v > 0.3 && u < 0.95 || p.bubble > 0.0 && u > -0.1 && u < 0.6 && v < 0.2 && v > -0.6
        };
        let in_face = |u: f64, v: f64| {
            let eye = (v - 0.24 - gy).abs() < 0.32 && ((u - 0.21 - gx).abs() < 0.17 || (u + 0.21 - gx).abs() < 0.17);
            let mouth = u.abs() < 0.32 && v < 0.05 && v > -0.5;
            let cheek = (u.abs() - 0.44).abs() < 0.3 && v.abs() < 0.16;
            eye || mouth || cheek
        };
        for j in j0..=j1 {
            let v = (y0 + j as f64 * dj - cy) / ky;
            let body = Cut::new(v, 0.0, 0.02, 1.0, 0.93, w);
            let arms = [Cut::new(v, p.arm_l.0, p.arm_l.1, 0.3, 0.23, w), Cut::new(v, p.arm_r.0, p.arm_r.1, 0.3, 0.23, w)];
            let feet = [Cut::new(v, -0.48, -0.87 + p.foot_l, 0.42, 0.21, w), Cut::new(v, 0.5, -0.87 + p.foot_r, 0.42, 0.21, w)];
            let lo = body.lo.min(arms[0].lo).min(feet[0].lo);
            let hi = body.hi.max(arms[1].hi).max(feet[1].hi);
            // Whatever goes over it on this row widens the stretch to walk (only then).
            let mut reach = (lo, hi);
            let mut widen = |a: f64, b: f64| reach = (reach.0.min(a), reach.1.max(b));
            if p.hat > 0.0 && v > 1.0 && v < 1.65 {
                widen(-0.3, 0.3);
            }
            match p.cap {
                Cap::Night if v > 0.5 => widen(-1.1, 1.3),
                Cap::Fire if v > 0.6 => widen(-0.62, 0.62),
                Cap::Band if v > 0.4 && v < 0.75 => widen(-1.05, 1.35),
                _ => {}
            }
            if p.sweat.is_some() && v > 0.3 && v < 1.1 {
                widen(0.5, 0.95);
            }
            if p.bubble > 0.0 && v > -0.6 && v < 0.2 {
                widen(-0.1, 0.6);
            }
            if reach.0 > reach.1 {
                continue;
            }
            let maybe_over = reach != (lo, hi) || v > 0.4 && p.cap != Cap::None;
            // From u back to columns: only the dots of this stretch.
            let col = |u: f64| (u * kx + cx - x0) / di;
            let (ca, cb) = (col(reach.0).floor(), col(reach.1).ceil());
            let first = (ca.max(i0 as f64)) as usize;
            let last = (cb.min(i1 as f64)).max(0.0) as usize;
            for i in first..=last.max(first) {
                let u = (x0 + i as f64 * di - cx) / kx;
                let odd = p.hatch && (i + j) % 2 == 1;
                let put = |g: &mut Grid, ink: u8| g.dot(i, j, 3, ink);
                if maybe_over && over(u, v) {
                    if let Some((tone, ink)) = extras(&p, u, v) {
                        g.dot(i, j, tone, ink);
                        continue;
                    }
                }
                if u < lo || u > hi {
                    continue;
                }
                if feet[0].has(u) || feet[1].has(u) {
                    if feet[0].deep(u) || feet[1].deep(u) {
                        if !odd {
                            put(g, FEET);
                        }
                    } else {
                        put(g, FEET_LINE);
                    }
                    continue;
                }
                let inside = body.has(u) || arms[0].has(u) || arms[1].has(u);
                if !inside {
                    continue;
                }
                if !(body.deep(u) || arms[0].deep(u) || arms[1].deep(u)) {
                    put(g, LINE);
                    continue;
                }
                if in_face(u, v) {
                    if let Some((tone, ink)) = face(&p, u, v) {
                        g.dot(i, j, tone, ink);
                        continue;
                    }
                }
                if v > 0.4 && u < -0.25 && q(u, v, -0.45, 0.55, 0.16, 0.1) < 1.0 {
                    put(g, WHITE);
                } else if !odd {
                    put(g, PINK);
                }
            }
        }

        // What it inhales, with a short trail.
        for m in &self.motes {
            let (x, y) = (cx + m.x * kx, cy + m.y * ky);
            let ink = if m.white { WHITE } else { INK_MAIN };
            g.plot(x, y, 3, ink);
            let back = 0.06 * m.x.signum();
            g.plot(x + back * kx, y, 2, ink);
        }
        // The celebration's stars: a small cross that shrinks to a dot.
        for s in &self.sparks {
            let (x, y) = (cx + s.x * kx, cy + s.y * ky);
            g.plot(x, y, 3, STAR);
            if s.age < 0.6 {
                for (ox, oy) in [(du, 0.0), (-du, 0.0), (0.0, du), (0.0, -du)] {
                    g.plot(x + ox, y + oy, 3, STAR);
                }
            }
        }
    }
}

export_part!(Kirby);
