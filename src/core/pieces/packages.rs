//! PACKETS (on the network): they leave the body for the ring and come back; where they bounce
//! the scale lights up.

use std::f64::consts::TAU;

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Packages {
    count: usize,
    /// How far they reach, and the scale radii they light up when bouncing.
    reach: f64,
    bounce: [f64; 2],
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub packets: f64,
    pub low: bool,
}

impl Packages {
    pub fn new() -> Self {
        Packages { count: 4, reach: 0.84, bounce: [0.94, 0.90] }
    }
}

impl Piece for Packages {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, base) = (ctx.t, m.anchor.base);
        if m.packets > 0.01 {
            for k in 0..self.count {
                let trip = t * 0.8 + k as f64 / self.count as f64;
                let (n, u) = (trip.floor(), trip.fract());
                let h = ((n * self.count as f64 + k as f64) * 12.9898).sin() * 43758.5453;
                let a = h.fract().abs() * TAU;
                let out = if u < 0.5 { u * 2.0 } else { 2.0 - u * 2.0 };
                let r = base + 0.06 + out * (self.reach - base) * m.packets;
                for s in 0..if m.low { 2 } else { 4 } {
                    let rs = r - if u < 0.5 { 1.0 } else { -1.0 } * s as f64 * 0.04;
                    if rs > base {
                        let (x, y) = m.anchor.dot(rs, a);
                        g.plot(x, y, if s == 0 { 3 } else if s < 2 { 2 } else { 1 }, INK_MAIN);
                    }
                }
                if (u - 0.5).abs() < 0.08 {
                    g.polar(self.bounce[0], a, 3, INK_MAIN);
                    g.polar(self.bounce[1], a, 3, INK_MAIN);
                }
            }
        }
    }
}
