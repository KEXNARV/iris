//! Dibujo suelto sobre el buffer: escribir alineado, líneas, medidores, esquinas y letras grandes.

use super::*;

/// Escribe recortando a `max` columnas y al borde del buffer; devuelve dónde terminó.
pub(in crate::ui) fn put(buf: &mut Buffer, x: u16, y: u16, s: &str, st: Style, max: u16) -> u16 {
    let a = buf.area;
    if y < a.y || y >= a.bottom() || x < a.x || x >= a.right() || max == 0 {
        return x;
    }
    buf.set_stringn(x, y, s, max as usize, st).0
}

/// Alineado a la derecha terminando en `xr` (incluido).
pub(in crate::ui) fn put_right(buf: &mut Buffer, xr: u16, y: u16, s: &str, st: Style) {
    let w = s.width() as u16;
    put(buf, (xr + 1).saturating_sub(w), y, s, st, w);
}

pub(in crate::ui) fn put_center(buf: &mut Buffer, r: Rect, y: u16, s: &str, st: Style) {
    let w = (s.width() as u16).min(r.width);
    put(buf, r.x + (r.width - w) / 2, y, s, st, r.width);
}

pub(in crate::ui) fn put_line_center(buf: &mut Buffer, r: Rect, y: u16, spans: &[(String, Style)]) {
    let w: u16 = spans.iter().map(|(s, _)| s.width() as u16).sum::<u16>().min(r.width);
    let mut x = r.x + (r.width - w) / 2;
    for (s, st) in spans {
        x = put(buf, x, y, s, *st, r.right().saturating_sub(x));
    }
}

pub(in crate::ui) fn hline(buf: &mut Buffer, x0: u16, x1: u16, y: u16, ch: &str, st: Style) {
    for x in x0..=x1 {
        put(buf, x, y, ch, st, 1);
    }
}

/// «ESPERANDO RESPUESTA...» → «E S P E R A N D O   R E S P U E S T A...»: los puntos que
/// laten no se separan.
pub(in crate::ui) fn spaced_title(s: &str) -> String {
    let base = s.trim_end_matches(['.', ' ']);
    format!("{}{}", spaced(base), &s[base.len()..])
}

pub(in crate::ui) fn spaced(s: &str) -> String {
    s.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
}

/// Una regla con título a la izquierda: «A H O R A ┈┈┈┈┈».
pub(in crate::ui) fn section(buf: &mut Buffer, x0: u16, x1: u16, y: u16, title: &str) {
    let x = put(buf, x0, y, &spaced(title), Style::new().fg(theme::faint()).bold(), x1.saturating_sub(x0) + 1);
    if x + 1 <= x1 {
        hline(buf, x + 1, x1, y, "┈", Style::new().fg(theme::dim()));
    }
}

/// Medidor `━━━━━────` de `w` columnas.
pub(in crate::ui) fn meter(buf: &mut Buffer, x: u16, y: u16, w: u16, frac: f64, on: Color) -> u16 {
    let n = ((frac.clamp(0.0, 1.0) * w as f64).round()) as u16;
    let x = put(buf, x, y, &"━".repeat(n as usize), Style::new().fg(on), n);
    put(buf, x, y, &"━".repeat((w - n) as usize), Style::new().fg(theme::dim()), w - n)
}

/// Esquinas de mira en vez de una caja: `┌──   ──┐ / └──   ──┘`.
pub(in crate::ui) fn corners(buf: &mut Buffer, r: Rect, st: Style, label: Option<&str>) {
    if r.width < 8 || r.height < 2 {
        return;
    }
    let (x0, x1, y0, y1) = (r.x, r.right() - 1, r.y, r.bottom() - 1);
    put(buf, x0, y0, "┌───", st, 4);
    put(buf, x1 - 3, y0, "───┐", st, 4);
    put(buf, x0, y1, "└───", st, 4);
    put(buf, x1 - 3, y1, "───┘", st, 4);
    if let Some(l) = label {
        put(buf, x0 + 5, y0, &format!(" {l} "), Style::new().fg(theme::faint()).bold(), r.width.saturating_sub(10));
    }
}

pub(in crate::ui) fn mix(a: Color, b: Color, f: f64) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * f).round() as u8;
            Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
        }
        _ => a,
    }
}

pub(in crate::ui) fn h_rows(r: Rect) -> usize {
    r.height as usize
}

pub(in crate::ui) fn rgb3(c: [f64; 3]) -> Color {
    Color::Rgb(c[0].round() as u8, c[1].round() as u8, c[2].round() as u8)
}

/// Letras de bloque de 3 filas para titulares. Sin tildes; lo que no está, no se dibuja.
pub(in crate::ui) fn big_glyph(c: char) -> Option<[&'static str; 3]> {
    Some(match c {
        'A' => ["█▀█", "█▀█", "▀ ▀"],
        'B' => ["█▀▄", "█▀▄", "▀▀ "],
        'C' => ["█▀▀", "█  ", "▀▀▀"],
        'D' => ["█▀▄", "█ █", "▀▀ "],
        'E' => ["█▀▀", "█▀ ", "▀▀▀"],
        'F' => ["█▀▀", "█▀ ", "▀  "],
        'G' => ["█▀▀", "█ █", "▀▀▀"],
        'H' => ["█ █", "█▀█", "▀ ▀"],
        'I' => ["▀█▀", " █ ", "▀▀▀"],
        'J' => ["  █", "  █", "▀▀ "],
        'K' => ["█ █", "█▀▄", "▀ ▀"],
        'L' => ["█  ", "█  ", "▀▀▀"],
        'M' => ["█▄ ▄█", "█ ▀ █", "▀   ▀"],
        'N' => ["█▄ █", "█ ▀█", "▀  ▀"],
        'O' => ["█▀█", "█ █", "▀▀▀"],
        'P' => ["█▀█", "█▀▀", "▀  "],
        'Q' => ["█▀█", "█ █", "▀▀▄"],
        'R' => ["█▀▄", "█▀▄", "▀ ▀"],
        'S' => ["█▀▀", "▀▀█", "▀▀▀"],
        'T' => ["▀█▀", " █ ", " ▀ "],
        'U' => ["█ █", "█ █", "▀▀▀"],
        'V' => ["█ █", "█ █", " ▀ "],
        'W' => ["█   █", "█ █ █", " ▀ ▀ "],
        'X' => ["█ █", "▄▀▄", "▀ ▀"],
        'Y' => ["█ █", "▀█▀", " ▀ "],
        'Z' => ["▀▀█", "▄▀ ", "▀▀▀"],
        '0' => ["█▀█", "█ █", "▀▀▀"],
        '1' => ["▄█ ", " █ ", "▀▀▀"],
        '2' => ["▀▀█", "█▀▀", "▀▀▀"],
        '3' => ["▀▀█", " ▀█", "▀▀▀"],
        '4' => ["█ █", "▀▀█", "  ▀"],
        '5' => ["█▀▀", "▀▀█", "▀▀▀"],
        '6' => ["█▀▀", "█▀█", "▀▀▀"],
        '7' => ["▀▀█", "  █", "  ▀"],
        '8' => ["█▀█", "█▀█", "▀▀▀"],
        '9' => ["█▀█", "▀▀█", "▀▀▀"],
        '%' => ["▀ █", " █ ", "█ ▄"],
        '$' => ["▄█▀", "▀█▄", "▀▀ "],
        '.' => [" ", " ", "▀"],
        ' ' => ["  ", "  ", "  "],
        _ => return None,
    })
}

/// El texto en letras de bloque, o `None` si no entra en `max` columnas.
pub(in crate::ui) fn big_text(s: &str, max: usize) -> Option<[String; 3]> {
    let s: String = s
        .trim_end_matches(['.', ' '])
        .to_uppercase()
        .chars()
        .map(|c| match c {
            'Á' => 'A',
            'É' => 'E',
            'Í' => 'I',
            'Ó' => 'O',
            'Ú' | 'Ü' => 'U',
            'Ñ' => 'N',
            c => c,
        })
        .collect();
    let mut rows = [String::new(), String::new(), String::new()];
    for (i, c) in s.chars().filter_map(big_glyph).enumerate() {
        for k in 0..3 {
            if i > 0 {
                rows[k].push(' ');
            }
            rows[k].push_str(c[k]);
        }
    }
    (rows[0].width() <= max && !rows[0].is_empty()).then_some(rows)
}
