//! Files Claude hands you: every path to an existing file mentioned in its reply shows up in
//! the chat as an attachment. A click copies it to the clipboard as a file
//! (`text/uri-list`), so it pastes as-is into the file manager, the browser, an email or a
//! chat — same as if you had copied it in Nautilus.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct Attachment {
    pub path: PathBuf,
    pub bytes: u64,
}

impl Attachment {
    pub fn name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn label(&self) -> String {
        format!("{} · {}", self.name(), size(self.bytes))
    }
}

/// The absolute (or `~/`) paths in the text that are real files, without duplicates.
pub fn in_text(text: &str) -> Vec<Attachment> {
    let home = crate::paths::home().unwrap_or_default();
    let mut out: Vec<Attachment> = vec![];
    for raw in candidates(text) {
        let p = match raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
            Some(rest) => home.join(rest),
            None => PathBuf::from(&raw),
        };
        let Ok(meta) = std::fs::metadata(&p) else { continue };
        if meta.is_file() && !out.iter().any(|a| a.path == p) {
            out.push(Attachment { path: p, bytes: meta.len() });
        }
    }
    out
}

/// What could be a path: whatever is between backticks and every loose word that starts
/// with `/`, `~/` or a Windows drive (`C:\`, `C:/`), without the surrounding punctuation or
/// the trailing `:line`.
fn candidates(text: &str) -> Vec<String> {
    let mut out = vec![];
    for (k, part) in text.split('`').enumerate() {
        // Odd parts are between backticks: the path may contain spaces.
        if k % 2 == 1 {
            out.push(clean(part));
        }
        for w in part.split_whitespace() {
            out.push(clean(w));
        }
    }
    out.retain(|c| c.starts_with('/') || c.starts_with("~/") || c.starts_with("~\\") || is_drive(c));
    out
}

/// `C:\…` or `C:/…`: an absolute Windows path.
fn is_drive(c: &str) -> bool {
    let b = c.as_bytes();
    b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
}

fn clean(s: &str) -> String {
    let s = s.trim_start_matches(|c: char| "*_\"'«»([<".contains(c));
    let mut s = s.trim_end_matches(|c: char| "*_\"'«»)]>,;!?.:".contains(c));
    // `path:12` or `path:12:3`: the number is a line, not part of the name.
    while let Some((head, tail)) = s.rsplit_once(':') {
        if tail.is_empty() || !tail.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        s = head;
    }
    s.to_string()
}

/// To the clipboard as a file, not as text with the path.
#[cfg(windows)]
pub fn copy(a: &Attachment) -> bool {
    // Like copying it in Explorer: it pastes as a file into folders, emails and chats.
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Set-Clipboard -LiteralPath $args[0]"])
        .arg(&a.path)
        .status()
        .is_ok_and(|s| s.success())
}

/// To the clipboard as a file, not as text with the path.
#[cfg(not(windows))]
pub fn copy(a: &Attachment) -> bool {
    let uri = format!("file://{}\r\n", percent_encode(&a.path.to_string_lossy()));
    Command::new("wl-copy")
        .args(["--type", "text/uri-list"])
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(uri.as_bytes())?;
            c.wait()
        })
        .is_ok_and(|s| s.success())
}

fn percent_encode(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn size(n: u64) -> String {
    match n {
        n if n >= 1 << 20 => format!("{:.1} MB", n as f64 / (1 << 20) as f64),
        n if n >= 1 << 10 => format!("{} KB", n >> 10),
        n => format!("{n} B"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_paths_from_text() {
        let t = "Listo: **`~/Documents/Reporte WEGA.xlsx`** y /tmp/a.rs:12, (ver /etc/hosts).";
        let c = candidates(t);
        assert!(c.contains(&"~/Documents/Reporte WEGA.xlsx".to_string()));
        assert!(c.contains(&"/tmp/a.rs".to_string()));
        assert!(c.contains(&"/etc/hosts".to_string()));
    }

    #[test]
    fn extracts_windows_paths() {
        let t = r"Listo: `C:\Users\kex\Reporte WEGA.xlsx` y C:\tmp\a.rs:12, (ver D:/datos/b.csv).";
        let c = candidates(t);
        assert!(c.contains(&r"C:\Users\kex\Reporte WEGA.xlsx".to_string()));
        assert!(c.contains(&r"C:\tmp\a.rs".to_string()));
        assert!(c.contains(&"D:/datos/b.csv".to_string()));
        assert!(!c.contains(&"Listo".to_string()));
    }

    #[test]
    fn only_existing_files() {
        let a = in_text("mira /etc/hosts, /etc y /no/existe.txt; otra vez `/etc/hosts`");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name(), "hosts");
    }

    #[test]
    fn uri_escapes_spaces_and_accents() {
        assert_eq!(percent_encode("/home/kex/Reporte año 1.xlsx"), "/home/kex/Reporte%20a%C3%B1o%201.xlsx");
    }
}
