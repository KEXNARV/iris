//! El estilo Cine de /theme.

use crate::ui::*;

pub(in crate::ui) fn cine(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let hear = listening(state);

    // Las cuatro esquinas.
    {
        let buf = f.buffer_mut();
        let top = area.y + 1;
        let bot = area.bottom().saturating_sub(2);
        put(buf, area.x + 3, top, &spaced("IRIS"), Style::new().fg(theme::accent()).bold(), 20);
        put_right(buf, area.right().saturating_sub(4), top, &now_hhmmss(), Style::new().fg(theme::text()));
        let x = put(buf, area.x + 3, bot, "ctx ", Style::new().fg(theme::faint()), 4);
        let x = meter(buf, x, bot, 20, ctx_pct(app) / 100.0, tone);
        put(buf, x + 1, bot, &format!("{:.0}%", ctx_pct(app)), Style::new().fg(theme::faint()), 5);
        let right = format!("{} · ${:.2} · {}", app.model.trim_start_matches("claude-"), app.cost, turns(app));
        put_right(buf, area.right().saturating_sub(4), bot, &right, Style::new().fg(theme::faint()));
    }

    // ^T: la conversación entera como una cortina, con su markdown; la orden sigue abajo.
    if app.transcript {
        return transcript_view(f, app, Rect { y: area.y + 3, height: area.height.saturating_sub(3), ..area });
    }

    let inner = Rect {
        x: area.x + 3,
        y: area.y + 3,
        width: area.width.saturating_sub(6),
        height: area.height.saturating_sub(6),
    };
    // La respuesta que se abre para leer: la más nueva que se ve en el historial (con pgup se
    // puede ir a una anterior). Se abre sola si terminó el turno y no cabe en el subtítulo.
    let said: Vec<(usize, &crate::Msg)> = app
        .messages
        .iter()
        .enumerate()
        .filter(|(_, m)| matches!(m.role, Role::User | Role::Assistant) && m.waiting.is_none())
        .collect();
    let skip = (app.scroll / 5).min(said.len().saturating_sub(1));
    let pick = said.iter().rev().skip(skip).find(|(_, m)| m.role == Role::Assistant).copied();
    let long = pick.is_some_and(|(_, m)| wrap(&plain(&m.text), 60).len() > 4 || m.text.contains('\n'));
    let reading = !hear && pick.is_some() && app.read_override.unwrap_or(!app.busy && long);
    app.reading.set(reading);

    let center_w = inner.width.min(if reading { 110 } else { 72 });
    let center = Rect { x: inner.x + (inner.width - center_w) / 2, width: center_w, ..inner };

    // Centro: núcleo, estado, subtítulo (o la respuesta entera) y la orden.
    let caption_h = if hear { 6 } else { 5 };
    let [core, label, caption, orden, keyrow] = if reading {
        Layout::vertical([
            Constraint::Length(9.min(center.height / 4).max(4)),
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(center)
    } else {
        Layout::vertical([
            Constraint::Min(6),
            Constraint::Length(2),
            Constraint::Length(caption_h),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(center)
    };
    let core_w = core.width.min(core.height * 2 + 8);
    core_draw(app, Rect { x: core.x + (core.width - core_w) / 2, width: core_w, ..core }, f.buffer_mut());
    let (title, detail) = core_label(app, t, label.width as usize);
    {
        let buf = f.buffer_mut();
        put_center(buf, label, label.y, &spaced_title(&title), Style::new().fg(tone).bold());
        put_center(buf, label, label.y + 1, &detail, Style::new().fg(theme::faint()));
    }

    if let (true, Some((i, m))) = (reading, pick) {
        // Los archivos que entrega van en la última fila, debajo de la respuesta.
        let files = app.adjuntos(i, &m.text);
        if files.is_empty() {
            read_panel(f, caption, app, i, &m.text);
        } else {
            read_panel(f, Rect { height: caption.height.saturating_sub(1), ..caption }, app, i, &m.text);
            chips(f, Rect { y: caption.bottom().saturating_sub(1), height: 1, ..caption }, app, &files);
        }
    }
    let mut subtitle: Option<crate::select::View> = None;
    // El subtítulo: lo último que se dijo, con el final a la vista mientras llega.
    let last = app.messages.iter().enumerate().rev().find(|(_, m)| matches!(m.role, Role::User | Role::Assistant) && m.waiting.is_none());
    let last = if reading { None } else { last };
    let files = match last {
        Some((i, m)) if m.role == Role::Assistant && !hear => app.adjuntos(i, &m.text),
        _ => vec![],
    };
    let cap_lines = if hear { 2 } else { (caption.height as usize).saturating_sub(1 + (!files.is_empty()) as usize).max(1) };
    if let Some((_, m)) = last {
        let text = plain(&m.text);
        let ls = wrap(&text, caption.width.saturating_sub(4) as usize);
        let tail = &ls[ls.len().saturating_sub(cap_lines)..];
        let st = match (m.role, hear) {
            (_, true) => Style::new().fg(theme::faint()),
            (Role::User, _) => Style::new().fg(theme::faint()),
            _ => Style::new().fg(theme::text()),
        };
        let buf = f.buffer_mut();
        for (i, l) in tail.iter().enumerate() {
            put_center(buf, caption, caption.y + 1 + i as u16, l, st);
        }
        // El subtítulo también se selecciona y se copia con el mouse.
        let area = Rect { y: caption.y + 1, height: tail.len() as u16, ..caption };
        let mut view = crate::select::View { area, rows: Vec::with_capacity(tail.len()), first: 0 };
        for (i, l) in tail.iter().enumerate() {
            let pad = (caption.width as usize).saturating_sub(l.width().min(caption.width as usize)) / 2;
            view.rows.push(crate::select::Row { text: format!("{}{l}", " ".repeat(pad)), skip: pad, cont: i > 0 });
        }
        subtitle = Some(view);
        if !files.is_empty() {
            let row = Rect { y: caption.y + 1 + tail.len() as u16, height: 1, ..caption };
            chips(f, row, app, &files);
        }
    } else if !reading {
        let buf = f.buffer_mut();
        put_center(buf, caption, caption.y + 1, &format!("{}.", greeting()), Style::new().fg(theme::text()).bold());
        put_center(buf, caption, caption.y + 2, "¿En qué trabajamos?", Style::new().fg(theme::accent()).bold());
    }
    if hear {
        let w = Rect { x: caption.x + 2, y: caption.y + 3, width: caption.width.saturating_sub(4), height: 3 };
        wave(f, w, app, t, tone, theme::dim());
    }

    // La orden: una caja que se ve, del color del estado.
    let active = !app.busy || hear;
    let col = if hear { tone } else if active { theme::accent() } else { theme::dim() };
    {
        let buf = f.buffer_mut();
        let (x0, x1) = (orden.x, orden.right() - 1);
        let st = Style::new().fg(col);
        put(buf, x0, orden.y, &format!("╭{}╮", "─".repeat(orden.width.saturating_sub(2) as usize)), st, orden.width);
        put(buf, x0, orden.y + 2, &format!("╰{}╯", "─".repeat(orden.width.saturating_sub(2) as usize)), st, orden.width);
        put(buf, x0, orden.y + 1, "│", st, 1);
        put(buf, x1, orden.y + 1, "│", st, 1);
        let lab = if active { " escribe aquí " } else { " o escribe aquí " };
        put(buf, x0 + 2, orden.y, lab, st.bold(), orden.width.saturating_sub(4));
        put(buf, x0 + 2, orden.y + 1, "›", Style::new().fg(col).bold(), 1);
    }
    let line = Rect { x: orden.x + 4, y: orden.y + 1, width: orden.width.saturating_sub(6), height: 1 };
    let spans = if hear && app.input.is_empty() {
        vec![Span::styled("te escucho… suelta espacio para enviar", Style::new().fg(theme::faint()))]
    } else {
        input_spans(app, state, t, line.width as usize, col)
    };
    f.render_widget(Paragraph::new(Line::from(spans)), line);
    {
        let buf = f.buffer_mut();
        match flash(app) {
            Some(l) => {
                let w = l.width() as u16;
                buf.set_line(keyrow.x + keyrow.width.saturating_sub(w) / 2, keyrow.y, &l, keyrow.width);
            }
            None => {
                let mut items = hints_for(app, state);
                if reading {
                    items.insert(0, ("^O", "núcleo"));
                } else if long && !hear {
                    items.insert(0, ("^O", "leer entera"));
                }
                let spans: Vec<(String, Style)> = items
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (k, d))| {
                        let sep = if i == 0 { String::new() } else { "   ".into() };
                        [(sep, Style::new()), ((*k).to_string(), Style::new().fg(col)), (format!(" {d}"), Style::new().fg(theme::faint()))]
                    })
                    .collect();
                put_line_center(buf, keyrow, keyrow.y, &spans);
            }
        }
    }

    // Lo que se puede seleccionar con el mouse: la respuesta abierta (ya puesta por
    // read_panel) o, si no, el subtítulo.
    if !reading {
        *app.view.borrow_mut() = subtitle.unwrap_or_default();
    }
    if let Some(sel) = &app.sel {
        let view = app.view.borrow();
        let buf = f.buffer_mut();
        for (x, y) in sel.cells(&view) {
            buf[(x, y)].set_bg(theme::dim()).set_fg(Color::White);
        }
    }

    if !hear {
        draw_modal(f, Rect { y: orden.y, height: 1, ..center }, app);
    }
}
