//! QUESTION (can't hear you): a "?" floating next to the drooping body.

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct QuestionMark {
    /// The glyph, dot by dot (column, row).
    glyph: [(i32, i32); 9],
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub droop: f64,
}

impl QuestionMark {
    pub fn new() -> Self {
        QuestionMark { glyph: [(0, 4), (1, 5), (2, 5), (3, 4), (3, 3), (2, 2), (1, 1), (1, 0), (1, -2)] }
    }
}

impl Piece for QuestionMark {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, a) = (ctx.t, m.anchor);
        if m.droop > 0.5 {
            let (qx, qy) = (a.cx + 0.32, a.cy + a.base + 0.12 + 0.02 * (t * 3.0).sin());
            for (a, b) in self.glyph {
                g.plot(qx + a as f64 * 0.025 - 0.04, qy + b as f64 * 0.03, 3, INK_MAIN);
            }
        }
    }
}
