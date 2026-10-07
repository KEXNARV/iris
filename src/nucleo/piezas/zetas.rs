//! ZETAS (en reposo): tres zetas que suben desde el cuerpo dormido.

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Zetas {
    cuantas: usize,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub zzz: f64,
    pub low: bool,
}

impl Zetas {
    pub fn new() -> Self {
        Zetas { cuantas: 3 }
    }
}

impl Pieza for Zetas {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, a) = (ctx.t, m.ancla);
        if m.zzz > 0.01 && !m.low {
            for k in 0..self.cuantas {
                let u = (t * 0.22 + k as f64 / self.cuantas as f64).fract();
                let s = 0.03 + u * 0.04;
                let (zx, zy) = (a.cx + a.base * 0.6 + u * 0.32, a.cy + a.base * 0.5 + u * 0.38);
                let tone = if u < 0.5 { 2 } else { 1 };
                for q in -1..=1 {
                    g.plot(zx + q as f64 * s, zy + s, tone, INK_MAIN);
                    g.plot(zx + q as f64 * s, zy - s, tone, INK_MAIN);
                }
                g.plot(zx, zy, tone, INK_MAIN);
            }
        }
    }
}
