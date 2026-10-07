//! ARCOS: una corona de arcos que gira alrededor del cuerpo. El buddy original tiene dos:
//! tres largos por fuera que se ralean al apagarse y siguen el arranque, y seis cortos por
//! dentro, en sentido contrario, que se acortan.

use std::f64::consts::TAU;

use super::Pieza;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Arcos {
    cuantos: usize,
    radio: f64,
    /// Puntos por arco, encendido del todo.
    puntos: f64,
    /// Lo que mide cada arco, en radianes.
    tramo: f64,
    /// Cuánto gira respecto del giro del buddy (negativo: al revés).
    giro: f64,
    tono: u8,
    /// Largos: siguen el largo que pide el estado y el arranque, y al apagarse se ralean.
    /// Si no, al apagarse se acortan.
    largos: bool,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    /// Giro de lo de alrededor.
    pub giro: f64,
    /// Cuánto se ven (0..1) y qué largo piden.
    pub arcs: f64,
    pub arc_len: f64,
    pub boot: f64,
}

impl Arcos {
    /// Los de afuera del original.
    pub fn afuera() -> Self {
        Arcos { cuantos: 3, radio: 0.80, puntos: 60.0, tramo: TAU / 5.0, giro: 1.0, tono: 2, largos: true }
    }

    /// Los de adentro del original.
    pub fn adentro() -> Self {
        Arcos { cuantos: 6, radio: 0.70, puntos: 14.0, tramo: TAU / 16.0, giro: -1.6, tono: 1, largos: false }
    }
}

impl Pieza for Arcos {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, _ctx: &Contexto, m: Pinta) {
        if m.arcs * m.boot <= 0.05 {
            return;
        }
        let n = if self.largos {
            (self.puntos * m.arc_len * m.arcs * (m.boot * 1.4 - 0.4).max(0.0)).round() as usize
        } else {
            (self.puntos * m.arcs).round() as usize
        };
        for k in 0..self.cuantos {
            for q in 0..n {
                let a = if self.largos {
                    m.giro * self.giro + k as f64 * TAU / self.cuantos as f64 + self.tramo * m.arc_len * q as f64 / n.max(1) as f64
                } else {
                    m.giro * self.giro + k as f64 * TAU / self.cuantos as f64 + self.tramo * q as f64 / self.puntos
                };
                g.polar(self.radio, a, self.tono, m.ancla.borde);
            }
        }
    }
}
