//! Pac-Man: a yellow disc and a mouth, cut out every frame at any angle, so he can face any way
//! and chomp, fold up and come back. What he does:
//! - he faces a way for each state: right waiting, speaking and reading, up thinking, left
//!   listening, down editing, left on git, back and forth running;
//! - waiting: a chomp now and then, a look behind him, blinks; music: he spins to the beat,
//!   chomping, facing right, down, left, up;
//! - listening: turned, eye wide; asking: a power pellet blinking before him, his mouth ajar;
//! - thinking, planning: facing up at a little maze where a dot runs the corridors;
//! - speaking: the mouth with the voice, "waka" ripples out of it;
//! - reading, searching, on the web: pac-dots flowing into his mouth (power pellets on the web);
//! - running: Blinky chasing him back and forth; testing: he chases a frightened ghost;
//! - git: two lines of dots, a branch merging into the main one, into his mouth;
//! - done: he swallows a power pellet and glows, "50" floating up;
//! - error: the classic death: he faces up and folds open until he's gone, pops, and returns;
//! - a key: one dot, chomped; copying: a cherry appears, he eats it: "100";
//! - asleep: mouth shut, a nightcap; offline: gray, "GAME OVER"; booting: "READY!";
//! - little context left: a weak, slow chomp and a sweat drop.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Pac-Man";
pub const SCENE: &str = "city";
pub const DESCRIPTION: &str = "Pac-Man en pixel art: se come los puntitos al leer, muere con su animacion clasica ante un error, brilla con un power pellet al terminar y aparece una cereza al copiar";

const LINE: u8 = 1;
const YELLOW: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const EYE: u8 = 5;
const WHITE: u8 = 6;
const DOT: u8 = 7;
const RED: u8 = 8;
const STEM: u8 = 9;
const LEAF: u8 = 10;
const WALL: u8 = 11;
const GLOW: u8 = 12;
const PALE: u8 = 13;
const DROP: u8 = 14;
const ORANGE: u8 = 15;

const RIGHT: f64 = 0.0;
const DOWN: f64 = PI / 2.0;
const LEFT: f64 = PI;
const UP: f64 = -PI / 2.0;

/// The death, in seconds after an error: folding until `FOLD`, a pop until `POP`, gone until
/// `GONE`, growing back by `BACK`.
const FOLD: f64 = 1.1;
const POP: f64 = 1.45;
const GONE: f64 = 1.8;
const BACK: f64 = 2.2;

const CX: f64 = 17.0;
const CY: f64 = 18.0;
const R: f64 = 15.0;

/// A 3×5 font, for the arcade's words and scores.
fn glyph(ch: char) -> [&'static str; 5] {
    match ch {
        'A' => [".X.", "X.X", "XXX", "X.X", "X.X"],
        'D' => ["XX.", "X.X", "X.X", "X.X", "XX."],
        'E' => ["XXX", "X..", "XX.", "X..", "XXX"],
        'G' => [".XX", "X..", "X.X", "X.X", ".XX"],
        'M' => ["X.X", "XXX", "XXX", "X.X", "X.X"],
        'O' => [".X.", "X.X", "X.X", "X.X", ".X."],
        'R' => ["XX.", "X.X", "XX.", "X.X", "X.X"],
        'V' => ["X.X", "X.X", "X.X", "X.X", ".X."],
        'Y' => ["X.X", "X.X", ".X.", ".X.", ".X."],
        '!' => [".X.", ".X.", ".X.", "...", ".X."],
        '0' => ["XXX", "X.X", "X.X", "X.X", "XXX"],
        '1' => [".X.", "XX.", ".X.", ".X.", "XXX"],
        '5' => ["XXX", "X..", "XX.", "..X", "XX."],
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

/// A little ghost, 8×8, its top-left at (x, y): `body` its color (or frightened), looking `dir`.
fn ghost(w: &mut World, x: f64, y: f64, body: u8, scared: bool, dir: f64, t: f64) {
    const ROWS: [&str; 7] = ["..XXXX..", ".XXXXXX.", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX"];
    for (r, line) in ROWS.iter().enumerate() {
        for (c, p) in line.chars().enumerate() {
            if p == 'X' {
                w.px(x + c as f64, y + r as f64, if scared { WALL } else { body });
            }
        }
    }
    let skirt = if (t * 8.0).sin() > 0.0 { "X.XX.XX." } else { ".XX.XX.X" };
    for (c, p) in skirt.chars().enumerate() {
        if p == 'X' {
            w.px(x + c as f64, y + 7.0, if scared { WALL } else { body });
        }
    }
    if scared {
        w.px(x + 2.0, y + 3.0, DOT);
        w.px(x + 5.0, y + 3.0, DOT);
        for k in 0..6 {
            w.px(x + 1.0 + k as f64, y + if k % 2 == 0 { 6.0 } else { 5.0 }, DOT);
        }
    } else {
        let lx = dir.cos().round();
        let ly = dir.sin().round();
        for ex in [1.0, 5.0] {
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.0, 2.0), (1.0, 2.0)] {
                w.px(x + ex + dx + lx * 0.5, y + 1.0 + dy, WHITE);
            }
            w.px(x + ex + 0.5 + lx * 0.6, y + 2.0 + ly * 0.6, WALL);
        }
    }
}

/// A cherry pair, its top-left at (x, y).
fn cherry(w: &mut World, x: f64, y: f64) {
    for (dx, dy) in [(4.0, 0.0), (5.0, 0.0), (6.0, 0.0), (3.0, 1.0), (6.0, 1.0), (2.0, 2.0), (6.0, 2.0), (2.0, 3.0), (6.0, 3.0)] {
        w.px(x + dx, y + dy, STEM);
    }
    w.px(x + 7.0, y, LEAF);
    w.px(x + 7.0, y + 1.0, LEAF);
    for (cx, cy) in [(1.5, 5.5), (6.0, 6.5)] {
        for dy in -2..=1 {
            for dx in -2..=1 {
                let d = (dx as f64 + 0.5).powi(2) + (dy as f64 + 0.5).powi(2);
                if d <= 4.6 {
                    w.px(x + cx + dx as f64, y + cy + dy as f64, RED);
                }
            }
        }
        w.px(x + cx - 1.0, y + cy - 1.0, WHITE);
    }
}

pub struct Char {
    /// A chomp now and then while waiting, and a look behind him.
    chomp: f64,
    next_chomp: f64,
    turn: f64,
    next_turn: f64,
}

impl Char {
    /// Where he faces, as an angle.
    fn facing(&self, f: &Frame) -> f64 {
        let es = f.since(f.times.error);
        if (0.0..POP).contains(&es) {
            return UP;
        }
        if f.within(f.times.done, 1.0) || f.within(f.times.copy, 2.4) {
            return RIGHT;
        }
        if f.groove > 0.1 && !f.asleep() {
            return [RIGHT, DOWN, LEFT, UP][((f.t / 0.45) as usize) % 4];
        }
        match f.state {
            State::Thinking | State::Planning | State::Delegating => UP,
            State::Listening | State::Transcribing | State::Git => LEFT,
            State::Editing | State::Compacting => DOWN,
            State::Searching => {
                if (f.st / 1.4) as i32 % 2 == 0 { RIGHT } else { LEFT }
            }
            State::Running | State::Testing => {
                if (f.t * 1.6).cos() >= 0.0 { RIGHT } else { LEFT }
            }
            State::Idle if f.within(self.turn, 1.2) => LEFT,
            _ => RIGHT,
        }
    }

    /// How open the mouth is (0..1 of its widest).
    fn open(&self, f: &Frame) -> f64 {
        let t = f.t;
        if f.within(f.times.done, 0.35) {
            return 1.0;
        }
        if let Some(u) = f.progress(f.times.key, 0.25) {
            return (u * PI).sin();
        }
        if let Some(u) = f.progress(f.times.copy + 1.6, 0.5) {
            return (u * PI * 2.0).sin().abs();
        }
        let chomp = |speed: f64| (t * speed).sin().abs();
        if f.groove > 0.1 && !f.asleep() {
            return chomp(PI / 0.45);
        }
        if f.context > 0.85 && !f.asleep() {
            return 0.15 + 0.25 * chomp(4.0);
        }
        match f.state {
            State::Sleeping | State::Booting | State::Listening | State::Transcribing => 0.08,
            State::Offline => 0.35,
            State::Speaking => (f.voice * 6.0 + 0.35 * chomp(11.0)).clamp(0.15, 1.0),
            State::Asking => 0.45,
            State::Reading | State::Web | State::Searching | State::Git => 0.1 + 0.9 * chomp(11.0),
            State::Running | State::Testing => chomp(16.0),
            State::Editing => chomp(7.0),
            State::Typing => 0.3 + 0.4 * chomp(14.0),
            State::Thinking | State::Planning => 0.3 + 0.3 * chomp(2.0),
            _ => {
                if let Some(u) = f.progress(self.chomp, 1.0) {
                    (u * PI * 4.0).sin().abs()
                } else {
                    0.55
                }
            }
        }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#d08a00", "#ffe31a", "#f0b400", "#fff7a8", "#1c1408", "#ffffff", "#ffb8ae", "#ff2a2a", "#d0882e", "#3ad04a", "#4a4aff", "#fffbe0",
        "#d6d0b0", "#8fd6ff", "#ffb852",
    ];
    const W: usize = 34;
    const H: usize = 34;

    fn new(seed: u64) -> Self {
        Char { chomp: -99.0, next_chomp: 2.0 + hash(seed) * 2.0, turn: -99.0, next_turn: 6.0 + hash(seed ^ 9) * 5.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_chomp {
            self.chomp = f.t;
            self.next_chomp = f.t + 3.0 + f.rand(f.t as u64 ^ 0x31) * 4.0;
        }
        if f.t > self.next_turn {
            self.turn = f.t;
            self.next_turn = f.t + 7.0 + f.rand(f.t as u64 ^ 0x41) * 6.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let es = f.since(f.times.error);
        if (FOLD..GONE).contains(&es) {
            return;
        }
        let dir = self.facing(f);
        // His size: growing back after dying, or appearing when Iris boots.
        let r = if (GONE..BACK).contains(&es) {
            R * ((es - GONE) / (BACK - GONE)).max(0.15)
        } else {
            R
        };
        c.ellipse(CX, CY, r, r, YELLOW);

        // The mouth: a wedge cut out, its half-angle up to 50° (and, dying, all the way round).
        let half = if (0.0..FOLD).contains(&es) {
            let u = es / FOLD;
            PI / 5.0 + (PI - PI / 5.0) * u.powf(0.8)
        } else {
            self.open(f) * PI * 0.28
        };
        if half > 0.03 {
            for y in 0..34 {
                for x in 0..34 {
                    let (dx, dy) = (x as f64 + 0.5 - CX, y as f64 + 0.5 - CY);
                    let mut d = dy.atan2(dx) - dir;
                    while d > PI {
                        d -= 2.0 * PI;
                    }
                    while d < -PI {
                        d += 2.0 * PI;
                    }
                    if d.abs() < half && dx * dx + dy * dy > 0.6 {
                        c.clear(x, y);
                    }
                }
            }
        }

        c.outline(LINE, &[]);
        c.shade(YELLOW, SHADE, LINE);
        c.light(YELLOW, LIGHT, LINE);
        // A round highlight up and to the left.
        if r > 8.0 {
            let k = r / R;
            c.recolor((CX - 9.0 * k) as i32, (CY - 10.0 * k) as i32, (4.0 * k) as i32, (2.0 * k).max(1.0) as i32, YELLOW, LIGHT);
            c.recolor((CX - 11.0 * k) as i32, (CY - 8.0 * k) as i32, (2.0 * k).max(1.0) as i32, (3.0 * k) as i32, YELLOW, LIGHT);
        }

        // The eye: above his mouth, whichever way he faces (gone once he folds past halfway).
        if half < PI * 0.55 && r > 8.0 {
            let k = r / R;
            let (ox, oy) = (1.5 * k, -9.0 * k);
            let (ex, ey) = if dir == RIGHT {
                (ox, oy)
            } else if dir == LEFT {
                (-ox - 2.0, oy)
            } else if dir == UP {
                (oy, -ox)
            } else {
                (-oy - 2.0, ox)
            };
            let mut eyes = f.eyes();
            if (0.0..FOLD).contains(&es) {
                eyes = Eyes::Cross;
            }
            if f.context > 0.85 && !f.asleep() && !f.blink && eyes == Eyes::Open {
                eyes = Eyes::Shut;
            }
            if f.state == State::Offline {
                eyes = Eyes::Shut;
            }
            let look = if f.thinking() { (0, -1) } else { f.look() };
            c.eye((CX + ex) as i32, (CY + ey) as i32, 2, 3, eyes, look, EYE, WHITE, YELLOW, dir != LEFT);
        }

        // Asleep: a nightcap, its tip drooping over to the side.
        if f.state == State::Sleeping {
            c.triangle((7.0, 6.0), (24.0, 3.0), (30.0, 12.0), WALL);
            c.triangle((7.0, 6.0), (13.0, 2.0), (24.0, 3.0), WALL);
            c.line(7, 6, 24, 3, WHITE);
            c.ellipse(31.0, 13.0, 2.0, 2.0, WHITE);
        }

        // Done: glowing with the power pellet, flashing.
        if let Some(u) = f.progress(f.times.done + 0.3, 1.5) {
            if ((u * 14.0) as i32) % 2 == 0 {
                c.swap(YELLOW, GLOW);
                c.swap(SHADE, YELLOW);
                c.swap(LINE, ORANGE);
            }
        }
        if f.state == State::Offline {
            for col in [YELLOW, SHADE, LIGHT] {
                c.swap(col, PALE);
            }
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        if f.is(&[State::Running, State::Testing]) {
            m.dx += 0.45 * (f.t * 1.6).sin();
        }
        if f.state == State::Booting {
            m.dy = 0.0;
        }
        // A gulp: the pellet goes down.
        if let Some(u) = f.progress(f.times.done + 0.3, 0.3) {
            let k = (u * PI).sin() * 0.08;
            m.sx *= 1.0 + k;
            m.sy *= 1.0 - k;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let es = f.since(f.times.error);
        let dir = self.facing(f);
        let asleep = f.asleep();

        // A stream of dots flowing into his mouth from where he faces.
        let stream = |w: &mut World, a: f64, speed: f64, big: bool, from: (f64, f64)| {
            for k in 0..6 {
                let p = 7.0 + (k as f64 * 6.0 - t * speed).rem_euclid(36.0);
                let (x, y) = (from.0 + a.cos() * p, from.1 + a.sin() * p);
                if big && k % 3 == 0 {
                    if (t * 6.0).sin() > -0.3 {
                        for (dx, dy) in [(-1.0, -1.0), (0.0, -1.0), (-1.0, 0.0), (0.0, 0.0), (-1.0, -2.0), (-2.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
                            w.px(x + dx, y + dy, DOT);
                        }
                    }
                } else {
                    for (dx, dy) in [(-1.0, -1.0), (0.0, -1.0), (-1.0, 0.0), (0.0, 0.0)] {
                        w.px(x + dx, y + dy, DOT);
                    }
                }
            }
        };
        if !asleep && f.groove < 0.1 && es > BACK {
            match f.state {
                State::Reading | State::Searching => stream(w, dir, 14.0, false, (CX, CY)),
                State::Web => stream(w, dir, 14.0, true, (CX, CY)),
                State::Editing => stream(w, dir, 8.0, false, (CX, CY)),
                State::Git => {
                    stream(w, dir, 12.0, false, (CX, CY));
                    // The branch, joining the line from above.
                    for k in 0..4 {
                        let p = (k as f64 * 5.0 - t * 12.0).rem_euclid(20.0);
                        let (x, y) = (CX - 20.0 - p * 0.7, CY - p * 0.7);
                        w.px(x, y, ORANGE);
                        w.px(x - 1.0, y, ORANGE);
                    }
                }
                _ => {}
            }
        }

        // Thinking, planning: a little maze above him, a dot running its corridors.
        if f.thinking() {
            let rect = |w: &mut World, x0: f64, y0: f64, x1: f64, y1: f64| {
                let mut x = x0;
                while x <= x1 {
                    w.px(x, y0, WALL);
                    w.px(x, y1, WALL);
                    x += 1.0;
                }
                let mut y = y0;
                while y <= y1 {
                    w.px(x0, y, WALL);
                    w.px(x1, y, WALL);
                    y += 1.0;
                }
            };
            rect(w, 4.0, -15.0, 13.0, -10.0);
            rect(w, 21.0, -15.0, 30.0, -10.0);
            rect(w, 0.0, -21.0, 34.0, -4.0);
            // The corridor: around the left block, through the middle, around the right.
            let path: [(f64, f64); 6] = [(2.0, -6.0), (2.0, -19.0), (17.0, -19.0), (17.0, -6.0), (32.0, -6.0), (32.0, -19.0)];
            let total = 13.0 + 15.0 + 13.0 + 15.0 + 13.0;
            let mut d = (t * 9.0) % total;
            for i in 0..path.len() - 1 {
                let (a, b) = (path[i], path[i + 1]);
                let len = (b.0 - a.0).abs() + (b.1 - a.1).abs();
                if d <= len {
                    let v = d / len;
                    let (x, y) = (a.0 + (b.0 - a.0) * v, a.1 + (b.1 - a.1) * v);
                    w.px(x, y, YELLOW);
                    w.px(x + 1.0, y, YELLOW);
                    w.px(x, y + 1.0, YELLOW);
                    w.px(x + 1.0, y + 1.0, YELLOW);
                    break;
                }
                d -= len;
            }
            for (x, y) in [(2.0, -12.0), (9.0, -19.0), (17.0, -12.0), (25.0, -6.0), (32.0, -12.0), (25.0, -19.0)] {
                w.px(x, y, DOT);
            }
        }

        // Speaking: "waka" ripples out of the mouth.
        if f.state == State::Speaking && f.voice > 0.04 {
            for k in 0..2 {
                let p = (t * 2.0 + k as f64 * 0.5) % 1.0;
                let r = 4.0 + p * 8.0;
                for s in -2..=2 {
                    let a = s as f64 * 0.28;
                    let (x, y) = (CX + 15.0 + a.cos() * r, CY + a.sin() * r);
                    if p < 0.7 { w.px(x, y, YELLOW) } else { w.px_dim(x, y, YELLOW) }
                }
            }
        }

        // Listening: sound coming in from the left.
        if f.is(&[State::Listening, State::Transcribing]) && f.voice > 0.02 {
            for k in 0..2 {
                let p = (t * 1.5 + k as f64 * 0.5) % 1.0;
                let r = 14.0 - p * 10.0;
                for s in -2..=2 {
                    let a = PI + s as f64 * 0.3;
                    w.px_dim(CX - 15.0 + a.cos() * r, CY + a.sin() * r, DOT);
                }
            }
        }

        // Asking: a power pellet before him, blinking.
        if f.state == State::Asking && (t * 4.0).sin() > -0.2 {
            for dy in -2..=1 {
                for dx in -2..=1 {
                    if !((dx == -2 || dx == 1) && (dy == -2 || dy == 1)) {
                        w.px(CX + 22.0 + dx as f64, CY + dy as f64, DOT);
                    }
                }
            }
        }

        // Running: Blinky on his tail; testing: he chases a frightened ghost.
        if f.is(&[State::Running, State::Testing]) {
            let behind = f.state == State::Running;
            let fx = dir.cos();
            let gx = if behind { CX - fx * 27.0 - 4.0 } else { CX + fx * 23.0 - 4.0 };
            let bob = (t * 9.0).sin();
            ghost(w, gx, CY - 4.0 + bob, RED, !behind, dir, t);
        }

        // Done: a power pellet into his mouth, then he glows: rays and "50".
        if let Some(u) = f.progress(f.times.done, 0.3) {
            let x = CX + 34.0 - u * 26.0;
            for dy in -2..=1 {
                for dx in -2..=1 {
                    if !((dx == -2 || dx == 1) && (dy == -2 || dy == 1)) {
                        w.px(x + dx as f64, CY + dy as f64, DOT);
                    }
                }
            }
        }
        if let Some(u) = f.progress(f.times.done + 0.25, 1.6) {
            let r = 18.0 + u * 10.0;
            for k in 0..12 {
                let a = k as f64 * PI / 6.0 + u;
                for s in 0..3 {
                    let rr = r + s as f64 * 1.5;
                    let (x, y) = (CX + a.cos() * rr, CY + a.sin() * rr);
                    if u < 0.7 { w.px(x, y, if s == 0 { WHITE } else { YELLOW }) } else { w.px_dim(x, y, YELLOW) }
                }
            }
            text(w, CX - 4.0, -4.0 - u * 8.0, "50", WHITE, u > 0.75);
        }

        // The death's pop: little lines bursting where he was.
        if let Some(u) = f.progress(f.times.error + FOLD, POP - FOLD) {
            for k in 0..8 {
                let a = k as f64 * PI / 4.0;
                for s in 0..3 {
                    let rr = 3.0 + u * 7.0 + s as f64;
                    w.px(CX + a.cos() * rr, CY + 8.0 + a.sin() * rr, YELLOW);
                }
            }
        }

        // A key: one dot, chomped.
        if let Some(u) = f.progress(f.times.key, 0.25) {
            let x = CX + dir.cos() * (17.0 - u * 12.0);
            let y = CY + dir.sin() * (17.0 - u * 12.0);
            w.px(x, y, DOT);
            w.px(x - 1.0, y, DOT);
            w.px(x, y - 1.0, DOT);
            w.px(x - 1.0, y - 1.0, DOT);
        }

        // Copying: a cherry pops up, bobbing; he eats it: "100".
        if let Some(u) = f.progress(f.times.copy, 2.8) {
            let s = u * 2.8;
            if s < 1.9 {
                let pop = (s / 0.2).min(1.0);
                let x = if s < 1.6 { CX + 20.0 } else { CX + 20.0 - (s - 1.6) / 0.3 * 12.0 };
                let y = CY - 5.0 + (1.0 - pop) * 8.0 + (t * 4.0).sin();
                cherry(w, x, y);
                if s < 0.4 {
                    for k in 0..6 {
                        let a = k as f64 * PI / 3.0;
                        w.px(x + 4.0 + a.cos() * (6.0 + s * 10.0), y + 5.0 + a.sin() * (6.0 + s * 10.0), WHITE);
                    }
                }
            } else {
                text(w, CX + 10.0, CY - 12.0 - (s - 1.9) * 6.0, "100", WHITE, s > 2.5);
            }
        }

        // Merge: two ghosts' worth of points.
        if let Some(u) = f.progress(f.times.merge, 1.0) {
            text(w, CX - 6.0, -4.0 - u * 6.0, "100", ORANGE, u > 0.7);
        }

        // Booting: "READY!"; offline: "GAME OVER".
        if f.state == State::Booting && (f.st < 1.2 || (t * 5.0).sin() > 0.0) {
            text(w, CX - 12.0, CY + 19.0, "READY!", YELLOW, false);
        }
        if f.state == State::Offline {
            text(w, CX - 18.0, CY + 19.0, "GAME", RED, false);
            text(w, CX + 2.0, CY + 19.0, "OVER", RED, false);
        }

        // Little context left: a sweat drop.
        if f.context > 0.85 && !asleep {
            let y = 4.0 + ((t * 0.8) % 1.0) * 4.0;
            w.px(30.0, y, DROP);
            w.px(30.0, y + 1.0, DROP);
            w.px(29.0, y + 1.0, DROP);
        }
    }
}
