//! El estado de Iris: la conversación, la actividad, los subagentes, la voz y lo que se ve.

use crate::*;

pub enum AppEvent {
    /// Generación del proceso que lo emitió (ver `Claude::id`).
    Claude(u64, ClaudeEvent),
    Voice(VoiceEvent),
    Habla(habla::HablaEvent),
    Update(update::UpdateEvent),
}

/// Cuándo contesta en voz alta: `Auto` cuando le hablaste (si escribiste, en silencio).
#[derive(Clone, Copy, PartialEq)]
pub enum VozModo {
    Auto,
    Siempre,
    Nunca,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
    Error,
    /// Una herramienta que usó, en su lugar de la conversación (`Msg::tool` dice cuál).
    Tool,
}

pub struct Msg {
    pub role: Role,
    pub text: String,
    /// Escrito a mitad de un turno y todavía sin leer: el id que el motor repetirá al tomarlo.
    pub waiting: Option<String>,
    /// Las imágenes que mandaste con este mensaje, para verlas en el chat.
    pub images: Vec<std::rc::Rc<miniatura::Thumb>>,
    /// Para `Role::Tool`: qué actividad es (índice en `App::activity`).
    pub tool: Option<usize>,
}

#[derive(PartialEq)]
pub enum ToolStatus {
    Running,
    Ok,
    Err,
}

pub struct Activity {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub status: ToolStatus,
    pub started: Instant,
    pub took: Option<Duration>,
    /// La entrada completa (el comando, el archivo y el cambio…) y lo que devolvió.
    pub input: serde_json::Value,
    pub output: String,
}

/// Un subagente vivo (o recién terminado): un hijo del núcleo.
pub struct Agent {
    /// Id de la llamada a `Agent` que lo lanzó; sus mensajes llegan con este id.
    pub tool: String,
    /// Con este número lo conoce el núcleo.
    pub kid: u64,
    pub description: String,
    pub kind: String,
    pub started: Instant,
    /// La herramienta que tiene en curso (nombre y detalle); sin ninguna, está pensando.
    pub current: Option<(String, String)>,
    /// La última que terminó y cuándo. Un Write dura 50 ms y el modelo tarda segundos en
    /// pedir la siguiente: sin esto el hijo se ve siempre «pensando».
    pub last: Option<(String, String, Instant)>,
    pub tools: usize,
    /// Cuándo terminó y si salió bien; se queda un momento para que se vea volver.
    pub ended: Option<(Instant, bool)>,
    /// Todo lo que hizo, en orden: herramientas y lo que escribió. Para Ctrl+G, Agentes.
    pub log: Vec<Paso>,
}

/// Un paso de un subagente.
pub enum Paso {
    Herramienta(Activity),
    Texto(String),
}

impl Agent {
    pub fn state(&self) -> State {
        match (&self.current, &self.last) {
            (Some((name, detail)), _) => nucleo::tool_state(name, detail),
            (None, Some((name, detail, t))) if t.elapsed() < Duration::from_secs(3) => nucleo::tool_state(name, detail),
            _ => State::Thinking,
        }
    }
}

#[derive(PartialEq)]
pub enum VoiceState {
    Loading,
    Ready,
    Listening,
    Transcribing,
    Off,
}

/// Lo que se superpone a la conversación y se lleva las flechas y el Enter.
pub enum Modal {
    Sessions { list: Vec<sessions::Session>, sel: usize },
    Ask(Ask),
    Model { sel: usize },
    /// `/theme`: moverse ya cambia la pantalla; Esc vuelve a `antes`.
    Estilo { sel: usize, antes: estilo::Estilo },
    /// `/core`: el color de la cara de Baymax.
    Color { sel: usize, antes: baymax::Modo },
    /// `/buddy`: como `/theme`, con quién vive en el núcleo.
    Buddy { sel: usize, antes: buddy::Buddy },
    /// Hay una versión nueva en GitHub: ¿actualizar?
    Update(update::Info),
}

pub struct App {
    pub messages: Vec<Msg>,
    pub activity: Vec<Activity>,
    pub input: String,
    pub scroll: usize,
    pub busy: bool,
    pub thinking: bool,
    pub voice: VoiceState,
    pub voice_model: String,
    pub levels: Vec<f32>,
    pub model: String,
    pub session: String,
    pub cost: f64,
    pub turns: u32,
    pub started: Instant,
    pub alive: bool,
    pub commands: Vec<commands::Command>,
    /// Fila elegida en el menú de comandos.
    pub menu: usize,
    pub modal: Option<Modal>,
    pub(crate) interrupted: bool,
    pub(crate) assistant_open: bool,
    pub(crate) level: Level,
    /// La terminal avisa cuándo se suelta una tecla (protocolo de teclado de kitty).
    pub(crate) key_release: bool,
    /// Espacio apretado y todavía sin soltar: lo que llegue mientras tanto es auto-repetición.
    pub(crate) space_down: bool,
    /// Sin aviso de soltar, la auto-repetición se reconoce por lo seguido que llega.
    pub(crate) last_space: Instant,
    /// Un soltar recién llegado. Solo cuenta si no lo sigue otro apretar enseguida: hay
    /// terminales que auto-repiten como pares soltar+apretar.
    pub(crate) released: Option<Instant>,
    /// Cuándo empezó a escuchar el espacio en curso; al soltarlo tras un rato, envía.
    pub ptt: Option<Instant>,
    /// Lo último que se dibujó del chat; con eso el mouse sabe qué texto hay debajo.
    pub view: std::cell::RefCell<select::View>,
    pub sel: Option<select::Selection>,
    /// Aviso breve en la barra de abajo («copiado…»).
    pub flash: Option<(String, Instant)>,
    /// El primer Ctrl+C: si llega otro mientras se ve el aviso, se sale.
    pub(crate) quit_armed: Option<Instant>,
    /// Versión nueva por ofrecer: se muestra cuando no estorbe.
    pub(crate) update: Option<update::Info>,
    /// Se instaló la versión nueva: al salir, Jarvis se vuelve a abrir con ella.
    pub(crate) reexec: bool,
    /// El blob del panel NÚCLEO.
    pub nucleo: nucleo::Core,
    /// Tokens de contexto en uso y la ventana del modelo.
    pub ctx_used: u64,
    pub ctx_window: u64,
    /// Tareas de la última TodoWrite: (total, completadas).
    pub todos: (usize, usize),
    /// Se pidió `/compact` y todavía no terminó.
    pub(crate) compacting: bool,
    /// Última tecla, clic o evento del motor; con eso se sabe si está en reposo.
    pub(crate) last_activity: Instant,
    pub(crate) last_key: Instant,
    /// Escuchando: última vez que el nivel superó el piso de ruido, y ese piso.
    pub(crate) last_voice: Instant,
    pub(crate) noise_floor: f32,
    /// Al arrancar o reiniciar, el núcleo se arma.
    pub(crate) booted: Instant,
    /// `/calm`: menos movimiento en el núcleo.
    pub calm: bool,
    /// Markdown ya dibujado de cada mensaje, con el largo del texto y el ancho con que se hizo.
    /// A 60 fps no conviene volver a interpretar toda la conversación en cada cuadro.
    pub(crate) md_cache: std::cell::RefCell<Vec<Option<(usize, usize, Vec<md::Row>)>>>,
    /// Los archivos que menciona cada respuesta, con el largo del texto del que salieron.
    pub(crate) adjuntos_cache: std::cell::RefCell<Vec<Option<(usize, Vec<adjunto::Adjunto>)>>>,
    /// Imágenes pegadas que se van con el próximo mensaje.
    pub images: Vec<clip::Image>,
    /// Dónde está el cursor en la orden (en caracteres), y lo que ya enviaste.
    pub cur: usize,
    pub(crate) historial: entrada::Historial,
    /// Modo flotante (`--flotante`): aparece con un atajo, escucha sola y se esconde al terminar.
    pub flotante: bool,
    /// La ventana tiene el foco (la terminal avisa al ganarlo y perderlo).
    pub focused: bool,
    /// Escuchando sin espacio (flotante): se envía solo al callarte. `heard`: ya dijiste algo.
    pub(crate) hands_free: Option<Instant>,
    pub(crate) heard: bool,
    /// Cuándo esconderse, si no pasa nada antes.
    pub(crate) hide_at: Option<Instant>,
    /// La voz de JARVIS (Kokoro) y el que arma las frases de la respuesta para decirlas.
    pub(crate) habla: habla::Habla,
    pub(crate) lector: habla::Lector,
    pub voz_modo: VozModo,
    /// El próximo mensaje llegó por voz; y si este turno se contesta hablando.
    pub(crate) spoken_next: bool,
    pub(crate) speak_turn: bool,
    /// Está sonando su voz, y con qué volumen.
    pub talking: bool,
    pub(crate) tts_level: f32,
    /// Lo que suena en el equipo (cava), y la última vez que sonó algo.
    pub(crate) musica: musica::Musica,
    pub(crate) surco: surco::Surco,
    pub(crate) music_at: std::cell::Cell<Option<Instant>>,
    /// Cómo se compone la pantalla (`/theme`).
    pub estilo: estilo::Estilo,
    /// Quién vive en el núcleo (`/buddy`).
    pub buddy: buddy::Buddy,
    /// Cine: la respuesta abierta entera en el centro (lo decide el dibujo, que sabe si cabe).
    pub reading: std::cell::Cell<bool>,
    /// Cine: la conversación entera desplegada como una cortina (^T).
    pub transcript: bool,
    /// Ctrl+G: el visor de herramientas abierto (cuál está elegida y cuánto se bajó su detalle).
    pub tools_view: Option<(usize, usize)>,
    /// En Ctrl+G, la pestaña de subagentes en vez de la de herramientas (Tab alterna).
    pub agents_tab: bool,
    /// Hasta dónde se puede bajar el detalle de Ctrl+G (lo calcula el dibujo); PgUp parte de ahí.
    pub tools_view_max: std::cell::Cell<usize>,
    /// ^O fuerza leer o volver al núcleo; un mensaje nuevo devuelve la decisión al dibujo.
    pub read_override: Option<bool>,
    /// Primera fila visible de la respuesta abierta, y qué mensaje era (al cambiar, vuelve arriba).
    pub read_top: std::cell::Cell<usize>,
    pub read_msg: std::cell::Cell<usize>,
    /// Núcleo como imagen de puntos: el tamaño en píxeles de una celda, si la terminal muestra
    /// Sixel (foot); `None` es braille. Dónde va este cuadro, y dónde quedó la última imagen.
    pub sixel_cell: Option<(u16, u16)>,
    pub core_rect: std::cell::Cell<Option<ratatui::layout::Rect>>,
    /// Miniaturas del chat: dónde va cada una en este cuadro (lo llena el dibujo del chat) y
    /// cuáles quedaron dibujadas (posición e identidad), para redibujar solo si se movieron.
    pub thumb_slots: std::cell::RefCell<Vec<(usize, u16, u16, u16, std::rc::Rc<miniatura::Thumb>)>>,
    /// Los adjuntos del chat como las miniaturas: su fila, y dónde quedaron en pantalla para el clic.
    pub file_slots: std::cell::RefCell<Vec<(usize, u16, u16, adjunto::Adjunto)>>,
    pub file_targets: std::cell::RefCell<Vec<(ratatui::layout::Rect, adjunto::Adjunto)>>,
    /// (dónde, cuál, (desde, hasta, alto total) en filas de celda: el recorte visible).
    pub thumb_targets: std::cell::RefCell<Vec<(ratatui::layout::Rect, std::rc::Rc<miniatura::Thumb>, (u16, u16, u16))>>,
    pub(crate) thumbs_shown: Vec<(ratatui::layout::Rect, usize, (u16, u16, u16))>,
    pub(crate) sixel_shown: Option<ratatui::layout::Rect>,
    /// Dónde empieza en `activity` el turno en curso, para la línea de tiempo.
    pub turn_from: usize,
    /// Subagentes, los hijos del núcleo.
    pub agents: Vec<Agent>,
    /// Los subagentes que ya terminaron (los últimos 40), para Ctrl+G.
    pub agents_done: Vec<Agent>,
    pub(crate) next_kid: u64,
}

impl App {
    /// Qué está haciendo en este momento, para el núcleo y la barra de estado.
    /// Por prioridad: la voz, luego lo que bloquea (motor caído, una pregunta), luego el trabajo.
    pub fn state(&self) -> State {
        match self.voice {
            VoiceState::Listening if self.last_voice.elapsed() > Duration::from_secs(2) => return State::NoVoice,
            VoiceState::Listening => return State::Listening,
            VoiceState::Transcribing => return State::Transcribing,
            _ => {}
        }
        if !self.alive {
            State::Offline
        } else if matches!(self.modal, Some(Modal::Ask(_))) {
            State::Asking
        } else if self.talking {
            State::Speaking
        } else if self.booted.elapsed() < Duration::from_millis(2400) {
            State::Booting
        } else if self.compacting {
            State::Compacting
        } else if let Some(a) = self.current_tool() {
            nucleo::tool_state(&a.name, &a.detail)
        } else if self.busy && self.thinking {
            State::Thinking
        } else if self.busy {
            State::Speaking
        } else if !self.input.is_empty() && self.last_key.elapsed() < Duration::from_secs(3) {
            State::Typing
        } else if self.last_activity.elapsed() > Duration::from_secs(120)
            && self.music_at.get().is_none_or(|t| t.elapsed() > Duration::from_secs(10))
        {
            // Con música no se duerme: se queda bailando.
            State::Sleeping
        } else {
            State::Idle
        }
    }

    pub fn signals(&self) -> nucleo::Signals {
        nucleo::Signals {
            level: if self.talking { self.tts_level } else { voice::level_get(&self.level) },
            ctx: self.ctx_used as f64 / self.ctx_window.max(1) as f64,
            queue: self.messages.iter().filter(|m| m.waiting.is_some()).count(),
            todos: self.todos,
            kids: self.agents.iter().filter(|a| a.ended.is_none()).map(|a| (a.kid, a.state())).collect(),
            idle: self.last_activity.elapsed().as_secs_f64(),
            calm: self.calm,
            music: self.music(),
        }
    }

    /// Lo que suena, como en el teclado: cuenta recién 3 s después de la última tecla.
    pub(crate) fn music(&self) -> f32 {
        let l = self.musica.level();
        if l > 0.05 {
            self.music_at.set(Some(Instant::now()));
        }
        if self.last_key.elapsed() < Duration::from_secs(3) { 0.0 } else { l }
    }

    pub fn md_rows(&self, i: usize, text: &str, width: usize) -> Vec<md::Row> {
        let mut cache = self.md_cache.borrow_mut();
        if cache.len() <= i {
            cache.resize_with(i + 1, || None);
        }
        match &cache[i] {
            Some((len, w, rows)) if *len == text.len() && *w == width => rows.clone(),
            _ => {
                let rows = md::render(text, width);
                cache[i] = Some((text.len(), width, rows.clone()));
                rows
            }
        }
    }

    pub fn adjuntos(&self, i: usize, text: &str) -> Vec<adjunto::Adjunto> {
        let mut cache = self.adjuntos_cache.borrow_mut();
        if cache.len() <= i {
            cache.resize_with(i + 1, || None);
        }
        match &cache[i] {
            Some((len, list)) if *len == text.len() => list.clone(),
            _ => {
                let list = adjunto::en_texto(text);
                cache[i] = Some((text.len(), list.clone()));
                list
            }
        }
    }

    pub fn current_tool(&self) -> Option<&Activity> {
        self.activity.iter().rev().find(|a| a.status == ToolStatus::Running)
    }

    pub(crate) fn push(&mut self, role: Role, text: impl Into<String>) {
        self.assistant_open = false;
        self.messages.push(Msg { role, text: text.into(), waiting: None, images: vec![], tool: None });
        self.scroll = 0;
    }

    /// El proceso que los iba a leer ya no existe.
    pub(crate) fn drop_waiting(&mut self) {
        let lost = self.messages.iter_mut().filter_map(|m| m.waiting.take()).count();
        if lost > 0 {
            self.push(Role::Error, format!("{lost} mensaje(s) en espera no llegaron a Claude"));
        }
    }

}
