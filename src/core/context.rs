//! What Iris gives any buddy, without knowing anything about how it is drawn: the time, the
//! visible state and since when, the app's signals and, if someone asks for it, where to
//! look. Plus the one-off notices (events and subagents) and the anchor, which each buddy
//! publishes on every frame so its pieces can position themselves around it.

use super::state::{Event, Signals, State};

pub(crate) struct Context {
    /// Seconds since the core was born.
    pub t: f64,
    /// The visible state, which can lag a bit behind the requested one (see `State::dwell`).
    pub state: State,
    /// When it started being shown.
    pub since: f64,
    pub sig: Signals,
    /// If set, the gaze goes there (−1..1) instead of whatever the state says: the
    /// child looks at the main one when it sends it something.
    pub look: Option<[f64; 2]>,
}

impl Context {
    /// Seconds in the visible state.
    pub fn st(&self) -> f64 {
        self.t - self.since
    }

    /// Less motion (`/calm`).
    pub fn calm(&self) -> f64 {
        if self.sig.calm { 0.4 } else { 1.0 }
    }
}

/// What happens once and the buddy may want to respond to.
#[derive(Clone, Copy)]
pub(crate) enum Notice {
    Event(Event),
    /// A subagent is born.
    Born(u64),
    /// The subagent did something; `true` if it was an error.
    Pulse(u64, bool),
    /// The subagent finished; `true` if it went well.
    Finish(u64, bool),
    /// The engine went away and the subagents with it.
    Gone,
}

/// Where a buddy's body is in this frame and with which inks, so what surrounds it can
/// position itself without knowing which body it is.
#[derive(Clone, Copy)]
pub(crate) struct Anchor {
    pub cx: f64,
    pub cy: f64,
    /// The radius it has right now (with boot-up, deflation and music).
    pub radius: f64,
    /// The radius the state asks for, without what moves it.
    pub base: f64,
    /// The body's ink: the state's, or red on an error.
    pub ink: u8,
    /// The ink of what surrounds it: the accent, or red on an error.
    pub border: u8,
}

impl Anchor {
    /// The point at `r` from the body's center, at angle `a`.
    pub fn dot(&self, r: f64, a: f64) -> (f64, f64) {
        (self.cx + r * a.cos(), self.cy + r * a.sin())
    }
}
