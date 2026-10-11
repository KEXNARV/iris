//! GESTURES: what it does after a while standing still, so it doesn't always repeat the same
//! thing. It stretches, looks at the clock, looks at the chat or, if a long time passed, yawns.

use std::f64::consts::PI;

use super::spring::Spring;
use super::Piece;
use crate::core::rand::Rand;
use crate::core::context::Context;
use crate::core::state::State;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Gesture {
    Stretch,
    LookClock,
    LookChat,
    Yawn,
}

impl Gesture {
    /// How long it lasts.
    fn len(self) -> f64 {
        match self {
            Gesture::Stretch => 0.8,
            Gesture::LookClock => 1.6,
            Gesture::LookChat => 1.2,
            Gesture::Yawn => 2.2,
        }
    }
}

pub(crate) struct Gestures {
    /// The one it is doing and when it started.
    current: Option<(Gesture, f64)>,
    next_at: f64,
    /// Seconds without activity for the first one, and for yawning.
    still: f64,
    sleep_after: f64,
}

pub(crate) struct Step<'a> {
    pub rand: &'a mut Rand,
    /// Stretching is a push.
    pub spring: &'a mut Spring,
    /// No gestures while dancing.
    pub groove: f64,
}

impl Gestures {
    pub fn new() -> Self {
        Gestures { current: None, next_at: 20.0, still: 15.0, sleep_after: 95.0 }
    }

    pub fn current(&self) -> Option<Gesture> {
        self.current.map(|(g, _)| g)
    }

    /// Any state change cuts it short.
    pub fn cut(&mut self) {
        self.current = None;
    }

    /// How much it is yawning (0..1).
    pub fn yawn(&self, t: f64) -> f64 {
        match self.current {
            Some((Gesture::Yawn, t0)) => (PI * ((t - t0) / 2.2).min(1.0)).sin(),
            _ => 0.0,
        }
    }
}

impl Piece for Gestures {
    type Step<'a> = Step<'a>;
    type Paint<'a> = ();

    fn step(&mut self, ctx: &Context, m: Step, _dt: f64) {
        let (t, idle) = (ctx.t, ctx.sig.idle);
        if ctx.state == State::Idle && m.groove < 0.05 && idle > self.still && t > self.next_at && self.current.is_none() {
            let g = if idle > self.sleep_after {
                Gesture::Yawn
            } else {
                [Gesture::Stretch, Gesture::LookClock, Gesture::LookChat][(m.rand.rand() * 3.0) as usize % 3]
            };
            if g == Gesture::Stretch {
                m.spring.push(3.5, ctx.calm());
            }
            self.current = Some((g, t));
            self.next_at = t + 8.0 + m.rand.rand() * 8.0;
        }
        if let Some((g, t0)) = self.current {
            if t - t0 > g.len() || ctx.state != State::Idle {
                self.current = None;
            }
        }
    }
}
