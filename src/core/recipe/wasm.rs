//! A recipe part in WebAssembly (ABI 2, see `iris_piece::abi` and `iris_piece::part`): someone
//! else's code, in the sandbox, taking a place in the recipe like any built-in part.
//!
//! Before every call Iris writes the part's io block (the head, its inputs read from the bus and
//! its settings); after it, reads its outputs back into the bus. If it runs out of fuel, that call
//! is skipped (nothing drawn that frame) and the next one tries again: a frame that's heavier
//! than usual (a state, an event, a bigger window) doesn't turn it off. Only `STRIKES` in a row
//! do; any other failure (trap, memory) turns it off at once. Broken, it does nothing from then
//! on: the rest of the buddy keeps going.

use std::cell::RefCell;

use iris_piece::abi::{self, head, happening};
use wasmi::{Memory, Store, TypedFunc};

use super::bus::{Bus, Ch};
use super::part::{Frame, Part, World};
use super::Build;
use crate::core::context::{Context, Notice};
use crate::core::grid::Grid;
use crate::core::color::tones;
use crate::core::host::{apply_ops, sandbox, HostData, OwnInks, FUEL};
use crate::core::rand::Rand;
use crate::core::state::{State, ALL};

struct Inner {
    store: Store<HostData>,
    memory: Memory,
    io: u32,
    f_step: Option<TypedFunc<(), ()>>,
    f_event: Option<TypedFunc<(u32, f64, f64), ()>>,
    f_layout: Option<TypedFunc<(), ()>>,
    f_paint: Option<TypedFunc<(), u64>>,
    /// Why it stopped working, if it did.
    broken: Option<String>,
    /// Calls in a row that ran out of fuel.
    strikes: u32,
}

/// How a call went wrong: skipped this time (out of fuel), or broken for good.
enum Failure {
    Skip(String),
    Fail(String),
}
use Failure::{Fail, Skip};

impl From<String> for Failure {
    fn from(why: String) -> Self {
        Fail(why)
    }
}

/// Calls in a row out of fuel before a part counts as broken: about two seconds of frames.
const STRIKES: u32 = 60;

pub(crate) struct WasmPart {
    inner: RefCell<Inner>,
    inputs: Vec<Ch>,
    settings: Vec<f64>,
    outputs: Vec<Ch>,
    /// Its own colors (0–255 RGB), its inks 5 onwards.
    colors: Vec<[f64; 3]>,
}

/// A setting as the part declares it.
struct Declared {
    name: String,
    default: f64,
    min: f64,
    max: f64,
}

fn number(v: &toml::Value) -> Option<f64> {
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).or_else(|| v.as_bool().map(|b| if b { 1.0 } else { 0.0 }))
}

/// Work for a call that draws: what's drawn grows with the grid (a body filled dot by dot in a
/// big window has tens of thousands), so 150 per dot on top of the base, up to a ceiling that
/// still cuts a runaway piece in a few tens of milliseconds.
fn fuel_for(dw: usize, dh: usize) -> u64 {
    (150 * (dw * dh) as u64).clamp(FUEL, 20 * FUEL)
}

fn state_index(s: State) -> f64 {
    ALL.iter().position(|x| *x == s).unwrap_or(0) as f64
}

impl WasmPart {
    pub fn load(wasm: &[u8], b: &mut Build) -> Result<Self, String> {
        let (mut store, instance) = sandbox(wasm)?;
        let func = |name: &str| format!("le falta la función {name}");
        let call_u32 = |store: &mut Store<HostData>, name: &str| -> Result<u32, String> {
            instance.get_typed_func::<(), u32>(&*store, name).map_err(|_| func(name))?.call(store, ()).map_err(|e| e.to_string())
        };
        let v = call_u32(&mut store, "iris_abi")?;
        if v != abi::VERSION_PART {
            return Err(format!("es del contrato {v}; una parte de receta habla el {}", abi::VERSION_PART));
        }
        let memory = instance.get_memory(&store, "memory").ok_or("no exporta su memoria")?;

        // What it declares: the channels it reads and writes, and its settings.
        let r = instance.get_typed_func::<(), u64>(&store, "iris_decl").map_err(|_| func("iris_decl"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        let mut text = vec![0u8; (r as u32 as usize).min(64 << 10)];
        memory.read(&store, (r >> 32) as usize, &mut text).map_err(|e| format!("declaración fuera de su memoria: {e}"))?;
        let decl: toml::Table = String::from_utf8(text).map_err(|_| "su declaración no es texto")?.parse().map_err(|e: toml::de::Error| format!("su declaración no se entiende: {e}"))?;
        let names = |key: &str| -> Vec<String> {
            decl.get(key).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|n| n.as_str().map(str::to_string)).collect()).unwrap_or_default()
        };
        let declared: Vec<Declared> = decl
            .get("settings")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| {
                        let n = |k: &str| s.get(k).and_then(number);
                        Some(Declared { name: s.get("name")?.as_str()?.to_string(), default: n("default")?, min: n("min").unwrap_or(f64::MIN), max: n("max").unwrap_or(f64::MAX) })
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(k) = b.spec.settings.keys().find(|k| !declared.iter().any(|d| &d.name == *k)) {
            return Err(format!("no tiene ningún ajuste «{k}»"));
        }
        let settings = declared.iter().map(|d| b.spec.settings.get(&d.name).and_then(number).unwrap_or(d.default).clamp(d.min, d.max)).collect();
        let mut colors = vec![];
        for c in names("colors") {
            colors.push(iris_shape::hex(&c).ok_or(format!("«{c}» no es un color: va como \"#rrggbb\""))?);
        }
        if colors.len() > iris_shape::MAX_COLORS {
            return Err(format!("declara {} colores propios; el máximo es {}", colors.len(), iris_shape::MAX_COLORS));
        }
        let inputs = names("inputs").iter().map(|n| b.input(n)).collect();
        let outputs = names("outputs").iter().map(|n| b.bus.ch(n)).collect();

        let io = call_u32(&mut store, "iris_io")?;
        let f_new = instance.get_typed_func::<(), ()>(&store, "iris_new").map_err(|_| func("iris_new"))?;
        let (f_step, f_event, f_layout, f_paint) = (
            instance.get_typed_func(&store, "iris_step").ok(),
            instance.get_typed_func(&store, "iris_event").ok(),
            instance.get_typed_func(&store, "iris_layout").ok(),
            instance.get_typed_func(&store, "iris_paint").ok(),
        );
        let inner = Inner { store, memory, io, f_step, f_event, f_layout, f_paint, broken: None, strikes: 0 };
        let part = WasmPart { inner: RefCell::new(inner), inputs, settings, outputs, colors };
        // It is born with its settings written, and with the buddy's randomness, lent.
        let mut bus = std::mem::take(b.bus);
        let ok = part.call(&mut bus, None, 0.0, None, Some(b.rand), |s| f_new.call(s, ()));
        *b.bus = bus;
        match ok {
            Ok(()) => Ok(part),
            Err(e) => Err(format!("falló al nacer: {e}")),
        }
    }

    /// Writes the io block, makes the call, reads the outputs back. The randomness, if any, is
    /// lent for the call and returned as it ended up.
    fn call<T>(
        &self,
        bus: &mut Bus,
        ctx: Option<&Context>,
        dt: f64,
        grid: Option<&Grid>,
        rand: Option<&mut Rand>,
        f: impl FnOnce(&mut Store<HostData>) -> Result<T, wasmi::Error>,
    ) -> Result<T, String> {
        let mut inner = self.inner.borrow_mut();
        let inner = &mut *inner;
        if let Some(why) = &inner.broken {
            return Err(why.clone());
        }
        let mut slots = vec![0.0; head::LEN];
        slots[head::CALM] = 1.0;
        if let Some(c) = ctx {
            slots[head::T] = c.t;
            slots[head::DT] = dt;
            slots[head::SINCE] = c.since;
            slots[head::STATE] = state_index(c.state);
            slots[head::CALM] = c.calm();
            slots[head::VOICE] = c.sig.level as f64;
            slots[head::MUSIC] = c.sig.music as f64;
            slots[head::CONTEXT] = c.sig.ctx;
        }
        if let Some(g) = grid {
            (slots[head::DU], slots[head::DW], slots[head::DH]) = (g.du, g.dw as f64, g.dh as f64);
            (slots[head::ASP], slots[head::S], slots[head::OX], slots[head::OY], slots[head::K]) = (g.asp, g.s, g.ox, g.oy, g.k);
        }
        slots.extend(self.inputs.iter().map(|c| bus.get(*c)));
        slots.extend(&self.settings);
        let bytes: Vec<u8> = slots.iter().flat_map(|v| v.to_le_bytes()).collect();
        let result: Result<T, Failure> = (|| {
            inner.memory.write(&mut inner.store, inner.io as usize, &bytes).map_err(|e| format!("no se puede escribir su entrada: {e}"))?;
            inner.store.set_fuel(grid.map_or(FUEL, |g| fuel_for(g.dw, g.dh))).map_err(|e| e.to_string())?;
            inner.store.data_mut().rand = rand.as_ref().map(|r| (**r).clone());
            let out = f(&mut inner.store).map_err(|e| {
                let fuel = e.as_trap_code() == Some(wasmi::TrapCode::OutOfFuel);
                (e.to_string(), fuel)
            });
            if let (Some(r), Some(a)) = (rand, inner.store.data_mut().rand.take()) {
                *r = a;
            }
            let out = match out {
                Ok(o) => {
                    inner.strikes = 0;
                    o
                }
                Err((why, true)) => {
                    // Out of fuel: this call is lost; it only breaks if it keeps happening.
                    inner.strikes += 1;
                    if inner.strikes < STRIKES {
                        return Err(Skip(why));
                    }
                    return Err(Fail(format!("{why} ({STRIKES} veces seguidas)")));
                }
                Err((why, false)) => return Err(Fail(why)),
            };
            let mut buf = vec![0u8; self.outputs.len() * 8];
            inner.memory.read(&inner.store, inner.io as usize + bytes.len(), &mut buf).map_err(|e| format!("salidas fuera de su memoria: {e}"))?;
            for (c, v) in self.outputs.iter().zip(buf.chunks_exact(8)) {
                bus.set(*c, f64::from_le_bytes(v.try_into().unwrap()));
            }
            Ok(out)
        })();
        match result {
            Ok(o) => Ok(o),
            // Skipped, not broken: the next call tries again.
            Err(Skip(why)) => Err(why),
            Err(Fail(why)) => {
                inner.broken = Some(why.clone());
                Err(why)
            }
        }
    }

    fn event(&mut self, w: &mut World, kind: u32, a: f64, b: f64) {
        let Some(f) = self.inner.borrow().f_event else { return };
        let _ = self.call(w.bus, Some(w.ctx), 0.0, None, Some(w.rand), |s| f.call(s, (kind, a, b)));
    }

}

impl Part for WasmPart {
    fn broken(&self) -> Option<String> {
        self.inner.borrow().broken.clone()
    }

    fn step(&mut self, w: &mut World, dt: f64) {
        let Some(f) = self.inner.borrow().f_step else { return };
        let _ = self.call(w.bus, Some(w.ctx), dt, None, Some(w.rand), |s| f.call(s, ()));
    }

    fn change(&mut self, w: &mut World, from: State, to: State) {
        self.event(w, happening::CHANGE, state_index(from), state_index(to));
    }

    fn notice(&mut self, w: &mut World, n: Notice) {
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        match n {
            Notice::Event(ev) => self.event(w, ev as u32, 0.0, 0.0),
            Notice::Born(id) => self.event(w, happening::BORN, id as f64, 0.0),
            Notice::Pulse(id, error) => self.event(w, happening::PULSE, id as f64, flag(error)),
            Notice::Finish(id, ok) => self.event(w, happening::FINISH, id as f64, flag(ok)),
            Notice::Gone => self.event(w, happening::GONE, 0.0, 0.0),
        }
    }

    fn layout(&self, f: &mut Frame, g: &Grid) {
        let Some(func) = self.inner.borrow().f_layout else { return };
        let _ = self.call(&mut f.bus, Some(f.ctx), 0.0, Some(g), None, |s| func.call(s, ()));
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        let Some(func) = self.inner.borrow().f_paint else { return };
        let mut bus = f.bus.clone();
        let Ok(r) = self.call(&mut bus, Some(f.ctx), 0.0, Some(g), None, |s| func.call(s, ())) else { return };
        let inner = &mut *self.inner.borrow_mut();
        if let Err(e) = apply_ops(g, &inner.memory, &inner.store, r, OwnInks { base: f.ink_base, count: self.colors.len() as u8 }) {
            inner.broken = Some(e);
        }
    }

    /// Its own colors, after the five base inks.
    fn inks(&self, _col: [f64; 3], _base: u8) -> Vec<[[f64; 3]; 4]> {
        self.colors.iter().map(|c| tones(*c)).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::core::recipe::files::Memory;
    use crate::core::recipe::Recipe;
    use crate::core::state::{Signals, State};
    use crate::core::Core;

    /// A part whose paint loops forever on its first `heavy` calls, and then draws nothing.
    fn heavy_part(heavy: u32) -> Vec<u8> {
        let decl = "inputs = []\noutputs = []\n";
        let (len, data) = (decl.len(), decl.replace('\n', "\\0a"));
        wat::parse_str(format!(
            r#"(module
                (memory (export "memory") 1)
                (data (i32.const 0) "{data}")
                (global $n (mut i32) (i32.const 0))
                (func (export "iris_abi") (result i32) i32.const 2)
                (func (export "iris_decl") (result i64) i64.const {len})
                (func (export "iris_io") (result i32) i32.const 1024)
                (func (export "iris_new"))
                (func (export "iris_paint") (result i64)
                    (global.set $n (i32.add (global.get $n) (i32.const 1)))
                    (if (i32.le_u (global.get $n) (i32.const {heavy})) (then (loop (br 0))))
                    i64.const 0))"#
        ))
        .unwrap()
    }

    fn core_with(wasm: Vec<u8>) -> Core {
        let mut r = Recipe::parse("[recipe]\nname = \"t\"\n[script.default]\nx = 0.0\n[[piece]]\nuse = \"builtin:presence\"\n[[piece]]\nuse = \"file:p.wasm\"\nid = \"p\"\n[draw]\norder = [\"p\"]\n").unwrap();
        r.files = Some(Arc::new(Memory { files: [("p.wasm".to_string(), wasm)].into() }));
        Core::from_recipe(&r).unwrap()
    }

    fn frames(c: &mut Core, n: usize) -> Vec<(String, String)> {
        let mut broken = vec![];
        for _ in 0..n {
            c.step(0.04, State::Idle, &Signals::default());
            c.dots(80, 60);
            broken.extend(c.take_broken());
        }
        broken
    }

    #[test]
    fn a_few_heavy_frames_dont_turn_a_part_off() {
        let mut c = core_with(heavy_part(5));
        assert!(frames(&mut c, 100).is_empty(), "5 frames out of fuel turned it off");
    }

    #[test]
    fn running_out_of_fuel_every_frame_does() {
        let mut c = core_with(heavy_part(u32::MAX));
        let broken = frames(&mut c, 61);
        assert!(broken.iter().any(|(p, why)| p == "p" && why.contains("fuel")), "{broken:?}");
    }
}
