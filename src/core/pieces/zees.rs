//! ZEES (idle): three z's rising from the sleeping body.

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Zees {
    count: usize,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub zzz: f64,
    pub low: bool,
}

impl Zees {
    pub fn new() -> Self {
        Zees { count: 3 }
    }
}

impl Piece for Zees {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, a) = (ctx.t, m.anchor);
        if m.zzz > 0.01 && !m.low {
            for k in 0..self.count {
                let u = (t * 0.22 + k as f64 / self.count as f64).fract();
                let s = 0.03 + u * 0.04;
                let (zx, zy) = (a.cx + a.base * 0.6 + u * 0.32, a.cy + a.base * 0.5 + u * 0.38);
                let tone = if u < 0.5 { 2 } else { 1 };
                for q in -1..=1 {
                    g.plot(zx + q as f64 * s, zy + s, tone, INK_MAIN);
                    g.plot(zx + q as f64 * s, zy - s, tone, INK_MAIN);
                }
                g.plot(zx, zy, tone, INK_MAIN);
            }
        }
    }
}
