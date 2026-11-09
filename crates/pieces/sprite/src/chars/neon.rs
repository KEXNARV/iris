//! Neón: un robot de los 80 con cabeza de monitor CRT beige, antenas de conejo con bolitas de
//! neón, un visor de luces tipo KITT en la barbilla del monitor y una ranura de impresora en el
//! pecho. Su cara ES la pantalla: ojos de fósforo cian que cambian por íconos según lo que hace.
//! What it does:
//! - waiting: the CRT refresh line rolls down the screen, the KITT light sweeps slowly, the
//!   antennas sway, it winks now and then, and an antenna crackles with a spark;
//! - listening: eyes wide, an equalizer dancing with the voice on the screen;
//! - thinking: an hourglass on the screen that drains and flips, a hand at its "chin", the
//!   antenna balls blinking in turn; planning: a checklist ticking itself off;
//! - speaking: an oscilloscope mouth that follows the voice, the antenna balls flashing;
//! - reading, searching, on the web: the red scanner sweeps side to side across the screen and
//!   its eyes follow it (a magnifier or a globe in the corner), the KITT light racing;
//! - editing: lines of code typing themselves with a blinking cursor, hands typing;
//! - running, testing: a progress bar filling, marching on the spot, arms swinging;
//! - git: a branch icon with a commit running along it; a merge: two lines joining;
//! - compacting: the picture collapses into a line like an old CRT, and comes back;
//! - done: a beating pixel heart on the screen, a robot dance in stiff poses, neon lasers out
//!   of its antennas and a burst of neon sparks;
//! - error: static, then a blue screen ":(" — sparks off the antennas and smoke out the top;
//! - a key: a blip of the KITT light and a spark; nope: a red X; copying: it prints a ticket
//!   out of the slot in its chest that tears off and flutters away;
//! - asking: a big "?" on the screen, waving; music: equalizer face and the robot dance;
//! - asleep: screen dimmed with closed eyes, antennas drooping, the power light breathing;
//! - offline: color bars and "NO SIGNAL", its plug lying unplugged beside it;
//! - booting: the CRT powers on (a dot, a line, the picture) and it waves hello;
//! - little context left: a blinking low-battery icon, droopy eyes.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Neón";
pub const SCENE: &str = "city";
pub const DESCRIPTION: &str = "Neón, un robot ochentero con cabeza de monitor CRT: su pantalla muestra caras e íconos, escanea en rojo al leer, se cuelga con pantalla azul y humo, imprime tickets al copiar y celebra con baile robótico";

const LINE: u8 = 1;
const CASE: u8 = 2;
const CASE_SH: u8 = 3;
const CASE_LI: u8 = 4;
const SCREEN: u8 = 5;
const GLOW: u8 = 6;
const GLOW_DIM: u8 = 7;
const RED: u8 = 8;
const RED_DIM: u8 = 9;
const PINK: u8 = 10;
const YELLOW: u8 = 11;
const BSOD: u8 = 12;
const WHITE: u8 = 13;
const SMOKE: u8 = 14;
const DARK: u8 = 15;
const METAL: u8 = 16;

/// The screen, in canvas pixels: x 7..=29, y 9..=21 (its center column is 18).
const SX: i32 = 7;
const SY: i32 = 9;
const SW: i32 = 23;
const SH: i32 = 13;

const HOURGLASS: [&str; 9] = ["#######", ".#...#.", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", ".#...#.", "#######"];
const HEART_S: [&str; 6] = [".##.##.", "#o#####", "#######", ".#####.", "..###..", "...#..."];
const HEART_B: [&str; 8] = [".###.###.", "#o#######", "#oo######", "#########", ".#######.", "..#####..", "...###...", "....#...."];
const QUESTION: [&str; 9] = [".####.", "##..##", "##..##", "...##.", "..##..", "..##..", "......", "..##..", "..##.."];
const SAD: [&str; 7] = ["...#", "#.#.", "..#.", "..#.", "..#.", "#.#.", "...#"];
const XMARK: [&str; 7] = ["#.....#", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", "#.....#"];
const LENS: [&str; 5] = [".##..", "#..#.", "#..#.", ".##..", "...##"];
const GLOBE: [&str; 5] = [".###.", "#.#.#", "#####", "#.#.#", ".###."];
const BATTERY: [&str; 4] = ["#####.", "#o..##", "#o..##", "#####."];
const MIC: [&str; 5] = [".#.", "###", "###", ".#.", "###"];

/// Its two arms this frame: (elbow, hand) for the left; the right as given (not mirrored).
type Arm = ((f64, f64), (f64, f64));

pub struct Char {
    /// A wink while waiting, and the next one.
    wink: f64,
    next_wink: f64,
    /// An antenna spark, and the next one (and which antenna).
    spark: f64,
    spark_left: bool,
    next_spark: f64,
}

fn glyph(c: &mut Canvas, x: i32, y: i32, rows: &[&str], a: u8, b: u8) {
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            let col = match ch {
                b'#' => a,
                b'o' => b,
                _ => 0,
            };
            c.set(x + i as i32, y + j as i32, col);
        }
    }
}

/// A letter of its 3×5 font (N is 4 wide, I is 1).
fn letter(ch: char) -> [&'static str; 5] {
    match ch {
        'N' => ["#..#", "##.#", "#.##", "#..#", "#..#"],
        'O' => [".#.", "#.#", "#.#", "#.#", ".#."],
        'S' => [".##", "#..", ".#.", "..#", "##."],
        'I' => ["#", "#", "#", "#", "#"],
        'G' => [".##", "#..", "#.#", "#.#", ".##"],
        'A' => [".#.", "#.#", "###", "#.#", "#.#"],
        'L' => ["#..", "#..", "#..", "#..", "###"],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        _ => ["...", "...", "...", "...", "..."],
    }
}

/// A word in its font, centered on the screen, its top at `y`.
fn word(c: &mut Canvas, s: &str, y: i32, col: u8) {
    let w: i32 = s.chars().map(|ch| letter(ch)[0].len() as i32 + 1).sum::<i32>() - 1;
    let mut x = SX + (SW - w) / 2;
    for ch in s.chars() {
        let l = letter(ch);
        glyph(c, x, y, &l, col, col);
        x += l[0].len() as i32 + 1;
    }
}

/// A glyph centered on the screen's middle column, its top at `y`.
fn centered(c: &mut Canvas, rows: &[&str], y: i32, a: u8, b: u8) {
    let w = rows[0].len() as i32;
    glyph(c, 18 - (w - 1) / 2, y, rows, a, b);
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u.clamp(0.0, 1.0)).powi(3)
}

/// The left arm's pose mirrored to the right.
fn mirror(a: Arm) -> Arm {
    ((37.0 - a.0 .0, a.0 .1), (37.0 - a.1 .0, a.1 .1))
}

const REST: Arm = ((6.0, 33.5), (6.0, 37.5));
const UP: Arm = ((5.0, 27.0), (4.5, 21.5));
const BENT_UP: Arm = ((4.0, 30.5), (4.0, 25.0));
const BENT_DOWN: Arm = ((4.0, 30.5), (4.0, 36.0));
const FORWARD: Arm = ((6.0, 34.0), (10.0, 36.0));
const HANG: Arm = ((6.5, 34.5), (7.5, 39.0));

impl Char {
    /// Which robot-dance pose (0..4), or none.
    fn dance(&self, f: &Frame) -> Option<usize> {
        if let Some(u) = f.progress(f.times.done, 2.6) {
            return Some(((u * 2.6 * 4.0) as usize) % 4);
        }
        if f.groove > 0.1 && !f.asleep() {
            return Some(((f.t * 2.2) as usize) % 4);
        }
        None
    }

    fn arms(&self, f: &Frame) -> (Arm, Arm) {
        let t = f.t;
        if let Some(u) = f.progress(f.times.error, 1.6) {
            return if u < 0.35 { (UP, mirror(UP)) } else { (HANG, mirror(HANG)) };
        }
        if let Some(p) = self.dance(f) {
            return match p {
                0 => (BENT_UP, mirror(BENT_DOWN)),
                1 => (FORWARD, mirror(FORWARD)),
                2 => (BENT_DOWN, mirror(BENT_UP)),
                _ => (UP, mirror(UP)),
            };
        }
        if f.asleep() {
            return (HANG, mirror(HANG));
        }
        let wave = |f: &Frame| -> Arm { ((33.5, 27.5), (34.5 + f.wave(12.0, 0.0) * 1.5, 21.5)) };
        match f.state {
            State::Asking => (REST, wave(f)),
            State::Booting if f.st > 1.3 => (REST, wave(f)),
            State::Editing => {
                let k = f.wave(16.0, 0.0);
                (((6.5, 34.0), (11.0, 35.5 + k)), ((30.5, 34.0), (26.0, 35.5 - k)))
            }
            State::Running | State::Testing => {
                let k = f.wave(12.0, 0.0) * 2.5;
                (((6.0, 33.5), (6.0 + k, 37.0)), ((31.0, 33.5), (31.0 - k, 37.0)))
            }
            _ if f.thinking() => (REST, ((33.0, 33.0), (29.0, 28.5))),
            _ => {
                let s = f.wave(1.7, 0.0) * 0.4;
                (((6.0, 33.5), (6.0 - s, 37.5)), ((31.0, 33.5), (31.0 + s, 37.5)))
            }
        }
    }

    /// The antenna tips (left, right).
    fn antennas(&self, f: &Frame) -> ((f64, f64), (f64, f64)) {
        if f.asleep() {
            return ((7.5, 5.0), (29.5, 5.0));
        }
        let mut sway = f.wave(1.3, 0.0) * 0.8;
        if f.groove > 0.1 {
            sway += f.wave(PI * 4.4, 0.0) * 1.5 * f.groove;
        }
        if f.is(&[State::Running, State::Testing]) {
            sway = -1.5 + f.wave(24.0, 0.0) * 0.6;
        }
        let (mut l, mut r) = ((10.5 + sway, 1.5), (26.5 + sway, 1.5));
        if f.is(&[State::Listening, State::Transcribing]) {
            l = (13.0, 0.8);
            r = (24.0, 0.8);
        }
        if f.context > 0.85 {
            l.1 += 1.5;
            r.1 += 1.5;
            l.0 -= 1.0;
            r.0 += 1.0;
        }
        (l, r)
    }

    /// The screen's picture this frame.
    fn screen(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let frame = (t * 25.0) as u64;
        c.rect(SX, SY, SW, SH, SCREEN);
        // Booting: the CRT powers on — a dot, a line, the picture opening up.
        if f.state == State::Booting && f.st < 1.3 {
            let s = f.st;
            if s < 0.35 {
                c.set(18, 15, WHITE);
                if s > 0.15 {
                    c.set(17, 15, GLOW);
                    c.set(19, 15, GLOW);
                }
            } else if s < 0.7 {
                let half = (((s - 0.35) / 0.35) * 11.0) as i32;
                c.line(18 - half, 15, 18 + half, 15, WHITE);
            } else {
                let half = (((s - 0.7) / 0.6) * 6.5) as i32;
                c.rect(SX, 15 - half, SW, half * 2 + 1, GLOW_DIM);
                c.line(SX, 15, SX + SW - 1, 15, WHITE);
                if s > 1.0 {
                    word(c, "OK", 13, WHITE);
                }
            }
            return;
        }
        // Offline: color bars, then "NO SIGNAL".
        if f.state == State::Offline {
            if (t / 2.5) as i64 % 2 == 0 {
                let bars = [WHITE, YELLOW, GLOW, PINK, RED, BSOD];
                for y in SY..SY + SH {
                    for x in SX..SX + SW {
                        let col = if y > SY + 9 { if (x - SX) % 8 < 4 { BSOD } else { DARK } } else { bars[((x - SX) * 6 / SW) as usize] };
                        c.set(x, y, col);
                    }
                }
            } else {
                c.rect(SX, SY, SW, SH, DARK);
                word(c, "NO", 10, WHITE);
                word(c, "SIGNAL", 16, WHITE);
            }
            return;
        }
        // An error: static, then the blue screen.
        if let Some(u) = f.progress(f.times.error, 2.2) {
            if u < 0.18 {
                for y in SY..SY + SH {
                    for x in SX..SX + SW {
                        let r = hash(frame.wrapping_mul(7919) ^ (y * 64 + x) as u64);
                        c.set(x, y, if r < 0.3 { WHITE } else if r < 0.6 { SMOKE } else { DARK });
                    }
                }
            } else {
                c.rect(SX, SY, SW, SH, BSOD);
                glyph(c, 10, 11, &SAD, WHITE, WHITE);
                // Some white "text" under it, and a blinking cursor.
                c.line(17, 12, 27, 12, SMOKE);
                c.line(17, 14, 25, 14, SMOKE);
                c.line(17, 16, 27, 16, SMOKE);
                c.line(10, 19, 22, 19, WHITE);
                if f.wave(10.0, 0.0) > 0.0 {
                    c.rect(24, 19, 2, 1, WHITE);
                }
            }
            return;
        }
        // The refresh line rolling down (under everything).
        let roll = SY + ((t * 4.0) as i32).rem_euclid(SH + 6);
        if roll < SY + SH {
            c.line(SX, roll, SX + SW - 1, roll, GLOW_DIM);
        }
        // Asleep: dim, eyes closed.
        if f.state == State::Sleeping {
            c.rect(SX, SY, SW, SH, SCREEN);
            c.eye(11, 13, 3, 2, Eyes::Asleep, (0, 0), GLOW_DIM, 0, 0, false);
            c.eye(23, 13, 3, 2, Eyes::Asleep, (0, 0), GLOW_DIM, 0, 0, true);
            if f.wave(0.9, 0.0) > 0.6 {
                c.set(18, 18, GLOW_DIM);
            }
            return;
        }
        // Done: a beating heart.
        if f.within(f.times.done, 2.6) {
            if f.wave(14.0, 0.0) > -0.2 {
                centered(c, &HEART_B, 11, PINK, WHITE);
            } else {
                centered(c, &HEART_S, 12, PINK, WHITE);
            }
            return;
        }
        // Nope: a red X.
        if f.within(f.times.nope, 0.7) {
            centered(c, &XMARK, 12, RED, RED);
            return;
        }
        // A merge: two lines joining into one.
        if let Some(u) = f.progress(f.times.merge, 1.0) {
            let k = ((1.0 - u) * 5.0) as i32;
            c.line(12, 11, 18 - k / 2, 15, GLOW);
            c.line(24, 11, 18 + k / 2, 15, GLOW);
            c.line(18, 15, 18, 20, GLOW);
            c.set(12, 11, WHITE);
            c.set(24, 11, WHITE);
            c.set(18, 20, WHITE);
            return;
        }
        let look = f.look();
        let eyes = if f.context > 0.85 && f.eyes() == Eyes::Open { Eyes::Shut } else { f.eyes() };
        match f.state {
            State::Thinking => {
                // The hourglass: sand from the top into the bottom, then a flip.
                let cyc = (t / 2.2).fract();
                if cyc > 0.88 {
                    // Mid-flip: on its side.
                    let rows = ["#.....#", "##...##", "#.#.#.#", "#..#..#", "#.#.#.#", "##...##", "#.....#"];
                    glyph(c, 15, 12, &rows, GLOW, GLOW);
                } else {
                    let p = cyc / 0.88;
                    centered(c, &HOURGLASS, 11, GLOW, GLOW);
                    let top = ((1.0 - p) * 3.0).ceil() as i32;
                    let bot = (p * 3.0).ceil() as i32;
                    for k in 0..top {
                        let y = 14 - k;
                        let half = 2 - k.min(2);
                        let half = if k == 0 { 0 } else { half.max(1) };
                        c.line(18 - half, y, 18 + half, y, YELLOW);
                    }
                    for k in 0..bot {
                        let y = 18 - k;
                        let half = if k == 0 { 2 } else { 1 };
                        c.line(18 - half, y, 18 + half, y, YELLOW);
                    }
                    if p < 0.95 && f.wave(20.0, 0.0) > -0.5 {
                        c.set(18, 15, YELLOW);
                        c.set(18, 16, YELLOW);
                    }
                }
            }
            State::Planning => {
                // A checklist ticking itself off.
                let done = ((t * 0.8) as i32) % 4;
                for k in 0..3 {
                    let y = 11 + k * 4;
                    c.rect(10, y, 3, 3, GLOW);
                    c.set(11, y + 1, SCREEN);
                    c.line(15, y + 1, 15 + 9 - k * 2, y + 1, GLOW_DIM);
                    if k < done {
                        c.set(10, y, YELLOW);
                        c.set(11, y + 1, YELLOW);
                        c.set(12, y, YELLOW);
                        c.set(12, y - 1, YELLOW);
                        c.set(13, y - 2, YELLOW);
                    }
                }
            }
            State::Asking => {
                let bob = if f.wave(5.0, 0.0) > 0.0 { 0 } else { 1 };
                centered(c, &QUESTION, 10 + bob, GLOW, GLOW);
                c.set(19, 10 + bob, WHITE);
            }
            State::Editing => {
                // Code typing itself, a blinking cursor at its end.
                let typed = ((t * 9.0) as i32) % 60;
                let lens = [9, 14, 6, 11, 16];
                let indent = [0, 2, 4, 4, 2];
                let mut left = typed;
                for (k, (&len, &ind)) in lens.iter().zip(indent.iter()).enumerate() {
                    let y = 10 + k as i32 * 2;
                    let n = left.min(len);
                    if n > 0 {
                        c.line(9 + ind, y, 9 + ind + n - 1, y, if k % 2 == 0 { GLOW } else { GLOW_DIM });
                        c.set(9 + ind, y, PINK);
                    }
                    if left < len {
                        if f.wave(12.0, 0.0) > 0.0 {
                            c.rect(9 + ind + n, y - 1, 1, 2, WHITE);
                        }
                        break;
                    }
                    left -= len;
                }
            }
            State::Running | State::Testing => {
                c.eye(12, 11, 2, 2, Eyes::Squint, (0, 0), GLOW, 0, 0, false);
                c.eye(23, 11, 2, 2, Eyes::Squint, (0, 0), GLOW, 0, 0, true);
                // The progress bar.
                c.rect(9, 16, 19, 4, GLOW_DIM);
                c.rect(10, 17, 17, 2, SCREEN);
                let p = (f.st * 0.45).fract();
                let n = (p * 17.0) as i32;
                let col = if f.state == State::Testing { YELLOW } else { GLOW };
                if n > 0 {
                    c.rect(10, 17, n, 2, col);
                }
                if f.state == State::Testing {
                    for k in 0..n / 4 {
                        c.set(11 + k * 4, 20, YELLOW);
                    }
                }
            }
            State::Git => {
                // A branch: the main line, a branch off it, a commit running along.
                c.line(13, 10, 13, 20, GLOW);
                c.line(13, 17, 16, 14, GLOW);
                c.line(16, 14, 22, 14, GLOW);
                c.line(22, 14, 24, 11, GLOW);
                for (x, y) in [(13, 11), (13, 19), (24, 11)] {
                    c.rect(x - 1, y - 1, 3, 3, PINK);
                    c.set(x, y, WHITE);
                }
                let p = (t * 0.7).fract();
                let pts = [(13.0, 19.0), (13.0, 17.0), (16.0, 14.0), (22.0, 14.0), (24.0, 11.0)];
                let s = p * 4.0;
                let i = (s as usize).min(3);
                let v = s - i as f64;
                let (x, y) = (pts[i].0 + (pts[i + 1].0 - pts[i].0) * v, pts[i].1 + (pts[i + 1].1 - pts[i].1) * v);
                c.set(x.round() as i32, y.round() as i32, YELLOW);
            }
            State::Compacting => {
                // The picture squeezed into a line, like a CRT switching off, and back.
                let h = ((0.5 + 0.5 * f.wave(3.0, 0.0)) * 6.0) as i32;
                c.rect(SX, 15 - h, SW, h * 2 + 1, GLOW_DIM);
                c.line(SX, 15, SX + SW - 1, 15, WHITE);
                if h > 2 {
                    c.rect(11, 15 - h / 2, 3, h.min(4), GLOW);
                    c.rect(23, 15 - h / 2, 3, h.min(4), GLOW);
                }
            }
            _ => {
                // The face.
                let wink = f.within(self.wink, 0.35) && f.state == State::Idle;
                c.eye(11, 12, 3, 4, eyes, look, GLOW, WHITE, 0, false);
                c.eye(23, 12, 3, 4, if wink { Eyes::Happy } else { eyes }, look, GLOW, WHITE, 0, true);
                let my = 18;
                if f.state == State::Speaking {
                    // An oscilloscope mouth.
                    let amp = (f.voice * 16.0).clamp(0.6, 2.4);
                    for x in 11..=26 {
                        let y = my as f64 + (x as f64 * 1.3 - t * 18.0).sin() * amp * (1.0 - ((x as f64 - 18.5) / 9.0).abs());
                        c.set(x, y.round() as i32, GLOW);
                    }
                } else if f.is(&[State::Listening]) || (f.groove > 0.1 && f.state == State::Idle) {
                    // An equalizer.
                    for k in 0..6 {
                        let x = 12 + k * 3;
                        let lvl = if f.state == State::Listening { f.voice * 18.0 } else { f.groove * 2.0 };
                        let h = (1.0 + (lvl.min(2.5) * (0.5 + 0.5 * (t * 13.0 + k as f64 * 1.9).sin()))).round() as i32;
                        c.rect(x, 21 - h, 2, h, if h >= 3 { PINK } else { GLOW });
                    }
                } else if f.state == State::Transcribing {
                    let n = ((t * 3.0) as i32) % 4;
                    for k in 0..n {
                        c.set(15 + k * 3, 19, GLOW);
                    }
                } else if f.state == State::NoVoice {
                    glyph(c, 17, 16, &MIC, GLOW_DIM, GLOW_DIM);
                    c.line(15, 21, 21, 15, RED);
                } else if f.reading() {
                    c.line(17, my, 19, my, GLOW);
                } else if f.state == State::Delegating {
                    let off = ((t * 6.0) as i32) % 3;
                    for k in 0..3 {
                        let x = 12 + k * 5 + off;
                        c.set(x, my, GLOW);
                        c.set(x + 1, my + 1, GLOW);
                        c.set(x, my + 2, GLOW);
                    }
                } else if f.hurt() {
                    for (k, x) in (15..=21).enumerate() {
                        c.set(x, my + (k as i32 % 2), GLOW);
                    }
                } else {
                    c.smile(18, my, 7, GLOW);
                }
                // Reading: the red scanner sweeping across, side to side (a corner icon for
                // searching and the web).
                if f.reading() {
                    if f.state == State::Searching {
                        glyph(c, 25, 9, &LENS, YELLOW, YELLOW);
                    } else if f.state == State::Web {
                        glyph(c, 25, 9, &GLOBE, GLOW_DIM, GLOW_DIM);
                    }
                    let ph = (t * 1.6).sin();
                    let x = SX + ((0.5 + 0.5 * ph) * (SW - 1) as f64).round() as i32;
                    let back = if (t * 1.6).cos() > 0.0 { -1 } else { 1 };
                    c.line(x + back * 2, SY, x + back * 2, SY + SH - 1, RED_DIM);
                    c.line(x + back, SY, x + back, SY + SH - 1, RED_DIM);
                    c.line(x, SY, x, SY + SH - 1, RED);
                }
            }
        }
        // Little context left: a blinking low battery in the corner.
        if f.context > 0.85 && f.wave(5.0, 0.0) > -0.3 {
            glyph(c, 8, 9, &BATTERY, RED, RED);
        }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#8b7cc8", "#e6dcc4", "#b3a68a", "#fffaf0", "#123844", "#5cf6ff", "#2b8fa3", "#ff2d55", "#9c1a3c", "#ff52d9", "#ffe14d", "#2e5cff",
        "#ffffff", "#9a98b0", "#1b1530", "#a9bde0",
    ];
    const W: usize = 37;
    const H: usize = 42;

    fn new(seed: u64) -> Self {
        Char { wink: -99.0, next_wink: 4.0 + hash(seed) * 4.0, spark: -99.0, spark_left: false, next_spark: 2.0 + hash(seed ^ 5) * 3.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_wink {
            self.wink = f.t;
            self.next_wink = f.t + 5.0 + f.rand(f.t as u64 + 13) * 5.0;
        }
        if f.t > self.next_spark {
            self.spark = f.t;
            self.spark_left = f.rand(f.t as u64 + 3) < 0.5;
            self.next_spark = f.t + 2.5 + f.rand(f.t as u64 + 29) * 3.5;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let running = f.is(&[State::Running, State::Testing]);

        // Antennas, behind: rods and their neon balls.
        let (la, ra) = self.antennas(f);
        c.thick((16.0, 6.5), la, 1.0, METAL);
        c.thick((21.0, 6.5), ra, 1.0, METAL);
        c.ellipse(18.5, 6.2, 3.2, 1.7, METAL);
        let blink = f.thinking() && f.wave(6.0, 0.0) > 0.0;
        let flash = f.state == State::Speaking && f.voice > 0.06 && f.wave(18.0, 0.0) > 0.0;
        c.ellipse(la.0, la.1, 1.6, 1.6, if blink || f.asleep() { RED_DIM } else if flash { WHITE } else { PINK });
        c.ellipse(ra.0, ra.1, 1.6, 1.6, if f.thinking() && !blink || f.asleep() { GLOW_DIM } else if flash { WHITE } else { GLOW });

        // Arms (behind the torso), with claw hands.
        let (l, r) = self.arms(f);
        for (sh, (el, ha)) in [((9.5, 30.5), l), ((27.5, 30.5), r)] {
            c.thick(sh, el, 2.4, METAL);
            c.thick(el, ha, 2.2, METAL);
            c.ellipse(ha.0, ha.1, 1.8, 1.8, CASE);
        }

        // Legs (marching when running) and feet.
        let step = if running { f.wave(12.0, 0.0) } else { 0.0 };
        let (ll, rl) = ((step.max(0.0) * 2.0).round() as i32, ((-step).max(0.0) * 2.0).round() as i32);
        c.rect(12, 36 - ll, 4, 3, METAL);
        c.rect(21, 36 - rl, 4, 3, METAL);
        c.rect(10, 39 - ll, 7, 3, CASE);
        c.rect(20, 39 - rl, 7, 3, CASE);

        // Torso and neck.
        c.rect(9, 28, 19, 8, CASE);
        c.rect(15, 27, 7, 1, METAL);

        // The monitor, its corners rounded, a little hood on top.
        c.rect(4, 6, 29, 21, CASE);
        for (x, y) in [(4, 6), (32, 6), (4, 26), (32, 26)] {
            c.clear(x, y);
        }
        c.rect(8, 5, 21, 1, CASE);

        c.outline(LINE, &[]);
        c.shade(CASE, CASE_SH, LINE);
        c.light(CASE, CASE_LI, LINE);
        c.shade(METAL, LINE, LINE);

        // The bezel and the screen (rounded corners).
        c.rect(SX - 1, SY - 1, SW + 2, SH + 2, CASE_SH);
        self.screen(f, c);
        for (x, y) in [(SX, SY), (SX + SW - 1, SY), (SX, SY + SH - 1), (SX + SW - 1, SY + SH - 1)] {
            c.set(x, y, CASE_SH);
        }
        // A glint on the glass.
        if !f.within(f.times.error, 2.2) && f.state != State::Offline {
            c.set(SX + 1, SY + 1, CASE_LI);
            c.set(SX + 2, SY + 1, GLOW_DIM);
            c.set(SX + 1, SY + 2, GLOW_DIM);
        }

        // The KITT light on the chin: a red glow sweeping side to side, trailing.
        c.rect(8, 23, 21, 2, DARK);
        let speed = if f.reading() {
            5.0
        } else if running {
            4.0
        } else if f.asleep() {
            0.0
        } else {
            1.4
        };
        if f.asleep() {
            if f.state == State::Sleeping && f.wave(1.2, 0.0) > 0.0 {
                c.rect(17, 23, 3, 2, RED_DIM);
            }
        } else {
            let happy = f.within(f.times.done, 2.6);
            let p = 8.0 + (0.5 + 0.5 * (t * speed).sin()) * 20.0;
            let dir = (t * speed).cos();
            for x in 8..=28 {
                let d = x as f64 + 0.5 - p;
                let behind = if dir > 0.0 { -d } else { d };
                let col = if d.abs() < 1.2 {
                    RED
                } else if behind > 0.0 && behind < 5.0 {
                    RED_DIM
                } else {
                    0
                };
                let col = if happy && col > 0 { [PINK, GLOW, YELLOW][((x as f64 + t * 20.0) as usize / 2) % 3] } else { col };
                c.set(x, 23, col);
                c.set(x, 24, col);
            }
            if f.within(f.times.key, 0.15) {
                c.rect(8, 23, 21, 2, RED);
            }
        }
        // The power light and a knob.
        let power = if f.state == State::Offline {
            0
        } else if f.asleep() {
            if f.wave(1.2, 0.0) > 0.0 { YELLOW } else { CASE_SH }
        } else {
            GLOW
        };
        c.set(30, 24, power);
        c.rect(5, 23, 2, 2, CASE_SH);
        c.set(5, 23, CASE_LI);

        // The chest: the ticket slot, three blinking lights and a speaker grille.
        c.line(13, 30, 23, 30, DARK);
        c.line(13, 31, 23, 31, CASE_SH);
        let lit = ((t * 3.0) as i32).rem_euclid(3);
        for (k, col) in [PINK, YELLOW, GLOW].iter().enumerate() {
            let on = !f.asleep() && (k as i32 == lit || f.within(f.times.done, 2.6));
            c.set(12 + k as i32 * 2, 33, if on { *col } else { CASE_SH });
        }
        for k in 0..3 {
            c.line(20 + k * 2, 33, 20 + k * 2, 34, CASE_SH);
        }

        // Offline: grey all over.
        if f.state == State::Offline {
            c.swap(CASE, SMOKE);
            c.swap(CASE_LI, CASE_SH);
            c.swap(PINK, SMOKE);
            c.swap(GLOW, WHITE);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // The robot dance: stiff steps between poses.
        if let Some(p) = self.dance(f) {
            m.lean += [0.07, 0.0, -0.07, 0.0][p];
            m.dy += [0.0, 0.05, 0.0, 0.1][p];
        }
        if f.state == State::Sleeping {
            m.lean -= 0.05;
            m.sy *= 0.98;
        }
        if f.state == State::Asking {
            m.lean -= 0.03;
        }
        // An error: a jolt, then a slump.
        if let Some(u) = f.progress(f.times.error, 1.6) {
            if u < 0.3 {
                m.dy += 0.1 * (u * PI * 8.0).sin().abs();
            } else {
                m.sy *= 0.97;
                m.lean += 0.04;
            }
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let (la, ra) = self.antennas(f);
        // A spark: a short zigzag out of a point, flickering.
        let zap = |w: &mut World, x: f64, y: f64, dx: f64, n: u64| {
            let mut p = (x, y);
            for k in 0..4 {
                let jig = if (k + n) % 2 == 0 { 1.0 } else { -1.0 };
                p = (p.0 + dx, p.1 - 0.6 + jig * 0.8);
                w.px(p.0, p.1, if k == 0 { WHITE } else { YELLOW });
            }
        };
        // Now and then (and with a key) an antenna crackles.
        let busy = !f.asleep() && f.state != State::Offline;
        if busy && (f.within(self.spark, 0.25) || f.within(f.times.key, 0.15)) {
            let left = if f.within(f.times.key, 0.15) { (f.times.key * 10.0) as u64 % 2 == 0 } else { self.spark_left };
            let (p, dx) = if left { (la, -1.0) } else { (ra, 1.0) };
            zap(w, p.0, p.1 - 1.0, dx, (t * 12.0) as u64);
        }
        // Speaking: little radio waves out of the antennas.
        if f.state == State::Speaking && f.voice > 0.05 {
            let r = ((t * 2.5).fract()) * 4.0 + 2.0;
            for (p, s) in [(la, -1.0), (ra, 1.0)] {
                for k in -1..=1 {
                    w.px_dim(p.0 + s * r, p.1 + k as f64 * r * 0.4, GLOW);
                }
            }
        }

        // An error: sparks off both antennas, then smoke out of the top of the monitor.
        if let Some(u) = f.progress(f.times.error, 2.6) {
            if u < 0.35 {
                zap(w, la.0, la.1, -1.0, (t * 15.0) as u64);
                zap(w, ra.0, ra.1, 1.0, (t * 15.0) as u64 + 1);
            }
            for k in 0..6 {
                let ph = (u * 1.8 + k as f64 / 6.0).fract();
                if u + ph * 0.1 < 0.12 {
                    continue;
                }
                let x = 12.0 + (k * 5 % 13) as f64 + (ph * 6.0 + k as f64).sin() * 2.0;
                let y = 4.0 - ph * 14.0;
                let col = if ph < 0.5 { SMOKE } else { CASE_SH };
                w.px_dim(x, y, col);
                if ph < 0.6 {
                    w.px_dim(x + 1.0, y, col);
                    w.px_dim(x, y - 1.0, col);
                }
            }
        }

        // Done: neon lasers out of the antennas, then a burst of neon sparks.
        if let Some(u) = f.progress(f.times.done, 2.6) {
            if u < 0.5 {
                let a = u * 6.0;
                for (k, (p, s)) in [(la, -1.0), (ra, 1.0)].iter().enumerate() {
                    let ang = -PI / 2.0 + s * (0.3 + 0.35 * (a + k as f64).sin());
                    for d in 2..14 {
                        let (x, y) = (p.0 + ang.cos() * d as f64, p.1 + ang.sin() * d as f64);
                        w.px(x, y, if k == 0 { PINK } else { GLOW });
                    }
                }
            }
            let r = u * 30.0;
            for k in 0..12 {
                let a = k as f64 * PI / 6.0 + u;
                let (x, y) = (18.5 + a.cos() * r, 18.0 + a.sin() * r * 0.8);
                let col = [PINK, GLOW, YELLOW][k % 3];
                if u < 0.8 || k % 2 == 0 {
                    w.px(x, y, col);
                    w.px_dim(x + 1.0, y, col);
                    w.px_dim(x - 1.0, y, col);
                    w.px_dim(x, y + 1.0, col);
                    w.px_dim(x, y - 1.0, col);
                }
            }
        }

        // Copying: a ticket prints out of the chest slot, tears off and flutters away.
        if let Some(u) = f.progress(f.times.copy, 2.8) {
            let len = ((u / 0.45).min(1.0) * 10.0) as i32;
            let fly = ((u - 0.55) / 0.45).max(0.0);
            let (ox, oy) = (fly * 14.0 + (fly * 12.0).sin() * 1.5, fly * 4.0 - (fly * PI).sin() * 6.0);
            for y in 0..len {
                for x in 0..9 {
                    // The torn bottom: a zigzag.
                    if y == len - 1 && fly > 0.0 && (x % 2 == 0) {
                        continue;
                    }
                    let col = if (y == 2 || y == 4 || y == 6) && x > 0 && x < 8 && (x + y) % 5 != 0 {
                        SMOKE
                    } else if y == 8 && (2..7).contains(&x) {
                        PINK
                    } else {
                        WHITE
                    };
                    w.px(14.0 + x as f64 + ox, 31.0 + y as f64 + oy, col);
                }
            }
        }

        // Thinking: the hourglass makes it tick: dim dots orbiting the antenna balls.
        if f.thinking() {
            let p = if f.wave(6.0, 0.0) > 0.0 { la } else { ra };
            let a = t * 6.0;
            w.px_dim(p.0 + a.cos() * 3.0, p.1 + a.sin() * 2.0, YELLOW);
        }

        // Running: steam puffs from the feet.
        if f.is(&[State::Running, State::Testing]) {
            for k in 0..3 {
                let ph = (t * 2.0 + k as f64 / 3.0).fract();
                let side = if k % 2 == 0 { -1.0 } else { 1.0 };
                w.px_dim(18.5 + side * (10.0 + ph * 6.0), 41.0 - ph * 3.0, SMOKE);
            }
        }

        // Offline: its plug, lying unplugged on the floor, a cable to its back.
        if f.state == State::Offline {
            let mut prev = (28.0, 34.0);
            for k in 1..=8 {
                let v = k as f64 / 8.0;
                let p = (28.0 + v * 9.0, 34.0 + (v * PI).sin() * -1.0 + v * 7.0);
                for s in 0..3 {
                    let q = s as f64 / 3.0;
                    w.px(prev.0 + (p.0 - prev.0) * q, prev.1 + (p.1 - prev.1) * q, LINE);
                }
                prev = p;
            }
            for (x, y, col) in [(37.0, 40.0, SMOKE), (38.0, 40.0, SMOKE), (39.0, 40.0, SMOKE), (37.0, 41.0, SMOKE), (38.0, 41.0, SMOKE), (39.0, 41.0, SMOKE), (40.0, 40.0, METAL), (40.0, 41.0, METAL)] {
                w.px(x, y, col);
            }
            if f.wave(1.0, 0.0) > 0.7 {
                w.px_dim(41.5, 39.0, YELLOW);
            }
        }

        // Booting: scan lines settling as the picture opens.
        if f.state == State::Booting && f.st > 0.35 && f.st < 1.3 {
            let u = (f.st - 0.35) / 0.95;
            let e = ease_out(u);
            for side in [-1.0, 1.0] {
                w.px_dim(18.5 + side * (14.0 + e * 4.0), 15.0, GLOW);
            }
        }

        // Asleep: the screen's faint glow breathing on the floor.
        if f.state == State::Sleeping && f.wave(0.9, 0.0) > 0.0 {
            for x in [12.0, 16.0, 20.0, 24.0] {
                w.speck(x, 42.5, GLOW_DIM);
            }
        }
    }
}
