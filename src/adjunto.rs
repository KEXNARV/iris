//! Archivos que Claude te entrega: toda ruta a un archivo que existe, mencionada en su
//! respuesta, sale en el chat como adjunto. Un clic lo copia al portapapeles como archivo
//! (`text/uri-list`), así que se pega tal cual en el gestor de archivos, el navegador, el
//! correo o un chat — igual que si lo hubieras copiado en Nautilus.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct Adjunto {
    pub path: PathBuf,
    pub bytes: u64,
}

impl Adjunto {
    pub fn name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn label(&self) -> String {
        format!("{} · {}", self.name(), size(self.bytes))
    }
}

/// Las rutas absolutas (o con `~/`) del texto que son archivos de verdad, sin repetir.
pub fn en_texto(text: &str) -> Vec<Adjunto> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut out: Vec<Adjunto> = vec![];
    for raw in candidatos(text) {
        let p = match raw.strip_prefix("~/") {
            Some(rest) => PathBuf::from(&home).join(rest),
            None => PathBuf::from(&raw),
        };
        let Ok(meta) = std::fs::metadata(&p) else { continue };
        if meta.is_file() && !out.iter().any(|a| a.path == p) {
            out.push(Adjunto { path: p, bytes: meta.len() });
        }
    }
    out
}

/// Lo que puede ser una ruta: lo que va entre comillas invertidas y cada palabra suelta que
/// empiece con `/` o `~/`, sin la puntuación de alrededor ni el `:línea` del final.
fn candidatos(text: &str) -> Vec<String> {
    let mut out = vec![];
    for (k, part) in text.split('`').enumerate() {
        // Las partes impares están entre comillas invertidas: la ruta puede llevar espacios.
        if k % 2 == 1 {
            out.push(limpia(part));
        }
        for w in part.split_whitespace() {
            out.push(limpia(w));
        }
    }
    out.retain(|c| c.starts_with('/') || c.starts_with("~/"));
    out
}

fn limpia(s: &str) -> String {
    let s = s.trim_start_matches(|c: char| "*_\"'«»([<".contains(c));
    let mut s = s.trim_end_matches(|c: char| "*_\"'«»)]>,;!?.:".contains(c));
    // `ruta:12` o `ruta:12:3`: el número es de línea, no parte del nombre.
    while let Some((head, tail)) = s.rsplit_once(':') {
        if tail.is_empty() || !tail.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        s = head;
    }
    s.to_string()
}

/// Al portapapeles como archivo, no como texto con la ruta.
#[cfg(windows)]
pub fn copiar(a: &Adjunto) -> bool {
    // Como copiarlo en el Explorador: se pega como archivo en carpetas, correos y chats.
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Set-Clipboard -LiteralPath $args[0]"])
        .arg(&a.path)
        .status()
        .is_ok_and(|s| s.success())
}

/// Al portapapeles como archivo, no como texto con la ruta.
#[cfg(not(windows))]
pub fn copiar(a: &Adjunto) -> bool {
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
    fn saca_rutas_del_texto() {
        let t = "Listo: **`~/Documents/Reporte WEGA.xlsx`** y /tmp/a.rs:12, (ver /etc/hosts).";
        let c = candidatos(t);
        assert!(c.contains(&"~/Documents/Reporte WEGA.xlsx".to_string()));
        assert!(c.contains(&"/tmp/a.rs".to_string()));
        assert!(c.contains(&"/etc/hosts".to_string()));
    }

    #[test]
    fn solo_archivos_que_existen() {
        let a = en_texto("mira /etc/hosts, /etc y /no/existe.txt; otra vez `/etc/hosts`");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name(), "hosts");
    }

    #[test]
    fn la_uri_escapa_espacios_y_acentos() {
        assert_eq!(percent_encode("/home/kex/Reporte año 1.xlsx"), "/home/kex/Reporte%20a%C3%B1o%201.xlsx");
    }
}
