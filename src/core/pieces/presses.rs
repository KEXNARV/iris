//! PRESSES (compacting): two bars that squeeze the body from above and below.

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Presses {
    /// Half width of each bar.
    width: f64,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub press: f64,
}

impl Presses {
    pub fn new() -> Self {
        Presses { width: 0.35 }
    }
}

impl Piece for Presses {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, a, press) = (ctx.t, m.anchor, m.press);
        if press > 0.01 {
            let gap = a.base + 0.08 + 0.1 * (0.5 + 0.5 * (t * 5.0).cos());
            let mut x = -self.width;
            while x <= self.width {
                g.plot(a.cx + x, a.cy + gap * press + (1.0 - press) * 0.8, 3, INK_MAIN);
                g.plot(a.cx + x, a.cy - gap * press - (1.0 - press) * 0.8, 3, INK_MAIN);
                x += 0.03;
            }
        }
    }
}
