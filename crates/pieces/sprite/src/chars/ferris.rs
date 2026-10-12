//! Ferris, the Rust crab: drawn from shapes every frame (a spiky shell, six legs, two big
//! pincers and two eyes on stalks), so everything on it moves. What it does:
//! - waiting: the eye stalks sway on their own, a pincer snips now and then; music: it dances
//!   sideways, snapping on the beat;
//! - listening: stalks up tall, eyes wide; asking: one stalk bends, a pincer waves;
//! - thinking: the stalks circle, a pincer at its mouth, "fn" being typed over its head;
//! - speaking: the mouth with the voice and bubbles rising from it;
//! - reading, searching, on the web: crates flying in, caught by its pincers;
//! - editing: snip-snip, sparks at the pincers; running, testing: it scuttles sideways, a gear
//!   turning over it (tests tick green); git: a branch graph growing beside it;
//! - done: both pincers up, snapping, bursts at the tips;
//! - error: the borrow checker: a padlock and E0502 over it, it turns red with anger, steam;
//! - a key: a pincer clicks; copying: `.clone()` — a little Ferris pops out beside it;
//! - asleep: flat on the sand, stalks folded, big slow bubbles; offline: pale;
//! - booting: the stalks grow out; little context left: stalks droop, a sweat drop.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Ferris";
pub const SCENE: &str = "ocean";
pub const DESCRIPTION: &str = "Ferris, el cangrejo de Rust: camina de lado al correr, chasquea las pinzas al terminar, pelea con el borrow checker en los errores y se clona al copiar";

const LINE: u8 = 1;
const ORANGE: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const EYE: u8 = 5;
const WHITE: u8 = 6;
const MOUTH: u8 = 7;
const BLUSH: u8 = 8;
const BUBBLE: u8 = 9;
const GEAR: u8 = 10;
const GOLD: u8 = 11;
const RED: u8 = 12;
const PALE: u8 = 13;
const GREEN: u8 = 14;
const CRATE: u8 = 15;
const STEEL: u8 = 16;

/// A 3×5 pixel font, one row per 3 bits (top row first).
fn glyph(ch: char) -> [u8; 5] {
    match ch {
        'E' => [7, 4, 6, 4, 7],
        '0' => [7, 5, 5, 5, 7],
        '5' => [7, 4, 7, 1, 7],
        '2' => [7, 1, 7, 4, 7],
        'f' => [3, 4, 7, 4, 4],
        'n' => [0, 6, 5, 5, 5],
        '_' => [0, 0, 0, 0, 7],
        '(' => [1, 2, 2, 2, 1],
        ')' => [4, 2, 2, 2, 4],
        _ => [0; 5],
    }
}

fn text(w: &mut World, s: &str, x: f64, y: f64, col: u8) {
    for (i, ch) in s.chars().enumerate() {
        let g = glyph(ch);
        for (row, bits) in g.iter().enumerate() {
            for b in 0..3 {
                if bits & (4 >> b) != 0 {
                    w.px(x + i as f64 * 4.0 + b as f64, y + row as f64, col);
                }
            }
        }
    }
}

/// Clears the pixels inside a triangle (a notch cut into a shape).
fn cut(c: &mut Canvas, a: (f64, f64), b: (f64, f64), d: (f64, f64)) {
    let (x0, x1) = (a.0.min(b.0).min(d.0).floor() as i32, a.0.max(b.0).max(d.0).ceil() as i32);
    let (y0, y1) = (a.1.min(b.1).min(d.1).floor() as i32, a.1.max(b.1).max(d.1).ceil() as i32);
    let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = (x as f64 + 0.5, y as f64 + 0.5);
            let (d1, d2, d3) = (side(a, b, p), side(b, d, p), side(d, a, p));
            if !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0)) {
                c.clear(x, y);
            }
        }
    }
}

const CLONE: &str = "
....w...w....
....o...o....
o.o.o...o.o.o
.o..ooooo..o.
..ooooooooo..
.ooooooooooo.
..ooooooooo..
.l.l.l.l.l.l.";

const LOCK: &str = "
..sss..
.s...s.
.s...s.
ggggggg
ggg.ggg
ggg.ggg
ggggggg";

pub struct Char {
    /// The last idle snip, which pincer, and the next one.
    snip: f64,
    snip_left: bool,
    next_snip: f64,
    clone: Sprite,
    lock: Sprite,
}

impl Char {
    /// How open a pincer is (0 shut, 1 wide).
    fn open(&self, f: &Frame, left: bool) -> f64 {
        let t = f.t;
        if f.asleep() {
            return 0.1;
        }
        if f.happy() {
            return 0.5 + 0.5 * (t * 28.0 + if left { 0.0 } else { 1.5 }).sin();
        }
        if f.hurt() {
            return 0.6 + 0.4 * (t * 40.0).sin();
        }
        if f.within(f.times.nope, 0.5) {
            return 0.0;
        }
        if f.within(f.times.key, 0.15) && left == (f.times.key as u64 % 2 == 0) {
            return 0.0;
        }
        if f.state == State::Editing {
            return 0.5 + 0.5 * (t * 18.0 + if left { 0.0 } else { PI }).sin();
        }
        if f.groove > 0.1 {
            return 0.5 + 0.5 * (t * PI * 2.2 + if left { 0.0 } else { PI }).sin();
        }
        if let Some(u) = f.progress(self.snip, 0.5) {
            if left == self.snip_left {
                return 0.5 * (1.0 - (u * PI * 2.0).sin().abs());
            }
        }
        0.5
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#c4500f", "#f74c00", "#b83000", "#ffae70", "#140a08", "#ffffff", "#5a1408", "#ff8f8f", "#8fd6ff", "#d4dbe8", "#ffd23a", "#ff3434",
        "#e2cfc0", "#5ef08a", "#c8894a", "#8a94a8",
    ];
    const W: usize = 40;
    const H: usize = 36;

    fn new(seed: u64) -> Self {
        Char {
            snip: -99.0,
            snip_left: true,
            next_snip: 1.5 + hash(seed) * 2.0,
            clone: Sprite::parse(CLONE, "o.lw"),
            lock: Sprite::parse(LOCK, "sg"),
        }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_snip {
            self.snip = f.t;
            self.snip_left = f.rand(f.t as u64 + 11) < 0.5;
            self.next_snip = f.t + 2.0 + f.rand(f.t as u64 + 5) * 3.5;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let hurt = f.hurt();
        let happy = f.happy();
        let asleep = f.asleep();
        let running = f.is(&[State::Running, State::Testing]);
        let tired = f.context > 0.85 && !asleep;
        let base = 35.0;

        // The body sits lower asleep (flat on the sand).
        let (bx, by) = (20.0, if asleep { 27.0 } else { 23.5 });
        let (rx, ry) = (12.0, if asleep { 5.5 } else { 6.0 });

        // Legs (behind): three a side, pointy, scuttling when running or dancing.
        let scuttle = running || f.groove > 0.1;
        for k in 0..3 {
            let kf = k as f64;
            for side in [-1.0, 1.0] {
                let lift = if scuttle { (t * 16.0 + kf * 2.1 + if side > 0.0 { PI } else { 0.0 }).sin().max(0.0) * 2.0 } else { 0.0 };
                let root = (bx + side * (5.0 + kf * 2.8), by + 3.5);
                let knee = (bx + side * (9.5 + kf * 3.6), by + 6.0 - kf * 0.8 - lift);
                let foot = (bx + side * (10.5 + kf * 4.0), base - lift * 0.6);
                let (knee, foot) = if asleep { ((knee.0 + side * 2.0, base - 2.0), (foot.0 + side * 2.5, base)) } else { (knee, foot) };
                c.thick(root, knee, 1.3, ORANGE);
                c.line(knee.0.round() as i32, knee.1.round() as i32, foot.0.round() as i32, foot.1.round() as i32, ORANGE);
            }
        }

        // The eye stalks: where the eyes are (sway, tall listening, circling thinking, folded
        // asleep, stiff angry, drooping tired, growing as it boots).
        let grow = if f.state == State::Booting { ((f.st - 0.5) / 0.9).clamp(0.0, 1.0) } else { 1.0 };
        let sway = |ph: f64| f.wave(1.3, ph) * 0.8;
        let top = by - 11.5;
        let (mut le, mut re) = ((15.5 + sway(0.0), top), (24.5 + sway(2.0), top));
        if f.is(&[State::Listening, State::Transcribing]) {
            le = (15.0, top - 3.0);
            re = (25.0, top - 3.0);
        } else if f.thinking() {
            le = (15.5 + (t * 2.0).cos() * 1.5, top + (t * 2.0).sin());
            re = (24.5 - (t * 2.0).cos() * 1.5, top + (t * 2.0).sin());
        } else if f.state == State::Asking {
            re = (27.5, top + 1.5);
        } else if hurt {
            le = (15.5, top - 1.0);
            re = (24.5, top - 1.0);
        } else if tired {
            le = (13.0, top + 3.0);
            re = (27.0, top + 3.0);
        } else if running {
            let lag = (t * 12.0).sin() * 0.7;
            le = (16.5 + lag, top);
            re = (25.5 + lag, top);
        }
        if f.groove > 0.1 {
            let b = f.wave(PI * 2.2, 0.0) * 1.8 * f.groove;
            le.0 += b;
            re.0 += b;
        }
        let (lb, rb) = ((16.5, by - 4.0), (23.5, by - 4.0));
        if asleep {
            le = (14.0, by - 6.5);
            re = (26.0, by - 6.5);
        }
        le = (lb.0 + (le.0 - lb.0) * grow, lb.1 + (le.1 - lb.1) * grow);
        re = (rb.0 + (re.0 - rb.0) * grow, rb.1 + (re.1 - rb.1) * grow);
        c.thick(lb, le, 1.6, ORANGE);
        c.thick(rb, re, 1.6, ORANGE);

        // The pincers: where they are, and arms to them (shoulder, elbow, pincer).
        let (mut lc, mut rc) = ((5.0, 13.0), (35.0, 13.0));
        if happy || hurt {
            let up = if happy { (t * 14.0).sin() } else { 0.0 };
            lc = (5.0, 6.5 + up);
            rc = (35.0, 6.5 - up);
        } else if asleep {
            lc = (6.5, by + 2.5);
            rc = (33.5, by + 2.5);
        } else if f.state == State::Asking {
            rc = (35.0, 6.5 + f.wave(9.0, 0.0) * 1.5);
        } else if f.thinking() {
            lc = (9.5, 18.0 + f.wave(6.0, 0.0) * 0.6);
        } else if running {
            let s = (t * 12.0).sin() * 2.0;
            lc = (5.0, 13.0 + s);
            rc = (35.0, 13.0 - s);
        } else if f.groove > 0.1 {
            let s = f.wave(PI * 2.2, 0.0) * 3.5 * f.groove;
            lc = (5.0, 10.0 + s);
            rc = (35.0, 10.0 - s);
        } else if f.state == State::Speaking {
            lc.1 += f.wave(5.0, 0.0) * 1.3;
            rc.1 += f.wave(5.0, 2.0) * 1.3;
        } else if tired {
            lc.1 += 4.0;
            rc.1 += 4.0;
        }
        let small = if asleep { 0.75 } else { 1.0 };
        for (p, side) in [(lc, -1.0), (rc, 1.0)] {
            let sh = (bx + side * 10.0, by + 0.5);
            let el = (bx + side * 14.5, (by + p.1) / 2.0 + 3.0);
            c.thick(sh, el, 2.2, ORANGE);
            c.thick(el, (p.0, p.1 + 2.0), 2.2, ORANGE);
            c.ellipse(p.0, p.1, 4.4 * small, 4.0 * small, ORANGE);
        }

        // The shell: a wide dome with little spikes around its top edge.
        c.ellipse(bx, by, rx, ry, ORANGE);
        for deg in [-162.0f64, -138.0, -112.0, -68.0, -42.0, -18.0] {
            let a = deg.to_radians();
            let (ca, sa) = (a.cos(), a.sin());
            let edge = (bx + rx * ca * 0.92, by + ry * sa * 0.92);
            let tip = (bx + (rx + 2.6) * ca, by + (ry + 2.6) * sa);
            let (tx, ty) = (-sa * 1.7, ca * 1.7);
            c.triangle((edge.0 + tx, edge.1 + ty), (edge.0 - tx, edge.1 - ty), tip, ORANGE);
        }

        // The eyes: black balls open, orange lids shut.
        let eyes = if f.thinking() && !hurt { Eyes::Open } else { f.eyes() };
        let open = matches!(eyes, Eyes::Open | Eyes::Wide);
        let er = if eyes == Eyes::Wide { 2.7 } else { 2.3 };
        for e in [le, re] {
            if grow > 0.3 {
                c.ellipse(e.0, e.1, er, er + 0.3, if open { EYE } else { ORANGE });
            }
        }

        // The notches of the pincers (cut out, so the outline goes around them).
        for (p, left) in [(lc, true), (rc, false)] {
            let o = self.open(f, left);
            let dir = if left { -105.0f64 } else { -75.0 }.to_radians();
            let spread = (10.0 + o * 32.0).to_radians();
            let r = 7.0 * small;
            let a1 = (p.0 + (dir - spread).cos() * r, p.1 + (dir - spread).sin() * r);
            let a2 = (p.0 + (dir + spread).cos() * r, p.1 + (dir + spread).sin() * r);
            cut(c, (p.0 - (dir).cos() * 0.6, p.1 - dir.sin() * 0.6), a1, a2);
        }

        c.outline(LINE, &[]);
        c.shade(ORANGE, SHADE, LINE);
        c.light(ORANGE, LIGHT, LINE);

        // Glints (open) or lids (shut).
        let look = f.look();
        for (k, e) in [le, re].iter().enumerate() {
            if grow <= 0.3 {
                continue;
            }
            let (x, y) = (e.0.floor() as i32, e.1.floor() as i32);
            if open {
                c.set(x - 1 + look.0, y - 1 + look.1, WHITE);
                c.set(x + look.0, y - 1 + look.1, WHITE);
                c.set(x - 1 + look.0, y + look.1, WHITE);
                c.set(x + 1 + look.0, y + 1 + look.1, GEAR);
            } else {
                c.eye(x - 1, y - 1, 3, 3, eyes, (0, 0), EYE, 0, 0, k == 1);
            }
        }
        // Angry brows.
        if hurt && !asleep {
            for (k, e) in [le, re].iter().enumerate() {
                let (x, y) = (e.0.round() as i32, (e.1 - 4.0).round() as i32);
                if k == 0 {
                    c.line(x - 3, y - 1, x + 1, y + 1, RED);
                    c.line(x - 3, y, x + 1, y + 2, RED);
                } else {
                    c.line(x - 1, y + 1, x + 3, y - 1, RED);
                    c.line(x - 1, y + 2, x + 3, y, RED);
                }
            }
        }

        // The face on the shell: blush and a mouth.
        let my = by as i32 + 1;
        c.ellipse(bx - 6.5, by + 1.5, 1.6, 0.9, BLUSH);
        c.ellipse(bx + 6.5, by + 1.5, 1.6, 0.9, BLUSH);
        let mo = f.mouth();
        if happy {
            c.mouth(20, my - 1, 0.9, 6, 4, MOUTH, BLUSH);
        } else if hurt {
            // A grumpy frown.
            c.line(17, my + 1, 18, my, EYE);
            c.line(19, my, 21, my, EYE);
            c.line(22, my, 23, my + 1, EYE);
        } else if mo > 0.08 {
            c.mouth(20, my - 1, mo, 6, 5, MOUTH, BLUSH);
        } else if asleep {
            c.line(19, my, 21, my, EYE);
        } else {
            c.smile(20, my - 1, 5, EYE);
        }

        // Offline: pale. Angry: flushing red.
        if f.state == State::Offline {
            for col in [ORANGE, SHADE, LIGHT, BLUSH] {
                c.swap(col, PALE);
            }
        }
        if f.progress(f.times.error, 1.3).is_some_and(|u| ((u * 10.0) as i32) % 2 == 0) {
            c.swap(ORANGE, RED);
            c.swap(LIGHT, BLUSH);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        let t = f.t;
        // Crabs walk sideways: back and forth while running, a shuffle when dancing.
        if f.is(&[State::Running, State::Testing]) {
            m.dx += 0.28 * (t * 1.4).sin();
            m.lean += 0.03 * (t * 12.0).sin();
        }
        if f.groove > 0.1 {
            m.dx += 0.12 * f.groove * (t * PI * 1.1).sin();
        }
        if f.state == State::Asking {
            m.lean += 0.04;
        }
        // Stomping in anger.
        if let Some(u) = f.progress(f.times.error, 1.0) {
            m.dy += 0.05 * (u * PI * 8.0).sin().abs() * (1.0 - u);
        }
        if f.asleep() {
            m.sx *= 1.03;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let bubble = |w: &mut World, x: f64, y: f64, big: bool| {
            if big {
                for (dx, dy) in [(0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (2.0, 0.0), (-1.0, 1.0), (2.0, 1.0), (0.0, 2.0), (1.0, 2.0)] {
                    w.px(x + dx, y + dy, BUBBLE);
                }
                w.px(x, y, WHITE);
            } else {
                for (dx, dy) in [(0.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
                    w.px(x + dx, y + dy, BUBBLE);
                }
            }
        };

        // Speaking: bubbles from its mouth, more with a louder voice.
        if f.state == State::Speaking {
            for k in 0..4u64 {
                let ph = (t * 0.9 + k as f64 * 0.25) % 1.0;
                if f.voice < 0.03 && ph > 0.4 {
                    continue;
                }
                let x = 24.0 + ph * 18.0 + (ph * 9.0 + k as f64).sin() * 1.5;
                bubble(w, x, 25.0 - ph * 22.0, k % 2 == 0);
            }
        }
        // Asleep: big slow bubbles.
        if f.state == State::Sleeping {
            let ph = (t * 0.35) % 1.0;
            bubble(w, 23.0 + ph * 5.0 + (ph * 6.0).sin(), 27.0 - ph * 24.0, true);
            let ph2 = (t * 0.35 + 0.5) % 1.0;
            bubble(w, 24.0 + ph2 * 4.0, 27.0 - ph2 * 24.0, false);
        }

        // Thinking: "fn" being typed over its head, a cursor blinking after it.
        if f.thinking() {
            let n = ((f.st * 2.0) as usize % 5).min(4);
            let s = ["", "f", "fn", "fn(", "fn()"][n];
            text(w, s, 22.0, -7.0, GOLD);
            if f.wave(8.0, 0.0) > 0.0 {
                text(w, "_", 22.0 + s.len() as f64 * 4.0, -7.0, WHITE);
            }
        }

        // Reading, searching, on the web: crates flying in to its pincers.
        if f.reading() {
            for k in 0..3u64 {
                let ph = (t * 0.7 + k as f64 / 3.0) % 1.0;
                let left = k % 2 == 0;
                let (sx, sy) = (if left { -14.0 } else { 54.0 }, -10.0 + k as f64 * 5.0);
                let (ex, ey) = (if left { 5.0 } else { 35.0 }, 14.0);
                let (x, y) = (sx + (ex - sx) * ph, sy + (ey - sy) * ph - (ph * PI).sin() * 6.0);
                for yy in 0..4 {
                    for xx in 0..4 {
                        let col = if yy == 0 || yy == 3 || xx == 0 || xx == 3 { CRATE } else { GOLD };
                        w.px(x + xx as f64, y + yy as f64, col);
                    }
                }
            }
        }

        // Editing: little cut sparks at the pincers.
        if f.state == State::Editing && f.wave(18.0, 0.0) > 0.6 {
            for (x, y) in [(3.0, 10.0), (37.0, 10.0)] {
                w.px(x, y, WHITE);
                w.px(x - 1.0, y - 1.0, GOLD);
                w.px(x + 1.0, y - 1.0, GOLD);
            }
        }

        // Running, testing, compacting: a gear turning over it (tests tick green).
        if f.is(&[State::Running, State::Testing, State::Compacting]) {
            let (gx, gy) = (33.0, -3.0);
            let a0 = t * 3.0;
            for k in 0..8 {
                let a = a0 + k as f64 * PI / 4.0;
                w.px(gx + a.cos() * 4.2, gy + a.sin() * 4.2, GEAR);
            }
            for k in 0..12 {
                let a = a0 + k as f64 * PI / 6.0;
                w.px(gx + a.cos() * 3.0, gy + a.sin() * 3.0, if k % 3 == 0 { STEEL } else { GEAR });
            }
            w.px(gx, gy, STEEL);
            if f.state == State::Testing {
                let n = ((f.st * 1.5) as usize) % 5;
                for k in 0..n {
                    let x = -4.0 + k as f64 * 5.0;
                    for (dx, dy) in [(0.0, 1.0), (1.0, 2.0), (2.0, 1.0), (3.0, 0.0)] {
                        w.px(x + dx, -6.0 + dy, GREEN);
                    }
                }
            }
            // Sand kicked up by its feet.
            if f.state != State::Compacting {
                for k in 0..3 {
                    let ph = (t * 2.0 + k as f64 / 3.0) % 1.0;
                    w.px_dim(4.0 - ph * 6.0, 35.0 - (ph * PI).sin() * 3.0, CRATE);
                    w.px_dim(36.0 + ph * 6.0, 35.0 - (ph * PI).sin() * 3.0, CRATE);
                }
            }
        }

        // Git: a branch graph growing beside it.
        if f.state == State::Git || f.within(f.times.merge, 1.5) {
            let grow = (f.st * 6.0).min(20.0);
            let (x0, y0) = (44.0, 30.0);
            for k in 0..grow as i32 {
                w.px(x0, y0 - k as f64, STEEL);
            }
            let side = (grow - 6.0).max(0.0).min(8.0);
            for k in 0..side as i32 {
                let y = y0 - 6.0 - k as f64;
                w.px(x0 + 3.0, y, GREEN);
            }
            w.px(x0 + 1.0, y0 - 5.0, GREEN);
            w.px(x0 + 2.0, y0 - 6.0, GREEN);
            if grow > 15.0 {
                w.px(x0 + 2.0, y0 - 14.0, GREEN);
                w.px(x0 + 1.0, y0 - 15.0, GREEN);
            }
            for k in 0..4 {
                let y = y0 - k as f64 * 5.0;
                if y0 - y <= grow {
                    w.px(x0, y, GOLD);
                }
            }
        }

        // Done: bursts at the pincer tips as they snap, bubbles of joy.
        if let Some(u) = f.progress(f.times.done, 1.4) {
            for (cx, cy) in [(3.0, 4.0), (37.0, 4.0)] {
                let r = 2.0 + u * 9.0;
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0 + 0.3;
                    let col = if k % 2 == 0 { GOLD } else { WHITE };
                    if u < 0.8 || k % 2 == 0 {
                        w.px(cx + a.cos() * r, cy + a.sin() * r, col);
                    }
                }
                w.px(cx, cy, WHITE);
            }
            for k in 0..5u64 {
                let x = 6.0 + f.rand(k + 40) * 28.0;
                let y = 30.0 - u * (20.0 + f.rand(k + 50) * 15.0);
                bubble(w, x, y, k % 2 == 0);
            }
        }

        // Error: the borrow checker — a padlock and E0502 over it, steam off its stalks.
        if let Some(u) = f.progress(f.times.error, 2.2) {
            let drop = (u * 8.0).min(1.0);
            let (lx, ly) = (16.5, -16.0 + drop * 6.0 + (t * 6.0).sin() * 0.6);
            w.sprite(&self.lock, lx, ly);
            if (u * 12.0) as i32 % 3 != 0 || u < 0.5 {
                text(w, "E0502", 10.0, ly + 9.0, RED);
            }
            for k in 0..2 {
                let ph = (t * 2.5 + k as f64 * 0.5) % 1.0;
                for x in [9.0, 31.0] {
                    w.px_dim(x + (ph * 8.0).sin(), 4.0 - ph * 8.0, WHITE);
                }
            }
        }

        // A key: a click spark at a pincer.
        if f.within(f.times.key, 0.15) {
            let x = if f.times.key as u64 % 2 == 0 { 3.0 } else { 37.0 };
            w.px(x, 9.0, WHITE);
            w.px(x - 1.0, 8.0, GOLD);
            w.px(x + 1.0, 8.0, GOLD);
        }

        // Copying: .clone() — a little Ferris pops out beside it.
        if let Some(u) = f.progress(f.times.copy, 2.8) {
            let out = (u * 5.0).min(1.0);
            let hop = ((u * 14.0).sin().abs()) * 1.5 * (1.0 - out * 0.5);
            let x = 26.0 + out * 16.0;
            w.sprite(&self.clone, x, 28.0 - hop);
            if u > 0.25 && u < 0.85 {
                text(w, "()", x + 3.0, 19.0, GOLD);
            }
        }

        // Little context left: a sweat drop.
        if f.context > 0.85 && !f.asleep() {
            let y = 14.0 + ((t * 0.8) % 1.0) * 4.0;
            w.px(31.0, y, BUBBLE);
            w.px(31.0, y + 1.0, BUBBLE);
            w.px(30.0, y + 1.0, BUBBLE);
        }
    }
}
