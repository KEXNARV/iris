//! For writing Iris pieces: what gets drawn in the core, loaded as a WebAssembly plugin. A
//! piece implements [`Piece`] and is exported with [`export!`]:
//!
//! ```ignore
//! use iris_piece::*;
//!
//! struct Antenna { spark: f64 }
//!
//! impl Piece for Antenna {
//!     const SPACE: Space = Space::Particles;
//!     fn new(_rand: &mut Rand) -> Self { Antenna { spark: 0.0 } }
//!     fn step(&mut self, _ctx: &Context, dt: f64) { self.spark = (self.spark - dt * 2.0).max(0.0); }
//!     fn event(&mut self, ev: Event, _ctx: &Context) { if ev == Event::Error { self.spark = 1.0; } }
//!     fn draw(&self, g: &mut Grid, ctx: &Context) { g.plot(0.0, -ctx.anchor.radius * 1.4, 3, INK_ACCENT); }
//! }
//!
//! export!(Antenna);
//! ```
//!
//! Built with `cargo build --release --target wasm32-unknown-unknown` (crate-type `cdylib`).
//!
//! The contract with Iris ([`abi`]) is small on purpose: Iris writes the [`abi::Input`] into the
//! piece's memory before each call and reads back the list of drawing [`abi::Op`]s. The piece
//! has no files, network or clock: only what Iris hands it.

pub mod abi;

pub use abi::{Space, State, Event};

/// Inks of the buddy's palette (the same ones the built-in pieces use).
pub const INK_MAIN: u8 = 0;
pub const INK_GREEN: u8 = 1;
pub const INK_RED: u8 = 2;
pub const INK_WARM: u8 = 3;
pub const INK_ACCENT: u8 = 4;

/// Where the buddy's body is in this frame, to position things around it.
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub cx: f64,
    pub cy: f64,
    /// The radius it has right now (with boot-up, deflation and music).
    pub radius: f64,
    /// The radius the state asks for, without what moves it.
    pub base: f64,
    pub ink: u8,
    pub border: u8,
}

/// What Iris tells the piece on each frame.
#[derive(Clone, Debug)]
pub struct Context {
    /// Seconds since the core was born.
    pub t: f64,
    /// The visible state and since when.
    pub state: State,
    pub since: f64,
    /// 1 normally, less with `/calm`.
    pub calm: f64,
    /// Voice and music level (0..1) and Claude context used (0..1).
    pub voice: f64,
    pub music: f64,
    pub context: f64,
    pub anchor: Anchor,
    /// The space's own parameters (see [`Space`]).
    pub p: [f64; 16],
}

impl Context {
    /// Seconds in the visible state.
    pub fn st(&self) -> f64 {
        self.t - self.since
    }
}

/// What the piece draws in this frame. Coordinates are those of the core's world: the
/// center at (0, 0) and the edge at 1.
pub struct Grid {
    pub(crate) actions: Vec<abi::Op>,
    /// The size of a dot, in world units.
    pub du: f64,
}

impl Grid {
    /// A dot with its tone (1 dim … 3 full) and its ink. On the grid the brightest one wins.
    pub fn plot(&mut self, x: f64, y: f64, tone: u8, ink: u8) {
        self.push(x, y, tone, ink, abi::MODE_PLOT);
    }

    /// Only where there is nothing: for what goes behind everything.
    pub fn plot_under(&mut self, x: f64, y: f64, tone: u8) {
        self.push(x, y, tone, 0, abi::MODE_UNDER);
    }

    /// A dot at `r` from the center, at angle `a`.
    pub fn polar(&mut self, r: f64, a: f64, tone: u8, ink: u8) {
        self.plot(r * a.cos(), r * a.sin(), tone, ink);
    }

    fn push(&mut self, x: f64, y: f64, tone: u8, ink: u8, mode: u8) {
        if self.actions.len() < abi::MAX_OPS {
            self.actions.push(abi::Op { x, y, tone, ink, mode, _pad: [0; 5] });
        }
    }
}

/// The buddy's randomness: it asks Iris for it, so the seeding order is part of how it looks.
pub struct Rand(());

impl Rand {
    /// A number in 0..1.
    pub fn rand(&mut self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            abi::iris_rand()
        }
        #[cfg(not(target_arch = "wasm32"))]
        0.5
    }
}

/// An Iris piece.
pub trait Piece: Sized {
    /// Which space it goes into: what it replaces in the core and what it gets in [`Context::p`].
    const SPACE: Space;
    /// Born. Whatever it asks of chance comes from the buddy's randomness, in order.
    fn new(rand: &mut Rand) -> Self;
    /// Advances `dt` seconds.
    fn step(&mut self, _ctx: &Context, _dt: f64) {}
    /// Something that happens all at once.
    fn event(&mut self, _ev: Event, _ctx: &Context) {}
    /// Draws.
    fn draw(&self, _g: &mut Grid, _ctx: &Context) {}
}

#[doc(hidden)]
pub mod __export {
    use super::*;
    use core::cell::UnsafeCell;

    /// What Iris and the piece share: the input Iris writes and the ops it reads.
    /// A piece runs on a single thread; this is only touched from the exported functions.
    pub struct Shared<P> {
        pub input: UnsafeCell<abi::Input>,
        pub actions: UnsafeCell<Vec<abi::Op>>,
        pub piece: UnsafeCell<Option<P>>,
    }
    unsafe impl<P> Sync for Shared<P> {}

    impl<P: Piece> Shared<P> {
        pub const fn new() -> Self {
            Shared { input: UnsafeCell::new(abi::Input::ZERO), actions: UnsafeCell::new(Vec::new()), piece: UnsafeCell::new(None) }
        }

        fn context(&self) -> Context {
            let e = unsafe { &*self.input.get() };
            Context {
                t: e.t,
                state: State::from(e.state),
                since: e.since,
                calm: e.calm,
                voice: e.voice,
                music: e.music,
                context: e.context,
                anchor: Anchor { cx: e.anchor_cx, cy: e.anchor_cy, radius: e.anchor_radius, base: e.anchor_base, ink: e.anchor_ink as u8, border: e.anchor_border as u8 },
                p: e.p,
            }
        }

        pub fn input(&self) -> u32 {
            self.input.get() as u32
        }

        pub fn create(&self) {
            unsafe { *self.piece.get() = Some(P::new(&mut Rand(()))) };
        }

        pub fn step(&self) {
            let ctx = self.context();
            let dt = unsafe { (*self.input.get()).dt };
            if let Some(p) = unsafe { &mut *self.piece.get() } {
                p.step(&ctx, dt);
            }
        }

        pub fn event(&self, ev: u32) {
            let ctx = self.context();
            if let (Some(p), Some(ev)) = (unsafe { &mut *self.piece.get() }, Event::from(ev)) {
                p.event(ev, &ctx);
            }
        }

        /// Draws and leaves the ops where Iris reads them: (address << 32) | count.
        pub fn draw(&self) -> u64 {
            let ctx = self.context();
            let du = unsafe { (*self.input.get()).du };
            let actions = unsafe { &mut *self.actions.get() };
            let mut g = Grid { actions: core::mem::take(actions), du };
            g.actions.clear();
            if let Some(p) = unsafe { &*self.piece.get() } {
                p.draw(&mut g, &ctx);
            }
            *actions = g.actions;
            ((actions.as_ptr() as u64) << 32) | actions.len() as u64
        }
    }
}

/// Exports a [`Piece`] with the functions Iris looks for (ABI 1).
#[macro_export]
macro_rules! export {
    ($kind:ty) => {
        static __IRIS: $crate::__export::Shared<$kind> = $crate::__export::Shared::new();

        #[unsafe(no_mangle)]
        pub extern "C" fn iris_abi() -> u32 {
            $crate::abi::VERSION
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_space() -> u32 {
            <$kind as $crate::Piece>::SPACE as u32
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_input() -> u32 {
            __IRIS.input()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_new() {
            __IRIS.create()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_step() {
            __IRIS.step()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_event(ev: u32) {
            __IRIS.event(ev)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_paint() -> u64 {
            __IRIS.draw()
        }
    };
}
