//! Engine: a live `claude -p` process that speaks stream-json over stdin/stdout.

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
    /// `input` goes whole: that's where, for example, TodoWrite's tasks come from.
    ToolUse { id: String, name: String, detail: String, input: Value },
    /// `output`: what the tool returned (text), to view it with Ctrl+G.
    ToolResult { id: String, is_error: bool, output: String },
    /// `window`: the model's context window, if the engine reported it.
    Done { cost: f64, secs: f64, is_error: bool, window: Option<u64> },
    /// Context tokens the conversation is using (input + cache of the last message).
    Usage(u64),
    /// The engine awaits a reply from the user: an `AskUserQuestion` or a permission.
    /// `suggestions`: the rules the engine proposes so it doesn't ask again.
    Ask { request_id: String, tool: String, input: Value, suggestions: Value },
    /// The engine took the message with this id (echoed on read, via `--replay-user-messages`).
    Taken(String),
    /// `/compact` finished: context tokens before and after.
    Compacted { pre: u64, post: u64 },
    /// A subagent was born (`task_started` of agent type). `tool` is the id of the `Agent` call
    /// that launched it: that's how its messages are attributed to it (`parent_tool_use_id`).
    AgentStarted { tool: String, description: String, kind: String },
    /// A subagent requested a tool (`id` is that call's), or received its result.
    AgentTool { tool: String, id: String, name: String, detail: String, input: Value },
    AgentToolResult { tool: String, id: String, is_error: bool, output: String },
    /// What a subagent wrote between tools (arrives as a whole message, not live).
    AgentText { tool: String, text: String },
    /// A subagent finished: `ok` if it completed, not if it failed or was killed.
    AgentDone { tool: String, ok: bool },
    Stderr(String),
    Exited,
}

/// Each launched process carries its number: on restart or resume, the old one dies and its
/// `Exited` arrives after launching the new one; with the number, the app spots and ignores it.
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
                // Echoes each of our messages the moment it reads it: that's how we know when one
                // written mid-turn stops being pending.
                "--replay-user-messages",
                // Without this `AskUserQuestion` doesn't exist in `-p`; with it the engine asks us
                // over stdout (`control_request` `can_use_tool`) and waits for the reply.
                "--permission-prompt-tool",
                "stdio",
                // The UI turns any path to an existing file into an attachment (attachment.rs).
                "--append-system-prompt",
                "Hablas con el usuario a través de Iris, una TUI propia. Cuando le entregues un archivo \
                 (un reporte, un Excel, una imagen…), escribe su ruta completa en la respuesta: Iris la \
                 muestra como adjunto y un clic lo copia al portapapeles para pegarlo donde quiera. \
                 No digas que no puedes pasarle archivos.",
            ])
            // Permissions, model and effort chosen in the welcome screen.
            .args(crate::settings::engine_args(extra_args))
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

    /// Returns the id with which the engine will report that it took it.
    pub fn send(&mut self, text: &str, images: &[crate::clip::Image]) -> Result<String> {
        let uuid = uuid();
        // With images the content goes in blocks: first the images, then the text.
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

    /// Lets the tool run; for `AskUserQuestion`, `input` already carries the `answers`.
    pub fn allow(&mut self, request_id: &str, input: Value) -> Result<()> {
        self.respond(request_id, json!({ "behavior": "allow", "updatedInput": input }))
    }

    /// Lets the tool run and adds the rules (`updatedPermissions`) for the rest of the engine's
    /// session: it won't ask about the same thing again.
    pub fn allow_always(&mut self, request_id: &str, input: Value, rules: Value) -> Result<()> {
        self.respond(request_id, json!({ "behavior": "allow", "updatedInput": input, "updatedPermissions": rules }))
    }

    pub fn deny(&mut self, request_id: &str, why: &str) -> Result<()> {
        self.respond(request_id, json!({ "behavior": "deny", "message": why }))
    }

    /// Takes effect from the next turn; that turn's `init` brings the new model.
    pub fn set_model(&mut self, model: &str) -> Result<()> {
        self.next_req += 1;
        self.write(json!({
            "type": "control_request",
            "request_id": format!("iris-{}", self.next_req),
            "request": { "subtype": "set_model", "model": model },
        }))
    }

    /// `None` goes back to the model's default level.
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
        // Without this the dead process stays a zombie until Iris exits.
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
        // The end arrives twice (`task_updated` with the status and `task_notification` with the
        // summary) and not always in that order: the app takes it once.
        Some("system") if v["subtype"] == "task_notification" && !v["tool_use_id"].is_null() => {
            vec![ClaudeEvent::AgentDone { tool: s(v, "tool_use_id"), ok: v["status"] == "completed" }]
        }
        // A subagent's stuff arrives with the id of the call that launched it: its tools (the core
        // shows them on the kid) and what it writes (to see it in Ctrl+G, Agents).
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
        // The text already arrived via deltas; from complete messages only the tools matter.
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
            suggestions: v["request"]["permission_suggestions"].clone(),
        }],
        Some("result") => vec![ClaudeEvent::Done {
            cost: v["total_cost_usd"].as_f64().unwrap_or(0.0),
            secs: v["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0,
            is_error: v["is_error"].as_bool().unwrap_or(false),
            // With subagents there are several models; the conversation's window is the largest.
            window: v["modelUsage"]
                .as_object()
                .and_then(|m| m.values().filter_map(|x| x["contextWindow"].as_u64()).max()),
        }],
        _ => vec![],
    }
}

/// The content blocks of a message that are of type `kind`.
fn blocks<'a>(v: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    v["message"]["content"].as_array().into_iter().flatten().filter(move |b| b["type"] == kind)
}

/// UUID v4 from /dev/urandom; the engine stores it as is in the transcript.
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

/// The most representative part of a tool's input, on one line.
pub fn tool_detail(input: &Value) -> String {
    const KEYS: &[&str] = &[
        "command", "file_path", "path", "pattern", "query", "url", "description", "prompt",
    ];
    let raw = KEYS
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .or_else(|| input.as_object()?.values().find_map(Value::as_str))
        .unwrap_or("");
    let home = crate::paths::home().map(|h| h.display().to_string()).unwrap_or_default();
    let line = raw.lines().next().unwrap_or("");
    if !home.is_empty() && line.contains(&home) {
        line.replace(&home, "~")
    } else {
        line.to_string()
    }
}

/// The text a tool returned: it comes as text or as blocks; it is cut at 40,000 characters
/// so huge outputs aren't stored.
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

    /// Real `claude -p` events with a subagent (trimmed): it is born, requests a Bash, receives
    /// its result and finishes.
    #[test]
    fn follows_a_subagent() {
        let lines = [
            r#"{"type":"system","subtype":"task_started","task_id":"a9","tool_use_id":"toolu_A","description":"Ejecutar ls","subagent_type":"general-purpose","is_backgrounded":false,"task_type":"local_agent"}"#,
            r#"{"type":"system","subtype":"task_started","task_id":"b1","tool_use_id":"toolu_B","description":"Loop","is_backgrounded":true,"task_type":"local_bash"}"#,
            r#"{"type":"assistant","parent_tool_use_id":"toolu_A","message":{"content":[{"type":"tool_use","id":"toolu_C","name":"Bash","input":{"command":"ls /tmp | head -3"}}]}}"#,
            r#"{"type":"user","parent_tool_use_id":"toolu_A","message":{"content":[{"tool_use_id":"toolu_C","type":"tool_result","content":"x","is_error":false}]}}"#,
            r#"{"type":"system","subtype":"task_notification","task_id":"a9","tool_use_id":"toolu_A","status":"completed"}"#,
        ];
        let evs: Vec<ClaudeEvent> = lines.iter().flat_map(|l| parse(&serde_json::from_str(l).unwrap())).collect();
        assert!(matches!(&evs[0], ClaudeEvent::AgentStarted { tool, kind, .. } if tool == "toolu_A" && kind == "general-purpose"));
        // The background bash is not a kid.
        assert!(matches!(&evs[1], ClaudeEvent::AgentTool { tool, name, .. } if tool == "toolu_A" && name == "Bash"));
        assert!(matches!(&evs[2], ClaudeEvent::AgentToolResult { tool, id, is_error: false, output } if tool == "toolu_A" && id == "toolu_C" && output == "x"));
        assert!(matches!(&evs[3], ClaudeEvent::AgentDone { tool, ok: true } if tool == "toolu_A"));
        assert_eq!(evs.len(), 4);
    }

    #[test]
    fn subagent_text() {
        let v: Value = serde_json::from_str(r#"{"type":"assistant","parent_tool_use_id":"toolu_A","message":{"content":[{"type":"text","text":"Reviso los logs."},{"type":"tool_use","id":"toolu_D","name":"Read","input":{"file_path":"/x"}}]}}"#).unwrap();
        let evs = parse(&v);
        assert!(matches!(&evs[0], ClaudeEvent::AgentText { tool, text } if tool == "toolu_A" && text == "Reviso los logs."));
        assert!(matches!(&evs[1], ClaudeEvent::AgentTool { id, name, .. } if id == "toolu_D" && name == "Read"));
    }
}
