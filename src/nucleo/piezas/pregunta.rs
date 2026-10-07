//! PREGUNTA (no te oigo): un «?» que flota junto al cuerpo encorvado.

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Pregunta {
    /// El signo, punto por punto (columna, fila).
    signo: [(i32, i32); 9],
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub droop: f64,
}

impl Pregunta {
    pub fn new() -> Self {
        Pregunta { signo: [(0, 4), (1, 5), (2, 5), (3, 4), (3, 3), (2, 2), (1, 1), (1, 0), (1, -2)] }
    }
}

impl Pieza for Pregunta {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, a) = (ctx.t, m.ancla);
        if m.droop > 0.5 {
            let (qx, qy) = (a.cx + 0.32, a.cy + a.base + 0.12 + 0.02 * (t * 3.0).sin());
            for (a, b) in self.signo {
                g.plot(qx + a as f64 * 0.025 - 0.04, qy + b as f64 * 0.03, 3, INK_MAIN);
            }
        }
    }
}
