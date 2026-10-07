//! El teclado: atajos globales, edición de la orden, menús y listas abiertas.

use crate::*;

/// Abre Ctrl+G en una pestaña con lo último elegido; sin nada que mostrar, avisa y no abre.
pub(crate) fn abrir_vista(app: &mut App, agentes: bool) -> Option<(usize, usize)> {
    let n = if agentes { app.agents_done.len() + app.agents.len() } else { app.activity.len() };
    if n == 0 {
        let msg = if agentes { "todavía no lancé ningún subagente" } else { "todavía no usé ninguna herramienta" };
        app.flash = Some((msg.into(), Instant::now()));
        return None;
    }
    // Los agentes se leen desde el final, como una terminal; las herramientas, desde arriba.
    Some((n - 1, if agentes { usize::MAX } else { 0 }))
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
pub(crate) const REPEAT_GAP: Duration = Duration::from_millis(80);

pub(crate) fn space_repeat(app: &mut App) -> bool {
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
pub(crate) fn space_up(app: &mut App, voice_tx: &Sender<VoiceCmd>) {
    let Some(t) = app.ptt.take() else { return };
    if t.elapsed() < Duration::from_millis(400) {
        app.ptt = None;
    } else if app.voice == VoiceState::Listening {
        let _ = voice_tx.send(VoiceCmd::Stop);
    }
}

/// Teclas que se lleva lo superpuesto; `None` deja pasar la tecla al manejo normal
/// (escribir, borrar, hablar con espacio).
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

/// Hacia arriba o abajo: la respuesta abierta en Cine si la hay; si no, la conversación.
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
