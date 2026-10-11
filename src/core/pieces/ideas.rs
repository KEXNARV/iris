//! IDEAS (thinking): three comets orbiting the body. When switching to another state they sink into it.

use std::f64::consts::TAU;

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Ideas {
    count: usize,
    /// Dots of each comet (the first two, the head).
    tail: usize,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub orbit: f64,
    /// Undulation of the outline: the ideas orbit outside it.
    pub amp: f64,
    /// Body phase: sets the pace of the orbit.
    pub phase: f64,
    pub low: bool,
}

impl Ideas {
    pub fn new() -> Self {
        Ideas { count: 3, tail: 7 }
    }
}

impl Piece for Ideas {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, base) = (ctx.t, m.anchor.base);
        if m.orbit > 0.01 {
            for k in 0..self.count {
                let a0 = m.phase * (1.1 + k as f64 * 0.25) + k as f64 * TAU / self.count as f64;
                let rr = base + m.amp + 0.12 + k as f64 * 0.06 + 0.03 * (t * 1.7 + k as f64 * 2.0).sin();
                let r = base + (rr - base) * m.orbit;
                for s in 0..self.tail {
                    let a = a0 - s as f64 * 0.11;
                    let (x, y) = m.anchor.dot(r, a);
                    g.plot(x, y, if s < 2 { 3 } else { 2 }, INK_MAIN);
                    if s < 2 && !m.low {
                        for dr in [0.035, -0.035] {
                            let (x, y) = m.anchor.dot(r + dr, a);
                            g.plot(x, y, 3, INK_MAIN);
                        }
                    }
                }
            }
        }
    }
}
