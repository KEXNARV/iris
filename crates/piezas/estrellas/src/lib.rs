//! Estrellas: las motas de Iris como estrellitas de cuatro puntas en tinta cálida, que giran
//! despacio alrededor del cuerpo y destellan con cada evento. Una pieza de ejemplo de la tienda.

use std::f64::consts::TAU;

use iris_pieza::*;

struct Estrella {
    a: f64,
    r: f64,
    v: f64,
    fase: f64,
}

struct Estrellas {
    estrellas: Vec<Estrella>,
    destello: f64,
}

impl Pieza for Estrellas {
    const ESPACIO: Espacio = Espacio::Particulas;

    fn nueva(azar: &mut Azar) -> Self {
        let estrellas = (0..14)
            .map(|k| Estrella { a: k as f64 * TAU / 14.0 + azar.rand() * 0.4, r: 0.62 + azar.rand() * 0.36, v: 0.05 + azar.rand() * 0.12, fase: azar.rand() * TAU })
            .collect();
        Estrellas { estrellas, destello: 0.0 }
    }

    fn paso(&mut self, ctx: &Contexto, dt: f64) {
        // Escuchando giran más rápido; con cada evento, un destello que se apaga solo.
        let prisa = if ctx.estado == Estado::Escuchando { 3.0 + ctx.p[1] * 4.0 } else { 1.0 };
        for e in &mut self.estrellas {
            e.a += dt * e.v * prisa * ctx.calma;
        }
        self.destello = (self.destello - dt * 1.5).max(0.0);
    }

    fn evento(&mut self, _ev: Evento, _ctx: &Contexto) {
        self.destello = 1.0;
    }

    fn pintar(&self, g: &mut Rejilla, ctx: &Contexto) {
        let (vis, boot) = (ctx.p[4], ctx.p[5]);
        if vis < 0.05 {
            return;
        }
        let n = (self.estrellas.len() as f64 * vis * boot).round() as usize;
        let d = g.du;
        for e in self.estrellas.iter().take(n) {
            let (x, y) = (e.r * e.a.cos(), e.r * e.a.sin());
            // Titila: el brazo de la estrella crece y se encoge, más con un destello.
            let brillo = 0.5 + 0.5 * (ctx.t * 2.0 + e.fase).sin();
            let brazo = 1 + (brillo * 2.0 + self.destello * 3.0) as i32;
            g.plot(x, y, 3, TINTA_CALIDA);
            for k in 1..=brazo {
                let tono = if k == brazo { 1 } else { 2 };
                let k = k as f64 * d;
                for (ox, oy) in [(k, 0.0), (-k, 0.0), (0.0, k), (0.0, -k)] {
                    g.plot(x + ox, y + oy, tono, TINTA_CALIDA);
                }
            }
        }
    }
}

exportar!(Estrellas);
