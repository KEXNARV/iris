//! GIT: un grafo que crece debajo del cuerpo: tronco que sube, una rama y nodos que aparecen
//! con un pop.

use std::f64::consts::TAU;

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Git {
    /// Lo que tarda en crecer entero.
    crece: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub git: f64,
}

impl Git {
    pub fn new() -> Self {
        Git { crece: 2.2 }
    }
}

impl Pieza for Git {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (st, a) = (ctx.st(), m.ancla);
        if m.git > 0.01 {
            let gx = a.cx + 0.1;
            let base = a.cy + a.base + 0.02;
            let grow2 = (st / self.crece).min(1.0);
            let mut y = 0.0;
            while y < 0.42 * grow2 {
                g.plot(gx, base + y, 2, INK_MAIN);
                y += 0.03;
            }
            let mut u = 0.0;
            while u < (grow2 - 0.35).max(0.0) / 0.65 {
                g.plot(gx - u * 0.22, base + 0.12 + u * 0.22, 2, INK_MAIN);
                u += 0.04;
            }
            for (nx, ny, at) in [(gx, base + 0.10, 0.2), (gx, base + 0.26, 0.5), (gx - 0.22, base + 0.34, 0.8), (gx, base + 0.40, 0.95)] {
                if grow2 > at {
                    let pop = ((grow2 - at) * 8.0).min(1.0);
                    let rr = 0.035 + 0.06 * (1.0 - pop).max(0.0);
                    for q in 0..10 {
                        let a = q as f64 * TAU / 10.0;
                        g.plot(nx + rr * a.cos(), ny + rr * a.sin(), 3, INK_MAIN);
                    }
                }
            }
        }
    }
}
