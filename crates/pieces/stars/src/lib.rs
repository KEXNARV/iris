//! Stars: Iris's motes as little four-pointed stars in warm ink, slowly circling the body
//! and flashing with every event. An example piece for the store.

use std::f64::consts::TAU;

use iris_piece::*;

struct Star {
    a: f64,
    r: f64,
    v: f64,
    phase: f64,
}

struct Stars {
    stars: Vec<Star>,
    flash: f64,
}

impl Piece for Stars {
    const SPACE: Space = Space::Particles;

    fn new(rand: &mut Rand) -> Self {
        let stars = (0..14)
            .map(|k| Star { a: k as f64 * TAU / 14.0 + rand.rand() * 0.4, r: 0.62 + rand.rand() * 0.36, v: 0.05 + rand.rand() * 0.12, phase: rand.rand() * TAU })
            .collect();
        Stars { stars, flash: 0.0 }
    }

    fn step(&mut self, ctx: &Context, dt: f64) {
        // While listening they spin faster; every event triggers a flash that fades on its own.
        let hurry = if ctx.state == State::Listening { 3.0 + ctx.p[1] * 4.0 } else { 1.0 };
        for e in &mut self.stars {
            e.a += dt * e.v * hurry * ctx.calm;
        }
        self.flash = (self.flash - dt * 1.5).max(0.0);
    }

    fn event(&mut self, _ev: Event, _ctx: &Context) {
        self.flash = 1.0;
    }

    fn draw(&self, g: &mut Grid, ctx: &Context) {
        let (vis, boot) = (ctx.p[4], ctx.p[5]);
        if vis < 0.05 {
            return;
        }
        let n = (self.stars.len() as f64 * vis * boot).round() as usize;
        let d = g.du;
        for e in self.stars.iter().take(n) {
            let (x, y) = (e.r * e.a.cos(), e.r * e.a.sin());
            // Twinkle: the star's arm grows and shrinks, more so during a flash.
            let brightness = 0.5 + 0.5 * (ctx.t * 2.0 + e.phase).sin();
            let arm = 1 + (brightness * 2.0 + self.flash * 3.0) as i32;
            g.plot(x, y, 3, INK_WARM);
            for k in 1..=arm {
                let tone = if k == arm { 1 } else { 2 };
                let k = k as f64 * d;
                for (ox, oy) in [(k, 0.0), (-k, 0.0), (0.0, k), (0.0, -k)] {
                    g.plot(x + ox, y + oy, tone, INK_WARM);
                }
            }
        }
    }
}

export!(Stars);
