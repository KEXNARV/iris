//! Sesiones anteriores de Claude Code, leídas de sus transcripts en `~/.claude/projects`.
//! `claude -p` no tiene `/resume`; jarvis lo resuelve relanzando el motor con `--resume <id>`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::SystemTime;

use serde_json::Value;

use crate::{Msg, Role};

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
    Some(PathBuf::from(std::env::var("HOME").ok()?).join(".claude/projects").join(flat))
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

/// La conversación visible (lo que dijo cada quien), para repintar el chat al retomarla.
pub fn history(id: &str) -> Vec<Msg> {
    let Some(path) = project_dir().map(|d| d.join(format!("{id}.jsonl"))) else { return vec![] };
    let mut out: Vec<Msg> = vec![];
    for (role, text) in lines(&path).filter_map(|v| turn(&v)) {
        // Una respuesta con herramientas de por medio llega en varios mensajes; se juntan.
        match out.last_mut() {
            Some(last) if last.role == Role::Assistant && role == Role::Assistant => {
                last.text.push_str("\n\n");
                last.text.push_str(&text);
            }
            _ => out.push(Msg { role, text, waiting: None }),
        }
    }
    out
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
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
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
