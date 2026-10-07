//! LÍQUIDO: el contexto usado, como un líquido que llena el cuerpo desde abajo, con una ola
//! en la superficie. Pasando el umbral se pone ámbar. Es una capa de adentro del cuerpo.

use super::Relleno;
use crate::nucleo::color::INK_WARM;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Liquido {
    /// Desde cuánto contexto va en ámbar.
    alerta: f64,
    /// Alto de la ola.
    ola: f64,
}

/// El líquido en este cuadro.
pub(crate) struct Nivel {
    ctx: f64,
    /// Altura de la superficie, sin la ola.
    tope: f64,
    ola: f64,
    t: f64,
    tinta: u8,
}

impl Liquido {
    pub fn new() -> Self {
        Liquido { alerta: 0.8, ola: 0.02 }
    }

    pub fn nivel(&self, ctx: &Contexto, ancla: &Ancla) -> Nivel {
        let c = ctx.sig.ctx.clamp(0.0, 1.0);
        let tinta = if c > self.alerta { INK_WARM } else { ancla.tinta };
        Nivel { ctx: c, tope: ancla.cy - ancla.radio + 2.0 * ancla.radio * c, ola: self.ola, t: ctx.t, tinta }
    }
}

impl Relleno for Nivel {
    fn punto(&self, g: &mut Grid, i: usize, j: usize, x: f64, y: f64) -> bool {
        if self.ctx > 0.01 && y < self.tope + self.ola * (x * 14.0 + self.t * 3.0).sin() {
            if (i + j) % 2 == 0 {
                g.dot(i, j, 2, self.tinta);
            }
            return true;
        }
        false
    }
}
