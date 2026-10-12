//! Blinky, the red ghost of Pac-Man: a dome, straight sides and a skirt whose points ripple
//! along every frame, with big eyes whose whites and pupils follow where Iris looks. What he does:
//! - waiting: hovering, skirt rippling, eyes wandering; music: he sways and bobs to the beat,
//!   eyes flicking side to side, skirt racing;
//! - listening: eyes wide, looking at you; asking: leaning, a little "o" of a mouth;
//! - thinking, planning: pupils rolling around, his target tile (a crosshair) hunting above;
//! - speaking: a mouth with the voice, his eyes bouncing with it;
//! - reading, searching, on the web: chasing pac-dots that flee from him;
//! - running, testing: chase mode, darting side to side with speed lines; editing: eyes down;
//! - git: he goes off to his scatter corner, a loop he runs round;
//! - done: eaten! "200", only his eyes left, they fly home to the ghost house and he's reborn;
//! - error: frightened: blue, a zigzag mouth, trembling, then flashing white;
//! - a key: his eyes dart; copying: Pinky, Inky and Clyde peek in from the sides;
//! - asleep: eyes shut, sunk low, a bubble; offline: gray; booting: rising out of the ghost
//!   house; little context left: heavy lids and a sweat drop.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Blinky";
pub const SCENE: &str = "city";
pub const DESCRIPTION: &str = "Blinky, el fantasma rojo de Pac-Man: persigue puntitos al leer, se asusta en azul con los errores, al terminar vuelve a casa como ojos y renace, y sus hermanos asoman al copiar";

const LINE: u8 = 1;
const RED: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const WHITE: u8 = 5;
const PUPIL: u8 = 6;
const SCARED: u8 = 7;
const SCARED_L: u8 = 8;
const DOT: u8 = 9;
const PINK: u8 = 10;
const CYAN: u8 = 11;
const ORANGE: u8 = 12;
const MOUTH: u8 = 13;
const PALE: u8 = 14;
const DROP: u8 = 15;
const SHEEN: u8 = 16;

/// Frightened for this long after an error, flashing after `FLASH`.
const AFRAID: f64 = 2.2;
const FLASH: f64 = 1.35;
/// Eaten, after done: a flash until `EAT`, only eyes until `HOME`, reborn by `REBORN`.
const EAT: f64 = 0.12;
const HOME: f64 = 1.35;
const REBORN: f64 = 1.7;

/// A 3×5 font for the score.
fn glyph(ch: char) -> [&'static str; 5] {
    match ch {
        '0' => ["XXX", "X.X", "X.X", "X.X", "XXX"],
        '2' => ["XX.", "..X", ".X.", "X..", "XXX"],
        '4' => ["X.X", "X.X", "XXX", "..X", "..X"],
        _ => ["...", "...", "...", "...", "..."],
    }
}

fn text(w: &mut World, x: f64, y: f64, s: &str, c: u8, dim: bool) {
    for (i, ch) in s.chars().enumerate() {
        for (row, line) in glyph(ch).iter().enumerate() {
            for (col, p) in line.chars().enumerate() {
                if p == 'X' {
                    let (px, py) = (x + i as f64 * 4.0 + col as f64, y + row as f64);
                    if dim { w.px_dim(px, py, c) } else { w.px(px, py, c) }
                }
            }
        }
    }
}

/// A pair of ghost eyes in the world (centers 11 apart), pupils looking (lx, ly).
fn eyes_in_world(w: &mut World, x: f64, y: f64, lx: f64, ly: f64) {
    for ex in [x, x + 11.0] {
        for dy in -4..=4 {
            for dx in -3..=3 {
                let (u, v) = (dx as f64 / 3.5, dy as f64 / 4.6);
                if u * u + v * v <= 1.0 {
                    w.px(ex + dx as f64 + lx * 0.5, y + dy as f64 + ly * 0.5, WHITE);
                }
            }
        }
        for dy in -1..=2 {
            for dx in -1..=2 {
                w.px(ex + dx as f64 - 0.5 + lx * 1.6, y + dy as f64 - 0.5 + ly * 1.6, PUPIL);
            }
        }
    }
}

/// A sibling, 8×8, peeking, its top-left at (x, y), looking toward `lx`.
fn sibling(w: &mut World, x: f64, y: f64, body: u8, lx: f64, t: f64) {
    const ROWS: [&str; 7] = ["..XXXX..", ".XXXXXX.", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX"];
    for (r, line) in ROWS.iter().enumerate() {
        for (c, p) in line.chars().enumerate() {
            if p == 'X' {
                w.px(x + c as f64, y + r as f64, body);
            }
        }
    }
    let skirt = if (t * 8.0).sin() > 0.0 { "X.XX.XX." } else { ".XX.XX.X" };
    for (c, p) in skirt.chars().enumerate() {
        if p == 'X' {
            w.px(x + c as f64, y + 7.0, body);
        }
    }
    for ex in [1.0, 5.0] {
        for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.0, 2.0), (1.0, 2.0)] {
            w.px(x + ex + dx + lx * 0.5, y + 1.0 + dy, WHITE);
        }
        w.px(x + ex + 0.5 + lx, y + 2.0, PUPIL);
    }
}

pub struct Char {
    /// Where his eyes dart on a key.
    dart: (i32, i32),
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#b02238", "#ff2a2a", "#d81c2c", "#ff8070", "#ffffff", "#2a3cff", "#3a44ff", "#8a90ff", "#ffb8ae", "#ffb8ff", "#3ff0ff", "#ffb852",
        "#7a1020", "#a8a4b8", "#8fd6ff", "#cfd6ff",
    ];
    const W: usize = 32;
    const H: usize = 34;

    fn new(_seed: u64) -> Self {
        Char { dart: (1, 0) }
    }

    fn event(&mut self, h: Happening, f: &Frame) {
        if let Happening::Event(Event::Key) = h {
            let r = f.rand(f.t.to_bits());
            self.dart = [(1, 0), (-1, 0), (0, -1), (1, -1), (-1, -1), (0, 1)][(r * 6.0) as usize % 6];
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let es = f.since(f.times.error);
        let ds = f.since(f.times.done);
        let afraid = (0.0..AFRAID).contains(&es);
        // Eaten: only his eyes are left, and they're out flying (see `around`).
        if (EAT..HOME).contains(&ds) {
            return;
        }
        // Reborn: flickering back into being.
        if (HOME..REBORN).contains(&ds) && ((ds * 18.0) as i32) % 2 == 0 {
            return;
        }
        let asleep = f.asleep();
        let tired = f.context > 0.85 && !asleep;
        let dance = f.groove > 0.1 && !asleep;

        // Rising out of the ghost house when Iris boots.
        let oy = if f.state == State::Booting { ((1.0 - (f.st / 1.3).min(1.0)).powi(2) * 34.0).round() } else { 0.0 };

        // The skirt ripples along: faster chasing and dancing, barely asleep.
        let speed = if asleep {
            0.25
        } else if afraid {
            2.6
        } else if dance {
            2.4
        } else if f.is(&[State::Running, State::Testing]) || f.reading() {
            3.2
        } else if tired {
            0.5
        } else {
            1.2
        };
        let shift = t * speed * 7.0;
        c.ellipse(16.0, 15.0 + oy, 14.0, 14.0, RED);
        for x in 2..30 {
            let fr = ((x as f64 - 2.0 + shift) / 7.0).rem_euclid(1.0);
            let tri = 1.0 - (2.0 * fr - 1.0).abs();
            let bottom = 29.0 + (3.0 * tri).round();
            let mut y = 15.0;
            while y <= bottom {
                c.set(x, (y + oy) as i32, RED);
                y += 1.0;
            }
        }
        c.outline(LINE, &[]);
        c.shade(RED, SHADE, LINE);
        c.light(RED, LIGHT, LINE);
        // A sheen on his dome.
        c.recolor(7, 4 + oy as i32, 4, 2, RED, LIGHT);
        c.recolor(5, 6 + oy as i32, 2, 3, RED, LIGHT);

        let oyi = oy as i32;
        if afraid {
            // Frightened: blue, little eyes, a zigzag mouth; flashing white near the end.
            let flash = es > FLASH && ((es * 8.0) as i32) % 2 == 0;
            let (body, light, feat) = if flash { (WHITE, WHITE, RED) } else { (SCARED, SCARED_L, DOT) };
            c.swap(RED, body);
            c.swap(SHADE, body);
            c.swap(LIGHT, light);
            c.swap(LINE, if flash { DOT } else { SCARED_L });
            let tr = if (t * 40.0).sin() > 0.0 { 1 } else { 0 };
            c.rect(9 + tr, 11 + oyi, 3, 3, feat);
            c.rect(20 + tr, 11 + oyi, 3, 3, feat);
            for k in 0..10 {
                let x = 6 + k * 2;
                let y = if k % 2 == 0 { 22 } else { 20 };
                c.line(x, y + oyi, x + 2, if k % 2 == 0 { 20 } else { 22 } + oyi, feat);
            }
            return;
        }

        // The eyes: big whites shifted toward where he looks, bigger pupils further.
        let (mut lx, mut ly) = f.look();
        if f.thinking() {
            let a = t * 2.4;
            lx = a.cos().round() as i32;
            ly = (a.sin() - 0.4).round() as i32;
        } else if f.reading() {
            lx = 1;
            ly = 0;
            if f.state == State::Searching {
                lx = if (t * 3.0).sin() > 0.0 { 1 } else { -1 };
            }
        } else if f.is(&[State::Running, State::Testing]) {
            lx = if (t * 1.8).cos() >= 0.0 { 1 } else { -1 };
        } else if f.state == State::Editing {
            ly = 1;
        } else if f.state == State::Git {
            lx = 1;
            ly = -1;
        } else if f.is(&[State::Listening, State::Transcribing]) {
            lx = 0;
            ly = 1;
        } else if dance {
            lx = if (t * PI * 1.1).sin() > 0.0 { 1 } else { -1 };
        } else if f.state == State::Asking {
            lx = if (t * 0.8).sin() > 0.3 { 1 } else { 0 };
        }
        if f.within(f.times.key, 0.4) {
            (lx, ly) = self.dart;
        }
        let bounce = if f.state == State::Speaking && f.voice > 0.08 && f.wave(12.0, 0.0) > 0.3 { -1 } else { 0 };
        let wide = f.is(&[State::Listening, State::Transcribing, State::Asking]);
        let (rx, ry) = if wide { (3.9, 5.0) } else { (3.5, 4.6) };
        let shut = asleep || (f.blink && !wide);
        for ex in [10.5, 21.5] {
            let (cx, cy) = (ex + lx as f64, 13.5 + ly as f64 + bounce as f64 + oy);
            if shut {
                // A closed eye: a soft curve.
                for dx in -3i32..=3 {
                    let y = cy + 1.0 + if dx.abs() >= 3 { -1.0 } else { 0.0 };
                    c.set((cx + dx as f64 - 0.5) as i32, y as i32, MOUTH);
                }
                continue;
            }
            c.ellipse(cx, cy, rx, ry, WHITE);
            // A touch of shade at the bottom of each white.
            c.recolor((cx - 2.0) as i32, (cy + ry - 1.5) as i32, 4, 1, WHITE, SHEEN);
            let (px, py) = (cx + lx as f64 * 1.6 - 2.0, cy + ly as f64 * 1.8 - 2.0);
            c.rect(px.round() as i32, py.round() as i32, 4, 4, PUPIL);
            c.set(px.round() as i32, py.round() as i32, SHEEN);
            // Tired: heavy lids over the top of the whites.
            if tired && !f.blink {
                c.recolor((cx - 4.0) as i32, (cy - ry - 1.0) as i32, 9, (ry + 0.5) as i32, WHITE, RED);
                c.recolor((cx - 4.0) as i32, (cy - ry - 1.0) as i32, 9, (ry + 0.5) as i32, PUPIL, RED);
                c.recolor((cx - 4.0) as i32, (cy - ry - 1.0) as i32, 9, (ry + 0.5) as i32, SHEEN, RED);
                c.line((cx - 3.0) as i32, (cy - 0.5) as i32, (cx + 3.0) as i32, (cy - 0.5) as i32, LINE);
            }
        }

        // A mouth: with the voice, an "o" asking, shocked saying no.
        let open = if f.state == State::Speaking {
            f.mouth()
        } else if f.state == State::Asking || f.within(f.times.nope, 0.5) {
            0.35
        } else {
            0.0
        };
        if open > 0.08 {
            c.mouth(16, 22 + oyi + bounce, (open * 1.4).min(1.0), 8, 6, MOUTH, LIGHT);
        }

        if f.state == State::Offline {
            for col in [RED, SHADE, LIGHT] {
                c.swap(col, PALE);
            }
            c.swap(PUPIL, MOUTH);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        let t = f.t;
        let es = f.since(f.times.error);
        if f.state == State::Booting {
            m.dy = 0.0;
            return;
        }
        // Ghosts hover; asleep he sinks and sags.
        if f.asleep() {
            m.dy -= 0.05;
            m.sy *= 0.94;
            m.sx *= 1.03;
        } else {
            m.dy += 0.045 * (t * 2.2).sin();
        }
        if f.is(&[State::Running, State::Testing]) {
            m.dx += 0.45 * (t * 1.8).sin();
        }
        if f.reading() {
            m.dx += 0.06 * (t * 6.0).sin().max(0.0);
            m.lean += 0.05;
        }
        if f.groove > 0.1 && !f.asleep() {
            m.dy += f.beat * 0.06 * f.groove;
        }
        if (0.0..AFRAID).contains(&es) {
            m.dx += 0.025 * (t * 47.0).sin();
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let es = f.since(f.times.error);
        let ds = f.since(f.times.done);
        let asleep = f.asleep();

        // Reading, searching, on the web: dots fleeing ahead of him.
        if f.reading() && !asleep {
            let n = 5;
            for k in 0..n {
                let p = (k as f64 * 7.0 + t * 9.0).rem_euclid(25.0);
                let (x, y) = (33.0 + p, 17.0 + (t * 7.0 + k as f64).sin() * 1.2);
                let big = f.state == State::Web && k == 2;
                let col = if big && (t * 6.0).sin() < -0.3 { 0 } else { DOT };
                let fade = p > 18.0;
                let pts: &[(f64, f64)] = if big {
                    &[(0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
                } else {
                    &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
                };
                for (dx, dy) in pts {
                    if fade { w.px_dim(x + dx, y + dy, col) } else { w.px(x + dx, y + dy, col) }
                }
                if f.state == State::Searching {
                    w.px_dim(-4.0 - p, y, DOT);
                }
            }
        }

        // Thinking, planning: his target tile, a crosshair hunting above him.
        if f.thinking() {
            let (x, y) = (16.0 + (t * 0.9).sin() * 14.0, -8.0 + (t * 1.3).cos() * 3.0);
            let on = (t * 5.0).sin() > -0.5;
            for k in -3..=3 {
                if k != 0 {
                    w.px(x + k as f64, y, if on { RED } else { LIGHT });
                    w.px(x, y + k as f64, if on { RED } else { LIGHT });
                }
            }
            for (dx, dy) in [(-2.0, -2.0), (2.0, -2.0), (-2.0, 2.0), (2.0, 2.0)] {
                w.px_dim(x + dx, y + dy, LIGHT);
            }
        }

        // Running, testing: speed lines behind him.
        if f.is(&[State::Running, State::Testing]) {
            let dir = if (t * 1.8).cos() >= 0.0 { 1.0 } else { -1.0 };
            for k in 0..4 {
                let p = (t * 3.0 + k as f64 / 4.0) % 1.0;
                let x0 = if dir > 0.0 { -2.0 - p * 8.0 } else { 34.0 + p * 8.0 };
                let y = 9.0 + k as f64 * 5.5;
                for s in 0..4 {
                    w.px_dim(x0 - dir * s as f64, y, if k % 2 == 0 { LIGHT } else { WHITE });
                }
            }
        }

        // Git: off to his scatter corner, running round its loop.
        if f.state == State::Git {
            let (x0, y0, side) = (30.0, -14.0, 10.0);
            let mut k = 0.0;
            while k < side {
                for (x, y) in [(x0 + k, y0), (x0 + side, y0 + k), (x0 + side - k, y0 + side), (x0, y0 + side - k)] {
                    if (k as i32) % 2 == 0 {
                        w.px_dim(x, y, DOT);
                    }
                }
                k += 1.0;
            }
            let d = (t * 12.0) % (side * 4.0);
            let (x, y) = if d < side {
                (x0 + d, y0)
            } else if d < 2.0 * side {
                (x0 + side, y0 + d - side)
            } else if d < 3.0 * side {
                (x0 + 3.0 * side - d, y0 + side)
            } else {
                (x0, y0 + 4.0 * side - d)
            };
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                w.px(x + dx - 0.5, y + dy - 0.5, RED);
            }
        }

        // Done: eaten — "200", then his eyes fly home to the ghost house, and back.
        if (0.0..REBORN + 0.3).contains(&ds) {
            if ds < 1.0 {
                text(w, 10.0, -6.0 - ds * 5.0, "200", CYAN, ds > 0.8);
            }
            // The ghost house: blue walls, a pink door at the bottom.
            if (EAT..HOME + 0.2).contains(&ds) {
                for x in 2..31 {
                    w.px(x as f64, -36.0, SCARED);
                    if !(12..21).contains(&x) {
                        w.px(x as f64, -24.0, SCARED);
                    } else {
                        w.px(x as f64, -24.0, PINK);
                    }
                }
                for y in -36..=-24 {
                    w.px(2.0, y as f64, SCARED);
                    w.px(30.0, y as f64, SCARED);
                }
            }
            if (EAT..HOME).contains(&ds) {
                let (ey, ly) = if ds < 0.7 {
                    let u = (ds - EAT) / (0.7 - EAT);
                    (13.5 - u * u * 44.0, -1.0)
                } else if ds < 1.0 {
                    (-30.5, if (t * 6.0).sin() > 0.0 { 1.0 } else { -1.0 })
                } else {
                    let u = (ds - 1.0) / (HOME - 1.0);
                    (-30.5 + u * 44.0, 1.0)
                };
                let lx = if (0.7..1.0).contains(&ds) { (t * 5.0).sin().round() } else { 0.0 };
                eyes_in_world(w, 10.5, ey, lx, ly);
            }
            // Reborn: a ring of sparkles.
            if let Some(u) = f.progress(f.times.done + HOME, 0.6) {
                for k in 0..10 {
                    let a = k as f64 * PI / 5.0 + u;
                    let r = 16.0 + u * 8.0;
                    if u < 0.6 { w.px(16.0 + a.cos() * r, 17.0 + a.sin() * r, if k % 2 == 0 { WHITE } else { PINK }) } else { w.px_dim(16.0 + a.cos() * r, 17.0 + a.sin() * r, PINK) }
                }
            }
        }

        // Frightened: sweat flying off him.
        if (0.0..AFRAID).contains(&es) {
            for k in 0..2 {
                let p = (es * 1.8 + k as f64 * 0.5) % 1.0;
                let side = if k == 0 { -1.0 } else { 1.0 };
                let (x, y) = (16.0 + side * (16.0 + p * 5.0), 6.0 - p * 4.0 + p * p * 8.0);
                w.px(x, y, DROP);
                w.px(x, y + 1.0, DROP);
            }
        }

        // Copying: Pinky, Inky and Clyde peek in.
        if let Some(u) = f.progress(f.times.copy, 2.8) {
            let s = u * 2.8;
            let peek = |start: f64| -> f64 {
                let a = ((s - start) / 0.25).clamp(0.0, 1.0);
                let b = ((2.8 - s) / 0.3).clamp(0.0, 1.0);
                a.min(b)
            };
            sibling(w, -12.0 + peek(0.0) * 6.0, 6.0, PINK, 1.0, t);
            sibling(w, 36.0 - peek(0.2) * 6.0, 4.0, CYAN, -1.0, t + 0.3);
            sibling(w, 37.0 - peek(0.4) * 5.0, 22.0, ORANGE, -1.0, t + 0.6);
        }

        // Merge: two ghosts caught: "400".
        if let Some(u) = f.progress(f.times.merge, 1.0) {
            text(w, 10.0, -6.0 - u * 5.0, "400", CYAN, u > 0.7);
        }

        // A key: a pac-dot where his eyes dart.
        if f.within(f.times.key, 0.4) {
            let (dx, dy) = self.dart;
            w.px(16.0 + dx as f64 * 21.0, 15.0 + dy as f64 * 20.0, DOT);
        }

        // Asleep: a little bubble growing and popping.
        if f.state == State::Sleeping {
            let p = (t / 2.8) % 1.0;
            if p < 0.85 {
                let r = 0.5 + p * 2.5;
                for k in 0..10 {
                    let a = k as f64 * PI / 5.0;
                    w.px_dim(31.0 + r + a.cos() * r, 20.0 + a.sin() * r, SHEEN);
                }
            }
        }

        // Booting: the ghost house's pink door he rises through.
        if f.state == State::Booting && f.st < 1.8 {
            for x in 0..32 {
                if f.st < 1.4 { w.px(x as f64, 34.0, PINK) } else { w.px_dim(x as f64, 34.0, PINK) }
            }
        }

        // Little context left: a sweat drop.
        if f.context > 0.85 && !asleep {
            let y = 3.0 + ((t * 0.8) % 1.0) * 4.0;
            w.px(28.0, y, DROP);
            w.px(28.0, y + 1.0, DROP);
            w.px(27.0, y + 1.0, DROP);
        }
    }
}
