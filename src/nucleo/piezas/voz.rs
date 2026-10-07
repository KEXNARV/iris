//! VOZ (respondiendo): ondas que salen del cuerpo mientras habla.

use std::f64::consts::TAU;

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Voz {
    ondas: usize,
    /// Hasta dónde llegan.
    alcance: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub ripple: f64,
}

impl Voz {
    pub fn new() -> Self {
        Voz { ondas: 2, alcance: 0.62 }
    }
}

impl Pieza for Voz {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, base) = (ctx.t, m.ancla.base);
        if m.ripple > 0.01 {
            for w in 0..self.ondas {
                let u = (t * 0.55 + w as f64 / self.ondas as f64).fract();
                if u > 0.9 {
                    continue;
                }
                let r = base + 0.1 + u * (self.alcance - base);
                let n = (40.0 + u * 30.0) as usize;
                for q in 0..n {
                    let (x, y) = m.ancla.punto(r, q as f64 * TAU / n as f64 + w as f64 * 0.2);
                    g.plot(x, y, if u < 0.4 { 2 } else { 1 }, INK_MAIN);
                }
            }
        }
    }
}
