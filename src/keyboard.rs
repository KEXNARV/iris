//! The keyboard as part of the core: tells ghubd (ghub-linux) what Iris is doing, and ghubd
//! draws it on top of its effect. One-line messages to a UNIX socket:
//! `state thinking`, `level 0.42`, `event error`. If ghubd isn't running, they're silently lost.

#[cfg(unix)]
use std::os::unix::net::UnixDatagram;

/// ghubd is Linux-only: on Windows, a socket that sends nothing.
#[cfg(not(unix))]
struct UnixDatagram;

#[cfg(not(unix))]
impl UnixDatagram {
    fn unbound() -> std::io::Result<Self> {
        Ok(UnixDatagram)
    }

    fn send_to(&self, _: &[u8], _: &std::path::Path) -> std::io::Result<usize> {
        Ok(0)
    }
}
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::core::{Event, State};

pub struct Keyboard {
    sock: Option<UnixDatagram>,
    path: PathBuf,
    last: Option<State>,
    /// With several Iris open, ghubd shows the one doing something: each one sends its pid.
    id: u32,
    beat: Instant,
    level_at: Instant,
}

impl Keyboard {
    pub fn new() -> Self {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        Keyboard {
            sock: UnixDatagram::unbound().ok(),
            path: dir.join("ghub-linux.sock"),
            last: None,
            id: std::process::id(),
            beat: Instant::now(),
            level_at: Instant::now(),
        }
    }

    fn send(&self, msg: &str) {
        if let Some(s) = &self.sock {
            let _ = s.send_to(msg.as_bytes(), &self.path);
        }
    }

    /// Every frame: the state if it changed (and once a second as a heartbeat, so ghubd knows
    /// Iris is still alive), and the voice level ~30 times a second while listening.
    pub fn tick(&mut self, state: State, level: f32) {
        if self.last != Some(state) || self.beat.elapsed() >= Duration::from_secs(1) {
            self.last = Some(state);
            self.beat = Instant::now();
            self.send(&format!("state {} {}", self.id, format!("{state:?}").to_lowercase()));
        }
        if matches!(state, State::Listening | State::Speaking) && self.level_at.elapsed() >= Duration::from_millis(33) {
            self.level_at = Instant::now();
            self.send(&format!("level {} {level:.3}", self.id));
        }
    }

    pub fn event(&self, ev: Event) {
        let name = match ev {
            Event::Error => "error",
            Event::Done | Event::Pass => "done",
            _ => return,
        };
        self.send(&format!("event {} {name}", self.id));
    }
}
