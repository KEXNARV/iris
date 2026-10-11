//! The grid of dots where everything is painted, in world coordinates. It knows nothing of
//! braille or Sixel: the output decides that.

use super::color::{INK_ACCENT, INK_MAIN};

/// Grid of dots. World coordinates: y ∈ [-1, 1] (x proportional); if the panel is taller
/// than wide, everything shrinks so the ring fits.
///
/// Each dot stores its tone (1-3, 0 = empty) and its ink. The output decides how they look: in
/// braille they are grouped 2×4 per cell and the cell takes the color of the brightest; as an
/// image (Sixel) each dot is a circle with its own color.
pub(crate) struct Grid {
    pub(crate) dw: usize,
    pub(crate) dh: usize,
    pub(crate) asp: f64,
    pub(crate) s: f64,
    /// Size of a dot in world units.
    pub(crate) du: f64,
    /// tone | ink << 2, per dot.
    pub(crate) dots: Vec<u8>,
    /// Blocks for `plot_under` (what is behind is only drawn where there is nothing): in braille,
    /// the cell; as an image, a square of a few dots.
    bw: usize,
    bh: usize,
    used: Vec<bool>,
    /// View for painting a child: the world shifts to (`ox`, `oy`) and scales by `k`, and the
    /// state's ink is painted with the child's (`ink`). Without a child: 0, 0, 1 and none.
    pub(crate) ox: f64,
    pub(crate) oy: f64,
    pub(crate) k: f64,
    ink: Option<u8>,
    du0: f64,
}

impl Grid {
    /// For braille: `cw`×`ch` cells of 2×4 dots.
    pub(crate) fn new(cw: usize, ch: usize) -> Self {
        Self::with_dots(cw * 2, ch * 4, 2, 4)
    }

    pub(crate) fn with_dots(dw: usize, dh: usize, bw: usize, bh: usize) -> Self {
        let asp = dw as f64 / dh as f64;
        let s = asp.min(1.0);
        let (bx, by) = (dw.div_ceil(bw), dh.div_ceil(bh));
        let du = 2.0 / (s * dh as f64);
        Grid { dw, dh, asp, s, du, dots: vec![0; dw * dh], bw, bh, used: vec![false; bx * by], ox: 0.0, oy: 0.0, k: 1.0, ink: None, du0: du }
    }

    pub(crate) fn view(&mut self, ox: f64, oy: f64, k: f64, ink: u8) {
        (self.ox, self.oy, self.k, self.ink) = (ox, oy, k, Some(ink));
        self.du = self.du0 / k;
    }

    pub(crate) fn unview(&mut self) {
        (self.ox, self.oy, self.k, self.ink) = (0.0, 0.0, 1.0, None);
        self.du = self.du0;
    }

    fn index(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let (x, y) = (self.ox + x * self.k, self.oy + y * self.k);
        let i = ((x * self.s / self.asp + 1.0) / 2.0 * (self.dw - 1) as f64).round();
        let j = ((1.0 - (y * self.s + 1.0) / 2.0) * (self.dh - 1) as f64).round();
        (i >= 0.0 && j >= 0.0 && i < self.dw as f64 && j < self.dh as f64).then_some((i as usize, j as usize))
    }

    pub(crate) fn plot(&mut self, x: f64, y: f64, tone: u8, ink: u8) {
        if let Some((i, j)) = self.index(x, y) {
            self.dot(i, j, tone, ink);
        }
    }

    /// Like `plot`, but only if its block is empty: for what stays behind (the motes,
    /// in the accent).
    pub(crate) fn plot_under(&mut self, x: f64, y: f64, tone: u8) {
        if let Some((i, j)) = self.index(x, y) {
            if !self.used[self.block(i, j)] {
                self.dot(i, j, tone, INK_ACCENT);
            }
        }
    }

    fn block(&self, i: usize, j: usize) -> usize {
        (j / self.bh) * self.dw.div_ceil(self.bw) + i / self.bw
    }

    pub(crate) fn polar(&mut self, r: f64, a: f64, tone: u8, ink: u8) {
        self.plot(r * a.cos(), r * a.sin(), tone, ink);
    }

    pub(crate) fn dot(&mut self, i: usize, j: usize, tone: u8, ink: u8) {
        let ink = match self.ink {
            Some(kid) if ink == INK_MAIN || ink == INK_ACCENT => kid,
            _ => ink,
        };
        let k = j * self.dw + i;
        let old = self.dots[k];
        let (ot, oi) = (old & 3, old >> 2);
        if tone > ot || (tone == ot && ink > oi) {
            self.dots[k] = tone | ink << 2;
        }
        let b = self.block(i, j);
        self.used[b] = true;
    }

    pub(crate) fn center(&self, i: usize, j: usize) -> (f64, f64) {
        let x = (i as f64 / (self.dw - 1) as f64 * 2.0 - 1.0) * self.asp / self.s;
        let y = (1.0 - j as f64 / (self.dh - 1) as f64 * 2.0) / self.s;
        ((x - self.ox) / self.k, (y - self.oy) / self.k)
    }
}
