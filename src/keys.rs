//! The keyboard: global shortcuts, prompt editing, menus and open lists.

use crate::*;

/// Opens Ctrl+G on a tab with the latest item selected; with nothing to show, it says so and doesn't open.
pub(crate) fn open_view(app: &mut App, agents: bool) -> Option<(usize, usize)> {
    let n = if agents { app.agents_done.len() + app.agents.len() } else { app.activity.len() };
    if n == 0 {
        let msg = if agents { "todavía no lancé ningún subagente" } else { "todavía no usé ninguna herramienta" };
        app.flash = Some((msg.into(), Instant::now()));
        return None;
    }
    // Agents are read from the bottom, like a terminal; tools, from the top.
    Some((n - 1, if agents { usize::MAX } else { 0 }))
}

pub(crate) fn handle_key(
    app: &mut App,
    k: KeyEvent,
    claude: &mut Option<Claude>,
    voice_tx: &Sender<VoiceCmd>,
) -> Flow {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    app.sel = None;
    app.last_activity = Instant::now();

    // Ctrl+G: what each tool did, in full, and on the other tab what each subagent did.
    // While open it takes the arrow keys.
    if ctrl && k.code == KeyCode::Char('g') {
        app.tools_view = match app.tools_view {
            Some(_) => None,
            None => open_view(app, app.agents_tab),
        };
        return Flow::Go;
    }
    if app.tools_view.is_some() && k.code == KeyCode::Tab {
        app.agents_tab = !app.agents_tab;
        app.tools_view = open_view(app, app.agents_tab).or(app.tools_view);
        return Flow::Go;
    }
    if let Some((sel, scroll)) = app.tools_view.as_mut() {
        let n = if app.agents_tab { app.agents_done.len() + app.agents.len() } else { app.activity.len() }.max(1);
        let scroll_top = if app.agents_tab { usize::MAX } else { 0 };
        match k.code {
            KeyCode::Up => (*sel, *scroll) = (sel.saturating_sub(1), scroll_top),
            KeyCode::Down => (*sel, *scroll) = ((*sel + 1).min(n - 1), scroll_top),
            KeyCode::PageUp => *scroll = (*scroll).min(app.tools_view_max.get()).saturating_sub(10),
            KeyCode::PageDown => *scroll += 10,
            KeyCode::Home => *scroll = 0,
            KeyCode::End => *scroll = usize::MAX,
            KeyCode::Esc => app.tools_view = None,
            _ => {}
        }
        return Flow::Go;
    }

    // Modals don't take Ctrl (Ctrl+C and the rest stay global); the store login form takes
    // Ctrl+N, to switch between logging in and creating the account.
    let login_toggle = k.code == KeyCode::Char('n') && matches!(app.modal, Some(Modal::StoreLogin(_)));
    if !ctrl || login_toggle {
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
        // Ctrl+C asks for confirmation: press it again before the notice clears.
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
        // Prompt editing, like in a shell.
        KeyCode::Char('a') if ctrl => app.cur = 0,
        KeyCode::Char('e') if ctrl => app.cur = app.input.chars().count(),
        KeyCode::Char('w') if ctrl => input::kill_word(&mut app.input, &mut app.cur),
        KeyCode::Char('k') if ctrl => input::kill_to_end(&mut app.input, &mut app.cur),
        KeyCode::Left if ctrl => input::word_left(&app.input, &mut app.cur),
        KeyCode::Right if ctrl => input::word_right(&app.input, &mut app.cur),
        KeyCode::Left => input::left(&app.input, &mut app.cur),
        KeyCode::Right => input::right(&app.input, &mut app.cur),
        KeyCode::Delete => input::delete(&mut app.input, &mut app.cur),
        KeyCode::Home if !app.input.is_empty() => app.cur = 0,
        KeyCode::End if !app.input.is_empty() => app.cur = app.input.chars().count(),
        // Shift+Enter (or Alt+Enter): a new line without sending.
        KeyCode::Enter if k.modifiers.intersects(KeyModifiers::SHIFT | KeyModifiers::ALT) => {
            input::insert(&mut app.input, &mut app.cur, "\n");
            app.last_key = Instant::now();
        }
        KeyCode::Char('v') if ctrl => paste(app),
        KeyCode::Char('o') if ctrl => app.read_override = Some(!app.reading.get()),
        KeyCode::Char('t') if ctrl => {
            app.transcript = !app.transcript;
            app.scroll = 0;
        }
        // With the conversation expanded, Esc only closes it.
        KeyCode::Esc if app.transcript => {
            app.transcript = false;
            app.scroll = 0;
        }
        // Outside Cine, ^O is a separate view: Esc closes it, like ^T.
        KeyCode::Esc if app.reading.get() && app.read_override == Some(true) && app.style != style::UiStyle::Cine => {
            app.read_override = None;
        }

        // Space with empty input: start/stop listening.
        KeyCode::Char(' ') if app.input.is_empty() && space_repeat(app) => {}
        KeyCode::Char(' ') if app.input.is_empty() => match app.voice {
            VoiceState::Ready => {
                app.speech.stop(); // if it was talking, it goes quiet to listen to you
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
        // In the floating window, with nothing in progress, Esc hides it.
        KeyCode::Esc if app.floating && !app.busy && !app.talking && app.voice == VoiceState::Ready => hide_floating(),
        KeyCode::Esc => {
            // Esc always silences it; if it's also working, it interrupts it.
            app.speech.stop();
            app.speak_turn = false;
            if app.voice == VoiceState::Listening {
                let _ = voice_tx.send(VoiceCmd::Cancel);
            } else if app.busy {
                if let Some(c) = claude {
                    let _ = c.interrupt();
                }
                app.core.fire(Gesture::Cancel);
                app.interrupted = true;
                app.push(Role::System, "interrumpido");
            }
        }
        KeyCode::Enter => {
            let text = app.input.trim().to_string();
            app.input.clear();
            app.cur = 0;
            app.history.push(&text);
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
                "/effort" => {
                    effort(app, claude, arg.trim());
                    return Flow::Go;
                }
                "/restart" => return Flow::Restart,
                "/login" if app.busy => {
                    app.push(Role::Error, "espera a que termine el turno (o Esc) antes de iniciar sesión");
                    return Flow::Go;
                }
                "/login" => return Flow::Login,
                "/welcome" => {
                    app.modal = Some(Modal::Welcome(welcome::Welcome::new(app)));
                    return Flow::Redraw;
                }
                "/voice" => {
                    app.voice_mode = match (arg.trim(), app.voice_mode) {
                        ("always", _) | ("on", _) => VoiceMode::Always,
                        ("never", _) | ("off", _) => VoiceMode::Never,
                        ("auto", _) => VoiceMode::Auto,
                        (_, VoiceMode::Auto) => VoiceMode::Always,
                        (_, VoiceMode::Always) => VoiceMode::Never,
                        (_, VoiceMode::Never) => VoiceMode::Auto,
                    };
                    if app.voice_mode == VoiceMode::Never {
                        app.speech.stop();
                    }
                    let msg = match app.voice_mode {
                        VoiceMode::Auto => "voz: contesta hablando cuando le hablas",
                        VoiceMode::Always => "voz: contesta hablando siempre",
                        VoiceMode::Never => "voz: no habla",
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
                    match arg.trim() {
                        "publish" | "publicar" => crate::storefront::publish_buddy(app),
                        "edit" | "editar" => edit_buddy(app),
                        a => choose_buddy(app, a),
                    }
                    return Flow::Go;
                }
                "/store" | "/tienda" => {
                    crate::storefront::command(app, arg.trim());
                    return Flow::Go;
                }
                "/core" => {
                    color(app, arg.trim());
                    return Flow::Go;
                }
                "/agents" => {
                    app.agents_tab = true;
                    app.tools_view = open_view(app, true);
                    return Flow::Go;
                }
                "/quit" => return Flow::Quit,
                _ => submit(app, claude, text),
            }
        }
        // With empty input, backspace removes the last attached image.
        KeyCode::Backspace if app.input.is_empty() && !app.images.is_empty() => {
            app.images.pop();
        }
        KeyCode::Backspace => {
            input::backspace(&mut app.input, &mut app.cur);
            app.menu = 0;
        }
        KeyCode::Char(c) => {
            // With the kitty protocol, Shift+n may arrive as "n" plus the modifier.
            let typed: String = if k.modifiers.contains(KeyModifiers::SHIFT) && c.is_lowercase() {
                c.to_uppercase().collect()
            } else {
                c.to_string()
            };
            input::insert(&mut app.input, &mut app.cur, &typed);
            app.menu = 0;
            app.last_key = Instant::now();
            app.core.fire(Gesture::Key);
        }
        KeyCode::PageUp => scroll(app, true, 10),
        KeyCode::PageDown => scroll(app, false, 10),
        // ↑ ↓ browse what you sent; when reading in Cine, they scroll the open text.
        KeyCode::Up if app.reading.get() && !app.history.browsing() => scroll(app, true, 1),
        KeyCode::Down if app.reading.get() && !app.history.browsing() => scroll(app, false, 1),
        KeyCode::Up => {
            if let Some(t) = app.history.prev(&app.input) {
                app.cur = t.chars().count();
                app.input = t;
            }
        }
        KeyCode::Down => {
            if let Some(t) = app.history.next() {
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

/// Space with empty input, two modes: hold it (listens until released) or tap it (one tap
/// starts, another sends). What arrives while it's still held down is auto-repeat.
pub(crate) const REPEAT_GAP: Duration = Duration::from_millis(80);

pub(crate) fn space_repeat(app: &mut App) -> bool {
    if app.key_release {
        let paired = app.released.take().is_some_and(|t| t.elapsed() < REPEAT_GAP);
        return std::mem::replace(&mut app.space_down, true) || paired;
    }
    // Without release events: repeat arrives every ~25 ms and a person can't tap that fast.
    let repeat = app.last_space.elapsed() < Duration::from_millis(150);
    app.last_space = Instant::now();
    repeat
}

/// Releasing after holding it a while means "I'm done talking"; releasing a short tap does
/// nothing and keeps listening until the next tap.
pub(crate) fn space_up(app: &mut App, voice_tx: &Sender<VoiceCmd>) {
    let Some(t) = app.ptt.take() else { return };
    if t.elapsed() < Duration::from_millis(400) {
        app.ptt = None;
    } else if app.voice == VoiceState::Listening {
        let _ = voice_tx.send(VoiceCmd::Stop);
    }
}

/// Keys taken by the overlay; `None` lets the key through to normal handling
/// (typing, deleting, talking with space).
pub(crate) fn modal_key(app: &mut App, k: KeyEvent, claude: &mut Option<Claude>) -> Option<Flow> {
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
        Modal::Store(_) => {
            crate::storefront::on_key(app, k);
            Some(Flow::Redraw)
        }
        Modal::StoreLogin(_) => {
            crate::storefront::login_key(app, k);
            Some(Flow::Redraw)
        }
        Modal::StoreUpdates { .. } => {
            match k.code {
                KeyCode::Enter => {
                    if let Some(Modal::StoreUpdates { index, list }) = app.modal.take() {
                        crate::storefront::install_updates(app, index, list);
                    }
                }
                KeyCode::Esc => {
                    if let Some(Modal::StoreUpdates { list, .. }) = app.modal.take() {
                        crate::store::dismiss(&list);
                        app.flash = Some(("no te las vuelvo a ofrecer · /store update para instalarlas".into(), std::time::Instant::now()));
                    }
                }
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
        Modal::Effort { sel } => {
            let n = commands::EFFORTS.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Enter => {
                    let level = commands::EFFORTS[*sel].0;
                    app.modal = None;
                    effort(app, claude, level);
                }
                KeyCode::Esc => app.modal = None,
                _ => {}
            }
            Some(Flow::Go)
        }
        Modal::UiStyle { sel, before } => {
            let n = style::ALL.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Char(c @ '1'..='9') if ((c as u8 - b'1') as usize) < n => *sel = (c as u8 - b'1') as usize,
                KeyCode::Enter => {
                    app.modal = None;
                    set_style(app, app.style);
                    return Some(Flow::Redraw);
                }
                KeyCode::Esc => {
                    app.style = *before;
                    app.modal = None;
                    return Some(Flow::Redraw);
                }
                _ => {}
            }
            // Preview: the screen changes while choosing.
            app.style = style::ALL[*sel].0;
            Some(Flow::Redraw)
        }
        Modal::Buddy { sel, before, list } => {
            let n = list.len().max(1);
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Char(c @ '1'..='9') if ((c as u8 - b'1') as usize) < n => *sel = (c as u8 - b'1') as usize,
                KeyCode::Enter => {
                    app.modal = None;
                    set_buddy(app, app.buddy.clone());
                    return Some(Flow::Redraw);
                }
                KeyCode::Esc => {
                    app.buddy = before.clone();
                    app.modal = None;
                    return Some(Flow::Redraw);
                }
                _ => {}
            }
            // Preview: the core changes while choosing.
            if let Some(e) = list.get(*sel) {
                app.buddy = e.buddy.clone();
            }
            Some(Flow::Redraw)
        }
        Modal::Color { sel, before } => {
            let n = baymax::OPTIONS.len();
            match k.code {
                KeyCode::Up => *sel = (*sel + n - 1) % n,
                KeyCode::Down => *sel = (*sel + 1) % n,
                KeyCode::Char(c @ '1'..='9') if ((c as u8 - b'1') as usize) < n => *sel = (c as u8 - b'1') as usize,
                KeyCode::Enter => {
                    let m = baymax::OPTIONS[*sel].2;
                    app.modal = None;
                    set_color(app, m);
                    return Some(Flow::Redraw);
                }
                KeyCode::Esc => {
                    baymax::set_core(*before);
                    app.modal = None;
                    return Some(Flow::Redraw);
                }
                _ => {}
            }
            baymax::set_core(baymax::OPTIONS[*sel].2);
            Some(Flow::Redraw)
        }
        Modal::Welcome(_) => {
            // Taken out of the modal so `app` can be touched while choosing.
            let Some(Modal::Welcome(mut b)) = app.modal.take() else { unreachable!() };
            use welcome::Action;
            let action = match k.code {
                KeyCode::Up => b.move_sel(app, false),
                KeyCode::Down => b.move_sel(app, true),
                KeyCode::Char(c @ '1'..='9') => b.choose_number(app, (c as u8 - b'1') as usize),
                KeyCode::Left | KeyCode::Backspace => b.back(app),
                KeyCode::Enter => b.choose(app),
                KeyCode::Esc => b.skip(app),
                _ => Action::Nothing,
            };
            let flow = match action {
                Action::Nothing => {
                    app.modal = Some(Modal::Welcome(b));
                    Flow::Redraw
                }
                Action::Login => {
                    app.modal = Some(Modal::Welcome(b));
                    Flow::Login
                }
                Action::Finish { relaunch } => {
                    app.flash = Some(("listo: /welcome lo repite cuando quieras".into(), Instant::now()));
                    if relaunch { Flow::Relaunch } else { Flow::Redraw }
                }
            };
            Some(flow)
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

/// Up or down: the reply open in Cine if there is one; otherwise, the conversation.
pub(crate) fn scroll(app: &mut App, up: bool, n: usize) {
    if app.reading.get() {
        let top = app.read_top.get();
        app.read_top.set(if up { top.saturating_sub(n) } else { top.saturating_add(n) });
    } else if up {
        app.scroll += n;
    } else {
        app.scroll = app.scroll.saturating_sub(n);
    }
}
