//! PARTICLES: the corona's motes, in three depth layers. They fall to the center while
//! listening, fly out while responding and float the rest of the time. A "camera" that sways
//! and follows the gaze a bit shifts them by how close they are, and that gives the parallax.
//! The far ones go behind everything (only where there's nothing); the near ones, in front.

use std::f64::consts::TAU;

use super::Piece;
use crate::core::rand::Rand;
use crate::core::color::INK_ACCENT;
use crate::core::context::Context;
use crate::core::state::State;
use crate::core::grid::Grid;

#[derive(Clone, Copy)]
struct Mote {
    a: f64,
    r: f64,
    s: f64,
    /// Depth: 0 at the back, 1 at the front. Near ones shine brighter, are bigger, move faster
    /// and shift more with the gaze: that's what gives the parallax.
    z: f64,
}

pub(crate) struct Motes {
    motes: Vec<Mote>,
    /// How many there are with everything on.
    count: usize,
}

pub(crate) struct Step {
    /// Body radius: the far ones live outside it.
    pub base: f64,
    /// Voice level (while listening they fall faster).
    pub level: f64,
    /// Swallowing (from listening to transcribing): they rush to the center.
    pub swallow: f64,
    /// When dancing they spin faster.
    pub dance: f64,
}

pub(crate) struct Paint {
    /// How visible they are (0..1) and how far into the boot.
    pub motes: f64,
    pub boot: f64,
    /// Compacting: they squeeze inward.
    pub press: f64,
    /// The gaze, which drags the camera along.
    pub gaze: [f64; 2],
    pub low: bool,
}

impl Motes {
    /// The startup ones, each in its layer and band. Consumes RNG even if they're emptied later
    /// (the kids): that way each one keeps the same random sequence as always.
    pub fn seed(rand: &mut Rand) -> Self {
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

    /// The "camera" sways slowly and follows the gaze a bit.
    fn camera(ctx: &Context, gaze: [f64; 2]) -> [f64; 2] {
        let c = ctx.calm();
        [0.12 * (ctx.t * 0.23).sin() * c - gaze[0] * 0.8, 0.07 * (ctx.t * 0.17 + 1.0).sin() * c - gaze[1] * 0.8]
    }
}

/// Where a mote lands: near ones shift a lot with the camera, far ones barely at all.
fn mote_pos(m: &Mote, cam: [f64; 2], press: f64, t: f64) -> (f64, f64) {
    let r = m.r * (1.0 - press * 0.25 * (1.0 + (t * 5.0).sin()));
    let shift = 0.1 + m.z * m.z * 1.9;
    (r * m.a.cos() + cam[0] * shift, r * m.a.sin() + cam[1] * shift)
}

impl Piece for Motes {
    type Step<'a> = Step;
    type Paint<'a> = Paint;

    /// They fall to the center while listening, fly out while responding, float otherwise.
    fn step(&mut self, ctx: &Context, m: Step, dt: f64) {
        let (t, calm) = (ctx.t, ctx.calm());
        let (r0, shown, level, swallow, groove) = (m.base, ctx.state, m.level, m.swallow, m.dance);
        for m in &mut self.motes {
            if swallow > 0.01 {
                m.r += (r0 - m.r) * (dt * 6.0).min(1.0);
                if m.r < r0 + 0.03 {
                    m.r = 0.86;
                }
            } else if shown == State::Listening {
                m.r -= dt * (0.18 + level * 0.5) * mote_speed(m.z);
                if m.r < r0 + 0.08 {
                    m.r = mote_band(m.z, r0).1;
                }
            } else if shown == State::Speaking {
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

    fn paint(&self, g: &mut Grid, ctx: &Context, m: Paint) {
        let t = ctx.t;
        let cam = Motes::camera(ctx, m.gaze);
        // Far and middle: last and only in empty cells, so the ring, the arcs and the body
        // cover them. The near ones come afterwards, on top of everything.
        if m.motes > 0.05 && !m.low {
            let n = (self.count as f64 * m.motes * m.boot).round() as usize;
            for mo in self.motes.iter().take(n).filter(|mo| mo.z < 0.66) {
                let (x, y) = mote_pos(mo, cam, m.press, t);
                g.plot_under(x, y, 1);
                if mo.z >= 0.33 {
                    g.plot_under(x + g.du, y, 1);
                }
            }
        }
        // Near: in front of everything, at full color, bigger and with a trail.
        if m.motes > 0.05 {
            let n = (self.count as f64 * m.motes * m.boot).round() as usize;
            let d = g.du;
            for mo in self.motes.iter().take(n).filter(|mo| mo.z >= 0.66) {
                let (x, y) = mote_pos(mo, cam, m.press, t);
                for (ox, oy) in [(0.0, 0.0), (d, 0.0), (0.0, d), (d, d)] {
                    g.plot(x + ox, y + oy, 3, INK_ACCENT);
                }
                // Trail behind them along their orbit.
                for k in 1..3 {
                    let back = Mote { a: mo.a - k as f64 * 0.05, ..*mo };
                    let (x, y) = mote_pos(&back, cam, m.press, t);
                    g.plot(x, y, if k == 1 { 2 } else { 1 }, INK_ACCENT);
                }
            }
        }
    }
}

/// Between which radii a mote lives according to its depth: far ones between the body and the
/// ring, near ones outside, over the scale.
fn mote_band(z: f64, blob_r: f64) -> (f64, f64) {
    if z < 0.33 {
        (blob_r + 0.12, 0.72)
    } else if z < 0.66 {
        (0.55, 0.88)
    } else {
        (0.70, 1.05)
    }
}

/// Relative speed of a mote: near ones go ~10× faster.
fn mote_speed(z: f64) -> f64 {
    0.25 + 2.4 * z * z
}
