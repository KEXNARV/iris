//! HOST: a store piece, loaded in WebAssembly (wasmi) and sandboxed. Iris writes the context
//! into its memory before each call and reads back the list of draw orders, which it applies
//! to the grid in one go (see the contract in `iris_piece::abi`). This file has the sandbox and
//! the particle pieces (ABI 1); recipe parts (ABI 2) are in `recipe::wasm`.
//!
//! Sandboxing: the piece can only import `iris.rand`; it has 16 MB of memory and a work cap
//! (fuel) per call; it only uses the base palette's inks. If it fails (trap, fuel, memory),
//! it is left broken and never called again: whoever uses it falls back to the built-in one.
//! Iris never goes down because of a piece.

use iris_piece::abi::{self, Input, Space};
use wasmi::{Config, Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

use super::rand::Rand;
use super::context::{Anchor, Context};
use super::state::Event;
use super::grid::Grid;

/// Work per call: more than enough for a piece that draws a few thousand dots, and it cuts an
/// infinite loop in milliseconds.
pub(crate) const FUEL: u64 = 5_000_000;
const MEMORY: usize = 16 << 20;
/// The inks a piece may use: main, green, red, warm and accent. An ink outside the palette
/// would make Iris step outside it when drawing.
const INKS: u8 = 5;

pub(crate) struct HostData {
    limits: StoreLimits,
    /// The buddy's randomness, lent while the piece is born (and, for parts, while it moves).
    pub rand: Option<Rand>,
}

/// A module instantiated in the sandbox: its only import is `iris.rand`, with the memory cap
/// and fuel on.
pub(crate) fn sandbox(wasm: &[u8]) -> Result<(Store<HostData>, wasmi::Instance), String> {
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, wasm).map_err(|e| format!("no es un módulo WASM válido: {e}"))?;
    let limits = StoreLimitsBuilder::new().memory_size(MEMORY).instances(1).memories(1).tables(1).build();
    let mut store = Store::new(&engine, HostData { limits, rand: None });
    store.limiter(|d| &mut d.limits);
    let mut linker = Linker::<HostData>::new(&engine);
    linker
        .func_wrap("iris", "rand", |mut caller: wasmi::Caller<'_, HostData>| -> f64 { caller.data_mut().rand.as_mut().map_or(0.5, Rand::rand) })
        .map_err(|e| e.to_string())?;
    store.set_fuel(FUEL).map_err(|e| e.to_string())?;
    // Any import other than iris.rand (files, network, WASI…) makes this fail.
    let instance = linker.instantiate_and_start(&mut store, &module).map_err(|e| format!("no se puede instanciar: {e}"))?;
    Ok((store, instance))
}

/// Where a part's own colors are: its ink 5 onwards go to the palette from `base`, and it has
/// `count` of them. A piece without them (`count` 0) only has the five base inks.
#[derive(Clone, Copy, Default)]
pub(crate) struct OwnInks {
    pub base: u8,
    pub count: u8,
}

/// Applies the draw ops a piece left at `r` (address << 32 | count) to the grid.
pub(crate) fn apply_ops(g: &mut Grid, memory: &Memory, store: &Store<HostData>, r: u64, own: OwnInks) -> Result<(), String> {
    let (dir, n) = ((r >> 32) as usize, (r as u32 as usize).min(abi::MAX_OPS));
    let mut buf = vec![0u8; n * std::mem::size_of::<abi::Op>()];
    memory.read(store, dir, &mut buf).map_err(|e| format!("órdenes fuera de su memoria: {e}"))?;
    for o in buf.chunks_exact(std::mem::size_of::<abi::Op>()) {
        let x = f64::from_le_bytes(o[0..8].try_into().unwrap());
        let y = f64::from_le_bytes(o[8..16].try_into().unwrap());
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        let tone = o[16].min(3);
        let ink = match o[17] {
            i if i < INKS => i,
            i if i - INKS < own.count => own.base.saturating_add(i - INKS),
            _ => INKS - 1,
        };
        match o[18] {
            abi::MODE_UNDER => g.plot_under(x, y, tone),
            abi::MODE_DOT => {
                if x >= 0.0 && y >= 0.0 && (x as usize) < g.dw && (y as usize) < g.dh {
                    g.dot(x as usize, y as usize, tone, ink);
                }
            }
            _ => g.plot(x, y, tone, ink),
        }
    }
    Ok(())
}

pub(crate) struct Host {
    store: Store<HostData>,
    memory: Memory,
    input: u32,
    f_step: TypedFunc<(), ()>,
    f_event: TypedFunc<u32, ()>,
    f_paint: TypedFunc<(), u64>,
    pub space: Space,
    /// Why it stopped working, if it did.
    pub broken: Option<String>,
}

impl Host {
    /// Loads a piece and has it born with the buddy's randomness (uses as much as the built-in).
    pub fn load(wasm: &[u8], rand: &mut Rand) -> Result<Self, String> {
        let (mut store, instance) = sandbox(wasm)?;
        let func = |name: &str| format!("le falta la función {name}");
        let abi_v: u32 = instance.get_typed_func::<(), u32>(&store, "iris_abi").map_err(|_| func("iris_abi"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        if abi_v != abi::VERSION {
            return Err(format!("es del contrato {abi_v} y este Iris habla el {}", abi::VERSION));
        }
        let space_n: u32 = instance.get_typed_func::<(), u32>(&store, "iris_space").map_err(|_| func("iris_space"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        let space = Space::from(space_n).ok_or_else(|| format!("espacio desconocido: {space_n}"))?;
        let input: u32 = instance.get_typed_func::<(), u32>(&store, "iris_input").map_err(|_| func("iris_input"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        let memory = instance.get_memory(&store, "memory").ok_or("no exporta su memoria")?;
        let f_new = instance.get_typed_func::<(), ()>(&store, "iris_new").map_err(|_| func("iris_new"))?;
        let f_step = instance.get_typed_func::<(), ()>(&store, "iris_step").map_err(|_| func("iris_step"))?;
        let f_event = instance.get_typed_func::<u32, ()>(&store, "iris_event").map_err(|_| func("iris_event"))?;
        let f_paint = instance.get_typed_func::<(), u64>(&store, "iris_paint").map_err(|_| func("iris_paint"))?;
        // It is born with the buddy's randomness, lent, and gives it back as it ended up.
        store.data_mut().rand = Some(rand.clone());
        store.set_fuel(FUEL).map_err(|e| e.to_string())?;
        f_new.call(&mut store, ()).map_err(|e| format!("falló al nacer: {e}"))?;
        if let Some(a) = store.data_mut().rand.take() {
            *rand = a;
        }
        Ok(Host { store, memory, input, f_step, f_event, f_paint, space, broken: None })
    }

    fn write_input(&mut self, ctx: &Context, p: &[f64; 16], anchor: Option<&Anchor>, du: f64, dt: f64) -> bool {
        let a = anchor.copied();
        let e = Input {
            t: ctx.t,
            dt,
            since: ctx.since,
            calm: ctx.calm(),
            voice: ctx.sig.level as f64,
            music: ctx.sig.music as f64,
            context: ctx.sig.ctx,
            anchor_cx: a.map_or(0.0, |a| a.cx),
            anchor_cy: a.map_or(0.0, |a| a.cy),
            anchor_radius: a.map_or(0.0, |a| a.radius),
            anchor_base: a.map_or(0.0, |a| a.base),
            du,
            state: ctx.state as u32,
            anchor_ink: a.map_or(0, |a| a.ink as u32),
            anchor_border: a.map_or(0, |a| a.border as u32),
            _pad: 0,
            p: *p,
        };
        // `Input` is repr(C) and only holds numbers: its bytes are its contract layout.
        let bytes = unsafe { std::slice::from_raw_parts((&e as *const Input).cast::<u8>(), std::mem::size_of::<Input>()) };
        match self.memory.write(&mut self.store, self.input as usize, bytes) {
            Ok(()) => true,
            Err(e) => {
                self.fail(format!("no se puede escribir su entrada: {e}"));
                false
            }
        }
    }

    fn fail(&mut self, reason: String) {
        if self.broken.is_none() {
            self.broken = Some(reason);
        }
    }

    fn fuel(&mut self) -> bool {
        match self.store.set_fuel(FUEL) {
            Ok(()) => true,
            Err(e) => {
                self.fail(e.to_string());
                false
            }
        }
    }

    pub fn step(&mut self, ctx: &Context, p: &[f64; 16], anchor: Option<&Anchor>, dt: f64) {
        if self.broken.is_some() || !self.write_input(ctx, p, anchor, 0.0, dt) || !self.fuel() {
            return;
        }
        if let Err(e) = self.f_step.call(&mut self.store, ()) {
            self.fail(format!("falló al avanzar: {e}"));
        }
    }

    pub fn event(&mut self, ev: Event, ctx: &Context, p: &[f64; 16]) {
        if self.broken.is_some() || !self.write_input(ctx, p, None, 0.0, 0.0) || !self.fuel() {
            return;
        }
        if let Err(e) = self.f_event.call(&mut self.store, ev as u32) {
            self.fail(format!("falló con un evento: {e}"));
        }
    }

    pub fn draw(&mut self, g: &mut Grid, ctx: &Context, p: &[f64; 16], anchor: Option<&Anchor>) {
        if self.broken.is_some() || !self.write_input(ctx, p, anchor, g.du, 0.0) || !self.fuel() {
            return;
        }
        let r = match self.f_paint.call(&mut self.store, ()) {
            Ok(r) => r,
            Err(e) => return self.fail(format!("falló al dibujar: {e}")),
        };
        if let Err(e) = apply_ops(g, &self.memory, &self.store, r, OwnInks::default()) {
            self.fail(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{Signals, State};
    use crate::core::pieces::Piece;
    use crate::core::pieces::particles::{self, Motes};

    const MOTES: &[u8] = include_bytes!("../../crates/pieces/motes/motes.wasm");

    fn ctx(t: f64, state: State) -> Context {
        Context { t, state, since: 0.0, sig: Signals::default(), look: None }
    }

    /// The test script: idle, listening, speaking and swallowing, with the gaze moving.
    fn frame(i: usize) -> (State, [f64; 16]) {
        let t = i as f64 / 30.0;
        let (state, level, swallow) = match i {
            0..90 => (State::Idle, 0.0, 0.0),
            90..180 => (State::Listening, 0.4 + 0.3 * (t * 7.0).sin(), 0.0),
            180..270 => (State::Speaking, 0.0, 0.0),
            _ => (State::Transcribing, 0.0, 0.8),
        };
        let mut p = [0.0; 16];
        p[..10].copy_from_slice(&[0.40, level, swallow, 0.0, 1.0, 1.0, 0.0, 0.3 * (t * 0.5).sin(), 0.2 * (t * 0.3).cos(), 0.0]);
        (state, p)
    }

    fn paint(p: &[f64; 16]) -> particles::Paint {
        particles::Paint { motes: p[4], boot: p[5], press: p[6], gaze: [p[7], p[8]], low: p[9] > 0.5 }
    }

    #[test]
    fn wasm_motes_draw_the_same_as_native() {
        let (mut an, mut aw) = (Rand::new(), Rand::new());
        let mut native = Motes::seed(&mut an);
        let mut piece = Host::load(MOTES, &mut aw).expect("carga");
        assert_eq!(piece.space, Space::Particles);
        // They used up the same amount of the buddy's randomness, in the same order.
        assert_eq!(an.clone().rand(), aw.clone().rand());
        let (mut dots, mut differing) = (0, 0);
        for i in 0..300 {
            let (state, p) = frame(i);
            let c = ctx(i as f64 / 30.0, state);
            native.step(&c, particles::Step { base: p[0], level: p[1], swallow: p[2], dance: p[3] }, 1.0 / 30.0);
            piece.step(&c, &p, None, 1.0 / 30.0);
            if i % 15 == 0 {
                let (mut gn, mut gw) = (Grid::with_dots(150, 150, 3, 3), Grid::with_dots(150, 150, 3, 3));
                native.paint(&mut gn, &c, paint(&p));
                piece.draw(&mut gw, &c, &p, None);
                dots += gn.dots.iter().filter(|&&d| d & 3 != 0).count();
                differing += gn.dots.iter().zip(&gw.dots).filter(|(a, b)| a != b).count();
            }
        }
        assert!(piece.broken.is_none(), "{:?}", piece.broken);
        assert!(dots > 500, "la prueba no dibujó casi nada: {dots}");
        println!("motas: {differing} de {dots} puntos distintos entre nativa y WASM");
        // wasm32's sine and cosine are not the machine's: some dot on the edge of a cell may fall
        // into the neighboring one. More than that is something else.
        assert!(differing * 200 <= dots, "{differing} de {dots} puntos distintos");
    }

    /// A handwritten piece, in WAT, with the full contract and `draw` as `iris_paint`.
    fn piece_wat(extra: &str, abi_v: u32, draw: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module {extra}
                 (memory (export "memory") 1)
                 (func (export "iris_abi") (result i32) i32.const {abi_v})
                 (func (export "iris_space") (result i32) i32.const 1)
                 (func (export "iris_input") (result i32) i32.const 1024)
                 (func (export "iris_new"))
                 (func (export "iris_step"))
                 (func (export "iris_event") (param i32))
                 (func (export "iris_paint") (result i64) {draw}))"#
        ))
        .unwrap()
    }

    #[test]
    fn infinite_loop_breaks_it_without_hanging_iris() {
        let wasm = piece_wat("", 1, "(loop (br 0)) i64.const 0");
        let mut piece = Host::load(&wasm, &mut Rand::new()).expect("carga");
        let mut g = Grid::with_dots(40, 40, 3, 3);
        let before = std::time::Instant::now();
        piece.draw(&mut g, &ctx(0.0, State::Idle), &[0.0; 16], None);
        assert!(piece.broken.is_some());
        assert!(before.elapsed().as_secs() < 2, "tardó {:?} en cortarla", before.elapsed());
        // Broken, it is no longer called.
        piece.draw(&mut g, &ctx(0.0, State::Idle), &[0.0; 16], None);
    }

    #[test]
    fn rejects_imports_outside_iris() {
        let wasm = piece_wat(r#"(import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))"#, 1, "i64.const 0");
        assert!(Host::load(&wasm, &mut Rand::new()).is_err());
    }

    #[test]
    fn rejects_another_contract_version() {
        let wasm = piece_wat("", 2, "i64.const 0");
        let e = Host::load(&wasm, &mut Rand::new()).err().unwrap();
        assert!(e.contains("contrato 2"), "{e}");
    }

    #[test]
    fn out_of_palette_inks_are_clamped() {
        // An order at 2048 (x=0, y=0, tone 9, ink 200): drawn with tone 3 and the accent ink.
        let op = "(i64.store (i32.const 2048) (i64.const 0)) (i64.store (i32.const 2056) (i64.const 0)) (i32.store8 (i32.const 2064) (i32.const 9)) (i32.store8 (i32.const 2065) (i32.const 200))";
        let wasm = piece_wat("", 1, &format!("{op} i64.const 8796093022209"));
        let mut piece = Host::load(&wasm, &mut Rand::new()).expect("carga");
        let mut g = Grid::with_dots(40, 40, 3, 3);
        piece.draw(&mut g, &ctx(0.0, State::Idle), &[0.0; 16], None);
        assert!(piece.broken.is_none(), "{:?}", piece.broken);
        let d = *g.dots.iter().find(|&&d| d != 0).expect("dibujó");
        assert_eq!((d & 3, d >> 2), (3, INKS - 1));
    }

    /// `cargo test --release host -- --ignored --nocapture`: what a piece costs per frame.
    #[test]
    #[ignore]
    fn cost_per_frame() {
        let (mut an, mut aw) = (Rand::new(), Rand::new());
        let mut native = Motes::seed(&mut an);
        let mut piece = Host::load(MOTES, &mut aw).unwrap();
        let mut g = Grid::with_dots(150, 150, 3, 3);
        let n = 3000;
        let measure = |f: &mut dyn FnMut(usize)| {
            let t0 = std::time::Instant::now();
            (0..n).for_each(&mut *f);
            t0.elapsed().as_secs_f64() * 1e6 / n as f64
        };
        let us_n = measure(&mut |i| {
            let (state, p) = frame(i % 300);
            let c = ctx(i as f64 / 30.0, state);
            native.step(&c, particles::Step { base: p[0], level: p[1], swallow: p[2], dance: p[3] }, 1.0 / 30.0);
            native.paint(&mut g, &c, paint(&p));
        });
        let us_w = measure(&mut |i| {
            let (state, p) = frame(i % 300);
            let c = ctx(i as f64 / 30.0, state);
            piece.step(&c, &p, None, 1.0 / 30.0);
            piece.draw(&mut g, &c, &p, None);
        });
        println!("partículas por cuadro: nativas {us_n:.1} µs · WASM {us_w:.1} µs ({:.0}×)", us_w / us_n);
        assert!(us_w < 2000.0, "{us_w} µs por cuadro: no entra en el presupuesto de 2 ms");
    }
}
