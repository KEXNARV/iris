//! RADAR (ejecutando): un barrido que gira desde el cuerpo hasta los arcos.

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Radar {
    /// Rayos de la estela y cuánto se separan.
    rayos: usize,
    separa: f64,
    /// Hasta dónde llega.
    alcance: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub sweep: f64,
    pub giro: f64,
}

impl Radar {
    pub fn new() -> Self {
        Radar { rayos: 10, separa: 0.06, alcance: 0.78 }
    }
}

impl Pieza for Radar {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, _ctx: &Contexto, m: Pinta) {
        if m.sweep <= 0.01 {
            return;
        }
        let a0 = -m.giro * 1.3;
        for s in 0..self.rayos {
            let a = a0 + s as f64 * self.separa;
            let mut r = m.ancla.base + 0.14;
            while r < self.alcance {
                g.polar(r, a, if s < 2 { 3 } else if s < 5 { 2 } else { 1 }, INK_MAIN);
                r += 0.035;
            }
        }
    }
}
