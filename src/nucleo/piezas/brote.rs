//! BROTE (delegando): una yema que se separa del cuerpo, se aleja y vuelve, unida por un
//! tallo punteado. El tallo es de esta pieza; la yema la dibuja el cuerpo, que la funde
//! consigo.

use std::f64::consts::PI;

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Brote {
    /// Desde qué salida se ve el tallo.
    tallo: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub bud: f64,
    /// Fase del cuerpo: hacia dónde sale.
    pub fase: f64,
}

impl Brote {
    pub fn new() -> Self {
        Brote { tallo: 0.35 }
    }

    /// Ángulo, cuánto salió (0..1) y a qué distancia del centro está.
    fn donde(&self, ctx: &Contexto, m: &Pinta) -> (f64, f64, f64) {
        let a = m.fase * 0.9;
        let out = 0.5 + 0.5 * (ctx.t * 1.1 - PI / 2.0).sin();
        let d = m.ancla.base + (0.05 + out * 0.26) * m.bud;
        (a, out, d)
    }

    /// La yema: centro y radio, si hay brote.
    pub fn yema(&self, ctx: &Contexto, m: &Pinta) -> Option<(f64, f64, f64)> {
        (m.bud > 0.01).then(|| {
            let (a, _, d) = self.donde(ctx, m);
            (m.ancla.cx + d * a.cos(), m.ancla.cy + d * a.sin(), 0.10 + 0.02 * (ctx.t * 3.0).sin())
        })
    }
}

impl Pieza for Brote {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        if m.bud > 0.01 {
            let (a, out, d) = self.donde(ctx, &m);
            if out > self.tallo {
                for q in (0..6).step_by(2) {
                    let (x, y) = m.ancla.punto(m.ancla.base + q as f64 / 6.0 * (d - m.ancla.base), a);
                    g.plot(x, y, 1, INK_MAIN);
                }
            }
        }
    }
}
