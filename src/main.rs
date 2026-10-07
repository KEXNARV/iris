mod adjunto;
mod ask;
mod baymax;
mod buddy;
mod claude;
mod clip;
mod commands;
mod entrada;
mod habla;
mod estilo;
mod md;
mod musica;
mod surco;
mod miniatura;
mod nucleo;
mod select;
mod sessions;
mod sixel;
mod teclado;
mod theme;
mod ui;
mod update;
mod voice;

use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

use ask::{Ask, Outcome};
use nucleo::{Event as Gesto, State};
use claude::{Claude, ClaudeEvent};
use voice::{Level, VoiceCmd, VoiceEvent};

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

/// Abre Ctrl+G en una pestaña con lo último elegido; sin nada que mostrar, avisa y no abre.
fn abrir_vista(app: &mut App, agentes: bool) -> Option<(usize, usize)> {
    let n = if agentes { app.agents_done.len() + app.agents.len() } else { app.activity.len() };
    if n == 0 {
        let msg = if agentes { "todavía no lancé ningún subagente" } else { "todavía no usé ninguna herramienta" };
        app.flash = Some((msg.into(), Instant::now()));
        return None;
    }
    // Los agentes se leen desde el final, como una terminal; las herramientas, desde arriba.
    Some((n - 1, if agentes { usize::MAX } else { 0 }))
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
    interrupted: bool,
    assistant_open: bool,
    level: Level,
    /// La terminal avisa cuándo se suelta una tecla (protocolo de teclado de kitty).
    key_release: bool,
    /// Espacio apretado y todavía sin soltar: lo que llegue mientras tanto es auto-repetición.
    space_down: bool,
    /// Sin aviso de soltar, la auto-repetición se reconoce por lo seguido que llega.
    last_space: Instant,
    /// Un soltar recién llegado. Solo cuenta si no lo sigue otro apretar enseguida: hay
    /// terminales que auto-repiten como pares soltar+apretar.
    released: Option<Instant>,
    /// Cuándo empezó a escuchar el espacio en curso; al soltarlo tras un rato, envía.
    pub ptt: Option<Instant>,
    /// Lo último que se dibujó del chat; con eso el mouse sabe qué texto hay debajo.
    pub view: std::cell::RefCell<select::View>,
    pub sel: Option<select::Selection>,
    /// Aviso breve en la barra de abajo («copiado…»).
    pub flash: Option<(String, Instant)>,
    /// El primer Ctrl+C: si llega otro mientras se ve el aviso, se sale.
    quit_armed: Option<Instant>,
    /// Versión nueva por ofrecer: se muestra cuando no estorbe.
    update: Option<update::Info>,
    /// Se instaló la versión nueva: al salir, Jarvis se vuelve a abrir con ella.
    reexec: bool,
    /// El blob del panel NÚCLEO.
    pub nucleo: nucleo::Core,
    /// Tokens de contexto en uso y la ventana del modelo.
    pub ctx_used: u64,
    pub ctx_window: u64,
    /// Tareas de la última TodoWrite: (total, completadas).
    pub todos: (usize, usize),
    /// Se pidió `/compact` y todavía no terminó.
    compacting: bool,
    /// Última tecla, clic o evento del motor; con eso se sabe si está en reposo.
    last_activity: Instant,
    last_key: Instant,
    /// Escuchando: última vez que el nivel superó el piso de ruido, y ese piso.
    last_voice: Instant,
    noise_floor: f32,
    /// Al arrancar o reiniciar, el núcleo se arma.
    booted: Instant,
    /// `/calm`: menos movimiento en el núcleo.
    pub calm: bool,
    /// Markdown ya dibujado de cada mensaje, con el largo del texto y el ancho con que se hizo.
    /// A 60 fps no conviene volver a interpretar toda la conversación en cada cuadro.
    md_cache: std::cell::RefCell<Vec<Option<(usize, usize, Vec<md::Row>)>>>,
    /// Los archivos que menciona cada respuesta, con el largo del texto del que salieron.
    adjuntos_cache: std::cell::RefCell<Vec<Option<(usize, Vec<adjunto::Adjunto>)>>>,
    /// Imágenes pegadas que se van con el próximo mensaje.
    pub images: Vec<clip::Image>,
    /// Dónde está el cursor en la orden (en caracteres), y lo que ya enviaste.
    pub cur: usize,
    historial: entrada::Historial,
    /// Modo flotante (`--flotante`): aparece con un atajo, escucha sola y se esconde al terminar.
    pub flotante: bool,
    /// La ventana tiene el foco (la terminal avisa al ganarlo y perderlo).
    pub focused: bool,
    /// Escuchando sin espacio (flotante): se envía solo al callarte. `heard`: ya dijiste algo.
    hands_free: Option<Instant>,
    heard: bool,
    /// Cuándo esconderse, si no pasa nada antes.
    hide_at: Option<Instant>,
    /// La voz de JARVIS (Kokoro) y el que arma las frases de la respuesta para decirlas.
    habla: habla::Habla,
    lector: habla::Lector,
    pub voz_modo: VozModo,
    /// El próximo mensaje llegó por voz; y si este turno se contesta hablando.
    spoken_next: bool,
    speak_turn: bool,
    /// Está sonando su voz, y con qué volumen.
    pub talking: bool,
    tts_level: f32,
    /// Lo que suena en el equipo (cava), y la última vez que sonó algo.
    musica: musica::Musica,
    surco: surco::Surco,
    music_at: std::cell::Cell<Option<Instant>>,
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
    thumbs_shown: Vec<(ratatui::layout::Rect, usize, (u16, u16, u16))>,
    sixel_shown: Option<ratatui::layout::Rect>,
    /// Dónde empieza en `activity` el turno en curso, para la línea de tiempo.
    pub turn_from: usize,
    /// Subagentes, los hijos del núcleo.
    pub agents: Vec<Agent>,
    /// Los subagentes que ya terminaron (los últimos 40), para Ctrl+G.
    pub agents_done: Vec<Agent>,
    next_kid: u64,
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
    fn music(&self) -> f32 {
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

    fn push(&mut self, role: Role, text: impl Into<String>) {
        self.assistant_open = false;
        self.messages.push(Msg { role, text: text.into(), waiting: None, images: vec![], tool: None });
        self.scroll = 0;
    }

    /// El proceso que los iba a leer ya no existe.
    fn drop_waiting(&mut self) {
        let lost = self.messages.iter_mut().filter_map(|m| m.waiting.take()).count();
        if lost > 0 {
            self.push(Role::Error, format!("{lost} mensaje(s) en espera no llegaron a Claude"));
        }
    }

    fn on_claude(&mut self, ev: ClaudeEvent) {
        self.last_activity = Instant::now();
        match ev {
            ClaudeEvent::Init { model, session, skills } => {
                self.model = model;
                self.session = session;
                commands::add_skills(&mut self.commands, &skills);
            }
            ClaudeEvent::Thinking => self.thinking = true,
            ClaudeEvent::Text(t) => {
                self.thinking = false;
                if self.speak_turn {
                    for frase in self.lector.push(&t) {
                        self.habla.say(&frase);
                    }
                }
                if self.assistant_open {
                    self.messages.last_mut().unwrap().text.push_str(&t);
                } else {
                    self.push(Role::Assistant, t.trim_start());
                    self.assistant_open = true;
                }
            }
            ClaudeEvent::ToolUse { id, name, detail, input } => {
                self.thinking = false;
                self.assistant_open = false;
                if name == "TodoWrite" {
                    let todos = input["todos"].as_array().map(Vec::as_slice).unwrap_or_default();
                    let done = todos.iter().filter(|t| t["status"] == "completed").count();
                    self.todos = (todos.len(), done);
                }
                self.activity.push(Activity {
                    id,
                    name,
                    detail,
                    status: ToolStatus::Running,
                    started: Instant::now(),
                    took: None,
                    input,
                    output: String::new(),
                });
                // En el chat, en su lugar de la conversación.
                let idx = self.activity.len() - 1;
                self.messages.push(Msg { role: Role::Tool, text: String::new(), waiting: None, images: vec![], tool: Some(idx) });
            }
            ClaudeEvent::ToolResult { id, is_error, output } => {
                if let Some(a) = self.activity.iter_mut().rev().find(|a| a.id == id) {
                    a.output = output;
                    a.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                    a.took = Some(a.started.elapsed());
                    let kind = nucleo::tool_state(&a.name, &a.detail);
                    if is_error {
                        self.nucleo.fire(Gesto::Error);
                    } else if kind == State::Testing {
                        self.nucleo.fire(Gesto::Pass);
                    } else if kind == State::Delegating && !self.agents.iter().any(|g| g.tool == id) {
                        // Sin hijo en el núcleo (un motor que no avisa de sus tareas): la gota de siempre.
                        self.nucleo.fire(Gesto::Merge);
                    }
                }
                self.thinking = true;
            }
            ClaudeEvent::Done { cost, secs, is_error, window } => {
                if self.flotante {
                    self.hide_at = Some(Instant::now() + Duration::from_secs(4));
                } else if !self.focused && !self.interrupted {
                    // En otra ventana: que se entere de que terminó.
                    let first = self
                        .messages
                        .iter()
                        .rev()
                        .find(|m| m.role == Role::Assistant)
                        .and_then(|m| m.text.lines().find(|l| !l.trim().is_empty()))
                        .unwrap_or("")
                        .replace(['*', '`', '#'], "");
                    let title = if is_error { "JARVIS: el turno terminó con error" } else { "JARVIS terminó" };
                    notify(title, &first);
                }
                if std::mem::take(&mut self.speak_turn) {
                    for frase in self.lector.finish() {
                        self.habla.say(&frase);
                    }
                }
                if let Some(w) = window {
                    self.ctx_window = w;
                }
                self.compacting = false;
                if is_error && !self.interrupted {
                    self.nucleo.fire(Gesto::Error);
                } else if !is_error {
                    self.nucleo.fire(Gesto::Done);
                }
                self.busy = false;
                self.thinking = false;
                self.assistant_open = false;
                self.cost = cost.max(self.cost);
                self.turns += 1;
                for a in self.activity.iter_mut().filter(|a| a.status == ToolStatus::Running) {
                    a.status = ToolStatus::Err;
                }
                if is_error && !std::mem::take(&mut self.interrupted) {
                    self.push(Role::Error, format!("el turno terminó con error ({secs:.1}s)"));
                }
            }
            ClaudeEvent::Ask { request_id, tool, input } => {
                if !self.focused && !self.flotante {
                    notify("JARVIS te pregunta algo", "Necesita tu respuesta para seguir.");
                }
                self.modal = Some(Modal::Ask(Ask::new(request_id, tool, input)));
            }
            ClaudeEvent::Taken(id) => {
                for m in self.messages.iter_mut().filter(|m| m.waiting.as_deref() == Some(&id)) {
                    m.waiting = None;
                }
            }
            ClaudeEvent::Usage(n) => {
                self.ctx_used = n;
                // La ventana real llega recién al terminar el turno; mientras tanto se supone de
                // 200k, y una sesión retomada ya puede pasarla (salía «235%»): entonces es de 1M.
                if n > self.ctx_window {
                    self.ctx_window = 1_000_000;
                }
            }
            ClaudeEvent::Compacted { pre, post } => {
                self.compacting = false;
                self.ctx_used = post;
                let k = |n: u64| format!("{:.1}k", n as f64 / 1000.0);
                self.push(Role::System, format!("contexto compactado: {} → {} tokens", k(pre), k(post)));
            }
            ClaudeEvent::AgentStarted { tool, description, kind } => {
                let kid = self.next_kid;
                self.next_kid += 1;
                self.nucleo.kid_born(kid);
                self.agents.push(Agent {
                    tool,
                    kid,
                    description,
                    kind,
                    started: Instant::now(),
                    current: None,
                    last: None,
                    tools: 0,
                    ended: None,
                    log: vec![],
                });
            }
            ClaudeEvent::AgentTool { tool, id, name, detail, input } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.log.push(Paso::Herramienta(Activity {
                        id,
                        name: name.clone(),
                        detail: detail.clone(),
                        status: ToolStatus::Running,
                        started: Instant::now(),
                        took: None,
                        input,
                        output: String::new(),
                    }));
                    a.current = Some((name, detail));
                    a.tools += 1;
                    self.nucleo.kid_pulse(a.kid, false);
                }
            }
            ClaudeEvent::AgentToolResult { tool, id, is_error, output } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    let paso = a.log.iter_mut().rev().find_map(|p| match p {
                        Paso::Herramienta(t) if t.id == id => Some(t),
                        _ => None,
                    });
                    if let Some(t) = paso {
                        t.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                        t.took = Some(t.started.elapsed());
                        t.output = output;
                    }
                    if let Some((name, detail)) = a.current.take() {
                        a.last = Some((name, detail, Instant::now()));
                    }
                    self.nucleo.kid_pulse(a.kid, is_error);
                }
            }
            ClaudeEvent::AgentText { tool, text } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.log.push(Paso::Texto(text));
                }
            }
            ClaudeEvent::AgentDone { tool, ok } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.ended = Some((Instant::now(), ok));
                    a.current = None;
                    self.nucleo.kid_end(a.kid, ok);
                }
            }
            ClaudeEvent::Stderr(line) => self.push(Role::Error, line),
            ClaudeEvent::Exited => {
                self.agents.clear();
                self.nucleo.kids_clear();
                self.modal = None;
                self.drop_waiting();
                self.alive = false;
                self.busy = false;
                self.push(Role::Error, "el proceso de claude terminó. Ctrl+R para reiniciar.");
            }
        }
    }
}

fn main() -> Result<()> {
    voice::prefer_discrete_gpu();
    let mut extra: Vec<String> = std::env::args().skip(1).collect();
    if extra.first().map(String::as_str) == Some("--transcribe") {
        return voice::transcribe_file(extra.get(1).map_or("", String::as_str));
    }
    // --flotante: la ventana que se llama con un atajo, te escucha sola, ejecuta y se esconde.
    let flotante = extra.iter().any(|a| a == "--flotante");
    extra.retain(|a| a != "--flotante");
    if flotante {
        extra.extend(["--append-system-prompt".into(), FLOTANTE_PROMPT.into()]);
    }
    let (tx, rx) = mpsc::channel();

    let mut claude = Some(Claude::spawn(&extra, tx.clone())?);
    let (voice_tx, level) = voice::spawn(tx.clone());
    update::spawn(tx.clone());

    // `jarvis --resume <id>`: además de pasárselo al motor, se muestra la conversación.
    let resumed = extra.iter().position(|a| a == "--resume").and_then(|i| extra.get(i + 1)).map(|id| sessions::replay(id));
    let (messages, activity, agents_done) = resumed.map(|r| (r.messages, r.activity, r.agents)).unwrap_or_default();
    let turn_from = activity.len();
    let mut app = App {
        messages,
        activity,
        input: String::new(),
        scroll: 0,
        busy: false,
        thinking: false,
        voice: VoiceState::Loading,
        voice_model: String::new(),
        levels: vec![0.0; 256],
        model: String::new(),
        session: String::new(),
        cost: 0.0,
        turns: 0,
        started: Instant::now(),
        alive: true,
        commands: commands::load(),
        menu: 0,
        modal: None,
        interrupted: false,
        assistant_open: false,
        level,
        key_release: false,
        space_down: false,
        last_space: Instant::now(),
        released: None,
        ptt: None,
        view: Default::default(),
        sel: None,
        flash: None,
        nucleo: nucleo::Core::new(),
        ctx_used: 0,
        ctx_window: 200_000,
        todos: (0, 0),
        compacting: false,
        last_activity: Instant::now(),
        quit_armed: None,
        update: None,
        reexec: false,
        last_key: Instant::now(),
        last_voice: Instant::now(),
        noise_floor: 0.0,
        booted: Instant::now(),
        calm: std::env::var_os("JARVIS_CALM").is_some(),
        md_cache: Default::default(),
        adjuntos_cache: Default::default(),
        images: vec![],
        cur: 0,
        historial: entrada::Historial::load(),
        flotante,
        focused: true,
        hands_free: None,
        heard: false,
        hide_at: None,
        habla: habla::Habla::new(tx.clone()),
        lector: habla::Lector::new(),
        voz_modo: match std::env::var("JARVIS_HABLA").as_deref() {
            _ if flotante => VozModo::Siempre,
            Ok("always") | Ok("siempre") => VozModo::Siempre,
            Ok("never") | Ok("nunca") | Ok("0") => VozModo::Nunca,
            _ => VozModo::Auto,
        },
        spoken_next: false,
        speak_turn: false,
        talking: false,
        tts_level: 0.0,
        musica: musica::Musica::spawn(),
        surco: surco::Surco::spawn(),
        music_at: std::cell::Cell::new(None),
        estilo: estilo::cargar(),
        buddy: buddy::cargar(),
        reading: Default::default(),
        transcript: false,
        tools_view: None,
        agents_tab: false,
        tools_view_max: Default::default(),
        read_override: None,
        read_top: Default::default(),
        read_msg: std::cell::Cell::new(usize::MAX),
        sixel_cell: sixel_cell(),
        core_rect: Default::default(),
        thumb_slots: Default::default(),
        file_slots: Default::default(),
        file_targets: Default::default(),
        thumb_targets: Default::default(),
        thumbs_shown: vec![],
        sixel_shown: None,
        turn_from,
        agents: vec![],
        agents_done,
        next_kid: 0,
    };

    let mut term = ratatui::init();
    // Para mantener espacio y hablar hace falta saber cuándo se suelta. Con «desambiguar» y
    // «tipos de evento» las letras siguen llegando como texto (acentos y tildes muertas
    // intactos) y además llega el soltar. «Todas las teclas como códigos» duplica la é en foot.
    use crossterm::event::{KeyboardEnhancementFlags as K, PushKeyboardEnhancementFlags};
    if crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false) {
        let flags = K::DISAMBIGUATE_ESCAPE_CODES | K::REPORT_EVENT_TYPES;
        app.key_release = crossterm::execute!(std::io::stdout(), PushKeyboardEnhancementFlags(flags)).is_ok();
    }
    // Con el mouse en manos de Jarvis, arrastrar copia solo texto del chat; Shift+arrastrar
    // sigue siendo la selección de la terminal.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    // Lo pegado llega en un solo evento: así un texto de varias líneas no se envía en el
    // primer salto, y una ruta de imagen arrastrada a la terminal se puede adjuntar.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste);
    // Saber si la ventana tiene el foco: para avisar con una notificación y, en el modo
    // flotante, para empezar a escuchar al aparecer.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableFocusChange);
    let res = run(&mut term, &mut app, &mut claude, &voice_tx, &tx, &rx, &extra);
    if app.key_release {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);
    }
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableBracketedPaste);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableFocusChange);
    ratatui::restore();
    if res.is_ok() && app.reexec {
        // Se instaló la versión nueva: se reabre con ella y retoma esta misma sesión.
        drop(claude);
        let mut args: Vec<String> = vec![];
        let mut old = std::env::args().skip(1);
        while let Some(a) = old.next() {
            if a == "--resume" {
                old.next();
            } else {
                args.push(a);
            }
        }
        if !app.session.is_empty() {
            args.extend(["--resume".into(), app.session.clone()]);
        }
        drop(app);
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(update::exe()).args(args).exec();
        return Err(err.into());
    }
    res
}

fn run(
    term: &mut ratatui::DefaultTerminal,
    app: &mut App,
    claude: &mut Option<Claude>,
    voice_tx: &Sender<VoiceCmd>,
    tx: &Sender<AppEvent>,
    rx: &mpsc::Receiver<AppEvent>,
    extra: &[String],
) -> Result<()> {
    // 60 fps: lo que más se mueve es el núcleo.
    let tick = Duration::from_millis(16);
    let mut frame = Instant::now();
    let mut frames: u64 = 0;
    let mut teclado = teclado::Teclado::new();
    let mut theme_check = Instant::now();
    theme::poll();
    // Los colores con que se dibujó el markdown guardado. /buddy, /core y sus vistas
    // previas los cambian sin pasar por `poll`, así que se compara en cada vuelta.
    let mut colores = (theme::acento_u32(), theme::texto_u32(), theme::fondo_rgb());
    loop {
        // El tema puede cambiar en cualquier momento (Omarchy, Aether): se mira una vez por segundo.
        if theme_check.elapsed() >= Duration::from_secs(1) {
            theme_check = Instant::now();
            theme::poll();
        }
        let ahora = (theme::acento_u32(), theme::texto_u32(), theme::fondo_rgb());
        if ahora != colores {
            colores = ahora;
            app.md_cache.borrow_mut().clear(); // el markdown guardado lleva los colores viejos
        }
        // La onda avanza aunque no haya audio, para que se vea viva.
        let lvl = if app.voice == VoiceState::Listening { voice::level_get(&app.level) } else { 0.0 };
        app.levels.remove(0);
        app.levels.push(lvl);
        if app.voice == VoiceState::Listening {
            hear(app, lvl);
        }
        flotante_tick(app, voice_tx);
        // La música de surco calla mientras escucha, transcribe, piensa lo que va a decir o habla.
        let quiet = matches!(app.voice, VoiceState::Listening | VoiceState::Transcribing)
            || app.talking
            || (app.busy && app.speak_turn);
        app.surco.quiet(quiet);
        // La versión nueva se ofrece cuando no estorba: si apareciera mientras escribes, el
        // Enter de tu mensaje la aceptaría.
        if app.update.is_some()
            && app.modal.is_none()
            && app.tools_view.is_none()
            && app.input.is_empty()
            && !app.busy
            && app.last_key.elapsed() > Duration::from_secs(3)
        {
            app.modal = app.update.take().map(Modal::Update);
        }

        let dt = frame.elapsed().as_secs_f64();
        frame = Instant::now();
        // Los que terminaron se quedan unos segundos en ACTIVIDAD y luego se van.
        // Los que terminaron hace rato dejan el núcleo, pero quedan para verlos en Ctrl+G.
        let (viejos, vivos): (Vec<Agent>, Vec<Agent>) =
            std::mem::take(&mut app.agents).into_iter().partition(|a| a.ended.is_some_and(|(t, _)| t.elapsed() >= Duration::from_secs(4)));
        app.agents = vivos;
        app.agents_done.extend(viejos);
        let sobra = app.agents_done.len().saturating_sub(40);
        app.agents_done.drain(..sobra);
        let (want, sig) = (app.state(), app.signals());
        app.nucleo.step(dt, want, &sig);
        // El teclado acompaña al núcleo: su estado, la voz y los eventos.
        teclado.tick(app.nucleo.state(), sig.level);
        for ev in app.nucleo.take_fired() {
            teclado.event(ev);
        }

        app.core_rect.set(None);
        app.thumb_slots.borrow_mut().clear();
        app.thumb_targets.borrow_mut().clear();
        app.file_slots.borrow_mut().clear();
        app.file_targets.borrow_mut().clear();
        // Lo que se dibujó en este cuadro: hace falta para reescribir el texto que tapaba una
        // miniatura que se movió.
        let snap = term.draw(|f| ui::draw(f, app))?.buffer.clone();
        if sixel_frame(term, app, &mut frames)? {
            app.thumbs_shown.clear(); // la pantalla se limpió: hay que volver a dibujarlas
        }
        thumbs_frame(term, app, &snap)?;
        follow_drag(app);

        if event::poll(tick)? {
            let ev = event::read()?;
            if let Event::Mouse(m) = ev {
                on_mouse(app, m);
            }
            match ev {
                Event::FocusGained => {
                    app.focused = true;
                    app.hide_at = None;
                    if app.flotante && app.voice == VoiceState::Ready && !app.busy && !app.talking {
                        let _ = voice_tx.send(VoiceCmd::Start);
                        app.hands_free = Some(Instant::now());
                        app.heard = false;
                    }
                }
                Event::FocusLost => app.focused = false,
                _ => {}
            }
            if let Event::Paste(text) = &ev {
                app.last_activity = Instant::now();
                pasted(app, text);
            }
            if let Event::Key(k) = ev {
                let space = k.code == KeyCode::Char(' ');
                if k.kind == KeyEventKind::Release && space {
                    app.space_down = false;
                    app.released = Some(Instant::now());
                }
                // Las repeticiones sirven para borrar o moverse con la tecla apretada.
                if k.kind == KeyEventKind::Press || (k.kind == KeyEventKind::Repeat && !space) {
                    match handle_key(app, k, claude, voice_tx) {
                        Flow::Quit => return Ok(()),
                        Flow::Restart => {
                            *claude = None;
                            app.drop_waiting();
                            *claude = Some(Claude::spawn(extra, tx.clone())?);
                            app.alive = true;
                            app.modal = None;
                            app.ctx_used = 0;
                            app.todos = (0, 0);
                            app.booted = Instant::now();
                            app.nucleo.reboot();
                            app.agents.clear();
                            app.push(Role::System, "claude reiniciado — sesión nueva");
                        }
                        Flow::NewChat => {
                            *claude = None;
                            app.drop_waiting();
                            *claude = Some(Claude::spawn(extra, tx.clone())?);
                            app.alive = true;
                            app.busy = false;
                            app.modal = None;
                            app.messages.clear();
                            app.md_cache.borrow_mut().clear();
                            app.adjuntos_cache.borrow_mut().clear();
                            app.activity.clear();
                            app.ctx_used = 0;
                            app.todos = (0, 0);
                            app.booted = Instant::now();
                            app.nucleo.reboot();
                            app.agents.clear();
                            app.push(Role::System, "conversación nueva · la anterior se retoma con /resume");
                            term.clear()?;
                        }
                        Flow::Resume(id) => {
                            let mut args = extra.to_vec();
                            args.extend(["--resume".into(), id.clone()]);
                            *claude = None;
                            *claude = Some(Claude::spawn(&args, tx.clone())?);
                            app.alive = true;
                            app.busy = false;
                            app.modal = None;
                            app.activity.clear();
                            app.todos = (0, 0);
                            app.booted = Instant::now();
                            app.nucleo.reboot();
                            app.agents.clear();
                            // Chat, herramientas y subagentes de la sesión: Ctrl+G los vuelve a ver.
                            let replay = sessions::replay(&id);
                            app.messages = replay.messages;
                            app.activity = replay.activity;
                            app.agents_done = replay.agents;
                            app.turn_from = app.activity.len();
                            app.md_cache.borrow_mut().clear();
                            app.adjuntos_cache.borrow_mut().clear();
                            app.push(Role::System, format!("sesión {} retomada", &id[..8]));
                            term.clear()?;
                        }
                        Flow::Redraw => term.clear()?,
                        Flow::Update => {
                            app.push(Role::System, "actualizando: bajando y compilando la versión nueva…");
                            update::install(tx.clone());
                        }
                        Flow::Go => {}
                    }
                }
            }
        }

        if app.released.is_some_and(|t| t.elapsed() >= REPEAT_GAP) {
            app.released = None;
            space_up(app, voice_tx);
        }

        while let Ok(ev) = rx.try_recv() {
            match ev {
                AppEvent::Claude(id, e) if claude.as_ref().is_some_and(|c| c.id == id) => {
                    app.on_claude(e)
                }
                AppEvent::Claude(..) => {}
                AppEvent::Voice(e) => on_voice(app, e, claude),
                AppEvent::Habla(e) => match e {
                    habla::HablaEvent::Start => app.talking = true,
                    habla::HablaEvent::Level(l) => app.tts_level = l,
                    habla::HablaEvent::Idle => {
                        app.talking = false;
                        app.tts_level = 0.0;
                        if app.flotante && !app.busy {
                            app.hide_at = Some(Instant::now() + Duration::from_secs(3));
                        }
                    }
                    habla::HablaEvent::Unavailable(why) => {
                        app.push(Role::Error, format!("sin voz: {why}"));
                    }
                },
                AppEvent::Update(e) => match e {
                    update::UpdateEvent::Available(info) => app.update = Some(info),
                    update::UpdateEvent::Installed => {
                        app.reexec = true;
                        return Ok(());
                    }
                    update::UpdateEvent::Failed(why) => {
                        app.push(Role::Error, format!("no se pudo actualizar: {why}"));
                    }
                },
            }
        }
    }
}

enum Flow {
    Go,
    /// `/clear`: conversación nueva, como en Claude Code. La anterior queda guardada.
    NewChat,
    Redraw,
    Quit,
    Restart,
    Resume(String),
    /// Bajar e instalar la versión nueva.
    Update,
}

fn handle_key(
    app: &mut App,
    k: KeyEvent,
    claude: &mut Option<Claude>,
    voice_tx: &Sender<VoiceCmd>,
) -> Flow {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    app.sel = None;
    app.last_activity = Instant::now();

    // Ctrl+G: qué hizo cada herramienta, entera, y en la otra pestaña qué hizo cada subagente.
    // Mientras está abierto se lleva las flechas.
    if ctrl && k.code == KeyCode::Char('g') {
        app.tools_view = match app.tools_view {
            Some(_) => None,
            None => abrir_vista(app, app.agents_tab),
        };
        return Flow::Go;
    }
    if app.tools_view.is_some() && k.code == KeyCode::Tab {
        app.agents_tab = !app.agents_tab;
        app.tools_view = abrir_vista(app, app.agents_tab).or(app.tools_view);
        return Flow::Go;
    }
    if let Some((sel, scroll)) = app.tools_view.as_mut() {
        let n = if app.agents_tab { app.agents_done.len() + app.agents.len() } else { app.activity.len() }.max(1);
        let arriba = if app.agents_tab { usize::MAX } else { 0 };
        match k.code {
            KeyCode::Up => (*sel, *scroll) = (sel.saturating_sub(1), arriba),
            KeyCode::Down => (*sel, *scroll) = ((*sel + 1).min(n - 1), arriba),
            KeyCode::PageUp => *scroll = (*scroll).min(app.tools_view_max.get()).saturating_sub(10),
            KeyCode::PageDown => *scroll += 10,
            KeyCode::Home => *scroll = 0,
            KeyCode::End => *scroll = usize::MAX,
            KeyCode::Esc => app.tools_view = None,
            _ => {}
        }
        return Flow::Go;
    }

    if !ctrl {
        if let Some(flow) = modal_key(app, k, claude) {
            return flow;
        }
    }

    let shown = commands::matches(&app.commands, &app.input).len();
    if shown > 0 && !ctrl {
        let pick = |app: &App| {
            let m = commands::matches(&app.commands, &app.input);
            format!("/{}", m[app.menu.min(m.len() - 1)].name)
        };
        match k.code {
            KeyCode::Up => {
                app.menu = (app.menu + shown - 1) % shown;
                return Flow::Go;
            }
            KeyCode::Down => {
                app.menu = (app.menu + 1) % shown;
                return Flow::Go;
            }
            KeyCode::Tab => {
                app.input = pick(app) + " ";
                app.cur = app.input.chars().count();
                return Flow::Go;
            }
            KeyCode::Enter => app.input = pick(app),
            KeyCode::Esc => {
                app.input.clear();
                app.cur = 0;
                return Flow::Go;
            }
            _ => {}
        }
    }

    match k.code {
        // Ctrl+C pide confirmación: hay que darlo otra vez antes de que se borre el aviso.
        KeyCode::Char('c') if ctrl => {
            if app.quit_armed.is_some_and(|t| t.elapsed() < Duration::from_millis(2500)) {
                return Flow::Quit;
            }
            app.quit_armed = Some(Instant::now());
            app.flash = Some(("Ctrl+C otra vez para salir".into(), Instant::now()));
        }
        KeyCode::Char('d') if ctrl => return Flow::Quit,
        KeyCode::Char('r') if ctrl => return Flow::Restart,
        KeyCode::Char('l') if ctrl => return clear(app),
        KeyCode::Char('u') if ctrl => {
            app.input.clear();
            app.cur = 0;
        }
        // Edición de la orden, como en un shell.
        KeyCode::Char('a') if ctrl => app.cur = 0,
        KeyCode::Char('e') if ctrl => app.cur = app.input.chars().count(),
        KeyCode::Char('w') if ctrl => entrada::kill_word(&mut app.input, &mut app.cur),
        KeyCode::Char('k') if ctrl => entrada::kill_to_end(&mut app.input, &mut app.cur),
        KeyCode::Left if ctrl => entrada::word_left(&app.input, &mut app.cur),
        KeyCode::Right if ctrl => entrada::word_right(&app.input, &mut app.cur),
        KeyCode::Left => entrada::left(&app.input, &mut app.cur),
        KeyCode::Right => entrada::right(&app.input, &mut app.cur),
        KeyCode::Delete => entrada::delete(&mut app.input, &mut app.cur),
        KeyCode::Home if !app.input.is_empty() => app.cur = 0,
        KeyCode::End if !app.input.is_empty() => app.cur = app.input.chars().count(),
        // Shift+Enter (o Alt+Enter): otra línea sin enviar.
        KeyCode::Enter if k.modifiers.intersects(KeyModifiers::SHIFT | KeyModifiers::ALT) => {
            entrada::insert(&mut app.input, &mut app.cur, "\n");
            app.last_key = Instant::now();
        }
        KeyCode::Char('v') if ctrl => paste(app),
        KeyCode::Char('o') if ctrl => app.read_override = Some(!app.reading.get()),
        KeyCode::Char('t') if ctrl => {
            app.transcript = !app.transcript;
            app.scroll = 0;
        }
        // Con la conversación desplegada, Esc solo la cierra.
        KeyCode::Esc if app.transcript => {
            app.transcript = false;
            app.scroll = 0;
        }
        // Fuera de Cine, ^O es una vista aparte: Esc la cierra, como a ^T.
        KeyCode::Esc if app.reading.get() && app.read_override == Some(true) && app.estilo != estilo::Estilo::Cine => {
            app.read_override = None;
        }

        // Espacio con la entrada vacía: empezar/terminar de escuchar.
        KeyCode::Char(' ') if app.input.is_empty() && space_repeat(app) => {}
        KeyCode::Char(' ') if app.input.is_empty() => match app.voice {
            VoiceState::Ready => {
                app.habla.stop(); // si estaba hablando, se calla para escucharte
                let _ = voice_tx.send(VoiceCmd::Start);
                app.ptt = Some(Instant::now());
            }
            VoiceState::Listening => {
                let _ = voice_tx.send(VoiceCmd::Stop);
                app.ptt = None;
            }
            VoiceState::Loading => app.push(Role::System, "el modelo de voz todavía está cargando…"),
            _ => {}
        },
        // En el flotante, sin nada en curso, Esc lo esconde.
        KeyCode::Esc if app.flotante && !app.busy && !app.talking && app.voice == VoiceState::Ready => hide_flotante(),
        KeyCode::Esc => {
            // Esc siempre lo calla; si además está trabajando, lo interrumpe.
            app.habla.stop();
            app.speak_turn = false;
            if app.voice == VoiceState::Listening {
                let _ = voice_tx.send(VoiceCmd::Cancel);
            } else if app.busy {
                if let Some(c) = claude {
                    let _ = c.interrupt();
                }
                app.nucleo.fire(Gesto::Cancel);
                app.interrupted = true;
                app.push(Role::System, "interrumpido");
            }
        }
        KeyCode::Enter => {
            let text = app.input.trim().to_string();
            app.input.clear();
            app.cur = 0;
            app.historial.push(&text);
            let (cmd, arg) = text.split_once(' ').unwrap_or((&text, ""));
            match cmd {
                "/resume" => return resume(app, arg.trim()),
                "/clear" if app.busy => {
                    app.push(Role::Error, "espera a que termine el turno (o Esc) antes de empezar otra conversación");
                    return Flow::Go;
                }
                "/clear" => return Flow::NewChat,
                "/copy" => {
                    copy_last(app);
                    return Flow::Go;
                }
                "/model" => {
                    model(app, claude, arg.trim());
                    return Flow::Go;
                }
                "/restart" => return Flow::Restart,
                "/voice" => {
                    app.voz_modo = match (arg.trim(), app.voz_modo) {
                        ("always", _) | ("on", _) => VozModo::Siempre,
                        ("never", _) | ("off", _) => VozModo::Nunca,
                        ("auto", _) => VozModo::Auto,
                        (_, VozModo::Auto) => VozModo::Siempre,
                        (_, VozModo::Siempre) => VozModo::Nunca,
                        (_, VozModo::Nunca) => VozModo::Auto,
                    };
                    if app.voz_modo == VozModo::Nunca {
                        app.habla.stop();
                    }
                    let msg = match app.voz_modo {
                        VozModo::Auto => "voz: contesta hablando cuando le hablas",
                        VozModo::Siempre => "voz: contesta hablando siempre",
                        VozModo::Nunca => "voz: no habla",
                    };
                    app.flash = Some((msg.into(), Instant::now()));
                    return Flow::Go;
                }
                "/calm" => {
                    app.calm = !app.calm;
                    let msg = if app.calm { "núcleo en calma" } else { "núcleo con todo su movimiento" };
                    app.flash = Some((msg.into(), Instant::now()));
                    return Flow::Go;
                }
                "/theme" | "/themes" => {
                    theme(app, arg.trim());
                    return Flow::Go;
                }
                "/buddy" | "/budy" => {
                    elegir_buddy(app, arg.trim());
                    return Flow::Go;
                }
                "/core" => {
                    color(app, arg.trim());
                    return Flow::Go;
                }
                "/agents" => {
                    app.agents_tab = true;
                    app.tools_view = abrir_vista(app, true);
                    return Flow::Go;
                }
                "/quit" => return Flow::Quit,
                _ => submit(app, claude, text),
            }
        }
        // Con la entrada vacía, retroceso quita la última imagen adjunta.
        KeyCode::Backspace if app.input.is_empty() && !app.images.is_empty() => {
            app.images.pop();
        }
        KeyCode::Backspace => {
            entrada::backspace(&mut app.input, &mut app.cur);
            app.menu = 0;
        }
        KeyCode::Char(c) => {
            // Con el protocolo de kitty Shift+n puede llegar como «n» más el modificador.
            let typed: String = if k.modifiers.contains(KeyModifiers::SHIFT) && c.is_lowercase() {
                c.to_uppercase().collect()
            } else {
                c.to_string()
            };
            entrada::insert(&mut app.input, &mut app.cur, &typed);
            app.menu = 0;
            app.last_key = Instant::now();
            app.nucleo.fire(Gesto::Key);
        }
        KeyCode::PageUp => scroll(app, true, 10),
        KeyCode::PageDown => scroll(app, false, 10),
        // ↑ ↓ recorren lo que enviaste; leyendo en Cine, mueven el texto abierto.
        KeyCode::Up if app.reading.get() && !app.historial.browsing() => scroll(app, true, 1),
        KeyCode::Down if app.reading.get() && !app.historial.browsing() => scroll(app, false, 1),
        KeyCode::Up => {
            if let Some(t) = app.historial.prev(&app.input) {
                app.cur = t.chars().count();
                app.input = t;
            }
        }
        KeyCode::Down => {
            if let Some(t) = app.historial.next() {
                app.cur = t.chars().count();
                app.input = t;
            }
        }
        KeyCode::Home if app.reading.get() => app.read_top.set(0),
        KeyCode::End if app.reading.get() => app.read_top.set(usize::MAX),
        KeyCode::End => app.scroll = 0,
        _ => {}
    }
    Flow::Go
}

/// Espacio con la entrada vacía, dos modos: mantenerlo (escucha hasta soltarlo) o tocarlo
/// (un toque empieza, otro envía). Lo que llega mientras sigue apretado es auto-repetición.
const REPEAT_GAP: Duration = Duration::from_millis(80);

fn space_repeat(app: &mut App) -> bool {
    if app.key_release {
        let paired = app.released.take().is_some_and(|t| t.elapsed() < REPEAT_GAP);
        return std::mem::replace(&mut app.space_down, true) || paired;
    }
    // Sin aviso de soltar: la repetición llega cada ~25 ms y una persona no toca tan rápido.
    let repeat = app.last_space.elapsed() < Duration::from_millis(150);
    app.last_space = Instant::now();
    repeat
}

/// Soltar tras mantenerlo un rato es «ya terminé de hablar»; soltar un toque corto no hace
/// nada y sigue escuchando hasta el siguiente toque.
fn space_up(app: &mut App, voice_tx: &Sender<VoiceCmd>) {
    let Some(t) = app.ptt.take() else { return };
    if t.elapsed() < Duration::from_millis(400) {
        app.ptt = None;
    } else if app.voice == VoiceState::Listening {
        let _ = voice_tx.send(VoiceCmd::Stop);
    }
}

fn on_mouse(app: &mut App, m: crossterm::event::MouseEvent) {
    let at = (m.column, m.row);
    let area = app.view.borrow().area;
    let inside = area.contains(ratatui::layout::Position::new(m.column, m.row));
    let dragging = app.sel.as_ref().is_some_and(|s| s.pointer.is_some());
    match m.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let hit = app.file_targets.borrow().iter().find(|(r, _)| r.contains(ratatui::layout::Position::new(m.column, m.row))).map(|(_, a)| a.clone());
            if let Some(a) = hit {
                app.sel = None;
                let msg = if adjunto::copiar(&a) {
                    app.nucleo.fire(Gesto::Copy);
                    "archivo copiado · pégalo donde quieras".to_string()
                } else {
                    "no pude copiar el archivo al portapapeles".to_string()
                };
                app.flash = Some((msg, Instant::now()));
                return;
            }
            app.sel = inside.then(|| {
                let p = app.view.borrow().locate(at);
                select::Selection { anchor: p, head: p, pointer: Some(at) }
            });
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(s) = &mut app.sel {
                s.pointer = Some(at);
                s.head = app.view.borrow().locate(at);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            let Some(s) = &mut app.sel else { return };
            s.pointer = None;
            if s.is_empty() {
                app.sel = None;
                return;
            }
            // Queda resaltado hasta el próximo clic o tecla, para ver qué se copió.
            let text = s.text(&app.view.borrow());
            copied(app, &text);
        }
        // Con la rueda se puede seguir estirando la selección más allá de lo que se ve.
        MouseEventKind::ScrollUp if app.reading.get() => scroll(app, true, 3),
        MouseEventKind::ScrollDown if app.reading.get() => scroll(app, false, 3),
        MouseEventKind::ScrollUp if inside || dragging => {
            app.scroll = (app.scroll + 3).min(app.view.borrow().max_scroll());
        }
        MouseEventKind::ScrollDown if inside || dragging => app.scroll = app.scroll.saturating_sub(3),
        _ => {}
    }
}

/// Cada cuadro, mientras se arrastra: la punta sigue al puntero sobre el texto que ahora
/// está debajo, y arrastrar por encima o por debajo del chat lo desplaza solo.
fn follow_drag(app: &mut App) {
    let Some(p) = app.sel.as_ref().and_then(|s| s.pointer) else { return };
    let area = app.view.borrow().area;
    if p.1 < area.y {
        app.scroll = (app.scroll + 1).min(app.view.borrow().max_scroll());
    } else if p.1 >= area.bottom() {
        app.scroll = app.scroll.saturating_sub(1);
    }
    let head = app.view.borrow().locate(p);
    if let Some(s) = &mut app.sel {
        s.head = head;
    }
}

/// `/copy`: la última respuesta entera, con su markdown.
fn copy_last(app: &mut App) {
    match app.messages.iter().rev().find(|m| m.role == Role::Assistant) {
        Some(m) => {
            let text = m.text.clone();
            copied(app, &text);
        }
        None => app.flash = Some(("todavía no hay respuesta que copiar".into(), Instant::now())),
    }
}

fn copied(app: &mut App, text: &str) {
    let n = text.chars().count();
    let msg = if text.is_empty() {
        "nada que copiar ahí".to_string()
    } else if select::copy(text) {
        app.nucleo.fire(Gesto::Copy);
        format!("copiado · {n} caracteres")
    } else {
        "no pude copiar al portapapeles".to_string()
    };
    app.flash = Some((msg, Instant::now()));
}

fn clear(app: &mut App) -> Flow {
    app.messages.clear();
    app.md_cache.borrow_mut().clear();
    app.adjuntos_cache.borrow_mut().clear();
    app.activity.clear();
    app.turn_from = 0;
    Flow::Redraw
}

/// Teclas que se lleva lo superpuesto; `None` deja pasar la tecla al manejo normal
/// (escribir, borrar, hablar con espacio).
fn modal_key(app: &mut App, k: KeyEvent, claude: &mut Option<Claude>) -> Option<Flow> {
    match app.modal.as_mut()? {
        Modal::Sessions { list, sel } => {
            match k.code {
                KeyCode::Up => *sel = (*sel + list.len() - 1) % list.len(),
                KeyCode::Down => *sel = (*sel + 1) % list.len(),
                KeyCode::Enter => {
                    let id = list[*sel].id.clone();
                    app.modal = None;
                    return Some(Flow::Resume(id));
                }
                KeyCode::Esc => app.modal = None,
                _ => {}
            }
            Some(Flow::Go)
        }
        Modal::Update(_) => {
            match k.code {
                KeyCode::Enter => {
                    app.modal = None;
                    return Some(Flow::Update);
                }
                KeyCode::Esc => app.modal = None,
                _ => {}
            }
            Some(Flow::Go)
        }
        Modal::Model { sel } => {
            let n = commands::MODELS.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Enter => {
                    let id = commands::MODELS[*sel].0;
                    app.modal = None;
                    model(app, claude, id);
                }
                KeyCode::Esc => app.modal = None,
                _ => {}
            }
            Some(Flow::Go)
        }
        Modal::Estilo { sel, antes } => {
            let n = estilo::TODOS.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Char(c @ '1'..='9') if ((c as u8 - b'1') as usize) < n => *sel = (c as u8 - b'1') as usize,
                KeyCode::Enter => {
                    app.modal = None;
                    set_estilo(app, app.estilo);
                    return Some(Flow::Redraw);
                }
                KeyCode::Esc => {
                    app.estilo = *antes;
                    app.modal = None;
                    return Some(Flow::Redraw);
                }
                _ => {}
            }
            // Vista previa: la pantalla cambia mientras se elige.
            app.estilo = estilo::TODOS[*sel].0;
            Some(Flow::Redraw)
        }
        Modal::Buddy { sel, antes } => {
            let n = buddy::TODOS.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Char(c @ '1'..='9') if ((c as u8 - b'1') as usize) < n => *sel = (c as u8 - b'1') as usize,
                KeyCode::Enter => {
                    app.modal = None;
                    set_buddy(app, app.buddy);
                    return Some(Flow::Redraw);
                }
                KeyCode::Esc => {
                    app.buddy = *antes;
                    app.modal = None;
                    return Some(Flow::Redraw);
                }
                _ => {}
            }
            // Vista previa: el núcleo cambia mientras se elige.
            app.buddy = buddy::TODOS[*sel].0;
            Some(Flow::Redraw)
        }
        Modal::Color { sel, antes } => {
            let n = baymax::OPCIONES.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Char(c @ '1'..='9') if ((c as u8 - b'1') as usize) < n => *sel = (c as u8 - b'1') as usize,
                KeyCode::Enter => {
                    let m = baymax::OPCIONES[*sel].2;
                    app.modal = None;
                    set_color(app, m);
                    return Some(Flow::Redraw);
                }
                KeyCode::Esc => {
                    baymax::poner_nucleo(*antes);
                    app.modal = None;
                    return Some(Flow::Redraw);
                }
                _ => {}
            }
            baymax::poner_nucleo(baymax::OPCIONES[*sel].2);
            Some(Flow::Redraw)
        }
        Modal::Ask(a) => {
            match k.code {
                KeyCode::Up => a.move_sel(false),
                KeyCode::Down => a.move_sel(true),
                KeyCode::Char(' ') if a.current().multi && app.input.is_empty() => a.toggle(),
                KeyCode::Enter => {
                    let typed = std::mem::take(&mut app.input);
                    app.cur = 0;
                    answer(app, claude, &typed);
                }
                KeyCode::Esc => {
                    let Some(Modal::Ask(a)) = app.modal.take() else { unreachable!() };
                    if let Some(c) = claude {
                        let _ = c.deny(&a.request_id, "el usuario cerró la pregunta sin responder");
                    }
                    app.push(Role::System, "pregunta descartada");
                }
                _ => return None,
            }
            Some(Flow::Go)
        }
    }
}

/// Sin argumento abre la lista; con uno (id, alias como `sonnet` o nombre de la lista) lo pide.
fn model(app: &mut App, claude: &mut Option<Claude>, arg: &str) {
    if arg.is_empty() {
        let sel = commands::MODELS.iter().position(|m| m.0 == app.model).unwrap_or(0);
        app.modal = Some(Modal::Model { sel });
        return;
    }
    let q = arg.to_lowercase();
    let id = commands::MODELS
        .iter()
        .find(|m| m.0 == q || m.1.to_lowercase().starts_with(&q))
        .map_or(arg, |m| m.0);
    let Some(c) = claude.as_mut().filter(|_| app.alive) else {
        app.push(Role::Error, "claude no está corriendo (Ctrl+R)");
        return;
    };
    match c.set_model(id) {
        Ok(()) => {
            app.model = id.to_string();
            let when = if app.busy { "desde el próximo turno" } else { "listo" };
            app.push(Role::System, format!("modelo → {id} ({when})"));
        }
        Err(e) => app.push(Role::Error, format!("no pude cambiar de modelo: {e}")),
    }
}

/// `/theme`: sin argumento abre la lista con vista previa; con uno (`cabina`, `cine`…) lo aplica.
fn theme(app: &mut App, arg: &str) {
    if arg.is_empty() {
        let sel = estilo::TODOS.iter().position(|e| e.0 == app.estilo).unwrap_or(0);
        app.modal = Some(Modal::Estilo { sel, antes: app.estilo });
        return;
    }
    match estilo::Estilo::buscar(arg) {
        Some(e) => set_estilo(app, e),
        None => {
            let ids: Vec<&str> = estilo::TODOS.iter().map(|e| e.1).collect();
            app.push(Role::Error, format!("no conozco el estilo «{arg}» · hay {}", ids.join(", ")));
        }
    }
}

/// `/buddy`: sin argumento abre la lista con vista previa; con uno (`baymax`, `original`) lo aplica.
fn elegir_buddy(app: &mut App, arg: &str) {
    if arg.is_empty() {
        let sel = buddy::TODOS.iter().position(|b| b.0 == app.buddy).unwrap_or(0);
        app.modal = Some(Modal::Buddy { sel, antes: app.buddy });
        return;
    }
    match buddy::Buddy::buscar(arg) {
        Some(b) => set_buddy(app, b),
        None => {
            let ids: Vec<&str> = buddy::TODOS.iter().map(|b| b.1).collect();
            app.push(Role::Error, format!("no conozco el buddy «{arg}» · hay {}", ids.join(", ")));
        }
    }
}

fn set_buddy(app: &mut App, b: buddy::Buddy) {
    app.buddy = b;
    let msg = match buddy::guardar(b) {
        Ok(()) => format!("buddy {}", b.nombre()),
        Err(err) => format!("buddy {} (no se guardó: {err})", b.nombre()),
    };
    app.flash = Some((msg, Instant::now()));
}

/// `/core`: sin argumento, la lista con vista previa; con uno (`auto`, `rosa`, `#ffd84a`) lo
/// aplica.
fn color(app: &mut App, arg: &str) {
    if app.buddy.paleta().is_none() {
        app.push(Role::Error, String::from("los colores del núcleo son de Baymax · /buddy baymax"));
        return;
    }
    if arg.is_empty() {
        let antes = baymax::nucleo();
        let sel = baymax::OPCIONES.iter().position(|o| o.2 == antes).unwrap_or(0);
        app.modal = Some(Modal::Color { sel, antes });
        return;
    }
    match baymax::Modo::buscar(arg) {
        Some(m) => set_color(app, m),
        None => {
            let ids: Vec<&str> = baymax::OPCIONES.iter().map(|o| o.0).collect();
            app.push(Role::Error, format!("no conozco el color «{arg}» · hay {} o un #rrggbb", ids.join(", ")));
        }
    }
}

fn set_color(app: &mut App, m: baymax::Modo) {
    baymax::poner_nucleo(m);
    let msg = match theme::guardar("nucleo", &m.texto()) {
        Ok(()) => format!("núcleo {}", m.nombre()),
        Err(err) => format!("núcleo {} (no se guardó: {err})", m.nombre()),
    };
    app.flash = Some((msg, Instant::now()));
}

fn set_estilo(app: &mut App, e: estilo::Estilo) {
    app.estilo = e;
    let msg = match estilo::guardar(e) {
        Ok(()) => format!("estilo {}", e.nombre()),
        Err(err) => format!("estilo {} (no se guardó: {err})", e.nombre()),
    };
    app.flash = Some((msg, Instant::now()));
}

/// Responde la pregunta en curso (escrita, dictada o elegida) y, si era la última, la envía.
fn answer(app: &mut App, claude: &mut Option<Claude>, typed: &str) {
    let Some(Modal::Ask(a)) = app.modal.as_mut() else { return };
    let q = a.current().text.clone();
    let outcome = a.answer(typed);
    let id = a.request_id.clone();
    let res = match outcome {
        Outcome::Next => return,
        Outcome::Allow(input) => {
            if let Some(ans) = input["answers"].as_object() {
                let mut text = String::new();
                for (q, v) in ans {
                    text.push_str(&format!("{q} → {}\n", v.as_str().unwrap_or("")));
                }
                app.push(Role::System, text.trim_end().to_string());
            } else {
                app.push(Role::System, format!("permitido: {q}"));
            }
            claude.as_mut().map(|c| c.allow(&id, input))
        }
        Outcome::Deny(why) => {
            app.push(Role::System, format!("rechazado: {q}"));
            claude.as_mut().map(|c| c.deny(&id, &why))
        }
    };
    app.modal = None;
    if let Some(Err(e)) = res {
        app.push(Role::Error, format!("no pude responder: {e}"));
    }
}

/// `claude -p` no entiende `/resume`: sin argumento abre la lista de sesiones, con `N` o un prefijo
/// del id relanza el motor sobre esa sesión.
fn resume(app: &mut App, arg: &str) -> Flow {
    if app.busy {
        app.push(Role::Error, "espera a que termine el turno (o Esc) antes de cambiar de sesión");
        return Flow::Go;
    }
    if arg.is_empty() {
        let list = sessions::list(&app.session, 30);
        if list.is_empty() {
            app.push(Role::System, "no hay sesiones anteriores en este directorio");
        } else {
            app.modal = Some(Modal::Sessions { list, sel: 0 });
        }
        return Flow::Go;
    }
    match sessions::find(&app.session, arg) {
        Some(id) => Flow::Resume(id),
        None => {
            app.push(Role::Error, format!("no encontré la sesión «{arg}»; `/resume` para ver la lista"));
            Flow::Go
        }
    }
}

fn submit(app: &mut App, claude: &mut Option<Claude>, text: String) {
    if text.is_empty() && app.images.is_empty() {
        return;
    }
    let Some(c) = claude.as_mut().filter(|_| app.alive) else {
        app.push(Role::Error, "claude no está corriendo (Ctrl+R)");
        return;
    };
    let queued = app.busy;
    app.read_override = None;
    // Contesta hablando si le hablaste (o si la voz está en «siempre»).
    app.habla.stop();
    app.speak_turn = match app.voz_modo {
        VozModo::Siempre => true,
        VozModo::Nunca => false,
        VozModo::Auto => std::mem::take(&mut app.spoken_next),
    };
    app.spoken_next = false;
    app.lector = habla::Lector::new();
    if text.trim() == "/compact" || text.starts_with("/compact ") {
        app.compacting = true;
    }
    let images = std::mem::take(&mut app.images);
    let shown = match images.len() {
        0 => text.clone(),
        n => {
            let list: Vec<String> = images.iter().enumerate().map(|(i, im)| im.label(i + 1)).collect();
            let head = format!("▣ {n} imagen{} ({})", if n == 1 { "" } else { "es" }, list.join(", "));
            if text.is_empty() { head } else { format!("{head}\n{text}") }
        }
    };
    app.push(Role::User, shown);
    app.messages.last_mut().unwrap().images = images.iter().filter_map(|i| miniatura::from_bytes(&i.raw)).collect();
    match c.send(&text, &images) {
        Ok(uuid) => {
            // Con el motor libre lo toma al instante; solo a mitad de turno queda esperando.
            if queued {
                app.messages.last_mut().unwrap().waiting = Some(uuid);
            } else {
                app.turn_from = app.activity.len();
            }
            app.busy = true;
            app.thinking = true;
            app.interrupted = false;
        }
        Err(e) => app.push(Role::Error, format!("no pude enviar: {e}")),
    }
}

fn on_voice(app: &mut App, ev: VoiceEvent, claude: &mut Option<Claude>) {
    match ev {
        VoiceEvent::Ready(name) => {
            app.voice = VoiceState::Ready;
            app.voice_model = name;
        }
        VoiceEvent::Unavailable(e) => {
            app.voice = VoiceState::Off;
            app.push(Role::Error, format!("voz desactivada: {e}"));
        }
        VoiceEvent::Listening => {
            app.voice = VoiceState::Listening;
            app.last_voice = Instant::now();
            app.noise_floor = 0.0;
        }
        VoiceEvent::Discarded(t) => {
            app.voice = VoiceState::Ready;
            app.nucleo.fire(Gesto::Nope);
            app.push(Role::System, format!("descarté «{t}»: whisper lo inventa sobre el ruido"));
        }
        VoiceEvent::Transcribing => app.voice = VoiceState::Transcribing,
        VoiceEvent::Transcript(t) if t.is_empty() => {
            app.voice = VoiceState::Ready;
            app.push(Role::System, "no escuché nada");
        }
        VoiceEvent::Transcript(t) if matches!(app.modal, Some(Modal::Ask(_))) => {
            app.voice = VoiceState::Ready;
            answer(app, claude, &t);
        }
        VoiceEvent::Transcript(t) => {
            app.voice = VoiceState::Ready;
            app.spoken_next = true;
            submit(app, claude, t);
        }
        VoiceEvent::Cancelled => app.voice = VoiceState::Ready,
        VoiceEvent::Error(e) => {
            app.voice = VoiceState::Ready;
            app.push(Role::Error, e);
        }
    }
}

/// Sigue el piso de ruido mientras escucha y anota cuándo hubo voz por encima. Un umbral fijo
/// no sirve: en una pieza con ruido de fondo el silencio ya marca tanto como la voz baja.
fn hear(app: &mut App, lvl: f32) {
    let floor = &mut app.noise_floor;
    // Baja de golpe con el silencio y sube muy despacio con la voz.
    *floor = if *floor == 0.0 || lvl < *floor { lvl } else { *floor + (lvl - *floor) * 0.002 };
    if lvl > (*floor * 1.8).max(*floor + 0.01) {
        app.last_voice = Instant::now();
        app.heard = true;
    }
}

const FLOTANTE_PROMPT: &str = "Modo flotante: Kevin te llamó con un atajo desde el escritorio y te habla por voz; \
tu respuesta se lee en voz alta. Si pide una acción del sistema (volumen, brillo, tema, abrir o cerrar apps, \
música, capturas, recordatorios), hazla directo con los comandos de Omarchy, Hyprland o wpctl, sin pedir \
confirmación salvo que sea destructiva, y contesta en UNA frase corta en español, sin markdown. Para preguntas, \
contesta breve.";

/// Modo flotante: corta la escucha cuando te callas (o si no dijiste nada) y se esconde un
/// rato después de terminar de contestar.
fn flotante_tick(app: &mut App, voice_tx: &Sender<VoiceCmd>) {
    if !app.flotante {
        return;
    }
    if let Some(start) = app.hands_free {
        if app.voice != VoiceState::Listening {
            if start.elapsed() > Duration::from_secs(3) {
                app.hands_free = None; // ya terminó por otro lado
            }
        } else if app.heard && app.last_voice.elapsed() > Duration::from_millis(1300) {
            let _ = voice_tx.send(VoiceCmd::Stop);
            app.hands_free = None;
        } else if !app.heard && start.elapsed() > Duration::from_secs(7) {
            let _ = voice_tx.send(VoiceCmd::Cancel);
            app.hands_free = None;
            app.hide_at = Some(Instant::now());
        } else if start.elapsed() > Duration::from_secs(20) {
            let _ = voice_tx.send(VoiceCmd::Stop);
            app.hands_free = None;
        }
    }
    let quiet = !app.busy && !app.talking && app.voice != VoiceState::Listening && app.voice != VoiceState::Transcribing;
    if let Some(at) = app.hide_at {
        if quiet && app.focused && Instant::now() >= at {
            app.hide_at = None;
            hide_flotante();
        } else if !quiet {
            app.hide_at = None;
        }
    }
}

fn hide_flotante() {
    let _ = std::process::Command::new("hyprctl")
        // Con la config en Lua, «dispatch» recibe una llamada de hl.dsp (la sintaxis vieja falla).
        .args(["dispatch", "hl.dsp.workspace.toggle_special(\"jarvis\")"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Ctrl+V: una imagen del portapapeles se adjunta; texto, se pega en la entrada.
fn paste(app: &mut App) {
    match clip::paste() {
        Ok(clip::Clip::Image(img)) => attach(app, img),
        Ok(clip::Clip::Text(t)) => pasted(app, &t),
        Ok(clip::Clip::Nothing) => app.flash = Some(("el portapapeles está vacío".into(), Instant::now())),
        Err(e) => app.push(Role::Error, format!("no pude pegar: {e}")),
    }
}

/// Texto pegado (o archivos arrastrados a la terminal, que llegan como rutas).
fn pasted(app: &mut App, text: &str) {
    if let Some(paths) = clip::paths(text) {
        for p in paths {
            match clip::read(&p) {
                Ok(img) => attach(app, img),
                Err(e) => app.push(Role::Error, format!("no pude adjuntar {p}: {e}")),
            }
        }
        return;
    }
    entrada::insert(&mut app.input, &mut app.cur, &text.replace("\r\n", "\n").replace('\r', "\n"));
    app.menu = 0;
    app.last_key = Instant::now();
}

fn attach(app: &mut App, img: clip::Image) {
    let label = img.label(app.images.len() + 1);
    app.images.push(img);
    app.flash = Some((format!("{label} adjunta · se va con el próximo mensaje"), Instant::now()));
}

/// Hacia arriba o abajo: la respuesta abierta en Cine si la hay; si no, la conversación.
fn scroll(app: &mut App, up: bool, n: usize) {
    if app.reading.get() {
        let top = app.read_top.get();
        app.read_top.set(if up { top.saturating_sub(n) } else { top.saturating_add(n) });
    } else if up {
        app.scroll += n;
    } else {
        app.scroll = app.scroll.saturating_sub(n);
    }
}


/// ¿Se puede dibujar el núcleo como imagen? Hace falta una terminal con Sixel (foot) y saber
/// cuántos píxeles mide una celda. `JARVIS_SIXEL=0` lo apaga; `=1` lo fuerza en otra terminal.
fn sixel_cell() -> Option<(u16, u16)> {
    let want = std::env::var("JARVIS_SIXEL").ok();
    let foot = std::env::var("TERM").is_ok_and(|t| t.starts_with("foot"));
    if want.as_deref() == Some("0") || (!foot && want.as_deref() != Some("1")) {
        return None;
    }
    cell_px()
}

fn cell_px() -> Option<(u16, u16)> {
    let ws = crossterm::terminal::window_size().ok()?;
    (ws.width > 0 && ws.columns > 0 && ws.rows > 0).then(|| (ws.width / ws.columns, ws.height / ws.rows))
}

/// Manda la imagen del núcleo a 30 fps (uno de cada dos cuadros). Si el panel se movió o el
/// núcleo volvió a braille (menú abierto), limpia la pantalla: la imagen vieja no se borra sola.
/// Devuelve si limpió la pantalla.
fn sixel_frame(term: &mut ratatui::DefaultTerminal, app: &mut App, frames: &mut u64) -> Result<bool> {
    use std::io::Write;
    let target = app.core_rect.get();
    let mut cleared = false;
    if app.sixel_shown.is_some() && app.sixel_shown != target {
        app.sixel_shown = None;
        term.clear()?;
        cleared = true;
        if target.is_none() {
            app.thumb_slots.borrow_mut().clear();
            app.thumb_targets.borrow_mut().clear();
            app.file_slots.borrow_mut().clear();
            app.file_targets.borrow_mut().clear();
            term.draw(|f| ui::draw(f, app))?;
        }
    }
    let Some(r) = target else { return Ok(cleared) };
    *frames += 1;
    if app.sixel_shown == Some(r) && *frames % 2 == 1 {
        return Ok(cleared);
    }
    // El tamaño de la celda cambia con el zoom de la fuente; se vuelve a medir.
    let Some((cw, ch)) = cell_px().or(app.sixel_cell) else { return Ok(cleared) };
    app.sixel_cell = Some((cw, ch));
    let sp = std::env::var("JARVIS_DOT").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let img = app.nucleo.sixel(r.width as usize * cw as usize, r.height as usize * ch as usize, sp);
    // Cada imagen tiene fondo transparente y foot deja ver por ahí la anterior: sin borrar el
    // panel, los cuadros viejos se acumulan. Borrar las celdas (ECH) elimina la imagen de abajo;
    // dentro de una actualización sincronizada (modo 2026) borrar y dibujar se ven juntos.
    let mut seq = String::with_capacity(img.len() + 32 * r.height as usize);
    seq.push_str("\x1b[?2026h\x1b7");
    for y in r.y..r.bottom() {
        seq.push_str(&format!("\x1b[{};{}H\x1b[{}X", y + 1, r.x + 1, r.width));
    }
    seq.push_str("\x1b[49m");
    seq.push_str(&format!("\x1b[{};{}H", r.y + 1, r.x + 1));
    seq.push_str(&img);
    seq.push_str("\x1b8\x1b[?2026l");
    let mut out = std::io::stdout().lock();
    out.write_all(seq.as_bytes())?;
    out.flush()?;
    app.sixel_shown = Some(r);
    Ok(cleared)
}

/// Las miniaturas del chat: se dibujan solo cuando cambian de lugar (desplazar, texto nuevo).
/// Donde estaban, se reescribe el texto de ese cuadro: escribir encima borra la imagen vieja
/// (foot no la borra solo) y deja el texto correcto. Borrar las celdas en blanco se comía el
/// texto que había ahí, y ratatui no lo volvía a escribir porque lo creía dibujado.
fn thumbs_frame(term: &mut ratatui::DefaultTerminal, app: &mut App, snap: &ratatui::buffer::Buffer) -> Result<()> {
    use std::io::Write;
    let targets = std::mem::take(&mut *app.thumb_targets.borrow_mut());
    let now: Vec<(ratatui::layout::Rect, usize, (u16, u16, u16))> =
        targets.iter().map(|(r, t, c)| (*r, std::rc::Rc::as_ptr(t) as usize, *c)).collect();
    if now == app.thumbs_shown {
        return Ok(());
    }
    let Some(cell) = app.sixel_cell else { return Ok(()) };
    {
        use ratatui::backend::Backend;
        let area = snap.area;
        let mut cells = vec![];
        for (r, _, _) in &app.thumbs_shown {
            for y in r.y..r.bottom().min(area.bottom()) {
                for x in r.x..r.right().min(area.right()) {
                    cells.push((x, y, &snap[(x, y)]));
                }
            }
        }
        let b = term.backend_mut();
        b.draw(cells.into_iter())?;
        Backend::flush(b)?;
    }
    let mut seq = String::from("\x1b[?2026h\x1b7");
    for (r, t, (from, to, total)) in &targets {
        seq.push_str(&format!("\x1b[{};{}H", r.y + 1, r.x + 1));
        seq.push_str(&t.sixel(cell, r.width, *total, *from, *to));
    }
    seq.push_str("\x1b8\x1b[?2026l");
    let mut out = std::io::stdout().lock();
    out.write_all(seq.as_bytes())?;
    out.flush()?;
    app.thumbs_shown = now;
    Ok(())
}

/// Una notificación del escritorio (notify-send); si no está, no pasa nada.
fn notify(title: &str, body: &str) {
    let body: String = body.chars().take(140).collect();
    let _ = std::process::Command::new("notify-send")
        .args(["-a", "JARVIS", title, &body])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
