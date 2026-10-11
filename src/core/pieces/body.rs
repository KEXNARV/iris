//! BODY: the blob. An outline that breathes, pulses with the voice, ripples according to the
//! state and sprouts spikes on an error; inside, the layers the buddy passes it (pupil, lens,
//! reading, liquid…) and beneath them all a pattern. It can carry a bud attached (the sprout).

use std::f64::consts::{PI, TAU};

use super::{Piece, Fill};
use crate::core::context::Context;
use crate::core::grid::Grid;

pub(crate) struct Blob {
    /// Outline thickness: at least this many dots, even if the panel is small.
    band: f64,
    band_dots: f64,
    /// Gap between the outline and what is inside.
    hollow: f64,
    /// Beyond this radius there is no body (except the bud).
    reach: f64,
}

/// The outline's shape in this frame.
pub(crate) struct Shape {
    pub radius: f64,
    pub breath: f64,
    pub beat: f64,
    pub voice: f64,
    /// Ripple, its phase and how much chaos it adds.
    pub amp: f64,
    pub phase: f64,
    pub chaos: f64,
    /// Error: spikes coming out.
    pub red: f64,
    pub t: f64,
}

impl Shape {
    /// The outline's radius at angle `th`.
    fn r_at(&self, th: f64) -> f64 {
        let Shape { radius, breath, beat, voice, amp, phase, chaos, red, t } = *self;
        radius + breath + beat + voice * (0.6 + 0.4 * (5.0 * th + t * 7.0).sin())
            + amp
                * (0.60 * (3.0 * th + phase).sin()
                    + 0.40 * (5.0 * th - phase * 1.6 + 1.0).sin()
                    + 0.30 * chaos * (7.0 * th + phase * 2.7 + 2.0).sin())
            + red * 0.09 * (11.0 * th + t * 20.0).sin().max(0.0)
    }
}

pub(crate) struct Paint<'a> {
    pub cx: f64,
    pub cy: f64,
    /// Stretched by the spring.
    pub sx: f64,
    pub sy: f64,
    pub shape: Shape,
    /// Offline: dotted outline and nothing inside.
    pub dashed: f64,
    /// Editing: a cursor that runs along the outline.
    pub stitch: f64,
    pub tip: Option<(f64, f64, f64)>,
    pub ink: u8,
    /// The inner layers, from the topmost to the bottommost. Beneath them all, the pattern.
    pub fills: &'a [&'a dyn Fill],
}

impl Blob {
    pub fn new() -> Self {
        Blob { band: 0.065, band_dots: 1.9, hollow: 0.035, reach: 0.8 }
    }
}

impl Piece for Blob {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (cx, cy, sx, sy) = (m.cx, m.cy, m.sx, m.sy);
        let cursor = ctx.t * 2.6;
        let band = self.band.max(self.band_dots * g.du);
        let err = m.ink;
        for j in 0..g.dh {
            for i in 0..g.dw {
                let (x, y) = g.center(i, j);
                let (dx, dy) = ((x - cx) / sx, (y - cy) / sy);
                let r = dx.hypot(dy);
                let mut in_bud = false;
                if let Some((bx, by, br)) = m.tip {
                    let e = br - (x - bx).hypot(y - by);
                    if (0.0..0.05).contains(&e) {
                        g.dot(i, j, 3, err);
                        continue;
                    }
                    in_bud = e >= 0.05;
                }
                if r > self.reach && !in_bud {
                    continue;
                }
                let th = dy.atan2(dx);
                let d = m.shape.r_at(th) - r;
                if (0.0..band).contains(&d) {
                    if m.dashed > 0.5 && (th * 18.0).sin() <= 0.0 {
                        continue;
                    }
                    let mut tone = 3;
                    if m.stitch > 0.5 {
                        let da = ((th - cursor).rem_euclid(TAU) - PI).abs();
                        tone = if da > PI - 0.5 { 3 } else { 2 };
                    }
                    g.dot(i, j, tone, err);
                } else if (d > band + self.hollow || in_bud) && m.dashed < 0.5 {
                    if m.fills.iter().any(|c| c.dot(g, i, j, x, y)) {
                        continue;
                    }
                    if (i + j * 2) % 3 == 0 && j % 2 == 0 {
                        g.dot(i, j, 2, err);
                    }
                }
            }
        }
    }
}
