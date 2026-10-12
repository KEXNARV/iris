mod attachment;
mod settings;
mod welcome;
mod link;
mod ask;
mod baymax;
mod buddy;
mod claude;
mod clip;
mod commands;
mod input;
mod speech;
mod login;
mod style;
mod md;
mod music;
mod surco;
mod thumbnail;
mod core;
mod select;
mod sessions;
mod sixel;
mod keyboard;
mod theme;
mod ui;
mod paths;
mod update;
mod voice;
mod app;
mod events;
mod keys;
mod mouse;
mod actions;
mod send;
mod listen;
mod floating;
mod screen;
mod net;
mod store;
mod live;
mod kit;
mod storefront;

use std::sync::mpsc::{self, Sender};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

use ask::{Ask, Outcome};
use core::{Event as Gesture, State};
use claude::{Claude, ClaudeEvent};
use voice::{Level, VoiceCmd, VoiceEvent};

use app::*;
use keys::*;
use mouse::*;
use actions::*;
use send::*;
use listen::*;
use floating::*;
use screen::*;

fn main() -> Result<()> {
    paths::migrate();
    voice::prefer_discrete_gpu();
    let mut extra: Vec<String> = std::env::args().skip(1).collect();
    // The store's commands (iris login, iris piece …) don't open the chat.
    if let Some(code) = kit::run(&extra) {
        std::process::exit(code);
    }
    if extra.first().map(String::as_str) == Some("--transcribe") {
        return voice::transcribe_file(extra.get(1).map_or("", String::as_str));
    }
    // --flotante: the window summoned with a shortcut; it listens on its own, runs and hides.
    let floating = extra.iter().any(|a| a == "--flotante");
    extra.retain(|a| a != "--flotante");
    if floating {
        extra.extend(["--append-system-prompt".into(), FLOATING_PROMPT.into()]);
    }
    let (tx, rx) = mpsc::channel();

    let account = login::account();
    if account.is_none() {
        eprintln!("Iris va encima de Claude Code y no lo encuentro (el comando claude).");
        eprintln!("Instálalo con: curl -fsSL https://claude.ai/install.sh | bash");
        std::process::exit(1);
    }
    let mut claude = Some(Claude::spawn(&extra, tx.clone())?);
    let (voice_tx, level) = voice::spawn(tx.clone());
    update::spawn(tx.clone());
    storefront::sync_drafts(tx.clone());
    storefront::check_updates(tx.clone());

    // `iris --resume <id>`: besides passing it to the engine, the conversation is shown.
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
        effort: settings::read("esfuerzo").unwrap_or_else(|| "auto".into()),
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
        core: core::Core::new(),
        ctx_used: 0,
        ctx_window: 200_000,
        todos: (0, 0),
        compacting: false,
        last_activity: Instant::now(),
        quit_armed: None,
        update: None,
        store_updates: None,
        live: Some(live::spawn(tx.clone())),
        live_said: None,
        reexec: false,
        last_key: Instant::now(),
        last_voice: Instant::now(),
        noise_floor: 0.0,
        booted: Instant::now(),
        calm: paths::has_var("CALM"),
        md_cache: Default::default(),
        attachments_cache: Default::default(),
        images: vec![],
        cur: 0,
        history: input::History::load(),
        floating,
        focused: true,
        hands_free: None,
        heard: false,
        hide_at: None,
        speech: speech::Speech::new(tx.clone()),
        reader: speech::Reader::new(),
        voice_mode: match paths::var("HABLA").as_deref() {
            _ if floating => VoiceMode::Always,
            Some("always") | Some("siempre") => VoiceMode::Always,
            Some("never") | Some("nunca") | Some("0") => VoiceMode::Never,
            _ => welcome::saved_voice().unwrap_or(VoiceMode::Auto),
        },
        spoken_next: false,
        speak_turn: false,
        talking: false,
        tts_level: 0.0,
        music: music::Music::spawn(),
        surco: surco::Surco::spawn(),
        music_at: std::cell::Cell::new(None),
        style: style::load(),
        buddy: buddy::load(),
        shown_recipe: None,
        events: tx.clone(),
        recipe_checked: Instant::now(),
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
        no_session: matches!(account, Some(login::Account::NoSession)),
    };
    if !floating && settings::first_run() {
        app.modal = Some(Modal::Welcome(welcome::Welcome::new(&app)));
    }

    let mut term = open_terminal(&mut app);
    let res = run(&mut term, &mut app, &mut claude, &voice_tx, &tx, &rx, &extra);
    close_terminal(&app);
    if res.is_ok() && app.reexec {
        // The new version was installed: reopen with it and resume this same session.
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
        // On Linux the new version takes this one's place; Windows has no exec, so it opens it,
        // waits for it and exits with whatever it returns.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = std::process::Command::new(update::exe()).args(args).exec();
            return Err(err.into());
        }
        #[cfg(not(unix))]
        {
            let status = std::process::Command::new(update::exe()).args(args).status()?;
            std::process::exit(status.code().unwrap_or(0));
        }
    }
    res
}

/// Iris's screen: ratatui plus what the terminal has to report.
fn open_terminal(app: &mut App) -> ratatui::DefaultTerminal {
    let term = ratatui::init();
    // Holding space to talk requires knowing when it is released. With «disambiguate» and
    // «event types» letters keep arriving as text (accents and dead keys intact) and the
    // release arrives too. «All keys as escape codes» duplicates the é in foot.
    use crossterm::event::{KeyboardEnhancementFlags as K, PushKeyboardEnhancementFlags};
    if crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false) {
        let flags = K::DISAMBIGUATE_ESCAPE_CODES | K::REPORT_EVENT_TYPES;
        app.key_release = crossterm::execute!(std::io::stdout(), PushKeyboardEnhancementFlags(flags)).is_ok();
    }
    // With the mouse in Iris's hands, dragging copies only chat text; Shift+drag is still
    // the terminal's selection.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    // Pasted content arrives in a single event: that way a multi-line text isn't sent at the
    // first newline, and an image path dragged onto the terminal can be attached.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste);
    // Knowing whether the window has focus: to alert with a notification and, in floating
    // mode, to start listening when it appears.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableFocusChange);
    term
}

/// Restores the terminal to how it was: on exit and while `claude auth login` runs.
fn close_terminal(app: &App) {
    if app.key_release {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);
    }
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableBracketedPaste);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableFocusChange);
    ratatui::restore();
}

/// Relaunches the engine in the same conversation, with whatever is saved now (account,
/// permissions, model, effort).
fn relaunch(app: &mut App, claude: &mut Option<Claude>, extra: &[String], tx: &Sender<AppEvent>) -> Result<()> {
    let mut args: Vec<String> = vec![];
    let mut stale = extra.iter();
    while let Some(a) = stale.next() {
        if a == "--resume" {
            stale.next();
        } else {
            args.push(a.clone());
        }
    }
    if !app.session.is_empty() {
        args.extend(["--resume".into(), app.session.clone()]);
    }
    *claude = None;
    app.drop_waiting();
    *claude = Some(Claude::spawn(&args, tx.clone())?);
    app.effort = settings::read("esfuerzo").unwrap_or_else(|| "auto".into());
    app.alive = true;
    app.busy = false;
    Ok(())
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
    // 60 fps: what moves the most is the core.
    let tick = Duration::from_millis(16);
    let mut frame = Instant::now();
    let mut frames: u64 = 0;
    let mut keyboard = keyboard::Keyboard::new();
    let mut theme_check = Instant::now();
    theme::poll();
    // The colors the cached markdown was drawn with. /buddy, /core and their previews
    // change them without going through `poll`, so they are compared on every loop.
    let mut colors = (theme::accent_u32(), theme::text_u32(), theme::background_rgb());
    loop {
        // The theme can change at any time (Omarchy, Aether): it is checked once per second.
        if theme_check.elapsed() >= Duration::from_secs(1) {
            theme_check = Instant::now();
            theme::poll();
        }
        let now = (theme::accent_u32(), theme::text_u32(), theme::background_rgb());
        if now != colors {
            colors = now;
            app.md_cache.borrow_mut().clear(); // the cached markdown carries the old colors
        }
        // The waveform advances even without audio, so it looks alive.
        let lvl = if app.voice == VoiceState::Listening { voice::level_get(&app.level) } else { 0.0 };
        app.levels.remove(0);
        app.levels.push(lvl);
        if app.voice == VoiceState::Listening {
            hear(app, lvl);
        }
        floating_tick(app, voice_tx);
        // surco's music goes quiet while it listens, transcribes, thinks what to say or speaks.
        let quiet = matches!(app.voice, VoiceState::Listening | VoiceState::Transcribing)
            || app.talking
            || (app.busy && app.speak_turn);
        app.surco.quiet(quiet);
        // The new version is offered when it's not in the way: if it showed up while you type,
        // your message's Enter would accept it.
        if app.update.is_some()
            && app.modal.is_none()
            && app.tools_view.is_none()
            && app.input.is_empty()
            && !app.busy
            && app.last_key.elapsed() > Duration::from_secs(3)
        {
            app.modal = app.update.take().map(Modal::Update);
        }
        // The same for new versions of store packages, after Iris's own.
        if app.store_updates.is_some()
            && app.update.is_none()
            && app.modal.is_none()
            && app.tools_view.is_none()
            && app.input.is_empty()
            && !app.busy
            && app.last_key.elapsed() > Duration::from_secs(3)
        {
            app.modal = app.store_updates.take().map(|(index, list)| Modal::StoreUpdates { index, list });
        }

        let dt = frame.elapsed().as_secs_f64();
        frame = Instant::now();
        // Finished ones stay a few seconds in ACTIVITY and then leave.
        // Ones that finished a while ago leave the core, but stay to be seen in Ctrl+G.
        let (stale, alive): (Vec<Agent>, Vec<Agent>) =
            std::mem::take(&mut app.agents).into_iter().partition(|a| a.ended.is_some_and(|(t, _)| t.elapsed() >= Duration::from_secs(4)));
        app.agents = alive;
        app.agents_done.extend(stale);
        let excess = app.agents_done.len().saturating_sub(40);
        app.agents_done.drain(..excess);
        sync_recipe(app);
        let (want, sig) = (app.state(), app.signals());
        app.core.step(dt, want, &sig);
        broken_parts(app);
        // The keyboard follows the core: its state, the voice and the events.
        keyboard.tick(app.core.state(), sig.level);
        for ev in app.core.take_fired() {
            keyboard.event(ev);
            if let Some(l) = &app.live {
                let _ = l.send(live::event(&format!("{ev:?}").to_lowercase()));
            }
        }
        // The web sees your Iris live: its state and buddy, each time they change.
        let now = (format!("{:?}", app.core.state()).to_lowercase(), app.buddy.id());
        if app.live_said.as_ref() != Some(&now) {
            if let Some(l) = &app.live {
                let _ = l.send(live::state(&now.0, &now.1));
            }
            app.live_said = Some(now);
        }

        app.core_rect.set(None);
        app.thumb_slots.borrow_mut().clear();
        app.thumb_targets.borrow_mut().clear();
        app.file_slots.borrow_mut().clear();
        app.file_targets.borrow_mut().clear();
        // What was drawn in this frame: needed to rewrite the text that a thumbnail which moved
        // was covering.
        let snap = term.draw(|f| ui::draw(f, app))?.buffer.clone();
        if sixel_frame(term, app, &mut frames)? {
            app.thumbs_shown.clear(); // the screen was cleared: they must be redrawn
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
                    if app.floating && app.voice == VoiceState::Ready && !app.busy && !app.talking {
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
                // Repeats are for deleting or moving with the key held down.
                if k.kind == KeyEventKind::Press || (k.kind == KeyEventKind::Repeat && !space) {
                    match handle_key(app, k, claude, voice_tx) {
                        Flow::Quit => return Ok(()),
                        Flow::Restart => {
                            *claude = None;
                            app.drop_waiting();
                            *claude = Some(Claude::spawn(extra, tx.clone())?);
                            app.effort = settings::read("esfuerzo").unwrap_or_else(|| "auto".into());
                            app.alive = true;
                            app.modal = None;
                            app.ctx_used = 0;
                            app.todos = (0, 0);
                            app.booted = Instant::now();
                            app.core.reboot();
                            app.agents.clear();
                            app.push(Role::System, "claude reiniciado — sesión nueva");
                        }
                        Flow::NewChat => {
                            *claude = None;
                            app.drop_waiting();
                            *claude = Some(Claude::spawn(extra, tx.clone())?);
                            app.effort = settings::read("esfuerzo").unwrap_or_else(|| "auto".into());
                            app.alive = true;
                            app.busy = false;
                            app.modal = None;
                            app.messages.clear();
                            app.md_cache.borrow_mut().clear();
                            app.attachments_cache.borrow_mut().clear();
                            app.activity.clear();
                            app.ctx_used = 0;
                            app.todos = (0, 0);
                            app.booted = Instant::now();
                            app.core.reboot();
                            app.agents.clear();
                            app.push(Role::System, "conversación nueva · la anterior se retoma con /resume");
                            term.clear()?;
                        }
                        Flow::Resume(id) => {
                            let mut args = extra.to_vec();
                            args.extend(["--resume".into(), id.clone()]);
                            *claude = None;
                            *claude = Some(Claude::spawn(&args, tx.clone())?);
                            app.effort = settings::read("esfuerzo").unwrap_or_else(|| "auto".into());
                            app.alive = true;
                            app.busy = false;
                            app.modal = None;
                            app.activity.clear();
                            app.todos = (0, 0);
                            app.booted = Instant::now();
                            app.core.reboot();
                            app.agents.clear();
                            // The session's chat, tools and subagents: Ctrl+G shows them again.
                            let replay = sessions::replay(&id);
                            app.messages = replay.messages;
                            app.activity = replay.activity;
                            app.agents_done = replay.agents;
                            app.turn_from = app.activity.len();
                            app.md_cache.borrow_mut().clear();
                            app.attachments_cache.borrow_mut().clear();
                            app.push(Role::System, format!("sesión {} retomada", &id[..8]));
                            term.clear()?;
                        }
                        Flow::Redraw => term.clear()?,
                        Flow::Relaunch => {
                            relaunch(app, claude, extra, tx)?;
                            app.push(Role::System, "motor relanzado con los ajustes nuevos");
                            term.clear()?;
                        }
                        Flow::Login => {
                            close_terminal(app);
                            let logged_in = login::log_in();
                            *term = open_terminal(app);
                            term.clear()?;
                            app.sixel_shown = None;
                            app.thumbs_shown.clear();
                            match logged_in {
                                Ok(true) => {
                                    // The engine read the credentials at startup: relaunch it so
                                    // it uses the new account.
                                    relaunch(app, claude, extra, tx)?;
                                    let who = match login::account() {
                                        Some(login::Account::Connected(Some(email))) => format!(" como {email}"),
                                        _ => String::new(),
                                    };
                                    app.no_session = false;
                                    app.push(Role::System, format!("sesión iniciada{who}"));
                                }
                                Ok(false) => app.push(Role::Error, "no se inició sesión; vuelve a intentarlo con /login"),
                                Err(e) => app.push(Role::Error, format!("no pude abrir claude auth login: {e}")),
                            }
                        }
                        Flow::Update => {
                            app.push(Role::System, "actualizando: bajando la versión nueva…");
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
                AppEvent::Speech(e) => match e {
                    speech::SpeechEvent::Start => app.talking = true,
                    speech::SpeechEvent::Level(l) => app.tts_level = l,
                    speech::SpeechEvent::Idle => {
                        app.talking = false;
                        app.tts_level = 0.0;
                        if app.floating && !app.busy {
                            app.hide_at = Some(Instant::now() + Duration::from_secs(3));
                        }
                    }
                    speech::SpeechEvent::Unavailable(why) => {
                        app.push(Role::Error, format!("sin voz: {why}"));
                    }
                },
                AppEvent::Store(e) => storefront::on_event(app, e),
                AppEvent::Live(e) => match e {
                    live::LiveEvent::Draft(name) => storefront::on_event(app, storefront::StoreEvent::Draft(name)),
                    live::LiveEvent::Review { name, version, status, reason } => {
                        let what = match status.as_str() {
                            "published" => format!("«{name}» {version} ya está publicado en la tienda"),
                            "rejected" => format!("«{name}» {version} fue rechazado: {}", reason.unwrap_or_default()),
                            "revoked" => format!("«{name}» {version} fue retirado de la tienda: {}", reason.unwrap_or_default()),
                            other => format!("«{name}» {version}: {other}"),
                        };
                        app.push(Role::System, what);
                    }
                    live::LiveEvent::Connected | live::LiveEvent::Store => {
                        // Say again where Iris is, and look for updates of what's installed now.
                        app.live_said = None;
                        let tx = app.events.clone();
                        std::thread::spawn(move || {
                            let Ok((index, _)) = store::refresh() else { return };
                            let no = store::dismissed();
                            let list: Vec<_> = store::updates(&index).into_iter().filter(|e| !no.contains(&format!("{} {}", e.name, e.version))).collect();
                            if !list.is_empty() {
                                let _ = tx.send(AppEvent::Store(storefront::StoreEvent::Updates(index, list)));
                            }
                        });
                    }
                    live::LiveEvent::IrisVersion => {
                        let tx = app.events.clone();
                        std::thread::spawn(move || {
                            if let Some(info) = update::check() {
                                let _ = tx.send(AppEvent::Update(update::UpdateEvent::Available(info)));
                            }
                        });
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
    /// `/clear`: new conversation, like in Claude Code. The previous one stays saved.
    NewChat,
    Redraw,
    Quit,
    Restart,
    Resume(String),
    /// Download and install the new version.
    Update,
    /// `/login`: log in to Claude with the whole terminal.
    Login,
    /// Relaunch the engine in the same conversation: permissions, model or effort changed.
    Relaunch,
}
