//! LUPA (buscando): un anillo con mango que recorre el cuerpo. Por fuera es un dibujo más;
//! por dentro, una capa del cuerpo (el vidrio, en trama).

use std::f64::consts::TAU;

use super::{Pieza, Relleno};
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Lupa {
    radio: f64,
    /// Puntos del anillo y del mango.
    anillo: usize,
    mango: usize,
}

/// Dónde está la lupa: centro y radio.
#[derive(Clone, Copy)]
pub(crate) struct Donde {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub lens: f64,
    pub low: bool,
}

impl Lupa {
    pub fn new() -> Self {
        Lupa { radio: 0.15, anillo: 40, mango: 5 }
    }

    /// Por dónde va: un Lissajous lento sobre el cuerpo.
    pub fn donde(&self, ctx: &Contexto, ancla: &Ancla) -> Donde {
        let (st, a) = (ctx.st(), ancla);
        Donde { x: a.cx + a.base * 0.55 * (st * 1.3).sin(), y: a.cy + a.base * 0.45 * (st * 2.1 + 1.0).sin(), r: self.radio }
    }
}

impl Pieza for Lupa {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let Donde { x: lx, y: ly, r: lr } = self.donde(ctx, m.ancla);
        if m.lens > 0.01 {
            for q in 0..self.anillo {
                let a = q as f64 * TAU / self.anillo as f64;
                g.plot(lx + lr * a.cos(), ly + lr * a.sin(), 3, INK_MAIN);
            }
            if !m.low {
                for q in 0..self.mango {
                    let d = (lr + q as f64 * 0.025) * 0.7;
                    g.plot(lx + d, ly - d, 3, INK_MAIN);
                }
            }
        }
    }
}

/// El vidrio: lo de adentro de la lupa, en trama, tapando lo que hay debajo.
pub(crate) struct Vidrio {
    pub donde: Donde,
    pub activa: bool,
}

impl Relleno for Vidrio {
    fn punto(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool {
        if self.activa && (x - self.donde.x).hypot(y - self.donde.y) < self.donde.r {
            if (i + j) % 2 == 0 {
                g.dot(i, j, 3, INK_MAIN);
            }
            return true;
        }
        false
    }
}
