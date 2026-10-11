//! Dream Land around Kirby: what tells what Iris is doing, drawn Kirby's way. It takes the
//! place of the original's arcs, motes and signs.
//!
//! - An orbit of stars (the arcs): their speed, length and color follow the script's.
//! - A sky that twinkles (the motes), and Kirby's shadow on the ground.
//! - Each state's sign: Zs while sleeping, a thought cloud while thinking, a "?" when asking,
//!   sound coming in while listening, notes while speaking or with music, dust while running,
//!   a warp star crossing while on the web, a spark per key.

use core::f64::consts::{PI, TAU};
use iris_piece::export_part;
use iris_piece::part::*;

const YELLOW: u8 = own(0);
const WHITE: u8 = own(1);
const PINK: u8 = own(2);
const CLOUD: u8 = own(3);
const ORANGE: u8 = own(4);
const PEACH: u8 = own(5);
const GRAY: u8 = own(6);
const DOOR: u8 = own(7);
const BLUE: u8 = own(8);
const DARK: u8 = own(9);

// The inputs, in the order of `INPUTS`.
const BX: usize = 0;
const BY: usize = 1;
const BR: usize = 2;
const ARC_PHASE: usize = 3;
const ARCS: usize = 4;
const ARC_LEN: usize = 5;
const RED: usize = 6;
const ZZZ: usize = 7;
const ORBIT: usize = 8;
const MOTES: usize = 9;
const PACKETS: usize = 10;
const SWEEP: usize = 11;
const VOICE: usize = 12;

/// How many stars the orbit has, and how many twinkle in the sky.
const ORBIT_STARS: usize = 8;
const SKY: usize = 26;

const QUESTION: [&str; 7] = [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."];
const ZED: [&str; 5] = ["#####", "...#.", "..#..", ".#...", "#####"];
const NOTE: [&str; 7] = ["..##.", "..#.#", "..#..", "..#..", "###..", "###..", "....."];

/// Something that comes and goes, in Kirby's units (1 = its radius, around its center).
#[derive(Clone, Copy)]
struct Bit {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
    /// For notes: which color; for dust and sparks, unused.
    alt: bool,
}

struct Dreamland {
    seed: u64,
    /// The sky: where each star is (angle, distance) and its twinkle's phase.
    sky: Vec<(f64, f64, f64)>,
    dust: Vec<Bit>,
    notes: Vec<Bit>,
    zs: Vec<Bit>,
    sparks: Vec<Bit>,
    /// Time until the next note, Z and dust puff; when the warp star started crossing.
    next_note: f64,
    next_z: f64,
    next_dust: f64,
    warp: f64,
    /// Stars it spits toward a newborn subagent (`alt` false) and those that come back when one
    /// finishes well (`alt` true).
    spat: Vec<Bit>,
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}

fn ease_out_back(u: f64) -> f64 {
    let u = u - 1.0;
    1.0 + 2.7 * u * u * u + 1.7 * u * u
}

fn len(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Where the Waddle Dee is (Kirby's units) and how big, if it's out: it walks along the ground
/// every 24 s while idle; every other walk Kirby inhales it halfway. `pull` (0..1) is how far
/// into being inhaled it is.
fn dee(state: State, st: f64) -> Option<(f64, f64, f64, f64)> {
    if state != State::Idle || st < 4.0 {
        return None;
    }
    let walk = ((st - 4.0) / 24.0) as u64;
    let s = (st - 4.0) % 24.0;
    let u = s / 10.0;
    if walk % 2 == 0 {
        return (u < 1.0).then_some((-2.7 + 5.4 * u, -0.92, 1.0, 0.0));
    }
    // It gets to -1.35, then it's pulled to the mouth, shrinking, in 0.6 s.
    let reach = 1.35 / 5.4 * 10.0;
    if s < reach {
        return Some((-2.7 + 5.4 * u, -0.92, 1.0, 0.0));
    }
    let p = (s - reach) / 0.6;
    if p >= 1.0 {
        return None;
    }
    let e = p * p;
    Some((-1.35 + 1.35 * e, -0.92 + 0.8 * e + 0.25 * (p * PI).sin(), 1.0 - e * 0.85, p))
}

/// Is (dx, dy) inside a heart of radius about `s`? (The classic curve, y up.)
fn in_heart(dx: f64, dy: f64, s: f64) -> bool {
    let (x, y) = (dx / s * 1.15, dy / s * 1.15 + 0.15);
    let a = x * x + y * y - 1.0;
    a * a * a - x * x * y * y * y <= 0.0
}

/// Is (dx, dy) inside a five-pointed star of radius `s`, turned `rot`?
fn in_star(dx: f64, dy: f64, s: f64, rot: f64) -> Option<bool> {
    let r = len(dx, dy);
    if r > s {
        return None;
    }
    let f = ((dy.atan2(dx) - rot - PI / 2.0) * 5.0 / TAU).rem_euclid(1.0);
    let tip = (f * 2.0 - 1.0).abs();
    let edge = s * (0.42 + 0.58 * tip * tip);
    (r < edge).then_some(r < edge * 0.45)
}

/// The grid as dots: turns world boxes into dot ranges and back.
struct Dots {
    x0: f64,
    y0: f64,
    di: f64,
    dj: f64,
    dw: usize,
    dh: usize,
}

impl Dots {
    fn new(io: &Io) -> Option<Dots> {
        let (dw, dh) = io.grid_size();
        if dw < 2 || dh < 2 {
            return None;
        }
        let (x0, y0) = io.center(0, 0);
        let (x1, y1) = io.center(1, 1);
        let (di, dj) = (x1 - x0, y1 - y0);
        (di != 0.0 && dj != 0.0).then_some(Dots { x0, y0, di, dj, dw, dh })
    }

    /// Every dot in the box (world units) that `f` says to paint.
    fn area(&self, g: &mut Grid, (xa, ya, xb, yb): (f64, f64, f64, f64), f: impl Fn(f64, f64) -> Option<(u8, u8)>) {
        let col = |x: f64| ((x - self.x0) / self.di).round();
        let row = |y: f64| ((y - self.y0) / self.dj).round();
        let clamp = |n: f64, m: usize| n.clamp(0.0, m as f64 - 1.0) as usize;
        let (a, b) = (col(xa), col(xb));
        let (c, d) = (row(ya), row(yb));
        if a.max(b) < 0.0 || c.max(d) < 0.0 || a.min(b) >= self.dw as f64 || c.min(d) >= self.dh as f64 {
            return;
        }
        for j in clamp(c.min(d), self.dh)..=clamp(c.max(d), self.dh) {
            let y = self.y0 + j as f64 * self.dj;
            for i in clamp(a.min(b), self.dw)..=clamp(a.max(b), self.dw) {
                if let Some((tone, ink)) = f(self.x0 + i as f64 * self.di, y) {
                    g.dot(i, j, tone, ink);
                }
            }
        }
    }
}

/// A star of radius `s` at (x, y): its middle in `core`, the rest in `ink`.
fn star(g: &mut Grid, d: &Dots, (x, y): (f64, f64), s: f64, rot: f64, tone: u8, ink: u8, core: u8) {
    d.area(g, (x - s, y - s, x + s, y + s), |px, py| in_star(px - x, py - y, s, rot).map(|mid| (tone, if mid { core } else { ink })));
}

/// A small bitmap (rows of '#') with its top-left at (x, y), each cell `cell` wide.
fn glyph(g: &mut Grid, rows: &[&str], (x, y): (f64, f64), cell: f64, du: f64, tone: u8, ink: u8) {
    let n = (cell / du).round().max(1.0) as usize;
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            if ch != b'#' {
                continue;
            }
            let (cx, cy) = (x + c as f64 * cell, y - r as f64 * cell);
            for a in 0..n {
                for b in 0..n {
                    g.plot(cx + a as f64 * du, cy - b as f64 * du, tone, ink);
                }
            }
        }
    }
}

impl Dreamland {
    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        (self.seed.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Part for Dreamland {
    const INPUTS: &'static [&'static str] =
        &["body.x", "body.y", "body.radius", "arc_phase", "arcs", "arc_len", "red", "zzz", "orbit", "motes", "packets", "sweep", "body.voice"];
    const SETTINGS: &'static [Setting] = &[Setting::new("size", 0.9, 0.4, 1.6)];
    const OUTPUTS: &'static [&'static str] = &["dee.pull"];
    const COLORS: &'static [&'static str] = &["#ffe14a", "#ffffff", "#ffa3cf", "#d9d4ff", "#ff9a3a", "#ffd9b0", "#8a8fa8", "#3a2a6a", "#6a7dff", "#1a1030"];

    fn new(_io: &Io, rand: &mut Rand) -> Self {
        let mut d = Dreamland {
            seed: (rand.rand() * 1e15) as u64 | 1,
            sky: vec![],
            dust: vec![],
            notes: vec![],
            zs: vec![],
            sparks: vec![],
            next_note: 0.0,
            next_z: 0.0,
            next_dust: 0.0,
            warp: -99.0,
            spat: vec![],
        };
        for _ in 0..SKY {
            let (a, r, p) = (d.rand() * TAU, 2.1 + d.rand() * 1.6, d.rand() * TAU);
            d.sky.push((a, r, p));
        }
        d
    }

    fn step(&mut self, io: &mut Io) {
        let (t, dt) = (io.t(), io.dt().clamp(0.0, 0.1));
        // Tells Kirby when the Waddle Dee is being inhaled (in step: layout's outputs only last
        // the frame being drawn).
        let pull = dee(io.state(), io.st()).map_or(0.0, |d| if d.3 > 0.0 { 1.0 } else { 0.0 });
        io.put(0, pull);
        let state = io.state();
        let v = |i: usize| io.input(i);

        // Notes: while speaking, or with music playing.
        let singing = state == State::Speaking || io.music() > 0.15;
        self.next_note -= dt;
        if singing && self.next_note <= 0.0 {
            self.next_note = 0.45 + self.rand() * 0.35;
            let side = if self.rand() < 0.5 { -1.0 } else { 1.0 };
            let alt = self.rand() < 0.5;
            let x = side * (0.95 + self.rand() * 0.3);
            self.notes.push(Bit { x, y: 0.1, vx: side * 0.12, vy: 0.55, age: 0.0, life: 1.7, alt });
        }
        // Zs: while it sleeps.
        self.next_z -= dt;
        if v(ZZZ) > 0.5 && self.next_z <= 0.0 {
            self.next_z = 0.95;
            self.zs.push(Bit { x: 0.5, y: 0.85, vx: 0.32, vy: 0.36, age: 0.0, life: 2.8, alt: false });
        }
        // Dust: running in place.
        self.next_dust -= dt;
        if (v(SWEEP) > 0.5 || matches!(state, State::Running | State::Testing)) && self.next_dust <= 0.0 {
            self.next_dust = 0.09;
            let side = if self.rand() < 0.5 { -0.5 } else { 0.5 };
            let (vx, vy) = (-0.5 - self.rand() * 0.5, 0.15 + self.rand() * 0.25);
            self.dust.push(Bit { x: side - 0.2, y: -1.02, vx, vy, age: 0.0, life: 0.55, alt: false });
        }
        // The warp star: one crossing every few seconds while on the web.
        if v(PACKETS) > 0.5 && t - self.warp > 2.6 {
            self.warp = t;
        }

        for list in [&mut self.notes, &mut self.zs, &mut self.dust, &mut self.sparks, &mut self.spat] {
            for b in list.iter_mut() {
                b.x += b.vx * dt;
                b.y += b.vy * dt;
                b.age += dt;
            }
            list.retain(|b| b.age < b.life);
        }
    }

    fn event(&mut self, h: Happening, _io: &mut Io) {
        match h {
            Happening::Born(_) => {
                let a = PI * (0.6 + self.rand() * 0.8);
                self.spat.push(Bit { x: 0.0, y: -0.1, vx: a.cos() * 2.4, vy: a.sin() * 2.4, age: 0.0, life: 0.8, alt: false });
            }
            Happening::Finish(_, true) => {
                let a = self.rand() * TAU;
                self.spat.push(Bit { x: a.cos() * 2.2, y: a.sin() * 2.2, vx: -a.cos() * 2.4, vy: -a.sin() * 2.4, age: 0.0, life: 0.9, alt: true });
            }
            _ => {}
        }
        if h == Happening::Event(Event::Key) {
            let x = (self.rand() - 0.5) * 1.8;
            self.sparks.push(Bit { x, y: -1.05, vx: 0.0, vy: 0.4, age: 0.0, life: 0.35, alt: false });
        }
    }

    fn paint(&self, g: &mut Grid, io: &Io) {
        let v = |i: usize| io.input(i);
        let r = v(BR) * io.setting(0);
        let du = io.du();
        if !(r > du * 2.0) || !v(BX).is_finite() || !v(BY).is_finite() {
            return;
        }
        let Some(d) = Dots::new(io) else { return };
        let (bx, by, t) = (v(BX), v(BY), io.t());
        let at = |x: f64, y: f64| (bx + x * r, by + y * r);
        let state = io.state();
        let main = if v(RED) > 0.35 { INK_RED } else { INK_MAIN };

        // The ground: Kirby's shadow, a dim checkered ellipse under its feet.
        let (sa, sb, sy) = (0.95 * r, 0.09 * r, by - 1.12 * r);
        let (x0, y0, di, dj) = (d.x0, d.y0, d.di, d.dj);
        d.area(g, (bx - sa, sy - sb, bx + sa, sy + sb), |x, y| {
            let (i, j) = (((x - x0) / di).round() as i64, ((y - y0) / dj).round() as i64);
            (len((x - bx) / sa, (y - sy) / sb) < 1.0 && (i + j) % 2 == 0 && j % 2 == 0).then_some((1, INK_MAIN))
        });

        // The sky: stars twinkling, a cross when bright, a dot when not.
        let sky = v(MOTES).clamp(0.0, 1.0);
        if sky > 0.05 {
            for (k, &(a, dist, ph)) in self.sky.iter().enumerate() {
                let tw = 0.5 + 0.5 * (t * (0.9 + (k % 5) as f64 * 0.23) + ph).sin();
                if tw * sky < 0.25 {
                    continue;
                }
                let (x, y) = at(a.cos() * dist * 1.25, a.sin() * dist * 0.9);
                let ink = if k % 3 == 0 { YELLOW } else { WHITE };
                g.plot(x, y, if tw > 0.6 { 3 } else { 2 }, ink);
                if tw * sky > 0.8 {
                    for (ox, oy) in [(du, 0.0), (-du, 0.0), (0.0, du), (0.0, -du)] {
                        g.plot(x + ox, y + oy, 2, ink);
                    }
                }
            }
        }

        // The orbit: stars on a tilted ring, as many as the arc is long, the leader biggest.
        let arcs = v(ARCS).clamp(0.0, 1.0);
        if arcs > 0.05 {
            let phase = if v(ARC_PHASE).is_finite() { v(ARC_PHASE) } else { t * 0.25 };
            let shown = ((ORBIT_STARS as f64) * v(ARC_LEN).clamp(0.2, 1.0)).round().max(1.0) as usize;
            for k in 0..shown {
                let a = phase - k as f64 * TAU / ORBIT_STARS as f64;
                let (x, y) = at(a.cos() * 1.85, a.sin() * 0.5 + 0.05);
                let fall = 1.0 - k as f64 / shown as f64 * 0.6;
                let s = (0.15 * r * fall * arcs.max(0.5)).max(2.5 * du);
                star(g, &d, (x, y), s, t * 2.0 + k as f64, 3, main, WHITE);
            }
        }

        // Listening: sound coming in from both sides, closer and brighter.
        if matches!(state, State::Listening | State::Transcribing) {
            let reach = 0.25 + 0.4 * v(VOICE).clamp(0.0, 1.0);
            for k in 0..3 {
                let p = (t * 0.9 + k as f64 / 3.0).fract();
                let rr = 2.3 - 1.0 * p;
                let tone = if p > 0.5 { 3 } else { 2 };
                for side in [0.0, PI] {
                    let steps = (rr * r * reach * 2.0 / du).ceil().max(4.0) as usize;
                    for q in 0..=steps {
                        let a = side - reach + 2.0 * reach * q as f64 / steps as f64;
                        let (x, y) = at(a.cos() * rr, a.sin() * rr);
                        g.plot(x, y, tone, main);
                    }
                }
            }
        }

        // Thinking: a thought cloud up to the right, with dots that take turns.
        if v(ORBIT) > 0.5 || matches!(state, State::Thinking | State::Planning) {
            let cloud: [(f64, f64, f64); 4] = [(1.38, 1.38, 0.2), (1.2, 1.32, 0.15), (1.56, 1.3, 0.15), (1.38, 1.24, 0.17)];
            let bubbles: [(f64, f64, f64); 2] = [(0.98, 0.98, 0.055), (1.1, 1.13, 0.08)];
            let w = 1.1 * du / r;
            let inside = |u: f64, vv: f64, shrink: f64| cloud.iter().any(|&(cx, cy, cr)| len(u - cx, vv - cy) < cr - shrink);
            let (xa, ya) = at(0.9, 0.85);
            let (xb, yb) = at(1.8, 1.65);
            d.area(g, (xa, ya, xb, yb), |x, y| {
                let (u, vv) = ((x - bx) / r, (y - by) / r);
                if inside(u, vv, 0.0) {
                    return (!inside(u, vv, w)).then_some((3, CLOUD));
                }
                bubbles.iter().any(|&(cx, cy, cr)| len(u - cx, vv - cy) < cr.max(w)).then_some((3, CLOUD))
            });
            let lit = (t * 2.5) as usize % 4;
            for k in 0..3 {
                let (x, y) = at(1.27 + k as f64 * 0.11, 1.3);
                let s = (0.03 * r).max(du);
                let tone = if k < lit { 3 } else { 2 };
                star(g, &d, (x, y), s.max(1.2 * du), 0.0, tone, main, main);
            }
        }

        // Asking: a "?" bobbing over its head, to the left.
        if state == State::Asking {
            let cell = (0.075 * r).max(du);
            let (x, y) = at(-1.05, 1.7 + 0.05 * (t * 4.0).sin());
            glyph(g, &QUESTION, (x, y), cell, du, 3, main);
        }

        // Zs, rising and growing.
        for z in &self.zs {
            let u = z.age / z.life;
            let cell = (0.03 * r * (1.0 + u * 1.3)).max(du);
            let (x, y) = at(z.x + 0.08 * (z.age * 2.0).sin(), z.y);
            glyph(g, &ZED, (x, y), cell, du, if u < 0.7 { 3 } else { 2 }, WHITE);
        }

        // Notes, swaying up.
        for n in &self.notes {
            let u = n.age / n.life;
            let cell = (0.03 * r).max(du);
            let (x, y) = at(n.x + 0.07 * (n.age * 5.0).sin(), n.y);
            glyph(g, &NOTE, (x, y), cell, du, if u < 0.75 { 3 } else { 2 }, if n.alt { PINK } else { YELLOW });
        }

        // Dust puffs behind the feet: rings that grow and fade.
        for p in &self.dust {
            let u = p.age / p.life;
            let rr = (0.05 + 0.1 * u) * r;
            let (x, y) = at(p.x, p.y);
            let tone = if u < 0.4 { 3 } else if u < 0.75 { 2 } else { 1 };
            d.area(g, (x - rr, y - rr, x + rr, y + rr), |px, py| (len(px - x, py - y) < rr.max(du)).then_some((tone, WHITE)));
        }

        // A spark per key, at its feet.
        for s in &self.sparks {
            let (x, y) = at(s.x, s.y);
            star(g, &d, (x, y), (0.05 * r).max(1.5 * du), 0.0, 3, YELLOW, WHITE);
        }

        let w = 1.1 * du / r;
        let ellipse = |u: f64, vv: f64, cx: f64, cy: f64, a: f64, b: f64| len((u - cx) / a, (vv - cy) / b) < 1.0;
        let local = |x: f64, y: f64| ((x - bx) / r, (y - by) / r);

        // Its vitality: the context left, as Kirby's health bar under the ground.
        {
            let left = (1.0 - io.context()).clamp(0.0, 1.0);
            let full = (left * 10.0).ceil() as usize;
            let ink = if full <= 2 { INK_RED } else { PINK };
            let (seg, gap, h) = (0.15, 0.03, (0.07_f64).max(1.5 * w));
            let x0 = -(seg * 10.0 + gap * 9.0) / 2.0;
            for k in 0..10 {
                let xa = x0 + k as f64 * (seg + gap);
                let (pa, pb) = (at(xa, -1.38 - h), at(xa + seg, -1.38));
                let lit = k < full;
                d.area(g, (pa.0, pa.1, pb.0, pb.1), |_, _| Some(if lit { (3, ink) } else { (1, INK_MAIN) }));
            }
        }

        // Sleeping: a crescent moon comes out.
        let zzz = v(ZZZ).clamp(0.0, 1.0);
        if zzz > 0.3 {
            let (mx, my) = (-1.65, 1.55);
            let (xa, ya) = at(mx - 0.3, my - 0.3);
            let (xb, yb) = at(mx + 0.3, my + 0.3);
            d.area(g, (xa, ya, xb, yb), |x, y| {
                let (u, vv) = local(x, y);
                (len(u - mx, vv - my) < 0.28 && len(u - mx - 0.13, vv - my - 0.07) > 0.24).then_some((3, YELLOW))
            });
        }

        // Offline: a rain cloud over its head.
        if state == State::Offline {
            let puffs: [(f64, f64, f64); 4] = [(-0.25, 1.55, 0.2), (0.0, 1.65, 0.26), (0.27, 1.55, 0.2), (0.0, 1.47, 0.2)];
            let (xa, ya) = at(-0.5, 1.25);
            let (xb, yb) = at(0.5, 1.95);
            d.area(g, (xa, ya, xb, yb), |x, y| {
                let (u, vv) = local(x, y);
                let inside = |s: f64| puffs.iter().any(|&(cx, cy, cr)| len(u - cx, vv - cy) < cr - s);
                inside(0.0).then(|| if inside(w) { (2, GRAY) } else { (3, GRAY) })
            });
            for k in 0..6 {
                let x = -0.35 + k as f64 * 0.14;
                let fall = (t * 1.6 + k as f64 * 0.37).fract();
                let y = 1.3 - fall * 0.4;
                let (pa, pb) = (at(x, y), at(x, y - 0.08));
                let steps = ((pa.1 - pb.1).abs() / du).ceil() as usize;
                for q in 0..=steps {
                    g.plot(pa.0, pa.1 - q as f64 * du, 3, BLUE);
                }
                let _ = pb;
            }
        }

        // Git: a warp door at its side, with its star.
        if state == State::Git {
            let pop = ease_out_back((io.st() * 3.0).min(1.0));
            let (dx, dy, hw, hh) = (-1.75, -0.45, 0.24 * pop, 0.42 * pop);
            if hw > 0.02 {
                let (xa, ya) = at(dx - hw - 0.05, dy - hh);
                let (xb, yb) = at(dx + hw + 0.05, dy + hh + hw);
                d.area(g, (xa, ya, xb, yb), |x, y| {
                    let (u, vv) = local(x, y);
                    let (ux, uy) = (u - dx, vv - dy);
                    let body = ux.abs() < hw && uy > -hh && uy < hh;
                    let arch = len(ux, uy - hh) < hw;
                    if !(body || arch) {
                        return None;
                    }
                    let edge = ux.abs() > hw - w || uy < -hh + w || (uy > hh && len(ux, uy - hh) > hw - w);
                    if let Some(mid) = in_star(ux, uy - hh * 0.55, hw * 0.55, 0.0) {
                        return Some((3, if mid { WHITE } else { YELLOW }));
                    }
                    Some(if edge { (3, YELLOW) } else { (2, DOOR) })
                });
            }
        }

        // The friend heart it throws to recruit a newborn subagent (as in Star Allies), and the
        // stars that come back when one finishes well.
        for b in &self.spat {
            let u = b.age / b.life;
            if b.alt {
                star(g, &d, at(b.x, b.y), (0.13 * (1.0 - u * 0.6) * r).max(1.5 * du), t * 8.0, 3, YELLOW, WHITE);
                continue;
            }
            let s = (0.12 * (0.6 + u * 0.6) * r).max(2.0 * du);
            let (x, y) = at(b.x, b.y + 0.15 * (u * PI).sin());
            d.area(g, (x - s * 1.2, y - s * 1.2, x + s * 1.2, y + s * 1.2), |px, py| {
                in_heart(px - x, py - y, s).then(|| if len(px - x + s * 0.35, py - y - s * 0.3) < s * 0.25 { (3, WHITE) } else { (3, PINK) })
            });
        }

        // Booting: it arrives riding a warp star, which then flies off.
        if state == State::Booting {
            let st = io.st();
            let (sx, sy) = if st < 1.2 {
                (0.0, -1.05 + (1.0 - ease_out(st / 1.2)) * 3.0)
            } else {
                let u = ((st - 1.2) / 0.6).min(1.0);
                (u * 3.4, -1.05 + u * u * 2.2)
            };
            if st < 1.8 {
                star(g, &d, at(sx, sy), (0.4 * r).max(3.0 * du), 0.0, 3, YELLOW, WHITE);
            }
        }

        // Idle for a while: a Waddle Dee strolls along the ground now and then.
        if let Some((wx, wy, size, pull)) = dee(state, io.st()) {
            {
                let wy = wy + if pull > 0.0 { 0.0 } else { 0.03 * (t * 9.0).sin().abs() };
                let local = |x: f64, y: f64| {
                    let (u, vv) = local(x, y);
                    // Shrunk around its own center while being inhaled.
                    (wx + (u - wx) / size, wy + (vv - wy) / size)
                };
                let step = (t * 9.0).sin();
                let (xa, ya) = at(wx - 0.3 * size, wy - 0.3 * size);
                let (xb, yb) = at(wx + 0.3 * size, wy + 0.3 * size);
                d.area(g, (xa, ya, xb, yb), |x, y| {
                    let (u, vv) = local(x, y);
                    let (ux, uy) = (u - wx, vv - wy);
                    for (fx, lift) in [(-0.09, step.max(0.0)), (0.09, (-step).max(0.0))] {
                        if ellipse(ux, uy, fx, -0.17 + lift * 0.04, 0.09, 0.05) {
                            return Some((3, YELLOW));
                        }
                    }
                    if !ellipse(ux, uy, 0.0, 0.0, 0.19, 0.18) {
                        return None;
                    }
                    // It walks to the right: its face is ahead.
                    for ex in [0.03, 0.11] {
                        if ellipse(ux, uy, ex, 0.04, 0.025_f64.max(w * 0.6), 0.045_f64.max(w)) {
                            return Some((3, DARK));
                        }
                    }
                    if ellipse(ux, uy, 0.07, 0.0, 0.13, 0.12) {
                        return Some((3, PEACH));
                    }
                    Some((3, ORANGE))
                });
            }
        }

        // The warp star: across the sky, with a trail in Kirby's colors.
        let u = (t - self.warp) / 1.4;
        if (0.0..1.0).contains(&u) {
            let path = |u: f64| (-3.2 + 6.4 * u, 1.9 - 1.0 * u + 0.25 * (u * PI).sin());
            for k in 1..14 {
                let back = u - k as f64 * 0.012;
                if back < 0.0 {
                    break;
                }
                let (px, py) = path(back);
                let (x, y) = at(px, py);
                let ink = [YELLOW, PINK, WHITE][k % 3];
                let tone = if k < 7 { 3 } else { 2 };
                for o in [-du, 0.0, du] {
                    g.plot(x, y + o, tone, ink);
                }
            }
            let (px, py) = path(u);
            star(g, &d, at(px, py), (0.16 * r).max(2.0 * du), t * 6.0, 3, YELLOW, WHITE);
        }
    }
}

export_part!(Dreamland);
