//! surco (~/code/surco), the player: while Iris listens or speaks, the music is paused, and
//! when it ends it resumes. Only what Iris paused: if you had already stopped it, it's left alone.
//!
//! surco is socket-activated (systemd), so `surco.sock` exists even when nothing is running,
//! and connecting would wake it up. That's why systemd is asked first whether the service is
//! already alive; if not, there is no music to pause.

use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

/// Between sentences, or while the engine thinks up the reply it is going to say, the state
/// goes off for a moment: resuming right away would make the music peek through in bits.
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
            // Whether it was Iris that paused, and therefore its job to resume.
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
                        // If you pressed play yourself meanwhile, it's already playing; if there's no
                        // track, there is nothing to resume.
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

    /// Called on every loop: `true` while silence is needed. Only the changes travel.
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
    // With SURCO_SOCKET it points to a test instance, outside systemd.
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

/// `Some(true)` playing, `Some(false)` paused with something loaded, `None` no surco or no track.
/// surco's `idle` is no use here: it's mpv's `core-idle`, which is also true while paused.
fn playing() -> Option<bool> {
    let v = send(r#"{"cmd":"status"}"#)?;
    let p = v.get("payload")?;
    if p.get("current")?.is_null() {
        return None;
    }
    Some(!p.get("playback")?.get("paused")?.as_bool()?)
}

/// One request, one line back. None of this may hang Iris: everything with a timeout.
fn send(req: &str) -> Option<serde_json::Value> {
    // surco is Linux-only (UNIX socket and systemd): on Windows there is nothing to pause.
    #[cfg(not(unix))]
    {
        let _ = req;
        return None;
    }
    #[cfg(unix)]
    send_unix(req)
}

#[cfg(unix)]
fn send_unix(req: &str) -> Option<serde_json::Value> {
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

