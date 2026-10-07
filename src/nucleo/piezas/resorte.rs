//! RESORTE: el aplastamiento elástico. Cada cosa que pasa le da un empujón; rebota y se
//! asienta. El cuerpo se estira o se aplasta con lo que marque.

use super::Pieza;
use crate::nucleo::contexto::Contexto;

pub(crate) struct Resorte {
    /// Cuánto está aplastado: + más ancho que alto, − al revés.
    pub sq: f64,
    v: f64,
    rigidez: f64,
    freno: f64,
    /// Cuánto de la velocidad se vuelve aplastamiento por segundo.
    paso: f64,
    tope: f64,
}

impl Resorte {
    pub fn new() -> Self {
        Resorte { sq: 0.0, v: 0.0, rigidez: 140.0, freno: 9.0, paso: 0.06, tope: 0.18 }
    }

    /// Un empujón; con `/calm`, más suave.
    pub fn empujar(&mut self, v: f64, calma: f64) {
        self.v += v * calma;
    }
}

impl Pieza for Resorte {
    type Paso<'a> = ();
    type Pinta<'a> = ();

    fn step(&mut self, _ctx: &Contexto, _: (), dt: f64) {
        self.v += (-self.rigidez * self.sq - self.freno * self.v) * dt;
        self.sq = (self.sq + self.v * dt * self.paso).clamp(-self.tope, self.tope);
    }
}
