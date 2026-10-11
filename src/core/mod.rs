//! CORE: the top-right panel, where the buddy lives. It shows what Iris is doing through
//! shape, motion and a gaze that turns toward where things are happening.
//!
//! The design was made in `docs/blob-hibrido.html`; this is its terminal version. It's drawn
//! on its own braille grid (2×4 dots per cell) and not with ratatui's `Canvas`, because there
//! each cell takes the color of the last dot drawn: here it keeps the brightest dot's color,
//! which is what makes the layers readable.
//!
//! The core only orchestrates: it keeps the clock and the shown state (with its minimum, so it
//! doesn't flicker), passes the signals, events and subagents to the buddy, and outputs what
//! it draws in braille or Sixel. What is shown is up to each buddy: a recipe (`recipe`) of parts
//! wired through channels, most of them built on the pieces in `pieces`.

use ratatui::style::Color;

mod rand;
mod buddies;
mod color;
mod context;
mod state;
// The store's pieces, in WebAssembly. The web build compiles the core without this.
#[cfg(feature = "pieces")]
pub(crate) mod host;
mod pieces;
mod grid;
mod output;
pub(crate) mod recipe;

pub use state::{tool_state, Event, Signals, State};

use buddies::Buddy;
use color::rgb;
use context::{Notice, Context};
use grid::Grid;

pub struct Core {
    ctx: Context,
    fired: Vec<Event>,
    /// The buddies that can live in the core, all running at once: that way switching from one
    /// to another (`/buddy`, even in the preview) doesn't reset them. The first is the original;
    /// the second, Baymax. Kids only have the first.
    buddies: Vec<Box<dyn Buddy>>,
    /// A buddy from a recipe that isn't built in (the user's, the store's, a draft): when set,
    /// it's the one shown.
    custom: Option<Box<dyn Buddy>>,
}

/// Which parts of the buddy are drawn, one bit per part (their `layer` in the recipe);
/// `u32::MAX` is all. For the Iris web page, which shows it taken apart.
#[allow(dead_code)]
pub fn layers(mask: u32) {
    recipe::LAYERS.store(mask, std::sync::atomic::Ordering::Relaxed);
}

impl Core {
    pub fn new() -> Self {
        let build = |r: std::sync::Arc<recipe::Recipe>| -> Box<dyn Buddy> { Box::new(recipe::Assembled::new(&r).expect("a built-in recipe")) };
        Core::with(vec![build(recipe::original()), build(recipe::baymax())], 0.0, State::Booting)
    }

    fn with(buddies: Vec<Box<dyn Buddy>>, t: f64, state: State) -> Self {
        Core { ctx: Context { t, state, since: t, sig: Signals::default(), look: None }, fired: vec![], buddies, custom: None }
    }

    /// A kid: already awake in `state`, with no boot or specks.
    fn mini(state: State, seed: u64) -> Self {
        // Each kid with its own phase, so they don't all breathe and blink in unison.
        let kid = recipe::Assembled::kid(&recipe::original(), state, seed).expect("a built-in recipe");
        Core::with(vec![Box::new(kid)], 3.7 * seed as f64, state)
    }

    /// A core with only this recipe's buddy (Iris Studio, `iris piece try`).
    pub(crate) fn from_recipe(r: &recipe::Recipe) -> Result<Self, String> {
        Ok(Core::with(vec![Box::new(recipe::Assembled::new(r)?)], 0.0, State::Booting))
    }

    /// Shows this recipe's buddy instead of the built-in ones (`None` goes back to them). It's
    /// born already awake, in the current state.
    pub(crate) fn set_recipe(&mut self, r: Option<&recipe::Recipe>) -> Result<(), String> {
        self.custom = match r {
            Some(r) => Some(Box::new(recipe::Assembled::new(r)?)),
            None => None,
        };
        Ok(())
    }

    /// The shown buddy's channels and their values (Iris Studio's inspector).
    #[allow(dead_code)]
    pub(crate) fn channels(&self) -> Vec<(String, f64)> {
        self.active_buddy().channels(&self.ctx)
    }

    /// What broke in the recipe shown since the last time: (part, why).
    pub(crate) fn take_broken(&mut self) -> Vec<(String, String)> {
        self.buddies.iter_mut().chain(self.custom.as_mut()).flat_map(|b| b.take_broken()).collect()
    }

    /// The one shown: a recipe's if there is one; otherwise Baymax if its palette is set (and
    /// it's not a kid).
    fn active_buddy(&self) -> &dyn Buddy {
        if let Some(c) = &self.custom {
            return &**c;
        }
        let i = if self.buddies.len() > 1 && crate::theme::baymax() { 1 } else { 0 };
        &*self.buddies[i]
    }

    fn notify(&mut self, notice: Notice) {
        for b in self.buddies.iter_mut().chain(self.custom.as_mut()) {
            b.notice(notice, &self.ctx);
        }
    }

    /// The shown state, which may lag a bit behind the requested one (see `State::dwell`).
    pub fn state(&self) -> State {
        self.ctx.state
    }

    /// The current color, with transitions and the red of errors.
    pub fn color(&self) -> Color {
        rgb(self.active_buddy().color())
    }

    /// Restarts the boot animation (when the engine is relaunched).
    pub fn reboot(&mut self) {
        self.kids_clear();
        self.switch(State::Booting);
    }

    /// A subagent is born: it sprouts from the body toward its spot in the orbit.
    pub fn kid_born(&mut self, id: u64) {
        self.notify(Notice::Born(id));
    }

    /// The kid did something (requested a tool or got its result): a particle rises.
    pub fn kid_pulse(&mut self, id: u64, error: bool) {
        self.notify(Notice::Pulse(id, error));
    }

    /// The subagent finished: if it went well it returns to the body; if not, it fades out in red.
    pub fn kid_end(&mut self, id: u64, ok: bool) {
        self.notify(Notice::Finish(id, ok));
    }

    /// The engine is gone: the kids go with it, without ceremony.
    pub fn kids_clear(&mut self) {
        self.notify(Notice::Gone);
    }

    /// How many kids there are (alive or saying goodbye).
    #[cfg(test)]
    pub fn kid_count(&self) -> usize {
        self.active_buddy().kids()
    }

    /// The events fired since last time, to notify the keyboard.
    pub fn take_fired(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.fired)
    }

    pub fn fire(&mut self, ev: Event) {
        self.fired.push(ev);
        self.notify(Notice::Event(ev));
    }

    fn switch(&mut self, to: State) {
        let from = self.ctx.state;
        self.ctx.state = to;
        self.ctx.since = self.ctx.t;
        for b in self.buddies.iter_mut().chain(self.custom.as_mut()) {
            b.change(from, to, &self.ctx);
        }
    }

    /// Advances the animation `dt` seconds toward the `want` state.
    pub fn step(&mut self, dt: f64, want: State, sig: &Signals) {
        let dt = dt.clamp(0.0, 0.1);
        self.ctx.t += dt;
        self.ctx.sig = sig.clone();
        let shown = self.ctx.state;
        let shown_for = self.ctx.t - self.ctx.since;
        if want != shown && (want.urgent() || shown.urgent() || shown_for >= shown.dwell()) {
            self.switch(want);
        }
        for b in self.buddies.iter_mut().chain(self.custom.as_mut()) {
            b.step(&self.ctx, dt);
        }
    }

    /// If set, the gaze goes there (−1..1) instead of wherever the state says.
    fn look_at(&mut self, look: Option<[f64; 2]>) {
        self.ctx.look = look;
    }

    /// How much error it's carrying (0..1).
    fn alarm(&self) -> f64 {
        self.active_buddy().alarm()
    }

    /// Its color before converting for the terminal: for a kid's ink.
    fn ink(&self) -> [f64; 3] {
        self.active_buddy().color()
    }

    fn paint(&self, g: &mut Grid) {
        self.active_buddy().paint(g, &self.ctx);
    }

    fn palette(&self) -> Vec<[[f64; 3]; 4]> {
        self.active_buddy().palette()
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
