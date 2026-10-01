//! Miniaturas de las imágenes que le mandas, para verlas en el chat (en foot, con Sixel).
//! Se decodifican una vez y se guardan chicas; la versión Sixel se arma para el tamaño de
//! celda actual y queda en caché.

use std::cell::RefCell;
use std::rc::Rc;

/// Lo más grande que se dibuja en el chat, en celdas.
pub const MAX_COLS: u16 = 48;
pub const MAX_ROWS: u16 = 10;

pub struct Thumb {
    rgba: Vec<u8>,
    w: u32,
    h: u32,
    cache: RefCell<Option<((u16, u16, u16, u16, u16, u16), String)>>,
}

/// Decodifica una PNG, JPG, WebP o GIF y la guarda achicada (como mucho 640×400).
pub fn from_bytes(raw: &[u8]) -> Option<Rc<Thumb>> {
    let img = image::load_from_memory(raw).ok()?;
    let img = if img.width() > 640 || img.height() > 400 { img.thumbnail(640, 400) } else { img };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(Rc::new(Thumb { rgba: rgba.into_raw(), w, h, cache: RefCell::new(None) }))
}

/// Para las imágenes de una sesión retomada, que el transcript guarda en base64.
pub fn from_base64(data: &str) -> Option<Rc<Thumb>> {
    from_bytes(&decode_base64(data)?)
}

impl Thumb {
    /// Cuántas celdas ocupa, sin pasar de `max_cols`×`MAX_ROWS` ni agrandarse.
    pub fn cells(&self, cell: (u16, u16), max_cols: u16) -> (u16, u16) {
        let (cw, ch) = (cell.0.max(1) as f64, cell.1.max(1) as f64);
        let k = (max_cols.min(MAX_COLS) as f64 * cw / self.w as f64)
            .min(MAX_ROWS as f64 * ch / self.h as f64)
            .min(1.0);
        let (pw, ph) = (self.w as f64 * k, self.h as f64 * k);
        // El margen evita que 220 / 22 = 10,0000001 redondee a 11 filas.
        (((pw / cw) - 1e-6).ceil().max(1.0) as u16, ((ph / ch) - 1e-6).ceil().max(1.0) as u16)
    }

    /// La imagen en Sixel para ocupar `cols`×`rows` celdas, mostrando solo las filas de celda
    /// `from..to` (cuando el chat la corta por arriba o por abajo).
    pub fn sixel(&self, cell: (u16, u16), cols: u16, rows: u16, from: u16, to: u16) -> String {
        let key = (cell.0, cell.1, cols, rows, from, to);
        if let Some((k, s)) = &*self.cache.borrow() {
            if *k == key {
                return s.clone();
            }
        }
        let (bw, bh) = (cols as f64 * cell.0 as f64, rows as f64 * cell.1 as f64);
        let k = (bw / self.w as f64).min(bh / self.h as f64).min(1.0);
        let (w, h) = (((self.w as f64 * k) as usize).max(1), ((self.h as f64 * k) as usize).max(1));
        // Muestreo simple al tamaño final y paleta de 6×6×6 colores (0 = transparente).
        let mut idx = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let sx = (x as f64 / k) as usize;
                let sy = (y as f64 / k) as usize;
                let o = (sy.min(self.h as usize - 1) * self.w as usize + sx.min(self.w as usize - 1)) * 4;
                let p = &self.rgba[o..o + 4];
                if p[3] < 128 {
                    continue;
                }
                let q = |v: u8| ((v as u32 * 5 + 127) / 255) as u8;
                idx[y * w + x] = 1 + q(p[0]) * 36 + q(p[1]) * 6 + q(p[2]);
            }
        }
        let colors: Vec<[f64; 3]> = (0..216u32)
            .map(|i| [(i / 36) as f64 * 51.0, (i / 6 % 6) as f64 * 51.0, (i % 6) as f64 * 51.0])
            .collect();
        // Recorte vertical en filas de celda.
        let (y0, y1) = ((from as usize * cell.1 as usize).min(h), (to as usize * cell.1 as usize).min(h));
        let (y0, y1) = if y1 > y0 { (y0, y1) } else { (0, h) };
        let s = crate::sixel::encode(&idx[y0 * w..y1 * w], w, y1 - y0, &colors);
        *self.cache.borrow_mut() = Some((key, s.clone()));
        s
    }
}

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        let Some(v) = val(c) else {
            if c.is_ascii_whitespace() {
                continue;
            }
            return None;
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_ida_y_vuelta() {
        let raw = b"Hola, Kevin \xF0\x9F\x98\x80";
        assert_eq!(decode_base64(&crate::select::base64(raw)).as_deref(), Some(&raw[..]));
    }

    #[test]
    fn la_miniatura_entra_en_su_lugar() {
        let t = Thumb { rgba: vec![255; 800 * 400 * 4], w: 800, h: 400, cache: RefCell::new(None) };
        let (c, r) = t.cells((10, 22), 80);
        assert!(c <= MAX_COLS && r <= MAX_ROWS, "{c}×{r}");
        assert!(t.sixel((10, 22), c, r, 0, r).starts_with("\x1bP0;1;0q"));
        // Recortada a sus dos últimas filas de celda: 44 píxeles de alto.
        assert!(t.sixel((10, 22), c, r, r - 2, r).contains(";44#"));
    }
}
