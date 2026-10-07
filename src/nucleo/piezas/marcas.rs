//! MARCAS: lo que espera, contado con marcas. La cola (mensajes escritos a mitad de turno,
//! satélites sólidos que esperan su lectura) y las tareas de TodoWrite (una marca grande por
//! tarea; la actual parpadea).

use std::f64::consts::{PI, TAU};

use super::Pieza;
use crate::nucleo::color::INK_MAIN;
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Cola {
    /// Cuántos mensajes se muestran como mucho.
    max: usize,
    /// A qué distancia del centro del cuerpo orbitan.
    radio: f64,
}

impl Cola {
    pub fn new() -> Self {
        Cola { max: 6, radio: 0.6 }
    }
}

impl Pieza for Cola {
    type Paso<'a> = ();
    type Pinta<'a> = &'a Ancla;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, ancla: &Ancla) {
        let t = ctx.t;
        for k in 0..ctx.sig.queue.min(self.max) {
            let a = PI / 2.0 + t * 0.4 + k as f64 * 0.5;
            for (dr, da) in [(0.0, 0.0), (0.03, 0.0), (0.0, 0.06), (0.03, 0.06)] {
                let (x, y) = ancla.punto(self.radio + dr, a + da);
                g.plot(x, y, 3, INK_MAIN);
            }
        }
    }
}

pub(crate) struct Tareas {
    max: usize,
    radios: [f64; 2],
}

impl Tareas {
    pub fn new() -> Self {
        Tareas { max: 24, radios: [0.82, 0.78] }
    }
}

impl Pieza for Tareas {
    type Paso<'a> = ();
    /// Cuánto se ven (planificando).
    type Pinta<'a> = f64;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, plan: f64) {
        let t = ctx.t;
        let (n_todo, done) = ctx.sig.todos;
        if plan > 0.5 && n_todo > 0 {
            for i in 0..n_todo.min(self.max) {
                let a = PI / 2.0 - i as f64 * TAU / n_todo.min(self.max) as f64;
                let tone = if i < done {
                    3
                } else if i == done && (t * 8.0).sin() > 0.0 {
                    3
                } else {
                    1
                };
                g.polar(self.radios[0], a, tone, INK_MAIN);
                g.polar(self.radios[1], a, tone, INK_MAIN);
            }
        }
    }
}
