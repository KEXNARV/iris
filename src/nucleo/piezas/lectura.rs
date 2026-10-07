//! LECTURA (leyendo): una línea que sube y baja por el cuerpo, como un renglón que se lee. Es
//! una capa de adentro: la dibuja el cuerpo en sus puntos.

use super::Relleno;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Lectura {
    /// Hasta dónde llega, en radios del cuerpo, y su grosor.
    recorrido: f64,
    grosor: f64,
}

/// La línea en este cuadro.
pub(crate) struct Linea {
    y: f64,
    grosor: f64,
    activa: bool,
}

impl Lectura {
    pub fn new() -> Self {
        Lectura { recorrido: 0.8, grosor: 0.025 }
    }

    pub fn linea(&self, ctx: &Contexto, ancla: &Ancla, scan: f64) -> Linea {
        Linea { y: ancla.cy + ancla.base * self.recorrido * (ctx.st() * 1.6).cos(), grosor: self.grosor, activa: scan > 0.01 }
    }
}

impl Linea {
    /// ¿El punto a la altura `y` cae en la línea?
    pub fn cubre(&self, y: f64) -> bool {
        self.activa && (y - self.y).abs() < self.grosor
    }
}

impl Relleno for Linea {
    fn punto(&self, g: &mut Grid, i: usize, j: usize, _x: f64, y: f64) -> bool {
        if self.cubre(y) {
            g.dot(i, j, 3, INK_MAIN);
            return true;
        }
        false
    }
}
