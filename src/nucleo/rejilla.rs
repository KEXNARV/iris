//! La rejilla de puntos donde se pinta todo, en coordenadas del mundo. No sabe de braille ni
//! de Sixel: eso lo decide la salida.

use super::color::{INK_ACCENT, INK_MAIN};

/// Rejilla de puntos. Coordenadas del mundo: y ∈ [-1, 1] (x proporcional); si el panel es
/// más alto que ancho, se encoge todo para que el anillo entre.
///
/// Cada punto guarda su tono (1-3, 0 = vacío) y su tinta. La salida decide cómo se ven: en
/// braille se agrupan de a 2×4 por celda y la celda toma el color del más brillante; en
/// imagen (Sixel) cada punto es un círculo con su propio color.
pub(crate) struct Grid {
    pub(crate) dw: usize,
    pub(crate) dh: usize,
    asp: f64,
    s: f64,
    /// Tamaño de un punto en unidades del mundo.
    pub(crate) du: f64,
    /// tono | tinta << 2, por punto.
    pub(crate) dots: Vec<u8>,
    /// Bloques para `plot_under` (lo de atrás solo se dibuja donde no hay nada): en braille,
    /// la celda; en imagen, un cuadrado de unos pocos puntos.
    bw: usize,
    bh: usize,
    used: Vec<bool>,
    /// Vista para pintar un hijo: el mundo se corre a (`ox`, `oy`) y se escala por `k`, y la
    /// tinta del estado se pinta con la del hijo (`ink`). Sin hijo: 0, 0, 1 y ninguna.
    ox: f64,
    oy: f64,
    pub(crate) k: f64,
    ink: Option<u8>,
    du0: f64,
}

impl Grid {
    /// Para braille: `cw`×`ch` celdas de 2×4 puntos.
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

    /// Como `plot`, pero solo si su bloque está vacío: para lo que queda detrás (las motas,
    /// en el acento).
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
