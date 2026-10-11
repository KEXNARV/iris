//! Iris's particles (src/core/pieces/particles.rs), written as a store piece.
//! They are the test of the WASM host: they must draw the same as the native ones, frame by frame.

use std::f64::consts::TAU;

use iris_piece::*;

#[derive(Clone, Copy)]
struct Mote {
    a: f64,
    r: f64,
    s: f64,
    z: f64,
}

struct Motes {
    motes: Vec<Mote>,
    count: usize,
}

impl Piece for Motes {
    const SPACE: Space = Space::Particles;

    fn new(rand: &mut Rand) -> Self {
        let count = 24;
        let motes = (0..count)
            .map(|k| {
                let z = ((k % 3) as f64 / 2.0 + (rand.rand() - 0.5) * 0.2).clamp(0.0, 1.0);
                let (lo, hi) = mote_band(z, 0.40);
                Mote { a: k as f64 * TAU / count as f64 + rand.rand(), r: lo + rand.rand() * (hi - lo), s: rand.rand() - 0.5, z }
            })
            .collect();
        Motes { motes, count }
    }

    fn step(&mut self, ctx: &Context, dt: f64) {
        let (t, calm) = (ctx.t, ctx.calm);
        let (r0, level, swallow, groove) = (ctx.p[0], ctx.p[1], ctx.p[2], ctx.p[3]);
        for m in &mut self.motes {
            if swallow > 0.01 {
                m.r += (r0 - m.r) * (dt * 6.0).min(1.0);
                if m.r < r0 + 0.03 {
                    m.r = 0.86;
                }
            } else if ctx.state == State::Listening {
                m.r -= dt * (0.18 + level * 0.5) * mote_speed(m.z);
                if m.r < r0 + 0.08 {
                    m.r = mote_band(m.z, r0).1;
                }
            } else if ctx.state == State::Speaking {
                m.r += dt * 0.22 * mote_speed(m.z);
                if m.r > mote_band(m.z, r0).1 {
                    m.r = r0 + 0.1;
                }
            } else {
                let (lo, hi) = mote_band(m.z, r0);
                m.a += dt * 0.12 * (1.0 + m.s * 0.5) * mote_speed(m.z) * calm * (1.0 + groove * 4.0);
                m.r += (t * 0.7 + m.a * 3.0).sin() * dt * 0.02;
                m.r = m.r.clamp(lo, hi);
            }
        }
    }

    fn draw(&self, g: &mut Grid, ctx: &Context) {
        let t = ctx.t;
        let (motes, boot, press, low) = (ctx.p[4], ctx.p[5], ctx.p[6], ctx.p[9] > 0.5);
        let c = ctx.calm;
        let cam = [0.12 * (t * 0.23).sin() * c - ctx.p[7] * 0.8, 0.07 * (t * 0.17 + 1.0).sin() * c - ctx.p[8] * 0.8];
        if motes > 0.05 && !low {
            let n = (self.count as f64 * motes * boot).round() as usize;
            for mo in self.motes.iter().take(n).filter(|mo| mo.z < 0.66) {
                let (x, y) = mote_pos(mo, cam, press, t);
                g.plot_under(x, y, 1);
                if mo.z >= 0.33 {
                    g.plot_under(x + g.du, y, 1);
                }
            }
        }
        if motes > 0.05 {
            let n = (self.count as f64 * motes * boot).round() as usize;
            let d = g.du;
            for mo in self.motes.iter().take(n).filter(|mo| mo.z >= 0.66) {
                let (x, y) = mote_pos(mo, cam, press, t);
                for (ox, oy) in [(0.0, 0.0), (d, 0.0), (0.0, d), (d, d)] {
                    g.plot(x + ox, y + oy, 3, INK_ACCENT);
                }
                for k in 1..3 {
                    let back = Mote { a: mo.a - k as f64 * 0.05, ..*mo };
                    let (x, y) = mote_pos(&back, cam, press, t);
                    g.plot(x, y, if k == 1 { 2 } else { 1 }, INK_ACCENT);
                }
            }
        }
    }
}

fn mote_pos(m: &Mote, cam: [f64; 2], press: f64, t: f64) -> (f64, f64) {
    let r = m.r * (1.0 - press * 0.25 * (1.0 + (t * 5.0).sin()));
    let shift = 0.1 + m.z * m.z * 1.9;
    (r * m.a.cos() + cam[0] * shift, r * m.a.sin() + cam[1] * shift)
}

fn mote_band(z: f64, blob_r: f64) -> (f64, f64) {
    if z < 0.33 {
        (blob_r + 0.12, 0.72)
    } else if z < 0.66 {
        (0.55, 0.88)
    } else {
        (0.70, 1.05)
    }
}

fn mote_speed(z: f64) -> f64 {
    0.25 + 2.4 * z * z
}

export!(Motes);
