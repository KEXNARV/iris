//! Para escribir piezas de Iris: lo que se dibuja en el núcleo, cargado como plugin en
//! WebAssembly. Una pieza implementa [`Pieza`] y se exporta con [`exportar!`]:
//!
//! ```ignore
//! use iris_pieza::*;
//!
//! struct Antena { chispa: f64 }
//!
//! impl Pieza for Antena {
//!     const ESPACIO: Espacio = Espacio::Particulas;
//!     fn nueva(_azar: &mut Azar) -> Self { Antena { chispa: 0.0 } }
//!     fn paso(&mut self, _ctx: &Contexto, dt: f64) { self.chispa = (self.chispa - dt * 2.0).max(0.0); }
//!     fn evento(&mut self, ev: Evento, _ctx: &Contexto) { if ev == Evento::Error { self.chispa = 1.0; } }
//!     fn pintar(&self, g: &mut Rejilla, ctx: &Contexto) { g.plot(0.0, -ctx.ancla.radio * 1.4, 3, TINTA_ACENTO); }
//! }
//!
//! exportar!(Antena);
//! ```
//!
//! Se compila con `cargo build --release --target wasm32-unknown-unknown` (crate-type `cdylib`).
//!
//! El contrato con Iris ([`abi`]) es chico a propósito: Iris escribe la [`abi::Entrada`] en la
//! memoria de la pieza antes de cada llamada y lee de vuelta la lista de [`abi::Orden`]es de
//! dibujo. La pieza no tiene archivos, red ni reloj: solo lo que Iris le pasa.

pub mod abi;

pub use abi::{Espacio, Estado, Evento};

/// Tintas de la paleta del buddy (las mismas que usan las piezas de fábrica).
pub const TINTA_PRINCIPAL: u8 = 0;
pub const TINTA_VERDE: u8 = 1;
pub const TINTA_ROJA: u8 = 2;
pub const TINTA_CALIDA: u8 = 3;
pub const TINTA_ACENTO: u8 = 4;

/// Dónde está el cuerpo del buddy en este cuadro, para ubicarse alrededor de él.
#[derive(Clone, Copy, Debug)]
pub struct Ancla {
    pub cx: f64,
    pub cy: f64,
    /// El radio que tiene ahora (con arranque, desinflado y música).
    pub radio: f64,
    /// El radio que pide el estado, sin lo que lo mueve.
    pub base: f64,
    pub tinta: u8,
    pub borde: u8,
}

/// Lo que Iris le cuenta a la pieza en cada cuadro.
#[derive(Clone, Debug)]
pub struct Contexto {
    /// Segundos desde que nació el núcleo.
    pub t: f64,
    /// El estado que se ve y desde cuándo.
    pub estado: Estado,
    pub desde: f64,
    /// 1 normal, menos con `/calm`.
    pub calma: f64,
    /// Nivel de la voz y de la música (0..1) y contexto usado de Claude (0..1).
    pub voz: f64,
    pub musica: f64,
    pub contexto: f64,
    pub ancla: Ancla,
    /// Los parámetros propios del espacio (ver [`Espacio`]).
    pub p: [f64; 16],
}

impl Contexto {
    /// Segundos en el estado que se ve.
    pub fn st(&self) -> f64 {
        self.t - self.desde
    }
}

/// Lo que la pieza dibuja en este cuadro. Las coordenadas son las del mundo del núcleo: el
/// centro en (0, 0) y el borde a 1.
pub struct Rejilla {
    pub(crate) ordenes: Vec<abi::Orden>,
    /// El tamaño de un punto, en unidades del mundo.
    pub du: f64,
}

impl Rejilla {
    /// Un punto con su tono (1 apagado … 3 pleno) y su tinta. En la rejilla gana el más brillante.
    pub fn plot(&mut self, x: f64, y: f64, tono: u8, tinta: u8) {
        self.empujar(x, y, tono, tinta, abi::MODO_PLOT);
    }

    /// Solo donde no hay nada: para lo que va detrás de todo.
    pub fn plot_under(&mut self, x: f64, y: f64, tono: u8) {
        self.empujar(x, y, tono, 0, abi::MODO_DEBAJO);
    }

    /// Un punto a `r` del centro, en el ángulo `a`.
    pub fn polar(&mut self, r: f64, a: f64, tono: u8, tinta: u8) {
        self.plot(r * a.cos(), r * a.sin(), tono, tinta);
    }

    fn empujar(&mut self, x: f64, y: f64, tono: u8, tinta: u8, modo: u8) {
        if self.ordenes.len() < abi::MAX_ORDENES {
            self.ordenes.push(abi::Orden { x, y, tono, tinta, modo, _relleno: [0; 5] });
        }
    }
}

/// El azar del buddy: lo pide a Iris, así el orden en que se siembra es parte de cómo se ve.
pub struct Azar(());

impl Azar {
    /// Un número en 0..1.
    pub fn rand(&mut self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            abi::iris_azar()
        }
        #[cfg(not(target_arch = "wasm32"))]
        0.5
    }
}

/// Una pieza de Iris.
pub trait Pieza: Sized {
    /// A qué espacio va: qué reemplaza en el núcleo y qué recibe en [`Contexto::p`].
    const ESPACIO: Espacio;
    /// Nace. Lo que pida al azar sale del azar del buddy, en orden.
    fn nueva(azar: &mut Azar) -> Self;
    /// Avanza `dt` segundos.
    fn paso(&mut self, _ctx: &Contexto, _dt: f64) {}
    /// Algo que pasa de golpe.
    fn evento(&mut self, _ev: Evento, _ctx: &Contexto) {}
    /// Dibuja.
    fn pintar(&self, _g: &mut Rejilla, _ctx: &Contexto) {}
}

#[doc(hidden)]
pub mod __exportar {
    use super::*;
    use core::cell::UnsafeCell;

    /// Lo que comparten Iris y la pieza: la entrada que escribe Iris y las órdenes que lee.
    /// Una pieza corre en un solo hilo; esto solo se toca desde las funciones exportadas.
    pub struct Compartido<P> {
        pub entrada: UnsafeCell<abi::Entrada>,
        pub ordenes: UnsafeCell<Vec<abi::Orden>>,
        pub pieza: UnsafeCell<Option<P>>,
    }
    unsafe impl<P> Sync for Compartido<P> {}

    impl<P: Pieza> Compartido<P> {
        pub const fn nuevo() -> Self {
            Compartido { entrada: UnsafeCell::new(abi::Entrada::CERO), ordenes: UnsafeCell::new(Vec::new()), pieza: UnsafeCell::new(None) }
        }

        fn contexto(&self) -> Contexto {
            let e = unsafe { &*self.entrada.get() };
            Contexto {
                t: e.t,
                estado: Estado::de(e.estado),
                desde: e.desde,
                calma: e.calma,
                voz: e.voz,
                musica: e.musica,
                contexto: e.contexto,
                ancla: Ancla { cx: e.ancla_cx, cy: e.ancla_cy, radio: e.ancla_radio, base: e.ancla_base, tinta: e.ancla_tinta as u8, borde: e.ancla_borde as u8 },
                p: e.p,
            }
        }

        pub fn entrada(&self) -> u32 {
            self.entrada.get() as u32
        }

        pub fn nueva(&self) {
            unsafe { *self.pieza.get() = Some(P::nueva(&mut Azar(()))) };
        }

        pub fn paso(&self) {
            let ctx = self.contexto();
            let dt = unsafe { (*self.entrada.get()).dt };
            if let Some(p) = unsafe { &mut *self.pieza.get() } {
                p.paso(&ctx, dt);
            }
        }

        pub fn evento(&self, ev: u32) {
            let ctx = self.contexto();
            if let (Some(p), Some(ev)) = (unsafe { &mut *self.pieza.get() }, Evento::de(ev)) {
                p.evento(ev, &ctx);
            }
        }

        /// Dibuja y deja las órdenes donde Iris las lee: (dirección << 32) | cantidad.
        pub fn pintar(&self) -> u64 {
            let ctx = self.contexto();
            let du = unsafe { (*self.entrada.get()).du };
            let ordenes = unsafe { &mut *self.ordenes.get() };
            let mut g = Rejilla { ordenes: core::mem::take(ordenes), du };
            g.ordenes.clear();
            if let Some(p) = unsafe { &*self.pieza.get() } {
                p.pintar(&mut g, &ctx);
            }
            *ordenes = g.ordenes;
            ((ordenes.as_ptr() as u64) << 32) | ordenes.len() as u64
        }
    }
}

/// Exporta una [`Pieza`] con las funciones que busca Iris (ABI 1).
#[macro_export]
macro_rules! exportar {
    ($tipo:ty) => {
        static __IRIS: $crate::__exportar::Compartido<$tipo> = $crate::__exportar::Compartido::nuevo();

        #[unsafe(no_mangle)]
        pub extern "C" fn iris_abi() -> u32 {
            $crate::abi::VERSION
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_espacio() -> u32 {
            <$tipo as $crate::Pieza>::ESPACIO as u32
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_entrada() -> u32 {
            __IRIS.entrada()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_nueva() {
            __IRIS.nueva()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_paso() {
            __IRIS.paso()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_evento(ev: u32) {
            __IRIS.evento(ev)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn iris_pintar() -> u64 {
            __IRIS.pintar()
        }
    };
}
