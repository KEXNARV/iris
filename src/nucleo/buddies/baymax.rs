//! BAYMAX: hoy es el buddy original con su cara en lugar del blob; lo de alrededor sigue
//! diciendo el estado. La cara (`crate::baymax::cara`) es el borde pleno, el relleno más
//! apagado y los ojos huecos: miran hacia donde pasan las cosas, parpadean, se cierran al
//! dormir, la cara se enciende al hablar y con un error se pone roja. Sin conexión queda solo
//! el borde, punteado. La línea de lectura y la lupa (buscando) van encima de la cara.
//!
//! Para rehacerlo desde cero, este archivo puede pasar a tener su propio `Buddy` con sus
//! piezas; el núcleo no cambia.

use super::original::escena::Escena;
use super::original::{Centro, Original};
use crate::nucleo::color::{INK_CARA, INK_MAIN, INK_RED};
use crate::nucleo::contexto::Contexto;
use crate::nucleo::rejilla::Grid;

pub(crate) type Baymax = Original<Cara>;

pub(crate) struct Cara {
    /// Media cara, en radios del cuerpo.
    tamaño: f64,
}

impl Cara {
    pub fn new() -> Self {
        Cara { tamaño: 1.8 }
    }
}

impl Centro for Cara {
    fn pintar(&self, g: &mut Grid, ctx: &Contexto, e: &Escena) {
        let (p, a, t) = (e.p, e.animo, ctx.t);
        let half = (e.ancla.radio + e.breath + e.beat + e.voice * 0.5) * self.tamaño;
        let open = if p.ripple > 0.5 { if a.level > 0.01 { a.level * 1.6 } else { (t * 5.0).sin().abs() } } else { 0.0 };
        let open = open.max(e.yawn);
        let lupa = (p.lens > 0.01).then_some(e.lupa);
        let (cx, cy, sx, sy) = (e.ancla.cx, e.ancla.cy, e.sx, e.sy);

        let lim = e.lim.max(1e-3);
        let gesto = crate::baymax::Gesto {
            cerrados: e.blink || p.pupil < 0.3,
            mirada: [(e.mirada[0] / lim).clamp(-1.0, 1.0), (e.mirada[1] / lim).clamp(-1.0, 1.0)],
        };
        let ink = if a.red > 0.35 { INK_RED } else { INK_CARA };
        let relleno = if open > 0.4 { 2 } else { 1 };
        let du = g.du / half;
        for j in 0..g.dh {
            for i in 0..g.dw {
                let (x, y) = g.center(i, j);
                let (u, v) = ((x - cx) / sx / half, (y - cy) / sy / half);
                if u.abs() > 1.0 || v.abs() > 1.0 {
                    continue;
                }
                match crate::baymax::cara(u, v, du, &gesto) {
                    0 => {}
                    3 if p.dashed > 0.5 => {
                        if (i + j) % 2 == 0 {
                            g.dot(i, j, 1, ink);
                        }
                    }
                    _ if p.dashed > 0.5 => {}
                    // Leyendo: la línea que baja por la cara.
                    _ if e.linea.cubre(y) => g.dot(i, j, 3, INK_MAIN),
                    _ if lupa.is_some_and(|l| ((x - l.x).hypot(y - l.y) - l.r).abs() < 1.5 * g.du) => g.dot(i, j, 3, INK_MAIN),
                    3 => g.dot(i, j, 3, ink),
                    _ => g.dot(i, j, relleno, ink),
                }
            }
        }
    }

    /// Los tonos apagados van hacia el fondo. En Auto, el color del estado, como el núcleo de
    /// siempre (pensando, buscando, error…); con un color elegido en /core, ese. El relleno
    /// de la cara va claro (es blanca); al hablar, más todavía.
    fn tonos(&self, col: [f64; 3]) -> [[f64; 3]; 4] {
        let estado = col.map(|x| x.round().clamp(0.0, 255.0) as u32);
        let estado = estado[0] << 16 | estado[1] << 8 | estado[2];
        let (fondo, c) = (crate::theme::fondo_rgb(), crate::baymax::nucleo().color(estado));
        let full = crate::baymax::hacia(c, fondo, 0.0);
        [full, crate::baymax::hacia(c, fondo, 0.38), crate::baymax::hacia(c, fondo, 0.15), full]
    }
}
