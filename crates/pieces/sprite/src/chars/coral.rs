//! Coral: a curious little octopus who changes color with her mood, like camouflage. Drawn from
//! shapes every frame (a round mantle, big eyes, six tentacles that ripple one after another),
//! so everything on her moves. What she does:
//! - waiting: her tentacles ripple one by one, the spots on her skin breathe in the state's
//!   color (her camouflage: whatever Iris is doing shows through her skin);
//! - listening: two tentacles cupped by her head like ears, eyes wide;
//! - thinking: a tentacle tip on her chin, her color slowly shifting, spots pulsing;
//! - speaking: the mouth with the voice, little bubbles coming out of it;
//! - reading, searching, on the web: she holds a magnifying glass up to one big eye;
//! - editing, running, testing, git: she holds several things at once — a pencil, a wrench, a
//!   magnifier, a key — each in its own tentacle, the one in use busy;
//! - typing: her front tentacles tap like on a keyboard;
//! - done: she juggles three balls, flashing through her colors;
//! - error: she squirts a cloud of black ink and goes dark herself for a moment;
//! - a key: a bubble pops off a tentacle tip; copying: a shell opens beside her with a pearl;
//! - asking: one tentacle curls into a "?"; music: tentacles dance on the beat, left and right;
//! - asleep: tentacles curled up under her, lavender and calm; offline: pale, see-through;
//! - booting: she pops out of a big bubble; little context left: droopy tentacles, sleepy lids.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Coral";
pub const SCENE: &str = "ocean";
pub const DESCRIPTION: &str = "Coral, una pulpita curiosa que cambia de color con su animo: sostiene lapiz, llave y lupa a la vez al trabajar, hace malabares al terminar y suelta tinta con los errores";

const LINE: u8 = 1;
const SKIN: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const EYE: u8 = 5;
const WHITE: u8 = 6;
const SUCK: u8 = 7;
const INK: u8 = 8;
const INK2: u8 = 9;
const GOLD: u8 = 10;
const CYAN: u8 = 11;
const RED: u8 = 12;
const MOUTH: u8 = 13;
const PALE: u8 = 14;
const GRAY: u8 = 15;
const GREEN: u8 = 16;

/// A tentacle: its five points, from the base to the tip.
type Arm = [(f64, f64); 5];

/// What a tentacle tip is holding.
#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Pencil,
    Wrench,
    Lens,
    Key,
}

pub struct Char {
    next_glance: f64,
    glance: f64,
}

/// A wave in 0..1 for tentacle `i`: they go one after another.
fn ripple(f: &Frame, i: usize, speed: f64) -> f64 {
    (f.t * speed - i as f64 * 0.9).sin()
}

impl Char {
    /// Where her six tentacles are this frame (left to right).
    fn arms(&self, f: &Frame) -> [Arm; 6] {
        self.pose(f).0
    }

    /// Her tentacles, and which of them are raised (drawn in front of her).
    fn pose(&self, f: &Frame) -> ([Arm; 6], [bool; 6]) {
        let bases = [(11.0, 22.0), (14.0, 24.0), (17.0, 25.0), (21.0, 25.0), (24.0, 24.0), (27.0, 22.0)];
        // Their resting directions: spread out like a skirt.
        let dirs = [2.45, 2.0, 1.72, 1.42, 1.14, 0.69];
        let hurt = f.hurt();
        let happy = f.happy();
        let asleep = f.asleep();
        let tired = f.context > 0.85 && !asleep;
        let speed = if f.groove > 0.1 { 7.0 } else if f.working() { 5.0 } else if f.thinking() { 1.4 } else { 2.4 };
        let mut out = [[(0.0, 0.0); 5]; 6];
        for i in 0..6 {
            let side = if i < 3 { -1.0 } else { 1.0 };
            let mut a = dirs[i];
            let mut len = 3.4;
            let mut amp = 0.22;
            let mut curl = side * -0.25; // tips curl outward
            if asleep {
                // Curled up under her.
                a = PI / 2.0 + side * 0.9;
                len = 2.6;
                amp = 0.05;
                curl = side * -0.9;
            } else if tired {
                a = PI / 2.0 + (a - PI / 2.0) * 1.4;
                amp = 0.08;
                len = 3.3;
            } else if hurt {
                a = PI / 2.0 + (a - PI / 2.0) * 1.7;
                amp = 0.45;
            }
            if f.groove > 0.1 {
                amp += 0.25 * f.groove;
            }
            let mut p = bases[i];
            out[i][0] = p;
            for s in 1..5 {
                let w = ripple(f, i, speed) * amp * (0.6 + s as f64 * 0.25);
                let ang = a + w + curl * (s as f64 - 1.0) * 0.5;
                p = (p.0 + ang.cos() * len, p.1 + ang.sin() * len);
                out[i][s] = p;
            }
        }
        // Tentacles raised for something: (index, direction of the base, its curl, how long).
        let mut up = [false; 6];
        let mut raise = |i: usize, a: f64, curl: f64, len: f64, wob: f64| {
            up[i] = true;
            let mut p = out[i][0];
            for s in 1..5 {
                let ang = a + curl * (s as f64 - 1.0) + ripple(f, i, 4.0) * wob;
                p = (p.0 + ang.cos() * len, p.1 + ang.sin() * len);
                out[i][s] = p;
            }
        };
        if asleep {
            return (out, [false; 6]);
        }
        if happy {
            // Juggling: the outer ones up, tossing in turn.
            let toss = f.wave(12.0, 0.0);
            raise(0, -2.3 + toss * 0.25, 0.25, 3.4, 0.0);
            raise(5, -0.85 - toss * 0.25, -0.25, 3.4, 0.0);
        } else if hurt {
            raise(0, -2.6, 0.35, 3.2, 0.3);
            raise(5, -0.55, -0.35, 3.2, 0.3);
        } else if f.is(&[State::Listening, State::Transcribing]) {
            // Cupped by her head, like ears.
            raise(0, -1.9, -0.75, 3.2, 0.05);
            raise(5, -1.25, 0.75, 3.2, 0.05);
        } else if f.thinking() {
            // Out and up, the tip tapping her temple.
            raise(5, -0.5, -0.55, 3.0, 0.12);
        } else if f.state == State::Asking {
            // A "?" curl, up and waving.
            raise(5, -1.35, -0.62, 3.0, 0.1);
        } else if f.reading() {
            // Out and up, holding the magnifier to her right eye.
            raise(5, -0.5, -0.5, 3.1, 0.04);
        } else if f.working() {
            // Several tools at once.
            raise(0, -2.5, 0.1, 3.3, 0.12);
            raise(5, -0.65, -0.1, 3.3, 0.12);
            raise(1, 2.9, -0.2, 3.3, 0.1);
            raise(4, 0.25, 0.2, 3.3, 0.1);
        } else if f.state == State::Typing || f.within(f.times.key, 0.2) {
            for i in [2, 3] {
                let tap = ((f.t * 14.0 + i as f64 * PI).sin()).max(0.0);
                let dir = if i == 2 { 1.9 } else { 1.24 };
                raise(i, dir - tap * 0.35 * if i == 2 { -1.0 } else { 1.0 }, 0.0, 3.1, 0.0);
            }
        } else if f.groove > 0.1 {
            // Dancing: the outer ones up and down on the beat, left and right in turn.
            let b = f.wave(PI * 2.2, 0.0);
            raise(0, -2.6 + b * 0.7, 0.3, 3.3, 0.0);
            raise(5, -0.55 + b * 0.7, -0.3, 3.3, 0.0);
        }
        (out, up)
    }

    /// The tools she holds this frame, with the tentacle holding each.
    fn tools(&self, f: &Frame) -> Vec<(usize, Tool)> {
        if f.hurt() || f.happy() || f.asleep() {
            return Vec::new();
        }
        if f.reading() {
            return vec![(5, Tool::Lens)];
        }
        if f.working() {
            let fourth = if f.state == State::Git { Tool::Key } else { Tool::Lens };
            return vec![(0, Tool::Pencil), (5, Tool::Wrench), (1, fourth), (4, if f.state == State::Git { Tool::Pencil } else { Tool::Key })];
        }
        Vec::new()
    }

    /// The tool in use right now: the one shaking (by state).
    fn busy(&self, f: &Frame) -> Option<Tool> {
        match f.state {
            State::Editing => Some(Tool::Pencil),
            State::Running | State::Compacting => Some(Tool::Wrench),
            State::Testing => Some(Tool::Lens),
            State::Git => Some(Tool::Key),
            _ => None,
        }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#b8327a", "#ff7d8a", "#e4566e", "#ffb7b0", "#140a1a", "#ffffff", "#ffd8c8", "#4a3c72", "#8c7ac4", "#ffd23f", "#7fe8ff", "#ff4f4f",
        "#8a1f3c", "#cfc8e6", "#a3adbf", "#6fe08a",
    ];
    const W: usize = 38;
    const H: usize = 38;

    fn new(seed: u64) -> Self {
        Char { next_glance: 2.0 + hash(seed) * 3.0, glance: -99.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_glance {
            self.glance = f.t;
            self.next_glance = f.t + 3.0 + f.rand(f.t as u64 + 11) * 4.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let hurt = f.hurt();
        let happy = f.happy();
        let asleep = f.asleep();
        let (arms, up) = self.pose(f);

        // Tentacles behind first (the outer two), then the mantle, then the front ones.
        let tentacle = |c: &mut Canvas, a: &Arm| {
            for s in 0..4 {
                let w = 3.6 - s as f64 * 0.65;
                c.thick(a[s], a[s + 1], w, SKIN);
            }
        };
        for i in [0, 5, 1, 4] {
            if !up[i] {
                tentacle(c, &arms[i]);
            }
        }
        // The mantle: a big round head, a little taller than wide, slightly bulging back.
        let squash = if asleep { 1.0 } else { 0.0 };
        c.ellipse(19.0, 13.0 + squash, 10.5, 10.0 - squash, SKIN);
        c.ellipse(19.0, 20.5, 8.5, 4.5, SKIN);
        for i in [2, 3] {
            if !up[i] {
                tentacle(c, &arms[i]);
            }
        }
        c.outline(LINE, &[]);
        // Raised tentacles go in front, each with its own line around it.
        if up.iter().any(|u| *u) {
            for i in 0..6 {
                if up[i] {
                    for s in 0..4 {
                        c.thick(arms[i][s], arms[i][s + 1], 5.4 - s as f64 * 0.65, LINE);
                    }
                }
            }
            for i in 0..6 {
                if up[i] {
                    tentacle(c, &arms[i]);
                }
            }
            c.outline(LINE, &[]);
        }
        c.shade(SKIN, SHADE, LINE);
        c.light(SKIN, LIGHT, LINE);
        // A shine on her mantle.
        c.ellipse(14.0, 6.5, 2.0, 1.4, LIGHT);
        c.set(13, 6, WHITE);

        // Suckers: little pale dots along the front tentacles.
        if !asleep {
            for i in 1..5 {
                for s in 3..4 {
                    let (x, y) = arms[i][s];
                    c.set(x.round() as i32, y.round() as i32, SUCK);
                }
            }
        }

        // The face: big eyes with whites.
        let mut eyes = if f.thinking() { Eyes::Open } else { f.eyes() };
        let tired = f.context > 0.85 && !asleep;
        let mut look = f.look();
        if f.within(self.glance, 0.6) {
            look.0 = if (self.glance as i64) % 2 == 0 { -1 } else { 1 };
        }
        if f.reading() {
            look = (1, 0);
            if eyes == Eyes::Squint {
                eyes = Eyes::Open;
            }
        }
        let open = matches!(eyes, Eyes::Open | Eyes::Wide);
        for (ex, right) in [(14.0, false), (24.0, true)] {
            if open {
                let tall = if eyes == Eyes::Wide { 3.4 } else { 3.0 };
                c.ellipse(ex, 14.0, 2.6, tall, WHITE);
                if tired {
                    c.rect(ex as i32 - 3, 10, 6, 3, SKIN);
                }
                let px = ex as i32 - 1 + look.0;
                let py = 14 + look.1 - 1;
                c.rect(px, py, 2, 3, EYE);
                c.set(px + if right { 1 } else { 0 }, py, WHITE);
            } else {
                c.eye(ex as i32 - 1, 13, 3, 3, eyes, look, LINE, 0, 0, right);
            }
        }
        // Cheeks.
        c.ellipse(10.5, 18.0, 1.6, 1.0, SUCK);
        c.ellipse(27.5, 18.0, 1.6, 1.0, SUCK);
        // The mouth: a tiny beak-like "u", open with the voice, a big smile when done.
        let m = f.mouth();
        if happy {
            c.mouth(19, 18, 0.9, 5, 3, MOUTH, RED);
        } else if hurt {
            c.mouth(19, 18, 0.5, 3, 3, MOUTH, 0);
        } else if m > 0.08 && !f.reading() {
            c.mouth(19, 18, m, 3, 3, MOUTH, RED);
        } else if f.reading() {
            c.ellipse(19.5, 19.0, 1.0, 1.0, MOUTH);
        } else {
            c.smile(19, 18, 3, LINE);
        }

        // Her colors: the mood. Asleep, lavender; offline, pale; done, flashing through her
        // colors; thinking, slowly blushing to violet and back; an error, inked herself.
        let mood = |c: &mut Canvas, base: u8, shade: u8, light: u8| {
            c.swap(SKIN, base);
            c.swap(SHADE, shade);
            c.swap(LIGHT, light);
        };
        if f.state == State::Offline {
            mood(c, PALE, INK2, WHITE);
            // See-through: every other pixel of her goes.
            for y in 0..Self::H as i32 {
                for x in 0..Self::W as i32 {
                    if (x + y) % 2 == 0 && matches!(c.get(x, y), PALE | INK2) {
                        c.clear(x, y);
                    }
                }
            }
        } else if asleep {
            mood(c, INK2, INK, PALE);
        } else if let Some(u) = f.progress(f.times.error, 1.4) {
            if u < 0.75 {
                mood(c, INK, INK, INK2);
            }
        } else if happy {
            match ((f.t * 8.0) as i32).rem_euclid(4) {
                0 => mood(c, GOLD, SHADE, WHITE),
                1 => mood(c, CYAN, INK2, WHITE),
                2 => mood(c, GREEN, INK2, WHITE),
                _ => {}
            }
        } else if f.thinking() && f.wave(1.2, 0.0) > 0.3 {
            mood(c, INK2, INK, LIGHT);
        } else if f.reading() && f.wave(2.0, 0.0) > 0.5 {
            mood(c, CYAN, INK2, WHITE);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // She bobs, as if floating in water, rather than breathing.
        if !f.asleep() {
            m.dy += 0.03 * f.wave(1.6, 0.0);
        }
        if let Some(u) = f.progress(f.times.error, 0.4) {
            // Jet backwards from the ink.
            m.dy += 0.25 * (u * PI).sin();
        }
        if f.state == State::Booting {
            // Out of a bubble: from small to her size.
            let k = (f.st / 1.0).min(1.0);
            m.sx *= 0.4 + 0.6 * k;
            m.sy *= 0.4 + 0.6 * k;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let arms = self.arms(f);
        let asleep = f.asleep();

        // Her camouflage: spots on her mantle in the state's color, breathing one by one.
        if !asleep && !f.hurt() && !f.happy() {
            let spots = [(13.0, 4.0), (19.0, 3.0), (25.0, 5.0), (11.0, 9.0), (28.0, 10.0), (17.0, 7.0), (22.0, 8.0)];
            let speed = if f.thinking() { 4.0 } else { 1.6 };
            for (k, (x, y)) in spots.iter().enumerate() {
                if (t * speed - k as f64 * 0.8).sin() > -0.2 {
                    w.px_state(*x, *y);
                    if k % 2 == 0 {
                        w.px_state(x + 1.0, *y);
                    }
                }
            }
        }

        // Tools in her tentacles.
        let busy = self.busy(f);
        for (i, tool) in self.tools(f) {
            let tip = arms[i][4];
            let jig = if Some(tool) == busy { f.wave(18.0, i as f64).round() } else { 0.0 };
            let (x, y) = (tip.0.round() + jig, tip.1.round());
            match tool {
                Tool::Pencil => {
                    // Diagonal: tip down-left, eraser up-right.
                    w.px(x - 1.0, y + 1.0, GRAY);
                    for k in 0..4 {
                        w.px(x + k as f64, y - k as f64, GOLD);
                    }
                    w.px(x + 4.0, y - 4.0, RED);
                }
                Tool::Wrench => {
                    for k in 0..4 {
                        w.px(x + k as f64 * 0.5, y - k as f64, GRAY);
                    }
                    w.px(x + 1.0, y - 5.0, GRAY);
                    w.px(x + 3.0, y - 5.0, GRAY);
                    w.px(x + 1.0, y - 4.0, GRAY);
                    w.px(x + 3.0, y - 4.0, GRAY);
                }
                Tool::Lens => {
                    let (cx, cy) = if f.reading() { (25.0, 14.0) } else { (x, y - 3.0) };
                    if f.reading() {
                        for k in 1..4 {
                            let v = k as f64 / 4.0;
                            w.px((x + (cx + 2.0 - x) * v).round(), (y + (cy - 1.0 - y) * v).round(), GRAY);
                        }
                    }
                    for k in 0..10 {
                        let a = k as f64 * PI / 5.0;
                        w.px((cx + a.cos() * 2.5).round(), (cy + a.sin() * 2.5).round(), GOLD);
                    }
                    w.px_dim(cx, cy, CYAN);
                    w.px_dim(cx - 1.0, cy, CYAN);
                    w.px(cx - 1.0, cy - 1.0, WHITE);
                    w.px(x, y, GOLD);
                }
                Tool::Key => {
                    w.px(x, y - 3.0, GOLD);
                    w.px(x + 1.0, y - 3.0, GOLD);
                    w.px(x, y - 4.0, GOLD);
                    w.px(x + 1.0, y - 4.0, GOLD);
                    w.px(x - 1.0, y - 3.5, GOLD);
                    w.px(x + 2.0, y - 3.5, GOLD);
                    for k in 0..3 {
                        w.px(x + 0.5, y - k as f64, GOLD);
                    }
                    w.px(x + 1.5, y, GOLD);
                    w.px(x + 1.5, y - 1.0, GOLD);
                }
            }
        }

        // Asking: the "?" dot under her curled tentacle tip.
        if f.state == State::Asking && !f.hurt() {
            let tip = arms[5][4];
            if f.wave(5.0, 0.0) > -0.5 {
                w.px(tip.0.round() - 1.0, tip.1.round() + 4.0, SUCK);
            }
        }

        // Speaking: bubbles out of her mouth.
        if f.state == State::Speaking && f.voice > 0.03 {
            for k in 0..3 {
                let u = ((t * 0.9 + k as f64 / 3.0) % 1.0).max(0.0);
                let (x, y) = (21.0 + u * 6.0 + (u * 9.0 + k as f64).sin(), 18.0 - u * 16.0);
                w.px(x, y, CYAN);
                if k == 1 {
                    w.px(x + 1.0, y, CYAN);
                }
            }
        }

        // A key: a bubble pops off a tip.
        if let Some(u) = f.progress(f.times.key, 0.35) {
            let tip = arms[if (f.times.key * 3.0) as i64 % 2 == 0 { 2 } else { 3 }][4];
            let y = tip.1 - 2.0 - u * 5.0;
            if u < 0.7 {
                w.px(tip.0, y, CYAN);
            } else {
                w.px(tip.0 - 1.0, y, WHITE);
                w.px(tip.0 + 1.0, y, WHITE);
            }
        }

        // Done: juggling three balls over her raised tentacles.
        if let Some(u) = f.progress(f.times.done, 1.6) {
            let (l, r) = (arms[0][4], arms[5][4]);
            for (k, col) in [RED, GOLD, GREEN].into_iter().enumerate() {
                let ph = (t * 2.6 + k as f64 / 3.0) % 1.0;
                // Half the loop flying left to right high, half going back low (a cascade).
                let (x, y) = if ph < 0.5 {
                    let v = ph * 2.0;
                    (l.0 + (r.0 - l.0) * v, l.1.min(r.1) - 2.0 - (v * PI).sin() * 12.0)
                } else {
                    let v = (ph - 0.5) * 2.0;
                    (r.0 + (l.0 - r.0) * v, r.1.min(l.1) - 1.0 - (v * PI).sin() * 3.0)
                };
                w.px(x, y, col);
                w.px(x + 1.0, y, col);
                w.px(x, y + 1.0, col);
                w.px(x + 1.0, y + 1.0, WHITE);
            }
            // Sparkles at the start.
            if u < 0.3 {
                let r = u * 50.0;
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0;
                    w.px(19.0 + a.cos() * r, 16.0 + a.sin() * r * 0.8, if k % 2 == 0 { GOLD } else { CYAN });
                }
            }
        }

        // Error: a cloud of black ink, puffing out under and around her and fading.
        if let Some(u) = f.progress(f.times.error, 1.6) {
            let spread = 6.0 + u * 16.0;
            for k in 0..12 {
                let a = PI * 0.05 + k as f64 * PI * 0.9 / 11.0 + f.rand(k) * 0.25;
                let d = spread * (0.6 + f.rand(k + 40) * 0.5);
                let (x, y) = ((19.0 + a.cos() * d * 1.3).round(), (27.0 + a.sin() * d * 0.7).round());
                if u < 0.6 || (k + (t * 10.0) as u64) % 2 == 0 {
                    // A puff: a dark blob with a lighter rim.
                    w.px(x, y, INK);
                    w.px(x + 1.0, y, INK);
                    w.px(x, y + 1.0, INK);
                    w.px(x + 1.0, y + 1.0, INK);
                    w.px(x - 1.0, y, INK2);
                    w.px(x, y - 1.0, INK2);
                    w.px(x + 1.0, y - 1.0, INK2);
                }
            }
        }

        // Copying: a shell opens beside her, a pearl inside.
        if let Some(u) = f.progress(f.times.copy, 2.4) {
            let rise = (u * 5.0).min(1.0);
            let (sx, sy) = (31.0, 32.0 - rise * 4.0);
            let gape = ((u - 0.15) * 6.0).clamp(0.0, 1.0) * 3.0;
            // The bottom shell: a pink fan.
            for x in 0..7 {
                let d = (x as f64 - 3.0).abs();
                let h = 3.0 - d * 0.6;
                for y in 0..h.ceil() as i32 {
                    w.px(sx + x as f64, sy + y as f64, if x % 2 == 0 { SUCK } else { LIGHT });
                }
            }
            // The lid, opening.
            for x in 0..7 {
                let d = (x as f64 - 3.0).abs();
                w.px(sx + x as f64, sy - 1.0 - gape + d * 0.3 * (gape / 3.0), if x % 2 == 0 { SUCK } else { LIGHT });
                w.px(sx + x as f64, sy - 2.0 - gape + d * 0.6 * (gape / 3.0), SUCK);
            }
            if gape > 1.0 {
                w.px(sx + 3.0, sy - 1.0, WHITE);
                w.px(sx + 2.0, sy - 1.0, PALE);
                if f.wave(10.0, 0.0) > 0.3 {
                    w.px(sx + 4.0, sy - 3.0, WHITE);
                }
            }
        }

        // Booting: the bubble she comes out of, popping.
        if f.state == State::Booting && f.st < 1.4 {
            let r = 14.0 + f.st * 6.0;
            let n = 20;
            for k in 0..n {
                let a = k as f64 * 2.0 * PI / n as f64;
                if f.st < 0.9 || k % 3 == 0 {
                    w.px(19.0 + a.cos() * r, 19.0 + a.sin() * r, if k % 5 == 0 { WHITE } else { CYAN });
                }
            }
        }

        // Asleep: a tiny bubble rising from her, now and then.
        if f.state == State::Sleeping {
            let u = (t * 0.35) % 1.0;
            w.px(26.0 + (u * 8.0).sin(), 16.0 - u * 14.0, CYAN);
        }

        // Little context left: an ink drip from a tentacle, tired.
        if f.context > 0.85 && !asleep {
            let tip = arms[1][4];
            let u = (t * 0.7) % 1.0;
            w.px(tip.0, tip.1 + 1.0 + u * 4.0, INK2);
        }
    }
}
