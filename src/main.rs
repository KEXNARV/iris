mod ask;
mod claude;
mod commands;
mod sessions;
mod ui;
mod voice;

use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use ask::{Ask, Outcome};
use claude::{Claude, ClaudeEvent};
use voice::{Level, VoiceCmd, VoiceEvent};

pub enum AppEvent {
    /// Generación del proceso que lo emitió (ver `Claude::id`).
    Claude(u64, ClaudeEvent),
    Voice(VoiceEvent),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
    Error,
}

pub struct Msg {
    pub role: Role,
    pub text: String,
    /// Escrito a mitad de un turno y todavía sin leer: el id que el motor repetirá al tomarlo.
    pub waiting: Option<String>,
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
}

impl App {
    /// Qué está haciendo en este momento, para el núcleo y la barra de estado.
    pub fn state(&self) -> ui::State {
        use ui::State::*;
        match self.voice {
            VoiceState::Listening => return Listening,
            VoiceState::Transcribing => return Transcribing,
            _ => {}
        }
        if !self.alive {
            Offline
        } else if matches!(self.modal, Some(Modal::Ask(_))) {
            Asking
        } else if self.activity.iter().any(|a| a.status == ToolStatus::Running) {
            Working
        } else if self.busy && self.thinking {
            Thinking
        } else if self.busy {
            Speaking
        } else {
            Idle
        }
    }

    pub fn current_tool(&self) -> Option<&Activity> {
        self.activity.iter().rev().find(|a| a.status == ToolStatus::Running)
    }

    fn push(&mut self, role: Role, text: impl Into<String>) {
        self.assistant_open = false;
        self.messages.push(Msg { role, text: text.into(), waiting: None });
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
        match ev {
            ClaudeEvent::Init { model, session, skills } => {
                self.model = model;
                self.session = session;
                commands::add_skills(&mut self.commands, &skills);
            }
            ClaudeEvent::Thinking => self.thinking = true,
            ClaudeEvent::Text(t) => {
                self.thinking = false;
                if self.assistant_open {
                    self.messages.last_mut().unwrap().text.push_str(&t);
                } else {
                    self.push(Role::Assistant, t.trim_start());
                    self.assistant_open = true;
                }
            }
            ClaudeEvent::ToolUse { id, name, detail } => {
                self.thinking = false;
                self.assistant_open = false;
                self.activity.push(Activity {
                    id,
                    name,
                    detail,
                    status: ToolStatus::Running,
                    started: Instant::now(),
                    took: None,
                });
            }
            ClaudeEvent::ToolResult { id, is_error } => {
                if let Some(a) = self.activity.iter_mut().rev().find(|a| a.id == id) {
                    a.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                    a.took = Some(a.started.elapsed());
                }
                self.thinking = true;
            }
            ClaudeEvent::Done { cost, secs, is_error } => {
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
                self.modal = Some(Modal::Ask(Ask::new(request_id, tool, input)));
            }
            ClaudeEvent::Taken(id) => {
                for m in self.messages.iter_mut().filter(|m| m.waiting.as_deref() == Some(&id)) {
                    m.waiting = None;
                }
            }
            ClaudeEvent::Compacted { pre, post } => {
                let k = |n: u64| format!("{:.1}k", n as f64 / 1000.0);
                self.push(Role::System, format!("contexto compactado: {} → {} tokens", k(pre), k(post)));
            }
            ClaudeEvent::Stderr(line) => self.push(Role::Error, line),
            ClaudeEvent::Exited => {
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
    let extra: Vec<String> = std::env::args().skip(1).collect();
    if extra.first().map(String::as_str) == Some("--transcribe") {
        return voice::transcribe_file(extra.get(1).map_or("", String::as_str));
    }
    let (tx, rx) = mpsc::channel();

    let mut claude = Some(Claude::spawn(&extra, tx.clone())?);
    let (voice_tx, level) = voice::spawn(tx.clone());

    let mut app = App {
        messages: vec![],
        activity: vec![],
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
    let res = run(&mut term, &mut app, &mut claude, &voice_tx, &tx, &rx, &extra);
    if app.key_release {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
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
    let tick = Duration::from_millis(33);
    loop {
        // La onda avanza aunque no haya audio, para que se vea viva.
        let lvl = if app.voice == VoiceState::Listening { voice::level_get(&app.level) } else { 0.0 };
        app.levels.remove(0);
        app.levels.push(lvl);

        term.draw(|f| ui::draw(f, app))?;

        if event::poll(tick)? {
            if let Event::Key(k) = event::read()? {
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
                            app.push(Role::System, "claude reiniciado — sesión nueva");
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
                            app.messages = sessions::history(&id);
                            app.push(Role::System, format!("sesión {} retomada", &id[..8]));
                            term.clear()?;
                        }
                        Flow::Redraw => term.clear()?,
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
            }
        }
    }
}

enum Flow {
    Go,
    Redraw,
    Quit,
    Restart,
    Resume(String),
}

fn handle_key(
    app: &mut App,
    k: KeyEvent,
    claude: &mut Option<Claude>,
    voice_tx: &Sender<VoiceCmd>,
) -> Flow {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);

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
                return Flow::Go;
            }
            KeyCode::Enter => app.input = pick(app),
            KeyCode::Esc => {
                app.input.clear();
                return Flow::Go;
            }
            _ => {}
        }
    }

    match k.code {
        KeyCode::Char('c') | KeyCode::Char('d') if ctrl => return Flow::Quit,
        KeyCode::Char('r') if ctrl => return Flow::Restart,
        KeyCode::Char('l') if ctrl => return clear(app),
        KeyCode::Char('u') if ctrl => app.input.clear(),

        // Espacio con la entrada vacía: empezar/terminar de escuchar.
        KeyCode::Char(' ') if app.input.is_empty() && space_repeat(app) => {}
        KeyCode::Char(' ') if app.input.is_empty() => match app.voice {
            VoiceState::Ready => {
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
        KeyCode::Esc => {
            if app.voice == VoiceState::Listening {
                let _ = voice_tx.send(VoiceCmd::Cancel);
            } else if app.busy {
                if let Some(c) = claude {
                    let _ = c.interrupt();
                }
                app.interrupted = true;
                app.push(Role::System, "interrumpido");
            }
        }
        KeyCode::Enter => {
            let text = app.input.trim().to_string();
            app.input.clear();
            let (cmd, arg) = text.split_once(' ').unwrap_or((&text, ""));
            match cmd {
                "/resume" => return resume(app, arg.trim()),
                "/clear" => return clear(app),
                "/model" => {
                    model(app, claude, arg.trim());
                    return Flow::Go;
                }
                "/restart" => return Flow::Restart,
                "/quit" => return Flow::Quit,
                _ => submit(app, claude, text),
            }
        }
        KeyCode::Backspace => {
            app.input.pop();
            app.menu = 0;
        }
        KeyCode::Char(c) => {
            // Con el protocolo de kitty Shift+n puede llegar como «n» más el modificador.
            if k.modifiers.contains(KeyModifiers::SHIFT) && c.is_lowercase() {
                app.input.extend(c.to_uppercase());
            } else {
                app.input.push(c);
            }
            app.menu = 0;
        }
        KeyCode::PageUp => app.scroll += 10,
        KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(10),
        KeyCode::Up if app.input.is_empty() => app.scroll += 1,
        KeyCode::Down if app.input.is_empty() => app.scroll = app.scroll.saturating_sub(1),
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

fn clear(app: &mut App) -> Flow {
    app.messages.clear();
    app.activity.clear();
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
        Modal::Ask(a) => {
            match k.code {
                KeyCode::Up => a.move_sel(false),
                KeyCode::Down => a.move_sel(true),
                KeyCode::Char(' ') if a.current().multi && app.input.is_empty() => a.toggle(),
                KeyCode::Enter => {
                    let typed = std::mem::take(&mut app.input);
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
    if text.is_empty() {
        return;
    }
    let Some(c) = claude.as_mut().filter(|_| app.alive) else {
        app.push(Role::Error, "claude no está corriendo (Ctrl+R)");
        return;
    };
    let queued = app.busy;
    app.push(Role::User, text.clone());
    match c.send(&text) {
        Ok(uuid) => {
            // Con el motor libre lo toma al instante; solo a mitad de turno queda esperando.
            if queued {
                app.messages.last_mut().unwrap().waiting = Some(uuid);
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
        VoiceEvent::Listening => app.voice = VoiceState::Listening,
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
            submit(app, claude, t);
        }
        VoiceEvent::Cancelled => app.voice = VoiceState::Ready,
        VoiceEvent::Error(e) => {
            app.voice = VoiceState::Ready;
            app.push(Role::Error, e);
        }
    }
}
