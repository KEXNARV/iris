//! Sesiones anteriores de Claude Code, leídas de sus transcripts en `~/.claude/projects`.
//! `claude -p` no tiene `/resume`; Iris lo resuelve relanzando el motor con `--resume <id>`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::claude::{tool_detail, tool_output};
use crate::{Activity, Agent, Msg, Paso, Role, ToolStatus};

pub struct Session {
    pub id: String,
    pub title: String,
    pub age: String,
}

/// Claude Code guarda cada proyecto en un directorio con el cwd «aplanado»: todo lo que no es
/// alfanumérico se vuelve `-` (`/home/kex` → `-home-kex`).
fn project_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let flat: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    Some(crate::rutas::home()?.join(".claude").join("projects").join(flat))
}

/// Las más recientes primero, sin la sesión en curso ni las que no tienen ningún mensaje.
pub fn list(current: &str, max: usize) -> Vec<Session> {
    let Some(dir) = project_dir() else { return vec![] };
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|p| Some((fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));

    files
        .into_iter()
        .filter_map(|(mtime, path)| {
            let id = path.file_stem()?.to_string_lossy().into_owned();
            if id == current {
                return None;
            }
            let title = first_prompt(&path)?;
            Some(Session { id, title, age: age(mtime) })
        })
        .take(max)
        .collect()
}

/// `N` (posición en la lista) o un prefijo del id.
pub fn find(current: &str, arg: &str) -> Option<String> {
    let all = list(current, usize::MAX);
    if let Ok(n) = arg.parse::<usize>() {
        return all.into_iter().nth(n.checked_sub(1)?).map(|s| s.id);
    }
    all.into_iter().find(|s| s.id.starts_with(arg)).map(|s| s.id)
}

/// Una sesión retomada tal como la habría dejado vivirla: el chat, lo que hizo cada
/// herramienta y los subagentes que lanzó. Sin esto Ctrl+G quedaba vacío al retomar.
pub struct Replay {
    pub messages: Vec<Msg>,
    pub activity: Vec<Activity>,
    pub agents: Vec<Agent>,
}

/// Lee el transcript igual que llegan los eventos en vivo: cada herramienta entra en
/// `activity` y deja su fila en el chat en el punto de la conversación donde se usó; su
/// resultado (salida, error, duración) llega después en un mensaje de usuario.
pub fn replay(id: &str) -> Replay {
    let mut out = Replay { messages: vec![], activity: vec![], agents: vec![] };
    let Some(dir) = project_dir() else { return out };
    let path = dir.join(format!("{id}.jsonl"));
    for v in lines(&path) {
        if v["isSidechain"] == true || v["isMeta"] == true {
            continue;
        }
        let at = timestamp(&v);
        if let Some((role, text)) = turn(&v) {
            let images = images(&v);
            // Una respuesta llega en varios mensajes; se juntan mientras no haya una herramienta
            // de por medio (en vivo, una herramienta también corta el mensaje).
            match out.messages.last_mut() {
                Some(last) if last.role == Role::Assistant && role == Role::Assistant => {
                    last.text.push_str("\n\n");
                    last.text.push_str(&text);
                }
                _ => out.messages.push(Msg { role, text, waiting: None, images, tool: None }),
            }
        }
        match v["type"].as_str() {
            Some("assistant") => {
                for b in blocks(&v, "tool_use") {
                    out.activity.push(Activity {
                        id: s(b, "id"),
                        name: s(b, "name"),
                        detail: tool_detail(&b["input"]),
                        // Sin resultado en el transcript: el turno se cortó con ella en curso.
                        status: ToolStatus::Err,
                        started: at,
                        took: None,
                        input: b["input"].clone(),
                        output: String::new(),
                    });
                    let idx = out.activity.len() - 1;
                    out.messages.push(Msg { role: Role::Tool, text: String::new(), waiting: None, images: vec![], tool: Some(idx) });
                }
            }
            Some("user") => {
                for b in blocks(&v, "tool_result") {
                    let id = s(b, "tool_use_id");
                    if let Some(a) = out.activity.iter_mut().rev().find(|a| a.id == id) {
                        a.status = if b["is_error"] == true { ToolStatus::Err } else { ToolStatus::Ok };
                        a.took = at.checked_duration_since(a.started);
                        a.output = tool_output(&b["content"]);
                    }
                }
            }
            _ => {}
        }
    }
    out.agents = agents(&dir.join(id).join("subagents"), &out.activity);
    out
}

/// Los subagentes de la sesión, de `<sesión>/subagents/agent-*.jsonl` y su `.meta.json`
/// (`toolUseId`, `description`, `agentType`), en el orden en que se lanzaron.
fn agents(dir: &PathBuf, activity: &[Activity]) -> Vec<Agent> {
    let Ok(entries) = fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<Agent> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".meta.json"))
        .filter_map(|meta_path| {
            let meta: Value = serde_json::from_str(&fs::read_to_string(&meta_path).ok()?).ok()?;
            let transcript = PathBuf::from(meta_path.to_string_lossy().replace(".meta.json", ".jsonl"));
            let tool = s(&meta, "toolUseId");
            let call = activity.iter().find(|a| a.id == tool);
            let mut log: Vec<Paso> = vec![];
            let mut first: Option<Instant> = None;
            let mut last = None;
            for v in lines(&transcript) {
                let at = timestamp(&v);
                first.get_or_insert(at);
                last = Some(at);
                match v["type"].as_str() {
                    Some("assistant") => {
                        for b in v["message"]["content"].as_array().into_iter().flatten() {
                            match b["type"].as_str() {
                                Some("tool_use") => log.push(Paso::Herramienta(Activity {
                                    id: s(b, "id"),
                                    name: s(b, "name"),
                                    detail: tool_detail(&b["input"]),
                                    status: ToolStatus::Err,
                                    started: at,
                                    took: None,
                                    input: b["input"].clone(),
                                    output: String::new(),
                                })),
                                Some("text") if !s(b, "text").trim().is_empty() => log.push(Paso::Texto(s(b, "text"))),
                                _ => {}
                            }
                        }
                    }
                    Some("user") => {
                        for b in blocks(&v, "tool_result") {
                            let id = s(b, "tool_use_id");
                            let paso = log.iter_mut().rev().find_map(|p| match p {
                                Paso::Herramienta(t) if t.id == id => Some(t),
                                _ => None,
                            });
                            if let Some(t) = paso {
                                t.status = if b["is_error"] == true { ToolStatus::Err } else { ToolStatus::Ok };
                                t.took = at.checked_duration_since(t.started);
                                t.output = tool_output(&b["content"]);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let tools = log.iter().filter(|p| matches!(p, Paso::Herramienta(_))).count();
            let started = call.map(|a| a.started).or(first)?;
            Some(Agent {
                tool,
                kid: 0,
                description: s(&meta, "description"),
                kind: s(&meta, "agentType"),
                started,
                current: None,
                last: None,
                tools,
                ended: Some((last.unwrap_or(started), call.is_none_or(|a| a.status != ToolStatus::Err))),
                log,
            })
        })
        .collect();
    out.sort_by_key(|a| a.started);
    out
}

fn blocks<'a>(v: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
    v["message"]["content"].as_array().into_iter().flatten().filter(move |b| b["type"] == kind)
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or_default().to_string()
}

/// El `timestamp` del transcript (`2026-10-06T21:24:17.840Z`, siempre UTC) llevado a un
/// `Instant` de este proceso, para que duraciones y «hace cuánto» salgan como en vivo.
fn timestamp(v: &Value) -> Instant {
    let now = Instant::now();
    let Some(ms) = v["timestamp"].as_str().and_then(epoch_ms) else { return now };
    let then = UNIX_EPOCH + Duration::from_millis(ms);
    let ago = SystemTime::now().duration_since(then).unwrap_or_default();
    now.checked_sub(ago).unwrap_or(now)
}

fn epoch_ms(t: &str) -> Option<u64> {
    let n = |r: std::ops::Range<usize>| t.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (h, mi, sec) = (n(11..13)?, n(14..16)?, n(17..19)?);
    let frac = t.get(19..).filter(|r| r.starts_with('.')).map(|r| {
        let digits: String = r[1..].chars().take_while(char::is_ascii_digit).take(3).collect();
        format!("{digits:0<3}").parse::<i64>().unwrap_or(0)
    });
    // Días desde 1970-01-01 (algoritmo de Howard Hinnant, calendario civil).
    let (y, m) = if mo <= 2 { (y - 1, mo + 9) } else { (y, mo - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + mi * 60 + sec;
    u64::try_from(secs * 1000 + frac.unwrap_or(0)).ok()
}

fn lines(path: &PathBuf) -> impl Iterator<Item = Value> {
    fs::File::open(path)
        .ok()
        .into_iter()
        .flat_map(|f| BufReader::new(f).lines().map_while(Result::ok))
        .filter_map(|l| serde_json::from_str(&l).ok())
}

fn first_prompt(path: &PathBuf) -> Option<String> {
    lines(path).find_map(|v| match turn(&v) {
        Some((Role::User, t)) => Some(t.lines().next().unwrap_or("").to_string()),
        _ => None,
    })
}

/// Solo texto escrito por una persona o por Claude en el hilo principal: fuera resultados de
/// herramientas, subagentes, mensajes meta y las salidas de comandos locales (`<local-command…>`).
/// Las imágenes que mandaste en ese mensaje, como miniaturas para el chat.
fn images(v: &Value) -> Vec<std::rc::Rc<crate::miniatura::Thumb>> {
    v["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["type"] == "image")
        .filter_map(|b| b["source"]["data"].as_str())
        .filter_map(crate::miniatura::from_base64)
        .collect()
}

fn turn(v: &Value) -> Option<(Role, String)> {
    if v["isSidechain"] == true || v["isMeta"] == true {
        return None;
    }
    let role = match v["type"].as_str()? {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        _ => return None,
    };
    let content = &v["message"]["content"];
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => {
            let text = blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            match blocks.iter().filter(|b| b["type"] == "image").count() {
                0 => text,
                n => format!("▣ {n} imagen{}\n{text}", if n == 1 { "" } else { "es" }),
            }
        }
        _ => return None,
    };
    let text = text.trim();
    if text.is_empty() || text.starts_with('<') {
        return None;
    }
    Some((role, text.to_string()))
}

fn age(t: SystemTime) -> String {
    let s = SystemTime::now().duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
    match s {
        0..3600 => format!("hace {} min", s / 60),
        3600..86_400 => format!("hace {} h", s / 3600),
        _ => format!("hace {} d", s / 86_400),
    }
}
