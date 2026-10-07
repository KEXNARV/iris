//! El estilo Bitácora de /theme.

use crate::ui::*;

pub(in crate::ui) fn bitacora(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let inner = Rect { x: area.x + 2, width: area.width.saturating_sub(4), ..area };
    let input_h = if listening(state) { 3 } else { 1 };
    let [head, rule, cols, _, body, rule2, input, _, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(input_h),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x, head.y, "JARVIS", Style::new().fg(theme::accent()).bold(), 6);
        put(buf, x + 1, head.y, "bitácora", Style::new().fg(theme::faint()), 10);
        // El estado como una traza: el nivel de la voz al escuchar, el pulso del núcleo si no.
        let clock = now_hhmmss();
        let n = 30usize;
        let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let trace: String = (0..n)
            .map(|i| {
                let v = if listening(state) {
                    let l = app.levels[app.levels.len().saturating_sub(n) + i.min(n - 1)] as f64;
                    (l * 5.0).sqrt().min(1.0)
                } else {
                    let speed = if app.busy { 3.0 } else { 0.8 };
                    0.5 + 0.45 * ((i as f64 * 0.45) - t * speed).sin() * if app.busy { 1.0 } else { 0.4 }
                };
                bars[((v * 7.0).round() as usize).min(7)]
            })
            .collect();
        put_right(buf, head.right() - 1, head.y, &clock, Style::new().fg(theme::text()));
        let tx = head.right().saturating_sub(clock.len() as u16 + 2 + n as u16);
        put(buf, tx, head.y, &trace, Style::new().fg(tone), n as u16);
        put_right(buf, tx.saturating_sub(2), head.y, "estado", Style::new().fg(theme::faint()));
        hline(buf, rule.x, rule.right() - 1, rule.y, "─", Style::new().fg(theme::dim()));
        let h = Style::new().fg(theme::faint()).bold();
        put(buf, cols.x, cols.y, "HORA", h, 8);
        put(buf, cols.x + 10, cols.y, "TIPO", h, 8);
        put(buf, cols.x + 18, cols.y, "CONTENIDO", h, 20);
        put_right(buf, cols.right() - 1, cols.y, "DURACIÓN", h);
    }

    // Cada mensaje y cada herramienta es una entrada; lo largo sigue debajo, sangrado.
    let w = body.width as usize;
    let cw = w.saturating_sub(18 + 10).max(10);
    let mut rows: Vec<(Line<'static>, usize, bool)> = Vec::new();
    let entry = |rows: &mut Vec<(Line<'static>, usize, bool)>, time: String, tipo: &str, tc: Color, text: &str, fg: Color, dur: Option<(String, Color)>| {
        let lines = wrap_cont(text, cw);
        for (k, (l, cont)) in lines.into_iter().enumerate() {
            let mut spans = vec![
                Span::styled(format!("{:<10}", if k == 0 { time.as_str() } else { "" }), Style::new().fg(theme::dim())),
                Span::styled(format!("{:<8}", if k == 0 { tipo } else { "" }), Style::new().fg(tc).bold()),
                Span::styled(format!("{:<cw$}", l), Style::new().fg(fg)),
            ];
            if k == 0 {
                if let Some((d, c)) = &dur {
                    spans.push(Span::styled(format!("{d:>10}"), Style::new().fg(*c)));
                }
            }
            rows.push((Line::from(spans), 18, cont));
        }
    };
    entry(
        &mut rows,
        clock_ago(app.started.elapsed().as_secs()),
        "SES",
        theme::faint(),
        &format!("sesión {} · {} · {}", ses8(app), model_short(app), cwd()),
        theme::faint(),
        None,
    );
    for m in &app.messages {
        match m.role {
            Role::User if m.waiting.is_some() => entry(&mut rows, String::new(), "COLA", theme::faint(), &m.text, theme::faint(), None),
            Role::User => entry(&mut rows, String::new(), "TÚ", theme::text(), &m.text, theme::text(), None),
            Role::Assistant => entry(&mut rows, String::new(), "JARVIS", theme::accent(), &plain(&m.text), theme::text(), None),
            Role::System => entry(&mut rows, String::new(), "SYS", theme::faint(), &m.text, theme::faint(), None),
            Role::Error => entry(&mut rows, String::new(), "ERROR", RED, &m.text, RED, None),
            Role::Tool => {
                let Some(a) = m.tool.and_then(|i| app.activity.get(i)) else { continue };
                let (icon, c) = act_icon(a, t);
                let dur = Some((format!("{icon} {:.1}s", act_secs(a)), c));
                let name = truncate(&a.name.to_uppercase(), 7);
                entry(&mut rows, clock_ago(a.started.elapsed().as_secs()), &name, theme::text(), &a.detail, theme::faint(), dur);
            }
        }
    }
    if app.busy && app.current_tool().is_none() {
        let dots = ".".repeat((t * 3.0) as usize % 4 + 1);
        entry(&mut rows, String::new(), "PIENSA", tone, &dots, tone, None);
    }
    render_chat(f, body, app, rows);

    {
        let buf = f.buffer_mut();
        hline(buf, rule2.x, rule2.right() - 1, rule2.y, "─", Style::new().fg(theme::dim()));
    }
    if listening(state) {
        wave(f, input, app, t, tone, theme::dim());
    } else {
        orden_line(f, input, rule2, app, state, t, "▶ ", theme::accent());
    }
    let kc = if listening(state) { tone } else { theme::accent() };
    foot_keys(f.buffer_mut(), hints, app, state, kc, false);
}
