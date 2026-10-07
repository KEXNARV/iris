//! CUERPO: el blob. Un contorno que respira, late con la voz, ondula según el estado y echa
//! púas con un error; por dentro, las capas que le pase el buddy (pupila, lupa, lectura,
//! líquido…) y debajo de todas una trama. Puede llevar una yema pegada (el brote).

use std::f64::consts::{PI, TAU};

use super::{Pieza, Relleno};
use crate::nucleo::contexto::Contexto;
use crate::nucleo::rejilla::Grid;

pub(crate) struct Blob {
    /// Grosor del contorno: al menos tantos puntos, aunque el panel sea chico.
    banda: f64,
    banda_puntos: f64,
    /// Hueco entre el contorno y lo de adentro.
    hueco: f64,
    /// Más allá de este radio no hay cuerpo (salvo la yema).
    alcance: f64,
}

/// La forma del contorno en este cuadro.
pub(crate) struct Forma {
    pub radius: f64,
    pub breath: f64,
    pub beat: f64,
    pub voice: f64,
    /// Ondulación, su fase y cuánto caos le suma.
    pub amp: f64,
    pub phase: f64,
    pub chaos: f64,
    /// Error: púas que salen.
    pub red: f64,
    pub t: f64,
}

impl Forma {
    /// El radio del contorno en el ángulo `th`.
    fn r_at(&self, th: f64) -> f64 {
        let Forma { radius, breath, beat, voice, amp, phase, chaos, red, t } = *self;
        radius + breath + beat + voice * (0.6 + 0.4 * (5.0 * th + t * 7.0).sin())
            + amp
                * (0.60 * (3.0 * th + phase).sin()
                    + 0.40 * (5.0 * th - phase * 1.6 + 1.0).sin()
                    + 0.30 * chaos * (7.0 * th + phase * 2.7 + 2.0).sin())
            + red * 0.09 * (11.0 * th + t * 20.0).sin().max(0.0)
    }
}

pub(crate) struct Pinta<'a> {
    pub cx: f64,
    pub cy: f64,
    /// Estirado por el resorte.
    pub sx: f64,
    pub sy: f64,
    pub forma: Forma,
    /// Sin conexión: contorno punteado y nada adentro.
    pub dashed: f64,
    /// Editando: un cursor que recorre el contorno.
    pub stitch: f64,
    pub yema: Option<(f64, f64, f64)>,
    pub tinta: u8,
    /// Las capas de adentro, de la de más arriba a la de más abajo. Debajo de todas, la trama.
    pub rellenos: &'a [&'a dyn Relleno],
}

impl Blob {
    pub fn new() -> Self {
        Blob { banda: 0.065, banda_puntos: 1.9, hueco: 0.035, alcance: 0.8 }
    }
}

impl Pieza for Blob {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (cx, cy, sx, sy) = (m.cx, m.cy, m.sx, m.sy);
        let cursor = ctx.t * 2.6;
        let band = self.banda.max(self.banda_puntos * g.du);
        let err = m.tinta;
        for j in 0..g.dh {
            for i in 0..g.dw {
                let (x, y) = g.center(i, j);
                let (dx, dy) = ((x - cx) / sx, (y - cy) / sy);
                let r = dx.hypot(dy);
                let mut in_bud = false;
                if let Some((bx, by, br)) = m.yema {
                    let e = br - (x - bx).hypot(y - by);
                    if (0.0..0.05).contains(&e) {
                        g.dot(i, j, 3, err);
                        continue;
                    }
                    in_bud = e >= 0.05;
                }
                if r > self.alcance && !in_bud {
                    continue;
                }
                let th = dy.atan2(dx);
                let d = m.forma.r_at(th) - r;
                if (0.0..band).contains(&d) {
                    if m.dashed > 0.5 && (th * 18.0).sin() <= 0.0 {
                        continue;
                    }
                    let mut tone = 3;
                    if m.stitch > 0.5 {
                        let da = ((th - cursor).rem_euclid(TAU) - PI).abs();
                        tone = if da > PI - 0.5 { 3 } else { 2 };
                    }
                    g.dot(i, j, tone, err);
                } else if (d > band + self.hueco || in_bud) && m.dashed < 0.5 {
                    if m.rellenos.iter().any(|c| c.punto(g, i, j, x, y)) {
                        continue;
                    }
                    if (i + j * 2) % 3 == 0 && j % 2 == 0 {
                        g.dot(i, j, 2, err);
                    }
                }
            }
        }
    }
}
