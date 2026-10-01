//! Codificador Sixel mínimo para el núcleo: una imagen de índices de paleta (0 = transparente)
//! con pocos colores fijos. Sin cuantizar ni difuminar: los puntos tienen colores exactos.
//!
//! Formato: `ESC P 0;1;0 q`, atributos de trama 1:1 con el tamaño, la paleta en RGB 0-100 y
//! la imagen en franjas de 6 filas; por cada color de la franja, un carácter por columna con
//! los 6 bits de esas filas, y las repeticiones comprimidas como `!n`.

use std::fmt::Write;

pub fn encode(img: &[u8], w: usize, h: usize, colors: &[[f64; 3]]) -> String {
    let mut out = String::with_capacity(w * h / 4);
    // P2 = 1: los píxeles sin color quedan transparentes (se ve lo que hay detrás).
    let _ = write!(out, "\x1bP0;1;0q\"1;1;{w};{h}");
    for (i, c) in colors.iter().enumerate() {
        let pct = |v: f64| (v / 255.0 * 100.0).round().clamp(0.0, 100.0) as u32;
        let _ = write!(out, "#{};2;{};{};{}", i + 1, pct(c[0]), pct(c[1]), pct(c[2]));
    }
    let n = colors.len();
    let mut cols = vec![vec![0u8; w]; n];
    let mut present = vec![false; n];
    for band in 0..h.div_ceil(6) {
        for c in cols.iter_mut() {
            c.fill(0);
        }
        present.fill(false);
        for k in 0..6 {
            let y = band * 6 + k;
            if y >= h {
                break;
            }
            let row = &img[y * w..(y + 1) * w];
            for (x, &p) in row.iter().enumerate() {
                if p != 0 {
                    let c = p as usize - 1;
                    cols[c][x] |= 1 << k;
                    present[c] = true;
                }
            }
        }
        let mut first = true;
        for c in 0..n {
            if !present[c] {
                continue;
            }
            if !first {
                out.push('$'); // volver al principio de la franja para el siguiente color
            }
            first = false;
            let _ = write!(out, "#{}", c + 1);
            // Lo vacío del final no hace falta mandarlo.
            let end = cols[c].iter().rposition(|&b| b != 0).map_or(0, |e| e + 1);
            let mut x = 0;
            while x < end {
                let b = cols[c][x];
                let mut run = 1;
                while x + run < end && cols[c][x + run] == b {
                    run += 1;
                }
                let ch = (63 + b) as char;
                if run > 3 {
                    let _ = write!(out, "!{run}{ch}");
                } else {
                    for _ in 0..run {
                        out.push(ch);
                    }
                }
                x += run;
            }
        }
        out.push('-');
    }
    out.push_str("\x1b\\");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codifica_un_punto() {
        // 2×7: un píxel del color 1 arriba a la izquierda, otro del 2 en la segunda franja.
        let mut img = vec![0u8; 2 * 7];
        img[0] = 1;
        img[6 * 2 + 1] = 2;
        let s = encode(&img, 2, 7, &[[255.0, 0.0, 0.0], [0.0, 0.0, 255.0]]);
        assert!(s.starts_with("\x1bP0;1;0q\"1;1;2;7#1;2;100;0;0#2;2;0;0;100"), "{s:?}");
        // Franja 1: color 1, bit 0 en la columna 0. Franja 2: color 2, bit 0 en la columna 1.
        assert!(s.contains("#1@-"), "{s:?}");
        assert!(s.contains("#2?@-"), "{s:?}");
        assert!(s.ends_with("\x1b\\"));
    }

    #[test]
    fn comprime_repeticiones() {
        let img = vec![1u8; 10];
        let s = encode(&img, 10, 1, &[[0.0, 0.0, 0.0]]);
        assert!(s.contains("#1!10@"), "{s:?}");
    }
}
