//! RADAR (running): a sweep that spins from the body out to the arcs.

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Radar {
    /// Rays of the trail and how far apart they are.
    rays: usize,
    spacing: f64,
    /// How far it reaches.
    reach: f64,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub sweep: f64,
    pub spin: f64,
}

impl Radar {
    pub fn new() -> Self {
        Radar { rays: 10, spacing: 0.06, reach: 0.78 }
    }
}

impl Piece for Radar {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, _ctx: &Context, m: Paint) {
        if m.sweep <= 0.01 {
            return;
        }
        let a0 = -m.spin * 1.3;
        for s in 0..self.rays {
            let a = a0 + s as f64 * self.spacing;
            let mut r = m.anchor.base + 0.14;
            while r < self.reach {
                g.polar(r, a, if s < 2 { 3 } else if s < 5 { 2 } else { 1 }, INK_MAIN);
                r += 0.035;
            }
        }
    }
}
