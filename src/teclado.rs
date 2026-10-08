//! El teclado como parte del núcleo: le cuenta a ghubd (ghub-linux) qué está haciendo Iris,
//! y ghubd lo dibuja encima de su efecto. Mensajes de una línea a un socket UNIX:
//! `state thinking`, `level 0.42`, `event error`. Si ghubd no corre, se pierden sin error.

use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::nucleo::{Event, State};

pub struct Teclado {
    sock: Option<UnixDatagram>,
    path: PathBuf,
    last: Option<State>,
    /// Con varios Iris abiertos, ghubd muestra el que está haciendo algo: cada uno manda su pid.
    id: u32,
    beat: Instant,
    level_at: Instant,
}

impl Teclado {
    pub fn new() -> Self {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        Teclado {
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

    /// Cada cuadro: el estado si cambió (y una vez por segundo de latido, así ghubd sabe que
    /// Iris sigue vivo), y el nivel de la voz a ~30 por segundo mientras escucha.
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
