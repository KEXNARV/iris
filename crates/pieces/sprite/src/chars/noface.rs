//! Sin Cara / Kaonashi (El viaje de Chihiro): drawn from shapes every frame (a tall shadowy
//! body with a wavy hem, thin arms that come out when needed, the white mask with its purple
//! marks, and the huge mouth hidden in its belly). It is see-through by nature: parts of it fade
//! away and come back. What it does:
//! - waiting: swaying, the hem rippling, fading a little now and then, a shy "ah...";
//! - listening: leaning in, eyes wide; asking: an empty palm held out, head tilted;
//! - thinking: it goes see-through, vanishing in bands that run down its body;
//! - speaking: "ah... ah...": the mask's mouth with the voice, little "ah"s floating off;
//! - reading, searching, on the web: it eats: food flies into the mouth in its belly, chomping;
//! - running, testing: gliding along, leaving afterimages of itself behind;
//! - editing: rubbing its hands, making gold appear; git: its shadow splits into two branches;
//! - delegating: little shadows with masks float around it;
//! - done: it bows, offering gold, and gold rains around it;
//! - error: it swells up, the belly mouth opens grotesquely with all its teeth, and it spits
//!   out what it ate;
//! - a key: an "ah"; copying: its hand comes out full of gold nuggets;
//! - music: swaying with arms open, gold glints on the beat;
//! - asleep: slumped, almost transparent, wisps rising; offline: only the mask is left, pale;
//! - booting: it fades in from nothing; little context left: it melts, dripping.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Sin Cara";
pub const SCENE: &str = "spooky";
pub const DESCRIPTION: &str = "Sin Cara (Kaonashi) en pixel art: se vuelve transparente al pensar, devora lo que lee, ofrece pepitas de oro al copiar y con un error se agranda y abre su boca grotesca";

const LINE: u8 = 1;
const BODY: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const MASK: u8 = 5;
const MASKSH: u8 = 6;
const MARK: u8 = 7;
const HOLE: u8 = 8;
const MOUTH: u8 = 9;
const GOLD: u8 = 10;
const GOLDD: u8 = 11;
const TONGUE: u8 = 12;
const FROG: u8 = 13;
const SWEET: u8 = 14;
const GHOST: u8 = 15;
const PALE: u8 = 16;

const BAYER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

pub struct Char {
    /// A shy "ah..." now and then while waiting, and a passing fade.
    ah: f64,
    next_ah: f64,
}

/// Fades the body (not the mask, mouth or gold) where `amount(y)` says, with an ordered dither:
/// 0 solid, 1 gone.
fn fade(c: &mut Canvas, amount: impl Fn(i32) -> f64) {
    for y in 0..c.h as i32 {
        let a = amount(y);
        if a <= 0.0 {
            continue;
        }
        for x in 0..c.w as i32 {
            let p = c.get(x, y);
            if matches!(p, BODY | SHADE | LIGHT | LINE) {
                let th = BAYER[((y & 3) * 4 + (x & 3)) as usize] as f64 / 16.0;
                if th < a {
                    c.clear(x, y);
                }
            }
        }
    }
}

/// "ah" in tiny letters, its top-left at (x, y).
fn ah(w: &mut World, x: f64, y: f64, col: u8, dim: bool) {
    let rows = ["....#...", "....#...", ".##.###.", "#.#.#.#.", ".##.#.#.", "........", ".....#.#"];
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            if ch == '#' {
                if dim {
                    w.px_dim(x + i as f64, y + j as f64, col);
                } else {
                    w.px(x + i as f64, y + j as f64, col);
                }
            }
        }
    }
}

/// A gold nugget at (x, y).
fn nugget(w: &mut World, x: f64, y: f64) {
    w.px(x, y, GOLD);
    w.px(x + 1.0, y, GOLD);
    w.px(x, y + 1.0, GOLDD);
    w.px(x + 1.0, y + 1.0, GOLD);
}

/// Its silhouette, faint, at (dx, dy) from where it stands: an afterimage.
fn ghost(w: &mut World, dx: f64, dy: f64, col: u8) {
    for k in 0..14 {
        let a = PI + k as f64 * PI / 13.0;
        w.px_dim(17.0 + dx + a.cos() * 9.0, 12.0 + dy + a.sin() * 10.0, col);
    }
    for k in 0..12 {
        let y = 14.0 + k as f64 * 2.4;
        let s = (y - 12.0) / 29.0;
        w.px_dim(8.5 - s * 5.0 + dx, y + dy, col);
        w.px_dim(25.5 + s * 5.0 + dx, y + dy, col);
    }
    w.px_dim(15.0 + dx, 11.0 + dy, MASK);
    w.px_dim(19.0 + dx, 11.0 + dy, MASK);
}

/// A tiny masked shadow, 5 wide, its top-left at (x, y).
fn shadeling(w: &mut World, x: f64, y: f64, flap: bool) {
    let rows = [".bbb.", "bmmmb", "bhmhb", "bmmmb", "bbbbb", if flap { "b.b.b" } else { ".b.b." }];
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            let col = match ch {
                'b' => LIGHT,
                'm' => MASK,
                'h' => HOLE,
                _ => 0,
            };
            w.px(x + i as f64, y + j as f64, col);
        }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#9d93c4", "#4d4769", "#3d3857", "#6e6794", "#f6f3f8", "#c8c1da", "#a95ed0", "#0a0810", "#c63c4e", "#ffcd45", "#d48c20", "#ff7590",
        "#6fd66c", "#ff9ccc", "#7f77aa", "#dcd8e8",
    ];
    const W: usize = 34;
    const H: usize = 44;

    fn new(seed: u64) -> Self {
        Char { ah: -99.0, next_ah: 4.0 + hash(seed) * 4.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.state == State::Idle && f.t > self.next_ah {
            self.ah = f.t;
            self.next_ah = f.t + 6.0 + f.rand(f.t as u64 + 3) * 6.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let hurt = f.hurt();
        let swell = f.progress(f.times.error, 1.6).map(|u| (u * PI).sin().max(0.0).powf(0.5)).unwrap_or(0.0);
        let bow = f.happy();
        let asleep = f.asleep();
        let tired = f.context > 0.85 && !asleep;
        let eating = f.reading() && !hurt && !bow;
        let gliding = f.is(&[State::Running, State::Testing]);
        let dance = f.groove > 0.1;
        let copying = f.within(f.times.copy, 2.6);

        // Where the head is: slumped asleep or tired, sinking a little when it eats.
        let sink = if asleep { 9.0 } else if tired { 3.0 } else { 0.0 };
        let hy = 12.0 + sink;
        let hx = 17.0 + if dance { f.wave(PI * 1.1, 0.0) * 1.5 * f.groove } else { 0.0 };

        // The body: a dome of a head widening down to a wavy hem.
        let flutter = if gliding { 2.0 } else { 1.0 };
        let wide = if asleep { 3.0 } else { 0.0 };
        c.ellipse(hx, hy, 9.0, 10.0, BODY);
        c.quad((hx - 9.0, hy + 1.0), (hx + 9.0, hy + 1.0), (30.0 + wide, 40.0), (4.0 - wide, 40.0), BODY);
        for k in 0..6 {
            let x = 4.0 - wide + k as f64 * (26.0 + wide * 2.0) / 5.0;
            let wv = f.wave(3.0 * flutter, k as f64 * 1.3) * 0.8 * flutter;
            c.ellipse(x, 40.5 + wv * 0.5, 3.0, 2.5 + wv * 0.4, BODY);
        }

        // Arms: hidden unless needed. A hand out offering (gold when copying), hands up in a
        // fright, rubbing hands editing, arms open dancing, an empty palm asking.
        let arm = |c: &mut Canvas, from: (f64, f64), to: (f64, f64)| {
            c.thick(from, to, 2.4, BODY);
            c.ellipse(to.0, to.1, 1.6, 1.4, BODY);
        };
        let lsh = (hx - 7.5, hy + 9.0);
        let rsh = (hx + 7.5, hy + 9.0);
        if hurt {
            let claw = f.wave(14.0, 0.0);
            arm(c, lsh, (1.5, 9.0 + claw));
            arm(c, rsh, (32.5, 9.0 - claw));
        } else if bow || copying || f.state == State::Asking {
            arm(c, rsh, (32.0, hy + 12.0));
        } else if f.state == State::Editing {
            let r = f.wave(9.0, 0.0) * 1.5;
            arm(c, lsh, (hx - 2.0 + r, hy + 15.0));
            arm(c, rsh, (hx + 2.0 + r, hy + 15.0));
        } else if dance {
            let s = f.wave(PI * 2.2, 0.0) * 2.0;
            arm(c, lsh, (1.5, hy + 4.0 + s));
            arm(c, rsh, (32.5, hy + 4.0 - s));
        } else if gliding {
            arm(c, rsh, (32.0, hy + 8.0 + f.wave(6.0, 0.0)));
        }

        c.outline(LINE, &[GOLD, GOLDD]);
        c.shade(BODY, SHADE, LINE);
        c.light(BODY, LIGHT, LINE);

        // The mouth in its belly: chomping while it eats, gaping with teeth in a fright.
        let gape = if hurt { 0.5 + swell * 0.5 } else if eating { (0.5 + 0.5 * f.wave(7.0, 0.0)).powi(2) * 0.55 } else { 0.0 };
        if gape > 0.05 {
            let (mx, my) = (hx, hy + 19.0);
            let (rx, ry) = (3.0 + gape * 7.0, 0.8 + gape * 7.0);
            c.ellipse(mx, my, rx + 1.0, ry + 1.0, LINE);
            c.ellipse(mx, my, rx, ry, MOUTH);
            if ry > 2.5 {
                c.ellipse(mx, my + ry * 0.55, rx * 0.6, ry * 0.4, TONGUE);
            }
            // Teeth along the top and the bottom: fangs, longer the wider it gapes.
            let long = if gape > 0.6 { 2 } else { 1 };
            let inside = |p: u8| p == MOUTH || p == TONGUE;
            for x in ((mx - rx + 1.0) as i32..=(mx + rx - 1.0) as i32).step_by(2) {
                if let Some(y) = ((my - ry - 1.0) as i32..=my as i32).find(|&y| inside(c.get(x, y))) {
                    for k in 0..long {
                        c.set(x, y + k, MASK);
                    }
                }
                if gape > 0.3 {
                    if let Some(y) = (my as i32..=(my + ry + 1.0) as i32).rev().find(|&y| inside(c.get(x + 1, y))) {
                        for k in 0..long {
                            c.set(x + 1, y - k, MASK);
                        }
                    }
                }
            }
        }

        // The mask: its shading, its marks, the eye holes and a small mouth.
        let tilt = if f.state == State::Asking { 1.0 } else if f.is(&[State::Listening, State::Transcribing]) { 0.0 } else { 0.0 };
        let (mx, my) = (hx + tilt * 0.5, hy + 1.0);
        c.ellipse(mx + 0.6, my + 0.6, 5.7, 7.8, MASKSH);
        c.ellipse(mx, my, 5.4, 7.5, MASK);
        let (lx, rx) = (mx - 3.0, mx + 3.0);
        // Purple marks above and below the eyes.
        c.ellipse(lx, my - 4.3, 0.8, 1.6, MARK);
        c.ellipse(rx, my - 4.3, 0.8, 1.6, MARK);
        let tear = if bow { 1.6 } else { 2.2 };
        c.ellipse(lx, my + 2.6, 1.0, tear, MARK);
        c.ellipse(rx, my + 2.6, 1.0, tear, MARK);
        // Eyes: dark slits; wide listening or in a fright; closed asleep; arcs bowing.
        let ey = (my - 1.0) as i32;
        let look = f.look();
        for x in [lx, rx] {
            let x = x as i32 - 1;
            if asleep || f.blink || bow {
                c.line(x, ey + 1, x + 2, ey + 1, HOLE);
                if bow {
                    c.set(x + 1, ey, HOLE);
                    c.clear(x + 1, ey + 1);
                    c.set(x + 1, ey + 1, MASK);
                }
            } else if hurt || f.is(&[State::Listening, State::Transcribing]) {
                c.rect(x, ey - 1, 3, 3, HOLE);
            } else {
                c.rect(x, ey, 3, 2, HOLE);
                // A glint where it looks.
                if look.0 != 0 || look.1 != 0 {
                    c.set(x + 1 + look.0, ey + if look.1 > 0 { 1 } else { 0 }, MASKSH);
                }
            }
        }
        // The small mouth on the mask: "ah..." with the voice.
        let ahh = f.progress(self.ah, 1.6).map(|u| ((u * PI * 2.0).sin()).max(0.0)).unwrap_or(0.0);
        let open = if f.state == State::Speaking { f.mouth() } else { ahh * 0.6 + if f.within(f.times.key, 0.2) { 0.4 } else { 0.0 } };
        let mouth_y = (my + 4.5) as i32;
        if open > 0.15 {
            c.ellipse(mx, mouth_y as f64 + 0.5, 1.2 + open * 0.8, 0.6 + open * 1.2, HOLE);
        } else {
            c.line(mx as i32 - 1, mouth_y, mx as i32 + 1, mouth_y, HOLE);
            c.set(mx as i32 - 2, mouth_y - 1, MASKSH);
            c.set(mx as i32 + 2, mouth_y - 1, MASKSH);
        }

        // Gold in its hand: offering nuggets.
        if copying || bow {
            for (dx, dy) in [(-2, -2), (-1, -2), (0, -2), (1, -2), (2, -2), (-1, -3), (0, -3), (1, -3), (0, -4), (-1, -4)] {
                let (x, y) = (32 + dx, (hy + 12.0) as i32 + dy);
                c.set(x, y, if (dx * 3 + dy) % 4 == 0 { GOLDD } else { GOLD });
            }
            c.set(31, (hy + 12.0) as i32 - 4, MASK);
        }

        // See-through: thinking vanishes in bands running down it; it fades in booting,
        // waits half-there asleep, and flickers now and then while waiting.
        if f.thinking() {
            fade(c, |y| 0.55 + 0.45 * (y as f64 * 0.35 - t * 3.0).sin());
        } else if f.state == State::Booting {
            let a = 1.0 - (f.st / 1.5).min(1.0);
            fade(c, |_| a);
        } else if f.state == State::Offline {
            fade(c, |_| 0.85);
        } else if asleep {
            fade(c, |y| 0.4 + 0.15 * (y as f64 * 0.2 + t).sin());
        } else if gliding {
            fade(c, |y| if y > 30 { (y - 30) as f64 / 14.0 } else { 0.0 });
        } else if f.state == State::Idle {
            let flick = (0.5 + 0.5 * (t * 0.45 + 1.5).sin()).powi(24);
            fade(c, |y| flick * 0.6 * (y as f64 / 44.0));
        }
        if tired {
            // Melting: the bottom thins out.
            fade(c, |y| if y > 34 { 0.3 + 0.1 * (t * 2.0).sin() } else { 0.0 });
        }
        if f.state == State::Offline {
            for col in [MASK, MASKSH, MARK] {
                c.swap(col, PALE);
            }
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // A slow ghostly sway (gliding leans into it).
        m.lean += 0.03 * f.wave(0.8, 0.0);
        if f.is(&[State::Running, State::Testing]) {
            m.lean += 0.08;
            m.dy += 0.04 * f.wave(3.0, 0.0);
        }
        if f.is(&[State::Listening, State::Transcribing]) {
            m.lean -= 0.05;
        }
        if f.state == State::Asking {
            m.lean += 0.05;
        }
        // Bowing when done.
        if let Some(u) = f.progress(f.times.done, 1.4) {
            m.lean += 0.14 * (u * PI).sin();
        }
        // Swelling up with an error.
        if let Some(u) = f.progress(f.times.error, 1.6) {
            let k = (u * PI).sin().max(0.0).powf(0.5);
            m.sx *= 1.0 + 0.3 * k;
            m.sy *= 1.0 + 0.22 * k;
        }
        // Floating: it barely touches the floor.
        m.dy += 0.02 + 0.015 * f.wave(1.4, 0.0);
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        // Speaking: "ah..." floating off.
        if f.state == State::Speaking && f.voice > 0.03 {
            let ph = (t * 0.5) % 1.0;
            ah(w, 28.0 + ph * 4.0, 6.0 - ph * 10.0, LINE, ph > 0.6);
        }
        // Waiting: its shy "ah...".
        if let Some(u) = f.progress(self.ah, 1.6) {
            if f.state == State::Idle {
                ah(w, 28.0, 4.0 - u * 4.0, GHOST, u > 0.6);
            }
        }
        if let Some(u) = f.progress(f.times.key, 0.5) {
            ah(w, -8.0, 6.0 - u * 3.0, GHOST, true);
        }
        // Eating: food flies into the mouth in its belly.
        if f.reading() && !f.hurt() {
            for k in 0..4 {
                let ph = (t * 0.8 + k as f64 * 0.25) % 1.0;
                let side = if k % 2 == 0 { -1.0 } else { 1.0 };
                let x = 17.0 + side * (1.0 - ph) * 24.0;
                let y = 31.0 - (ph * PI).sin() * 10.0 - (1.0 - ph) * 4.0;
                let col = [FROG, SWEET, GOLD, SWEET][k];
                w.px(x, y, col);
                w.px(x + 1.0, y, col);
                w.px(x, y + 1.0, col);
                if col == FROG {
                    w.px(x + 1.0, y - 1.0, MASK);
                }
            }
        }
        // Gliding: afterimages left behind.
        if f.is(&[State::Running, State::Testing]) {
            let ph = (t * 2.0) % 1.0;
            ghost(w, -6.0 - ph * 6.0, 0.0, GHOST);
            ghost(w, -14.0 - ph * 6.0, 1.0, SHADE);
        }
        // Git: its shadow splits into two branches.
        if f.state == State::Git {
            let s = (f.st * 0.8).min(1.0) * 12.0 + f.wave(1.5, 0.0) * 1.5;
            ghost(w, -s, -s * 0.3, GHOST);
            ghost(w, s, -s * 0.3, GHOST);
        }
        // Delegating: little masked shadows float around it.
        if f.state == State::Delegating {
            for k in 0..3 {
                let a = t * 0.8 + k as f64 * 2.1;
                let (x, y) = (15.0 + a.cos() * 24.0, 16.0 + a.sin() * 12.0);
                shadeling(w, x, y, (t * 4.0 + k as f64) as i32 % 2 == 0);
            }
        }
        // Copying: gold glints round the hand, a nugget dropping now and then.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            for k in 0..4 {
                let a = t * 4.0 + k as f64 * PI / 2.0;
                if (t * 9.0 + k as f64).sin() > 0.0 {
                    w.px(32.0 + a.cos() * 4.0, 21.0 + a.sin() * 3.0, GOLD);
                }
            }
            let d = (u * 3.0) % 1.0;
            nugget(w, 31.0, 22.0 + d * 18.0);
        }
        // Done: gold rains all around it.
        if let Some(u) = f.progress(f.times.done, 1.6) {
            for k in 0..10 {
                let x = -8.0 + f.rand(k) * 50.0;
                let y = -16.0 + ((u * 1.4 + f.rand(k + 20)) % 1.0) * 58.0;
                if y < 40.0 {
                    nugget(w, x, y);
                }
                if k % 3 == 0 && (t * 12.0 + k as f64).sin() > 0.3 {
                    w.px(x + 2.0, y - 2.0, MASK);
                }
            }
        }
        // Error: it spits out what it ate.
        if let Some(u) = f.progress(f.times.error, 1.4) {
            for k in 0..6 {
                let side = if k % 2 == 0 { -1.0 } else { 1.0 };
                let sp = 0.6 + f.rand(k + 40) * 0.8;
                let x = 17.0 + side * u * 30.0 * sp;
                let y = 31.0 - u * 18.0 * sp + u * u * 30.0;
                let col = [FROG, SWEET, GOLD][k as usize % 3];
                w.px(x, y, col);
                w.px(x + 1.0, y, col);
                w.px(x, y + 1.0, col);
            }
        }
        // Merge: two nuggets clink into one.
        if let Some(u) = f.progress(f.times.merge, 1.2) {
            let d = (1.0 - (u * 2.0).min(1.0)) * 8.0;
            nugget(w, 16.0 - d, -5.0);
            nugget(w, 16.0 + d, -5.0);
            if d < 0.5 {
                w.px(14.0, -7.0, MASK);
                w.px(19.0, -7.0, MASK);
            }
        }
        // Music: gold glints on the beat.
        if f.groove > 0.1 && (f.beat * PI * 2.0).sin() > 0.6 {
            for k in 0..3 {
                w.px(-4.0 + k as f64 * 20.0, 4.0 + (k as f64 * 7.0) % 10.0, GOLD);
            }
        }
        // Asleep: wisps rising off it.
        if f.state == State::Sleeping {
            for k in 0..3 {
                let ph = (t * 0.4 + k as f64 / 3.0) % 1.0;
                let x = 8.0 + k as f64 * 9.0 + (t * 2.0 + k as f64).sin() * 1.5;
                w.px_dim(x, 20.0 - ph * 22.0, GHOST);
                w.px_dim(x + 1.0, 21.0 - ph * 22.0, GHOST);
            }
        }
        // Booting: wisps gathering into it.
        if f.state == State::Booting && f.st < 1.5 {
            let r = (1.0 - f.st / 1.5) * 22.0;
            for k in 0..8 {
                let a = k as f64 * PI / 4.0 + t;
                w.px_dim(17.0 + a.cos() * r, 22.0 + a.sin() * r, GHOST);
            }
        }
        // Little context left: it melts, dripping.
        if f.context > 0.85 && !f.asleep() {
            for k in 0..3 {
                let ph = (t * 0.7 + k as f64 / 3.0) % 1.0;
                w.px(8.0 + k as f64 * 9.0, 42.0 + ph * 5.0, LIGHT);
            }
        }
    }
}
