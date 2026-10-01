//! Imágenes para adjuntar: del portapapeles (Ctrl+V) o de rutas pegadas o arrastradas a la
//! terminal. La terminal solo pega texto, así que el portapapeles se lee con `wl-paste`.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};

/// La API acepta hasta 5 MB por imagen; más que eso se achica.
const MAX_BYTES: usize = 5 * 1024 * 1024;

pub struct Image {
    pub media: String,
    /// En base64, listo para el bloque `image` del mensaje.
    pub data: String,
    pub bytes: usize,
    /// Los bytes tal cual, para la miniatura del chat.
    pub raw: Vec<u8>,
}

impl Image {
    pub fn label(&self, n: usize) -> String {
        format!("imagen {n} · {}", size(self.bytes))
    }
}

pub enum Clip {
    Image(Image),
    Text(String),
    Nothing,
}

const TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

/// Lo que haya en el portapapeles: una imagen si la hay, si no el texto.
pub fn paste() -> Result<Clip> {
    let out = Command::new("wl-paste").arg("--list-types").output().map_err(|_| anyhow!("falta wl-paste (wl-clipboard)"))?;
    let types = String::from_utf8_lossy(&out.stdout);
    if let Some(t) = TYPES.iter().find(|t| types.lines().any(|l| l.trim() == **t)) {
        let raw = Command::new("wl-paste").args(["--no-newline", "--type", t]).output()?.stdout;
        return Ok(Clip::Image(image(raw, t)?));
    }
    if types.lines().any(|l| l.starts_with("text/")) {
        let raw = Command::new("wl-paste").arg("--no-newline").output()?.stdout;
        let text = String::from_utf8_lossy(&raw).into_owned();
        // Copiar un archivo en el gestor de archivos deja su ruta (o un file://) como texto.
        if let Some(imgs) = paths(&text) {
            if let [one] = &imgs[..] {
                return Ok(Clip::Image(read(one)?));
            }
        }
        return Ok(Clip::Text(text));
    }
    Ok(Clip::Nothing)
}

/// Si el texto pegado son solo rutas a imágenes (arrastrar archivos a foot las pega así),
/// las rutas; si no, `None` y se trata como texto.
pub fn paths(text: &str) -> Option<Vec<String>> {
    let items: Vec<String> = text
        .split(['\n', '\r'])
        .flat_map(shell_words)
        .map(|s| s.strip_prefix("file://").map(percent_decode).unwrap_or(s))
        .filter(|s| !s.is_empty())
        .collect();
    let all_images = !items.is_empty()
        && items.iter().all(|p| {
            let p = Path::new(p);
            p.is_file() && media_of(p).is_some()
        });
    all_images.then_some(items)
}

pub fn read(path: &str) -> Result<Image> {
    let p = Path::new(path);
    let media = media_of(p).ok_or_else(|| anyhow!("{path}: no es png, jpg, webp ni gif"))?;
    image(std::fs::read(p)?, media)
}

fn image(raw: Vec<u8>, media: &str) -> Result<Image> {
    if raw.is_empty() {
        bail!("el portapapeles está vacío");
    }
    let (raw, media) = if raw.len() > MAX_BYTES { (shrink(&raw)?, "image/jpeg") } else { (raw, media) };
    Ok(Image { media: media.into(), bytes: raw.len(), data: crate::select::base64(&raw), raw })
}

/// Achica con ImageMagick a JPEG de 2000 px de lado como máximo.
fn shrink(raw: &[u8]) -> Result<Vec<u8>> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("magick")
        .args(["-", "-resize", "2000x2000>", "-quality", "85", "jpg:-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| anyhow!("la imagen pasa de 5 MB y no hay ImageMagick para achicarla"))?;
    child.stdin.take().unwrap().write_all(raw)?;
    let out = child.wait_with_output()?.stdout;
    if out.is_empty() || out.len() > MAX_BYTES {
        bail!("la imagen pasa de 5 MB y no pude achicarla");
    }
    Ok(out)
}

fn media_of(p: &Path) -> Option<&'static str> {
    match p.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

/// Palabras como las separa un shell: comillas simples o dobles y `\ ` para los espacios.
fn shell_words(line: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quote = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (None, '\'' | '"') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '\\') => cur.extend(chars.next()),
            (None, ' ' | '\t') => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn size(n: usize) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", n.div_ceil(1024))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_rutas_de_imagenes() {
        let dir = std::env::temp_dir().join("jarvis-clip-test");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("captura de pantalla.png");
        std::fs::write(&a, b"x").unwrap();
        let s = a.display().to_string();
        assert_eq!(paths(&format!("'{s}'")), Some(vec![s.clone()]));
        assert_eq!(paths(&s.replace(' ', "\\ ")), Some(vec![s.clone()]));
        assert_eq!(paths(&format!("file://{}", s.replace(' ', "%20"))), Some(vec![s.clone()]));
        // Texto normal, o una ruta que no es imagen, se pega como texto.
        assert_eq!(paths("hola mundo"), None);
        assert_eq!(paths("/etc/hostname"), None);
    }
}

/// `cargo test clip::de_verdad -- --ignored --nocapture`: lee el portapapeles real.
#[cfg(test)]
#[test]
#[ignore]
fn de_verdad() {
    match paste().unwrap() {
        Clip::Image(i) => println!("imagen {} · {} bytes · base64 {}", i.media, i.bytes, i.data.len()),
        Clip::Text(t) => println!("texto: {t:?}"),
        Clip::Nothing => println!("vacío"),
    }
}
