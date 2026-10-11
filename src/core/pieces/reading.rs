//! READING (reading): a line that moves up and down the body, like a line of text being read.
//! It is an inner layer: the body draws it in its own dots.

use super::Fill;
use crate::core::color::INK_MAIN;
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct ReadingBar {
    /// How far it reaches, in body radii, and its thickness.
    sweep: f64,
    thickness: f64,
}

/// The line in this frame.
pub(crate) struct ReadingLine {
    y: f64,
    thickness: f64,
    active: bool,
}

impl ReadingBar {
    pub fn new() -> Self {
        ReadingBar { sweep: 0.8, thickness: 0.025 }
    }

    pub fn line(&self, ctx: &Context, anchor: &Anchor, scan: f64) -> ReadingLine {
        ReadingLine { y: anchor.cy + anchor.base * self.sweep * (ctx.st() * 1.6).cos(), thickness: self.thickness, active: scan > 0.01 }
    }
}

impl ReadingLine {
    /// Does the point at height `y` fall on the line?
    pub fn covers(&self, y: f64) -> bool {
        self.active && (y - self.y).abs() < self.thickness
    }
}

impl Fill for ReadingLine {
    fn dot(&self, g: &mut Grid, i: usize, j: usize, _x: f64, y: f64) -> bool {
        if self.covers(y) {
            g.dot(i, j, 3, INK_MAIN);
            return true;
        }
        false
    }
}
