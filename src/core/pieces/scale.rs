//! SCALE: the marks anchored to the frame, around everything. They act as a meter: the voice when
//! listening, progress when transcribing and testing, the radar's sweep, the music at rest,
//! and they light up green on an end of turn or passing tests.

use std::f64::consts::{PI, TAU};

use super::Piece;
use crate::core::color::{INK_ACCENT, INK_GREEN, INK_RED};
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;

pub(crate) struct Scale {
    marks: usize,
    /// Every how many marks a major one goes (longer, and the one that blinks when asking).
    major: usize,
    /// Radii of the marks; the third only on the major ones.
    radii: [f64; 3],
    /// Up to what fraction of the music meter it uses the accent; from there on, red
    /// (like the keyboard bar).
    gauge: f64,
}

/// What the scale needs to know from the buddy this frame.
pub(crate) struct Paint<'a> {
    pub anchor: &'a Anchor,
    /// How far into the boot it is (0..1): the marks appear in order.
    pub boot: f64,
    /// Voice level, 0..1.
    pub level: f64,
    /// Spin of what surrounds it, the same as the arcs'.
    pub spin: f64,
    /// Music: level and how much it is dancing.
    pub music: f64,
    pub groove: f64,
    /// When there was an end of turn: a green sweep.
    pub dones: &'a [f64],
    /// Flashes that light it up entirely: passing tests, copy, error.
    pub pass: f64,
    pub copy: f64,
    pub red: f64,
    pub low: bool,
    /// How much of each meter: voice, progress, radar, asking and tests.
    pub vu: f64,
    pub fill: f64,
    pub sweep: f64,
    pub cardinal: f64,
    pub tests: f64,
}

impl Scale {
    pub fn new() -> Self {
        Scale { marks: 24, major: 6, radii: [0.94, 0.90, 0.86], gauge: 0.5 }
    }
}

impl Piece for Scale {
    type Step<'a> = ();
    type Paint<'a> = Paint<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let (t, st) = (ctx.t, ctx.st());
        let n = self.marks as f64;
        for k in 0..self.marks {
            if k as f64 / n > m.boot * 1.05 {
                continue;
            }
            let a = PI / 2.0 - k as f64 * TAU / n;
            let (mut tone, mut ink) = (1, m.anchor.border);
            if m.vu > 0.01 && (k as f64 / n) < m.level * 1.15 * m.vu {
                tone = 3;
            }
            if m.fill > 0.01 && k as f64 / n <= (st * 0.35).fract() * m.fill {
                tone = 3;
            }
            if m.sweep > 0.01 {
                let d = (a + m.spin * 1.3).rem_euclid(TAU);
                if d < 0.7 * m.sweep {
                    tone = if d < 0.25 { 3 } else { 2 };
                }
            }
            if m.cardinal > 0.01 && k % self.major == 0 && (t * 4.0).sin() > 0.0 {
                tone = 3;
            }
            // Music: the keyboard bar's meter, filling from the bottom toward both sides, in the
            // accent and red when the beat goes past half.
            if m.groove > 0.01 {
                let half = n / 2.0;
                let from = (k as f64 - half).abs() / half;
                if from <= m.music * m.groove {
                    tone = 3;
                    ink = if from < self.gauge * 0.6 { INK_ACCENT } else { INK_RED };
                }
            }
            if m.tests > 0.5 && (k as f64) < (st * 9.0) % (n + 1.0) {
                tone = 3; // progress of the run; Pass or Error tell the result
            }
            for b in m.dones {
                let u = (t - b) / 1.1;
                if u > 0.45 && u < 1.0 && ((k as f64 / n - (u - 0.45) * 1.8).rem_euclid(1.0)) < 0.08 {
                    tone = 3;
                    ink = INK_GREEN;
                }
            }
            if m.pass > 0.3 {
                tone = 3;
                ink = INK_GREEN;
            }
            if m.copy > 0.5 {
                tone = 3;
            }
            if m.red > 0.35 {
                tone = tone.max(2);
            }
            g.polar(self.radii[0], a, tone, ink);
            g.polar(self.radii[1], a, tone, ink);
            if k % self.major == 0 && !m.low {
                g.polar(self.radii[2], a, tone, ink);
            }
        }
    }
}
