//! VOICE (responding): waves coming out of the body while it speaks.

use std::f64::consts::TAU;

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Voice {
    waves: usize,
    /// How far they reach.
    reach: f64,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub ripple: f64,
}

impl Voice {
    pub fn new() -> Self {
        Voice { waves: 2, reach: 0.62 }
    }
}

impl Piece for Voice {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, base) = (ctx.t, m.anchor.base);
        if m.ripple > 0.01 {
            for w in 0..self.waves {
                let u = (t * 0.55 + w as f64 / self.waves as f64).fract();
                if u > 0.9 {
                    continue;
                }
                let r = base + 0.1 + u * (self.reach - base);
                let n = (40.0 + u * 30.0) as usize;
                for q in 0..n {
                    let (x, y) = m.anchor.dot(r, q as f64 * TAU / n as f64 + w as f64 * 0.2);
                    g.plot(x, y, if u < 0.4 { 2 } else { 1 }, INK_MAIN);
                }
            }
        }
    }
}
