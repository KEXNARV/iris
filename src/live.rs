//! The live connection with the store (`/api/live`, a WebSocket): what happens there arrives
//! here as it happens, and Iris tells the web what it's doing.
//!
//! In: a draft that Iris Studio is editing (written to disk as it changes, so the core shows it
//! live), a review of one of your packages, something published or revoked (the check for
//! updates runs right away), a new Iris. Out: the state of the core and the buddy, and the
//! events (for the web to show your Iris as it is).
//!
//! It reconnects on its own, waiting a little longer each time; the periodic checks stay as
//! they were, so nothing depends on it being up.

use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::app::AppEvent;

pub enum LiveEvent {
    /// Studio changed a draft: its files are already written (the core reloads from disk).
    Draft(String),
    /// One of your packages was reviewed.
    Review { name: String, version: String, status: String, reason: Option<String> },
    /// Something was published or revoked: look for updates now.
    Store,
    /// A new Iris was announced: check it now.
    IrisVersion,
    /// Just connected: whatever state Iris is in goes out, and the checks run.
    Connected,
}

/// Starts the connection in the background; what's sent to the returned channel goes out.
pub fn spawn(tx: Sender<AppEvent>) -> Sender<Value> {
    let (out_tx, out_rx) = channel::<Value>();
    std::thread::spawn(move || {
        // rustls needs its crypto chosen once per process: ring (no C toolchain beyond cc).
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut wait = Duration::from_secs(2);
        loop {
            let started = Instant::now();
            if let Err(e) = session(&tx, &out_rx) {
                if std::env::var_os("IRIS_LIVE_DEBUG").is_some() {
                    eprintln!("live: {e}");
                }
            }
            // A connection that lasted resets the wait; one that failed right away doubles it.
            wait = if started.elapsed() > Duration::from_secs(60) { Duration::from_secs(2) } else { (wait * 2).min(Duration::from_secs(120)) };
            std::thread::sleep(wait);
        }
    });
    out_tx
}

fn url() -> String {
    let base = crate::net::base();
    let ws = base.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
    format!("{ws}/api/live")
}

/// One connection, until it drops.
fn session(tx: &Sender<AppEvent>, out: &Receiver<Value>) -> Result<(), String> {
    let (mut ws, _) = tungstenite::connect(url()).map_err(|e| e.to_string())?;
    let mut hello = json!({ "t": "hello", "client": "iris" });
    if let Some(t) = crate::store::token() {
        hello["token"] = Value::String(t);
    }
    ws.send(Message::text(hello.to_string())).map_err(|e| e.to_string())?;
    // Reading waits a little at a time, so what's queued to send doesn't wait for a message.
    set_timeout(&mut ws, Some(Duration::from_millis(200)));
    // Anything said while it was down is dropped: only the current state matters. And what
    // happened meanwhile is caught up with: the checks run now.
    while out.try_recv().is_ok() {}
    let _ = tx.send(AppEvent::Live(LiveEvent::Connected));
    loop {
        match ws.read() {
            Ok(Message::Text(t)) => {
                let v: Value = serde_json::from_str(t.as_str()).unwrap_or_default();
                if let Some(ev) = handle(&v) {
                    if tx.send(AppEvent::Live(ev)).is_err() {
                        return Ok(());
                    }
                }
            }
            Ok(Message::Close(_)) => return Err("closed".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return Err(e.to_string()),
        }
        while let Ok(v) = out.try_recv() {
            ws.send(Message::text(v.to_string())).map_err(|e| e.to_string())?;
        }
    }
}

fn set_timeout(ws: &mut WebSocket<MaybeTlsStream<TcpStream>>, t: Option<Duration>) {
    let tcp = match ws.get_mut() {
        MaybeTlsStream::Plain(s) => s,
        MaybeTlsStream::Rustls(s) => s.get_mut(),
        _ => return,
    };
    let _ = tcp.set_read_timeout(t);
}

/// A message from the server, turned into what Iris does with it.
fn handle(v: &Value) -> Option<LiveEvent> {
    match v["t"].as_str()? {
        "draft" => {
            let name = v["name"].as_str()?;
            write_draft(name, v["files"].as_object()?)?;
            Some(LiveEvent::Draft(name.to_string()))
        }
        "review" => Some(LiveEvent::Review {
            name: v["name"].as_str()?.to_string(),
            version: v["version"].as_str().unwrap_or_default().to_string(),
            status: v["status"].as_str().unwrap_or_default().to_string(),
            reason: v["reason"].as_str().map(str::to_string),
        }),
        "store" => Some(LiveEvent::Store),
        "iris_version" => Some(LiveEvent::IrisVersion),
        _ => None,
    }
}

/// A draft's files into its folder (binary ones come in base64, as `<name>.b64`), as the
/// periodic sync writes them; the core sees the folder change and reloads it.
pub fn write_draft(name: &str, files: &serde_json::Map<String, Value>) -> Option<()> {
    if name.is_empty() || name.starts_with('.') || name.contains('/') || name.contains('\\') {
        return None;
    }
    let dir = crate::paths::config()?.join("buddies").join(name);
    std::fs::create_dir_all(&dir).ok()?;
    for (f, body) in files {
        if f.contains('/') || f.contains('\\') || f.starts_with('.') {
            continue;
        }
        let Some(text) = body.as_str() else { continue };
        // Only what changed: writing the same bytes would still touch the folder.
        if let Some(bin) = f.strip_suffix(".b64") {
            use base64::Engine;
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(text.trim()) {
                if std::fs::read(dir.join(bin)).ok().as_deref() != Some(&bytes[..]) {
                    let _ = std::fs::write(dir.join(bin), bytes);
                }
            }
            continue;
        }
        if std::fs::read_to_string(dir.join(f)).ok().as_deref() != Some(text) {
            let _ = std::fs::write(dir.join(f), text);
        }
    }
    Some(())
}

/// What Iris tells the web: the core's state and the buddy.
pub fn state(state: &str, buddy: &str) -> Value {
    json!({ "t": "state", "state": state, "buddy": buddy })
}

pub fn event(ev: &str) -> Value {
    json!({ "t": "event", "ev": ev })
}
