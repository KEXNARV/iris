//! Edición de la orden: cursor, saltos por palabra, borrar, y el historial de lo que enviaste
//! (con ↑ ↓, como en un shell). El cursor cuenta caracteres, no bytes: la entrada tiene tildes.

use std::path::PathBuf;

/// Byte donde empieza el carácter número `c` (o el final).
fn byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

fn len(s: &str) -> usize {
    s.chars().count()
}

pub fn insert(s: &mut String, cur: &mut usize, text: &str) {
    *cur = (*cur).min(len(s));
    s.insert_str(byte(s, *cur), text);
    *cur += len(text);
}

pub fn backspace(s: &mut String, cur: &mut usize) {
    *cur = (*cur).min(len(s));
    if *cur > 0 {
        let b = byte(s, *cur - 1);
        s.remove(b);
        *cur -= 1;
    }
}

pub fn delete(s: &mut String, cur: &mut usize) {
    *cur = (*cur).min(len(s));
    if *cur < len(s) {
        let b = byte(s, *cur);
        s.remove(b);
    }
}

pub fn left(s: &str, cur: &mut usize) {
    *cur = (*cur).min(len(s)).saturating_sub(1);
}

pub fn right(s: &str, cur: &mut usize) {
    *cur = (*cur + 1).min(len(s));
}

/// Al principio de la palabra anterior (saltando los espacios de por medio).
pub fn word_left(s: &str, cur: &mut usize) {
    let chars: Vec<char> = s.chars().collect();
    let mut i = (*cur).min(chars.len());
    while i > 0 && !chars[i - 1].is_alphanumeric() {
        i -= 1;
    }
    while i > 0 && chars[i - 1].is_alphanumeric() {
        i -= 1;
    }
    *cur = i;
}

/// Al final de la palabra siguiente.
pub fn word_right(s: &str, cur: &mut usize) {
    let chars: Vec<char> = s.chars().collect();
    let mut i = (*cur).min(chars.len());
    while i < chars.len() && !chars[i].is_alphanumeric() {
        i += 1;
    }
    while i < chars.len() && chars[i].is_alphanumeric() {
        i += 1;
    }
    *cur = i;
}

/// Ctrl+W: borra la palabra antes del cursor.
pub fn kill_word(s: &mut String, cur: &mut usize) {
    let end = (*cur).min(len(s));
    let mut start = end;
    word_left(s, &mut start);
    s.replace_range(byte(s, start)..byte(s, end), "");
    *cur = start;
}

/// Ctrl+K: borra desde el cursor hasta el final.
pub fn kill_to_end(s: &mut String, cur: &mut usize) {
    *cur = (*cur).min(len(s));
    s.truncate(byte(s, *cur));
}

/// Lo que enviaste, para recorrerlo con ↑ ↓. Se guarda entre sesiones.
pub struct Historial {
    items: Vec<String>,
    /// Dónde estás recorriendo (None = escribiendo algo nuevo) y lo que tenías escrito.
    pos: Option<usize>,
    draft: String,
}

const MAX: usize = 500;

fn path() -> Option<PathBuf> {
    Some(crate::rutas::estado()?.join("historial"))
}

impl Historial {
    pub fn load() -> Self {
        // Una entrada por línea; los saltos de línea de una orden van como «\n» escapado.
        let items = path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| t.lines().map(|l| l.replace("\\n", "\n")).collect())
            .unwrap_or_default();
        Historial { items, pos: None, draft: String::new() }
    }

    pub fn push(&mut self, text: &str) {
        self.pos = None;
        let text = text.trim();
        if text.is_empty() || self.items.last().is_some_and(|l| l == text) {
            return;
        }
        self.items.push(text.to_string());
        if self.items.len() > MAX {
            self.items.drain(..self.items.len() - MAX);
        }
        if let Some(p) = path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let body: Vec<String> = self.items.iter().map(|l| l.replace('\n', "\\n")).collect();
            let _ = std::fs::write(p, body.join("\n") + "\n");
        }
    }

    /// ↑: la orden anterior. Devuelve el texto a poner en la entrada.
    pub fn prev(&mut self, current: &str) -> Option<String> {
        if self.items.is_empty() {
            return None;
        }
        let pos = match self.pos {
            None => {
                self.draft = current.to_string();
                self.items.len() - 1
            }
            Some(0) => 0,
            Some(p) => p - 1,
        };
        self.pos = Some(pos);
        Some(self.items[pos].clone())
    }

    /// ↓: la siguiente; pasando la última, vuelve lo que estabas escribiendo.
    pub fn next(&mut self) -> Option<String> {
        let p = self.pos?;
        if p + 1 < self.items.len() {
            self.pos = Some(p + 1);
            Some(self.items[p + 1].clone())
        } else {
            self.pos = None;
            Some(std::mem::take(&mut self.draft))
        }
    }

    pub fn browsing(&self) -> bool {
        self.pos.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edita_en_medio_con_tildes() {
        let (mut s, mut c) = (String::from("acción"), 6);
        left(&s, &mut c);
        left(&s, &mut c);
        insert(&mut s, &mut c, "X");
        assert_eq!((s.as_str(), c), ("acciXón", 5));
        backspace(&mut s, &mut c);
        delete(&mut s, &mut c);
        assert_eq!((s.as_str(), c), ("accin", 4));
    }

    #[test]
    fn palabras() {
        let (mut s, mut c) = (String::from("revisa el núcleo ya"), 19);
        word_left(&s, &mut c);
        assert_eq!(c, 17);
        word_left(&s, &mut c);
        assert_eq!(c, 10);
        word_right(&s, &mut c);
        assert_eq!(c, 16);
        kill_word(&mut s, &mut c);
        assert_eq!((s.as_str(), c), ("revisa el  ya", 10));
        kill_to_end(&mut s, &mut c);
        assert_eq!(s, "revisa el ");
    }

    #[test]
    fn recorre_el_historial_y_vuelve_al_borrador() {
        let mut h = Historial { items: vec!["uno".into(), "dos".into()], pos: None, draft: String::new() };
        assert_eq!(h.prev("escribiendo").as_deref(), Some("dos"));
        assert_eq!(h.prev("").as_deref(), Some("uno"));
        assert_eq!(h.prev("").as_deref(), Some("uno"));
        assert_eq!(h.next().as_deref(), Some("dos"));
        assert_eq!(h.next().as_deref(), Some("escribiendo"));
        assert_eq!(h.next(), None);
    }
}
