//! MARKS: what is waiting, counted with marks. The queue (messages typed mid-turn, solid
//! satellites waiting to be read) and the TodoWrite tasks (one big mark per task; the
//! current one blinks).

use std::f64::consts::{PI, TAU};

use super::Piece;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Tail {
    /// How many messages are shown at most.
    max: usize,
    /// How far from the body's center they orbit.
    radius: f64,
}

impl Tail {
    pub fn new() -> Self {
        Tail { max: 6, radius: 0.6 }
    }
}

impl Piece for Tail {
    type Step<'a> = ();
    type Paint<'a> = &'a Anchor;

    fn paint(&self, g: &mut Grid, ctx: &Context, anchor: &Anchor) {
        let t = ctx.t;
        for k in 0..ctx.sig.queue.min(self.max) {
            let a = PI / 2.0 + t * 0.4 + k as f64 * 0.5;
            for (dr, da) in [(0.0, 0.0), (0.03, 0.0), (0.0, 0.06), (0.03, 0.06)] {
                let (x, y) = anchor.dot(self.radius + dr, a + da);
                g.plot(x, y, 3, INK_MAIN);
            }
        }
    }
}

pub(crate) struct Tasks {
    max: usize,
    radii: [f64; 2],
}

impl Tasks {
    pub fn new() -> Self {
        Tasks { max: 24, radii: [0.82, 0.78] }
    }
}

impl Piece for Tasks {
    type Step<'a> = ();
    /// How visible they are (planning).
    type Paint<'a> = f64;

    fn paint(&self, g: &mut Grid, ctx: &Context, plan: f64) {
        let t = ctx.t;
        let (n_todo, done) = ctx.sig.todos;
        if plan > 0.5 && n_todo > 0 {
            for i in 0..n_todo.min(self.max) {
                let a = PI / 2.0 - i as f64 * TAU / n_todo.min(self.max) as f64;
                let tone = if i < done {
                    3
                } else if i == done && (t * 8.0).sin() > 0.0 {
                    3
                } else {
                    1
                };
                g.polar(self.radii[0], a, tone, INK_MAIN);
                g.polar(self.radii[1], a, tone, INK_MAIN);
            }
        }
    }
}
