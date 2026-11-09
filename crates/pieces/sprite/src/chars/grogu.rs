//! Grogu (The Mandalorian): drawn from shapes every frame (long ears, a big head with huge dark
//! eyes, a fluffy collar, his brown robe), so everything on him moves. What he does:
//! - waiting: his ears rise and fall slowly, one flicks now and then, he blinks; music: he
//!   dances, ears flapping one after the other, little arms up and down;
//! - listening: ears straight up, eyes wide; asking: his head tilts, one ear down, a finger at
//!   his mouth;
//! - thinking, planning: the Force — eyes shut, brow furrowed, one hand up, the Razor Crest's
//!   shift knob floating over it;
//! - speaking: his ears go up and down with the voice, a little mouth cooing;
//! - reading, searching, on the web: the Force again, stones orbiting his raised hand;
//! - editing: a stone tumbling in front of his hand; running, testing: he waddles, feet out;
//!   git: he plays with the shift knob in both hands; delegating: a Force push, waves outward;
//! - done: both arms up, he floats, stones rising all around and a ring of the Force;
//! - error: the knob falls on his head — bonk, x x, a bump, ears down — and rolls away;
//! - a key: an ear flick and a glint; copying: a frog hops in and he eats it, legs dangling;
//! - asleep: in his floating cradle, ears over the edge, its hover glow under it;
//! - offline: the cradle closed, a red light; booting: the cradle opens and he peeks out;
//! - little context left: ears drooping, heavy eyelids.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Grogu";
pub const SCENE: &str = "space";
pub const DESCRIPTION: &str = "Grogu en pixel art: usa la Fuerza para leer y pensar, mueve las orejas al hablar, se come una rana al copiar y duerme en su cuna flotante";

const LINE: u8 = 1;
const GREEN: u8 = 2;
const GSHADE: u8 = 3;
const GLIGHT: u8 = 4;
const EAR: u8 = 5;
const EYE: u8 = 6;
const WHITE: u8 = 7;
const ROBE: u8 = 8;
const RSHADE: u8 = 9;
const RLINE: u8 = 10;
const COLLAR: u8 = 11;
const FORCE: u8 = 12;
const MOUTH: u8 = 13;
const METAL: u8 = 14;
const MSHADE: u8 = 15;
const RED: u8 = 16;

/// Pixel chars for sprites: the n-th char is color n + 1.
const KEY: &str = "lgsLpeWrRbcfmMXd";

const KNOB: &str = "
.MM.
MWMX
MMXX
.XX.
";

const FROG: &str = "
.ll.ll.
lWelWel
lgggggl
gLLLLLg
ll...ll
";

const ROCK_A: &str = "
.rr.
rrrR
.RR.
";

const ROCK_B: &str = "
rr
RR
";

#[derive(Clone, Copy, PartialEq)]
enum Look {
    Open,
    Wide,
    Shut,
    Force,
    Asleep,
    Happy,
    Cross,
    Heavy,
    Squeeze,
}

pub struct Char {
    flick: f64,
    flick_left: bool,
    next_flick: f64,
}

fn lerp(a: (f64, f64), b: (f64, f64), k: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k)
}

/// The ears and the head, green (at head center (cx, cy); tips relative to it).
fn head_shape(c: &mut Canvas, cx: f64, cy: f64, lt: (f64, f64), rt: (f64, f64)) {
    let (l, r) = ((cx + lt.0, cy + lt.1), (cx + rt.0, cy + rt.1));
    c.triangle(l, (cx - 6.0, cy - 4.5), (cx - 5.0, cy + 3.0), GREEN);
    c.triangle(r, (cx + 6.0, cy - 4.5), (cx + 5.0, cy + 3.0), GREEN);
    c.ellipse(cx, cy - 2.0, 8.5, 6.5, GREEN);
    c.ellipse(cx, cy + 2.0, 7.0, 5.5, GREEN);
}

/// The pink inside of the ears.
fn ear_insides(c: &mut Canvas, cx: f64, cy: f64, lt: (f64, f64), rt: (f64, f64)) {
    for (tip, a, b) in [
        ((cx + lt.0, cy + lt.1), (cx - 6.0, cy - 4.5), (cx - 5.0, cy + 3.0)),
        ((cx + rt.0, cy + rt.1), (cx + 6.0, cy - 4.5), (cx + 5.0, cy + 3.0)),
    ] {
        let mid = ((tip.0 + a.0 + b.0) / 3.0, (tip.1 + a.1 + b.1) / 3.0);
        let t2 = lerp(tip, mid, 0.3);
        let a2 = lerp(a, mid, 0.45);
        let b2 = lerp(b, mid, 0.45);
        c.triangle(t2, a2, b2, EAR);
        // Keep the inner pink off the head itself.
        let _ = a2;
    }
}

fn big_eye(c: &mut Canvas, cx: i32, cy: i32, e: Look, look: (i32, i32), right: bool) {
    let (fx, fy) = (cx as f64 + 0.5, cy as f64 + 0.5);
    match e {
        Look::Open | Look::Wide | Look::Heavy => {
            let ry = if e == Look::Wide { 3.0 } else { 2.6 };
            let (lx, ly) = (look.0 as f64 * 0.6, look.1 as f64 * 0.5);
            c.ellipse(fx + lx, fy + ly, 2.5, ry, EYE);
            let gx = cx - 1 + look.0;
            let gy = cy - 1 - if e == Look::Wide { 1 } else { 0 } + look.1;
            c.set(gx, gy, WHITE);
            c.set(gx + 1, gy, WHITE);
            c.set(gx, gy + 1, WHITE);
            c.set(cx + 1 + look.0, cy + 2 + look.1, WHITE);
            if e == Look::Heavy {
                for y in cy - 3..=cy {
                    for x in cx - 3..=cx + 3 {
                        if matches!(c.get(x, y), EYE | WHITE) {
                            c.set(x, y, GSHADE);
                        }
                    }
                }
                c.line(cx - 2, cy + 1, cx + 2, cy + 1, LINE);
            }
        }
        Look::Shut => c.line(cx - 2, cy + 1, cx + 2, cy + 1, LINE),
        Look::Force => {
            c.line(cx - 2, cy + 1, cx + 2, cy + 1, LINE);
            // The furrowed brow over a shut eye.
            let s = if right { -1 } else { 1 };
            c.set(cx - 2 * s, cy - 2, GSHADE);
            c.set(cx - s, cy - 2, GSHADE);
            c.set(cx, cy - 1, GSHADE);
        }
        Look::Asleep => {
            c.line(cx - 1, cy + 1, cx + 1, cy + 1, LINE);
            c.set(cx - 2, cy, LINE);
            c.set(cx + 2, cy, LINE);
        }
        Look::Happy => {
            c.set(cx - 2, cy + 1, EYE);
            c.set(cx - 1, cy, EYE);
            c.set(cx, cy - 1, EYE);
            c.set(cx + 1, cy, EYE);
            c.set(cx + 2, cy + 1, EYE);
        }
        Look::Squeeze => {
            let s = if right { -1 } else { 1 };
            c.set(cx - 2 * s, cy - 1, EYE);
            c.set(cx - s, cy, EYE);
            c.set(cx, cy + 1, EYE);
            c.set(cx - s, cy + 2, EYE);
            c.set(cx - 2 * s, cy + 3, EYE);
        }
        Look::Cross => {
            for k in -2..=2 {
                c.set(cx + k, cy + k, EYE);
                c.set(cx + k, cy - k, EYE);
            }
        }
    }
}

/// A hand at (x, y): a little green blob, fingers spread when `open`.
fn hand(c: &mut Canvas, x: f64, y: f64, open: bool) {
    c.ellipse(x, y, 1.7, 1.4, GREEN);
    if open {
        c.set(x as i32 - 2, y as i32 - 2, GREEN);
        c.set(x as i32, y as i32 - 3, GREEN);
        c.set(x as i32 + 1, y as i32 - 2, GREEN);
    }
}

impl Char {
    fn force(f: &Frame) -> bool {
        f.thinking() || f.reading() || f.state == State::Editing
    }

    fn copying(f: &Frame) -> Option<f64> {
        f.progress(f.times.copy, 2.6)
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#5f9e4a", "#a6d47e", "#7fb35f", "#d4f2a8", "#eba3a0", "#160c0a", "#ffffff", "#c99a62", "#a57a48", "#e2b878", "#efe2c2", "#8ff0ff",
        "#a8434a", "#d0d9e6", "#8c97ab", "#ff5050",
    ];
    const W: usize = 40;
    const H: usize = 40;

    fn new(seed: u64) -> Self {
        Char { flick: -99.0, flick_left: false, next_flick: 2.0 + hash(seed) * 3.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_flick {
            self.flick = f.t;
            self.flick_left = f.rand(f.t as u64) < 0.5;
            self.next_flick = f.t + 2.5 + f.rand(f.t as u64 + 9) * 4.0;
        }
    }

    fn event(&mut self, h: Happening, f: &Frame) {
        if let Happening::Event(Event::Key) = h {
            self.flick = f.t;
            self.flick_left = f.rand(f.t as u64 * 5 + 3) < 0.5;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let hurt = f.hurt();
        let happy = f.happy();
        let tired = f.context > 0.85 && !f.asleep();
        let running = f.is(&[State::Running, State::Testing]);
        let dance = f.groove > 0.1 && !hurt && !f.asleep();
        let copy = Self::copying(f);
        let booting = f.state == State::Booting && f.st < 2.2;

        // Asleep, offline or booting: the floating cradle.
        if f.asleep() || booting {
            let closed = f.state == State::Offline;
            // How open its lid is (booting: it opens, then he peeks out).
            let open = if booting { ((f.st - 0.4) / 0.8).clamp(0.0, 1.0) } else { 1.0 };
            let peek = if booting { ((f.st - 1.1) / 0.8).clamp(0.0, 1.0) } else { 1.0 };
            let (cx, cy) = (20.0, 28.0 - peek * 3.0);
            if !closed && peek > 0.0 {
                let droop = if f.asleep() { 3.0 } else { -1.0 };
                let lt = (-17.0, -2.0 + droop);
                let rt = (17.0, -2.0 + droop);
                head_shape(c, cx, cy, lt, rt);
                c.outline(LINE, &[]);
                c.shade(GREEN, GSHADE, LINE);
                c.light(GREEN, GLIGHT, LINE);
                ear_insides(c, cx, cy, lt, rt);
                let e = if f.asleep() { Look::Asleep } else if f.blink { Look::Shut } else { Look::Open };
                big_eye(c, 16, cy as i32 - 1, e, (0, 0), false);
                big_eye(c, 24, cy as i32 - 1, e, (0, 0), true);
                c.set(20, cy as i32 + 3, GSHADE);
            }
            // The pod: a silver bowl, its rim, a band, a light.
            for y in 30..40 {
                for x in 4..36 {
                    let (dx, dy) = ((x as f64 + 0.5 - 20.0) / 15.5, (y as f64 + 0.5 - 29.0) / 10.5);
                    if dx * dx + dy * dy <= 1.0 {
                        c.set(x, y, METAL);
                    }
                }
            }
            c.line(5, 30, 34, 30, WHITE);
            for x in 6..34 {
                if c.get(x, 34) == METAL {
                    c.set(x, 34, MSHADE);
                }
            }
            // The lid: closed (offline) it's a dome; opening, it tips back.
            if open < 1.0 || closed {
                let k = if closed { 0.0 } else { open };
                let (lx, ly) = (20.0 + k * 10.0, 30.0 - 6.5 * (1.0 - k) - k * 4.0);
                c.ellipse(lx, ly + 1.0, 14.5 * (1.0 - 0.6 * k), 6.5, METAL);
                c.rect(4, 30, 32, 1, WHITE);
            }
            c.outline(MSHADE, &[LINE, GREEN, GSHADE, GLIGHT, EAR, EYE, WHITE]);
            c.shade(METAL, MSHADE, MSHADE);
            // Its light: red offline, blinking.
            let blink = (t * 2.0).sin() > 0.0;
            let lc = if closed { if blink { RED } else { MSHADE } } else if blink { FORCE } else { WHITE };
            c.set(20, 35, lc);
            c.set(19, 36, MSHADE);
            c.set(21, 36, MSHADE);
            return;
        }

        // The ears: tips relative to the head center (they rise and fall slowly).
        let bob = f.wave(1.3, 0.0) * 1.2;
        let (mut lt, mut rt) = ((-18.5, -6.0 + bob), (18.5, -6.0 + bob));
        if hurt {
            lt = (-17.0, 4.0);
            rt = (17.0, 4.0);
        } else if f.is(&[State::Listening, State::Transcribing]) {
            lt = (-16.0, -11.0);
            rt = (16.0, -11.0);
        } else if running {
            let flap = f.wave(14.0, 0.0) * 1.5;
            lt = (-18.0, -1.0 + flap);
            rt = (18.0, -1.0 - flap);
        } else if f.state == State::Speaking {
            let v = (f.voice * 18.0).min(3.0) * f.wave(9.0, 0.0);
            lt = (-18.5, -6.0 - v);
            rt = (18.5, -6.0 - v);
        } else if f.state == State::Asking {
            rt = (17.5, 2.0);
        } else if tired {
            lt = (-17.5, 1.5);
            rt = (17.5, 1.5);
        } else if happy {
            lt = (-17.5, -10.0);
            rt = (17.5, -10.0);
        } else if f.state == State::NoVoice {
            lt = (-18.0, -2.0);
            rt = (18.0, -2.0);
        }
        if dance {
            let b = f.wave(PI * 2.2, 0.0) * 3.0 * f.groove.min(1.0);
            lt.1 += b;
            rt.1 -= b;
        }
        if let Some(u) = f.progress(self.flick, 0.3) {
            let k = (u * PI).sin() * 3.5;
            if self.flick_left { lt.1 -= k } else { rt.1 -= k }
        }
        if f.within(f.times.nope, 0.6) {
            let k = f.wave(30.0, 0.0) * 2.5;
            lt.1 += k;
            rt.1 -= k;
        }

        // The head: center, and his little feet waddling.
        let waddle = if running { f.wave(12.0, 0.0) } else { 0.0 };
        let (hx, hy) = (20.0 + waddle * 0.8, 14.0);

        // The robe: a bell of brown cloth, the sleeves, the hands.
        let hem = if running { 37.0 } else { 39.5 };
        if running {
            c.ellipse(16.0, 38.0 - waddle.max(0.0) * 1.5, 2.0, 1.4, GREEN);
            c.ellipse(24.0, 38.0 - (-waddle).max(0.0) * 1.5, 2.0, 1.4, GREEN);
        }
        c.quad((14.0, 21.0), (26.0, 21.0), (31.0, hem), (9.0, hem), ROBE);
        c.ellipse(20.0, hem - 5.0, 11.0, 5.0, ROBE);

        // The arms: where his hands go.
        let force = Self::force(f) && !hurt && !happy;
        let (mut lh, mut rh) = ((14.0, 30.0), (26.0, 30.0));
        let (mut lopen, mut ropen) = (false, false);
        if happy {
            lh = (7.0, 19.0);
            rh = (33.0, 19.0);
            lopen = true;
            ropen = true;
        } else if hurt {
            lh = (13.0, 22.0);
            rh = (27.0, 22.0);
        } else if let Some(u) = copy {
            if u > 0.3 && u < 0.5 {
                let k = ((u - 0.3) / 0.2).min(1.0);
                rh = lerp((33.0, 29.0), (23.0, 21.0), k);
            } else if u <= 0.3 {
                rh = (31.0, 28.0);
                ropen = true;
            } else {
                rh = (24.0, 24.0);
            }
        } else if force {
            rh = (33.0, 18.0 + f.wave(2.0, 0.0) * 0.6);
            ropen = true;
        } else if f.state == State::Delegating {
            lh = (6.0, 25.0);
            rh = (34.0, 25.0);
            lopen = true;
            ropen = true;
        } else if f.state == State::Git {
            let roll = f.wave(4.0, 0.0);
            lh = (17.0 + roll * 0.5, 27.0);
            rh = (23.0 + roll * 0.5, 27.0);
        } else if f.state == State::Asking {
            rh = (22.0, 21.5);
        } else if dance {
            let s = f.wave(PI * 2.2, 0.0);
            lh = (9.0, 25.0 - s * 4.0);
            rh = (31.0, 25.0 + s * 4.0);
        } else if running {
            lh = (11.0, 29.0 + waddle * 2.0);
            rh = (29.0, 29.0 - waddle * 2.0);
        }
        c.thick((15.5, 24.5), (lh.0 + 1.0, lh.1), 4.0, ROBE);
        c.thick((24.5, 24.5), (rh.0 - 1.0, rh.1), 4.0, ROBE);
        c.outline(RLINE, &[GREEN]);
        c.shade(ROBE, RSHADE, RLINE);
        // The robe's front, and a fold.
        c.line(20, 26, 20, hem as i32 - 1, RSHADE);
        c.set(17, 33, RSHADE);
        c.set(23, 34, RSHADE);

        // The collar: fluffy, ragged at the bottom.
        c.ellipse(20.0, 22.5, 8.0, 2.6, COLLAR);
        for x in (13..28).step_by(2) {
            c.set(x, 25, COLLAR);
        }
        // The hands.
        hand(c, lh.0, lh.1, lopen);
        hand(c, rh.0, rh.1, ropen);

        // The head and ears.
        head_shape(c, hx, hy, lt, rt);
        c.outline(LINE, &[ROBE, RSHADE, RLINE, COLLAR]);
        c.shade(GREEN, GSHADE, LINE);
        c.light(GREEN, GLIGHT, LINE);
        ear_insides(c, hx, hy, lt, rt);
        // A little hair fuzz on top.
        c.set(hx as i32 - 1, 6, GLIGHT);
        c.set(hx as i32 + 1, 6, GLIGHT);
        c.set(hx as i32, 5, GLIGHT);

        // The face.
        let err = f.progress(f.times.error, 1.3);
        let eyes = if let Some(u) = err {
            if u < 0.5 { Look::Cross } else { Look::Squeeze }
        } else if f.red > 0.5 {
            Look::Squeeze
        } else if happy {
            Look::Happy
        } else if copy.is_some_and(|u| u > 0.45 && u < 0.85) {
            Look::Happy
        } else if force {
            if f.blink || f.state == State::Thinking || f.state == State::Planning { Look::Force } else { Look::Open }
        } else if f.state == State::Compacting {
            Look::Squeeze
        } else if f.blink || f.yawn > 0.6 {
            Look::Shut
        } else if tired {
            Look::Heavy
        } else if f.is(&[State::Listening, State::Asking]) {
            Look::Wide
        } else {
            Look::Open
        };
        let look = if force { (1, -1) } else { f.look() };
        let (ex, ey) = (hx as i32, hy as i32 + 1);
        big_eye(c, ex - 4, ey, eyes, look, false);
        big_eye(c, ex + 3, ey, eyes, look, true);
        // Brow wrinkles.
        if eyes != Look::Force {
            c.set(ex - 5, ey - 4, GSHADE);
            c.set(ex - 4, ey - 4, GSHADE);
            c.set(ex + 3, ey - 4, GSHADE);
            c.set(ex + 4, ey - 4, GSHADE);
        }
        // The nose and mouth.
        c.set(ex - 1, ey + 3, GSHADE);
        c.set(ex, ey + 3, GSHADE);
        let my = ey + 5;
        if let Some(u) = copy.filter(|u| *u > 0.45 && *u < 0.85) {
            // Frog legs dangling from his mouth, slurped up bit by bit.
            let k = ((u - 0.45) / 0.4 * 4.0) as i32;
            c.set(ex, my, MOUTH);
            for d in 0..(4 - k).max(0) {
                c.set(ex - 1 - (d % 2), my + 1 + d, GREEN);
                c.set(ex + 1 + (d % 2), my + 1 + d, GREEN);
            }
        } else if happy {
            c.mouth(ex, my - 1, 0.8, 4, 3, MOUTH, EAR);
        } else if f.state == State::Speaking {
            let o = f.mouth();
            if o > 0.15 { c.mouth(ex, my - 1, o * 0.8, 3, 3, MOUTH, 0) } else { c.line(ex - 1, my, ex + 1, my, LINE) }
        } else if err.is_some_and(|u| u < 0.5) || f.state == State::Listening {
            c.mouth(ex, my - 1, 0.5, 3, 2, MOUTH, 0);
        } else {
            c.line(ex - 1, my, ex + 1, my, LINE);
        }

        // The bump of the knob's bonk.
        if err.is_some_and(|u| u > 0.12) {
            c.ellipse(hx + 3.0, 7.0, 1.6, 1.3, GLIGHT);
            c.set(hx as i32 + 3, 5, LINE);
        }

        // Git: the shift knob in his hands.
        if f.state == State::Git && !hurt && !happy {
            let roll = f.wave(4.0, 0.0);
            let k = Sprite::parse(KNOB, KEY);
            c.blit(&k, 18 + (roll * 0.5).round() as i32, 23, roll > 0.0);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        if f.asleep() {
            // The cradle hovers.
            m.dy += 0.03 * (f.t * 1.6).sin() + 0.04;
        }
        if let Some(u) = f.progress(f.times.done, 1.4) {
            // He floats up with the Force.
            m.dy += (u * PI).sin() * 0.12;
        }
        if f.thinking() {
            m.dy += 0.015 * (f.t * 2.0).sin();
        }
        if let Some(u) = f.progress(f.times.error, 0.4) {
            m.sy *= 1.0 - 0.08 * (u * PI).sin();
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let knob = Sprite::parse(KNOB, KEY);
        let rock_a = Sprite::parse(ROCK_A, KEY);
        let rock_b = Sprite::parse(ROCK_B, KEY);
        let hurt = f.hurt();
        let happy = f.happy();
        let force = Char::force(f) && !hurt && !happy && !f.asleep();
        // A glint of the Force: a little cross that comes and goes.
        let glint = |w: &mut World, x: f64, y: f64, k: f64| {
            if (t * 5.0 + k).sin() > -0.2 {
                w.px(x, y, FORCE);
                if (t * 5.0 + k).sin() > 0.6 {
                    w.speck(x - 1.0, y, WHITE);
                    w.speck(x + 1.0, y, WHITE);
                    w.speck(x, y - 1.0, WHITE);
                    w.speck(x, y + 1.0, WHITE);
                }
            }
        };

        // The cradle's hover glow.
        if f.asleep() {
            for k in 0..5 {
                let x = 12.0 + k as f64 * 4.0;
                if (t * 9.0 + k as f64 * 1.7).sin() > -0.3 {
                    w.px(x, 40.5, FORCE);
                }
                if (t * 7.0 + k as f64).sin() > 0.3 {
                    w.px_dim(x, 42.0, FORCE);
                }
            }
        }

        if force {
            let (hx, hy) = (33.0, 18.0 + f.wave(2.0, 0.0) * 0.6);
            if f.thinking() {
                // The shift knob floating over his hand, turning.
                let y = hy - 10.0 + f.wave(2.0, 0.5) * 1.5;
                w.sprite(&knob, hx - 1.5, y);
                glint(w, hx - 3.0, y + 5.0, 0.0);
                glint(w, hx + 3.0, y - 1.0, 2.0);
            } else if f.reading() {
                // Stones orbiting the raised hand.
                for k in 0..3 {
                    let a = t * 1.6 + k as f64 * 2.0 * PI / 3.0;
                    let (x, y) = (hx + a.cos() * 7.0, hy - 6.0 + a.sin() * 4.0);
                    w.sprite(if k == 1 { &rock_b } else { &rock_a }, x - 1.5, y - 1.0);
                }
                glint(w, hx, hy - 6.0, 1.0);
            } else {
                // Editing: one stone tumbling in front of his hand.
                let y = hy - 7.0 + f.wave(3.0, 0.0) * 2.0;
                w.sprite(&rock_a, hx - 1.5, y);
                glint(w, hx + 3.0, y, 0.5);
            }
            // Sparkles drifting up from the hand.
            for k in 0..3 {
                let u = (t * 0.7 + k as f64 / 3.0) % 1.0;
                w.speck(hx - 1.0 + k as f64, hy - 2.0 - u * 6.0, FORCE);
            }
        }

        // Delegating: a Force push, waves going out both sides.
        if f.state == State::Delegating {
            for k in 0..3 {
                let u = (t * 1.2 + k as f64 / 3.0) % 1.0;
                for s in [-1.0, 1.0] {
                    let x = 20.0 + s * (17.0 + u * 10.0);
                    for d in -2..=2 {
                        let y = 25.0 + d as f64 * 1.5;
                        if u < 0.85 {
                            w.px_dim(x - s * (d as f64).abs() * 0.6, y, FORCE);
                        }
                    }
                }
            }
        }

        // Running: dust from his little feet.
        if f.is(&[State::Running, State::Testing]) {
            for k in 0..3 {
                let u = (t * 2.0 + k as f64 / 3.0) % 1.0;
                w.px_dim(10.0 - u * 6.0, 38.0 - u * 2.0, COLLAR);
            }
        }

        // A key: a glint by a hand.
        if f.within(f.times.key, 0.2) {
            glint(w, if f.times.key as u64 % 2 == 0 { 9.0 } else { 31.0 }, 26.0, PI / 2.0);
        }

        // Done: he floats, stones rise all around him, a ring of the Force.
        if let Some(u) = f.progress(f.times.done, 1.4) {
            for k in 0..6 {
                let x = -4.0 + k as f64 * 9.6;
                let rise = (u * 1.6 - k as f64 * 0.05).clamp(0.0, 1.0);
                let y = 40.0 - rise * (14.0 + (k % 3) as f64 * 5.0) + (t * 3.0 + k as f64).sin();
                w.sprite(if k % 2 == 0 { &rock_a } else { &rock_b }, x, y);
            }
            let r = u * 30.0;
            if u < 0.8 {
                for k in 0..16 {
                    let a = k as f64 * PI / 8.0;
                    w.px(20.0 + a.cos() * r, 22.0 + a.sin() * r * 0.7, if k % 2 == 0 { FORCE } else { WHITE });
                }
            }
        }

        // Error: the shift knob falls on his head — bonk — and bounces away.
        if let Some(u) = f.progress(f.times.error, 1.6) {
            let (x, y) = if u < 0.12 {
                (22.0, -20.0 + u / 0.12 * 23.0)
            } else if u < 0.6 {
                let v = (u - 0.12) / 0.48;
                (22.0 + v * 16.0, 3.0 - (v * PI).sin() * 10.0 + v * 33.0)
            } else {
                let v = (u - 0.6) / 0.4;
                (38.0 + v * 10.0, 36.0)
            };
            w.sprite(&knob, x, y);
            if (0.1..0.25).contains(&u) {
                // The bonk.
                for (dx, dy) in [(-3.0, 0.0), (5.0, 0.0), (-2.0, -2.0), (4.0, -2.0), (1.0, -3.0)] {
                    w.px(22.0 + dx, 3.0 + dy, WHITE);
                }
            }
        }

        // Copying: a frog hops in, he grabs it and eats it; a happy burp.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let frog = Sprite::parse(FROG, KEY);
            if u < 0.3 {
                let v = u / 0.3;
                let x = 48.0 - v * 17.0;
                let hop = ((v * 3.0 * PI).sin()).abs() * 4.0;
                w.sprite(&frog, x, 33.0 - hop);
            } else if u < 0.45 {
                let v = (u - 0.3) / 0.15;
                let (x, y) = lerp((31.0, 25.0), (21.0, 16.0), v.min(1.0));
                w.sprite(&frog, x - 3.0, y);
            } else if u > 0.85 {
                let r = 1.0 + (u - 0.85) * 12.0;
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0;
                    w.speck(26.0 + a.cos() * r, 16.0 - (u - 0.85) * 20.0 + a.sin() * r, FORCE);
                }
            }
        }

        // Little context left: his ears droop and he yawns a slow sigh.
        if f.context > 0.85 && !f.asleep() {
            let u = (t * 0.5) % 1.0;
            w.speck(18.0 - u * 3.0, 21.0 - u * 6.0, COLLAR);
            w.speck(19.0 + u * 2.0, 22.0 - u * 7.0, COLLAR);
        }
    }
}
