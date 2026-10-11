//! A Waddle Dee ally with a bandana: the body of each subagent next to Kirby (a recipe part
//! with `kid = "only"`). The bandana is in the state's ink, which in a subagent is its own
//! color: what it's doing. It walks in place, looks where it looks, blinks, and when it goes
//! wrong it gets dizzy, with stars circling its head.

use core::f64::consts::{PI, TAU};
use iris_piece::export_part;
use iris_piece::part::*;

const ORANGE: u8 = own(0);
const PEACH: u8 = own(1);
const DARK: u8 = own(2);
const YELLOW: u8 = own(3);
const BLUSH: u8 = own(4);
const WHITE: u8 = own(5);
const LINE: u8 = own(6);

const BX: usize = 0;
const BY: usize = 1;
const BR: usize = 2;
const GX: usize = 3;
const GY: usize = 4;
const BLINK: usize = 5;
const RED: usize = 6;

struct Ally;

fn len(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn ellipse(u: f64, v: f64, cx: f64, cy: f64, a: f64, b: f64) -> f64 {
    (len((u - cx) / a, (v - cy) / b) - 1.0) * a.min(b)
}

/// Is (dx, dy) inside a small five-pointed star of radius `s`?
fn in_star(dx: f64, dy: f64, s: f64, rot: f64) -> bool {
    let r = len(dx, dy);
    if r > s {
        return false;
    }
    let f = ((dy.atan2(dx) - rot - PI / 2.0) * 5.0 / TAU).rem_euclid(1.0);
    let tip = (f * 2.0 - 1.0).abs();
    r < s * (0.42 + 0.58 * tip * tip)
}

struct Look {
    t: f64,
    w: f64,
    min: f64,
    gaze: (f64, f64),
    blink: bool,
    dizzy: bool,
    step: f64,
}

/// What dot (`u`, `v`) of the Dee is (its units: 1 = its radius, y up).
fn dot(l: &Look, u: f64, v: f64) -> Option<(u8, u8)> {
    let w = l.w;
    // Dizzy: three stars going round over its head.
    if l.dizzy && v > 0.75 {
        for k in 0..3 {
            let a = l.t * 4.0 + k as f64 * TAU / 3.0;
            let (sx, sy) = (a.cos() * 0.6, 1.2 + a.sin() * 0.15);
            if in_star(u - sx, v - sy, 0.2_f64.max(l.min * 1.5), l.t * 3.0) {
                return Some((3, YELLOW));
            }
        }
    }
    if u * u + v * v > 1.6 * 1.6 {
        return None;
    }
    // The bandana's knot, flapping behind on the right.
    let flap = (l.t * 8.0).sin();
    for (cy, k) in [(0.66, 1.0), (0.5, -0.8)] {
        if ellipse(u, v, 1.08 + 0.04 * flap, cy + 0.05 * flap * k, 0.22, 0.08_f64.max(l.min)) < 0.0 {
            return Some((3, INK_MAIN));
        }
    }
    // The feet, stepping in place.
    for (fx, lift) in [(-0.45, l.step.max(0.0)), (0.45, (-l.step).max(0.0))] {
        let d = ellipse(u, v, fx, -0.92 + lift * 0.12, 0.38, 0.2);
        if d < 0.0 {
            return Some((3, if d > -w { LINE } else { YELLOW }));
        }
    }
    let body = ellipse(u, v, 0.0, 0.0, 1.0, 0.95);
    if body >= 0.0 {
        return None;
    }
    if body > -w {
        return Some((3, LINE));
    }
    // The bandana across the top of its head.
    if v > 0.42 && v < 0.68 {
        return Some((3, INK_MAIN));
    }
    let face = ellipse(u, v, 0.0, -0.12, 0.74, 0.58);
    if face < 0.0 {
        for side in [-1.0, 1.0] {
            let (cx, cy) = (side * 0.24 + l.gaze.0, -0.04 + l.gaze.1);
            let (a, b) = (0.1_f64.max(l.min), 0.2_f64.max(l.min * 1.6));
            if l.dizzy {
                // Spirals: a ring with a dot in the middle.
                let r = len(u - cx, v - cy);
                if (r - a * 1.1).abs() < w * 0.7 || r < w * 0.6 {
                    return Some((3, DARK));
                }
            } else if l.blink {
                if (u - cx).abs() < a * 1.2 && (v - cy).abs() < w * 0.7 {
                    return Some((3, DARK));
                }
            } else if ellipse(u, v, cx, cy, a, b) < 0.0 {
                let shine = len(u - cx + a * 0.2, v - cy - b * 0.45) < a * 0.5;
                return Some((3, if shine { WHITE } else { DARK }));
            }
        }
        for side in [-1.0, 1.0] {
            if ellipse(u, v, side * 0.48, -0.3, 0.13, 0.07_f64.max(l.min * 0.7)) < 0.0 {
                return Some((3, BLUSH));
            }
        }
        return Some((3, PEACH));
    }
    Some((3, ORANGE))
}

impl Part for Ally {
    const INPUTS: &'static [&'static str] = &["body.x", "body.y", "body.radius", "gaze.x", "gaze.y", "blink", "red"];
    const SETTINGS: &'static [Setting] = &[Setting::new("size", 0.95, 0.4, 1.6)];
    const COLORS: &'static [&'static str] = &["#ff9a3a", "#ffd9b0", "#1a1030", "#ffd23a", "#ff8fb8", "#ffffff", "#b85a1a"];

    fn new(_io: &Io, _rand: &mut Rand) -> Self {
        Ally
    }

    fn paint(&self, g: &mut Grid, io: &Io) {
        let v = |i: usize| io.input(i);
        let r = v(BR) * io.setting(0);
        let du = io.du();
        if !(r > du * 1.5) || !v(BX).is_finite() || !v(BY).is_finite() {
            return;
        }
        let t = io.t();
        let lim = 0.06;
        let g0 = |x: f64| if x.is_finite() { (x / r * 0.4).clamp(-lim, lim) } else { 0.0 };
        let look = Look {
            t,
            w: 1.1 * du / r,
            min: 1.2 * du / r,
            gaze: (g0(v(GX)), g0(v(GY))),
            blink: v(BLINK) > 0.5,
            dizzy: v(RED) > 0.35,
            step: (t * 7.0).sin(),
        };
        let (cx, cy) = (v(BX), v(BY) + 0.04 * r * (t * 14.0).sin().abs());

        let (dw, dh) = io.grid_size();
        if dw < 2 || dh < 2 {
            return;
        }
        let (x0, y0) = io.center(0, 0);
        let (x1, y1) = io.center(1, 1);
        let (di, dj) = (x1 - x0, y1 - y0);
        if di == 0.0 || dj == 0.0 {
            return;
        }
        let col = |x: f64| ((x - x0) / di).round();
        let row = |y: f64| ((y - y0) / dj).round();
        let (a, b) = (col(cx - 1.4 * r), col(cx + 1.4 * r));
        let (c, d) = (row(cy + 1.5 * r), row(cy - 1.2 * r));
        let clamp = |n: f64, m: usize| n.clamp(0.0, m as f64 - 1.0) as usize;
        for j in clamp(c.min(d), dh)..=clamp(c.max(d), dh) {
            let vv = (y0 + j as f64 * dj - cy) / r;
            for i in clamp(a.min(b), dw)..=clamp(a.max(b), dw) {
                let u = (x0 + i as f64 * di - cx) / r;
                if let Some((tone, ink)) = dot(&look, u, vv) {
                    g.dot(i, j, tone, ink);
                }
            }
        }
    }
}

export_part!(Ally);
