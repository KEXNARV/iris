//! Motor: un proceso `claude -p` vivo que habla stream-json por stdin/stdout.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::AppEvent;

pub enum ClaudeEvent {
    Init { model: String, session: String, skills: Vec<String> },
    Thinking,
    Text(String),
    ToolUse { id: String, name: String, detail: String },
    ToolResult { id: String, is_error: bool },
    Done { cost: f64, secs: f64, is_error: bool },
    /// El motor espera respuesta del usuario: una `AskUserQuestion` o un permiso.
    Ask { request_id: String, tool: String, input: Value },
    /// El motor tomó el mensaje con este id (lo repite al leerlo, por `--replay-user-messages`).
    Taken(String),
    /// `/compact` terminó: tokens de contexto antes y después.
    Compacted { pre: u64, post: u64 },
    Stderr(String),
    Exited,
}

/// Cada proceso lanzado lleva su número: al reiniciar o retomar, el viejo muere y su `Exited`
/// llega después de lanzar el nuevo; con el número, la app lo reconoce y lo ignora.
static GEN: AtomicU64 = AtomicU64::new(0);

pub struct Claude {
    pub id: u64,
    child: Child,
    stdin: ChildStdin,
    next_req: u64,
}

impl Claude {
    pub fn spawn(extra_args: &[String], tx: Sender<AppEvent>) -> Result<Self> {
        let mut child = Command::new("claude")
            .args([
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--verbose",
                "--include-partial-messages",
                // Repite cada mensaje nuestro en el momento en que lo lee: así se sabe cuándo
                // uno escrito a mitad de turno deja de estar en espera.
                "--replay-user-messages",
                // Sin esto `AskUserQuestion` no existe en `-p`; con esto el motor nos pregunta a
                // nosotros por stdout (`control_request` `can_use_tool`) y espera la respuesta.
                "--permission-prompt-tool",
                "stdio",
            ])
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("no pude lanzar `claude`; ¿está en el PATH?")?;

        let id = GEN.fetch_add(1, Ordering::Relaxed);
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let out_tx = tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    for ev in parse(&v) {
                        let _ = out_tx.send(AppEvent::Claude(id, ev));
                    }
                }
            }
            let _ = out_tx.send(AppEvent::Claude(id, ClaudeEvent::Exited));
        });

        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    let _ = tx.send(AppEvent::Claude(id, ClaudeEvent::Stderr(line)));
                }
            }
        });

        Ok(Self { id, child, stdin, next_req: 0 })
    }

    /// Devuelve el id con que el motor avisará que lo tomó.
    pub fn send(&mut self, text: &str) -> Result<String> {
        let uuid = uuid();
        self.write(json!({
            "type": "user",
            "uuid": uuid,
            "message": { "role": "user", "content": text },
        }))?;
        Ok(uuid)
    }

    pub fn interrupt(&mut self) -> Result<()> {
        self.next_req += 1;
        self.write(json!({
            "type": "control_request",
            "request_id": format!("jarvis-{}", self.next_req),
            "request": { "subtype": "interrupt" },
        }))
    }

    /// Deja correr la herramienta; para `AskUserQuestion`, `input` ya lleva las `answers`.
    pub fn allow(&mut self, request_id: &str, input: Value) -> Result<()> {
        self.respond(request_id, json!({ "behavior": "allow", "updatedInput": input }))
    }

    pub fn deny(&mut self, request_id: &str, why: &str) -> Result<()> {
        self.respond(request_id, json!({ "behavior": "deny", "message": why }))
    }

    /// Vale desde el turno siguiente; el `init` de ese turno trae el modelo nuevo.
    pub fn set_model(&mut self, model: &str) -> Result<()> {
        self.next_req += 1;
        self.write(json!({
            "type": "control_request",
            "request_id": format!("jarvis-{}", self.next_req),
            "request": { "subtype": "set_model", "model": model },
        }))
    }

    fn respond(&mut self, request_id: &str, response: Value) -> Result<()> {
        self.write(json!({
            "type": "control_response",
            "response": { "subtype": "success", "request_id": request_id, "response": response },
        }))
    }

    fn write(&mut self, v: Value) -> Result<()> {
        writeln!(self.stdin, "{v}")?;
        self.stdin.flush()?;
        Ok(())
    }
}

impl Drop for Claude {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn parse(v: &Value) -> Vec<ClaudeEvent> {
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    match v["type"].as_str() {
        Some("system") if v["subtype"] == "init" => vec![ClaudeEvent::Init {
            model: s(v, "model"),
            session: s(v, "session_id"),
            skills: v["skills"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default(),
        }],
        Some("system") if v["subtype"] == "compact_boundary" => {
            let m = &v["compact_metadata"];
            vec![ClaudeEvent::Compacted {
                pre: m["pre_tokens"].as_u64().unwrap_or(0),
                post: m["post_tokens"].as_u64().unwrap_or(0),
            }]
        }
        Some("system") if v["subtype"] == "thinking_tokens" => vec![ClaudeEvent::Thinking],
        Some("stream_event") => {
            let delta = &v["event"]["delta"];
            match delta["type"].as_str() {
                Some("text_delta") => vec![ClaudeEvent::Text(s(delta, "text"))],
                Some("thinking_delta") => vec![ClaudeEvent::Thinking],
                _ => vec![],
            }
        }
        // El texto ya llegó por deltas; de los mensajes completos solo interesan las herramientas.
        // Los de subagentes (parent_tool_use_id) se ignoran para no ensuciar el panel.
        Some("assistant") if v["parent_tool_use_id"].is_null() => v["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "tool_use")
            .map(|b| ClaudeEvent::ToolUse {
                id: s(b, "id"),
                name: s(b, "name"),
                detail: tool_detail(&b["input"]),
            })
            .collect(),
        Some("user") if v["isReplay"] == true => vec![ClaudeEvent::Taken(s(v, "uuid"))],
        Some("user") => v["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "tool_result")
            .map(|b| ClaudeEvent::ToolResult {
                id: s(b, "tool_use_id"),
                is_error: b["is_error"].as_bool().unwrap_or(false),
            })
            .collect(),
        Some("control_request") if v["request"]["subtype"] == "can_use_tool" => vec![ClaudeEvent::Ask {
            request_id: s(v, "request_id"),
            tool: s(&v["request"], "tool_name"),
            input: v["request"]["input"].clone(),
        }],
        Some("result") => vec![ClaudeEvent::Done {
            cost: v["total_cost_usd"].as_f64().unwrap_or(0.0),
            secs: v["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0,
            is_error: v["is_error"].as_bool().unwrap_or(false),
        }],
        _ => vec![],
    }
}

/// UUID v4 de /dev/urandom; el motor lo guarda tal cual en el transcript.
fn uuid() -> String {
    use std::io::Read;
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        b = t.as_nanos().to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

/// Lo más representativo de la entrada de una herramienta, en una línea.
pub fn tool_detail(input: &Value) -> String {
    const KEYS: &[&str] = &[
        "command", "file_path", "path", "pattern", "query", "url", "description", "prompt",
    ];
    let raw = KEYS
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .or_else(|| input.as_object()?.values().find_map(Value::as_str))
        .unwrap_or("");
    let home = std::env::var("HOME").unwrap_or_default();
    let line = raw.lines().next().unwrap_or("");
    if !home.is_empty() && line.contains(&home) {
        line.replace(&home, "~")
    } else {
        line.to_string()
    }
}
