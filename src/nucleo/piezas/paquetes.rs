//! PAQUETES (en la red): salen del cuerpo al anillo y vuelven; donde rebotan se enciende la
//! escala.

use std::f64::consts::TAU;

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Paquetes {
    cuantos: usize,
    /// Hasta dónde llegan, y los radios de la escala que encienden al rebotar.
    alcance: f64,
    rebote: [f64; 2],
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub packets: f64,
    pub low: bool,
}

impl Paquetes {
    pub fn new() -> Self {
        Paquetes { cuantos: 4, alcance: 0.84, rebote: [0.94, 0.90] }
    }
}

impl Pieza for Paquetes {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, base) = (ctx.t, m.ancla.base);
        if m.packets > 0.01 {
            for k in 0..self.cuantos {
                let trip = t * 0.8 + k as f64 / self.cuantos as f64;
                let (n, u) = (trip.floor(), trip.fract());
                let h = ((n * self.cuantos as f64 + k as f64) * 12.9898).sin() * 43758.5453;
                let a = h.fract().abs() * TAU;
                let out = if u < 0.5 { u * 2.0 } else { 2.0 - u * 2.0 };
                let r = base + 0.06 + out * (self.alcance - base) * m.packets;
                for s in 0..if m.low { 2 } else { 4 } {
                    let rs = r - if u < 0.5 { 1.0 } else { -1.0 } * s as f64 * 0.04;
                    if rs > base {
                        let (x, y) = m.ancla.punto(rs, a);
                        g.plot(x, y, if s == 0 { 3 } else if s < 2 { 2 } else { 1 }, INK_MAIN);
                    }
                }
                if (u - 0.5).abs() < 0.08 {
                    g.polar(self.rebote[0], a, 3, INK_MAIN);
                    g.polar(self.rebote[1], a, 3, INK_MAIN);
                }
            }
        }
    }
}
