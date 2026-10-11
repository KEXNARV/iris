//! LIQUID: the context used, as a liquid filling the body from below, with a wave on the
//! surface. Past the threshold it turns amber. It's a layer inside the body.

use super::Fill;
use crate::core::color::INK_WARM;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Liquid {
    /// From how much context it turns amber.
    alert: f64,
    /// Wave height.
    swell: f64,
}

/// The liquid in this frame.
pub(crate) struct Level {
    pub(crate) ctx: f64,
    /// Surface height, without the wave.
    pub(crate) cap: f64,
    pub(crate) swell: f64,
    pub(crate) t: f64,
    pub(crate) ink: u8,
}

impl Liquid {
    pub fn new() -> Self {
        Liquid { alert: 0.8, swell: 0.02 }
    }

    pub fn level(&self, ctx: &Context, anchor: &Anchor) -> Level {
        let c = ctx.sig.ctx.clamp(0.0, 1.0);
        let ink = if c > self.alert { INK_WARM } else { anchor.ink };
        Level { ctx: c, cap: anchor.cy - anchor.radius + 2.0 * anchor.radius * c, swell: self.swell, t: ctx.t, ink }
    }
}

impl Fill for Level {
    fn dot(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool {
        if self.ctx > 0.01 && y < self.cap + self.swell * (x * 14.0 + self.t * 3.0).sin() {
            if (i + j) % 2 == 0 {
                g.dot(i, j, 2, self.ink);
            }
            return true;
        }
        false
    }
}
