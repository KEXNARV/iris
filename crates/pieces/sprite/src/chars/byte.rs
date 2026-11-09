//! Byte: un gatito hecho de píxeles verdes fosforescentes, escapado de una terminal vieja. Tiene
//! líneas de barrido como un monitor CRT, ojos que son cursores parpadeando, un collar con un LED
//! y una cola que es un cable de bits con su conector en la punta. What it does:
//! - waiting: the cursor eyes blink like a terminal, the bit-cable tail sways, phosphor specks
//!   flicker on its fur, the collar LED glows green;
//! - listening: ears straight up, cursors steady and wide, an oscilloscope trace by its ear;
//! - thinking, planning: its eyes turn into loading spinners (| / - \), the tail tip blinks;
//! - speaking: it purrs in binary — 0s and 1s float out of its mouth with the voice;
//! - reading, searching, on the web: characters rain down from above and it absorbs them;
//! - editing: paws typing, little lines of code being written beside it;
//! - running, testing: runs in place, ears back, ">" prompts streaming behind; tests pop ✓;
//! - git (and a merge): a commit graph grows beside it, a branch forking off and back;
//! - done: it turns into a rainbow, paws up, a rainbow trail and sparkles behind it;
//! - error: it glitches — a blue screen flash, then bands of it slip sideways, colors corrupt,
//!   broken pixels fly off, x x;
//! - a key: a bit hops off its paw; copying: Ctrl+C — a mini clone of itself pops out and hops;
//! - asking: head tilted, one ear folded; music: equalizer bars bounce beside it;
//! - asleep: curled into a loaf, standby LED breathing amber, fur dimmed;
//! - offline: grey, no signal: the tail's plug lies unplugged with static around it;
//! - booting: it assembles pixel by pixel under a scanning beam;
//! - compacting: compression artifacts (it goes blocky);
//! - little context left: a low-battery icon blinks over its head.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Byte";
pub const SCENE: &str = "code";
pub const DESCRIPTION: &str = "Byte, un gatito de pixeles verdes de terminal retro: ojos de cursor, cola de cable de bits, ronronea en binario, se glitchea con los errores y se vuelve arcoiris al terminar";

const LINE: u8 = 1;
const BODY: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const SCAN: u8 = 5;
const EYE: u8 = 6;
const DARK: u8 = 7;
const PINK: u8 = 8;
const MAGENTA: u8 = 9;
const CYAN: u8 = 10;
const RED: u8 = 11;
const ORANGE: u8 = 12;
const YELLOW: u8 = 13;
const BLUE: u8 = 14;
const GREY: u8 = 15;
const GREY_D: u8 = 16;

const RAINBOW: [u8; 7] = [RED, ORANGE, YELLOW, BODY, CYAN, BLUE, MAGENTA];

pub struct Char {
    /// The last idle ear flick, and the next.
    flick: f64,
    next_flick: f64,
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}

/// A 3×5 digit (0 or 1) in the world.
fn digit(w: &mut World, one: bool, x: f64, y: f64, col: u8, dim: bool) {
    const ZERO: [&str; 5] = ["###", "#.#", "#.#", "#.#", "###"];
    const ONE: [&str; 5] = [".#.", "##.", ".#.", ".#.", "###"];
    let rows = if one { ONE } else { ZERO };
    for (yy, row) in rows.iter().enumerate() {
        for (xx, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                if dim {
                    w.px_dim(x + xx as f64, y + yy as f64, col);
                } else {
                    w.px(x + xx as f64, y + yy as f64, col);
                }
            }
        }
    }
}

/// A tiny mini-Byte (the Ctrl+C clone), its feet at (x, y).
fn mini(w: &mut World, x: f64, y: f64, body: u8) {
    const ROWS: [&str; 8] = ["#...#.", "##.##.", "#####.", "#e#e#.", "#####.", ".###..", ".####.", ".#.#.."];
    for (yy, row) in ROWS.iter().enumerate() {
        for (xx, ch) in row.bytes().enumerate() {
            let col = match ch {
                b'#' => body,
                b'e' => EYE,
                _ => 0,
            };
            if col > 0 {
                w.px(x + xx as f64 - 2.5, y - 8.0 + yy as f64, col);
            }
        }
    }
    // Its tail: two bits.
    w.px(x + 3.0, y - 3.0, LIGHT);
    w.px(x + 4.0, y - 4.0, body);
}

impl Char {
    /// Where the head is (center) this frame, and whether it's curled asleep.
    fn head(&self, f: &Frame) -> (f64, f64) {
        let t = f.t;
        let mut hx = 18.0;
        let mut hy = 14.0;
        if f.asleep() {
            hy = 21.5;
        }
        if f.groove > 0.1 {
            hx += f.wave(PI * 2.2, 0.0) * 1.2 * f.groove;
            hy += (t * PI * 4.4).sin().abs() * 1.0 * f.groove;
        }
        if f.is(&[State::Running, State::Testing]) {
            hy += (t * 12.0).sin().abs() * 0.6;
        }
        if f.state == State::Editing {
            hy += (t * 9.0).sin().abs() * 0.4;
        }
        (hx, hy)
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#2fa85e", // line: a mid green, visible on black
        "#3dff7e", // phosphor body
        "#24cf62", // shade
        "#c2ffd6", // light / belly
        "#33e872", // scanline
        "#f2fff6", // cursor eyes
        "#062814", // mouth inside
        "#ff8fc8", // pink: nose, ears, cheeks
        "#ff3df0", // magenta (corruption)
        "#3df4ff", // cyan (corruption)
        "#ff4a4a", // red
        "#ffa93a", // orange
        "#fff04a", // yellow
        "#4a7cff", // blue
        "#9fb3a6", // offline grey
        "#6c8275", // offline grey line
    ];
    const W: usize = 36;
    const H: usize = 36;

    fn new(seed: u64) -> Self {
        Char { flick: -99.0, next_flick: 2.0 + hash(seed) * 3.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_flick {
            self.flick = f.t;
            self.next_flick = f.t + 3.0 + f.rand(f.t as u64 + 11) * 4.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let hurt = f.hurt();
        let asleep = f.asleep();
        let running = f.is(&[State::Running, State::Testing]);
        let cheer = f.within(f.times.done, 2.2);
        let (hx, hy) = self.head(f);

        // The tail: a cable of bits from its back, its plug at the tip. It sways idle, whips
        // running, wraps around its front asleep, lies unplugged offline.
        let n = 9;
        let mut pts = [(0.0f64, 0.0f64); 10];
        for (k, p) in pts.iter_mut().enumerate() {
            let s = k as f64 / n as f64;
            *p = if asleep {
                (29.0 - s * 21.0, 33.0 + (s * PI).sin() * 0.8)
            } else if running {
                (25.0 + s * 9.5, 30.0 - s * 6.0 + (t * 14.0 + s * 6.0).sin() * 1.5 * s)
            } else {
                let sway = if f.thinking() {
                    0.4
                } else if f.groove > 0.1 {
                    f.wave(PI * 2.2, -s * 2.0) * 2.5
                } else {
                    f.wave(2.0, -s * 2.5) * 1.6
                };
                (25.0 + s * 7.5 + sway * s, 31.0 - s * 17.0 + (s * PI).sin() * -1.0)
            };
        }
        for (k, p) in pts.iter().enumerate().take(n) {
            let col = if k % 2 == 0 { BODY } else { LIGHT };
            c.rect(p.0.round() as i32, p.1.round() as i32, 2, 2, col);
        }
        // The plug.
        let tip = pts[n];
        let (tx, ty) = (tip.0.round() as i32, tip.1.round() as i32);
        let blink_tip = f.thinking() && f.wave(8.0, 0.0) > 0.0;
        c.rect(tx - 1, ty - 1, 3, 3, if blink_tip { YELLOW } else { LIGHT });
        if !asleep {
            c.set(tx - 1, ty - 2, EYE);
            c.set(tx + 1, ty - 2, EYE);
        }

        // Ears: tips perk up listening, flatten hurt or asleep, fly back running, one folds
        // asking, flick now and then.
        let (mut lt, mut rt) = ((hx - 10.0, hy - 12.5), (hx + 10.0, hy - 12.5));
        if f.is(&[State::Listening, State::Transcribing]) || cheer {
            lt = (hx - 8.5, hy - 14.0);
            rt = (hx + 8.5, hy - 14.0);
        } else if hurt || asleep {
            lt = (hx - 13.0, hy - 7.5);
            rt = (hx + 13.0, hy - 7.5);
        } else if running {
            lt = (hx - 13.0, hy - 10.0);
            rt = (hx + 13.0, hy - 10.0);
        } else if f.state == State::Asking {
            rt = (hx + 13.0, hy - 7.0);
        }
        if let Some(u) = f.progress(self.flick, 0.3) {
            lt.0 -= (u * PI).sin() * 2.0;
            lt.1 += (u * PI).sin() * 1.5;
        }
        if f.groove > 0.1 {
            let b = f.wave(PI * 4.4, 0.0) * f.groove;
            lt.1 += b;
            rt.1 -= b;
        }
        let ear = |c: &mut Canvas, tip: (f64, f64), a: (f64, f64), b: (f64, f64)| {
            c.triangle(tip, a, b, BODY);
            let k = 0.55;
            let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0 + 1.0);
            c.triangle(
                (tip.0 + (m.0 - tip.0) * 0.2, tip.1 + (m.1 - tip.1) * 0.2),
                (m.0 + (a.0 - m.0) * k, m.1 + (a.1 - m.1) * k),
                (m.0 + (b.0 - m.0) * k, m.1 + (b.1 - m.1) * k),
                PINK,
            );
        };
        ear(c, lt, (hx - 10.5, hy - 2.5), (hx - 3.5, hy - 7.0));
        ear(c, rt, (hx + 3.5, hy - 7.0), (hx + 10.5, hy - 2.5));

        // Body: sitting up, or a loaf asleep. Paws: down, typing, running, up cheering.
        if asleep {
            c.ellipse(18.0, 29.5, 13.5, 5.5, BODY);
        } else {
            c.ellipse(18.0, 26.5, 8.5, 7.5, BODY);
            let (mut ly, mut ry) = (33.5, 33.5);
            let (mut lx, mut rx) = (14.0, 22.0);
            if cheer {
                ly = 19.5 + f.wave(16.0, 0.0);
                ry = 19.5 - f.wave(16.0, 0.0);
                lx = 7.5;
                rx = 28.5;
            } else if running {
                let s = f.wave(14.0, 0.0);
                ly -= s.max(0.0) * 2.0;
                ry -= (-s).max(0.0) * 2.0;
            } else if f.state == State::Editing || f.within(f.times.key, 0.15) {
                let s = f.wave(18.0, 0.0);
                ly -= s.max(0.0) * 1.5;
                ry -= (-s).max(0.0) * 1.5;
            } else if f.state == State::Asking {
                ry = 22.0 + f.wave(9.0, 0.0) * 0.8;
                rx = 25.5;
            }
            c.ellipse(lx, ly, 2.6, 1.8, BODY);
            c.ellipse(rx, ry, 2.6, 1.8, BODY);
            // The belly.
            c.ellipse(18.0, 27.5, 4.5, 5.0, LIGHT);
        }
        c.ellipse(hx, hy, 11.0, 8.5, BODY);

        c.outline(LINE, &[]);
        c.shade(BODY, SHADE, LINE);
        c.light(BODY, LIGHT, LINE);
        // CRT scanlines through its fur.
        let roll = (t * 6.0) as i32;
        for y in 0..36 {
            if (y + roll) % 3 == 0 {
                c.recolor(0, y, 36, 1, BODY, SCAN);
            }
        }
        // Phosphor specks flickering on its fur.
        let tick = (t * 7.0) as u64;
        for k in 0..6u64 {
            let (x, y) = ((f.rand(tick * 13 + k) * 36.0) as i32, (f.rand(tick * 17 + k + 50) * 36.0) as i32);
            if c.get(x, y) == BODY || c.get(x, y) == SCAN {
                c.set(x, y, LIGHT);
            }
        }

        // The collar, with its LED tag.
        if !asleep {
            let cy = (hy + 7.5).round() as i32;
            for x in (hx - 6.0).round() as i32..=(hx + 5.0).round() as i32 {
                if c.get(x, cy) != 0 && c.get(x, cy) != LINE {
                    c.set(x, cy, BLUE);
                }
            }
            let led = if hurt {
                if f.wave(30.0, 0.0) > 0.0 { RED } else { MAGENTA }
            } else if f.state == State::Offline {
                GREY_D
            } else if f.working() && f.wave(20.0, 0.0) > 0.0 {
                YELLOW
            } else {
                EYE
            };
            let lx = hx.round() as i32 - 1;
            c.rect(lx, cy + 1, 2, 2, led);
        } else {
            // Standby LED, breathing amber, on the curled front.
            let on = f.wave(1.6, 0.0) > -0.2;
            if f.state == State::Sleeping {
                c.rect(17, 31, 2, 1, if on { ORANGE } else { SHADE });
            }
        }

        // The face.
        let fy = hy.round() as i32;
        let fx = hx.round() as i32;
        let (el, er, ey) = (fx - 7, fx + 4, fy - 2);
        let mut eyes = if f.thinking() { Eyes::Open } else { f.eyes() };
        if f.state == State::Booting && f.st < 1.6 {
            eyes = Eyes::Open;
        }
        let look = f.look();
        if f.thinking() && !hurt && !f.happy() {
            // Loading spinners.
            let ph = ((t * 8.0) as i32).rem_euclid(4);
            for ex in [el, er] {
                let (cx, cy) = (ex + 1, ey + 1);
                let d: [(i32, i32); 2] = match ph {
                    0 => [(0, -1), (0, 1)],
                    1 => [(1, -1), (-1, 1)],
                    2 => [(-1, 0), (1, 0)],
                    _ => [(-1, -1), (1, 1)],
                };
                c.set(cx, cy, EYE);
                for (dx, dy) in d {
                    c.set(cx + dx, cy + dy, EYE);
                }
            }
        } else if eyes == Eyes::Open || eyes == Eyes::Wide {
            // Cursors: a block, blinking to an underscore like a terminal (steady listening).
            let steady = f.is(&[State::Listening, State::Transcribing, State::Asking]) || f.groove > 0.1;
            let on = steady || (t * 1.4).fract() < 0.62;
            for (k, ex) in [el, er].into_iter().enumerate() {
                let (x, y) = (ex + look.0, ey + look.1);
                if on {
                    let top = if eyes == Eyes::Wide { y - 1 } else { y };
                    c.rect(x, top, 3, y + 3 - top, EYE);
                    c.set(x + if k == 0 { 0 } else { 2 }, top, LIGHT);
                } else {
                    c.rect(x, y + 2, 3, 1, EYE);
                }
            }
        } else {
            c.eye(el, ey, 3, 3, eyes, look, EYE, 0, 0, false);
            c.eye(er, ey, 3, 3, eyes, look, EYE, 0, 0, true);
        }
        // Cheeks, nose, the cat mouth (open with the voice).
        if !asleep || f.state == State::Sleeping {
            c.rect(fx - 9, fy + 2, 2, 1, PINK);
            c.rect(fx + 7, fy + 2, 2, 1, PINK);
        }
        c.rect(fx - 1, fy + 1, 2, 1, PINK);
        let open = f.mouth();
        if f.happy() {
            c.mouth(fx - 1, fy + 3, 0.9, 5, 3, DARK, PINK);
        } else if open > 0.08 {
            c.mouth(fx - 1, fy + 3, open, 3, 3, DARK, PINK);
        } else {
            for (x, y) in [(-3, 2), (-2, 3), (-1, 2), (0, 2), (1, 3), (2, 2)] {
                c.set(fx + x, fy + y, LINE);
            }
        }
        // Whiskers.
        let wl = if hurt { 1 } else { 0 };
        for (dy, ey2) in [(0, -1), (2, 2)] {
            c.line(fx - 11, fy + dy, fx - 15, fy + ey2 + wl, LIGHT);
            c.line(fx + 10, fy + dy, fx + 14, fy + ey2 + wl, LIGHT);
        }

        // Offline: grey, no signal.
        if f.state == State::Offline {
            for col in [BODY, SCAN, LIGHT, SHADE, PINK, BLUE, YELLOW] {
                c.swap(col, GREY);
            }
            c.swap(LINE, GREY_D);
        }

        // Done: it turns into a rainbow, the bands scrolling through it.
        if let Some(u) = f.progress(f.times.done, 2.2) {
            let shift = (t * 14.0) as i32;
            let fade = u < 0.85 || ((u * 30.0) as i32) % 2 == 0;
            if fade {
                for y in 0..36i32 {
                    let col = RAINBOW[((y / 3 + 70 - shift).rem_euclid(7)) as usize];
                    for from in [BODY, SCAN, SHADE] {
                        c.recolor(0, y, 36, 1, from, col);
                    }
                }
            }
        }

        // An error: a blue-screen flash, then it glitches — bands slip, colors corrupt.
        if let Some(u) = f.progress(f.times.error, 1.4) {
            if u < 0.09 {
                for col in [BODY, SCAN, SHADE, LIGHT, PINK, LINE] {
                    c.swap(col, BLUE);
                }
            } else {
                let tick = (t * 16.0) as u64;
                let bands = if u < 0.6 { 4 } else { 2 };
                for b in 0..bands as u64 {
                    let r = |k: u64| f.rand(tick * 31 + b * 7 + k);
                    let y0 = (r(1) * 36.0) as i32;
                    let h = 1 + (r(2) * 4.0) as i32;
                    let dx = ((r(3) - 0.5) * 10.0) as i32;
                    let old = c.px;
                    for y in y0..(y0 + h).min(36) {
                        for x in 0..36i32 {
                            let sx = x - dx;
                            let v = if (0..36).contains(&sx) { old[y as usize * 48 + sx as usize] } else { 0 };
                            c.px[y as usize * 48 + x as usize] = v;
                        }
                    }
                    let to = if r(4) < 0.5 { MAGENTA } else { CYAN };
                    if r(5) < 0.7 {
                        for from in [BODY, SCAN, SHADE, LIGHT] {
                            c.recolor(0, y0, 36, h, from, to);
                        }
                    }
                }
            }
        }

        // Compacting: compression artifacts, going blocky in pulses.
        if f.state == State::Compacting && f.wave(2.5, 0.0) > 0.2 {
            let old = c.px;
            for y in 0..36usize {
                for x in 0..36usize {
                    c.px[y * 48 + x] = old[(y & !1) * 48 + (x & !1)];
                }
            }
        }

        // Booting: assembling pixel by pixel under a scanning beam.
        if f.state == State::Booting && f.st < 1.6 {
            let p = f.st / 1.4;
            let beam = (p * 40.0) as i32 - 2;
            for y in 0..36i32 {
                for x in 0..36i32 {
                    let keep = hash(f.seed ^ ((x * 64 + y) as u64 * 0x9E37)) < p * 1.1 - 0.05;
                    if !keep && y > beam - 6 {
                        c.clear(x, y);
                    }
                }
            }
            if (0..36).contains(&beam) {
                for x in 0..36 {
                    if x % 2 == 0 || c.get(x, beam) != 0 {
                        c.set(x, beam, EYE);
                    }
                }
            }
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // Glitching makes it jump sideways in whole steps.
        if let Some(u) = f.progress(f.times.error, 1.2) {
            let k = f.rand((f.t * 12.0) as u64);
            if k < 0.35 {
                m.dx += (k - 0.17) * 0.5 * (1.0 - u);
            }
        }
        if f.state == State::Asking {
            m.lean += 0.05;
        }
        if f.is(&[State::Running, State::Testing]) {
            m.lean += 0.06;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let (hx, hy) = self.head(f);

        // Speaking: it purrs in binary — 0s and 1s floating out of its mouth.
        if f.state == State::Speaking {
            for k in 0..5u64 {
                let u = (t * 0.9 + k as f64 * 0.2).fract();
                let cycle = (t * 0.9 + k as f64 * 0.2).floor() as u64;
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                let x = hx + side * (6.0 + u * 12.0) + (u * 9.0 + k as f64).sin() * 1.5 - 1.5;
                let y = hy + 2.0 - u * 18.0;
                if f.voice > 0.02 || u < 0.5 {
                    let one = f.rand(cycle * 7 + k) < 0.5;
                    digit(w, one, x, y, if k % 3 == 0 { LIGHT } else { BODY }, u > 0.7);
                }
            }
        }

        // Reading, searching, on the web: characters rain down and get absorbed into its head.
        if f.reading() {
            for k in 0..7u64 {
                let u = (t * 0.7 + k as f64 / 7.0).fract();
                let cycle = (t * 0.7 + k as f64 / 7.0).floor() as u64;
                let x0 = -8.0 + f.rand(k * 3 + cycle * 5) * 52.0;
                let ease = u * u;
                let x = x0 + (hx - x0) * ease;
                let y = -16.0 + u * (hy - 2.0 + 16.0);
                // A glyph: a random 3×3 pattern, a dim trail above it.
                let g = (f.rand(cycle * 11 + k + 900) * 512.0) as u32 | 0b010_000_000;
                for b in 0..9 {
                    if g >> b & 1 == 1 {
                        w.px(x + (b % 3) as f64, y + (b / 3) as f64, if u > 0.8 { LIGHT } else { BODY });
                    }
                }
                for s in 1..4 {
                    w.px_dim(x + 1.0 - (hx - x0) * 0.02 * s as f64, y - 2.0 - s as f64 * 2.0, SHADE);
                }
            }
        }

        // Listening: an oscilloscope trace by its ear.
        if f.is(&[State::Listening, State::Transcribing]) {
            let amp = 0.6 + (f.voice * 18.0).min(3.0);
            for k in 0..11 {
                let x = 33.0 + k as f64;
                let y = 8.0 + (t * 10.0 + k as f64 * 0.9).sin() * amp;
                w.px(x, y.round(), if k % 4 == 0 { LIGHT } else { BODY });
            }
            for k in 0..11 {
                w.px_dim(33.0 + k as f64, 12.0, LINE);
            }
        }

        // Editing: lines of code being typed beside it, a cursor at the end.
        if f.state == State::Editing {
            let p = (t * 0.6).fract();
            let total = p * 40.0;
            for row in 0..4 {
                let len = (3.0 + f.rand(row as u64 + (t * 0.6) as u64 * 4) * 7.0).floor();
                let done = (total - row as f64 * 10.0).clamp(0.0, len);
                let indent = if row == 1 || row == 2 { 2.0 } else { 0.0 };
                for k in 0..done as i32 {
                    let col = if k == 0 { CYAN } else if row % 2 == 0 { BODY } else { YELLOW };
                    w.px(-12.0 + indent + k as f64, 14.0 + row as f64 * 2.0, col);
                }
                if done > 0.0 && done < len && f.wave(20.0, 0.0) > 0.0 {
                    w.px(-12.0 + indent + done, 13.5 + row as f64 * 2.0, EYE);
                }
            }
        }

        // Running, testing: ">" prompts streaming behind; tests pop ✓ marks.
        if f.is(&[State::Running, State::Testing]) {
            for k in 0..4 {
                let u = (t * 1.8 + k as f64 * 0.25).fract();
                let (x, y) = (2.0 - u * 18.0, 18.0 + k as f64 * 4.0);
                let col = if k % 2 == 0 { BODY } else { LIGHT };
                for (dx, dy) in [(0.0, 0.0), (1.0, 1.0), (0.0, 2.0)] {
                    if u < 0.75 {
                        w.px(x + dx, y + dy, col);
                    } else {
                        w.px_dim(x + dx, y + dy, col);
                    }
                }
            }
            if f.state == State::Testing {
                for k in 0..3 {
                    let u = (t * 0.8 + k as f64 / 3.0).fract();
                    let (x, y) = (36.0 + k as f64 * 3.0, 20.0 - u * 16.0);
                    for (dx, dy) in [(0.0, 2.0), (1.0, 3.0), (2.0, 2.0), (3.0, 1.0), (4.0, 0.0)] {
                        w.px(x + dx, y + dy, if u < 0.8 { BODY } else { SHADE });
                    }
                }
            }
        }

        // Git: a commit graph growing beside it, a branch forking off and merging back.
        if f.state == State::Git || f.within(f.times.merge, 1.2) {
            let p = (t * 0.5).fract();
            let (x, top) = (-6.0, 6.0);
            for y in 0..26 {
                w.px(x, top + y as f64, LINE);
            }
            let bx = x - 5.0;
            for y in 0..14 {
                let yy = top + 6.0 + y as f64;
                let off = if y < 3 { y as f64 * 5.0 / 3.0 } else if y > 10 { (13 - y) as f64 * 5.0 / 3.0 } else { 5.0 };
                w.px(x - off, yy, ORANGE);
            }
            let node = |w: &mut World, x: f64, y: f64, col: u8| {
                for (dx, dy) in [(0.0, -1.0), (-1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
                    w.px(x + dx, y + dy, col);
                }
            };
            let n = (p * 6.0) as i32;
            for k in 0..=n.min(4) {
                node(w, x, top + 24.0 - k as f64 * 6.0, BODY);
            }
            if n >= 2 {
                node(w, bx, top + 13.0, ORANGE);
            }
            if n >= 3 {
                node(w, bx, top + 17.0, ORANGE);
            }
            if f.within(f.times.merge, 1.2) && f.wave(20.0, 0.0) > 0.0 {
                node(w, x, top + 6.0, EYE);
            }
        }

        // Done: a rainbow trail behind it and sparkles.
        if let Some(u) = f.progress(f.times.done, 2.2) {
            let len = (u * 4.0).min(1.0) * 22.0;
            for k in 0..len as i32 {
                let x = -1.0 - k as f64;
                let wob = ((k / 3) % 2) as f64;
                for (b, col) in RAINBOW.iter().enumerate().take(6) {
                    let y = 18.0 + b as f64 * 1.0 + wob;
                    if u < 0.8 || k % 2 == 0 {
                        w.px(x, y, *col);
                    }
                }
            }
            for k in 0..8 {
                let a = k as f64 * PI / 4.0 + u * 1.5;
                let r = 8.0 + u * 20.0;
                let (x, y) = (18.0 + a.cos() * r, 16.0 + a.sin() * r * 0.8);
                let col = RAINBOW[k % 7];
                w.px(x, y, col);
                if (t * 12.0 + k as f64).sin() > 0.0 {
                    w.px(x - 1.0, y, col);
                    w.px(x + 1.0, y, col);
                    w.px(x, y - 1.0, col);
                    w.px(x, y + 1.0, col);
                }
            }
        }

        // An error: broken pixels flying off, corrupted blocks flickering around it.
        if let Some(u) = f.progress(f.times.error, 1.4) {
            let tick = (t * 10.0) as u64;
            for k in 0..6u64 {
                let r = |j: u64| f.rand(tick * 19 + k * 5 + j + 300);
                if r(0) < 0.6 {
                    let (x, y) = (-6.0 + r(1) * 48.0, -2.0 + r(2) * 36.0);
                    let (bw, bh) = (1 + (r(3) * 4.0) as i32, 1 + (r(4) * 2.0) as i32);
                    let col = [MAGENTA, CYAN, RED, BODY][(r(5) * 4.0) as usize];
                    for yy in 0..bh {
                        for xx in 0..bw {
                            w.px(x + xx as f64, y + yy as f64, col);
                        }
                    }
                }
            }
            for k in 0..5 {
                let v = 0.6 + k as f64 * 0.35;
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let (x, y) = (18.0 + dir * (6.0 + u * 16.0 * v), 10.0 - u * 10.0 * v + u * u * 26.0);
                w.px(x, y, if k % 2 == 0 { MAGENTA } else { CYAN });
            }
        }

        // A key: a bit hops off its paw.
        if let Some(u) = f.progress(f.times.key, 0.5) {
            let side = if (f.times.key * 10.0) as u64 % 2 == 0 { -1.0 } else { 1.0 };
            let one = (f.times.key * 7.0) as u64 % 2 == 0;
            digit(w, one, 17.0 + side * (6.0 + u * 6.0), 28.0 - (u * PI).sin() * 8.0 - u * 2.0, LIGHT, u > 0.7);
        }

        // Copying: Ctrl+C — a mini clone of it pops out and hops.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let out = ease_out((u * 4.0).min(1.0));
            let hop = ((u * 3.0 * PI).sin()).abs() * 4.0 * (1.0 - u);
            let flick = u > 0.85 && ((u * 40.0) as i32) % 2 == 0;
            if !flick {
                mini(w, 30.0 + out * 8.0, 35.5 - hop, if u < 0.15 { LIGHT } else { BODY });
            }
            // "^C" over it for a moment.
            if u < 0.4 {
                let (x, y) = (35.0 + out * 6.0, 20.0 - u * 6.0);
                for (dx, dy) in [(1.0, 0.0), (0.0, 1.0), (2.0, 1.0)] {
                    w.px(x + dx, y + dy, YELLOW);
                }
                for (dx, dy) in [(5.0, 0.0), (6.0, 0.0), (4.0, 1.0), (4.0, 2.0), (4.0, 3.0), (5.0, 4.0), (6.0, 4.0)] {
                    w.px(x + dx, y + dy, YELLOW);
                }
            }
        }

        // Music: equalizer bars bouncing beside it.
        if f.groove > 0.1 {
            for (k, x) in [-8.0, -5.0, -2.0, 38.0, 41.0, 44.0].iter().enumerate() {
                let h = (2.0 + (f.beat * 4.0 + (t * (5.0 + k as f64) + k as f64).sin().abs() * 6.0) * f.groove).round() as i32;
                for y in 0..h {
                    let col = if y >= 8 { RED } else if y >= 5 { YELLOW } else { BODY };
                    w.px(*x, 34.0 - y as f64, col);
                    w.px(*x + 1.0, 34.0 - y as f64, col);
                }
            }
        }

        // Offline: no signal — static flickering around its head.
        if f.state == State::Offline {
            let tick = (t * 8.0) as u64;
            for k in 0..8u64 {
                if f.rand(tick * 23 + k) < 0.5 {
                    let (x, y) = (-4.0 + f.rand(tick * 29 + k) * 44.0, f.rand(tick * 37 + k) * 30.0);
                    w.px_dim(x, y, GREY);
                }
            }
            // The unplugged cable's spark, now and then.
            if f.wave(1.3, 0.0) > 0.85 {
                w.px(36.0, 31.0, YELLOW);
                w.px(37.0, 30.0, EYE);
            }
        }

        // Little context left: a low-battery icon blinking over its head.
        if f.context > 0.85 && !f.asleep() {
            let (x, y) = (24.0, -4.0);
            for xx in 0..7 {
                w.px(x + xx as f64, y, LIGHT);
                w.px(x + xx as f64, y + 4.0, LIGHT);
            }
            for yy in 1..4 {
                w.px(x, y + yy as f64, LIGHT);
                w.px(x + 6.0, y + yy as f64, LIGHT);
            }
            w.px(x + 7.0, y + 2.0, LIGHT);
            if f.wave(6.0, 0.0) > 0.0 {
                for yy in 1..4 {
                    w.px(x + 1.0, y + yy as f64, RED);
                }
            }
        }
    }
}
