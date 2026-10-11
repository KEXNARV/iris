//! GAZE: where the buddy looks. It jumps quickly toward its target, like a saccade: when
//! idle it looks around at random; in each state, toward where things happen (the input, the
//! clock, the tasks, the child talking to it). With a whisper hallucination, it shakes its head.

use std::f64::consts::{PI, TAU};

use super::gestures::Gesture;
use super::Piece;
use crate::core::rand::Rand;
use crate::core::context::Context;
use crate::core::state::State;

pub(crate) struct Gaze {
    /// Where it looks now, in world units from the body's center.
    pub gaze: [f64; 2],
    /// The target of the last idle saccade, and when the next one is due.
    gaze_t: [f64; 2],
    next_saccade: f64,
    /// How far it can stray, in body radii.
    reach: f64,
}

pub(crate) struct Step<'a> {
    pub rand: &'a mut Rand,
    /// Body radius: the gaze doesn't leave it.
    pub base: f64,
    pub gesture: Option<Gesture>,
    /// A child sent it something: (when, angle).
    pub heard: Option<(f64, f64)>,
    /// Body phase (delegating looks where the sprout goes).
    pub phase: f64,
    /// Shaking its head (0..1).
    pub nope: f64,
}

impl Gaze {
    pub fn new() -> Self {
        Gaze { gaze: [0.0; 2], gaze_t: [0.0; 2], next_saccade: 0.0, reach: 0.42 }
    }

    /// How far it reaches from the center.
    pub fn lim(&self, base: f64) -> f64 {
        base * self.reach
    }

    fn target(&mut self, ctx: &Context, m: &mut Step) -> [f64; 2] {
        use State::*;
        let (t, st) = (ctx.t, ctx.st());
        let lim = self.lim(m.base);
        if let Some([x, y]) = ctx.look {
            return [lim * x, lim * y];
        }
        if let Some(g) = m.gesture {
            match g {
                // The clock is at the top right of the screen; the chat, on the left.
                Gesture::LookClock => return [lim * 0.8, lim * 0.85],
                Gesture::LookChat => return [-lim * 0.9, 0.0],
                _ => {}
            }
        }
        // A child sent it something: it looks at it for a moment, unless it's busy with you.
        if let Some((t0, a)) = m.heard {
            if t - t0 < 0.7 && matches!(ctx.state, Idle | Thinking | Delegating | Speaking | Reading | Searching) {
                return [lim * 0.9 * a.cos(), lim * 0.9 * a.sin()];
            }
        }
        match ctx.state {
            Idle => {
                if t > self.next_saccade {
                    let a = m.rand.rand() * TAU;
                    let r = m.rand.rand() * lim;
                    self.gaze_t = [r * a.cos(), r * a.sin() * 0.8];
                    self.next_saccade = t + 1.2 + m.rand.rand() * 2.5;
                }
                self.gaze_t
            }
            Typing => [-lim * 0.6, -lim * 0.8],
            Listening => [0.0, -lim * 0.7],
            NoVoice => [lim * 0.9 * (t * 1.4).sin().signum(), -lim * 0.2],
            Transcribing => [0.02 * (t * 30.0).sin(), 0.02 * (t * 27.0).cos()],
            Thinking => [lim * 0.5 * (t * 0.7).sin(), lim * 0.75],
            Reading => {
                let line = st * 0.9;
                [-lim * 0.8 + line.fract() * lim * 1.6, lim * 0.6 - (line.floor() % 4.0) * lim * 0.4]
            }
            Searching => [m.base * 0.55 * (st * 1.3).sin(), m.base * 0.45 * (st * 2.1 + 1.0).sin()],
            Planning => {
                let (n, d) = ctx.sig.todos;
                let a = if n > 0 { PI / 2.0 - d.min(n - 1) as f64 * TAU / n as f64 } else { PI / 2.0 };
                [lim * 0.9 * a.cos(), lim * 0.9 * a.sin()]
            }
            Editing => [lim * 0.6 * (t * 2.6).cos(), lim * 0.6 * (t * 2.6).sin()],
            Running => [0.0, -lim * 0.75],
            Testing => {
                let a = PI / 2.0 - (st * 9.0 % 24.0) * TAU / 24.0;
                [lim * 0.8 * a.cos(), lim * 0.8 * a.sin()]
            }
            Git => [lim * 0.3, lim * 0.9],
            Web => [lim * 0.7 * (t * 0.8).cos(), lim * 0.7 * (t * 0.8).sin()],
            Delegating => {
                let a = m.phase * 0.9;
                [lim * 0.8 * a.cos(), lim * 0.8 * a.sin()]
            }
            Speaking => [-lim * 0.8, 0.0],
            Asking => [-lim * 0.4, -lim * 0.8],
            _ => [0.0, 0.0],
        }
    }
}

impl Piece for Gaze {
    type Step<'a> = Step<'a>;
    type Paint<'a> = ();

    fn step(&mut self, ctx: &Context, mut m: Step, dt: f64) {
        let t = ctx.t;
        let gt = self.target(ctx, &mut m);
        let gt = if m.nope > 0.01 { [m.base * 0.4 * (t * 22.0).sin() * (m.nope * 2.0).min(1.0), gt[1]] } else { gt };
        let kg = (dt * 14.0).min(1.0);
        for i in 0..2 {
            self.gaze[i] += (gt[i] - self.gaze[i]) * kg;
        }
    }
}

/// Blinks every so often when idle and typing, and with a flash (copy, waking up).
pub(crate) fn blink(ctx: &Context, copy: f64) -> bool {
    (matches!(ctx.state, State::Idle | State::Typing) && (ctx.t % 4.3) < 0.12) || copy > 0.3
}
