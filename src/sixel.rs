//! Minimal Sixel encoder for the core: an image of palette indices (0 = transparent) with a
//! few fixed colors. No quantizing or dithering: the dots have exact colors.
//!
//! Format: `ESC P 0;1;0 q`, 1:1 raster attributes with the size, the palette in RGB 0-100 and
//! the image in 6-row bands; for each color in the band, one character per column with the
//! 6 bits of those rows, and repeats compressed as `!n`.

use std::fmt::Write;

pub fn encode(img: &[u8], w: usize, h: usize, colors: &[[f64; 3]]) -> String {
    let mut out = String::with_capacity(w * h / 4);
    // P2 = 1: uncolored pixels stay transparent (what's behind shows through).
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
                out.push('$'); // back to the start of the band for the next color
            }
            first = false;
            let _ = write!(out, "#{}", c + 1);
            // Trailing empty space doesn't need to be sent.
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
    fn encodes_a_dot() {
        // 2×7: one pixel of color 1 at the top left, another of color 2 in the second band.
        let mut img = vec![0u8; 2 * 7];
        img[0] = 1;
        img[6 * 2 + 1] = 2;
        let s = encode(&img, 2, 7, &[[255.0, 0.0, 0.0], [0.0, 0.0, 255.0]]);
        assert!(s.starts_with("\x1bP0;1;0q\"1;1;2;7#1;2;100;0;0#2;2;0;0;100"), "{s:?}");
        // Band 1: color 1, bit 0 in column 0. Band 2: color 2, bit 0 in column 1.
        assert!(s.contains("#1@-"), "{s:?}");
        assert!(s.contains("#2?@-"), "{s:?}");
        assert!(s.ends_with("\x1b\\"));
    }

    #[test]
    fn compresses_repeats() {
        let img = vec![1u8; 10];
        let s = encode(&img, 10, 1, &[[0.0, 0.0, 0.0]]);
        assert!(s.contains("#1!10@"), "{s:?}");
    }
}
