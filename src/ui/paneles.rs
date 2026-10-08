//! Piezas con datos de Iris que comparten los estilos: núcleo, actividad, onda de voz, orden,
//! pie y línea de tiempo.

use super::*;

pub(in crate::ui) fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme::dim()))
        .title(Span::styled(format!(" {title} "), Style::new().fg(theme::accent()).bold()))
}

pub(in crate::ui) fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let model = app.model.trim_start_matches("claude-");
    let sess = app.session.get(..8).unwrap_or("········");
    let left = Line::from(vec![
        Span::styled(" ◆ I.R.I.S ", Style::new().fg(Color::Black).bg(theme::accent()).bold()),
        Span::styled(format!("  {model}"), Style::new().fg(theme::text())),
        Span::styled(format!("  ·  {}  ·  sesión {sess}", cwd()), Style::new().fg(theme::faint())),
    ]);
    f.render_widget(Paragraph::new(left), area);
    let clock = Line::from(Span::styled(format!("{} ", now_hhmmss()), Style::new().fg(theme::accent())));
    f.render_widget(Paragraph::new(clock).alignment(Alignment::Right), area);
}

pub(in crate::ui) fn draw_core(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("NÚCLEO");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    let [ring, label] = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(inner);
    core_draw(app, ring, f.buffer_mut());
    let (title, detail) = core_label(app, t, label.width as usize);
    let lines = vec![
        Line::from(Span::styled(title, Style::new().fg(app.nucleo.color()).bold())),
        Line::from(Span::styled(detail, Style::new().fg(theme::faint()))),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), label);
}

/// El rótulo del núcleo: el estado que se ve (no el pedido, así no se adelanta a la
/// animación) y un detalle de lo que pasa.
pub(in crate::ui) fn core_label(app: &App, t: f64, width: usize) -> (String, String) {
    let state = app.nucleo.state();
    let detail = match state {
        State::Planning if app.todos.0 > 0 => format!("{}/{} tareas", app.todos.1, app.todos.0),
        s if app.current_tool().is_some() && matches!(s,
            State::Searching | State::Reading | State::Editing | State::Running | State::Testing
            | State::Git | State::Web | State::Delegating | State::Planning) =>
        {
            let a = app.current_tool().unwrap();
            let room = width.saturating_sub(a.name.len() + 10);
            let what = truncate(&a.detail, room);
            let secs = a.started.elapsed().as_secs_f64();
            if what.is_empty() { format!("{} · {secs:.0}s", a.name) } else { format!("{} · {what} · {secs:.0}s", a.name) }
        }
        State::Listening if app.ptt.is_some() => "suelta espacio para enviar · esc cancela".into(),
        State::Listening => "espacio para enviar · esc cancela".into(),
        State::NoVoice => "no llega tu voz · ¿el micrófono correcto?".into(),
        State::Asking => "elige con ↑↓ y enter".into(),
        State::Compacting => "resumiendo la conversación".into(),
        State::Sleeping => "toca una tecla o espacio para hablar".into(),
        State::Booting | State::Idle if app.voice == VoiceState::Loading => "cargando voz…".into(),
        State::Idle if app.voice == VoiceState::Ready => "mantén o toca espacio para hablar".into(),
        _ => String::new(),
    };
    let dots = if matches!(state, State::Idle | State::Offline | State::Sleeping) {
        ""
    } else {
        ["   ", ".  ", ".. ", "..."][(t * 3.0) as usize % 4]
    };
    (format!("{}{dots}", state.label()), detail)
}

pub(in crate::ui) fn act_icon(a: &Activity, t: f64) -> (&'static str, Color) {
    match a.status {
        ToolStatus::Running => (SPIN[(t * 8.0) as usize % 4], WARM),
        ToolStatus::Ok => ("✓", GREEN),
        ToolStatus::Err => ("✗", RED),
    }
}

pub(in crate::ui) fn act_secs(a: &Activity) -> f64 {
    a.took.unwrap_or_else(|| a.started.elapsed()).as_secs_f64()
}

pub(in crate::ui) fn draw_activity(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("ACTIVIDAD");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width as usize;

    // Arriba, los subagentes: qué es cada hijo del núcleo y qué está haciendo.
    let mut lines: Vec<Line> = app
        .agents
        .iter()
        .map(|g| {
            let (icon, c) = match g.ended {
                None => ("◉", app.nucleo.color()),
                Some((_, true)) => ("✓", GREEN),
                Some((_, false)) => ("✗", RED),
            };
            let tool = g.current.as_ref().map(|(n, d)| (n, d)).or(g.last.as_ref().map(|(n, d, _)| (n, d)).filter(|_| g.state() != State::Thinking));
            let what = match (&g.ended, tool) {
                (Some(_), _) => String::new(),
                (None, Some((name, detail))) => format!("{} · {name} {detail}", g.state().label().to_lowercase()),
                (None, None) => g.state().label().to_lowercase(),
            };
            let time = format!(" {:.0}s", g.ended.map_or(g.started.elapsed(), |(t, _)| t - g.started).as_secs_f64());
            let desc = format!("{} ", truncate(&g.description, w.saturating_sub(time.len() + 6)));
            let room = w.saturating_sub(desc.chars().count() + 2 + time.len());
            Line::from(vec![
                Span::styled(format!("{icon} "), Style::new().fg(c)),
                Span::styled(desc, Style::new().fg(theme::text()).bold()),
                Span::styled(format!("{:<room$}", truncate(&what, room)), Style::new().fg(theme::faint())),
                Span::styled(time, Style::new().fg(theme::dim())),
            ])
        })
        .collect();
    let room = (inner.height as usize).saturating_sub(lines.len());

    lines.extend(app
        .activity
        .iter()
        .rev()
        .take(room)
        .map(|a| {
            let (icon, c) = act_icon(a, t);
            let time = format!(" {:.1}s", act_secs(a));
            let head = format!("{icon} {} ", a.name);
            let room = w.saturating_sub(head.chars().count() + time.len());
            Line::from(vec![
                Span::styled(format!("{icon} "), Style::new().fg(c)),
                Span::styled(format!("{} ", a.name), Style::new().fg(theme::text()).bold()),
                Span::styled(format!("{:<room$}", truncate(&a.detail, room)), Style::new().fg(theme::faint())),
                Span::styled(time, Style::new().fg(theme::dim())),
            ])
        }));

    if lines.is_empty() {
        let p = Paragraph::new(Span::styled("sin actividad todavía", Style::new().fg(theme::dim())));
        f.render_widget(p, inner);
    } else {
        f.render_widget(Paragraph::new(lines), inner);
    }
}

pub(in crate::ui) fn draw_wave(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("ESCUCHANDO").border_style(Style::new().fg(WARM));
    let inner = block.inner(area);
    f.render_widget(block, area);
    wave(f, inner, app, t, WARM, Color::Rgb(200, 120, 30));
}

/// Onda espejada: cada columna es un instante; la altura sale del RMS.
pub(in crate::ui) fn wave(f: &mut Frame, inner: Rect, app: &App, t: f64, mid_c: Color, edge_c: Color) {
    let w = inner.width as usize;
    let h = inner.height as usize;
    let hist = &app.levels[app.levels.len().saturating_sub(w)..];
    let bars = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mid = h as f64 / 2.0;
    let rows: Vec<Line> = (0..h)
        .map(|row| {
            let spans: Vec<Span> = hist
                .iter()
                .enumerate()
                .map(|(i, &l)| {
                    let shimmer = 0.015 * ((i as f64 * 0.35 + t * 6.0).sin() + 1.0);
                    let amp = ((l as f64 * 5.0).sqrt().min(1.0) + shimmer) * mid;
                    let dist = (row as f64 + 0.5 - mid).abs();
                    let fill = ((amp - dist + 0.5) * 8.0).clamp(0.0, 8.0) as usize;
                    let c = if dist < 0.6 { mid_c } else { edge_c };
                    Span::styled(bars[fill].to_string(), Style::new().fg(c))
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(rows), inner);
}

pub(in crate::ui) fn draw_input(f: &mut Frame, area: Rect, app: &App, state: State, t: f64) {
    let border = if app.busy { theme::dim() } else { theme::accent() };
    let block = panel("ORDEN").border_style(Style::new().fg(border));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut spans = vec![Span::styled(" › ", Style::new().fg(theme::accent()).bold())];
    spans.extend(input_spans(app, state, t, (inner.width as usize).saturating_sub(4), theme::accent()));
    f.render_widget(Paragraph::new(Line::from(spans)), inner);
}

/// Lo escrito (o la pista, si no hay nada) con el cursor, en `w` columnas.
pub(in crate::ui) fn input_spans(app: &App, state: State, t: f64, w: usize, accent: Color) -> Vec<Span<'static>> {
    let cursor = if (t * 2.0) as usize % 2 == 0 { "▏" } else { " " };
    let mut spans = vec![];
    // Las imágenes adjuntas van como fichas delante del texto.
    let mut used = 0;
    for (i, img) in app.images.iter().enumerate() {
        let chip = format!(" ▣ {} ", img.label(i + 1));
        used += chip.chars().count() + 1;
        spans.push(Span::styled(chip, Style::new().fg(Color::Black).bg(accent)));
        spans.push(Span::raw(" "));
    }
    let w = w.saturating_sub(used + 1).max(4);
    // Los saltos de línea se ven como «↵»; se envían tal cual. La ventana de texto se corre
    // para que el cursor siempre quede a la vista.
    let chars: Vec<char> = app.input.chars().map(|c| if c == '\n' { '↵' } else { c }).collect();
    let cur = app.cur.min(chars.len());
    let start = if chars.len() < w { 0 } else { (cur + 1).saturating_sub(w).min(chars.len() + 1 - w) };
    let end = (start + w).min(chars.len());
    let before: String = chars[start..cur].iter().collect();
    let at: Option<char> = chars.get(cur).copied().filter(|_| cur < end);
    let after: String = chars.get(cur + 1..end).map(|c| c.iter().collect()).unwrap_or_default();
    if app.input.is_empty() && !app.images.is_empty() {
        spans.push(Span::styled(cursor, Style::new().fg(accent)));
        spans.push(Span::styled("escribe algo para acompañarla, o enter · ⌫ quita", Style::new().fg(theme::dim())));
    } else if app.input.is_empty() {
        spans.push(Span::styled(cursor, Style::new().fg(accent)));
        let hint = match (state, &app.voice) {
            (State::Transcribing, _) => "transcribiendo…",
            (State::Asking, _) => "elige arriba, o escribe aquí otra respuesta",
            (_, _) if app.busy => "trabajando — esc para interrumpir",
            (_, VoiceState::Ready) => "escribe, espacio para hablar, ^V pega una imagen",
            _ => "escribe una orden",
        };
        spans.push(Span::styled(hint, Style::new().fg(theme::dim())));
    } else {
        spans.push(Span::styled(before, Style::new().fg(theme::text())));
        match at {
            // En medio del texto: un bloque sobre la letra, fijo (si parpadeara, la letra se perdería).
            Some(c) => spans.push(Span::styled(c.to_string(), Style::new().fg(Color::Black).bg(accent))),
            None => spans.push(Span::styled(cursor, Style::new().fg(accent))),
        }
        spans.push(Span::styled(after, Style::new().fg(theme::text())));
    }
    spans
}

pub(in crate::ui) fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let k = |s: &'static str| Span::styled(s, Style::new().fg(theme::accent()));
    let d = |s: &'static str| Span::styled(s, Style::new().fg(theme::faint()));
    let hints = Line::from(vec![
        k(" espacio"), d(" hablar  "),
        k("enter"), d(" enviar  "),
        k("esc"), d(" cancelar  "),
        k("pgup/pgdn"), d(" desplazar  "),
        k("^L"), d(" limpiar  "),
        k("^R"), d(" reiniciar  "),
        k("^C"), d(" salir"),
    ]);
    let hints = flash(app).unwrap_or(hints);
    let stats = format!("   {} ", stats(app));
    let sw = stats.chars().count() as u16;
    let [left, right] = Layout::horizontal([Constraint::Min(0), Constraint::Length(sw)]).areas(area);
    f.render_widget(Paragraph::new(hints), left);
    f.render_widget(Paragraph::new(Span::styled(stats, Style::new().fg(theme::dim()))), right);
}

/// La línea del turno: un bloque por herramienta, de ancho proporcional a lo que tardó.
pub(in crate::ui) fn timeline(buf: &mut Buffer, r: Rect, acts: &[Activity], with_secs: bool) {
    if r.width < 6 || r.height == 0 {
        return;
    }
    if acts.is_empty() {
        put(buf, r.x, r.y, &"░".repeat(r.width as usize), Style::new().fg(theme::dim()), r.width);
        if with_secs && r.height > 1 {
            put(buf, r.x, r.y + 1, "sin turno en curso", Style::new().fg(theme::faint()), r.width);
        }
        return;
    }
    // Lo que no cabe se cae por la izquierda: las últimas importan más.
    let room = r.width as usize;
    let mut take = acts.len();
    while take > 1 && take * 3 > room {
        take -= 1;
    }
    let acts = &acts[acts.len() - take..];
    let running = acts.last().is_some_and(|a| a.status == ToolStatus::Running);
    let room = room.saturating_sub(acts.len() + running as usize);
    let mins: Vec<usize> = acts.iter().map(|a| (a.name.chars().count() + 2).min(8).max(3)).collect();
    let tot: f64 = acts.iter().map(|a| act_secs(a).max(0.05)).sum();
    let mut ws: Vec<usize> = acts
        .iter()
        .zip(&mins)
        .map(|(a, m)| ((room as f64 * act_secs(a).max(0.05) / tot).round() as usize).max(*m))
        .collect();
    while ws.iter().sum::<usize>() > room {
        let Some(k) = (0..ws.len()).filter(|&j| ws[j] > 3).max_by_key(|&j| ws[j] - mins[j].min(ws[j])) else { break };
        ws[k] -= 1;
    }
    let mut x = r.x;
    for (a, w) in acts.iter().zip(ws) {
        let c = match a.status {
            ToolStatus::Running => theme::accent(),
            ToolStatus::Ok => theme::faint(),
            ToolStatus::Err => RED,
        };
        let label = format!(" {:<w$}", truncate(&a.name.to_uppercase(), w.saturating_sub(1)), w = w.saturating_sub(1));
        put(buf, x, r.y, &label, Style::new().fg(Color::Black).bg(c).bold(), w as u16);
        if with_secs && r.height > 1 {
            put(buf, x, r.y + 1, &format!("{:.1}s", act_secs(a)), Style::new().fg(theme::dim()), w as u16);
        }
        x += w as u16 + 1;
    }
    if running {
        put(buf, x.saturating_sub(1), r.y, "▶", Style::new().fg(theme::accent()).bold(), 1);
    }
}

/// Atajos `tecla desc · tecla desc` desde `x`.
pub(in crate::ui) fn keys(buf: &mut Buffer, x: u16, y: u16, max_x: u16, items: &[(&str, &str)], kc: Color, brackets: bool) {
    let mut x = x;
    for (k, d) in items {
        let k = if brackets { format!("[{k}]") } else { (*k).to_string() };
        x = put(buf, x, y, &k, Style::new().fg(kc), max_x.saturating_sub(x));
        x = put(buf, x + 1, y, d, Style::new().fg(theme::faint()), max_x.saturating_sub(x + 1)) + 3;
        if x >= max_x {
            break;
        }
    }
}

/// El núcleo: como imagen de puntos (Sixel) si se puede, y si no en braille. En modo imagen
/// el texto deja el panel vacío y se anota dónde va; la app manda la imagen después del cuadro.
/// Con un menú abierto va en braille: la imagen se pintaría encima del menú.
pub(in crate::ui) fn core_draw(app: &App, r: Rect, buf: &mut Buffer) {
    if app.sixel_cell.is_some() && app.modal.is_none() && app.tools_view.is_none() && r.width >= 6 && r.height >= 3 {
        app.core_rect.set(Some(r));
    } else {
        app.nucleo.draw(r, buf);
    }
}

/// La línea de escritura con su prompt, y lo que se despliega encima (preguntas, menús).
/// `anchor` es la fila desde la que suben los menús.
pub(in crate::ui) fn orden_line(f: &mut Frame, line: Rect, anchor: Rect, app: &App, state: State, t: f64, prompt: &str, col: Color) {
    if line.width < 4 || line.height == 0 {
        return;
    }
    let pw = prompt.width();
    let mut spans = vec![Span::styled(prompt.to_string(), Style::new().fg(col).bold())];
    spans.extend(input_spans(app, state, t, (line.width as usize).saturating_sub(pw + 1), col));
    f.render_widget(Paragraph::new(Line::from(spans)), Rect { height: 1, ..line });
    draw_modal(f, anchor, app);
}

/// El aviso breve si hay uno; si no, los atajos.
pub(in crate::ui) fn foot_keys(buf: &mut Buffer, r: Rect, app: &App, state: State, kc: Color, brackets: bool) {
    match flash(app) {
        Some(l) => {
            buf.set_line(r.x, r.y, &l, r.width);
        }
        None => keys(buf, r.x, r.y, r.right(), &hints_for(app, state), kc, brackets),
    }
}
