//! PRENSAS (compactando): dos barras que aprietan el cuerpo desde arriba y abajo.

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Prensas {
    /// Media anchura de cada barra.
    ancho: f64,
}

pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    pub press: f64,
}

impl Prensas {
    pub fn new() -> Self {
        Prensas { ancho: 0.35 }
    }
}

impl Pieza for Prensas {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, a, press) = (ctx.t, m.ancla, m.press);
        if press > 0.01 {
            let gap = a.base + 0.08 + 0.1 * (0.5 + 0.5 * (t * 5.0).cos());
            let mut x = -self.ancho;
            while x <= self.ancho {
                g.plot(a.cx + x, a.cy + gap * press + (1.0 - press) * 0.8, 3, INK_MAIN);
                g.plot(a.cx + x, a.cy - gap * press - (1.0 - press) * 0.8, 3, INK_MAIN);
                x += 0.03;
            }
        }
    }
}
