//! Qué está haciendo Iris, visto desde el núcleo: los estados, los eventos sueltos y las
//! señales que manda la app en cada cuadro, y cómo una herramienta se traduce en estado.

use ratatui::style::Color;

use super::color::{apart, mix, rgb, PURPLE, YELLOW};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Booting,
    Sleeping,
    Idle,
    Typing,
    Listening,
    NoVoice,
    Transcribing,
    Thinking,
    Planning,
    Searching,
    Reading,
    Editing,
    Running,
    Testing,
    Git,
    Web,
    Delegating,
    Speaking,
    Asking,
    Compacting,
    Offline,
}

/// Cosas que pasan una vez, encima de cualquier estado.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// Una tecla en la entrada: respingo y onda corta.
    Key,
    /// Una herramienta terminó en error: destello rojo, sacudida y púas.
    Error,
    /// Fin de turno: rebote y anillo verde.
    Done,
    /// Esc a mitad de turno: se desinfla de golpe.
    Cancel,
    /// Whisper devolvió una alucinación y se descartó: niega con la mirada.
    Nope,
    /// Volvió un subagente: una gota llega desde la escala y se funde.
    Merge,
    /// Algo se copió al portapapeles: destello y guiño.
    Copy,
    /// Las pruebas pasaron: la escala se pinta de verde.
    Pass,
}

/// Lo que el núcleo necesita saber de la app en cada cuadro.
#[derive(Clone, Default)]
pub struct Signals {
    /// RMS del micrófono.
    pub level: f32,
    /// Contexto usado, 0..1.
    pub ctx: f64,
    /// Mensajes escritos a mitad de turno que esperan su lectura.
    pub queue: usize,
    /// Tareas de TodoWrite: (total, completadas).
    pub todos: (usize, usize),
    /// Subagentes vivos: (id, lo que están haciendo). Cada uno es un hijo del núcleo.
    pub kids: Vec<(u64, State)>,
    /// Segundos sin actividad.
    pub idle: f64,
    /// Menos movimiento (`/calm`).
    pub calm: bool,
    /// Lo que suena en el equipo, 0..1 (cava, como el teclado). En reposo, baila con eso.
    pub music: f32,
}

impl State {
    pub fn label(self) -> &'static str {
        use State::*;
        match self {
            Booting => "ARRANCANDO",
            Sleeping => "EN REPOSO",
            Idle => "EN ESPERA",
            Typing => "TE LEO",
            Listening => "ESCUCHANDO",
            NoVoice => "NO TE OIGO",
            Transcribing => "TRANSCRIBIENDO",
            Thinking => "PENSANDO",
            Planning => "PLANIFICANDO",
            Searching => "BUSCANDO",
            Reading => "LEYENDO",
            Editing => "EDITANDO",
            Running => "EJECUTANDO",
            Testing => "PROBANDO",
            Git => "GIT",
            Web => "EN LA RED",
            Delegating => "DELEGANDO",
            Speaking => "RESPONDIENDO",
            Asking => "ESPERANDO RESPUESTA",
            Compacting => "COMPACTANDO",
            Offline => "DESCONECTADO",
        }
    }

    /// Por familias: presencia en el acento del tema, la mente en azules y violetas, las
    /// herramientas del verde al rosa. Escuchar y transcribir, en el amarillo y el morado del
    /// teclado (ghubd). Lo de alrededor (escala, arcos, motas) va siempre en el acento: el
    /// color del cuerpo es el que dice el estado.
    pub(crate) fn rgb(self) -> [f64; 3] {
        use State::*;
        use crate::theme::{accent_darker, accent_rgb, accent_toward_white};
        let c = match self {
            Booting | Idle => return accent_rgb(),
            Sleeping => return accent_darker(0.43),
            Typing => return accent_toward_white(0.35),
            Speaking => return accent_toward_white(0.6),
            Listening => return YELLOW,
            NoVoice => return mix(YELLOW, [0.0; 3], 0.35),
            Transcribing => return PURPLE,
            Asking => [255.0, 205.0, 60.0],
            Thinking => [90.0, 150.0, 255.0],
            Planning => [125.0, 125.0, 255.0],
            Delegating => [175.0, 140.0, 255.0],
            Compacting => [160.0, 165.0, 205.0],
            Searching => [60.0, 215.0, 165.0],
            Reading => [70.0, 220.0, 215.0],
            Editing => [175.0, 235.0, 80.0],
            Testing => [120.0, 235.0, 120.0],
            Running => [235.0, 225.0, 90.0],
            Git => [255.0, 130.0, 70.0],
            Web => [255.0, 110.0, 170.0],
            Offline => return [90.0, 110.0, 125.0],
        };
        apart(c, accent_rgb())
    }

    pub fn color(self) -> Color {
        rgb(self.rgb())
    }

    /// Estos se muestran al instante; el resto espera a que el anterior haya durado su mínimo.
    pub(crate) fn urgent(self) -> bool {
        use State::*;
        matches!(self, Listening | NoVoice | Transcribing | Asking | Offline)
    }

    /// Lo mínimo que se muestra un estado, para que un Read de 50 ms no sea un parpadeo.
    pub(crate) fn dwell(self) -> f64 {
        use State::*;
        match self {
            Planning => 1.4,
            Searching | Reading | Editing | Running | Testing | Git | Web | Delegating => 0.6,
            Thinking => 0.35,
            _ => 0.25,
        }
    }

    pub(crate) fn is_tool(self) -> bool {
        use State::*;
        matches!(self, Planning | Searching | Reading | Editing | Running | Testing | Git | Web | Delegating)
    }

}

/// Qué estado corresponde a una herramienta. `detail` es la primera línea de su entrada
/// (el comando, en el caso de Bash).
pub fn tool_state(name: &str, detail: &str) -> State {
    match name {
        "Read" | "NotebookRead" => State::Reading,
        "Grep" | "Glob" | "LS" => State::Searching,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => State::Editing,
        "WebFetch" | "WebSearch" => State::Web,
        "Agent" | "Task" => State::Delegating,
        "TodoWrite" => State::Planning,
        "Bash" => bash_state(detail),
        _ => State::Running,
    }
}

fn bash_state(cmd: &str) -> State {
    const TESTS: &[&str] = &[
        "cargo test", "cargo nextest", "pytest", "npm test", "npm run test", "pnpm test", "pnpm run test",
        "yarn test", "bun test", "vitest", "jest", "go test", "make test", "deno test",
    ];
    // Cada tramo de `a && b; c | d`, sin variables de entorno delante.
    let parts = cmd.split(['&', ';', '|']).map(|p| {
        p.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-')).collect::<Vec<_>>().join(" ")
    });
    let mut state = State::Running;
    for p in parts {
        if TESTS.iter().any(|t| p == *t || p.starts_with(&format!("{t} ")) || p.contains(&format!(" {t}"))) {
            return State::Testing;
        }
        if p == "git" || p.starts_with("git ") {
            state = State::Git;
        }
    }
    state
}
