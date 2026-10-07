//! surco (~/code/surco), el reproductor: mientras JARVIS escucha o habla, la música se pausa,
//! y al terminar se reanuda. Solo lo que JARVIS pausó: si ya la habías parado tú, no la toca.
//!
//! surco se activa por socket (systemd), así que `surco.sock` existe aunque no haya nada
//! corriendo, y conectarse lo despertaría. Por eso primero se pregunta a systemd si el
//! servicio ya está vivo; si no, no hay música que pausar.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

/// Entre frases, o mientras el motor piensa la respuesta que va a decir, el estado se apaga
/// un momento: reanudar en seguida haría que la música asome a pedazos.
const RESUME_DELAY: Duration = Duration::from_millis(1200);
const IO_TIMEOUT: Duration = Duration::from_millis(500);

pub struct Surco {
    tx: Sender<bool>,
    quiet: bool,
}

impl Surco {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<bool>();
        thread::spawn(move || {
            // Si fue JARVIS quien pausó, y por lo tanto le toca reanudar.
            let mut paused_by_us = false;
            let mut pending_resume = false;
            loop {
                let msg = if pending_resume { rx.recv_timeout(RESUME_DELAY) } else { rx.recv().map_err(|_| RecvTimeoutError::Disconnected) };
                match msg {
                    Ok(true) => {
                        pending_resume = false;
                        if !paused_by_us && playing() == Some(true) {
                            paused_by_us = send(r#"{"cmd":"pause"}"#).is_some();
                        }
                    }
                    Ok(false) => pending_resume = paused_by_us,
                    Err(RecvTimeoutError::Timeout) => {
                        pending_resume = false;
                        paused_by_us = false;
                        // Si mientras tanto le diste play tú, ya suena; si ya no hay pista, no hay
                        // nada que reanudar.
                        if playing() == Some(false) {
                            send(r#"{"cmd":"resume"}"#);
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        });
        Surco { tx, quiet: false }
    }

    /// Se llama en cada vuelta: `true` mientras haga falta silencio. Solo los cambios viajan.
    pub fn quiet(&mut self, want: bool) {
        if want != self.quiet {
            self.quiet = want;
            let _ = self.tx.send(want);
        }
    }
}

fn socket() -> PathBuf {
    if let Some(p) = std::env::var_os("SURCO_SOCKET") {
        return PathBuf::from(p);
    }
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    dir.join("surco.sock")
}

fn alive() -> bool {
    // Con SURCO_SOCKET se apunta a una instancia de pruebas, fuera de systemd.
    if std::env::var_os("SURCO_SOCKET").is_some() {
        return true;
    }
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "surco.service"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// `Some(true)` sonando, `Some(false)` en pausa con algo cargado, `None` sin surco o sin pista.
/// El `idle` de surco no sirve aquí: es el `core-idle` de mpv, que también es cierto en pausa.
fn playing() -> Option<bool> {
    let v = send(r#"{"cmd":"status"}"#)?;
    let p = v.get("payload")?;
    if p.get("current")?.is_null() {
        return None;
    }
    Some(!p.get("playback")?.get("paused")?.as_bool()?)
}

/// Una petición, una línea de vuelta. Nada de esto debe colgar a JARVIS: todo con timeout.
fn send(req: &str) -> Option<serde_json::Value> {
    if !alive() {
        return None;
    }
    let mut s = UnixStream::connect(socket()).ok()?;
    s.set_read_timeout(Some(IO_TIMEOUT)).ok()?;
    s.set_write_timeout(Some(IO_TIMEOUT)).ok()?;
    s.write_all(format!("{req}\n").as_bytes()).ok()?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).ok()?;
    let v: serde_json::Value = serde_json::from_str(&line).ok()?;
    (v.get("status")?.as_str()? != "error").then_some(v)
}

