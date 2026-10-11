//! ARCS: a crown of arcs spinning around the body. The original buddy has two: three long
//! ones outside that thin out when dimming and follow the boot, and six short ones inside,
//! spinning the other way, that get shorter.

use std::f64::consts::TAU;

use super::Piece;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Arcs {
    count: usize,
    radius: f64,
    /// Dots per arc, fully lit.
    dots: f64,
    /// How long each arc is, in radians.
    segment: f64,
    /// How much it spins relative to the buddy's spin (negative: the other way).
    spin: f64,
    tone: u8,
    /// Long: follow the length the state asks for and the boot, and thin out when dimming.
    /// Otherwise they get shorter when dimming.
    long: bool,
}

pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    /// Spin of the surroundings.
    pub spin: f64,
    /// How visible they are (0..1) and what length they ask for.
    pub arcs: f64,
    pub arc_len: f64,
    pub boot: f64,
}

impl Arcs {
    /// The original's outer ones.
    pub fn outer() -> Self {
        Arcs { count: 3, radius: 0.80, dots: 60.0, segment: TAU / 5.0, spin: 1.0, tone: 2, long: true }
    }

    /// The original's inner ones.
    pub fn inner() -> Self {
        Arcs { count: 6, radius: 0.70, dots: 14.0, segment: TAU / 16.0, spin: -1.6, tone: 1, long: false }
    }
}

impl Piece for Arcs {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, _ctx: &Context, m: Paint) {
        if m.arcs * m.boot <= 0.05 {
            return;
        }
        let n = if self.long {
            (self.dots * m.arc_len * m.arcs * (m.boot * 1.4 - 0.4).max(0.0)).round() as usize
        } else {
            (self.dots * m.arcs).round() as usize
        };
        for k in 0..self.count {
            for q in 0..n {
                let a = if self.long {
                    m.spin * self.spin + k as f64 * TAU / self.count as f64 + self.segment * m.arc_len * q as f64 / n.max(1) as f64
                } else {
                    m.spin * self.spin + k as f64 * TAU / self.count as f64 + self.segment * q as f64 / self.dots
                };
                g.polar(self.radius, a, self.tone, m.anchor.border);
            }
        }
    }
}
