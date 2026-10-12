//! Totoro (Mi vecino Totoro): drawn from shapes every frame (ears, head, round body, cream belly
//! with its chevrons, arms, clawed feet, whiskers and the leaf on its head). What it does:
//! - waiting: breathing, the leaf on its head wobbling, whiskers twitching, an ear flicking; now
//!   and then its huge Cheshire grin (the bus stop one);
//! - listening: ears up, eyes wide; asking: head tilted, one ear down, offering an acorn;
//! - thinking: it opens its umbrella and listens to the rain drumming on it;
//! - speaking: the wide mouth with the voice, whiskers bristling;
//! - reading, searching, on the web: it reads a big leaf held in both hands, leaves drifting in;
//! - running, testing: it flies on its spinning top, arms out, wind behind;
//! - editing: kneading with its claws; git: a sapling grows beside it and forks into branches;
//! - delegating: Chibi (white) and Chu (blue) Totoro hop around it;
//! - done: its ROAR, the mouth full of teeth, arms up, a gust blowing leaves away;
//! - error: a huge raindrop falls on it, it bristles all over with tiny pupils;
//! - a key: the leaf hops; copying: Chibi Totoro pops up with an acorn;
//! - music: the sprout dance, arms up, sprouts popping out of the ground;
//! - asleep: belly up, the belly rising and falling, feet in the air;
//! - offline: pale and sleepy; little context left: heavy eyelids and big yawns.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Totoro";
pub const SCENE: &str = "forest";
pub const DESCRIPTION: &str = "Totoro en pixel art: abre su paraguas para pensar, vuela en su trompo al trabajar, ruge con todos los dientes al terminar y duerme panza arriba";

const LINE: u8 = 1;
const FUR: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const BELLY: u8 = 5;
const WHITE: u8 = 6;
const PUPIL: u8 = 7;
const MOUTH: u8 = 8;
const LEAF: u8 = 9;
const LEAFD: u8 = 10;
const BLUE: u8 = 11;
const RAIN: u8 = 12;
const ACORN: u8 = 13;
const CAP: u8 = 14;
const GLOW: u8 = 15;
const PALE: u8 = 16;

pub struct Char {
    /// The idle grin, an ear flick, a yawn when tired.
    grin: f64,
    next_grin: f64,
    flick: f64,
    flick_left: bool,
    next_flick: f64,
    yawn: f64,
    next_yawn: f64,
}

/// An acorn, its top-left at (x, y), in the world.
fn acorn(w: &mut World, x: f64, y: f64) {
    for k in 0..3 {
        w.px(x + k as f64, y, CAP);
    }
    for yy in 1..3 {
        for k in 0..3 {
            w.px(x + k as f64, y + yy as f64, ACORN);
        }
    }
    w.px(x + 1.0, y + 3.0, ACORN);
    w.px(x + 1.0, y - 1.0, CAP);
}

/// A little leaf at (x, y) turning with `a`.
fn leaf(w: &mut World, x: f64, y: f64, a: f64) {
    let (dx, dy) = (a.cos(), a.sin());
    w.px(x, y, LEAF);
    w.px(x + dx, y + dy, LEAF);
    w.px(x - dx, y - dy, LEAFD);
}

/// A mini Totoro (Chibi white, Chu blue), 5 wide, its top-left at (x, y).
fn mini(w: &mut World, x: f64, y: f64, body: u8) {
    let rows = [".b.b.", "bbbbb", "bkbkb", "bbbbb", "bwwwb", ".b.b."];
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            let col = match ch {
                'b' => body,
                'k' => PUPIL,
                'w' => if body == WHITE { BELLY } else { WHITE },
                _ => 0,
            };
            w.px(x + i as f64, y + j as f64, col);
        }
    }
}

impl Char {
    /// Belly up, feet in the air, the belly breathing.
    fn draw_asleep(&self, f: &Frame, c: &mut Canvas) {
        let br = f.wave(0.9, 0.0);
        // Feet up in the air at the right.
        c.ellipse(32.5, 22.0, 2.8, 4.0, FUR);
        c.ellipse(37.5, 24.5, 2.5, 3.6, FUR);
        // The mound of its body, the belly dome on top, rising and falling.
        c.ellipse(23.0, 32.0, 16.5, 7.5 + br * 0.3, FUR);
        c.ellipse(23.5, 29.5 - br * 0.6, 13.0, 6.0 + br * 0.5, BELLY);
        // An arm resting on the belly.
        c.thick((14.0, 31.0), (17.0, 26.0 - br * 0.5), 3.4, FUR);
        // The head on the left, its ears pointing up and away.
        c.triangle((0.5, 17.0), (2.0, 27.0), (7.0, 25.0), FUR);
        c.triangle((8.0, 15.5), (7.5, 25.0), (12.0, 26.0), FUR);
        c.ellipse(7.5, 32.0, 7.5, 7.0, LINE);
        c.ellipse(7.5, 32.0, 6.6, 6.2, FUR);
        c.outline(LINE, &[]);
        c.shade(FUR, SHADE, LINE);
        c.light(FUR, LIGHT, LINE);
        // Chevrons.
        let by = (29.5 - br * 0.6) as i32;
        for (x, y) in [(19, by - 2), (24, by - 3), (29, by - 2), (21, by + 1), (26, by + 1), (31, by + 1)] {
            c.set(x, y, LINE);
            c.set(x - 1, y + 1, LINE);
            c.set(x + 1, y + 1, LINE);
        }
        // Claws on the feet.
        for (x, y) in [(31, 18), (33, 18), (36, 21), (38, 21)] {
            c.set(x, y, WHITE);
        }
        // Its face: eyes shut, a snoring mouth, the nose.
        let e = if f.state == State::Offline { Eyes::Shut } else { Eyes::Asleep };
        c.eye(3, 30, 3, 2, e, (0, 0), PUPIL, 0, 0, false);
        c.eye(10, 30, 3, 2, e, (0, 0), PUPIL, 0, 0, true);
        c.set(7, 29, PUPIL);
        c.set(8, 29, PUPIL);
        let snore = (0.5 + 0.5 * br).max(0.0);
        if snore > 0.45 {
            c.ellipse(7.5, 35.0, 1.0 + snore * 1.4, 0.6 + snore * 1.1, MOUTH);
        } else {
            c.line(4, 35, 11, 35, PUPIL);
        }
        if f.state == State::Offline {
            for col in [FUR, SHADE, LIGHT] {
                c.swap(col, PALE);
            }
        }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#6e7894", "#9aa5ba", "#7c879e", "#c6cedb", "#f3ebd7", "#ffffff", "#0d0d16", "#b03a52", "#78d050", "#3f9c3e", "#6a7ee0", "#a4e2ff",
        "#d6974a", "#9a6838", "#f6ff9e", "#d2d5de",
    ];
    const W: usize = 40;
    const H: usize = 40;

    fn new(seed: u64) -> Self {
        Char {
            grin: -99.0,
            next_grin: 5.0 + hash(seed) * 4.0,
            flick: -99.0,
            flick_left: false,
            next_flick: 2.0 + hash(seed ^ 5) * 3.0,
            yawn: -99.0,
            next_yawn: 3.0,
        }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_flick {
            self.flick = f.t;
            self.flick_left = f.rand(f.t as u64) < 0.5;
            self.next_flick = f.t + 2.5 + f.rand(f.t as u64 + 9) * 4.0;
        }
        if f.state == State::Idle && f.t > self.next_grin {
            self.grin = f.t;
            self.next_grin = f.t + 7.0 + f.rand(f.t as u64 + 2) * 6.0;
        }
        if f.context > 0.85 && f.t > self.next_yawn {
            self.yawn = f.t;
            self.next_yawn = f.t + 5.0 + f.rand(f.t as u64 + 4) * 3.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        if f.state == State::Sleeping {
            self.draw_asleep(f, c);
            return;
        }
        let t = f.t;
        let hurt = f.hurt();
        let roar = f.happy();
        let flying = f.is(&[State::Running, State::Testing]);
        let listen = f.is(&[State::Listening, State::Transcribing]);
        let nope = f.within(f.times.nope, 0.5);
        let dance = f.groove > 0.1;
        let umbrella = f.thinking() && !roar && !hurt;
        let reading = f.reading() && !roar && !hurt;

        // Ears: up listening, flat when hurt or saying no, blown back flying, one down asking.
        let (mut lt, mut rt) = ((11.5, 0.0), (28.5, 0.0));
        if hurt || nope {
            lt = (6.5, 5.0);
            rt = (33.5, 5.0);
        } else if listen {
            lt = (12.5, -1.0);
            rt = (27.5, -1.0);
        } else if flying {
            lt = (8.0, 2.5);
            rt = (32.0, 2.5);
        } else if f.state == State::Asking {
            rt = (33.0, 5.0);
        }
        if let Some(u) = f.progress(self.flick, 0.25) {
            let k = (u * PI).sin() * 2.0;
            if self.flick_left { lt.0 -= k } else { rt.0 += k }
        }
        if dance {
            let b = f.wave(PI * 2.2, 0.0) * 1.5 * f.groove;
            lt.0 += b;
            rt.0 += b;
        }
        c.triangle(lt, (9.5, 11.0), (16.5, 7.0), FUR);
        c.triangle(rt, (23.5, 7.0), (30.5, 11.0), FUR);

        // The leaf on its head: wobbling, hopping with a key.
        let hop = if f.within(f.times.key, 0.2) { -1.5 } else { 0.0 };
        let sway = f.wave(2.0, 0.0) * 0.8 + if dance { f.wave(PI * 2.2, 0.0) * 1.5 } else { 0.0 };
        c.line(20, 6, 20, 4 + hop as i32, LEAFD);
        c.ellipse(22.0 + sway, 3.0 + hop, 2.6, 1.3, LEAF);

        // Body, head, belly.
        c.ellipse(20.0, 27.0, 15.5, 12.5, FUR);
        c.ellipse(20.0, 15.5, 11.0, 9.5, FUR);
        c.ellipse(20.0, 30.0, 11.0, 9.0, BELLY);

        // Arms: down, up roaring or dancing, out flying, kneading, holding the umbrella or a
        // leaf, up in a fright.
        let knead = if f.state == State::Editing { f.wave(10.0, 0.0) * 2.0 } else { 0.0 };
        let (mut lh, mut rh) = ((4.5, 31.0 + knead), (35.5, 31.0 - knead));
        if roar || hurt {
            lh = (2.5, 11.0);
            rh = (37.5, 11.0);
        } else if dance {
            let s = f.wave(PI * 2.2, 0.0) * 3.0;
            lh = (3.0 + s, 10.0);
            rh = (37.0 + s, 10.0);
        } else if flying {
            lh = (1.5, 21.0 + f.wave(6.0, 0.0));
            rh = (38.5, 21.0 - f.wave(6.0, 0.0));
        } else if reading {
            lh = (12.0, 27.0);
            rh = (28.0, 27.0);
        } else if umbrella {
            rh = (35.0, 17.0);
        } else if f.state == State::Asking {
            rh = (34.0, 24.0);
        }
        c.thick((8.0, 21.0), lh, 4.6, FUR);
        c.thick((32.0, 21.0), rh, 4.6, FUR);

        // Feet with claws.
        c.ellipse(13.0, 38.3, 4.2, 1.9, FUR);
        c.ellipse(27.0, 38.3, 4.2, 1.9, FUR);

        c.outline(LINE, &[]);
        c.shade(FUR, SHADE, LINE);
        c.light(FUR, LIGHT, LINE);

        // Bristling all over in a fright.
        if hurt {
            for k in 0..14 {
                let a = -PI + k as f64 * PI / 13.0;
                let (x, y) = (20.0 + a.cos() * 12.5, 15.5 + a.sin() * 10.5);
                c.set(x as i32, y as i32, LINE);
                c.set((20.0 + a.cos() * 13.5) as i32, (15.5 + a.sin() * 11.5) as i32, LIGHT);
            }
            for k in 0..6 {
                let y = 22.0 + k as f64 * 2.6;
                let jig = if k % 2 == 0 { 1 } else { 0 };
                c.set(3 - jig, y as i32, LIGHT);
                c.set(36 + jig, y as i32, LIGHT);
            }
        }

        // Chevrons on the belly: three over four.
        for (x, y) in [(16, 23), (20, 23), (24, 23), (14, 26), (18, 26), (22, 26), (26, 26)] {
            c.set(x, y, LINE);
            c.set(x - 1, y + 1, LINE);
            c.set(x + 1, y + 1, LINE);
            c.set(x - 2, y + 2, SHADE);
            c.set(x + 2, y + 2, SHADE);
        }
        // Claws.
        for x in [10, 12, 14, 25, 27, 29] {
            c.set(x, 39, WHITE);
        }

        // Whiskers, outside the face, twitching (bristling when it talks).
        let tw = if f.state == State::Speaking { f.wave(25.0, 0.0) * f.voice * 8.0 } else { f.wave(1.3, 0.0) * 0.6 };
        for (k, y) in [(0, 15), (1, 17), (2, 19)] {
            let end = y as f64 + (k as f64 - 1.0) * 2.0 + tw;
            c.line(8, y, 3, end.round() as i32, LIGHT);
            c.line(32, y, 37, end.round() as i32, LIGHT);
        }

        // Eyes: white rounds with a little pupil; tiny pupils when frightened or roaring.
        let tired = f.context > 0.85 && !roar && !hurt;
        let mut eyes = if hurt { Eyes::Wide } else if roar { Eyes::Wide } else { f.eyes() };
        if reading {
            eyes = Eyes::Open;
        }
        if f.state == State::Offline {
            eyes = Eyes::Asleep;
        }
        let mut look = f.look();
        if reading {
            look = (look.0, 1);
        }
        for (cx, right) in [(14.5, false), (25.5, true)] {
            match eyes {
                Eyes::Open | Eyes::Wide => {
                    let r = if eyes == Eyes::Wide { 3.0 } else { 2.6 };
                    c.ellipse(cx, 12.0, r + 0.7, r + 0.7, PUPIL);
                    c.ellipse(cx, 12.0, r, r, WHITE);
                    let (px, py) = ((cx - 1.0) as i32 + look.0, 11 + look.1);
                    if hurt || roar {
                        c.set(px + if right { 0 } else { 1 }, py + 1, PUPIL);
                    } else {
                        c.rect(px, py, 2, 2, PUPIL);
                        c.set(px + if right { 1 } else { 0 }, py, WHITE);
                    }
                    if tired {
                        c.rect(cx as i32 - 3, 9, 7, 3, FUR);
                        c.line(cx as i32 - 3, 12, cx as i32 + 3, 12, LINE);
                    }
                }
                e => c.eye(cx as i32 - 2, 11, 4, 3, e, look, PUPIL, 0, 0, right),
            }
        }
        // The nose.
        c.line(19, 9, 21, 9, PUPIL);
        c.set(20, 10, PUPIL);

        // The mouth: a wide calm line; open with the voice; a grin full of teeth now and then;
        // the roar; a yawn when tired.
        let grin = f.within(self.grin, 1.3) && f.state == State::Idle;
        let yawning = f.progress(self.yawn, 1.6).map(|u| (u * PI).sin());
        let open = if roar {
            1.0
        } else if hurt {
            0.45
        } else if let Some(y) = yawning {
            y * 0.85
        } else if reading {
            0.0
        } else {
            f.mouth()
        };
        if open > 0.08 {
            let rx = 3.0 + open * 6.0;
            let ry = 0.8 + open * 4.2;
            c.ellipse(20.0, 17.0 + ry, rx + 1.0, ry + 1.0, PUPIL);
            c.ellipse(20.0, 17.0 + ry, rx, ry, MOUTH);
            // Teeth: a row on top and, wide open, a row below.
            let top = (17.0 + 0.5) as i32;
            for x in ((20.0 - rx + 1.0) as i32..=(20.0 + rx - 1.0) as i32).step_by(2) {
                if c.get(x, top) == MOUTH {
                    c.set(x, top, WHITE);
                }
                if open > 0.5 {
                    let bot = (17.0 + ry * 2.0 - 1.0) as i32;
                    if c.get(x, bot) == MOUTH {
                        c.set(x, bot, WHITE);
                    }
                }
            }
        } else if grin {
            c.ellipse(20.0, 18.0, 9.0, 2.2, PUPIL);
            c.ellipse(20.0, 18.0, 8.0, 1.2, WHITE);
            for x in (13..=27).step_by(2) {
                c.set(x, 18, LINE);
            }
        } else {
            c.line(13, 18, 27, 18, PUPIL);
            c.set(12, 17, PUPIL);
            c.set(28, 17, PUPIL);
        }

        // Reading: a big leaf held in both hands.
        if reading {
            let flip = (t * 1.5) as i32 % 2;
            c.ellipse(20.0, 26.5, 7.5, 4.5, LEAFD);
            c.ellipse(20.0, 26.5, 6.5, 3.5, LEAF);
            c.line(14, 26 + flip, 26, 27 - flip, LEAFD);
            for x in [16, 20, 24] {
                c.set(x, 25, LEAFD);
                c.set(x + 1, 28, LEAFD);
            }
            c.ellipse(12.0, 27.0, 2.0, 2.0, FUR);
            c.ellipse(28.0, 27.0, 2.0, 2.0, FUR);
        }
        // Error: the raindrop's water on its head, dripping down.
        if let Some(u) = f.progress(f.times.error, 1.3) {
            if u > 0.1 {
                c.ellipse(20.0, 6.5, 7.0 - u * 2.0, 1.8, RAIN);
                for (k, x) in [(0.0, 12), (0.5, 28), (0.25, 17), (0.75, 24)] {
                    let y = 7 + ((u * 1.6 + k) * 14.0) as i32 % 14;
                    c.set(x, y, RAIN);
                    c.set(x, y + 1, RAIN);
                }
            }
        }
        // Asking: an acorn held out.
        if f.state == State::Asking && !hurt {
            c.rect(34, 20, 3, 1, CAP);
            c.rect(34, 21, 3, 2, ACORN);
            c.set(35, 23, ACORN);
        }

        if f.state == State::Offline {
            for col in [FUR, SHADE, LIGHT] {
                c.swap(col, PALE);
            }
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // Flying on its top: lifted, bobbing.
        if f.is(&[State::Running, State::Testing]) {
            m.dy += 0.26 + 0.03 * f.wave(5.0, 0.0);
        }
        // The roar puffs it up.
        if let Some(u) = f.progress(f.times.done, 1.4) {
            let k = (u * PI).sin();
            m.sx *= 1.0 + 0.06 * k;
            m.sy *= 1.0 + 0.08 * k;
        }
        // The raindrop squashes it.
        if let Some(u) = f.progress(f.times.error, 0.5) {
            let k = (u * PI).sin();
            m.sy *= 1.0 - 0.12 * k;
            m.sx *= 1.0 + 0.08 * k;
        }
        if f.state == State::Asking {
            m.lean += 0.05;
        }
        // The sprout dance: a bigger sway.
        if f.groove > 0.1 {
            m.lean += 0.06 * f.groove * f.wave(PI * 1.1, 0.0);
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        // Thinking: the umbrella open over its head, rain drumming on it.
        if f.thinking() && !f.happy() && !f.hurt() {
            for y in -6..17 {
                w.px(35.0, y as f64, LINE);
            }
            w.px(34.0, 17.0, LINE);
            w.px(33.0, 16.0, LINE);
            // The canopy: a dome with ribs.
            for row in 0..5 {
                let half = 3.0 + row as f64 * 2.2;
                let y = -11.0 + row as f64;
                let mut x = 35.0 - half;
                while x <= 35.0 + half {
                    let rib = ((x - 35.0).abs() as i32) % 4 == 0 && row > 0;
                    w.px(x, y, if rib { LINE } else { BLUE });
                    x += 1.0;
                }
            }
            for k in [-10.0, -6.0, -2.0, 2.0, 6.0, 10.0] {
                w.px(35.0 + k, -6.0, BLUE);
            }
            // Drops falling on it and bouncing off.
            for k in 0..7 {
                let ph = (t * 1.3 + f.rand(k) * 3.0) % 1.0;
                let x = 22.0 + f.rand(k + 30) * 26.0;
                let y = -26.0 + ph * 16.0;
                w.px(x, y, RAIN);
                w.px(x, y + 1.0, RAIN);
                if ph > 0.85 {
                    w.px(x - 1.0, -13.0, RAIN);
                    w.px(x + 1.0, -13.0, RAIN);
                }
            }
        }
        // Reading: leaves drifting in to its leaf.
        if f.reading() {
            for k in 0..5 {
                let ph = (t * 0.6 + k as f64 / 5.0) % 1.0;
                let a = k as f64 * 1.3 + ph * 2.0;
                let r = 24.0 * (1.0 - ph);
                leaf(w, 20.0 + a.cos() * r, 26.0 + a.sin() * r * 0.7, a * 2.0 + t * 3.0);
            }
        }
        // Flying: the spinning top under it, wind streaming behind.
        if f.is(&[State::Running, State::Testing]) {
            let spin = (t * 20.0) as i32 % 3;
            for row in 0..6 {
                let half = [5, 7, 6, 4, 2, 1][row];
                for x in -half..=half {
                    let stripe = (x + spin + 9) % 3 == 0;
                    w.px(20.0 + x as f64, 40.0 + row as f64, if stripe { CAP } else { ACORN });
                }
            }
            w.px(20.0, 46.0, GLOW);
            for k in 0..5 {
                let ph = (t * 3.0 + k as f64 * 0.37) % 1.0;
                let y = 8.0 + k as f64 * 7.0;
                let x = -2.0 - ph * 14.0;
                w.px_dim(x, y, LIGHT);
                w.px_dim(x - 1.0, y, LIGHT);
                w.px_dim(x + 42.0 + ph * 28.0, y + 3.0, LIGHT);
            }
        }
        // Git: a sapling grows beside it and forks.
        if f.state == State::Git {
            let g = (f.st / 2.5).min(1.0);
            let h = 4.0 + g * 14.0;
            for y in 0..h as i32 {
                w.px(44.0, 39.0 - y as f64, CAP);
            }
            if g > 0.4 {
                let top = 39.0 - h;
                for k in 0..((g - 0.4) * 10.0) as i32 {
                    w.px(44.0 + k as f64, top + 6.0 - k as f64, CAP);
                    w.px(44.0 - k as f64 * 0.7, top + 4.0 - k as f64, CAP);
                }
                for (dx, dy) in [(0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (5.0, 0.0), (6.0, 1.0), (-3.0, 0.0), (-4.0, 1.0)] {
                    w.px(44.0 + dx, top + dy, LEAF);
                }
            }
        }
        // Delegating: Chibi and Chu Totoro hop around it.
        if f.state == State::Delegating {
            for k in 0..2 {
                let a = t * 0.9 + k as f64 * PI;
                let x = 17.0 + a.cos() * 26.0;
                let hop = (t * 6.0 + k as f64).sin().abs() * 3.0;
                if a.sin() > -0.3 {
                    mini(w, x, 33.0 - hop, if k == 0 { WHITE } else { BLUE });
                }
            }
        }
        // Done: the roar's gust, leaves blown away.
        if let Some(u) = f.progress(f.times.done, 1.4) {
            for k in 0..8 {
                let a = -0.5 + k as f64 * 0.14;
                let r = 6.0 + u * 34.0 + f.rand(k + 50) * 6.0;
                let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                let (x, y) = (20.0 + side * a.cos() * r, 20.0 + a.sin() * r * 0.6);
                leaf(w, x, y, t * 9.0 + k as f64);
            }
            for k in 0..3 {
                let y = 16.0 + k as f64 * 4.0;
                let r = 12.0 + u * 20.0;
                for s in 0..4 {
                    w.px_dim(20.0 - r - s as f64, y, WHITE);
                    w.px_dim(20.0 + r + s as f64, y, WHITE);
                }
            }
        }
        // Error: a huge raindrop falls on it and bursts.
        if let Some(u) = f.progress(f.times.error, 1.3) {
            if u < 0.12 {
                let y = -20.0 + u / 0.12 * 20.0;
                for (dx, dy) in [(0.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0), (-1.0, 2.0), (0.0, 2.0), (1.0, 2.0), (0.0, 3.0)] {
                    w.px(20.0 + dx, y + dy - 3.0, RAIN);
                }
            } else {
                let r = (u - 0.12) * 26.0;
                for k in 0..10 {
                    let a = PI + k as f64 * PI / 9.0;
                    let fall = (u - 0.12) * (u - 0.12) * 40.0;
                    w.px(20.0 + a.cos() * r, 4.0 + a.sin() * r * 0.7 + fall, RAIN);
                }
                // Water running down it.
                for k in 0..4 {
                    let y = 6.0 + ((u - 0.12) * 30.0 + k as f64 * 8.0) % 32.0;
                    w.px(7.0 + k as f64 * 8.5, y, RAIN);
                }
            }
        }
        // A key: a tiny leaf flutters off its head.
        if let Some(u) = f.progress(f.times.key, 0.6) {
            leaf(w, 23.0 + u * 6.0, 1.0 - u * 6.0, u * 12.0);
        }
        // Copying: Chibi Totoro pops up beside it with an acorn.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let rise = (u * 6.0).min(1.0);
            let out = ((u - 0.8) * 5.0).max(0.0);
            let x = 38.0 + out * 12.0;
            let y = 39.0 - rise * 6.0 - (t * 8.0).sin().abs() * 1.5;
            mini(w, x, y, WHITE);
            acorn(w, x + 1.0, y - 5.0);
            if u > 0.15 && u < 0.8 {
                w.px(x - 2.0, y - 3.0, GLOW);
                w.px(x + 6.0, y - 4.0, GLOW);
            }
        }
        // Merge: two acorns bump into one.
        if let Some(u) = f.progress(f.times.merge, 1.2) {
            let d = (1.0 - (u * 2.0).min(1.0)) * 8.0;
            acorn(w, 18.0 - d, -6.0);
            if d > 0.5 {
                acorn(w, 18.0 + d, -6.0);
            } else {
                w.px(17.0, -9.0, GLOW);
                w.px(21.0, -9.0, GLOW);
            }
        }
        // Music: the sprout dance, sprouts popping out of the ground.
        if f.groove > 0.1 {
            for k in 0..4 {
                let x = [-6.0, -2.0, 42.0, 46.0][k];
                let h = 1.0 + (f.beat * 2.0 + k as f64 * 0.8).sin().abs() * 4.0 * f.groove;
                for y in 0..h as i32 {
                    w.px(x, 39.0 - y as f64, LEAFD);
                }
                let top = 39.0 - h.floor();
                w.px(x - 1.0, top, LEAF);
                w.px(x + 1.0, top, LEAF);
            }
        }
        // Booting: leaves falling with it.
        if f.state == State::Booting && f.st < 2.0 {
            for k in 0..6 {
                let y = -14.0 + ((f.st * 14.0 + k as f64 * 9.0) % 50.0);
                leaf(w, -4.0 + k as f64 * 9.5 + (t * 2.0 + k as f64).sin() * 2.0, y, t * 3.0 + k as f64);
            }
        }
        // Asleep: a leaf riding on the belly, and a snore puff.
        if f.state == State::Sleeping {
            let br = f.wave(0.9, 0.0);
            leaf(w, 24.0, 22.0 - br * 1.2, 0.3);
            if br > 0.7 {
                let k = (br - 0.7) / 0.3;
                w.px_dim(8.0 - k * 4.0, 36.0 + k * 2.0, WHITE);
                w.px_dim(9.0 - k * 6.0, 37.0 + k * 2.0, WHITE);
            }
        }
    }
}
