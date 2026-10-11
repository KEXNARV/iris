//! Colors of the active Omarchy theme (the ones Aether applies): the accent replaces the usual
//! light blue and the dim tones derive from it. It's only reread when the theme changes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use ratatui::style::Color;

// The usual ones, in case there's no theme.
static ACCENT: AtomicU32 = AtomicU32::new(0x00c8ff);
static TEXT: AtomicU32 = AtomicU32::new(0xcde1eb);
static FAINT: AtomicU32 = AtomicU32::new(0x5a6e7d);
static BG: AtomicU32 = AtomicU32::new(0x05090d);
/// The content read last time: Omarchy copies themes preserving their dates, so the file's
/// date isn't enough to tell whether it changed.
static SEEN: Mutex<String> = Mutex::new(String::new());
/// The buddy's file (Baymax), where its face color is stored (`nucleo`, with `/core`).
/// The other colors always come from the Omarchy theme.
static PALETTE: Mutex<Option<PathBuf>> = Mutex::new(None);
/// The theme's named colors (red, green…): states are painted with them under Baymax.
/// The ones the theme doesn't provide stay at 0 (and the usual ones are used).
static ANSI: Mutex<[u32; 8]> = Mutex::new([0; 8]);
const ANSI_NAMES: [&str; 8] = ["red", "green", "yellow", "blue", "magenta", "cyan", "orange", "muted"];

/// Switches to a buddy's file (`None`: the original, no file). If it's a different one, goes back
/// to the factory colors and rereads the theme.
pub fn use_palette(p: Option<PathBuf>) {
    let mut current = PALETTE.lock().unwrap();
    if *current == p {
        return;
    }
    *current = p;
    drop(current);
    reload();
}

/// A named theme color (`red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `orange`,
/// `muted`), only with Baymax and if the theme provides it.
pub fn ansi(name: &str) -> Option<[f64; 3]> {
    if !baymax() {
        return None;
    }
    let i = ANSI_NAMES.iter().position(|n| *n == name)?;
    let v = ANSI.lock().unwrap()[i];
    (v != 0).then(|| rgb(v))
}

pub fn text_u32() -> u32 {
    TEXT.load(Ordering::Relaxed)
}

fn luminance(c: u32) -> f64 {
    let lin = |v: u32| {
        let x = v as f64 / 255.0;
        if x <= 0.03928 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c >> 16 & 0xff) + 0.7152 * lin(c >> 8 & 0xff) + 0.0722 * lin(c & 0xff)
}

/// WCAG contrast between two colors (1 to 21).
pub fn contrast(a: u32, b: u32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Goes back to the factory colors and rereads whatever applies (also undoes `preview_accent`).
pub fn reload() {
    ACCENT.store(0x00c8ff, Ordering::Relaxed);
    TEXT.store(0xcde1eb, Ordering::Relaxed);
    FAINT.store(0x5a6e7d, Ordering::Relaxed);
    BG.store(0x05090d, Ordering::Relaxed);
    SEEN.lock().unwrap().clear();
    *ANSI.lock().unwrap() = [0; 8];
    poll();
}

/// The background of the selected row in lists.
pub fn selection() -> Color {
    Color::Rgb(15, 45, 65)
}

/// The current background, as `0xrrggbb`.
pub fn background_rgb() -> u32 {
    BG.load(Ordering::Relaxed)
}

/// Is Baymax in the core?
pub fn baymax() -> bool {
    PALETTE.lock().unwrap().is_some()
}

pub fn accent_u32() -> u32 {
    ACCENT.load(Ordering::Relaxed)
}

/// Writes `key = "value"` to the buddy's file (with no buddy that has a file, does nothing).
pub fn save(key: &str, value: &str) -> std::io::Result<()> {
    let Some(p) = PALETTE.lock().unwrap().clone() else { return Ok(()) };
    let text = std::fs::read_to_string(&p).unwrap_or_default();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&p, with_key(&text, key, value))
}

/// The toml with `key` changed (or appended at the end if it wasn't there).
fn with_key(toml: &str, key: &str, value: &str) -> String {
    let line = format!("{key} = \"{value}\"");
    let mut found = false;
    let mut out: Vec<String> = toml
        .lines()
        .map(|l| match l.split_once('=') {
            Some((k, _)) if k.trim() == key => {
                found = true;
                line.clone()
            }
            _ => l.to_string(),
        })
        .collect();
    if !found {
        out.push(line);
    }
    out.join("\n") + "\n"
}

fn path() -> Option<PathBuf> {
    if let Some(p) = crate::paths::var("THEME") {
        return Some(p.into());
    }
    Some(crate::paths::home()?.join(".local/state/omarchy/current/theme/colors.toml"))
}

/// Is there an Omarchy theme to follow? Without it, the accent is the one chosen in the welcome.
pub fn has_omarchy() -> bool {
    path().is_some_and(|p| p.exists())
}

/// The accents to choose from without Omarchy: (id, name, color). The first is the usual one.
pub const ACCENTS: [(&str, &str, u32); 8] = [
    ("celeste", "Celeste", 0x00c8ff),
    ("menta", "Menta", 0x3ee8b0),
    ("verde", "Verde", 0x7bd88f),
    ("ambar", "Ámbar", 0xffb84a),
    ("coral", "Coral", 0xff7a6b),
    ("rosa", "Rosa", 0xff7ac6),
    ("violeta", "Violeta", 0xa78bfa),
    ("hielo", "Hielo", 0xd8e6f0),
];

/// Sets the accent right away, without saving it: the preview while choosing.
pub fn preview_accent(c: u32) {
    ACCENT.store(c, Ordering::Relaxed);
}

/// Rereads the theme if it changed since last time. Returns true if anything changed.
pub fn poll() -> bool {
    let Some(p) = path() else { return false };
    // Without Omarchy, the accent saved in the welcome acts as the theme (it only brings that color).
    // It's read here and not via `settings`: this file is also compiled into the web core.
    let own_file = || {
        let a = std::fs::read_to_string(crate::paths::config()?.join("acento")).ok()?;
        Some(format!("accent = \"{}\"\n", a.trim()))
    };
    let Some(text) = std::fs::read_to_string(&p).ok().or_else(own_file) else { return false };
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
    // The dim text: of the ones the theme provides, the one that stands out least on the background.
    // Aether provides both and its «light» is brighter than the text in dark themes.
    let bg = BG.load(Ordering::Relaxed);
    let dimmed = [get("light_foreground"), get("dark_foreground")]
        .into_iter()
        .flatten()
        .min_by(|a, b| contrast(*a, bg).total_cmp(&contrast(*b, bg)));
    if let Some(f) = dimmed {
        FAINT.store(f, Ordering::Relaxed);
    }
    let mut ansi = ANSI.lock().unwrap();
    for (i, n) in ANSI_NAMES.iter().enumerate() {
        ansi[i] = get(n).unwrap_or(0);
    }
    drop(ansi);
    if baymax() {
        // His face color is in his file; the rest, in the theme.
        let own = PALETTE.lock().unwrap().clone().and_then(|p| std::fs::read_to_string(p).ok());
        crate::baymax::read(own.as_deref().unwrap_or(""));
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

/// The accent dimmed toward the background: borders, scale marks, secondary stuff.
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

/// Accent variants for the core's «presence» states.
pub fn accent_toward_white(f: f64) -> [f64; 3] {
    mix(accent_rgb(), [255.0; 3], f)
}

pub fn accent_darker(f: f64) -> [f64; 3] {
    mix(accent_rgb(), [0.0; 3], f)
}

#[cfg(test)]
pub(crate) mod tests {
    /// Colors are global: the tests that touch them take turns.
    pub(crate) static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());


    use super::*;

    #[test]
    fn reads_theme_accent() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let toml = "mode = \"dark\"\naccent = \"#e68e0d\"\nforeground = \"#bebebe\"\nbackground = \"#121212\"\n";
        assert_eq!(parse_hex("#e68e0d"), Some(0xe68e0d));
        assert_eq!(parse_hex("e68e0d"), None);
        assert!(apply(toml));
        assert_eq!(accent(), Color::Rgb(0xe6, 0x8e, 0x0d));
        assert_eq!(text(), Color::Rgb(0xbe, 0xbe, 0xbe));
        // A file without an accent touches nothing.
        assert!(!apply("foreground = \"#ffffff\""));
        assert_eq!(text(), Color::Rgb(0xbe, 0xbe, 0xbe));
    }

    #[test]
    fn changes_one_key_leaving_the_rest() {
        let toml = "# Cara:\naccent = \"#F56FA1\"\nnucleo = \"rosa\"\n";
        assert_eq!(with_key(toml, "nucleo", "#1A2332"), "# Cara:\naccent = \"#F56FA1\"\nnucleo = \"#1A2332\"\n");
        assert_eq!(with_key("accent = \"#F56FA1\"\n", "nucleo", "auto"), "accent = \"#F56FA1\"\nnucleo = \"auto\"\n");
    }
}

#[cfg(test)]
#[test]
#[ignore]
fn real_theme() {
    println!("poll={} accent={:?} dim={:?} text={:?}", poll(), accent(), dim(), text());
}
