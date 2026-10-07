//! PUNTADAS (editando): piezas que entran desde la escala hacia el contorno.

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Puntadas {
    cuantas: usize,
    /// Desde dónde entran.
    desde: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub stitch: f64,
}

impl Puntadas {
    pub fn new() -> Self {
        Puntadas { cuantas: 3, desde: 0.84 }
    }
}

impl Pieza for Puntadas {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, base) = (ctx.t, m.ancla.base);
        if m.stitch > 0.01 {
            for k in 0..self.cuantas {
                let v = t * 0.9 + k as f64 / self.cuantas as f64;
                let a = k as f64 * 2.4 + v.floor() * 1.7;
                let r = self.desde - v.fract() * (self.desde - base - 0.06);
                let (x, y) = m.ancla.punto(r, a);
                g.plot(x, y, 3, INK_MAIN);
                let (x, y) = m.ancla.punto(r + 0.04, a);
                g.plot(x, y, 2, INK_MAIN);
            }
        }
    }
}
