//! BAYMAX: for now it's the original buddy with his face instead of the blob; everything around
//! it still tells the state. The face (`crate::baymax::face`) is the full border, the dimmer
//! fill and the hollow eyes: they look toward where things happen, blink, close when
//! sleeping, the face lights up when speaking and turns red on an error. Offline only the
//! border is left, dotted. The reading line and the magnifier (searching) go over the face.
//!
//! To redo it from scratch, this file can get its own `Buddy` with its own pieces; the core
//! doesn't change.

use super::original::scene::Scene;
use super::original::{Center, Original};
use crate::core::color::{INK_FACE, INK_MAIN, INK_RED};
use crate::core::context::Context;
use crate::core::grid::Grid;

pub(crate) type Baymax = Original<Face>;

pub(crate) struct Face {
    /// Half the face, in body radii.
    size: f64,
}

impl Face {
    pub fn new() -> Self {
        Face { size: 1.8 }
    }
}

impl Center for Face {
    fn draw(&self, g: &mut Grid, ctx: &Context, e: &Scene) {
        let (p, a, t) = (e.p, e.mood, ctx.t);
        let half = (e.anchor.radius + e.breath + e.beat + e.voice * 0.5) * self.size;
        let open = if p.ripple > 0.5 { if a.level > 0.01 { a.level * 1.6 } else { (t * 5.0).sin().abs() } } else { 0.0 };
        let open = open.max(e.yawn);
        let magnifier = (p.lens > 0.01).then_some(e.magnifier);
        let (cx, cy, sx, sy) = (e.anchor.cx, e.anchor.cy, e.sx, e.sy);

        let lim = e.lim.max(1e-3);
        let gesture = crate::baymax::Gesture {
            closed: e.blink || p.pupil < 0.3,
            gaze: [(e.gaze[0] / lim).clamp(-1.0, 1.0), (e.gaze[1] / lim).clamp(-1.0, 1.0)],
        };
        let ink = if a.red > 0.35 { INK_RED } else { INK_FACE };
        let fill = if open > 0.4 { 2 } else { 1 };
        let du = g.du / half;
        for j in 0..g.dh {
            for i in 0..g.dw {
                let (x, y) = g.center(i, j);
                let (u, v) = ((x - cx) / sx / half, (y - cy) / sy / half);
                if u.abs() > 1.0 || v.abs() > 1.0 {
                    continue;
                }
                match crate::baymax::face(u, v, du, &gesture) {
                    0 => {}
                    3 if p.dashed > 0.5 => {
                        if (i + j) % 2 == 0 {
                            g.dot(i, j, 1, ink);
                        }
                    }
                    _ if p.dashed > 0.5 => {}
                    // Reading: the line that moves down the face.
                    _ if e.line.covers(y) => g.dot(i, j, 3, INK_MAIN),
                    _ if magnifier.is_some_and(|l| ((x - l.x).hypot(y - l.y) - l.r).abs() < 1.5 * g.du) => g.dot(i, j, 3, INK_MAIN),
                    3 => g.dot(i, j, 3, ink),
                    _ => g.dot(i, j, fill, ink),
                }
            }
        }
    }

    /// Dim tones go toward the background. In Auto, the state's color, like the usual core
    /// (thinking, searching, error…); with a color chosen in /core, that one. The face's fill
    /// goes light (it's white); when speaking, even more so.
    fn tones(&self, col: [f64; 3]) -> [[f64; 3]; 4] {
        let state = col.map(|x| x.round().clamp(0.0, 255.0) as u32);
        let state = state[0] << 16 | state[1] << 8 | state[2];
        let (background, c) = (crate::theme::background_rgb(), crate::baymax::core().color(state));
        let full = crate::baymax::toward(c, background, 0.0);
        [full, crate::baymax::toward(c, background, 0.38), crate::baymax::toward(c, background, 0.15), full]
    }
}
