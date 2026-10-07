//! Lo que Iris le da a cualquier buddy, sin saber nada de cómo se dibuja: la hora, el estado
//! que se ve y desde cuándo, las señales de la app y, si alguien se lo pide, hacia dónde
//! mirar. Más los avisos sueltos (eventos y subagentes) y el ancla, que cada buddy publica
//! en cada cuadro para que sus piezas se ubiquen alrededor de él.

use super::estado::{Event, Signals, State};

pub(crate) struct Contexto {
    /// Segundos desde que nació el núcleo.
    pub t: f64,
    /// El estado que se ve, que puede ir un poco detrás del pedido (ver `State::dwell`).
    pub estado: State,
    /// Cuándo empezó a verse.
    pub desde: f64,
    pub sig: Signals,
    /// Si está puesto, la mirada va hacia ahí (−1..1) en vez de lo que diga el estado: el
    /// hijo mira al principal cuando le manda algo.
    pub mira: Option<[f64; 2]>,
}

impl Contexto {
    /// Segundos en el estado que se ve.
    pub fn st(&self) -> f64 {
        self.t - self.desde
    }

    /// Menos movimiento (`/calm`).
    pub fn calma(&self) -> f64 {
        if self.sig.calm { 0.4 } else { 1.0 }
    }
}

/// Lo que pasa una vez y el buddy puede querer contestar.
#[derive(Clone, Copy)]
pub(crate) enum Aviso {
    Evento(Event),
    /// Nace un subagente.
    Nace(u64),
    /// El subagente hizo algo; `true` si fue un error.
    Pulso(u64, bool),
    /// Terminó el subagente; `true` si salió bien.
    Fin(u64, bool),
    /// El motor se fue y los subagentes con él.
    Fuera,
}

/// Dónde está el cuerpo de un buddy en este cuadro y con qué tintas, para que lo de alrededor
/// se ubique sin saber qué cuerpo es.
#[derive(Clone, Copy)]
pub(crate) struct Ancla {
    pub cx: f64,
    pub cy: f64,
    /// El radio que tiene ahora (con arranque, desinflado y música).
    pub radio: f64,
    /// El radio que pide el estado, sin lo que lo mueve.
    pub base: f64,
    /// La tinta del cuerpo: la del estado, o la roja con un error.
    pub tinta: u8,
    /// La tinta de lo de alrededor: el acento, o la roja con un error.
    pub borde: u8,
}

impl Ancla {
    /// El punto a `r` del centro del cuerpo, en el ángulo `a`.
    pub fn punto(&self, r: f64, a: f64) -> (f64, f64) {
        (self.cx + r * a.cos(), self.cy + r * a.sin())
    }
}
