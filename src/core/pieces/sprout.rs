//! SPROUT (delegating): a bud that splits from the body, moves away and comes back, joined by
//! a dotted stem. The stem belongs to this piece; the bud is drawn by the body, which merges
//! it into itself.

use std::f64::consts::PI;

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Sprout {
    /// From what amount of outgrowth the stem is visible.
    stem: f64,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub bud: f64,
    /// Body phase: which way it comes out.
    pub phase: f64,
}

impl Sprout {
    pub fn new() -> Self {
        Sprout { stem: 0.35 }
    }

    /// Angle, how far it came out (0..1) and how far from the center it is.
    fn spot(&self, ctx: &Context, m: &Paint) -> (f64, f64, f64) {
        let a = m.phase * 0.9;
        let out = 0.5 + 0.5 * (ctx.t * 1.1 - PI / 2.0).sin();
        let d = m.anchor.base + (0.05 + out * 0.26) * m.bud;
        (a, out, d)
    }

    /// The bud: center and radius, if there is a sprout.
    pub fn tip(&self, ctx: &Context, m: &Paint) -> Option<(f64, f64, f64)> {
        (m.bud > 0.01).then(|| {
            let (a, _, d) = self.spot(ctx, m);
            (m.anchor.cx + d * a.cos(), m.anchor.cy + d * a.sin(), 0.10 + 0.02 * (ctx.t * 3.0).sin())
        })
    }
}

impl Piece for Sprout {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        if m.bud > 0.01 {
            let (a, out, d) = self.spot(ctx, &m);
            if out > self.stem {
                for q in (0..6).step_by(2) {
                    let (x, y) = m.anchor.dot(m.anchor.base + q as f64 / 6.0 * (d - m.anchor.base), a);
                    g.plot(x, y, 1, INK_MAIN);
                }
            }
        }
    }
}
