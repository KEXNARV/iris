//! PART: what every piece of a recipe is to the buddy that runs it. A part reads and writes
//! channels on the bus, reacts to what happens and draws; it never touches another part.
//! When one has to tell the others something that isn't a number (a subagent came back and
//! there has to be a green ring), it sends a message, delivered right after its call to every
//! other part, in recipe order.

use super::bus::Bus;
use crate::core::context::{Context, Notice};
use crate::core::grid::Grid;
use crate::core::rand::Rand;
use crate::core::state::State;

/// A message between parts.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Msg {
    /// A green ring around the body, at that moment (a subagent came back).
    Ring(f64),
}

/// What a part gets while the buddy moves.
pub(crate) struct World<'a> {
    pub ctx: &'a Context,
    pub bus: &'a mut Bus,
    /// The buddy's RNG, shared: whoever uses it, in recipe order.
    pub rand: &'a mut Rand,
    pub out: &'a mut Vec<Msg>,
}

/// What a part gets while the buddy is drawn: a copy of the bus for this frame, where parts
/// publish what depends on the grid's size or on where the body is now.
pub(crate) struct Frame<'a> {
    pub ctx: &'a Context,
    pub bus: Bus,
}

pub(crate) trait Part {
    /// Advances `dt` seconds.
    fn step(&mut self, _w: &mut World, _dt: f64) {}

    /// The visible state changed.
    fn change(&mut self, _w: &mut World, _from: State, _to: State) {}

    /// An event or something from the subagents.
    fn notice(&mut self, _w: &mut World, _n: Notice) {}

    fn message(&mut self, _w: &mut World, _m: Msg) {}

    /// Before drawing, once per grid and in recipe order.
    fn layout(&self, _f: &mut Frame, _g: &Grid) {}

    fn paint(&self, _g: &mut Grid, _f: &Frame) {}

    /// Why it stopped working, if it did (parts from the store can; built-in ones don't).
    fn broken(&self) -> Option<String> {
        None
    }

    /// The inks it adds to the palette, after the five base ones, in drawing order.
    fn inks(&self, _col: [f64; 3]) -> Vec<[[f64; 3]; 4]> {
        vec![]
    }

    /// How many subagents it has in view.
    #[cfg(test)]
    fn kids(&self) -> usize {
        0
    }

    /// How many green rings it's showing.
    #[cfg(test)]
    fn rings(&self) -> usize {
        0
    }
}
