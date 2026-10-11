//! BAYMAX: the original recipe with his face instead of the blob (`recipe/baymax.toml`);
//! everything around it still tells the state. The face (`crate::baymax::face`) is the full border, the dimmer
//! fill and the hollow eyes: they look toward where things happen, blink, close when
//! sleeping, the face lights up when speaking and turns red on an error. Offline only the
//! border is left, dotted. The reading line and the magnifier (searching) go over the face.

use crate::core::color::{INK_FACE, INK_MAIN, INK_RED};
use crate::core::context::{Anchor, Context};
use crate::core::grid::Grid;
use crate::core::pieces::magnifier::Spot;
use crate::core::pieces::reading::ReadingLine;

/// What the face reads from the bus to draw itself.
pub(crate) struct Look {
    pub ripple: f64,
    pub level: f64,
    pub yawn: f64,
    pub lens: f64,
    pub magnifier: Spot,
    pub line: ReadingLine,
    pub anchor: Anchor,
    pub breath: f64,
    pub beat: f64,
    pub voice: f64,
    pub sx: f64,
    pub sy: f64,
    pub lim: f64,
    pub gaze: [f64; 2],
    pub blink: bool,
    pub pupil: f64,
    pub red: f64,
    pub dashed: f64,
}

pub(crate) struct Face {
    /// Half the face, in body radii.
    size: f64,
}

impl Face {
    pub fn new() -> Self {
        Face { size: 1.8 }
    }
}

impl Face {
    pub fn draw(&self, g: &mut Grid, ctx: &Context, e: &Look) {
        let t = ctx.t;
        let half = (e.anchor.radius + e.breath + e.beat + e.voice * 0.5) * self.size;
        let open = if e.ripple > 0.5 { if e.level > 0.01 { e.level * 1.6 } else { (t * 5.0).sin().abs() } } else { 0.0 };
        let open = open.max(e.yawn);
        let magnifier = (e.lens > 0.01).then_some(e.magnifier);
        let (cx, cy, sx, sy) = (e.anchor.cx, e.anchor.cy, e.sx, e.sy);

        let lim = e.lim.max(1e-3);
        let gesture = crate::baymax::Gesture {
            closed: e.blink || e.pupil < 0.3,
            gaze: [(e.gaze[0] / lim).clamp(-1.0, 1.0), (e.gaze[1] / lim).clamp(-1.0, 1.0)],
        };
        let ink = if e.red > 0.35 { INK_RED } else { INK_FACE };
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
                    3 if e.dashed > 0.5 => {
                        if (i + j) % 2 == 0 {
                            g.dot(i, j, 1, ink);
                        }
                    }
                    _ if e.dashed > 0.5 => {}
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
    pub fn tones(&self, col: [f64; 3]) -> [[f64; 3]; 4] {
        let state = col.map(|x| x.round().clamp(0.0, 255.0) as u32);
        let state = state[0] << 16 | state[1] << 8 | state[2];
        let (background, c) = (crate::theme::background_rgb(), crate::baymax::core().color(state));
        let full = crate::baymax::toward(c, background, 0.0);
        [full, crate::baymax::toward(c, background, 0.38), crate::baymax::toward(c, background, 0.15), full]
    }
}
