//! BUDDIES: who lives in the core. Each buddy composes its own pieces, with its own script,
//! and decides the order they are drawn in; the core just tells it what happens and asks it to
//! paint itself.

use super::context::{Notice, Context};
use super::state::State;
use super::grid::Grid;

pub(crate) mod baymax;
pub(crate) mod original;

pub(crate) trait Buddy {
    /// Advances `dt` seconds. The core has already moved the clock and the state.
    fn step(&mut self, ctx: &Context, dt: f64);

    /// The visible state changed (the core has already recorded it in `ctx`).
    fn change(&mut self, from: State, to: State, ctx: &Context);

    /// An event or something from the subagents.
    fn notice(&mut self, notice: Notice, ctx: &Context);

    fn paint(&self, g: &mut Grid, ctx: &Context);

    /// The colors of its inks, by tone (1 dim … 3 full). The base ones (state, green, red,
    /// amber, accent) come first and in that order.
    fn palette(&self) -> Vec<[[f64; 3]; 4]>;

    /// Its color right now, with the transitions and the red of errors.
    fn color(&self) -> [f64; 3];

    /// How much error it carries (0..1): the main one tints a kid's line red with this.
    fn alarm(&self) -> f64;

    /// How many subagents it has in view.
    #[cfg(test)]
    fn kids(&self) -> usize;

    /// So tests can look inside.
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any;
}
