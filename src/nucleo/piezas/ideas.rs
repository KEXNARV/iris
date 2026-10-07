//! IDEAS (pensando): tres cometas que orbitan el cuerpo. Al irse a otro estado se hunden en él.

use std::f64::consts::TAU;

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Ideas {
    cuantas: usize,
    /// Puntos de cada cometa (los dos primeros, la cabeza).
    cola: usize,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub orbit: f64,
    /// Ondulación del contorno: las ideas orbitan por fuera de ella.
    pub amp: f64,
    /// Fase del cuerpo: marca el paso de la órbita.
    pub fase: f64,
    pub low: bool,
}

impl Ideas {
    pub fn new() -> Self {
        Ideas { cuantas: 3, cola: 7 }
    }
}

impl Pieza for Ideas {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, base) = (ctx.t, m.ancla.base);
        if m.orbit > 0.01 {
            for k in 0..self.cuantas {
                let a0 = m.fase * (1.1 + k as f64 * 0.25) + k as f64 * TAU / self.cuantas as f64;
                let rr = base + m.amp + 0.12 + k as f64 * 0.06 + 0.03 * (t * 1.7 + k as f64 * 2.0).sin();
                let r = base + (rr - base) * m.orbit;
                for s in 0..self.cola {
                    let a = a0 - s as f64 * 0.11;
                    let (x, y) = m.ancla.punto(r, a);
                    g.plot(x, y, if s < 2 { 3 } else { 2 }, INK_MAIN);
                    if s < 2 && !m.low {
                        for dr in [0.035, -0.035] {
                            let (x, y) = m.ancla.punto(r + dr, a);
                            g.plot(x, y, 3, INK_MAIN);
                        }
                    }
                }
            }
        }
    }
}
