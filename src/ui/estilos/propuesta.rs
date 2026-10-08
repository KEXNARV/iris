//! El estilo Propuesta de /theme.

use crate::ui::*;

pub(in crate::ui) fn propuesta(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let m = if area.width > 90 { 3 } else { 1 };
    let inner = Rect { x: area.x + m, width: area.width.saturating_sub(m * 2), ..area };
    let tone = app.nucleo.color();

    let input_h = if listening(state) { 4 } else { 2 };
    let [head, rule, body, input, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Cabecera: marca y modelo a la izquierda; carpeta, sesión y hora a la derecha.
    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x, head.y, "◆ ", Style::new().fg(theme::accent()).bold(), head.width);
        let x = put(buf, x, head.y, &spaced("IRIS"), Style::new().fg(theme::accent()).bold(), head.width);
        put(buf, x + 3, head.y, app.model.trim_start_matches("claude-"), Style::new().fg(theme::text()), head.width / 3);
        let sess = app.session.get(..8).unwrap_or("········");
        let clock = now_hhmmss();
        put_right(buf, head.right() - 1, head.y, &clock, Style::new().fg(theme::text()));
        put_right(buf, head.right() - 1 - clock.len() as u16 - 3, head.y, &format!("{}   {sess}", cwd()), Style::new().fg(theme::faint()));
        hline(buf, rule.x, rule.right() - 1, rule.y, "─", Style::new().fg(theme::dim()));
    }

    let side_w = (body.width * 36 / 100).clamp(30, 56);
    let narrow = body.width < 80;
    let (chat, side) = if narrow {
        (body, Rect::default())
    } else {
        let [c, _, s] = Layout::horizontal([Constraint::Min(30), Constraint::Length(4), Constraint::Length(side_w)]).areas(body);
        (c, s)
    };

    // La conversación como guion.
    let chat_in = Rect { y: chat.y + 1, height: chat.height.saturating_sub(1), ..chat };
    if app.messages.is_empty() {
        let buf = f.buffer_mut();
        let y = chat_in.y + chat_in.height / 3;
        put(buf, chat_in.x + GUION as u16, y, &format!("{}.", greeting()), Style::new().fg(theme::text()).bold(), chat_in.width);
        put(buf, chat_in.x + GUION as u16, y + 1, "¿En qué trabajamos?", Style::new().fg(theme::accent()).bold(), chat_in.width);
        put(buf, chat_in.x + GUION as u16, y + 3, "escribe, o mantén espacio y háblame", Style::new().fg(theme::faint()), chat_in.width);
        *app.view.borrow_mut() = Default::default();
    } else {
        let rows = chat_rows(app, chat_in.width as usize, Voz::Guion);
        render_chat(f, chat_in, app, rows);
    }

    // El núcleo, su estado y la telemetría.
    if !narrow {
        let core_h = (side.width / 2).min(side.height.saturating_sub(12)).max(6);
        let [core, label, _, tele, _, now] = Layout::vertical([
            Constraint::Length(core_h),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(side);
        core_draw(app, core, f.buffer_mut());
        let (title, detail) = core_label(app, t, label.width as usize);
        let buf = f.buffer_mut();
        put_center(buf, label, label.y, &spaced_title(&title), Style::new().fg(tone).bold());
        put_center(buf, label, label.y + 1, &detail, Style::new().fg(theme::faint()));
        let kx = tele.x + 1;
        let vx = tele.x + 13;
        let vw = tele.right().saturating_sub(vx);
        let k = Style::new().fg(theme::faint());
        let v = Style::new().fg(theme::text());
        put(buf, kx, tele.y, "contexto", k, 12);
        let x = meter(buf, vx, tele.y, vw.saturating_sub(6).min(20), ctx_pct(app) / 100.0, tone);
        put(buf, x + 1, tele.y, &format!("{:.0}%", ctx_pct(app)), v, 5);
        put(buf, kx, tele.y + 1, "sesión", k, 12);
        put(buf, vx, tele.y + 1, &format!("{} · ${:.2}", turns(app), app.cost), v, vw);
        put(buf, kx, tele.y + 2, "voz", k, 12);
        put(buf, vx, tele.y + 2, &voice_value(app), v, vw);
        put(buf, kx, tele.y + 3, "en línea", k, 12);
        put(buf, vx, tele.y + 3, &uptime(app), v, vw);
        // Lo último que hizo, lo más nuevo arriba.
        for (i, a) in app.activity.iter().rev().take(now.height as usize).enumerate() {
            let y = now.y + i as u16;
            let (icon, c) = act_icon(a, t);
            put(buf, kx, y, icon, Style::new().fg(c), 1);
            let x = put(buf, kx + 2, y, &a.name, Style::new().fg(theme::text()).bold(), 12);
            let secs = format!("{:.1}s", act_secs(a));
            put(buf, x + 1, y, &a.detail, Style::new().fg(theme::faint()), now.right().saturating_sub(x + 2 + secs.len() as u16));
            put_right(buf, now.right() - 1, y, &secs, Style::new().fg(theme::dim()));
        }
    }

    // La orden: una regla del color del estado y la línea de escritura.
    let input_w = Rect { width: chat.width, ..input };
    {
        let col = if listening(state) { tone } else if app.busy { theme::dim() } else { theme::accent() };
        let buf = f.buffer_mut();
        hline(buf, input_w.x, input_w.right() - 1, input_w.y, "─", Style::new().fg(col));
    }
    let line = Rect { y: input_w.y + 1, height: input_w.height - 1, ..input_w };
    if listening(state) {
        wave(f, line, app, t, tone, theme::dim());
    } else {
        let mut spans = vec![Span::styled("› ", Style::new().fg(theme::accent()).bold())];
        spans.extend(input_spans(app, state, t, line.width as usize - 2, theme::accent()));
        f.render_widget(Paragraph::new(Line::from(spans)), line);
        draw_modal(f, Rect { y: input_w.y, height: 1, ..input_w }, app);
    }

    let buf = f.buffer_mut();
    match flash(app) {
        Some(l) => {
            buf.set_line(hints.x, hints.y, &l, hints.width);
        }
        None => keys(buf, hints.x, hints.y, hints.right(), &hints_for(app, state), if listening(state) { tone } else { theme::accent() }, false),
    }
}
