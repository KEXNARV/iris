//! The Creeper: a column of pixelated greens (head, body, four stubby legs) rebuilt from blocks
//! every frame, its texture chunky like a Minecraft skin. What it does:
//! - waiting: it looks around (its whole face turns), and now and then it starts to swell and
//!   hiss... and thinks better of it; music: a jukebox appears and it stomps to the beat;
//! - listening: eyes wide; asking: it leans, tapping a front foot;
//! - thinking: it charges up (the blue crackle of a charged creeper around it);
//! - speaking: the mouth opens with the voice and "sss" floats out;
//! - reading, searching, on the web: experience orbs spiral into it;
//! - editing, running, testing: it walks in place on its four legs, kicking up dust;
//! - git: cobblestone blocks stack up beside it;
//! - done: a firework rocket whose burst is a creeper face;
//! - error: it swells and flashes white, hissing, then EXPLODES (smoke, fire, flying blocks)
//!   and respawns from the ground up;
//! - a key: a green speck; copying: it drops a music disc or a grass block that bobs and spins;
//! - asleep: crouched, eyes shut, puffing little smoke rings; offline: gray, under a cobweb;
//! - booting: built from the feet up inside a mob spawner; little context left: head droops.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Creeper";
pub const SCENE: &str = "cave";
pub const DESCRIPTION: &str = "El Creeper de Minecraft en pixel art: se hincha y explota con los errores (y reaparece), lanza un cohete con su cara al terminar, absorbe orbes de experiencia al leer y suelta un disco al copiar";

const LINE: u8 = 1;
const G: u8 = 2;
const GD: u8 = 3;
const GL: u8 = 4;
const GH: u8 = 5;
const FACE: u8 = 6;
const WHITE: u8 = 7;
const SMOKE: u8 = 8;
const SMOKE2: u8 = 9;
const DIRT: u8 = 10;
const RED: u8 = 11;
const FIRE: u8 = 12;
const XP: u8 = 13;
const DISC: u8 = 14;
const PALE: u8 = 15;
const BLUE: u8 = 16;

/// The error, in seconds after it: swelling until `SWELL`, gone (blown up) until `GONE`, back
/// (rebuilt from the feet up) by `BACK`.
const SWELL: f64 = 1.0;
const GONE: f64 = 2.3;
const BACK: f64 = 2.9;

/// The "s" it hisses, 3×5.
const S: [(f64, f64); 7] = [(1.0, 0.0), (2.0, 0.0), (0.0, 1.0), (1.0, 2.0), (2.0, 3.0), (0.0, 4.0), (1.0, 4.0)];

/// The creeper face, 8×8, for the firework.
const FACE8: [&str; 8] = ["........", "........", ".XX..XX.", ".XX..XX.", "...XX...", "..XXXX..", "..XXXX..", "..X..X.."];

pub struct Char {
    /// Where its face turns while waiting (-1, 0, 1) and when it turns next.
    look: i32,
    next_look: f64,
    /// A fake-out (it starts to swell, then doesn't), and the next one.
    fake: f64,
    next_fake: f64,
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u.clamp(0.0, 1.0)).powi(3)
}

/// A block of chunky Minecraft greens (2×2 cells), the same wherever the block goes.
#[allow(clippy::too_many_arguments)]
fn tex(c: &mut Canvas, x0: i32, y0: i32, w: i32, h: i32, salt: u64, dark: bool) {
    for y in 0..h.max(0) {
        for x in 0..w.max(0) {
            let r = hash(salt.wrapping_mul(0x2545_F491) ^ ((x / 2) as u64).wrapping_mul(73_856_093) ^ ((y / 2) as u64).wrapping_mul(19_349_663));
            let col = if dark {
                if r < 0.55 { GD } else { G }
            } else if r < 0.16 {
                GL
            } else if r < 0.22 {
                GH
            } else if r < 0.42 {
                GD
            } else {
                G
            };
            c.set(x0 + x, y0 + y, col);
        }
    }
}

/// The "s" glyph in the world.
fn hiss(w: &mut World, x: f64, y: f64, c: u8, dim: bool) {
    for (dx, dy) in S {
        if dim { w.px_dim(x + dx, y + dy, c) } else { w.px(x + dx, y + dy, c) }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#3c9c46", "#5fd35a", "#41a94a", "#8ce67e", "#c4f7b0", "#132a18", "#ffffff", "#8e8ea0", "#d4d4e0", "#b07a40", "#e8402a", "#ffb03a",
        "#d8ff4a", "#7a7a8c", "#aab6a6", "#7fd8ff",
    ];
    const W: usize = 30;
    const H: usize = 40;

    fn new(seed: u64) -> Self {
        Char { look: 0, next_look: 1.5 + hash(seed) * 2.0, fake: -99.0, next_fake: 9.0 + hash(seed ^ 5) * 8.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_look {
            let r = f.rand(f.t as u64 ^ 0x51);
            self.look = if r < 0.33 { -1 } else if r < 0.66 { 0 } else { 1 };
            self.next_look = f.t + 1.2 + f.rand(f.t as u64 ^ 0x77) * 2.5;
        }
        if f.t > self.next_fake {
            if f.state == State::Idle && f.groove < 0.1 && !f.within(f.times.error, BACK) {
                self.fake = f.t;
            }
            self.next_fake = f.t + 10.0 + f.rand(f.t as u64 ^ 0x99) * 10.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let es = f.since(f.times.error);
        // Blown up: nothing left (the smoke is in `around`).
        if (SWELL..GONE).contains(&es) {
            return;
        }
        let asleep = f.asleep();
        let tired = f.context > 0.85 && !asleep;
        let walking = f.is(&[State::Running, State::Testing, State::Editing]);
        let dance = f.groove > 0.1 && !asleep;

        // The feet: how high each one is lifted (front left, front right, back left, back right).
        let (mut fl, mut fr, mut bl, mut br) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        if walking {
            let speed = match f.state {
                State::Running => 13.0,
                State::Testing => 10.0,
                _ => 6.0,
            };
            let s = (t * speed).sin();
            fl = s.max(0.0) * 3.0;
            fr = (-s).max(0.0) * 3.0;
            bl = fr;
            br = fl;
        } else if dance {
            let s = (t * PI * 2.0 * 1.1).sin();
            fl = s.max(0.0) * 3.5 * f.groove;
            fr = (-s).max(0.0) * 3.5 * f.groove;
            bl = fr;
            br = fl;
        } else if f.state == State::Typing {
            let s = (t * 18.0).sin();
            fl = if s > 0.3 { 1.5 } else { 0.0 };
            fr = if s < -0.3 { 1.5 } else { 0.0 };
        } else if f.state == State::Asking {
            fr = (t * 9.0).sin().max(0.0) * 2.5;
        }
        if f.within(f.times.key, 0.12) {
            fl = fl.max(1.0);
        }
        let (fl, fr, bl, br) = (fl.round() as i32, fr.round() as i32, bl.round() as i32, br.round() as i32);

        // Crouched asleep (legs tucked), a little lower when tired.
        let crouch = if asleep { 4 } else if tired { 1 } else { 0 };
        let leg_h = 9 - crouch;
        let body_top = 15 + crouch;
        let body_bottom = 40 - leg_h;

        // The head: sways dancing, bobs with the voice, droops tired, shakes saying no.
        let mut hdx = 0;
        let mut hdy = 0;
        if dance {
            hdx = if (t * PI * 1.1).sin() > 0.0 { 1 } else { -1 };
            hdy = if f.beat > 0.5 { 1 } else { 0 };
        }
        if f.state == State::Speaking && f.voice > 0.08 && f.wave(13.0, 0.0) > 0.2 {
            hdy = -1;
        }
        if tired {
            hdy = 1;
        }
        if f.within(f.times.nope, 0.45) {
            hdx = if f.wave(30.0, 0.0) > 0.0 { 1 } else { -1 };
        }
        let hx = 8 + hdx;
        let hy = 1 + crouch + hdy;

        // Back legs (a step behind: higher and darker), body, front legs, head.
        tex(c, 6, body_bottom - 2, 5, leg_h - bl, 11, true);
        tex(c, 19, body_bottom - 2, 5, leg_h - br, 12, true);
        tex(c, 9, body_top, 12, body_bottom - body_top, 2, false);
        tex(c, 9, body_bottom, 6, leg_h - fl, 3, false);
        tex(c, 15, body_bottom, 6, leg_h - fr, 4, false);
        tex(c, hx, hy, 14, 14, 1, false);
        // The shadow under the head and between the front legs.
        for col in [G, GL, GH] {
            c.recolor(9, hy + 14, 12, 1, col, GD);
            c.recolor(9, body_bottom, 12, 1, col, GD);
        }
        for y in body_bottom + 3..40 {
            c.clear(15, y);
        }

        c.outline(LINE, &[]);
        c.shade(G, GD, LINE);
        c.light(G, GL, LINE);

        // The face: it turns with where it looks (or where it decided to look).
        let (mut lx, ly) = f.look();
        if f.state == State::Idle && lx == 0 {
            lx = self.look;
        }
        if f.within(f.times.nope, 0.45) {
            lx = 0;
        }
        let ly = ly.min(0);
        let (fx, fy) = (hx + lx, hy + ly);
        let mut eyes = f.eyes();
        if (0.0..SWELL).contains(&es) {
            eyes = Eyes::Open;
        }
        if f.thinking() {
            eyes = Eyes::Open;
        }
        if tired && !f.blink {
            eyes = Eyes::Shut;
        }
        c.eye(fx + 2, fy + 4, 3, 3, eyes, (0, 0), FACE, 0, 0, false);
        c.eye(fx + 9, fy + 4, 3, 3, eyes, (0, 0), FACE, 0, 0, true);
        // The mouth: its sad, iconic shape; wider with the voice and taking things in.
        let open = if f.state == State::Speaking { f.mouth() } else if f.reading() { 0.4 } else { 0.0 };
        if asleep {
            c.rect(fx + 4, fy + 10, 6, 2, FACE);
        } else {
            let cw = if open > 0.55 { 8 } else if open > 0.25 { 6 } else { 4 };
            c.rect(fx + 7 - cw / 2, fy + 7, cw, 2, FACE);
            c.rect(fx + 3, fy + 9, 8, 3, FACE);
            c.rect(fx + 3, fy + 12, 2, 2, FACE);
            c.rect(fx + 9, fy + 12, 2, 2, FACE);
            if open > 0.55 {
                c.rect(fx + 5, fy + 12, 4, 1, FACE);
            }
        }

        // Offline: gray and lifeless.
        if f.state == State::Offline {
            for col in [G, GL, GH] {
                c.swap(col, PALE);
            }
            c.swap(GD, SMOKE);
            c.swap(LINE, SMOKE);
        }

        // About to blow: flashing white, faster and faster (and the idle fake-out, once).
        let flash = if es >= 0.0 && es < SWELL {
            ((es * 8.0 + es * es * 14.0) as i32) % 2 == 0
        } else {
            f.progress(self.fake, 0.8).is_some_and(|u| (0.2..0.45).contains(&u))
        };
        if flash {
            for col in [G, GL, GH] {
                c.swap(col, WHITE);
            }
            c.swap(GD, SMOKE2);
        }

        // Respawning (and booting): built from the feet up.
        let reveal = if (GONE..BACK).contains(&es) {
            Some((es - GONE) / (BACK - GONE))
        } else if f.state == State::Booting && f.st < 1.4 {
            Some(f.st / 1.4)
        } else {
            None
        };
        if let Some(u) = reveal {
            let cut = (40.0 * (1.0 - u)) as i32;
            for y in 0..cut {
                for x in 0..30 {
                    c.clear(x, y);
                }
            }
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        let es = f.since(f.times.error);
        // Swelling before the blast, pulsing.
        if (0.0..SWELL).contains(&es) {
            let k = es / SWELL;
            let s = 1.0 + 0.3 * k.powf(0.7) + 0.035 * (es * 32.0).sin();
            m.sx *= s;
            m.sy *= s;
        }
        if let Some(u) = f.progress(self.fake, 0.8) {
            let s = 1.0 + 0.1 * (u * PI).sin();
            m.sx *= s;
            m.sy *= s;
        }
        // It's built in place, not dropped from above.
        if f.state == State::Booting {
            m.dy = 0.0;
        }
        if f.state == State::Editing {
            m.dy += (f.t * 6.0).sin().abs() * 0.03;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let es = f.since(f.times.error);
        let asleep = f.asleep();

        // The error: hissing while it swells...
        if (0.0..SWELL).contains(&es) {
            for k in 0..3 {
                let p = (es * 1.6 + k as f64 / 3.0) % 1.0;
                let x = if k % 2 == 0 { 26.0 + p * 4.0 } else { -2.0 - p * 4.0 };
                hiss(w, x, 8.0 - p * 10.0, if k == 1 { GL } else { SMOKE2 }, p > 0.6);
            }
        }
        // ...the blast: a ring of fire and smoke, blocks flying, then smoke drifting up.
        if (SWELL..GONE + 0.3).contains(&es) {
            let u = es - SWELL;
            let r = 4.0 + ease_out(u / 0.5) * 18.0;
            for k in 0..14 {
                let a = k as f64 * PI * 2.0 / 14.0 + f.rand(k) * 0.4;
                let rr = r * (0.75 + 0.35 * f.rand(k + 40));
                let (x, y) = (15.0 + a.cos() * rr, 22.0 + a.sin() * rr * 0.8 - u * 6.0);
                let col = if u < 0.18 { if k % 2 == 0 { WHITE } else { FIRE } } else if u < 0.35 && k % 3 == 0 { FIRE } else if k % 2 == 0 { SMOKE2 } else { SMOKE };
                let fade = u > 0.8;
                for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                    if fade { w.px_dim(x + dx, y + dy, col) } else { w.px(x + dx, y + dy, col) }
                }
            }
            // The flash in the middle.
            if u < 0.15 {
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0;
                    for s in 1..4 {
                        w.px(15.0 + a.cos() * s as f64 * 2.0, 22.0 + a.sin() * s as f64 * 2.0, if s == 1 { WHITE } else { FIRE });
                    }
                }
            }
            // Blocks: little cubes of grass and dirt thrown out, falling back.
            for k in 0..7 {
                let vx = (f.rand(k + 100) - 0.5) * 50.0;
                let vy = -25.0 - f.rand(k + 200) * 25.0;
                let (x, y) = (15.0 + vx * u, 26.0 + vy * u + 60.0 * u * u);
                if y < 40.0 {
                    let top = if k % 3 == 0 { G } else { DIRT };
                    w.px(x, y, top);
                    w.px(x + 1.0, y, top);
                    w.px(x, y + 1.0, DIRT);
                    w.px(x + 1.0, y + 1.0, DIRT);
                }
            }
        }
        // Respawn: a puff where it comes back.
        if let Some(u) = f.progress(f.times.error + GONE, BACK - GONE + 0.3) {
            for k in 0..6 {
                let x = 6.0 + k as f64 * 3.6;
                w.px_dim(x, 39.0 - u * 8.0 - (k % 2) as f64 * 2.0, SMOKE2);
            }
        }

        // The idle fake-out: one hiss, then a sweat drop of relief.
        if let Some(u) = f.progress(self.fake, 1.6) {
            if u < 0.5 {
                hiss(w, 25.0, 4.0 - u * 6.0, SMOKE2, false);
            } else {
                let y = 2.0 + (u - 0.5) * 8.0;
                w.px(24.0, y, BLUE);
                w.px(24.0, y + 1.0, BLUE);
                w.px(23.0, y + 1.0, BLUE);
            }
        }

        // Speaking: "sss" out of the mouth with the voice.
        if f.state == State::Speaking && f.voice > 0.04 {
            for k in 0..3 {
                let p = (t * 0.9 + k as f64 / 3.0) % 1.0;
                hiss(w, 25.0 + p * 10.0, 10.0 - p * 8.0 + (p * 9.0 + k as f64).sin(), if k == 0 { GL } else { SMOKE2 }, p > 0.65);
            }
        }

        // Reading, searching, on the web: experience orbs spiraling in.
        if f.reading() && !asleep {
            let n = if f.state == State::Searching { 8 } else { 6 };
            for k in 0..n {
                let p = (t * 0.6 + k as f64 / n as f64) % 1.0;
                let rad = 22.0 * (1.0 - p);
                let a = k as f64 * PI * 2.0 / n as f64 + p * 3.5;
                let (x, y) = (15.0 + a.cos() * rad, 18.0 + a.sin() * rad * 0.75);
                let col = if f.state == State::Web && k % 2 == 1 { BLUE } else { XP };
                if p < 0.92 {
                    w.px(x, y, col);
                    w.px(x + 1.0, y, col);
                    w.px(x, y + 1.0, col);
                    w.px(x + 1.0, y + 1.0, if (t * 8.0 + k as f64).sin() > 0.0 { GH } else { col });
                }
            }
        }

        // Thinking, planning: charged — blue crackle running around it.
        if f.thinking() {
            let (x0, y0, x1, y1) = (4.0, -2.0, 26.0, 40.0);
            let mut x = x0;
            while x <= x1 {
                for y in [y0, y1] {
                    if ((x + y + t * 14.0).rem_euclid(8.0)) < 2.5 {
                        w.px(x, y, BLUE);
                    }
                }
                x += 1.0;
            }
            let mut y = y0;
            while y <= y1 {
                for x in [x0, x1] {
                    if ((x + y + t * 14.0).rem_euclid(8.0)) < 2.5 {
                        w.px(x, y, BLUE);
                    }
                }
                y += 1.0;
            }
            if f.wave(17.0, 0.0) > 0.6 {
                w.px(x1 + 1.0, 10.0 + (t * 7.0).sin() * 8.0, WHITE);
            }
        }

        // Working: dust kicked up behind its feet.
        if f.is(&[State::Running, State::Testing]) {
            for k in 0..4 {
                let p = (t * 2.2 + k as f64 / 4.0) % 1.0;
                w.px_dim(4.0 - p * 9.0, 38.0 - p * 4.0, if k % 2 == 0 { SMOKE2 } else { DIRT });
            }
        }

        // Git: cobblestone blocks stacking up, a branch beside them.
        if f.state == State::Git {
            let n = 1 + ((f.st * 1.4) as usize % 4);
            let block = |w: &mut World, bx: f64, by: f64| {
                for y in 0..4 {
                    for x in 0..4 {
                        let col = if (x * 3 + y * 5) % 4 == 0 { SMOKE2 } else { SMOKE };
                        w.px(bx + x as f64, by + y as f64, col);
                    }
                }
            };
            for i in 0..n {
                block(w, 27.0, 36.0 - 4.0 * i as f64);
            }
            if n >= 3 {
                block(w, 31.0, 36.0 - 4.0 * (n as f64 - 2.0));
            }
        }

        // Compacting: smoke squeezed out of its sides.
        if f.state == State::Compacting {
            for k in 0..3 {
                let p = (t * 1.5 + k as f64 / 3.0) % 1.0;
                w.px_dim(2.0 - p * 6.0, 24.0 - p * 5.0, SMOKE2);
                w.px_dim(28.0 + p * 6.0, 24.0 - p * 5.0, SMOKE2);
            }
        }

        // Done: a firework rocket, and its burst is a creeper face.
        if let Some(s) = f.progress(f.times.done, 2.0).map(|u| u * 2.0) {
            let launch = 0.18;
            let (cx, cy) = (15.0, -14.0);
            if s < launch {
                let y = 36.0 + (cy - 36.0) * (s / launch);
                let x = 26.0 + (cx - 26.0) * (s / launch);
                w.px(x, y, RED);
                w.px(x, y + 1.0, RED);
                w.px(x, y - 1.0, WHITE);
                for k in 1..5 {
                    w.px_dim(x + 0.3 * k as f64, y + 1.0 + k as f64 * 2.0, FIRE);
                }
            } else {
                let e = ease_out((s - launch) / 0.5);
                let fall = ((s - launch - 0.5).max(0.0)) * 5.0;
                let fade = s > 1.5;
                let gap = 0.6 + e * 2.9;
                for (row, line) in FACE8.iter().enumerate() {
                    for (col, ch) in line.chars().enumerate() {
                        if ch != 'X' {
                            continue;
                        }
                        let x = cx + (col as f64 - 3.5) * gap;
                        let y = cy + (row as f64 - 4.0) * gap + fall;
                        let tw = (t * 20.0 + (row * 8 + col) as f64).sin() > -0.4;
                        if tw {
                            let c = if (row + col) % 3 == 0 { GH } else { G };
                            if fade { w.px_dim(x, y, c) } else { w.px(x, y, c) }
                        }
                    }
                }
                // A ring of golden sparks around the face.
                for k in 0..14 {
                    let a = k as f64 * PI / 7.0;
                    let r = e * 15.0;
                    let (x, y) = (cx + a.cos() * r, cy + a.sin() * r + fall);
                    if (t * 16.0 + k as f64).sin() > -0.2 {
                        if fade { w.px_dim(x, y, FIRE) } else { w.px(x, y, if k % 2 == 0 { FIRE } else { WHITE }) }
                    }
                }
            }
        }

        // A key: a green speck popping off it.
        if let Some(u) = f.progress(f.times.key, 0.3) {
            let x = 6.0 + f.rand(f.times.key as u64 ^ 3) * 18.0;
            w.px(x, -1.0 - u * 4.0, GL);
        }

        // Merge: a burst of experience orbs.
        if let Some(u) = f.progress(f.times.merge, 0.8) {
            for k in 0..8 {
                let a = k as f64 * PI / 4.0;
                w.px(15.0 + a.cos() * u * 18.0, 20.0 + a.sin() * u * 14.0, XP);
            }
        }

        // Copying: a music disc or a grass block drops out and bobs, spinning, like an item.
        if let Some(u) = f.progress(f.times.copy, 3.2) {
            let s = u * 3.2;
            let disc = (f.times.copy * 7.0) as u64 % 2 == 0;
            let (x, y) = if s < 0.35 {
                let k = s / 0.35;
                (15.0 + k * 13.0, 22.0 - (k * PI).sin() * 10.0 + k * 10.0)
            } else {
                (28.0, 32.0 + (t * 3.0).sin() * 1.2)
            };
            let spin = (t * 3.5).cos().abs().max(0.2);
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let (fx, fy) = (dx as f64 / spin, dy as f64);
                    if disc {
                        let d = fx * fx + fy * fy;
                        if d <= 9.5 {
                            let col = if d <= 1.2 { FIRE } else if d <= 2.5 { RED } else if (dx + dy) % 3 == 0 { SMOKE } else { DISC };
                            w.px(x + dx as f64, y + dy as f64, col);
                        }
                    } else if fx.abs() <= 3.0 && dy.abs() <= 3 {
                        let col = if dy <= -2 { if dx % 2 == 0 { GL } else { G } } else if (dx * 5 + dy * 3).rem_euclid(4) == 0 { GD } else { DIRT };
                        w.px(x + dx as f64, y + dy as f64, col);
                    }
                }
            }
        }

        // Music: a jukebox beside it.
        if f.groove > 0.1 && !asleep {
            for y in 0..6 {
                for x in 0..6 {
                    let col = if y == 0 && (2..4).contains(&x) { FACE } else if y == 0 || x == 0 || x == 5 || y == 5 { DIRT } else if (x + y) % 2 == 0 { DIRT } else { SMOKE };
                    w.px(-9.0 + x as f64, 34.0 + y as f64, col);
                }
            }
            if f.beat > 0.5 {
                w.px(-7.0, 32.0, XP);
                w.px(-6.0, 31.0, XP);
            }
        }

        // Asleep: little smoke rings puffed out, rising.
        if f.state == State::Sleeping {
            let p = (t / 2.6) % 1.0;
            if p < 0.7 {
                let q = p / 0.7;
                let (x, y) = (22.0 + q * 6.0, 14.0 - q * 10.0);
                let r = 1.0 + q * 1.5;
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0;
                    w.px_dim(x + a.cos() * r, y + a.sin() * r * 0.7, SMOKE2);
                }
            }
        }

        // Offline: a cobweb over its head.
        if f.state == State::Offline {
            let (ox, oy) = (3.0, 2.0);
            for (ex, ey) in [(12.0, 0.0), (0.0, 12.0), (9.0, 9.0), (12.0, 5.0)] {
                let n = 12;
                for k in 0..=n {
                    let v = k as f64 / n as f64;
                    w.px_dim(ox + ex * v, oy + ey * v, SMOKE2);
                }
            }
            for r in [4.0, 7.5] {
                for k in 0..7 {
                    let a = k as f64 * PI / 12.0;
                    w.px_dim(ox + a.cos() * r, oy + a.sin() * r, SMOKE2);
                }
            }
        }

        // Booting: a mob spawner's cage around it, flames inside.
        if f.state == State::Booting && f.st < 1.8 {
            let dim = f.st > 1.3;
            for y in 0..42 {
                for x in [3.0, 27.0] {
                    if dim { w.px_dim(x, y as f64 - 1.0, SMOKE) } else { w.px(x, y as f64 - 1.0, SMOKE) }
                }
            }
            for y in [-1.0, 40.0] {
                for x in 3..=27 {
                    if dim { w.px_dim(x as f64, y, SMOKE) } else { w.px(x as f64, y, SMOKE) }
                }
            }
            for x in [9.0, 15.0, 21.0] {
                for y in 0..41 {
                    if y % 2 == 0 {
                        w.px_dim(x, y as f64, SMOKE);
                    }
                }
            }
            for k in 0..6 {
                let p = (t * 1.3 + k as f64 / 6.0) % 1.0;
                w.px(6.0 + k as f64 * 3.5, 38.0 - p * 20.0, if p < 0.5 { FIRE } else { RED });
            }
        }

        // Little context left: a sweat drop.
        if f.context > 0.85 && !asleep {
            let y = 4.0 + ((t * 0.8) % 1.0) * 5.0;
            w.px(25.0, y, BLUE);
            w.px(25.0, y + 1.0, BLUE);
            w.px(24.0, y + 1.0, BLUE);
        }
    }
}
