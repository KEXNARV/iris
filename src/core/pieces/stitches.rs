//! STITCHES (editing): pieces coming in from the scale toward the outline.

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Stitches {
    count: usize,
    /// Where they come in from.
    since: f64,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub stitch: f64,
}

impl Stitches {
    pub fn new() -> Self {
        Stitches { count: 3, since: 0.84 }
    }
}

impl Piece for Stitches {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, base) = (ctx.t, m.anchor.base);
        if m.stitch > 0.01 {
            for k in 0..self.count {
                let v = t * 0.9 + k as f64 / self.count as f64;
                let a = k as f64 * 2.4 + v.floor() * 1.7;
                let r = self.since - v.fract() * (self.since - base - 0.06);
                let (x, y) = m.anchor.dot(r, a);
                g.plot(x, y, 3, INK_MAIN);
                let (x, y) = m.anchor.dot(r + 0.04, a);
                g.plot(x, y, 2, INK_MAIN);
            }
        }
    }
}
