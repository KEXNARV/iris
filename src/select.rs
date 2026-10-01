//! Selección con el mouse dentro de la conversación. La de la terminal agarra filas enteras de
//! pantalla (núcleo, bordes, actividad); esta solo ve el texto del chat.

use std::io::Write;
use std::process::{Command, Stdio};

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

/// Una fila del chat tal como se ve.
pub struct Row {
    pub text: String,
    /// Columnas de decoración al inicio (sangría, `│ ` del código, `· ` de los avisos).
    pub skip: usize,
    /// Sigue a la fila anterior porque el párrafo no cupo en una línea.
    pub cont: bool,
}

/// Lo último que se dibujó del chat: dónde está y qué filas se ven.
#[derive(Default)]
pub struct View {
    pub area: Rect,
    pub rows: Vec<Row>,
}

/// En coordenadas de pantalla: dónde empezó el arrastre y dónde va.
pub struct Selection {
    pub anchor: (u16, u16),
    pub head: (u16, u16),
}

impl Selection {
    /// (inicio, fin) en orden de lectura.
    fn ordered(&self) -> ((u16, u16), (u16, u16)) {
        let key = |p: (u16, u16)| (p.1, p.0);
        if key(self.anchor) <= key(self.head) { (self.anchor, self.head) } else { (self.head, self.anchor) }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Las celdas a resaltar, recortadas al área del chat.
    pub fn cells(&self, area: Rect) -> Vec<(u16, u16)> {
        let ((x0, y0), (x1, y1)) = self.ordered();
        let mut out = vec![];
        for y in y0.max(area.y)..=y1.min(area.bottom().saturating_sub(1)) {
            let from = if y == y0 { x0 } else { area.x };
            let to = if y == y1 { x1 } else { area.right().saturating_sub(1) };
            for x in from.max(area.x)..=to.min(area.right().saturating_sub(1)) {
                out.push((x, y));
            }
        }
        out
    }

    /// El texto seleccionado, sin decoración y con los párrafos partidos vueltos a unir.
    pub fn text(&self, view: &View) -> String {
        let ((x0, y0), (x1, y1)) = self.ordered();
        let a = view.area;
        let mut out = String::new();
        for y in y0.max(a.y)..=y1.min(a.bottom().saturating_sub(1)) {
            let Some(row) = view.rows.get((y - a.y) as usize) else { break };
            let from = if y == y0 { x0.saturating_sub(a.x) as usize } else { 0 };
            let to = if y == y1 { x1.saturating_sub(a.x) as usize + 1 } else { usize::MAX };
            let piece = columns(&row.text, from.max(row.skip), to);
            let piece = piece.trim_end();
            if !out.is_empty() {
                out.push(if row.cont { ' ' } else { '\n' });
            }
            out.push_str(piece);
        }
        out.trim_matches('\n').to_string()
    }
}

/// Lo que cae entre las columnas `from` y `to` (ancho de pantalla, no bytes).
fn columns(s: &str, from: usize, to: usize) -> String {
    let mut col = 0;
    let mut out = String::new();
    for c in s.chars() {
        if col >= to {
            break;
        }
        if col >= from {
            out.push(c);
        }
        col += c.width().unwrap_or(0);
    }
    out
}

/// Al portapapeles de Wayland; si no hay `wl-copy`, por OSC 52, que foot entiende.
pub fn copy(text: &str) -> bool {
    let wl = Command::new("wl-copy").stdin(Stdio::piped()).spawn().and_then(|mut c| {
        c.stdin.take().unwrap().write_all(text.as_bytes())?;
        c.wait()
    });
    if wl.is_ok_and(|s| s.success()) {
        return true;
    }
    let mut out = std::io::stdout();
    write!(out, "\x1b]52;c;{}\x07", base64(text.as_bytes())).is_ok() && out.flush().is_ok()
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        let row = |t: &str, skip, cont| Row { text: t.into(), skip, cont };
        View {
            area: Rect { x: 1, y: 1, width: 40, height: 5 },
            rows: vec![
                row("  JARVIS", 2, false),
                row("  Un párrafo largo que no", 2, false),
                row("  cupo en una línea.", 2, true),
                row("  │ cargo build", 4, false),
            ],
        }
    }

    #[test]
    fn quita_decoracion_y_une_parrafos() {
        let s = Selection { anchor: (1, 2), head: (40, 4) };
        assert_eq!(s.text(&view()), "Un párrafo largo que no cupo en una línea.\ncargo build");
    }

    #[test]
    fn respeta_columnas_en_la_primera_y_ultima_fila() {
        // «párrafo» empieza en la columna 5 de la fila (x = 1 + 5); «largo» termina en la 18.
        let s = Selection { anchor: (19, 2), head: (6, 2) };
        assert_eq!(s.text(&view()), "párrafo largo");
    }

    #[test]
    fn base64_estandar() {
        assert_eq!(base64(b"hola"), "aG9sYQ==");
        assert_eq!(base64("ñ".as_bytes()), "w7E=");
    }
}
