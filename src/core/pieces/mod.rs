//! PIECES: what a buddy can be composed of. Each has its own state, its configuration (as
//! instance fields: two halos can be two different instances), its animation, its reactions
//! and its drawing. They don't know which buddy uses them: what they need from it
//! (intensities, where the body is) the buddy hands them every frame.

use super::rand::Rand;
use super::context::Context;
use super::state::Event;
use super::grid::Grid;

pub(crate) mod arcs;
pub(crate) mod sprout;
pub(crate) mod body;
pub(crate) mod scale;
pub(crate) mod gestures;
pub(crate) mod git;
pub(crate) mod kids;
pub(crate) mod ideas;
pub(crate) mod reading;
pub(crate) mod liquid;
pub(crate) mod magnifier;
pub(crate) mod marks;
pub(crate) mod gaze;
pub(crate) mod eye;
pub(crate) mod waves;
pub(crate) mod packages;
pub(crate) mod particles;
pub(crate) mod presses;
pub(crate) mod question;
pub(crate) mod stitches;
pub(crate) mod radar;
pub(crate) mod spring;
pub(crate) mod voice;
pub(crate) mod zees;

/// A piece. Everything is optional: some pieces only draw, and others only move
/// (the gaze, the spring).
pub(crate) trait Piece {
    /// What the buddy passes it to advance one frame (`()` if it doesn't need it).
    type Step<'a>;
    /// What the buddy passes it to draw itself (`()` if it doesn't need it).
    type Paint<'a>;

    fn step(&mut self, _ctx: &Context, _step: Self::Step<'_>, _dt: f64) {}

    /// An Iris event. The RNG is the buddy's: if the piece uses it, order matters.
    fn event(&mut self, _ev: Event, _ctx: &Context, _rand: &mut Rand) {}

    fn paint(&self, _g: &mut Grid, _ctx: &Context, _m: Self::Paint<'_>) {}
}

/// An inner layer of a body: the body walks its interior dots and asks each layer, in order,
/// whether the dot is theirs. The first one that says yes paints it (or leaves it empty) and
/// the ones below never see it.
pub(crate) trait Fill {
    fn dot(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool;
}

/// Overshoots a little and comes back: for things being born.
pub(crate) fn ease_out_back(u: f64) -> f64 {
    1.0 + 2.7 * (u - 1.0).powi(3) + 1.7 * (u - 1.0).powi(2)
}
