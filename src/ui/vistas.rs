//! Lo que se abre encima de cualquier estilo: ^G (herramientas y agentes), ^T (la conversación
//! entera) y ^O (la respuesta entera), con la orden debajo.

use super::*;

/// Ctrl+G: la lista de herramientas a la izquierda y la elegida entera a la derecha.
pub(in crate::ui) fn tools_overlay(f: &mut Frame, app: &App) {
    use crate::ToolStatus;
    if app.agents_tab {
        return agents_overlay(f, app);
    }
    let Some((sel, scroll)) = app.tools_view else { return };
    let a = f.area();
    let area = Rect { x: a.x + 2, y: a.y + 1, width: a.width.saturating_sub(4), height: a.height.saturating_sub(2) };
    limpiar(f, area);
    let block = panel("HERRAMIENTAS · tab agentes · ↑↓ elegir · pgup/pgdn recorrer · esc cierra");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let list_w = (inner.width / 3).clamp(28, 44);
    let [list, _, detail] =
        Layout::horizontal([Constraint::Length(list_w), Constraint::Length(2), Constraint::Min(20)]).areas(inner);

    // La lista, con la elegida siempre a la vista.
    let n = app.activity.len();
    let sel = sel.min(n.saturating_sub(1));
    let h = list.height as usize;
    let first = (sel + 1).saturating_sub(h);
    let rows: Vec<Line> = app
        .activity
        .iter()
        .enumerate()
        .skip(first)
        .take(h)
        .map(|(i, t)| {
            let (icon, col) = match t.status {
                ToolStatus::Running => ("◐", WARM),
                ToolStatus::Ok => ("✓", GREEN),
                ToolStatus::Err => ("✗", RED),
            };
            let secs = t.took.unwrap_or_else(|| t.started.elapsed()).as_secs_f64();
            let label = truncate(&format!("{} {}", t.name, tool_summary(t)), (list.width as usize).saturating_sub(10));
            let st = if i == sel { Style::new().fg(Color::Black).bg(theme::accent()) } else { Style::new().fg(theme::text()) };
            Line::from(vec![
                Span::styled(format!("{icon} "), if i == sel { st } else { Style::new().fg(col) }),
                Span::styled(format!("{label:<w$}", w = (list.width as usize).saturating_sub(9)), st),
                Span::styled(format!("{secs:>5.1}s"), if i == sel { st } else { Style::new().fg(theme::dim()) }),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(rows), list);

    // El detalle de la elegida.
    let Some(t) = app.activity.get(sel) else { return };
    let w = detail.width as usize;
    let mut lines: Vec<Line> = vec![];
    let head = |s: &str| Line::from(Span::styled(s.to_string(), Style::new().fg(theme::accent()).bold()));
    let (state, col) = match t.status {
        ToolStatus::Running => ("en curso", WARM),
        ToolStatus::Ok => ("terminó bien", GREEN),
        ToolStatus::Err => ("terminó con error", RED),
    };
    let secs = t.took.unwrap_or_else(|| t.started.elapsed()).as_secs_f64();
    lines.push(Line::from(vec![
        Span::styled(t.name.clone(), Style::new().fg(theme::text()).bold()),
        Span::styled(format!("  ·  {state}  ·  {secs:.1}s"), Style::new().fg(col)),
    ]));
    lines.push(Line::default());
    let push_wrapped = |lines: &mut Vec<Line>, text: &str, st: Style, prefix: &str| {
        for raw in text.lines() {
            let raw = raw.replace('\t', "    ");
            for piece in wrap(&raw, w.saturating_sub(prefix.chars().count()).max(10)) {
                lines.push(Line::from(Span::styled(format!("{prefix}{piece}"), st)));
            }
            if raw.is_empty() {
                lines.push(Line::from(Span::styled(prefix.to_string(), st)));
            }
        }
    };
    let s = |k: &str| t.input.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    lines.push(head("ENTRADA"));
    match t.name.as_str() {
        "Bash" => {
            if !s("description").is_empty() {
                push_wrapped(&mut lines, &s("description"), Style::new().fg(theme::faint()), "");
            }
            push_wrapped(&mut lines, &s("command"), Style::new().fg(GREEN), "$ ");
        }
        "Edit" | "MultiEdit" => {
            push_wrapped(&mut lines, &s("file_path"), Style::new().fg(theme::faint()), "");
            let edits: Vec<(String, String)> = match t.input.get("edits").and_then(|e| e.as_array()) {
                Some(es) => es
                    .iter()
                    .map(|e| {
                        let g = |k: &str| e.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        (g("old_string"), g("new_string"))
                    })
                    .collect(),
                None => vec![(s("old_string"), s("new_string"))],
            };
            for (old, new) in edits {
                lines.push(Line::default());
                push_wrapped(&mut lines, &old, Style::new().fg(RED), "- ");
                push_wrapped(&mut lines, &new, Style::new().fg(GREEN), "+ ");
            }
        }
        "Write" => {
            push_wrapped(&mut lines, &s("file_path"), Style::new().fg(theme::faint()), "");
            push_wrapped(&mut lines, &s("content"), Style::new().fg(GREEN), "+ ");
        }
        _ => {
            let pretty = serde_json::to_string_pretty(&t.input).unwrap_or_default();
            push_wrapped(&mut lines, &pretty, Style::new().fg(theme::faint()), "");
        }
    }
    lines.push(Line::default());
    lines.push(head("SALIDA"));
    if t.output.trim().is_empty() {
        let none = if t.status == ToolStatus::Running { "todavía corriendo…" } else { "(sin salida)" };
        lines.push(Line::from(Span::styled(none, Style::new().fg(theme::dim()))));
    } else {
        let st = Style::new().fg(if t.status == ToolStatus::Err { RED } else { theme::text() });
        push_wrapped(&mut lines, &t.output, st, "");
    }
    let dh = detail.height as usize;
    let max = lines.len().saturating_sub(dh);
    app.tools_view_max.set(max);
    let top = scroll.min(max);
    let shown: Vec<Line> = lines.into_iter().skip(top).take(dh).collect();
    f.render_widget(Paragraph::new(shown), detail);
    if top < max {
        let tag = Span::styled(format!(" ↓ pgdn · {}% ", (top + dh) * 100 / (max + dh).max(1)), Style::new().fg(Color::Black).bg(theme::dim()));
        let r = Rect { y: detail.bottom().saturating_sub(1), height: 1, ..detail };
        f.render_widget(Paragraph::new(tag).alignment(Alignment::Right), r);
    }
}

/// Ctrl+G, pestaña de subagentes: la lista a la izquierda y, del elegido, todo lo que hizo en
/// orden (herramientas con su salida y lo que escribió) y al final lo que respondió.
pub(in crate::ui) fn agents_overlay(f: &mut Frame, app: &App) {
    use crate::{Paso, ToolStatus};
    let Some((sel, scroll)) = app.tools_view else { return };
    let a = f.area();
    let area = Rect { x: a.x + 2, y: a.y + 1, width: a.width.saturating_sub(4), height: a.height.saturating_sub(2) };
    limpiar(f, area);
    let block = panel("AGENTES · tab herramientas · ↑↓ elegir · pgup/pgdn recorrer · esc cierra");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let list_w = (inner.width / 3).clamp(28, 44);
    let [list, _, detail] =
        Layout::horizontal([Constraint::Length(list_w), Constraint::Length(2), Constraint::Min(20)]).areas(inner);

    // Los terminados primero y los vivos al final, como llegaron.
    let todos: Vec<&crate::Agent> = app.agents_done.iter().chain(app.agents.iter()).collect();
    let n = todos.len();
    let sel = sel.min(n.saturating_sub(1));
    let h = list.height as usize;
    let first = (sel + 1).saturating_sub(h);
    let rows: Vec<Line> = todos
        .iter()
        .enumerate()
        .skip(first)
        .take(h)
        .map(|(i, ag)| {
            let (icon, col) = match ag.ended {
                None => ("◐", ag.state().color()),
                Some((_, true)) => ("✓", GREEN),
                Some((_, false)) => ("✗", RED),
            };
            let secs = ag.ended.map_or_else(|| ag.started.elapsed(), |(t, _)| t.duration_since(ag.started)).as_secs_f64();
            let label = truncate(&ag.description, (list.width as usize).saturating_sub(10));
            let st = if i == sel { Style::new().fg(Color::Black).bg(theme::accent()) } else { Style::new().fg(theme::text()) };
            Line::from(vec![
                Span::styled(format!("{icon} "), if i == sel { st } else { Style::new().fg(col) }),
                Span::styled(format!("{label:<w$}", w = (list.width as usize).saturating_sub(9)), st),
                Span::styled(format!("{secs:>5.0}s"), if i == sel { st } else { Style::new().fg(theme::dim()) }),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(rows), list);

    let Some(ag) = todos.get(sel) else { return };
    let w = detail.width as usize;
    let mut lines: Vec<Line> = vec![];
    let head = |s: &str| Line::from(Span::styled(s.to_string(), Style::new().fg(theme::accent()).bold()));
    let push_wrapped = |lines: &mut Vec<Line>, text: &str, st: Style, prefix: &str| {
        for raw in text.lines() {
            let raw = raw.replace('\t', "    ");
            for piece in wrap(&raw, w.saturating_sub(prefix.chars().count()).max(10)) {
                lines.push(Line::from(Span::styled(format!("{prefix}{piece}"), st)));
            }
        }
    };
    let (estado, col) = match ag.ended {
        None => (ag.state().label().to_lowercase(), ag.state().color()),
        Some((_, true)) => ("terminó bien".to_string(), GREEN),
        Some((_, false)) => ("terminó con error".to_string(), RED),
    };
    let secs = ag.ended.map_or_else(|| ag.started.elapsed(), |(t, _)| t.duration_since(ag.started)).as_secs_f64();
    lines.push(Line::from(Span::styled(ag.description.clone(), Style::new().fg(theme::text()).bold())));
    lines.push(Line::from(vec![
        Span::styled(format!("{}  ·  ", if ag.kind.is_empty() { "agente" } else { &ag.kind }), Style::new().fg(theme::faint())),
        Span::styled(estado, Style::new().fg(col)),
        Span::styled(format!("  ·  {secs:.0}s  ·  {} herramientas", ag.tools), Style::new().fg(theme::faint())),
    ]));
    // Lo que se le pidió: la entrada de la llamada a Agent en el hilo principal.
    let llamada = app.activity.iter().find(|t| t.id == ag.tool);
    if let Some(prompt) = llamada.and_then(|t| t.input.get("prompt")).and_then(|v| v.as_str()) {
        lines.push(Line::default());
        lines.push(head("ENCARGO"));
        push_wrapped(&mut lines, prompt, Style::new().fg(theme::faint()), "");
    }
    lines.push(Line::default());
    lines.push(head("PASOS"));
    if ag.log.is_empty() {
        lines.push(Line::from(Span::styled("todavía no hizo nada…", Style::new().fg(theme::dim()))));
    }
    for paso in &ag.log {
        match paso {
            Paso::Texto(t) => {
                lines.push(Line::default());
                push_wrapped(&mut lines, t.trim(), Style::new().fg(theme::text()), "");
            }
            Paso::Herramienta(t) => {
                let (icon, c) = match t.status {
                    ToolStatus::Running => ("◐", WARM),
                    ToolStatus::Ok => ("✓", GREEN),
                    ToolStatus::Err => ("✗", RED),
                };
                let secs = t.took.unwrap_or_else(|| t.started.elapsed()).as_secs_f64();
                lines.push(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::new().fg(c)),
                    Span::styled(t.name.clone(), Style::new().fg(theme::text()).bold()),
                    Span::styled(format!(" {}", truncate(&t.detail, w.saturating_sub(t.name.len() + 12))), Style::new().fg(theme::faint())),
                    Span::styled(format!("  {secs:.1}s"), Style::new().fg(theme::dim())),
                ]));
                // De la salida, las primeras líneas: lo entero está en la pestaña de herramientas
                // de un agente cuando se abra, aquí basta para seguirle el hilo.
                let out: Vec<&str> = t.output.lines().filter(|l| !l.trim().is_empty()).collect();
                let st = Style::new().fg(if t.status == ToolStatus::Err { RED } else { theme::dim() });
                for l in out.iter().take(4) {
                    lines.push(Line::from(Span::styled(format!("  │ {}", truncate(l, w.saturating_sub(4))), st)));
                }
                if out.len() > 4 {
                    lines.push(Line::from(Span::styled(format!("  │ … {} líneas más", out.len() - 4), Style::new().fg(theme::dim()))));
                }
            }
        }
    }
    if let Some(t) = llamada.filter(|t| t.status != ToolStatus::Running && !t.output.trim().is_empty()) {
        lines.push(Line::default());
        lines.push(head("RESPUESTA"));
        push_wrapped(&mut lines, &t.output, Style::new().fg(theme::text()), "");
    }
    let dh = detail.height as usize;
    let max = lines.len().saturating_sub(dh);
    // Abre en el final (scroll = MAX) y lo sigue mientras el agente trabaja.
    app.tools_view_max.set(max);
    let top = scroll.min(max);
    let shown: Vec<Line> = lines.into_iter().skip(top).take(dh).collect();
    f.render_widget(Paragraph::new(shown), detail);
    if top < max {
        let tag = Span::styled(format!(" ↓ pgdn · {}% ", (top + dh) * 100 / (max + dh).max(1)), Style::new().fg(Color::Black).bg(theme::dim()));
        let r = Rect { y: detail.bottom().saturating_sub(1), height: 1, ..detail };
        f.render_widget(Paragraph::new(tag).alignment(Alignment::Right), r);
    }
}

/// ^T: la conversación entera con su markdown en `area`, la orden debajo y cómo volver.
pub(in crate::ui) fn transcript_view(f: &mut Frame, app: &App, area: Rect) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let body = Rect {
        x: area.x + 2,
        y: area.y,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(6),
    };
    draw_chat(f, body, app);
    orden_y_teclas(f, app, area, body.bottom() + 1, state, t, "^T", &[("pgup/pgdn", "recorre"), ("arrastra", "para copiar")]);
}

/// ^O fuera de Cine: la última respuesta (o la que se eligió con pgup) entera, desplazable.
/// `false` si todavía no hay ninguna que leer.
pub(in crate::ui) fn lectura(f: &mut Frame, app: &App) -> bool {
    let said: Vec<(usize, &crate::Msg)> = app
        .messages
        .iter()
        .enumerate()
        .filter(|(_, m)| matches!(m.role, Role::User | Role::Assistant) && m.waiting.is_none())
        .collect();
    let skip = (app.scroll / 5).min(said.len().saturating_sub(1));
    let Some((i, m)) = said.iter().rev().skip(skip).find(|(_, m)| m.role == Role::Assistant).copied() else {
        return false;
    };
    app.reading.set(true);
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let a = f.area();
    f.render_widget(Clear, a);
    let w = a.width.saturating_sub(6).min(110);
    let x = a.x + (a.width - w) / 2;
    put(f.buffer_mut(), x, a.y + 1, &spaced("RESPUESTA"), Style::new().fg(theme::accent()).bold(), w);
    let panel = Rect { x, y: a.y + 2, width: w, height: a.height.saturating_sub(8) };
    let files = app.adjuntos(i, &m.text);
    if files.is_empty() {
        read_panel(f, panel, app, i, &m.text);
    } else {
        read_panel(f, Rect { height: panel.height.saturating_sub(1), ..panel }, app, i, &m.text);
        chips(f, Rect { y: panel.bottom().saturating_sub(1), height: 1, ..panel }, app, &files);
    }
    orden_y_teclas(f, app, a, panel.bottom() + 1, state, t, "^O", &[("↑↓ pgup/pgdn", "recorre"), ("arrastra", "para copiar")]);
    if let Some(sel) = &app.sel {
        let view = app.view.borrow();
        let buf = f.buffer_mut();
        for (x, y) in sel.cells(&view) {
            buf[(x, y)].set_bg(theme::dim()).set_fg(Color::White);
        }
    }
    true
}

/// Debajo de ^T y ^O: la orden, y en la fila de abajo cómo volver al estilo (o el aviso).
pub(in crate::ui) fn orden_y_teclas(f: &mut Frame, app: &App, area: Rect, y: u16, state: State, t: f64, tecla: &str, mas: &[(&str, &str)]) {
    let line = Rect { x: area.x + 4, y, width: area.width.saturating_sub(8), height: 1 };
    let mut spans = vec![Span::styled("› ", Style::new().fg(theme::accent()).bold())];
    spans.extend(input_spans(app, state, t, line.width.saturating_sub(2) as usize, theme::accent()));
    f.render_widget(Paragraph::new(Line::from(spans)), line);
    let hint = flash(app).unwrap_or_else(|| {
        let volver = format!(" vuelve a {}   ", app.estilo.nombre());
        let mut v = vec![
            Span::styled(tecla.to_string(), Style::new().fg(theme::accent())),
            Span::styled(" o ", Style::new().fg(theme::faint())),
            Span::styled("esc", Style::new().fg(theme::accent())),
            Span::styled(volver, Style::new().fg(theme::faint())),
        ];
        for (k, d) in mas {
            v.push(Span::styled(k.to_string(), Style::new().fg(theme::accent())));
            v.push(Span::styled(format!(" {d}   "), Style::new().fg(theme::faint())));
        }
        Line::from(v)
    });
    let hr = Rect { y: line.y + 1, ..line };
    f.render_widget(Paragraph::new(hint).alignment(Alignment::Center), hr);
}

/// Los archivos que entregó Claude en una fila, centrados: un clic copia cada uno.
pub(in crate::ui) fn chips(f: &mut Frame, r: Rect, app: &App, files: &[crate::adjunto::Adjunto]) {
    let max = (r.width as usize / files.len().max(1)).saturating_sub(2).max(8);
    let labels: Vec<String> = files.iter().map(|a| truncate(&format!(" ⎘ {} ", a.label()), max)).collect();
    let total: usize = labels.iter().map(|l| l.width() + 2).sum::<usize>().saturating_sub(2);
    let mut x = r.x + (r.width as usize).saturating_sub(total) as u16 / 2;
    let buf = f.buffer_mut();
    for (a, l) in files.iter().zip(labels) {
        let w = (l.width() as u16).min(r.right().saturating_sub(x));
        if w == 0 {
            break;
        }
        put(buf, x, r.y, &l, Style::new().fg(Color::Black).bg(theme::accent()).bold(), w);
        app.file_targets.borrow_mut().push((Rect { x, y: r.y, width: w, height: 1 }, a.clone()));
        x += w + 2;
    }
}

/// Cine leyendo: la respuesta entera con su markdown, desde el principio, desplazable.
pub(in crate::ui) fn read_panel(f: &mut Frame, r: Rect, app: &App, i: usize, text: &str) {
    let rows = app.md_rows(i, text, r.width.saturating_sub(2) as usize);
    let h = r.height.saturating_sub(1) as usize;
    let max_top = rows.len().saturating_sub(h);
    // Otro mensaje: se lee desde arriba. El mismo creciendo (o desplazado): donde estaba.
    if app.read_msg.get() != i {
        app.read_msg.set(i);
        app.read_top.set(0);
    }
    let top = app.read_top.get().min(max_top);
    app.read_top.set(top);
    let body = Rect { y: r.y + 1, height: h as u16, ..r };
    let visible: Vec<Line> = rows.iter().skip(top).take(h).map(|(l, _, _)| l.clone()).collect();
    f.render_widget(Paragraph::new(visible), body);

    let buf = f.buffer_mut();
    let dim = Style::new().fg(theme::dim());
    put(buf, r.x, r.y, &"─".repeat(r.width as usize), dim, r.width);
    if top > 0 {
        put_right(buf, r.right().saturating_sub(1), r.y, " ↑ pgup ", Style::new().fg(theme::accent()));
    }
    if top < max_top {
        let pct = (top + h) * 100 / rows.len().max(1);
        put_right(buf, r.right().saturating_sub(1), r.bottom().saturating_sub(1), &format!(" ↓ pgdn · {pct}% "), Style::new().fg(theme::accent()));
    }

    // Para seleccionar y copiar con el mouse, como en la conversación del Clásico.
    let mut view = crate::select::View { area: body, rows: Vec::with_capacity(rows.len()), first: top };
    for (line, skip, cont) in &rows {
        let text = line.spans.iter().map(|s| s.content.as_ref()).collect();
        view.rows.push(crate::select::Row { text, skip: *skip, cont: *cont });
    }
    *app.view.borrow_mut() = view;
}
