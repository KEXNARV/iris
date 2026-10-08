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
    /// `input` va entero: de ahí salen, por ejemplo, las tareas de TodoWrite.
    ToolUse { id: String, name: String, detail: String, input: Value },
    /// `output`: lo que devolvió la herramienta (texto), para verlo con Ctrl+G.
    ToolResult { id: String, is_error: bool, output: String },
    /// `window`: ventana de contexto del modelo, si el motor la informó.
    Done { cost: f64, secs: f64, is_error: bool, window: Option<u64> },
    /// Tokens de contexto que lleva la conversación (entrada + caché del último mensaje).
    Usage(u64),
    /// El motor espera respuesta del usuario: una `AskUserQuestion` o un permiso.
    Ask { request_id: String, tool: String, input: Value },
    /// El motor tomó el mensaje con este id (lo repite al leerlo, por `--replay-user-messages`).
    Taken(String),
    /// `/compact` terminó: tokens de contexto antes y después.
    Compacted { pre: u64, post: u64 },
    /// Nació un subagente (`task_started` de tipo agente). `tool` es el id de la llamada a
    /// `Agent` que lo lanzó: con eso se le atribuyen sus mensajes (`parent_tool_use_id`).
    AgentStarted { tool: String, description: String, kind: String },
    /// Un subagente pidió una herramienta (`id` es el de esa llamada), o recibió su resultado.
    AgentTool { tool: String, id: String, name: String, detail: String, input: Value },
    AgentToolResult { tool: String, id: String, is_error: bool, output: String },
    /// Lo que escribió un subagente entre herramientas (llega por mensaje completo, no en vivo).
    AgentText { tool: String, text: String },
    /// Terminó un subagente: `ok` si completó, no si falló o lo mataron.
    AgentDone { tool: String, ok: bool },
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
                // Sin pedir permiso para cada herramienta.
                "--dangerously-skip-permissions",
                // Sin esto `AskUserQuestion` no existe en `-p`; con esto el motor nos pregunta a
                // nosotros por stdout (`control_request` `can_use_tool`) y espera la respuesta.
                "--permission-prompt-tool",
                "stdio",
                // La interfaz convierte en adjunto toda ruta a un archivo que exista (adjunto.rs).
                "--append-system-prompt",
                "Hablas con el usuario a través de Iris, una TUI propia. Cuando le entregues un archivo \
                 (un reporte, un Excel, una imagen…), escribe su ruta completa en la respuesta: Iris la \
                 muestra como adjunto y un clic lo copia al portapapeles para pegarlo donde quiera. \
                 No digas que no puedes pasarle archivos.",
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
    pub fn send(&mut self, text: &str, images: &[crate::clip::Image]) -> Result<String> {
        let uuid = uuid();
        // Con imágenes el contenido va en bloques: primero las imágenes, después el texto.
        let content = if images.is_empty() {
            json!(text)
        } else {
            let mut blocks: Vec<Value> = images
                .iter()
                .map(|i| json!({ "type": "image", "source": { "type": "base64", "media_type": i.media, "data": i.data } }))
                .collect();
            if !text.is_empty() {
                blocks.push(json!({ "type": "text", "text": text }));
            }
            Value::Array(blocks)
        };
        self.write(json!({
            "type": "user",
            "uuid": uuid,
            "message": { "role": "user", "content": content },
        }))?;
        Ok(uuid)
    }

    pub fn interrupt(&mut self) -> Result<()> {
        self.next_req += 1;
        self.write(json!({
            "type": "control_request",
            "request_id": format!("iris-{}", self.next_req),
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
            "request_id": format!("iris-{}", self.next_req),
            "request": { "subtype": "set_model", "model": model },
        }))
    }

    /// `None` vuelve al nivel por defecto del modelo.
    pub fn set_effort(&mut self, level: Option<&str>) -> Result<()> {
        self.next_req += 1;
        self.write(json!({
            "type": "control_request",
            "request_id": format!("iris-{}", self.next_req),
            "request": { "subtype": "apply_flag_settings", "settings": { "effortLevel": level } },
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
        // Sin esto el proceso muerto queda como zombi hasta que Iris termina.
        let _ = self.child.wait();
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
        Some("system") if v["subtype"] == "task_started" && v["task_type"] == "local_agent" => {
            vec![ClaudeEvent::AgentStarted {
                tool: s(v, "tool_use_id"),
                description: s(v, "description"),
                kind: s(v, "subagent_type"),
            }]
        }
        // El final llega dos veces (`task_updated` con el estado y `task_notification` con el
        // resumen) y no siempre en ese orden: la app lo toma una vez.
        Some("system") if v["subtype"] == "task_notification" && !v["tool_use_id"].is_null() => {
            vec![ClaudeEvent::AgentDone { tool: s(v, "tool_use_id"), ok: v["status"] == "completed" }]
        }
        // Lo de un subagente llega con el id de la llamada que lo lanzó: sus herramientas (el
        // núcleo las muestra en el hijo) y lo que escribe (para verlo en Ctrl+G, Agentes).
        Some("assistant") if !v["parent_tool_use_id"].is_null() => v["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|b| match b["type"].as_str() {
                Some("tool_use") => Some(ClaudeEvent::AgentTool {
                    tool: s(v, "parent_tool_use_id"),
                    id: s(b, "id"),
                    name: s(b, "name"),
                    detail: tool_detail(&b["input"]),
                    input: b["input"].clone(),
                }),
                Some("text") if !s(b, "text").trim().is_empty() => {
                    Some(ClaudeEvent::AgentText { tool: s(v, "parent_tool_use_id"), text: s(b, "text") })
                }
                _ => None,
            })
            .collect(),
        Some("user") if !v["parent_tool_use_id"].is_null() => blocks(v, "tool_result")
            .map(|b| ClaudeEvent::AgentToolResult {
                tool: s(v, "parent_tool_use_id"),
                id: s(b, "tool_use_id"),
                is_error: b["is_error"].as_bool().unwrap_or(false),
                output: tool_output(&b["content"]),
            })
            .collect(),
        // El texto ya llegó por deltas; de los mensajes completos solo interesan las herramientas.
        Some("assistant") => {
            let mut out: Vec<ClaudeEvent> = v["message"]["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| b["type"] == "tool_use")
                .map(|b| ClaudeEvent::ToolUse {
                    id: s(b, "id"),
                    name: s(b, "name"),
                    detail: tool_detail(&b["input"]),
                    input: b["input"].clone(),
                })
                .collect();
            let u = &v["message"]["usage"];
            let used: u64 = ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"]
                .iter()
                .filter_map(|k| u[*k].as_u64())
                .sum();
            if used > 0 {
                out.push(ClaudeEvent::Usage(used));
            }
            out
        }
        Some("user") if v["isReplay"] == true => vec![ClaudeEvent::Taken(s(v, "uuid"))],
        Some("user") => v["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "tool_result")
            .map(|b| ClaudeEvent::ToolResult {
                id: s(b, "tool_use_id"),
                is_error: b["is_error"].as_bool().unwrap_or(false),
                output: tool_output(&b["content"]),
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
            // Con subagentes hay varios modelos; la ventana de la conversación es la mayor.
            window: v["modelUsage"]
                .as_object()
                .and_then(|m| m.values().filter_map(|x| x["contextWindow"].as_u64()).max()),
        }],
        _ => vec![],
    }
}

/// Los bloques de contenido de un mensaje que son de tipo `kind`.
fn blocks<'a>(v: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    v["message"]["content"].as_array().into_iter().flatten().filter(move |b| b["type"] == kind)
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
    let home = crate::rutas::home().map(|h| h.display().to_string()).unwrap_or_default();
    let line = raw.lines().next().unwrap_or("");
    if !home.is_empty() && line.contains(&home) {
        line.replace(&home, "~")
    } else {
        line.to_string()
    }
}

/// El texto que devolvió una herramienta: viene como texto o como bloques; se corta en
/// 40 000 caracteres para no guardar salidas enormes.
pub fn tool_output(c: &Value) -> String {
    let text = match c {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .map(|b| match b["type"].as_str() {
                Some("text") => b["text"].as_str().unwrap_or("").to_string(),
                Some("image") => "[imagen]".to_string(),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    if text.chars().count() > 40_000 {
        let cut: String = text.chars().take(40_000).collect();
        format!("{cut}\n… (cortado)")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Eventos reales de `claude -p` con un subagente (recortados): nace, pide un Bash, recibe
    /// su resultado y termina.
    #[test]
    fn sigue_a_un_subagente() {
        let lines = [
            r#"{"type":"system","subtype":"task_started","task_id":"a9","tool_use_id":"toolu_A","description":"Ejecutar ls","subagent_type":"general-purpose","is_backgrounded":false,"task_type":"local_agent"}"#,
            r#"{"type":"system","subtype":"task_started","task_id":"b1","tool_use_id":"toolu_B","description":"Loop","is_backgrounded":true,"task_type":"local_bash"}"#,
            r#"{"type":"assistant","parent_tool_use_id":"toolu_A","message":{"content":[{"type":"tool_use","id":"toolu_C","name":"Bash","input":{"command":"ls /tmp | head -3"}}]}}"#,
            r#"{"type":"user","parent_tool_use_id":"toolu_A","message":{"content":[{"tool_use_id":"toolu_C","type":"tool_result","content":"x","is_error":false}]}}"#,
            r#"{"type":"system","subtype":"task_notification","task_id":"a9","tool_use_id":"toolu_A","status":"completed"}"#,
        ];
        let evs: Vec<ClaudeEvent> = lines.iter().flat_map(|l| parse(&serde_json::from_str(l).unwrap())).collect();
        assert!(matches!(&evs[0], ClaudeEvent::AgentStarted { tool, kind, .. } if tool == "toolu_A" && kind == "general-purpose"));
        // El bash de fondo no es un hijo.
        assert!(matches!(&evs[1], ClaudeEvent::AgentTool { tool, name, .. } if tool == "toolu_A" && name == "Bash"));
        assert!(matches!(&evs[2], ClaudeEvent::AgentToolResult { tool, id, is_error: false, output } if tool == "toolu_A" && id == "toolu_C" && output == "x"));
        assert!(matches!(&evs[3], ClaudeEvent::AgentDone { tool, ok: true } if tool == "toolu_A"));
        assert_eq!(evs.len(), 4);
    }

    #[test]
    fn texto_de_un_subagente() {
        let v: Value = serde_json::from_str(r#"{"type":"assistant","parent_tool_use_id":"toolu_A","message":{"content":[{"type":"text","text":"Reviso los logs."},{"type":"tool_use","id":"toolu_D","name":"Read","input":{"file_path":"/x"}}]}}"#).unwrap();
        let evs = parse(&v);
        assert!(matches!(&evs[0], ClaudeEvent::AgentText { tool, text } if tool == "toolu_A" && text == "Reviso los logs."));
        assert!(matches!(&evs[1], ClaudeEvent::AgentTool { id, name, .. } if id == "toolu_D" && name == "Read"));
    }
}
