//! Colores del tema activo de Omarchy (los que aplica Aether): el acento reemplaza al celeste
//! de siempre y los tonos apagados salen de él. Se relee solo cuando cambia el tema.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use ratatui::style::Color;

// Los de siempre, por si no hay tema.
static ACCENT: AtomicU32 = AtomicU32::new(0x00c8ff);
static TEXT: AtomicU32 = AtomicU32::new(0xcde1eb);
static FAINT: AtomicU32 = AtomicU32::new(0x5a6e7d);
static BG: AtomicU32 = AtomicU32::new(0x05090d);
/// El contenido leído la última vez: Omarchy copia los temas conservando sus fechas, así que
/// la fecha del archivo no alcanza para saber si cambió.
static SEEN: Mutex<String> = Mutex::new(String::new());
/// El archivo del buddy (Baymax), donde se guarda el color de su cara (`nucleo`, con `/core`).
/// Los demás colores salen siempre del tema de Omarchy.
static PALETA: Mutex<Option<PathBuf>> = Mutex::new(None);
/// Los colores con nombre del tema (rojo, verde…): con ellos se pintan los estados con
/// Baymax. Los que el tema no trae quedan en 0 (y se usan los de siempre).
static ANSI: Mutex<[u32; 8]> = Mutex::new([0; 8]);
const ANSI_NOMBRES: [&str; 8] = ["red", "green", "yellow", "blue", "magenta", "cyan", "orange", "muted"];

/// Cambia al archivo de un buddy (`None`: el original, sin archivo). Si es otro, vuelve a los
/// colores de fábrica y relee el tema.
pub fn usar(p: Option<PathBuf>) {
    let mut actual = PALETA.lock().unwrap();
    if *actual == p {
        return;
    }
    *actual = p;
    drop(actual);
    recargar();
}

/// Un color con nombre del tema (`red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `orange`,
/// `muted`), solo con Baymax y si el tema lo trae.
pub fn ansi(nombre: &str) -> Option<[f64; 3]> {
    if !baymax() {
        return None;
    }
    let i = ANSI_NOMBRES.iter().position(|n| *n == nombre)?;
    let v = ANSI.lock().unwrap()[i];
    (v != 0).then(|| rgb(v))
}

pub fn texto_u32() -> u32 {
    TEXT.load(Ordering::Relaxed)
}

fn luz(c: u32) -> f64 {
    let lin = |v: u32| {
        let x = v as f64 / 255.0;
        if x <= 0.03928 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c >> 16 & 0xff) + 0.7152 * lin(c >> 8 & 0xff) + 0.0722 * lin(c & 0xff)
}

/// Contraste WCAG entre dos colores (1 a 21).
pub fn contraste(a: u32, b: u32) -> f64 {
    let (a, b) = (luz(a), luz(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Vuelve a los colores de fábrica y relee lo que toque.
fn recargar() {
    ACCENT.store(0x00c8ff, Ordering::Relaxed);
    TEXT.store(0xcde1eb, Ordering::Relaxed);
    FAINT.store(0x5a6e7d, Ordering::Relaxed);
    BG.store(0x05090d, Ordering::Relaxed);
    SEEN.lock().unwrap().clear();
    *ANSI.lock().unwrap() = [0; 8];
    poll();
}

/// El fondo de la fila elegida en las listas.
pub fn seleccion() -> Color {
    Color::Rgb(15, 45, 65)
}

/// El fondo de ahora, como `0xrrggbb`.
pub fn fondo_rgb() -> u32 {
    BG.load(Ordering::Relaxed)
}

/// ¿Está Baymax en el núcleo?
pub fn baymax() -> bool {
    PALETA.lock().unwrap().is_some()
}

pub fn acento_u32() -> u32 {
    ACCENT.load(Ordering::Relaxed)
}

/// Escribe `clave = "valor"` en el archivo del buddy (sin buddy con archivo, no hace nada).
pub fn guardar(clave: &str, valor: &str) -> std::io::Result<()> {
    let Some(p) = PALETA.lock().unwrap().clone() else { return Ok(()) };
    let text = std::fs::read_to_string(&p).unwrap_or_default();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&p, con_clave(&text, clave, valor))
}

/// El toml con `clave` cambiada (o agregada al final si no estaba).
fn con_clave(toml: &str, clave: &str, valor: &str) -> String {
    let linea = format!("{clave} = \"{valor}\"");
    let mut hay = false;
    let mut out: Vec<String> = toml
        .lines()
        .map(|l| match l.split_once('=') {
            Some((k, _)) if k.trim() == clave => {
                hay = true;
                linea.clone()
            }
            _ => l.to_string(),
        })
        .collect();
    if !hay {
        out.push(linea);
    }
    out.join("\n") + "\n"
}

fn path() -> Option<PathBuf> {
    if let Some(p) = crate::rutas::var("THEME") {
        return Some(p.into());
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".local/state/omarchy/current/theme/colors.toml"))
}

/// Relee el tema si cambió desde la última vez. Devuelve true si cambió algo.
pub fn poll() -> bool {
    let Some(p) = path() else { return false };
    let Ok(text) = std::fs::read_to_string(&p) else { return false };
    let mut seen = SEEN.lock().unwrap();
    if *seen == text {
        return false;
    }
    *seen = text;
    apply(&seen)
}

fn apply(toml: &str) -> bool {
    let get = |key: &str| {
        toml.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| parse_hex(v.trim().trim_matches('"')))?
        })
    };
    let Some(accent) = get("accent") else { return false };
    ACCENT.store(accent, Ordering::Relaxed);
    if let Some(fg) = get("foreground") {
        TEXT.store(fg, Ordering::Relaxed);
    }
    if let Some(bg) = get("background") {
        BG.store(bg, Ordering::Relaxed);
    }
    // El texto apagado: de los que traiga el tema, el que menos resalta sobre el fondo. Aether
    // trae los dos y su «light» es más brillante que el texto en los temas oscuros.
    let bg = BG.load(Ordering::Relaxed);
    let apagado = [get("light_foreground"), get("dark_foreground")]
        .into_iter()
        .flatten()
        .min_by(|a, b| contraste(*a, bg).total_cmp(&contraste(*b, bg)));
    if let Some(f) = apagado {
        FAINT.store(f, Ordering::Relaxed);
    }
    let mut ansi = ANSI.lock().unwrap();
    for (i, n) in ANSI_NOMBRES.iter().enumerate() {
        ansi[i] = get(n).unwrap_or(0);
    }
    drop(ansi);
    if baymax() {
        // El color de su cara está en su archivo; los demás, en el tema.
        let propia = PALETA.lock().unwrap().clone().and_then(|p| std::fs::read_to_string(p).ok());
        crate::baymax::leer(propia.as_deref().unwrap_or(""));
    }
    true
}

fn parse_hex(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok())?
}

fn rgb(v: u32) -> [f64; 3] {
    [(v >> 16 & 0xff) as f64, (v >> 8 & 0xff) as f64, (v & 0xff) as f64]
}

fn color(c: [f64; 3]) -> Color {
    Color::Rgb(c[0].round() as u8, c[1].round() as u8, c[2].round() as u8)
}

fn mix(a: [f64; 3], b: [f64; 3], f: f64) -> [f64; 3] {
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f]
}

pub fn accent_rgb() -> [f64; 3] {
    rgb(ACCENT.load(Ordering::Relaxed))
}

/// El acento apagado hacia el fondo: bordes, marcas de la escala, lo secundario.
pub fn dim_rgb() -> [f64; 3] {
    mix(accent_rgb(), rgb(BG.load(Ordering::Relaxed)), 0.6)
}

pub fn accent() -> Color {
    color(accent_rgb())
}

pub fn dim() -> Color {
    color(dim_rgb())
}

pub fn text() -> Color {
    color(rgb(TEXT.load(Ordering::Relaxed)))
}

pub fn faint() -> Color {
    color(rgb(FAINT.load(Ordering::Relaxed)))
}

/// Variantes del acento para los estados de «presencia» del núcleo.
pub fn accent_toward_white(f: f64) -> [f64; 3] {
    mix(accent_rgb(), [255.0; 3], f)
}

pub fn accent_darker(f: f64) -> [f64; 3] {
    mix(accent_rgb(), [0.0; 3], f)
}

#[cfg(test)]
pub(crate) mod tests {
    /// Los colores son globales: las pruebas que los tocan se turnan.
    pub(crate) static TURNO: std::sync::Mutex<()> = std::sync::Mutex::new(());


    use super::*;

    #[test]
    fn lee_el_acento_del_tema() {
        let _turno = TURNO.lock().unwrap_or_else(|e| e.into_inner());
        let toml = "mode = \"dark\"\naccent = \"#e68e0d\"\nforeground = \"#bebebe\"\nbackground = \"#121212\"\n";
        assert_eq!(parse_hex("#e68e0d"), Some(0xe68e0d));
        assert_eq!(parse_hex("e68e0d"), None);
        assert!(apply(toml));
        assert_eq!(accent(), Color::Rgb(0xe6, 0x8e, 0x0d));
        assert_eq!(text(), Color::Rgb(0xbe, 0xbe, 0xbe));
        // Un archivo sin acento no toca nada.
        assert!(!apply("foreground = \"#ffffff\""));
        assert_eq!(text(), Color::Rgb(0xbe, 0xbe, 0xbe));
    }

    #[test]
    fn cambia_una_clave_sin_tocar_lo_demas() {
        let toml = "# Cara:\naccent = \"#F56FA1\"\nnucleo = \"rosa\"\n";
        assert_eq!(con_clave(toml, "nucleo", "#1A2332"), "# Cara:\naccent = \"#F56FA1\"\nnucleo = \"#1A2332\"\n");
        assert_eq!(con_clave("accent = \"#F56FA1\"\n", "nucleo", "auto"), "accent = \"#F56FA1\"\nnucleo = \"auto\"\n");
    }
}

#[cfg(test)]
#[test]
#[ignore]
fn tema_real() {
    println!("poll={} accent={:?} dim={:?} text={:?}", poll(), accent(), dim(), text());
}
