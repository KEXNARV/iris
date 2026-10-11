//! The Cabina style of /theme.

use crate::ui::*;

pub(in crate::ui) fn cabina(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.core.color();
    let [bar, body, foot] = Layout::vertical([Constraint::Length(1), Constraint::Min(8), Constraint::Length(1)]).areas(area);

    // Segmented status bar.
    {
        let buf = f.buffer_mut();
        let sep = Style::new().fg(theme::dim());
        let k = Style::new().fg(theme::faint());
        let v = Style::new().fg(theme::text());
        let mut x = put(buf, bar.x, bar.y, " ◢ IRIS ", Style::new().fg(Color::Black).bg(theme::accent()).bold(), 10) + 2;
        for (key, val) in [
            ("MODELO", app.model.trim_start_matches("claude-").to_string()),
            ("DIR", cwd()),
            ("SES", app.session.get(..8).unwrap_or("········").to_string()),
        ] {
            x = put(buf, x, bar.y, key, k, 8);
            x = put(buf, x + 1, bar.y, &val, v, 40);
            x = put(buf, x + 1, bar.y, "│", sep, 1) + 1;
        }
        let clock = now_hhmmss();
        let pct = ctx_pct(app);
        let right = format!("CTX ▮▮▮▮▮▮▮▮▮▮ {:>2.0}% │ ${:.2} │ T{} │ {clock} ", pct, app.cost, app.turns);
        let rx = bar.right().saturating_sub(right.width() as u16);
        if rx > x {
            let mut x = put(buf, rx, bar.y, "CTX ", k, 4);
            let n = (pct / 10.0).round().clamp(0.0, 10.0) as usize;
            x = put(buf, x, bar.y, &"▮".repeat(n), Style::new().fg(tone), 10);
            x = put(buf, x, bar.y, &"▯".repeat(10 - n), sep, 10);
            x = put(buf, x + 1, bar.y, &format!("{pct:>2.0}%"), v, 4);
            x = put(buf, x + 1, bar.y, "│", sep, 1);
            x = put(buf, x + 1, bar.y, &format!("${:.2}", app.cost), v, 10);
            x = put(buf, x + 1, bar.y, "│", sep, 1);
            x = put(buf, x + 1, bar.y, "T", k, 1);
            x = put(buf, x, bar.y, &app.turns.to_string(), v, 5);
            x = put(buf, x + 1, bar.y, "│", sep, 1);
            put(buf, x + 1, bar.y, &clock, Style::new().fg(theme::accent()).bold(), 8);
        }
    }

    let left_w = (body.width * 38 / 100).clamp(30, 56);
    let narrow = body.width < 90;
    let [left, _, right] = if narrow {
        [Rect::default(), Rect::default(), body]
    } else {
        Layout::horizontal([Constraint::Length(left_w), Constraint::Length(2), Constraint::Min(30)]).areas(body)
    };

    // Left column: framed core, log and turn line.
    if !narrow {
        let core_h = (left.width / 2 + 2).min(left.height.saturating_sub(10)).max(8);
        let [frame, _, reg, line] = Layout::vertical([
            Constraint::Length(core_h),
            Constraint::Length(1),
            Constraint::Min(2),
            Constraint::Length(3),
        ])
        .areas(Rect { x: left.x + 1, width: left.width.saturating_sub(1), ..left });
        {
            let buf = f.buffer_mut();
            corners(buf, frame, Style::new().fg(theme::dim()), Some("NÚCLEO"));
        }
        let core = Rect { x: frame.x + 1, y: frame.y + 1, width: frame.width.saturating_sub(2), height: frame.height.saturating_sub(4) };
        core_draw(app, core, f.buffer_mut());
        let (title, detail) = core_label(app, t, frame.width as usize / 2);
        let buf = f.buffer_mut();
        let ly = frame.bottom().saturating_sub(3);
        let blink = if (t * 1.5) as usize % 2 == 0 { "◉" } else { "○" };
        let x = put(buf, frame.x + 2, ly, blink, Style::new().fg(tone).bold(), 1);
        put(buf, x + 1, ly, &title, Style::new().fg(tone).bold(), frame.width / 2);
        put_right(buf, frame.right().saturating_sub(3), ly, &truncate(&detail, frame.width as usize / 2), Style::new().fg(theme::faint()));
        for x in frame.x + 2..frame.right().saturating_sub(2) {
            let ch = if (x - frame.x - 2) % 5 == 0 { "┊" } else { "┈" };
            put(buf, x, ly + 1, ch, Style::new().fg(theme::dim()), 1);
        }

        // Log: each tool with its time.
        section(buf, reg.x, reg.right() - 1, reg.y, "REGISTRO");
        let rows = reg.height.saturating_sub(2) as usize;
        let list: Vec<&Activity> = app.activity.iter().rev().take(rows).collect();
        if list.is_empty() {
            put(buf, reg.x + 2, reg.y + 2, "— sin eventos —", Style::new().fg(theme::dim()), reg.width);
        }
        for (i, a) in list.iter().rev().enumerate() {
            let y = reg.y + 2 + i as u16;
            let (icon, c) = act_icon(a, t);
            let ago = a.started.elapsed().as_secs();
            let x = put(buf, reg.x, y, &clock_ago(ago), Style::new().fg(theme::dim()), 8);
            put(buf, x + 1, y, icon, Style::new().fg(c).bold(), 1);
            let x = put(buf, x + 3, y, &format!("{:<6}", truncate(&a.name.to_uppercase(), 6)), Style::new().fg(theme::text()).bold(), 6);
            let secs = format!("{:.1}s", act_secs(a));
            put(buf, x + 1, y, &a.detail, Style::new().fg(theme::faint()), reg.right().saturating_sub(x + 3 + secs.len() as u16));
            let sc = match a.status {
                ToolStatus::Running => theme::accent(),
                ToolStatus::Err => RED,
                ToolStatus::Ok => theme::dim(),
            };
            put_right(buf, reg.right() - 1, y, &secs, Style::new().fg(sc));
        }

        // Turn line.
        put(buf, line.x, line.y + 1, "LÍNEA", Style::new().fg(theme::faint()).bold(), 6);
        timeline(buf, Rect { x: line.x + 7, y: line.y + 1, width: line.width.saturating_sub(7), height: 2 }, turn_acts(app), true);
    }

    // Right column: the channel and the prompt.
    let prompt_h = if listening(state) { 6 } else { 4 };
    let [channel, prompt] = Layout::vertical([Constraint::Min(4), Constraint::Length(prompt_h)]).areas(right);
    let channel = Rect { width: channel.width.saturating_sub(1), ..channel };
    {
        let buf = f.buffer_mut();
        corners(buf, channel, Style::new().fg(theme::dim()), Some("CANAL"));
    }
    let chat_in = Rect { x: channel.x + 1, y: channel.y + 2, width: channel.width.saturating_sub(3), height: channel.height.saturating_sub(3) };
    let rows = chat_rows(app, chat_in.width.saturating_sub(4) as usize, Voice::Channel);
    render_chat(f, chat_in, app, rows);

    let prompt = Rect { width: prompt.width.saturating_sub(1), ..prompt };
    let active = !app.busy || listening(state);
    let col = if listening(state) { tone } else if active { theme::accent() } else { theme::dim() };
    {
        let buf = f.buffer_mut();
        corners(buf, prompt, Style::new().fg(col), None);
        let y = prompt.y + 1;
        let x = put(buf, prompt.x + 3, y, "ORDEN", Style::new().fg(if active { col } else { theme::faint() }).bold(), 5);
        put(buf, x + 1, y, "│", Style::new().fg(theme::dim()), 1);
    }
    if listening(state) {
        let w = Rect { x: prompt.x + 3, y: prompt.y + 2, width: prompt.width.saturating_sub(6), height: prompt.height.saturating_sub(3) };
        wave(f, w, app, t, tone, theme::dim());
    } else {
        let line = Rect { x: prompt.x + 11, y: prompt.y + 1, width: prompt.width.saturating_sub(14), height: 1 };
        f.render_widget(Paragraph::new(Line::from(input_spans(app, state, t, line.width as usize, col))), line);
        draw_modal(f, Rect { x: channel.x, y: prompt.y, width: channel.width, height: 1 }, app);
    }

    let buf = f.buffer_mut();
    match flash(app) {
        Some(l) => {
            buf.set_line(foot.x + 1, foot.y, &l, foot.width);
        }
        None => keys(buf, foot.x + 1, foot.y, foot.right(), &hints_for(app, state), if listening(state) { tone } else { theme::accent() }, true),
    }
}
