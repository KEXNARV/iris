//! MAGNIFIER (searching): a ring with a handle that roams over the body. Outside it's just
//! another drawing; inside, a body layer (the glass, hatched).

use std::f64::consts::TAU;

use super::{Piece, Fill};
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Magnifier {
    radius: f64,
    /// Dots in the ring and in the handle.
    ring: usize,
    handle: usize,
}

/// Where the magnifier is: center and radius.
#[derive(Clone, Copy)]
pub(crate) struct Spot {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    pub lens: f64,
    pub low: bool,
}

impl Magnifier {
    pub fn new() -> Self {
        Magnifier { radius: 0.15, ring: 40, handle: 5 }
    }

    /// Where it goes: a slow Lissajous over the body.
    pub fn spot(&self, ctx: &Context, anchor: &Anchor) -> Spot {
        let (st, a) = (ctx.st(), anchor);
        Spot { x: a.cx + a.base * 0.55 * (st * 1.3).sin(), y: a.cy + a.base * 0.45 * (st * 2.1 + 1.0).sin(), r: self.radius }
    }
}

impl Piece for Magnifier {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let Spot { x: lx, y: ly, r: lr } = self.spot(ctx, m.anchor);
        if m.lens > 0.01 {
            for q in 0..self.ring {
                let a = q as f64 * TAU / self.ring as f64;
                g.plot(lx + lr * a.cos(), ly + lr * a.sin(), 3, INK_MAIN);
            }
            if !m.low {
                for q in 0..self.handle {
                    let d = (lr + q as f64 * 0.025) * 0.7;
                    g.plot(lx + d, ly - d, 3, INK_MAIN);
                }
            }
        }
    }
}

/// The glass: the inside of the magnifier, hatched, covering what's underneath.
pub(crate) struct Glass {
    pub spot: Spot,
    pub active: bool,
}

impl Fill for Glass {
    fn dot(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool {
        if self.active && (x - self.spot.x).hypot(y - self.spot.y) < self.spot.r {
            if (i + j) % 2 == 0 {
                g.dot(i, j, 3, INK_MAIN);
            }
            return true;
        }
        false
    }
}
