//! Mouse selection inside the conversation. The terminal's grabs whole screen rows (core,
//! borders, activity); this one only sees the chat text.

use std::io::Write;
use std::process::{Command, Stdio};

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

/// A chat row as it is shown.
pub struct Row {
    pub text: String,
    /// Decoration columns at the start (indent, the code's `│ `, the notices' `· `).
    pub skip: usize,
    /// Continues the previous row because the paragraph didn't fit on one line.
    pub cont: bool,
}

/// The last thing drawn of the chat: where it is, all its rows and which is the first visible.
#[derive(Default)]
pub struct View {
    pub area: Rect,
    pub rows: Vec<Row>,
    pub first: usize,
}

impl View {
    /// How far up you can scroll: beyond that there is no more text.
    pub fn max_scroll(&self) -> usize {
        self.rows.len().saturating_sub(self.area.height as usize)
    }

    /// From screen to (conversation row, column), clamped to the chat area.
    pub fn locate(&self, (x, y): (u16, u16)) -> (usize, usize) {
        let a = self.area;
        let y = y.clamp(a.y, a.bottom().saturating_sub(1));
        let x = x.clamp(a.x, a.right().saturating_sub(1));
        (self.first + (y - a.y) as usize, (x - a.x) as usize)
    }
}

/// In conversation coordinates, not screen ones: when the chat scrolls the selection stays
/// stuck to the text.
pub struct Selection {
    pub anchor: (usize, usize),
    pub head: (usize, usize),
    /// Where the pointer is while dragging; when scrolling, the head is recomputed with it.
    pub pointer: Option<(u16, u16)>,
}

impl Selection {
    /// (start, end) in reading order.
    fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        if self.anchor <= self.head { (self.anchor, self.head) } else { (self.head, self.anchor) }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The screen cells to highlight, only from the visible rows.
    pub fn cells(&self, view: &View) -> Vec<(u16, u16)> {
        let ((r0, c0), (r1, c1)) = self.ordered();
        let a = view.area;
        let mut out = vec![];
        for (i, y) in (a.y..a.bottom()).enumerate() {
            let r = view.first + i;
            if r < r0 || r > r1 {
                continue;
            }
            let from = if r == r0 { c0 } else { 0 };
            let to = if r == r1 { c1 } else { a.width as usize - 1 };
            for c in from..=to.min(a.width as usize - 1) {
                out.push((a.x + c as u16, y));
            }
        }
        out
    }

    /// The selected text, without decoration and with broken paragraphs joined back together.
    pub fn text(&self, view: &View) -> String {
        let ((r0, c0), (r1, c1)) = self.ordered();
        let mut out = String::new();
        for r in r0..=r1 {
            let Some(row) = view.rows.get(r) else { break };
            let from = if r == r0 { c0 } else { 0 };
            let to = if r == r1 { c1 + 1 } else { usize::MAX };
            let piece = columns(&row.text, from.max(row.skip), to);
            if r > r0 {
                out.push(if row.cont { ' ' } else { '\n' });
            }
            out.push_str(piece.trim_end());
        }
        out.trim_matches('\n').to_string()
    }
}

/// What falls between columns `from` and `to` (screen width, not bytes).
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

/// To the Wayland clipboard; if there is no `wl-copy`, via OSC 52, which foot understands.
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

pub(crate) fn base64(data: &[u8]) -> String {
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

    fn view(first: usize) -> View {
        let row = |t: &str, skip, cont| Row { text: t.into(), skip, cont };
        View {
            area: Rect { x: 1, y: 1, width: 40, height: 2 },
            rows: vec![
                row("  IRIS", 2, false),
                row("  Un párrafo largo que no", 2, false),
                row("  cupo en una línea.", 2, true),
                row("  │ cargo build", 4, false),
            ],
            first,
        }
    }

    fn sel(anchor: (usize, usize), head: (usize, usize)) -> Selection {
        Selection { anchor, head, pointer: None }
    }

    #[test]
    fn strips_decoration_and_joins_paragraphs() {
        assert_eq!(sel((1, 0), (3, 39)).text(&view(0)), "Un párrafo largo que no cupo en una línea.\ncargo build");
    }

    #[test]
    fn keeps_columns_on_first_and_last_row() {
        // «párrafo» starts at column 5; «largo» ends at 17.
        assert_eq!(sel((1, 18), (1, 5)).text(&view(0)), "párrafo largo");
    }

    #[test]
    fn copies_what_scrolled_off_screen() {
        // Only rows 2 and 3 are visible, but the selection started at 1.
        let v = view(2);
        assert_eq!(v.locate((5, 1)), (2, 4));
        let s = sel((1, 0), v.locate((40, 2)));
        assert_eq!(s.text(&v), "Un párrafo largo que no cupo en una línea.\ncargo build");
        assert!(s.cells(&v).iter().all(|&(_, y)| y >= 1 && y <= 2));
    }

    #[test]
    fn base64_standard() {
        assert_eq!(base64(b"hola"), "aG9sYQ==");
        assert_eq!(base64("ñ".as_bytes()), "w7E=");
    }
}
