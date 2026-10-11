//! EYE: the body's pupil. It dilates according to the state, contracts sharply on an error,
//! narrows when yawning and when blinking only a line remains. It looks wherever the gaze says.
//! It is an inner layer of the body, the topmost one.

use super::Fill;
use crate::core::grid::Grid;

pub(crate) struct Eye {
    radius: f64,
    /// Never goes below this many dots of radius (it got lost in Cine's big panel).
    min_dots: f64,
    /// How much it contracts on an error and when yawning; how much it grows under the magnifier.
    error: f64,
    yawn: f64,
    magnifier: f64,
}

/// What the eye needs from the buddy this frame.
pub(crate) struct Controls {
    /// Center of the pupil (the body's plus the gaze).
    pub x: f64,
    pub y: f64,
    /// How visible it is (0 no pupil) and how open it is (>1 dilated).
    pub pupil: f64,
    pub dilate: f64,
    pub red: f64,
    pub yawn: f64,
    /// It grows under the magnifier.
    pub lens: f64,
    pub blink: bool,
    pub ink: u8,
    /// The size of one dot.
    pub du: f64,
}

/// The pupil this frame.
pub(crate) struct Pupil {
    x: f64,
    y: f64,
    r: f64,
    blink: bool,
    ink: u8,
}

impl Eye {
    pub fn new() -> Self {
        Eye { radius: 0.085, min_dots: 2.5, error: 0.45, yawn: 0.8, magnifier: 1.5 }
    }

    pub fn pupil(&self, m: Controls) -> Pupil {
        let pr = if m.pupil > 0.01 {
            (self.radius * m.dilate * (1.0 - m.red * self.error)).max(self.min_dots * m.du) * m.pupil * (1.0 - m.yawn * self.yawn)
        } else {
            0.0
        };
        let r = pr * if m.lens > 0.5 { self.magnifier } else { 1.0 };
        Pupil { x: m.x, y: m.y, r, blink: m.blink, ink: m.ink }
    }
}

impl Fill for Pupil {
    fn dot(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool {
        let (ex, ey) = (x - self.x, y - self.y);
        let er = ex.hypot(ey);
        if self.r > 0.01 && er < self.r {
            // When blinking only a line remains.
            if !self.blink || ey.abs() < 0.02 {
                g.dot(i, j, 3, self.ink);
            }
            return true;
        }
        false
    }
}
