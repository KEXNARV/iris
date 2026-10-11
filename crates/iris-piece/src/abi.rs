//! The contract between Iris and a piece (ABI 1). Both sides use it: the piece when compiled to
//! WebAssembly and the Iris host when loading it, so they can't drift apart.
//!
//! Piece exports: `iris_abi() -> u32`, `iris_space() -> u32`, `iris_input() -> u32`
//! (where Iris writes the [`Input`]), `iris_new()`, `iris_step()`, `iris_event(u32)` and
//! `iris_paint() -> u64` (address << 32 | number of [`Op`]s).
//! Imports: only `iris.rand() -> f64`.

/// The contract version. A piece built for another one is not loaded.
pub const VERSION: u32 = 1;

/// Maximum number of draw ops per frame; the rest are ignored.
pub const MAX_OPS: usize = 20_000;

pub const MODE_PLOT: u8 = 0;
pub const MODE_UNDER: u8 = 1;

/// What Iris writes into the piece's memory before each call. All `f64`/`u32`, no pointers:
/// the same layout on the host (64 bits) and on wasm32.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub t: f64,
    pub dt: f64,
    pub since: f64,
    pub calm: f64,
    pub voice: f64,
    pub music: f64,
    pub context: f64,
    pub anchor_cx: f64,
    pub anchor_cy: f64,
    pub anchor_radius: f64,
    pub anchor_base: f64,
    /// The size of one dot in world units.
    pub du: f64,
    pub state: u32,
    pub anchor_ink: u32,
    pub anchor_border: u32,
    pub _pad: u32,
    /// The space's own parameters (see [`Space`]).
    pub p: [f64; 16],
}

impl Input {
    pub const ZERO: Input = Input {
        t: 0.0, dt: 0.0, since: 0.0, calm: 1.0, voice: 0.0, music: 0.0, context: 0.0,
        anchor_cx: 0.0, anchor_cy: 0.0, anchor_radius: 0.0, anchor_base: 0.0, du: 0.0,
        state: 0, anchor_ink: 0, anchor_border: 0, _pad: 0, p: [0.0; 16],
    };
}

/// A draw op: one dot in world coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Op {
    pub x: f64,
    pub y: f64,
    pub tone: u8,
    pub ink: u8,
    pub mode: u8,
    pub _pad: [u8; 5],
}

/// Which part of the core a piece goes into. Each space defines what's in `Input::p`.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    /// The specks around the body. `p`: 0 body radius, 1 voice level, 2 swallowing
    /// (from listening to transcribing), 3 dance, 4 how visible they are, 5 boot, 6 presses
    /// (compacting), 7 and 8 the gaze, 9 light mode (1 = yes).
    Particles = 1,
}

impl Space {
    pub fn from(n: u32) -> Option<Self> {
        match n {
            1 => Some(Space::Particles),
            _ => None,
        }
    }
}

/// The core's states, in Iris's order.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Booting, Sleeping, Idle, Typing, Listening, NoVoice, Transcribing, Thinking,
    Planning, Searching, Reading, Editing, Running, Testing, Git, Web, Delegating, Speaking,
    Asking, Compacting, Offline,
}

impl State {
    pub const ALL: [State; 21] = [
        State::Booting, State::Sleeping, State::Idle, State::Typing, State::Listening,
        State::NoVoice, State::Transcribing, State::Thinking, State::Planning, State::Searching,
        State::Reading, State::Editing, State::Running, State::Testing, State::Git, State::Web,
        State::Delegating, State::Speaking, State::Asking, State::Compacting, State::Offline,
    ];

    pub fn from(n: u32) -> Self {
        State::ALL.get(n as usize).copied().unwrap_or(State::Idle)
    }
}

/// Things that happen all at once, in Iris's order.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Key, Error, Done, Cancel, Nope, Merge, Copy, Pass,
}

impl Event {
    pub fn from(n: u32) -> Option<Self> {
        [Event::Key, Event::Error, Event::Done, Event::Cancel, Event::Nope, Event::Merge, Event::Copy, Event::Pass].get(n as usize).copied()
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "iris")]
unsafe extern "C" {
    #[link_name = "rand"]
    pub fn iris_rand() -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_does_not_change() {
        // If this changes, it's a new contract version.
        assert_eq!(core::mem::size_of::<Input>(), 12 * 8 + 4 * 4 + 16 * 8);
        assert_eq!(core::mem::size_of::<Op>(), 24);
    }
}
