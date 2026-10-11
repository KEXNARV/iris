//! Parts (ABI 2): anything that goes into a buddy's recipe. A part declares the channels it
//! reads and writes and the settings it offers, and does one thing with them: move, react,
//! publish numbers for the other parts, or draw.
//!
//! ```ignore
//! use iris_piece::part::*;
//!
//! /// An antenna that sparks on errors.
//! struct Antenna { spark: f64 }
//!
//! impl Part for Antenna {
//!     const INPUTS: &'static [&'static str] = &["body.x", "body.y", "body.radius"];
//!     const SETTINGS: &'static [Setting] = &[Setting::new("height", 1.4, 1.0, 3.0)];
//!     fn new(_io: &Io, _rand: &mut Rand) -> Self { Antenna { spark: 0.0 } }
//!     fn step(&mut self, io: &mut Io) { self.spark = (self.spark - io.dt() * 2.0).max(0.0); }
//!     fn event(&mut self, h: Happening, _io: &mut Io) { if h == Happening::Event(Event::Error) { self.spark = 1.0; } }
//!     fn paint(&self, g: &mut Grid, io: &Io) {
//!         let (x, y, r) = (io.get("body.x"), io.get("body.y"), io.get("body.radius"));
//!         g.plot(x, y + r * io.get("height"), if self.spark > 0.5 { 3 } else { 1 }, INK_ACCENT);
//!     }
//! }
//!
//! export_part!(Antenna);
//! ```

pub use crate::abi::{head, Event, State};
pub use crate::{Grid, Rand, INK_ACCENT, INK_GREEN, INK_MAIN, INK_RED, INK_WARM};

/// A setting a part offers: a number between `min` and `max`. The recipe can set it; otherwise
/// it takes `default`. (On/off is 0 or 1.)
#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub name: &'static str,
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

impl Setting {
    pub const fn new(name: &'static str, default: f64, min: f64, max: f64) -> Self {
        Setting { name, default, min, max }
    }
}

/// What happens to a part all at once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Happening {
    Event(Event),
    /// The visible state changed.
    Change { from: State, to: State },
    /// A subagent was born.
    Born(u64),
    /// A subagent did something; `true` if it was an error.
    Pulse(u64, bool),
    /// A subagent finished; `true` if it went well.
    Finish(u64, bool),
    /// The subagents left with the engine.
    Gone,
}

impl Happening {
    pub fn from(kind: u32, a: f64, b: f64) -> Option<Self> {
        use crate::abi::happening::*;
        Some(match kind {
            0..=EVENT_LAST => Happening::Event(Event::from(kind)?),
            CHANGE => Happening::Change { from: State::from(a as u32), to: State::from(b as u32) },
            BORN => Happening::Born(a as u64),
            PULSE => Happening::Pulse(a as u64, b > 0.5),
            FINISH => Happening::Finish(a as u64, b > 0.5),
            GONE => Happening::Gone,
            _ => return None,
        })
    }
}

/// The part's view of its io block: Iris's head, its inputs and settings, and its outputs.
pub struct Io {
    pub(crate) slots: Vec<f64>,
    inputs: &'static [&'static str],
    settings: &'static [Setting],
    outputs: &'static [&'static str],
}

impl Io {
    pub(crate) fn new(inputs: &'static [&'static str], settings: &'static [Setting], outputs: &'static [&'static str]) -> Self {
        let mut slots = vec![0.0; head::LEN + inputs.len() + settings.len() + outputs.len()];
        slots[head::CALM] = 1.0;
        Io { slots, inputs, settings, outputs }
    }

    pub fn t(&self) -> f64 {
        self.slots[head::T]
    }

    pub fn dt(&self) -> f64 {
        self.slots[head::DT]
    }

    /// Seconds in the visible state.
    pub fn st(&self) -> f64 {
        self.slots[head::T] - self.slots[head::SINCE]
    }

    pub fn state(&self) -> State {
        State::from(self.slots[head::STATE] as u32)
    }

    pub fn calm(&self) -> f64 {
        self.slots[head::CALM]
    }

    pub fn voice(&self) -> f64 {
        self.slots[head::VOICE]
    }

    pub fn music(&self) -> f64 {
        self.slots[head::MUSIC]
    }

    pub fn context(&self) -> f64 {
        self.slots[head::CONTEXT]
    }

    /// The size of one dot, in world units (while laying out and painting).
    pub fn du(&self) -> f64 {
        self.slots[head::DU]
    }

    /// The grid's dots across and down (while laying out and painting).
    pub fn grid_size(&self) -> (usize, usize) {
        (self.slots[head::DW] as usize, self.slots[head::DH] as usize)
    }

    /// The center of dot (`i`, `j`) in world coordinates: the same computation Iris does.
    pub fn center(&self, i: usize, j: usize) -> (f64, f64) {
        let s = &self.slots;
        let (dw, dh) = (s[head::DW], s[head::DH]);
        let x = (i as f64 / (dw - 1.0) * 2.0 - 1.0) * s[head::ASP] / s[head::S];
        let y = (1.0 - j as f64 / (dh - 1.0) * 2.0) / s[head::S];
        ((x - s[head::OX]) / s[head::K], (y - s[head::OY]) / s[head::K])
    }

    /// An input or a setting, by name. `NaN` for a name the part didn't declare.
    pub fn get(&self, name: &str) -> f64 {
        if let Some(i) = self.inputs.iter().position(|n| *n == name) {
            return self.slots[head::LEN + i];
        }
        if let Some(i) = self.settings.iter().position(|s| s.name == name) {
            return self.slots[head::LEN + self.inputs.len() + i];
        }
        f64::NAN
    }

    /// The `i`-th declared input (faster than by name).
    pub fn input(&self, i: usize) -> f64 {
        self.slots[head::LEN + i]
    }

    /// The `i`-th declared setting.
    pub fn setting(&self, i: usize) -> f64 {
        self.slots[head::LEN + self.inputs.len() + i]
    }

    /// Publishes an output, by name. Ignored if the part didn't declare it.
    pub fn set(&mut self, name: &str, v: f64) {
        if let Some(i) = self.outputs.iter().position(|n| *n == name) {
            self.put(i, v);
        }
    }

    /// Publishes the `i`-th declared output.
    pub fn put(&mut self, i: usize, v: f64) {
        let k = head::LEN + self.inputs.len() + self.settings.len() + i;
        self.slots[k] = v;
    }
}

/// A recipe part.
pub trait Part: Sized {
    /// The channels it reads (Iris writes them before every call).
    const INPUTS: &'static [&'static str] = &[];
    /// The channels it publishes (Iris reads them after every call).
    const OUTPUTS: &'static [&'static str] = &[];
    /// What a recipe can adjust.
    const SETTINGS: &'static [Setting] = &[];

    /// Born. Whatever it asks of chance comes from the buddy's randomness, in order.
    fn new(io: &Io, rand: &mut Rand) -> Self;
    /// Advances `io.dt()` seconds.
    fn step(&mut self, _io: &mut Io) {}
    /// Something that happens all at once.
    fn event(&mut self, _h: Happening, _io: &mut Io) {}
    /// Before painting, once per grid: publish what depends on the grid or on where the body is.
    fn layout(&self, _io: &mut Io) {}
    /// Draws.
    fn paint(&self, _g: &mut Grid, _io: &Io) {}
}

/// The declaration Iris reads from `iris_decl`: TOML with the inputs, outputs and settings
/// (an array, so their order is kept).
#[doc(hidden)]
pub fn declaration<P: Part>() -> String {
    let list = |names: &[&str]| names.iter().map(|n| format!("{n:?}")).collect::<Vec<_>>().join(", ");
    let mut s = format!("inputs = [{}]\noutputs = [{}]\n", list(P::INPUTS), list(P::OUTPUTS));
    for st in P::SETTINGS {
        s.push_str(&format!("\n[[settings]]\nname = {:?}\ndefault = {:?}\nmin = {:?}\nmax = {:?}\n", st.name, st.default, st.min, st.max));
    }
    s
}

#[doc(hidden)]
pub mod __export {
    use super::*;
    use core::cell::UnsafeCell;

    /// What Iris and the part share. A part runs on a single thread; this is only touched from
    /// the exported functions.
    pub struct Shared<P> {
        pub io: UnsafeCell<Option<Io>>,
        pub decl: UnsafeCell<String>,
        pub ops: UnsafeCell<Vec<crate::abi::Op>>,
        pub part: UnsafeCell<Option<P>>,
    }
    unsafe impl<P> Sync for Shared<P> {}

    impl<P: Part> Shared<P> {
        pub const fn new() -> Self {
            Shared { io: UnsafeCell::new(None), decl: UnsafeCell::new(String::new()), ops: UnsafeCell::new(Vec::new()), part: UnsafeCell::new(None) }
        }

        #[allow(clippy::mut_from_ref)]
        fn io(&self) -> &mut Io {
            unsafe { (*self.io.get()).get_or_insert_with(|| Io::new(P::INPUTS, P::SETTINGS, P::OUTPUTS)) }
        }

        pub fn decl(&self) -> u64 {
            let d = unsafe { &mut *self.decl.get() };
            if d.is_empty() {
                *d = declaration::<P>();
            }
            ((d.as_ptr() as u64) << 32) | d.len() as u64
        }

        pub fn io_ptr(&self) -> u32 {
            self.io().slots.as_ptr() as u32
        }

        pub fn create(&self) {
            let part = P::new(self.io(), &mut Rand(()));
            unsafe { *self.part.get() = Some(part) };
        }

        fn with(&self, f: impl FnOnce(&mut P, &mut Io)) {
            if let Some(p) = unsafe { &mut *self.part.get() } {
                f(p, self.io());
            }
        }

        pub fn step(&self) {
            self.with(|p, io| p.step(io));
        }

        pub fn event(&self, kind: u32, a: f64, b: f64) {
            if let Some(h) = Happening::from(kind, a, b) {
                self.with(|p, io| p.event(h, io));
            }
        }

        pub fn layout(&self) {
            self.with(|p, io| p.layout(io));
        }

        /// Draws and leaves the ops where Iris reads them: (address << 32) | count.
        pub fn paint(&self) -> u64 {
            let ops = unsafe { &mut *self.ops.get() };
            let mut g = Grid { actions: core::mem::take(ops), du: self.io().du() };
            g.actions.clear();
            self.with(|p, io| p.paint(&mut g, io));
            *ops = g.actions;
            ((ops.as_ptr() as u64) << 32) | ops.len() as u64
        }
    }
}

/// Exports a [`Part`] with the functions Iris looks for (ABI 2).
#[macro_export]
macro_rules! export_part {
    ($kind:ty) => {
        static __IRIS_PART: $crate::part::__export::Shared<$kind> = $crate::part::__export::Shared::new();

        #[unsafe(no_mangle)]
        pub extern "C" fn iris_abi() -> u32 {
            $crate::abi::VERSION_PART
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_decl() -> u64 {
            __IRIS_PART.decl()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_io() -> u32 {
            __IRIS_PART.io_ptr()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_new() {
            __IRIS_PART.create()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_step() {
            __IRIS_PART.step()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_event(kind: u32, a: f64, b: f64) {
            __IRIS_PART.event(kind, a, b)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_layout() {
            __IRIS_PART.layout()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_paint() -> u64 {
            __IRIS_PART.paint()
        }
    };
}
