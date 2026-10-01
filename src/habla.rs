//! La voz de JARVIS: un proceso de Python con Kokoro (`scripts/hablar.py`) que se abre la
//! primera vez que hace falta hablar y queda vivo. Acá se le mandan frases y se escuchan sus
//! eventos (empezó, volumen, terminó). `Lector` arma esas frases a partir del streaming.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use serde_json::json;

use crate::AppEvent;

pub enum HablaEvent {
    Start,
    Level(f32),
    Idle,
    Unavailable(String),
}

const SCRIPT: &str = include_str!("../scripts/hablar.py");

fn base() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share/jarvis")
}

pub struct Habla {
    tx: Sender<AppEvent>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    failed: bool,
}

impl Habla {
    pub fn new(tx: Sender<AppEvent>) -> Self {
        Habla { tx, child: None, stdin: None, failed: false }
    }

    fn start(&mut self) -> Result<(), String> {
        let python = base().join("py/bin/python");
        if !python.exists() {
            return Err("no está el entorno de voz (~/.local/share/jarvis/py)".into());
        }
        // El script viaja dentro de Jarvis; se escribe si cambió.
        let path = base().join("hablar.py");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(SCRIPT) {
            std::fs::write(&path, SCRIPT).map_err(|e| e.to_string())?;
        }
        let mut child = Command::new(python)
            .arg(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdout = child.stdout.take().unwrap();
        let tx = self.tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let ev = match line.split_once(' ') {
                    Some(("level", v)) => HablaEvent::Level(v.parse().unwrap_or(0.0)),
                    _ if line == "start" => HablaEvent::Start,
                    _ if line == "idle" => HablaEvent::Idle,
                    _ => continue,
                };
                if tx.send(AppEvent::Habla(ev)).is_err() {
                    break;
                }
            }
            let _ = tx.send(AppEvent::Habla(HablaEvent::Idle));
        });
        self.stdin = child.stdin.take();
        self.child = Some(child);
        Ok(())
    }

    fn send(&mut self, msg: serde_json::Value) {
        if self.stdin.is_none() && !self.failed {
            if let Err(e) = self.start() {
                self.failed = true;
                let _ = self.tx.send(AppEvent::Habla(HablaEvent::Unavailable(e)));
            }
        }
        if let Some(s) = &mut self.stdin {
            if writeln!(s, "{msg}").and_then(|_| s.flush()).is_err() {
                // Se murió: la próxima frase lo vuelve a abrir.
                self.stdin = None;
                self.child = None;
            }
        }
    }

    pub fn say(&mut self, text: &str) {
        self.send(json!({ "op": "say", "text": text }));
    }

    pub fn stop(&mut self) {
        if self.stdin.is_some() {
            self.send(json!({ "op": "stop" }));
        }
    }
}

impl Drop for Habla {
    fn drop(&mut self) {
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Arma frases para decir en voz alta a partir del texto que llega en streaming: sin código,
/// tablas, URLs ni marcas de markdown. Después de `BUDGET` caracteres avisa que el resto está
/// en pantalla y no dice más.
pub struct Lector {
    buf: String,
    code: bool,
    said: usize,
    done: bool,
}

const BUDGET: usize = 700;

impl Lector {
    pub fn new() -> Self {
        Lector { buf: String::new(), code: false, said: 0, done: false }
    }

    /// Lo que se puede decir ya: las frases completas que hay hasta ahora.
    pub fn push(&mut self, delta: &str) -> Vec<String> {
        self.buf.push_str(delta);
        let mut out = vec![];
        loop {
            let Some(end) = sentence_end(&self.buf) else { break };
            let piece: String = self.buf.drain(..end).collect();
            out.extend(self.take(&piece));
        }
        out
    }

    /// Fin del turno: lo que quedó sin cerrar.
    pub fn finish(&mut self) -> Vec<String> {
        let rest = std::mem::take(&mut self.buf);
        let out = self.take(&rest);
        self.code = false;
        out
    }

    fn take(&mut self, piece: &str) -> Vec<String> {
        let mut out = vec![];
        for line in piece.split('\n') {
            if line.trim_start().starts_with("```") {
                self.code = !self.code;
                continue;
            }
            if self.code || self.done {
                continue;
            }
            let Some(text) = clean(line) else { continue };
            if self.said + text.len() > BUDGET && self.said > 0 {
                self.done = true;
                out.push("El resto lo tienes en pantalla.".to_string());
                continue;
            }
            self.said += text.len();
            out.push(text);
        }
        out
    }
}

/// Dónde termina la primera frase completa: después de . ? ! o : seguidos de espacio, o en un
/// salto de línea (cada ítem de una lista se dice aparte).
fn sentence_end(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c == b'\n' {
            return Some(i + 1);
        }
        if matches!(c, b'.' | b'?' | b'!' | b':') && b.get(i + 1).is_some_and(|n| *n == b' ') {
            return Some(i + 2);
        }
    }
    None
}

/// Una línea lista para decir, o `None` si no tiene nada que se pueda leer en voz alta.
fn clean(line: &str) -> Option<String> {
    let t = line.trim();
    // Tablas y líneas horizontales no se leen.
    if t.is_empty() || t.starts_with('|') || t.chars().all(|c| matches!(c, '-' | '─' | '*' | '_' | ' ')) {
        return None;
    }
    let t = t.trim_start_matches(['#', '>', ' ']);
    let t = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).unwrap_or(t);
    // «1. » de las listas numeradas.
    let t = match t.split_once(". ") {
        Some((n, rest)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => rest,
        _ => t,
    };
    // [texto](url) → texto.
    let mut s = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(open) = rest.find('[') {
        let Some(mid) = rest[open..].find("](") else { break };
        let Some(close) = rest[open + mid..].find(')') else { break };
        s.push_str(&rest[..open]);
        s.push_str(&rest[open + 1..open + mid]);
        rest = &rest[open + mid + close + 1..];
    }
    s.push_str(rest);
    let s = s.replace(['*', '`'], "");
    // Ni URLs ni rutas largas: en voz alta no sirven.
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| !(w.starts_with("http") || (w.contains('/') && w.len() > 12)))
        .collect();
    let s = words.join(" ");
    (s.chars().filter(|c| c.is_alphanumeric()).count() >= 2).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arma_frases_mientras_llega_el_texto() {
        let mut l = Lector::new();
        assert!(l.push("Listo, lo ").is_empty());
        assert_eq!(l.push("revisé. Ahora sigue"), vec!["Listo, lo revisé."]);
        assert_eq!(l.finish(), vec!["Ahora sigue"]);
    }

    #[test]
    fn se_salta_codigo_tablas_y_urls() {
        let mut l = Lector::new();
        let mut said = l.push("Mira esto:\n```rust\nfn main() {}\n```\n| a | b |\n|---|---|\nLo vi en https://x.y/z y en ~/code/jarvis/src/ui.rs también.\n");
        said.extend(l.finish());
        assert_eq!(said, vec!["Mira esto:", "Lo vi en y en también."]);
    }

    #[test]
    fn limpia_el_markdown() {
        assert_eq!(clean("## **Qué** cambió").as_deref(), Some("Qué cambió"));
        assert_eq!(clean("- un [enlace](https://a.b) y `código`").as_deref(), Some("un enlace y código"));
        assert_eq!(clean("2. Segundo paso").as_deref(), Some("Segundo paso"));
        assert_eq!(clean("---"), None);
    }

    #[test]
    fn corta_las_respuestas_largas() {
        let mut l = Lector::new();
        let frase = "Esta es una frase bastante larga para llenar el presupuesto de la voz. ";
        let said = l.push(&frase.repeat(30));
        assert_eq!(said.last().map(String::as_str), Some("El resto lo tienes en pantalla."));
        assert!(said.iter().map(|s| s.len()).sum::<usize>() < BUDGET + 100);
    }
}
