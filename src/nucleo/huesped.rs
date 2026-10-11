//! HUÉSPED: una pieza de la tienda, cargada en WebAssembly (wasmi) y aislada. Iris le escribe
//! el contexto en su memoria antes de cada llamada y le lee de vuelta la lista de órdenes de
//! dibujo, que aplica a la rejilla de una sola vez (ver el contrato en `iris_pieza::abi`).
//!
//! Aislamiento: la pieza solo puede importar `iris.azar`; tiene 16 MB de memoria y un tope de
//! trabajo (combustible) por llamada; solo usa las tintas de la paleta base. Si falla (trap,
//! combustible, memoria), queda rota y no se vuelve a llamar: el que la usa pone la de fábrica.
//! Iris nunca se cae por una pieza.

use iris_pieza::abi::{self, Entrada, Espacio};
use wasmi::{Config, Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

use super::azar::Azar;
use super::contexto::{Ancla, Contexto};
use super::estado::Event;
use super::rejilla::Grid;

/// Trabajo por llamada: de sobra para una pieza que dibuja unos miles de puntos, y corta un
/// bucle infinito en milisegundos.
const COMBUSTIBLE: u64 = 5_000_000;
const MEMORIA: usize = 16 << 20;
/// Las tintas que puede usar una pieza: principal, verde, roja, cálida y acento. Una tinta
/// fuera de la paleta haría que Iris se salga de ella al dibujar.
const TINTAS: u8 = 5;

struct Datos {
    limites: StoreLimits,
    /// El azar del buddy, prestado mientras la pieza nace.
    azar: Option<Azar>,
}

pub(crate) struct Huesped {
    store: Store<Datos>,
    memoria: Memory,
    entrada: u32,
    f_paso: TypedFunc<(), ()>,
    f_evento: TypedFunc<u32, ()>,
    f_pintar: TypedFunc<(), u64>,
    pub espacio: Espacio,
    /// Por qué dejó de andar, si dejó.
    pub roto: Option<String>,
}

impl Huesped {
    /// Carga una pieza y la hace nacer con el azar del buddy (consume lo mismo que la de fábrica).
    pub fn cargar(wasm: &[u8], azar: &mut Azar) -> Result<Self, String> {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, wasm).map_err(|e| format!("no es un módulo WASM válido: {e}"))?;
        let limites = StoreLimitsBuilder::new().memory_size(MEMORIA).instances(1).memories(1).tables(1).build();
        let mut store = Store::new(&engine, Datos { limites, azar: None });
        store.limiter(|d| &mut d.limites);
        let mut linker = Linker::<Datos>::new(&engine);
        linker
            .func_wrap("iris", "azar", |mut caller: wasmi::Caller<'_, Datos>| -> f64 { caller.data_mut().azar.as_mut().map_or(0.5, Azar::rand) })
            .map_err(|e| e.to_string())?;
        store.set_fuel(COMBUSTIBLE).map_err(|e| e.to_string())?;
        // Con cualquier import que no sea iris.azar (archivos, red, WASI…) esto falla.
        let instancia = linker.instantiate_and_start(&mut store, &module).map_err(|e| format!("no se puede instanciar: {e}"))?;
        let func = |nombre: &str| format!("le falta la función {nombre}");
        let abi_v: u32 = instancia.get_typed_func::<(), u32>(&store, "iris_abi").map_err(|_| func("iris_abi"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        if abi_v != abi::VERSION {
            return Err(format!("es del contrato {abi_v} y este Iris habla el {}", abi::VERSION));
        }
        let espacio_n: u32 = instancia.get_typed_func::<(), u32>(&store, "iris_espacio").map_err(|_| func("iris_espacio"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        let espacio = Espacio::de(espacio_n).ok_or_else(|| format!("espacio desconocido: {espacio_n}"))?;
        let entrada: u32 = instancia.get_typed_func::<(), u32>(&store, "iris_entrada").map_err(|_| func("iris_entrada"))?.call(&mut store, ()).map_err(|e| e.to_string())?;
        let memoria = instancia.get_memory(&store, "memory").ok_or("no exporta su memoria")?;
        let f_nueva = instancia.get_typed_func::<(), ()>(&store, "iris_nueva").map_err(|_| func("iris_nueva"))?;
        let f_paso = instancia.get_typed_func::<(), ()>(&store, "iris_paso").map_err(|_| func("iris_paso"))?;
        let f_evento = instancia.get_typed_func::<u32, ()>(&store, "iris_evento").map_err(|_| func("iris_evento"))?;
        let f_pintar = instancia.get_typed_func::<(), u64>(&store, "iris_pintar").map_err(|_| func("iris_pintar"))?;
        // Nace con el azar del buddy, prestado, y lo devuelve como quedó.
        store.data_mut().azar = Some(azar.clone());
        store.set_fuel(COMBUSTIBLE).map_err(|e| e.to_string())?;
        f_nueva.call(&mut store, ()).map_err(|e| format!("falló al nacer: {e}"))?;
        if let Some(a) = store.data_mut().azar.take() {
            *azar = a;
        }
        Ok(Huesped { store, memoria, entrada, f_paso, f_evento, f_pintar, espacio, roto: None })
    }

    fn escribir(&mut self, ctx: &Contexto, p: &[f64; 16], ancla: Option<&Ancla>, du: f64, dt: f64) -> bool {
        let a = ancla.copied();
        let e = Entrada {
            t: ctx.t,
            dt,
            desde: ctx.desde,
            calma: ctx.calma(),
            voz: ctx.sig.level as f64,
            musica: ctx.sig.music as f64,
            contexto: ctx.sig.ctx,
            ancla_cx: a.map_or(0.0, |a| a.cx),
            ancla_cy: a.map_or(0.0, |a| a.cy),
            ancla_radio: a.map_or(0.0, |a| a.radio),
            ancla_base: a.map_or(0.0, |a| a.base),
            du,
            estado: ctx.estado as u32,
            ancla_tinta: a.map_or(0, |a| a.tinta as u32),
            ancla_borde: a.map_or(0, |a| a.borde as u32),
            _relleno: 0,
            p: *p,
        };
        // `Entrada` es repr(C) y solo tiene números: sus bytes son su layout del contrato.
        let bytes = unsafe { std::slice::from_raw_parts((&e as *const Entrada).cast::<u8>(), std::mem::size_of::<Entrada>()) };
        match self.memoria.write(&mut self.store, self.entrada as usize, bytes) {
            Ok(()) => true,
            Err(e) => {
                self.romper(format!("no se puede escribir su entrada: {e}"));
                false
            }
        }
    }

    fn romper(&mut self, por: String) {
        if self.roto.is_none() {
            self.roto = Some(por);
        }
    }

    fn combustible(&mut self) -> bool {
        match self.store.set_fuel(COMBUSTIBLE) {
            Ok(()) => true,
            Err(e) => {
                self.romper(e.to_string());
                false
            }
        }
    }

    pub fn paso(&mut self, ctx: &Contexto, p: &[f64; 16], ancla: Option<&Ancla>, dt: f64) {
        if self.roto.is_some() || !self.escribir(ctx, p, ancla, 0.0, dt) || !self.combustible() {
            return;
        }
        if let Err(e) = self.f_paso.call(&mut self.store, ()) {
            self.romper(format!("falló al avanzar: {e}"));
        }
    }

    pub fn evento(&mut self, ev: Event, ctx: &Contexto, p: &[f64; 16]) {
        if self.roto.is_some() || !self.escribir(ctx, p, None, 0.0, 0.0) || !self.combustible() {
            return;
        }
        if let Err(e) = self.f_evento.call(&mut self.store, ev as u32) {
            self.romper(format!("falló con un evento: {e}"));
        }
    }

    pub fn pintar(&mut self, g: &mut Grid, ctx: &Contexto, p: &[f64; 16], ancla: Option<&Ancla>) {
        if self.roto.is_some() || !self.escribir(ctx, p, ancla, g.du, 0.0) || !self.combustible() {
            return;
        }
        let r = match self.f_pintar.call(&mut self.store, ()) {
            Ok(r) => r,
            Err(e) => return self.romper(format!("falló al dibujar: {e}")),
        };
        let (dir, n) = ((r >> 32) as usize, (r as u32 as usize).min(abi::MAX_ORDENES));
        let mut buf = vec![0u8; n * std::mem::size_of::<abi::Orden>()];
        if let Err(e) = self.memoria.read(&self.store, dir, &mut buf) {
            return self.romper(format!("órdenes fuera de su memoria: {e}"));
        }
        for o in buf.chunks_exact(std::mem::size_of::<abi::Orden>()) {
            let x = f64::from_le_bytes(o[0..8].try_into().unwrap());
            let y = f64::from_le_bytes(o[8..16].try_into().unwrap());
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            let (tono, tinta) = (o[16].min(3), o[17].min(TINTAS - 1));
            match o[18] {
                abi::MODO_DEBAJO => g.plot_under(x, y, tono),
                _ => g.plot(x, y, tono, tinta),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nucleo::estado::{Signals, State};
    use crate::nucleo::piezas::Pieza;
    use crate::nucleo::piezas::particulas::{self, Motas};

    const MOTAS: &[u8] = include_bytes!("../../crates/piezas/motas/motas.wasm");

    fn ctx(t: f64, estado: State) -> Contexto {
        Contexto { t, estado, desde: 0.0, sig: Signals::default(), mira: None }
    }

    /// El guion de la prueba: reposo, escuchar, hablar y tragar, con la mirada moviéndose.
    fn cuadro(i: usize) -> (State, [f64; 16]) {
        let t = i as f64 / 30.0;
        let (estado, nivel, traga) = match i {
            0..90 => (State::Idle, 0.0, 0.0),
            90..180 => (State::Listening, 0.4 + 0.3 * (t * 7.0).sin(), 0.0),
            180..270 => (State::Speaking, 0.0, 0.0),
            _ => (State::Transcribing, 0.0, 0.8),
        };
        let mut p = [0.0; 16];
        p[..10].copy_from_slice(&[0.40, nivel, traga, 0.0, 1.0, 1.0, 0.0, 0.3 * (t * 0.5).sin(), 0.2 * (t * 0.3).cos(), 0.0]);
        (estado, p)
    }

    fn pinta(p: &[f64; 16]) -> particulas::Pinta {
        particulas::Pinta { motes: p[4], boot: p[5], press: p[6], mirada: [p[7], p[8]], low: p[9] > 0.5 }
    }

    #[test]
    fn las_motas_en_wasm_dibujan_lo_mismo_que_las_nativas() {
        let (mut an, mut aw) = (Azar::new(), Azar::new());
        let mut nativa = Motas::sembrar(&mut an);
        let mut pieza = Huesped::cargar(MOTAS, &mut aw).expect("carga");
        assert_eq!(pieza.espacio, Espacio::Particulas);
        // Gastaron lo mismo del azar del buddy, en el mismo orden.
        assert_eq!(an.clone().rand(), aw.clone().rand());
        let (mut puntos, mut distintos) = (0, 0);
        for i in 0..300 {
            let (estado, p) = cuadro(i);
            let c = ctx(i as f64 / 30.0, estado);
            nativa.step(&c, particulas::Paso { base: p[0], nivel: p[1], traga: p[2], baile: p[3] }, 1.0 / 30.0);
            pieza.paso(&c, &p, None, 1.0 / 30.0);
            if i % 15 == 0 {
                let (mut gn, mut gw) = (Grid::with_dots(150, 150, 3, 3), Grid::with_dots(150, 150, 3, 3));
                nativa.paint(&mut gn, &c, pinta(&p));
                pieza.pintar(&mut gw, &c, &p, None);
                puntos += gn.dots.iter().filter(|&&d| d & 3 != 0).count();
                distintos += gn.dots.iter().zip(&gw.dots).filter(|(a, b)| a != b).count();
            }
        }
        assert!(pieza.roto.is_none(), "{:?}", pieza.roto);
        assert!(puntos > 500, "la prueba no dibujó casi nada: {puntos}");
        println!("motas: {distintos} de {puntos} puntos distintos entre nativa y WASM");
        // El seno y el coseno de wasm32 no son los de la máquina: algún punto en el borde de una
        // celda puede caer en la vecina. Más de eso ya es otra cosa.
        assert!(distintos * 200 <= puntos, "{distintos} de {puntos} puntos distintos");
    }

    /// Una pieza a mano, en WAT, con el contrato completo y `cuerpo` como `iris_pintar`.
    fn pieza_wat(extra: &str, abi_v: u32, pintar: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module {extra}
                 (memory (export "memory") 1)
                 (func (export "iris_abi") (result i32) i32.const {abi_v})
                 (func (export "iris_espacio") (result i32) i32.const 1)
                 (func (export "iris_entrada") (result i32) i32.const 1024)
                 (func (export "iris_nueva"))
                 (func (export "iris_paso"))
                 (func (export "iris_evento") (param i32))
                 (func (export "iris_pintar") (result i64) {pintar}))"#
        ))
        .unwrap()
    }

    #[test]
    fn un_bucle_infinito_la_rompe_sin_colgar_a_iris() {
        let wasm = pieza_wat("", 1, "(loop (br 0)) i64.const 0");
        let mut pieza = Huesped::cargar(&wasm, &mut Azar::new()).expect("carga");
        let mut g = Grid::with_dots(40, 40, 3, 3);
        let antes = std::time::Instant::now();
        pieza.pintar(&mut g, &ctx(0.0, State::Idle), &[0.0; 16], None);
        assert!(pieza.roto.is_some());
        assert!(antes.elapsed().as_secs() < 2, "tardó {:?} en cortarla", antes.elapsed());
        // Rota, ya no se la llama.
        pieza.pintar(&mut g, &ctx(0.0, State::Idle), &[0.0; 16], None);
    }

    #[test]
    fn no_carga_si_pide_algo_fuera_de_iris() {
        let wasm = pieza_wat(r#"(import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))"#, 1, "i64.const 0");
        assert!(Huesped::cargar(&wasm, &mut Azar::new()).is_err());
    }

    #[test]
    fn no_carga_otra_version_del_contrato() {
        let wasm = pieza_wat("", 2, "i64.const 0");
        let e = Huesped::cargar(&wasm, &mut Azar::new()).err().unwrap();
        assert!(e.contains("contrato 2"), "{e}");
    }

    #[test]
    fn las_tintas_fuera_de_la_paleta_se_recortan() {
        // Una orden en 2048 (x=0, y=0, tono 9, tinta 200): se dibuja con tono 3 y tinta acento.
        let orden = "(i64.store (i32.const 2048) (i64.const 0)) (i64.store (i32.const 2056) (i64.const 0)) (i32.store8 (i32.const 2064) (i32.const 9)) (i32.store8 (i32.const 2065) (i32.const 200))";
        let wasm = pieza_wat("", 1, &format!("{orden} i64.const 8796093022209"));
        let mut pieza = Huesped::cargar(&wasm, &mut Azar::new()).expect("carga");
        let mut g = Grid::with_dots(40, 40, 3, 3);
        pieza.pintar(&mut g, &ctx(0.0, State::Idle), &[0.0; 16], None);
        assert!(pieza.roto.is_none(), "{:?}", pieza.roto);
        let d = *g.dots.iter().find(|&&d| d != 0).expect("dibujó");
        assert_eq!((d & 3, d >> 2), (3, TINTAS - 1));
    }

    /// `cargo test --release huesped -- --ignored --nocapture`: cuánto cuesta una pieza por cuadro.
    #[test]
    #[ignore]
    fn cuanto_cuesta_por_cuadro() {
        let (mut an, mut aw) = (Azar::new(), Azar::new());
        let mut nativa = Motas::sembrar(&mut an);
        let mut pieza = Huesped::cargar(MOTAS, &mut aw).unwrap();
        let mut g = Grid::with_dots(150, 150, 3, 3);
        let n = 3000;
        let medir = |f: &mut dyn FnMut(usize)| {
            let t0 = std::time::Instant::now();
            (0..n).for_each(&mut *f);
            t0.elapsed().as_secs_f64() * 1e6 / n as f64
        };
        let us_n = medir(&mut |i| {
            let (estado, p) = cuadro(i % 300);
            let c = ctx(i as f64 / 30.0, estado);
            nativa.step(&c, particulas::Paso { base: p[0], nivel: p[1], traga: p[2], baile: p[3] }, 1.0 / 30.0);
            nativa.paint(&mut g, &c, pinta(&p));
        });
        let us_w = medir(&mut |i| {
            let (estado, p) = cuadro(i % 300);
            let c = ctx(i as f64 / 30.0, estado);
            pieza.paso(&c, &p, None, 1.0 / 30.0);
            pieza.pintar(&mut g, &c, &p, None);
        });
        println!("partículas por cuadro: nativas {us_n:.1} µs · WASM {us_w:.1} µs ({:.0}×)", us_w / us_n);
        assert!(us_w < 2000.0, "{us_w} µs por cuadro: no entra en el presupuesto de 2 ms");
    }
}
