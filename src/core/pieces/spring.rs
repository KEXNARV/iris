//! SPRING: the elastic squash. Everything that happens gives it a push; it bounces and
//! settles. The body stretches or squashes by whatever it reads.

use super::Piece;
use crate::core::context::Context;

pub(crate) struct Spring {
    /// How squashed it is: + wider than tall, − the other way around.
    pub sq: f64,
    v: f64,
    stiffness: f64,
    damping: f64,
    /// How much of the velocity turns into squash per second.
    step: f64,
    cap: f64,
}

impl Spring {
    pub fn new() -> Self {
        Spring { sq: 0.0, v: 0.0, stiffness: 140.0, damping: 9.0, step: 0.06, cap: 0.18 }
    }

    /// A push; with `/calm`, gentler.
    pub fn push(&mut self, v: f64, calm: f64) {
        self.v += v * calm;
    }
}

impl Piece for Spring {
    type Step<'a> = ();
    type Paint<'a> = ();

    fn step(&mut self, _ctx: &Context, _: (), dt: f64) {
        self.v += (-self.stiffness * self.sq - self.damping * self.v) * dt;
        self.sq = (self.sq + self.v * dt * self.step).clamp(-self.cap, self.cap);
    }
}
