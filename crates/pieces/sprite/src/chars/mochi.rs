//! Mochi: un daifuku de fresa, blandito y redondo, con su relleno rosado asomando alrededor de
//! la fresa que lleva encima. Es masa: todo en él se aplasta, se estira y tiembla. What it does:
//! - waiting: jiggles like jelly, the strawberry leaves flutter, now and then a "boing" squish;
//! - listening: stretches up tall, leaves perked, eyes wide;
//! - thinking: thinks so hard it squashes flat as a pancake, pressure lines at its sides;
//! - speaking: wobbles with the voice, puffs of starch when it talks loud;
//! - reading, searching, on the web: inhales — sakura petals spiral into its mouth;
//! - editing: kneads itself (a quick squish rhythm), starch dust flying;
//! - running, testing: stretches like taffy, a strand of dough dragging behind;
//! - git (and a merge): pulls a blob of itself off to the side (a branch) and snaps it back;
//! - done: cherry blossoms sprout on its head, a ring of flowers bursts out, little nubs up;
//! - error: it cracks and the pink filling oozes out and drips, x x;
//! - a key: a tiny squish and a puff of starch; copying: a mini mochi pops out and hops;
//! - asking: tilts, one little nub raised; music: a jelly wobble dance;
//! - asleep: melts into a puddle beside a steaming cup of green tea;
//! - offline: goes stale — grey, hard and cracked; booting: splats down and wobbles into shape;
//! - little context left: sags and sweats.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Mochi";
pub const SCENE: &str = "sakura";
pub const DESCRIPTION: &str = "Mochi, un daifuku de fresa blandito: se aplasta cuando piensa, se estira como masa al correr, se agrieta y suelta relleno con los errores y le brotan flores de cerezo al terminar";

const LINE: u8 = 1;
const DOUGH: u8 = 2;
const DSH: u8 = 3;
const DLI: u8 = 4;
const FILL: u8 = 5;
const BERRY: u8 = 6;
const BERRY_SH: u8 = 7;
const SEED: u8 = 8;
const LEAF: u8 = 9;
const LEAF_DK: u8 = 10;
const EYE: u8 = 11;
const BLUSH: u8 = 12;
const PETAL: u8 = 13;
const TEA: u8 = 14;
const STALE: u8 = 15;
const MOUTH: u8 = 16;

pub struct Char {
    /// The last idle "boing" (a squish that wobbles out), and the next.
    boing: f64,
    next_boing: f64,
}

/// The dough this frame: center, radii, and the strawberry's spot.
struct Body {
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    top: f64,
    bx: f64,
    /// The git/merge blob: how far out (0: none).
    blob: f64,
    run: bool,
}

const FLOOR: f64 = 34.6;

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}

impl Char {
    fn body(&self, f: &Frame) -> Body {
        let t = f.t;
        let run = f.is(&[State::Running, State::Testing]);
        // `a`: how squashed (positive: flat and wide; negative: tall and thin).
        let mut a = 0.025 * (t * 2.3).sin();
        if let Some(u) = f.progress(self.boing, 0.9) {
            a += 0.22 * (u * PI * 4.0).cos() * (1.0 - u);
        }
        if f.thinking() {
            a = 0.38 + 0.06 * (t * 3.0).sin();
        } else if f.is(&[State::Listening, State::Transcribing]) {
            a = -0.12 + 0.02 * (t * 3.0).sin();
        } else if f.asleep() {
            a = 0.3 + 0.03 * (t * 0.9).sin();
        } else if f.reading() {
            a = -0.07 + 0.05 * (t * 4.0).sin();
        } else if f.state == State::Editing {
            a = 0.12 * (t * 9.0).sin().abs() - 0.02;
        } else if f.state == State::Speaking {
            a += f.voice * 0.9 * (t * 14.0).sin();
        } else if f.state == State::Booting {
            a = 0.55 * (1.0 - ease_out((f.st / 1.4).min(1.0))) * (f.st * 9.0).cos().abs().max(0.3);
        }
        if f.groove > 0.1 {
            a += 0.14 * f.groove * (t * PI * 2.2).sin();
        }
        if f.context > 0.85 && !f.asleep() {
            a += 0.1;
        }
        if f.hurt() {
            a -= 0.06;
        }
        if let Some(u) = f.progress(f.times.done, 0.8) {
            a -= 0.18 * (u * PI).sin();
        }
        if f.within(f.times.key, 0.15) {
            a += 0.08;
        }
        let a = a.clamp(-0.25, 0.48);
        let (mut cx, mut rx, ry) = (20.0, 14.0 * (1.0 + a), 10.5 * (1.0 - a * 0.85));
        if run {
            cx = 23.0;
            rx = 12.5 + 0.6 * (t * 12.0).sin();
        }
        // Git: a blob pulled off to the side and snapped back; a merge: one coming back in.
        let mut blob = 0.0;
        if f.state == State::Git {
            blob = 0.25 + 0.75 * (0.5 + 0.5 * (t * PI / 1.4).sin());
        }
        if let Some(u) = f.progress(f.times.merge, 1.0) {
            blob = blob.max(1.0 - u);
        }
        if blob > 0.0 {
            cx -= 2.5 * blob;
            rx -= 1.5 * blob;
        }
        let rx = rx.min(17.5);
        let cy = FLOOR - ry * 0.75;
        let top = cy - ry;
        let mut bx = cx;
        if f.state == State::Asking {
            bx += 2.0;
        }
        if run {
            bx -= 1.5;
        }
        if f.groove > 0.1 {
            bx += 1.5 * f.groove * (t * PI * 2.2).sin();
        }
        Body { cx, cy, rx, ry, top, bx, blob, run }
    }
}

/// A five-petal sakura blossom (center plus the cross and a notch), `s` 0..1 how open.
fn blossom(w: &mut World, x: f64, y: f64, s: f64) {
    if s < 0.3 {
        w.px(x, y, PETAL);
        return;
    }
    w.px(x, y, SEED);
    for (dx, dy) in [(0.0, -1.0), (1.0, 0.0), (-1.0, 0.0), (0.6, 1.0), (-0.6, 1.0)] {
        w.px(x + dx, y + dy, PETAL);
    }
    if s > 0.7 {
        for (dx, dy) in [(0.0, -2.0), (2.0, -0.4), (-2.0, -0.4), (1.2, 1.8), (-1.2, 1.8)] {
            w.px_dim(x + dx, y + dy, PETAL);
        }
    }
}

/// A mini mochi in the world (for copying): a little dough dome with its own strawberry.
fn mini(w: &mut World, x: f64, y: f64, squash: f64) {
    let (rx, ry) = (3.6 * (1.0 + squash), 2.8 * (1.0 - squash));
    for yy in -4..=1 {
        for xx in -5..=5 {
            let (px, py) = (xx as f64, yy as f64);
            let inside = |dx: f64, dy: f64| {
                let (u, v) = (dx / rx, (dy + ry - 0.5) / ry);
                u * u + v * v <= 1.0 && dy <= 0.5
            };
            if inside(px, py) {
                let edge = !inside(px - 1.0, py) || !inside(px + 1.0, py) || !inside(px, py - 1.0);
                w.px(x + px, y + py, if edge { LINE } else { DOUGH });
            }
        }
    }
    let top = y - 2.0 * ry + 0.5;
    w.px(x, top - 0.5, BERRY);
    w.px(x + 1.0, top - 0.5, BERRY);
    w.px(x, top - 1.5, LEAF);
    w.px(x - 1.5, y - ry + 0.3, EYE);
    w.px(x + 1.5, y - ry + 0.3, EYE);
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#d27f9c", "#fff4ef", "#f1d3db", "#ffffff", "#ff8fb4", "#ff3d55", "#c9243f", "#ffe58a", "#66d96f", "#2fa04c", "#2a1420", "#ffb0c8",
        "#ffc6dc", "#a8e070", "#b9b1bd", "#b0304a",
    ];
    const W: usize = 40;
    const H: usize = 36;

    fn new(seed: u64) -> Self {
        Char { boing: -99.0, next_boing: 3.0 + hash(seed) * 3.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_boing {
            if f.state == State::Idle {
                self.boing = f.t;
            }
            self.next_boing = f.t + 4.0 + f.rand(f.t as u64 + 11) * 4.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let b = self.body(f);
        let hurt = f.hurt();
        let happy = f.happy();

        // Running: a strand of dough dragging behind, like taffy.
        if b.run {
            let wig = (t * 12.0).sin();
            let end = (b.cx - b.rx - 9.0 + wig * 1.5, 31.0 + wig * 0.8);
            c.thick((b.cx - b.rx + 3.0, 30.5), (b.cx - b.rx - 3.0, 31.5), 4.5, DOUGH);
            c.thick((b.cx - b.rx - 3.0, 31.5), end, 2.4, DOUGH);
            c.ellipse(end.0 - 1.0, end.1, 2.2, 1.9, DOUGH);
        }
        // Git: a blob of itself pulled off on a strand.
        if b.blob > 0.05 {
            let bx = b.cx + b.rx + 1.0 + 6.0 * b.blob;
            c.thick((b.cx + b.rx - 3.0, 30.0), (bx - 2.0, 30.5), 2.0 + 2.5 * (1.0 - b.blob), DOUGH);
            c.ellipse(bx, 30.0, 3.6, 3.4, DOUGH);
        }

        // The dough, with a flat bottom where it sits.
        c.ellipse(b.cx, b.cy, b.rx, b.ry, DOUGH);

        // Little nubs: up cheering, one up when asking, waving to the music.
        let nub = |c: &mut Canvas, left: bool, up: f64| {
            let x = if left { b.cx - b.rx + 0.5 } else { b.cx + b.rx - 0.5 };
            let dx = if left { -1.5 } else { 1.5 };
            c.ellipse(x + dx * up, b.cy - 1.0 - up * 4.0, 1.9, 2.2, DOUGH);
        };
        if happy {
            let up = 0.6 + 0.4 * f.wave(18.0, 0.0);
            nub(c, true, up);
            nub(c, false, up);
        } else if f.state == State::Asking {
            nub(c, false, 0.8 + 0.2 * f.wave(10.0, 0.0));
        } else if f.groove > 0.1 {
            let s = f.wave(PI * 2.2, 0.0);
            nub(c, true, (0.5 + 0.5 * s) * f.groove);
            nub(c, false, (0.5 - 0.5 * s) * f.groove);
        } else if hurt {
            nub(c, true, 0.3);
            nub(c, false, 0.3);
        }

        // The filling peeking around the strawberry, the strawberry, its leaves and stem.
        let (bx, top) = (b.bx, b.top);
        c.ellipse(bx, top + 1.8, 6.0, 1.9, FILL);
        c.ellipse(bx, top - 1.6, 4.7, 4.1, BERRY);
        c.triangle((bx - 3.5, top + 0.5), (bx + 3.5, top + 0.5), (bx, top + 3.0), BERRY);
        let flutter = if f.asleep() { 1.5 } else if f.is(&[State::Listening, State::Transcribing]) { -1.5 } else { f.wave(3.1, 0.0) * 0.7 };
        let flutter = flutter + if f.groove > 0.1 { f.wave(PI * 4.4, 0.0) * f.groove } else { 0.0 };
        let ly = top - 5.4;
        c.triangle((bx - 0.5, ly), (bx - 5.5, ly - 0.8 + flutter), (bx - 2.0, ly + 1.6), LEAF);
        c.triangle((bx + 0.5, ly), (bx + 5.5, ly - 0.8 - flutter), (bx + 2.0, ly + 1.6), LEAF);
        c.triangle((bx - 1.5, ly + 0.6), (bx + 1.5, ly + 0.6), (bx + flutter * 0.3, ly - 2.0), LEAF);
        c.line((bx + 0.5) as i32, (ly - 1.0) as i32, (bx + 1.5) as i32, (ly - 3.5) as i32, LEAF_DK);

        // It sits flat: nothing under the floor.
        for y in FLOOR.round() as i32..Self::H as i32 {
            for x in 0..Self::W as i32 {
                c.clear(x, y);
            }
        }
        c.outline(LINE, &[]);
        c.shade(DOUGH, DSH, LINE);
        c.light(DOUGH, DLI, LINE);
        c.shade(BERRY, BERRY_SH, LINE);
        c.shade(LEAF, LEAF_DK, LINE);
        c.shade(FILL, BLUSH, LINE);
        // Seeds on the strawberry, and its shine.
        let (bxi, ti) = (bx.round() as i32, top.round() as i32);
        for (dx, dy) in [(-2, -3), (1, -4), (2, -1), (-1, -1), (-3, 0), (3, -3), (0, 1)] {
            c.set(bxi + dx, ti + dy, SEED);
        }
        c.set(bxi - 2, ti - 4, DLI);

        // The face.
        let (cx, cy) = (b.cx.round() as i32, b.cy.round() as i32);
        let ey = cy - 2 + if f.thinking() { 1 } else { 0 };
        let eyes = if f.thinking() {
            Eyes::Open
        } else if f.state == State::Offline {
            Eyes::Shut
        } else {
            f.eyes()
        };
        let look = f.look();
        c.eye(cx - 6, ey, 2, 3, eyes, look, EYE, DLI, 0, false);
        c.eye(cx + 4, ey, 2, 3, eyes, look, EYE, DLI, 0, true);
        // Blush (bigger and pinker when happy).
        let br = if happy { 2.4 } else { 1.8 };
        c.ellipse(b.cx - 8.0, cy as f64 + 2.0, br, 1.1, BLUSH);
        c.ellipse(b.cx + 8.0, cy as f64 + 2.0, br, 1.1, BLUSH);
        // The mouth.
        let open = f.mouth();
        let my = cy + 1;
        if happy {
            c.mouth(cx, my, 0.9, 5, 3, MOUTH, FILL);
        } else if hurt {
            // A wobbly zigzag.
            for (k, x) in (cx - 2..=cx + 2).enumerate() {
                c.set(x, my + (k as i32 % 2), MOUTH);
            }
        } else if open > 0.08 {
            c.mouth(cx, my, open, 4, 4, MOUTH, FILL);
        } else if f.thinking() {
            c.line(cx - 1, my + 1, cx + 1, my + 1, MOUTH);
        } else if f.state == State::Sleeping {
            c.ellipse(cx as f64 + 0.5, my as f64 + 1.0, 0.8, 0.8, MOUTH);
        } else {
            // A tiny "ω".
            for (x, y) in [(cx - 2, my), (cx - 1, my + 1), (cx, my), (cx + 1, my + 1), (cx + 2, my)] {
                c.set(x, y, MOUTH);
            }
        }

        // Offline: stale — grey, hard, cracked.
        if f.state == State::Offline {
            c.swap(DOUGH, STALE);
            c.swap(DLI, DSH);
            c.swap(BERRY, BERRY_SH);
            c.swap(LEAF, LEAF_DK);
            c.line(cx - 9, cy - 4, cx - 6, cy - 2, LINE);
            c.line(cx - 6, cy - 2, cx - 7, cy + 1, LINE);
            c.line(cx + 7, cy - 6, cx + 9, cy - 3, LINE);
            c.line(cx + 9, cy - 3, cx + 11, cy - 3, LINE);
        }

        // An error: it cracks, and the filling oozes out of the crack and down its side.
        if let Some(u) = f.progress(f.times.error, 1.6) {
            let (x0, y0) = (cx + 3, (b.top + 2.0) as i32);
            let pts = [(x0, y0), (x0 + 2, y0 + 2), (x0 + 1, y0 + 4), (x0 + 4, y0 + 6), (x0 + 3, y0 + 8)];
            for p in pts.windows(2) {
                c.line(p[0].0, p[0].1, p[1].0, p[1].1, FILL);
                c.line(p[0].0 + 1, p[0].1, p[1].0 + 1, p[1].1, LINE);
            }
            // The drip running down.
            let len = ((u * 3.0).min(1.0) * 9.0) as i32;
            let (dx, dy) = (x0 + 4, y0 + 8);
            for k in 0..len {
                c.set(dx, dy + k, FILL);
            }
            if len > 2 {
                c.set(dx - 1, dy + len - 1, FILL);
                c.set(dx + 1, dy + len - 1, FILL);
            }
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // The dough itself already squashes; keep the engine's squash for compacting only and
        // lean into the run.
        if f.is(&[State::Running, State::Testing]) {
            m.lean += 0.08;
        }
        if f.state == State::Asking {
            m.lean += 0.04;
        }
        if f.within(f.times.key, 0.15) {
            m.sy *= 0.95;
            m.sx *= 1.03;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let b = self.body(f);

        // Done: cherry blossoms sprout from its head, then a ring of them bursts out.
        if let Some(u) = f.progress(f.times.done, 2.4) {
            let grow = (u * 5.0).min(1.0);
            for (k, (dx, h)) in [(-7.0, 3.0), (-3.5, 5.0), (3.5, 5.5), (7.0, 3.5), (0.0, 8.5)].iter().enumerate() {
                let g = (grow * 1.4 - k as f64 * 0.1).clamp(0.0, 1.0);
                if g <= 0.0 {
                    continue;
                }
                let (x, base) = (b.cx + dx, b.top + 2.5 + dx.abs() * 0.25);
                let tip = base - h * g;
                let mut y = base - 1.0;
                while y > tip {
                    w.px(x, y, LEAF_DK);
                    y -= 1.0;
                }
                blossom(w, x, tip - 1.0, g);
            }
            if u > 0.15 {
                let r = (u - 0.15) * 34.0;
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0 + u * 2.0;
                    let (x, y) = (b.cx + a.cos() * r, b.cy - 4.0 + a.sin() * r * 0.75);
                    if u < 0.75 || k % 2 == 0 {
                        blossom(w, x, y, 0.6);
                    }
                }
            }
        }

        // An error: drops of filling flying off the crack, crumbs of dough.
        if let Some(u) = f.progress(f.times.error, 1.4) {
            for k in 0..4 {
                let v = k as f64 * 0.8 + 1.0;
                let (x, y) = (b.cx + 5.0 + v * u * 7.0, b.top + 2.0 - u * 9.0 * (1.2 - u) * v + u * u * 14.0);
                w.px(x, y, if k % 2 == 0 { FILL } else { DOUGH });
            }
        }

        // Copying: a mini mochi pops out and hops beside it.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let out = ease_out((u * 4.0).min(1.0));
            let hop = ((u * 4.0 * PI).sin()).abs() * 4.0 * (1.0 - u);
            let squash = if hop < 0.4 { 0.2 } else { -0.05 };
            mini(w, b.cx + b.rx + 2.0 + out * 4.0, FLOOR - 0.5 - hop, squash);
        }

        // Asleep: a cup of green tea beside it, steaming.
        if f.state == State::Sleeping {
            let (x, y) = (b.cx - b.rx - 10.0, FLOOR - 6.0);
            for yy in 0..6 {
                for xx in 0..7 {
                    let edge = xx == 0 || xx == 6 || yy == 5;
                    let col = if yy == 0 { if edge { LINE } else { TEA } } else if edge { LINE } else if yy == 2 { LEAF } else { LEAF_DK };
                    if yy == 5 && (xx == 0 || xx == 6) {
                        continue;
                    }
                    w.px(x + xx as f64, y + yy as f64, col);
                }
            }
            // A sakura on the cup.
            w.px(x + 3.0, y + 3.0, PETAL);
            for k in 0..3 {
                let ph = ((t * 0.7 + k as f64 / 3.0) % 1.0) * 6.0;
                let sx = x + 1.5 + k as f64 * 1.8 + (t * 2.0 + k as f64 * 2.0 + ph).sin() * 0.8;
                if ph < 5.0 {
                    w.px_dim(sx, y - 1.0 - ph, DLI);
                }
            }
        }

        // Thinking: pressure lines pushing in on its sides.
        if f.thinking() && f.wave(6.0, 0.0) > -0.2 {
            for side in [-1.0, 1.0] {
                let x = b.cx + side * (b.rx + 3.0);
                for k in 0..3 {
                    w.px_dim(x + side * k as f64 * 0.0, b.cy - 2.0 + k as f64 * 2.0, DSH);
                }
                w.px(x - side, b.cy, DLI);
            }
        }

        // Reading: inhaling — petals spiral into its mouth.
        if f.reading() {
            let (mx, my) = (b.cx, b.cy + 1.0);
            for k in 0..5 {
                let ph = (t * 0.6 + k as f64 / 5.0) % 1.0;
                let r = 18.0 * (1.0 - ph) + 1.0;
                let a = k as f64 * 1.3 + ph * 4.0;
                w.px(mx + a.cos() * r, my - 6.0 * (1.0 - ph) + a.sin() * r * 0.5, if k % 2 == 0 { PETAL } else { DLI });
            }
        }

        // Speaking loud, kneading, a key: puffs of starch.
        let puff = |w: &mut World, x: f64, y: f64, u: f64, n: u64| {
            for k in 0..5 {
                let a = k as f64 * 1.26 + n as f64;
                w.speck(x + a.cos() * (1.0 + u * 4.0), y + a.sin() * (1.0 + u * 3.0) - u * 2.0, DLI);
            }
        };
        if f.state == State::Speaking && f.voice > 0.1 {
            puff(w, b.cx + 3.0, b.cy + 2.0, (t * 3.0) % 1.0, 0);
        }
        if f.state == State::Editing {
            puff(w, b.cx - b.rx, FLOOR - 1.0, (t * 2.2) % 1.0, 1);
            puff(w, b.cx + b.rx, FLOOR - 1.0, (t * 2.2 + 0.5) % 1.0, 2);
        }
        if let Some(u) = f.progress(f.times.key, 0.35) {
            puff(w, b.cx + if f.times.key as u64 % 2 == 0 { -b.rx } else { b.rx }, FLOOR - 1.0, u, 3);
        }

        // Running: speed lines behind the strand.
        if b.run {
            for k in 0..3 {
                let x = b.cx - b.rx - 12.0 - ((t * 20.0 + k as f64 * 3.0) % 6.0);
                let y = 24.0 + k as f64 * 3.0;
                w.px_dim(x, y, DSH);
                w.px_dim(x - 1.0, y, DSH);
            }
        }

        // Booting: a ring of starch where it lands.
        if f.state == State::Booting && f.st > 0.9 && f.st < 1.8 {
            let u = (f.st - 0.9) / 0.9;
            for k in 0..6 {
                let side = if k % 2 == 0 { -1.0 } else { 1.0 };
                w.speck(b.cx + side * (b.rx + u * 6.0 + k as f64), FLOOR - 0.5 - (k / 2) as f64 * u * 2.0, DLI);
            }
        }

        // Little context left: a sweat drop sliding down.
        if f.context > 0.85 && !f.asleep() {
            let y = b.top + 2.0 + ((t * 0.8) % 1.0) * 4.0;
            let x = b.cx + b.rx - 3.0;
            w.px(x, y, DLI);
            w.px(x, y + 1.0, DSH);
            w.px(x - 1.0, y + 1.0, DLI);
        }
    }
}
