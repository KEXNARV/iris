//! Pikachu: drawn from shapes every frame (ears, head, body, arms, feet, a lightning tail), so
//! everything on it moves. What it does:
//! - waiting: the tail wags, an ear twitches now and then; music makes it dance;
//! - listening: ears straight up, eyes wide; asking: its head tilts, one ear down;
//! - thinking: eyes up, the tail still, a spark thinking with it;
//! - speaking: "pika": the mouth with the voice, sparks crackling off its cheeks;
//! - reading, searching, on the web: charging up, sparks circling it, cheeks flashing;
//! - running, testing: running in place, arms swinging, ears blown back;
//! - done: a Thunderbolt from the sky, happy eyes, arms up;
//! - error: shocked — its skeleton flashes through, ears down, x x;
//! - a key: a spark off a cheek; copying: it pulls out its ketchup bottle;
//! - asleep: ears down, curled tail, a snore that zaps now and then;
//! - offline: pale; little context left: a sweat drop.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Pikachu";
pub const SCENE: &str = "meadow";
pub const DESCRIPTION: &str = "Pikachu en pixel art: carga electricidad al leer, lanza un Impactrueno al terminar, se electrocuta con los errores y saca su ketchup al copiar";

// Its colors, by number (pixel value).
const LINE: u8 = 1;
const YELLOW: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const TIP: u8 = 5;
const CHEEK: u8 = 6;
const EYE: u8 = 7;
const WHITE: u8 = 8;
const MOUTH: u8 = 9;
const TONGUE: u8 = 10;
const BROWN: u8 = 11;
const SPARK: u8 = 12;
const BOLT: u8 = 13;
const PALE: u8 = 14;
const RED: u8 = 15;
const DROP: u8 = 16;

pub struct Char {
    /// When the last ear twitch started (one ear, for a moment), and which.
    twitch: f64,
    twitch_left: bool,
    /// Its next twitch, and the next snore zap.
    next_twitch: f64,
    next_zap: f64,
    zap: f64,
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#a8650f", "#ffd83a", "#e3a417", "#fff2a0", "#4a2c16", "#ff4a3a", "#1a0f0c", "#ffffff", "#9c2335", "#ff8a9a", "#9a6a2a", "#bff8ff",
        "#fff8b0", "#efe6b8", "#e8322a", "#8fd6ff",
    ];
    const W: usize = 34;
    const H: usize = 34;

    fn new(seed: u64) -> Self {
        Char { twitch: -99.0, twitch_left: false, next_twitch: 2.0 + hash(seed) * 3.0, next_zap: 3.0, zap: -99.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_twitch {
            self.twitch = f.t;
            self.twitch_left = f.rand(f.t as u64) < 0.5;
            self.next_twitch = f.t + 2.5 + f.rand(f.t as u64 + 7) * 4.0;
        }
        if f.state == State::Sleeping && f.t > self.next_zap {
            self.zap = f.t;
            self.next_zap = f.t + 2.5 + f.rand(f.t as u64 + 3) * 3.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let hurt = f.hurt();
        let happy = f.happy();
        let running = f.is(&[State::Running, State::Testing]);
        let shocked = f.progress(f.times.error, 0.55).is_some_and(|u| ((u * 9.0) as i32) % 2 == 0);

        // The ears: where their tips are (they droop asleep and hurt, stand up listening, fly
        // back running, one goes down when asking, one twitches now and then).
        let (mut lt, mut rt) = ((6.0, 0.5), (27.0, 0.5));
        if f.asleep() || hurt {
            lt = (1.0, 8.0);
            rt = (32.0, 8.0);
        } else if f.is(&[State::Listening, State::Transcribing]) {
            lt = (8.0, -0.5);
            rt = (25.0, -0.5);
        } else if running {
            lt = (2.0, 3.0);
            rt = (31.0, 3.0);
        } else if f.state == State::Asking {
            rt = (31.0, 6.0);
        }
        if let Some(u) = f.progress(self.twitch, 0.25) {
            let k = (u * PI).sin() * 2.0;
            if self.twitch_left { lt.0 -= k } else { rt.0 += k }
        }
        if f.groove > 0.1 {
            let b = f.wave(PI * 2.2, 0.0) * 1.5 * f.groove;
            lt.0 += b;
            rt.0 += b;
        }

        // The tail, behind: a lightning bolt from its back, wagging (fast with music, still
        // while thinking, curled asleep).
        let wag = if f.thinking() { 0.0 } else if f.groove > 0.1 { f.wave(9.0, 0.0) } else if running { f.wave(14.0, 0.0) * 0.6 } else { f.wave(2.2, 0.0) * 0.5 };
        let tail: [(f64, f64); 5] = if f.asleep() {
            [(22.0, 27.0), (27.0, 28.0), (25.0, 25.0), (30.0, 25.0), (29.0, 22.0)]
        } else {
            [(22.0, 26.0), (27.0 + wag, 22.0), (24.5 + wag, 19.0), (31.0 + wag * 1.5, 13.0), (28.5 + wag * 1.5, 9.0)]
        };
        c.thick(tail[0], tail[1], 2.6, BROWN);
        c.thick(tail[1], tail[2], 2.6, YELLOW);
        c.thick(tail[2], tail[3], 3.4, YELLOW);
        c.triangle(tail[3], tail[4], (tail[3].0 - 3.0, tail[3].1 - 3.5), YELLOW);

        // Ears (behind the head), black tips.
        let ear = |c: &mut Canvas, tip: (f64, f64), a: (f64, f64), b: (f64, f64)| {
            c.triangle(tip, a, b, YELLOW);
            let k = 0.38;
            c.triangle(tip, (tip.0 + (a.0 - tip.0) * k, tip.1 + (a.1 - tip.1) * k), (tip.0 + (b.0 - tip.0) * k, tip.1 + (b.1 - tip.1) * k), TIP);
        };
        ear(c, lt, (8.5, 11.0), (13.0, 8.0));
        ear(c, rt, (20.0, 8.0), (24.5, 11.0));

        // Feet, body, head.
        let step = if running { f.wave(14.0, 0.0) } else { 0.0 };
        c.ellipse(12.0, 31.5 - step.max(0.0) * 1.5, 3.0, 1.6, YELLOW);
        c.ellipse(22.0, 31.5 - (-step).max(0.0) * 1.5, 3.0, 1.6, YELLOW);
        c.ellipse(17.0, 25.0, 8.0, 7.0, YELLOW);
        c.ellipse(17.0, 15.0, 10.5, 8.0, YELLOW);

        // Arms: down, up cheering, waving when asking, swinging running, up in a shock.
        let (mut la, mut ra) = (22.5, 22.5);
        if happy || hurt {
            la = 17.5;
            ra = 17.5;
        } else if f.state == State::Asking {
            ra = 18.0 + f.wave(10.0, 0.0);
        } else if running {
            la += step * 1.5;
            ra -= step * 1.5;
        }
        c.ellipse(10.0, la, 1.6, 2.4, YELLOW);
        c.ellipse(24.0, ra, 1.6, 2.4, YELLOW);

        c.outline(LINE, &[]);
        c.shade(YELLOW, SHADE, LINE);
        c.light(YELLOW, LIGHT, LINE);

        // The face.
        let eyes = if f.thinking() { Eyes::Open } else { f.eyes() };
        let look = f.look();
        c.eye(12, 13, 2, 3, eyes, look, EYE, WHITE, YELLOW, false);
        c.eye(21, 13, 2, 3, eyes, look, EYE, WHITE, YELLOW, true);
        // Cheeks: red, flashing white while charging or speaking loud.
        let flash = (f.reading() && f.wave(14.0, 0.0) > 0.3) || (f.state == State::Speaking && f.voice > 0.12 && f.wave(20.0, 0.0) > 0.0) || happy;
        let cheek = if flash { BOLT } else { CHEEK };
        c.ellipse(9.0, 18.0, 2.2, 1.6, cheek);
        c.ellipse(25.0, 18.0, 2.2, 1.6, cheek);
        // The mouth: a little "ω" closed, open with the voice, a big smile when done.
        let open = f.mouth();
        if happy {
            c.mouth(17, 18, 0.9, 6, 4, MOUTH, TONGUE);
        } else if open > 0.08 {
            c.mouth(17, 18, open, 4, 4, MOUTH, TONGUE);
        } else {
            for (x, y) in [(15, 18), (16, 19), (17, 18), (18, 19), (19, 18)] {
                c.set(x, y, LINE);
            }
        }
        // The nose.
        c.set(17, 16, LINE);

        // Offline: pale. A shock: its skeleton through a dark body, flashing.
        if f.state == State::Offline {
            for col in [YELLOW, SHADE, LIGHT, BROWN] {
                c.swap(col, PALE);
            }
        }
        if shocked {
            for col in [YELLOW, SHADE, LIGHT, BROWN, CHEEK, TIP, BOLT] {
                c.swap(col, EYE);
            }
            c.swap(LINE, SPARK);
            // Skull, spine, ribs, arms and legs.
            c.ellipse(17.0, 15.0, 6.0, 5.0, WHITE);
            c.ellipse(14.0, 14.5, 1.3, 1.6, EYE);
            c.ellipse(20.0, 14.5, 1.3, 1.6, EYE);
            c.line(15, 18, 19, 18, EYE);
            c.line(17, 21, 17, 29, WHITE);
            for y in [23, 25, 27] {
                c.line(13, y, 21, y, WHITE);
            }
            c.line(13, 30, 11, 32, WHITE);
            c.line(21, 30, 23, 32, WHITE);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // Asking: its head tilts (the drawing leans).
        if f.state == State::Asking {
            m.lean += 0.06;
        }
        // A shock makes it jump in place, stiff.
        if let Some(u) = f.progress(f.times.error, 0.55) {
            m.dy += 0.08 * (u * PI * 6.0).sin().abs();
            m.sy *= 1.04;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        // Sparks: a short zigzag of 3–4 pixels from (x, y) going `dir`, flickering.
        let zap = |w: &mut World, x: f64, y: f64, dir: f64, n: u64| {
            let mut p = (x, y);
            for k in 0..4 {
                let jig = if (k + n) % 2 == 0 { 1.0 } else { -1.0 };
                p = (p.0 + dir, p.1 + jig);
                w.px(p.0, p.1, if k == 0 { BOLT } else { SPARK });
            }
        };
        // Speaking: crackling off the cheeks with the voice.
        if f.state == State::Speaking && f.voice > 0.04 {
            if f.wave(23.0, 0.0) > 0.0 {
                zap(w, 6.0, 18.0, -1.0, (t * 10.0) as u64);
            }
            if f.wave(19.0, 1.0) > 0.0 {
                zap(w, 28.0, 18.0, 1.0, (t * 10.0) as u64 + 1);
            }
        }
        // Charging: sparks circling it.
        if f.reading() {
            for k in 0..6 {
                let a = t * 2.4 + k as f64 * PI / 3.0;
                let (x, y) = (17.0 + a.cos() * 20.0, 18.0 + a.sin() * 14.0);
                if (t * 8.0 + k as f64).sin() > -0.3 {
                    w.px(x, y, if k % 2 == 0 { SPARK } else { BOLT });
                    w.px(x + 1.0, y - 1.0, SPARK);
                }
            }
        }
        // Thinking: a little spark over its head, coming and going.
        if f.thinking() && f.wave(3.0, 0.0) > 0.2 {
            zap(w, 25.0, -2.0, 1.0, (t * 4.0) as u64);
        }
        // A key: a spark off one cheek.
        if f.within(f.times.key, 0.18) {
            zap(w, if f.times.key as u64 % 2 == 0 { 6.0 } else { 28.0 }, 18.0, if f.times.key as u64 % 2 == 0 { -1.0 } else { 1.0 }, 3);
        }
        // Done: a Thunderbolt from the sky onto it, then sparks bursting out.
        if let Some(u) = f.progress(f.times.done, 1.0) {
            if u < 0.45 {
                let mut p = (17.0, -30.0);
                let mut k = 0;
                while p.1 < -1.0 {
                    let next = (17.0 + if k % 2 == 0 { 3.0 } else { -3.0 }, p.1 + 4.0);
                    for s in 0..8 {
                        let v = s as f64 / 8.0;
                        let (x, y) = (p.0 + (next.0 - p.0) * v, p.1 + (next.1 - p.1) * v);
                        w.px(x, y, BOLT);
                        w.px(x + 1.0, y, WHITE);
                        w.px(x - 1.0, y, SPARK);
                    }
                    p = next;
                    k += 1;
                }
            } else {
                let r = (u - 0.45) * 40.0;
                for k in 0..10 {
                    let a = k as f64 * PI / 5.0;
                    w.px(17.0 + a.cos() * r, 16.0 + a.sin() * r * 0.8, if k % 2 == 0 { BOLT } else { SPARK });
                }
            }
        }
        // Copying: its ketchup bottle, held up beside it.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let rise = (u * 6.0).min(1.0);
            let (bx, by) = (27.0, 22.0 - rise * 6.0);
            for y in 0..8 {
                for x in 0..4 {
                    let col = if y < 2 { WHITE } else if y == 2 && x > 0 && x < 3 { WHITE } else { RED };
                    if y == 0 && (x == 0 || x == 3) {
                        continue;
                    }
                    w.px(bx + x as f64, by + y as f64, col);
                }
            }
            // A heart over it.
            if u > 0.2 {
                for (x, y) in [(0.0, 0.0), (2.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (0.0, 2.0), (1.0, 2.0), (2.0, 2.0), (1.0, 3.0)] {
                    w.px(bx - 1.0 + x, by - 6.0 + y - u * 3.0, RED);
                }
            }
        }
        // Asleep: a snore that zaps.
        if f.state == State::Sleeping && f.within(self.zap, 0.3) {
            zap(w, 25.0, 18.0, 1.0, 1);
        }
        // Little context left: a sweat drop on its head.
        if f.context > 0.85 && !f.asleep() {
            let y = 4.0 + ((t * 0.8) % 1.0) * 4.0;
            w.px(28.0, y, DROP);
            w.px(28.0, y + 1.0, DROP);
            w.px(27.0, y + 1.0, DROP);
        }
    }
}
