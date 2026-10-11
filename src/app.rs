//! Iris's state: the conversation, the activity, the subagents, the voice and what is shown.

use crate::*;

pub enum AppEvent {
    /// Generation of the process that emitted it (see `Claude::id`).
    Claude(u64, ClaudeEvent),
    Voice(VoiceEvent),
    Speech(speech::SpeechEvent),
    Update(update::UpdateEvent),
    Store(crate::storefront::StoreEvent),
}

/// When it answers out loud: `Auto` when you spoke to it (if you typed, silently).
#[derive(Clone, Copy, PartialEq)]
pub enum VoiceMode {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    User,
    Assistant,
    System,
    Error,
    /// A tool it used, in its place in the conversation (`Msg::tool` says which).
    Tool,
}

pub struct Msg {
    pub role: Role,
    pub text: String,
    /// Typed mid-turn and still unread: the id the engine will echo back when it takes it.
    pub waiting: Option<String>,
    /// The images you sent with this message, to see them in the chat.
    pub images: Vec<std::rc::Rc<thumbnail::Thumb>>,
    /// For `Role::Tool`: which activity it is (index into `App::activity`).
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
    /// The full input (the command, the file and the change…) and what it returned.
    pub input: serde_json::Value,
    pub output: String,
}

/// A live (or just finished) subagent: a child of the core.
pub struct Agent {
    /// Id of the `Agent` call that launched it; its messages arrive with this id.
    pub tool: String,
    /// The number the core knows it by.
    pub kid: u64,
    pub description: String,
    pub kind: String,
    pub started: Instant,
    /// The tool it has in progress (name and detail); with none, it is thinking.
    pub current: Option<(String, String)>,
    /// The last one that finished and when. A Write lasts 50 ms and the model takes seconds to
    /// ask for the next one: without this the child always looks «thinking».
    pub last: Option<(String, String, Instant)>,
    pub tools: usize,
    /// When it ended and whether it went well; it lingers a moment so it can be seen returning.
    pub ended: Option<(Instant, bool)>,
    /// Everything it did, in order: tools and what it wrote. For Ctrl+G, Agents.
    pub log: Vec<Step>,
}

/// A step of a subagent.
pub enum Step {
    Tool(Activity),
    Text(String),
}

impl Agent {
    pub fn state(&self) -> State {
        match (&self.current, &self.last) {
            (Some((name, detail)), _) => core::tool_state(name, detail),
            (None, Some((name, detail, t))) if t.elapsed() < Duration::from_secs(3) => core::tool_state(name, detail),
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

/// What overlays the conversation and takes the arrows and Enter.
pub enum Modal {
    Sessions { list: Vec<sessions::Session>, sel: usize },
    Ask(Ask),
    Model { sel: usize },
    Effort { sel: usize },
    /// `/theme`: moving already changes the screen; Esc goes back to `before`.
    UiStyle { sel: usize, before: style::UiStyle },
    /// `/core`: the color of Baymax's face.
    Color { sel: usize, before: baymax::Mode },
    /// `/buddy`: like `/theme`, with who lives in the core.
    Buddy { sel: usize, before: buddy::Buddy, list: Vec<buddy::Entry> },
    /// `/store`: the catalog.
    Store(crate::storefront::StoreModal),
    /// There is a new version on GitHub: update?
    Update(update::Info),
    /// The first time (or `/welcome`): step-by-step settings and the tour.
    Welcome(crate::welcome::Welcome),
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
    /// What was requested with `/effort`; goes back to `auto` when the engine relaunches.
    pub effort: String,
    pub session: String,
    pub cost: f64,
    pub turns: u32,
    pub started: Instant,
    pub alive: bool,
    pub commands: Vec<commands::Command>,
    /// Selected row in the command menu.
    pub menu: usize,
    pub modal: Option<Modal>,
    pub(crate) interrupted: bool,
    pub(crate) assistant_open: bool,
    pub(crate) level: Level,
    /// The terminal reports when a key is released (kitty keyboard protocol).
    pub(crate) key_release: bool,
    /// Space pressed and not yet released: whatever arrives meanwhile is auto-repeat.
    pub(crate) space_down: bool,
    /// Without release events, auto-repeat is recognized by how closely it arrives.
    pub(crate) last_space: Instant,
    /// A release that just arrived. It only counts if no other press follows right away: some
    /// terminals auto-repeat as release+press pairs.
    pub(crate) released: Option<Instant>,
    /// When the current space started listening; releasing it after a while sends.
    pub ptt: Option<Instant>,
    /// The last thing drawn of the chat; with it the mouse knows what text is underneath.
    pub view: std::cell::RefCell<select::View>,
    pub sel: Option<select::Selection>,
    /// Brief notice in the bottom bar («copiado…»).
    pub flash: Option<(String, Instant)>,
    /// The first Ctrl+C: if another one arrives while the notice is shown, it quits.
    pub(crate) quit_armed: Option<Instant>,
    /// New version to offer: shown when it doesn't get in the way.
    pub(crate) update: Option<update::Info>,
    /// The new version was installed: on exit, Iris reopens with it.
    pub(crate) reexec: bool,
    /// The blob of the CORE panel.
    pub core: core::Core,
    /// Context tokens in use and the model's window.
    pub ctx_used: u64,
    pub ctx_window: u64,
    /// Tasks from the last TodoWrite: (total, completed).
    pub todos: (usize, usize),
    /// `/compact` was requested and hasn't finished yet.
    pub(crate) compacting: bool,
    /// Last key, click or engine event; with it we know whether it is idle.
    pub(crate) last_activity: Instant,
    pub(crate) last_key: Instant,
    /// Listening: last time the level exceeded the noise floor, and that floor.
    pub(crate) last_voice: Instant,
    pub(crate) noise_floor: f32,
    /// On start or restart, the core assembles itself.
    pub(crate) booted: Instant,
    /// `/calm`: less motion in the core.
    pub calm: bool,
    /// Already-rendered markdown of each message, with the text length and width it was made for.
    /// At 60 fps it doesn't pay to re-parse the whole conversation on every frame.
    pub(crate) md_cache: std::cell::RefCell<Vec<Option<(usize, usize, Vec<md::Row>)>>>,
    /// The files each reply mentions, with the length of the text they came from.
    pub(crate) attachments_cache: std::cell::RefCell<Vec<Option<(usize, Vec<attachment::Attachment>)>>>,
    /// Pasted images that go out with the next message.
    pub images: Vec<clip::Image>,
    /// Where the cursor is in the prompt (in characters), and what you already sent.
    pub cur: usize,
    pub(crate) history: input::History,
    /// Floating mode (`--flotante`): appears with a shortcut, listens on its own and hides when done.
    pub floating: bool,
    /// The window has focus (the terminal reports gaining and losing it).
    pub focused: bool,
    /// Listening without space (floating): sends by itself when you go quiet. `heard`: you spoke.
    pub(crate) hands_free: Option<Instant>,
    pub(crate) heard: bool,
    /// When to hide, if nothing happens before.
    pub(crate) hide_at: Option<Instant>,
    /// Iris's voice (Kokoro) and the one that builds the reply's sentences to speak them.
    pub(crate) speech: speech::Speech,
    pub(crate) reader: speech::Reader,
    pub voice_mode: VoiceMode,
    /// The next message arrived by voice; and whether this turn is answered by speaking.
    pub(crate) spoken_next: bool,
    pub(crate) speak_turn: bool,
    /// Its voice is playing, and at what volume.
    pub talking: bool,
    pub(crate) tts_level: f32,
    /// What is playing on the machine (cava), and the last time something played.
    pub(crate) music: music::Music,
    pub(crate) surco: surco::Surco,
    pub(crate) music_at: std::cell::Cell<Option<Instant>>,
    /// How the screen is laid out (`/theme`).
    pub style: style::UiStyle,
    /// Who lives in the core (`/buddy`).
    pub buddy: buddy::Buddy,
    /// The recipe the core is showing (if the buddy is one) and when its files last changed:
    /// editing them, by hand or from Iris Studio, reloads it.
    pub shown_recipe: Option<(String, std::time::SystemTime)>,
    /// For what runs on threads (the store) to report back.
    pub events: std::sync::mpsc::Sender<AppEvent>,
    pub recipe_checked: Instant,
    /// Cine: the open reply in full at the center (decided by the drawing, which knows if it fits).
    pub reading: std::cell::Cell<bool>,
    /// Cine: the whole conversation unrolled like a curtain (^T).
    pub transcript: bool,
    /// Ctrl+G: the tool viewer open (which one is selected and how far its detail is scrolled).
    pub tools_view: Option<(usize, usize)>,
    /// In Ctrl+G, the subagents tab instead of the tools one (Tab toggles).
    pub agents_tab: bool,
    /// How far Ctrl+G's detail can scroll (computed by the drawing); PgUp starts from there.
    pub tools_view_max: std::cell::Cell<usize>,
    /// ^O forces reading or going back to the core; a new message returns the decision to the drawing.
    pub read_override: Option<bool>,
    /// First visible row of the open reply, and which message it was (on change, back to the top).
    pub read_top: std::cell::Cell<usize>,
    pub read_msg: std::cell::Cell<usize>,
    /// Core as a dot image: the pixel size of a cell, if the terminal shows Sixel (foot);
    /// `None` is braille. Where this frame goes, and where the last image ended up.
    pub sixel_cell: Option<(u16, u16)>,
    pub core_rect: std::cell::Cell<Option<ratatui::layout::Rect>>,
    /// Chat thumbnails: where each one goes in this frame (filled by the chat drawing) and
    /// which ones were drawn (position and identity), to redraw only if they moved.
    pub thumb_slots: std::cell::RefCell<Vec<(usize, u16, u16, u16, std::rc::Rc<thumbnail::Thumb>)>>,
    /// Chat attachments like the thumbnails: their row, and where they landed on screen for clicks.
    pub file_slots: std::cell::RefCell<Vec<(usize, u16, u16, attachment::Attachment)>>,
    pub file_targets: std::cell::RefCell<Vec<(ratatui::layout::Rect, attachment::Attachment)>>,
    /// (where, which, (from, to, total height) in cell rows: the visible crop).
    pub thumb_targets: std::cell::RefCell<Vec<(ratatui::layout::Rect, std::rc::Rc<thumbnail::Thumb>, (u16, u16, u16))>>,
    pub(crate) thumbs_shown: Vec<(ratatui::layout::Rect, usize, (u16, u16, u16))>,
    pub(crate) sixel_shown: Option<ratatui::layout::Rect>,
    /// Where the current turn starts in `activity`, for the timeline.
    pub turn_from: usize,
    /// Subagents, the children of the core.
    pub agents: Vec<Agent>,
    /// Subagents that already finished (the last 40), for Ctrl+G.
    pub agents_done: Vec<Agent>,
    pub(crate) next_kid: u64,
    /// Claude Code has no logged-in session: the welcome asks for /login.
    pub no_session: bool,
}

impl App {
    /// What it is doing right now, for the core and the status bar.
    /// By priority: the voice, then what blocks (engine down, a question), then the work.
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
            core::tool_state(&a.name, &a.detail)
        } else if self.busy && self.thinking {
            State::Thinking
        } else if self.busy {
            State::Speaking
        } else if !self.input.is_empty() && self.last_key.elapsed() < Duration::from_secs(3) {
            State::Typing
        } else if self.last_activity.elapsed() > Duration::from_secs(120)
            && self.music_at.get().is_none_or(|t| t.elapsed() > Duration::from_secs(10))
        {
            // With music it doesn't fall asleep: it stays dancing.
            State::Sleeping
        } else {
            State::Idle
        }
    }

    pub fn signals(&self) -> core::Signals {
        core::Signals {
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

    /// What is playing, like with the keyboard: it only counts 3 s after the last key.
    pub(crate) fn music(&self) -> f32 {
        let l = self.music.level();
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

    pub fn attachments(&self, i: usize, text: &str) -> Vec<attachment::Attachment> {
        let mut cache = self.attachments_cache.borrow_mut();
        if cache.len() <= i {
            cache.resize_with(i + 1, || None);
        }
        match &cache[i] {
            Some((len, list)) if *len == text.len() => list.clone(),
            _ => {
                let list = attachment::in_text(text);
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

    /// The process that was going to read them no longer exists.
    pub(crate) fn drop_waiting(&mut self) {
        let lost = self.messages.iter_mut().filter_map(|m| m.waiting.take()).count();
        if lost > 0 {
            self.push(Role::Error, format!("{lost} mensaje(s) en espera no llegaron a Claude"));
        }
    }

}
