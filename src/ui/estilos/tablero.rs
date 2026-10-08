//! El estilo Tablero de /theme.

use crate::ui::*;

pub(in crate::ui) fn tablero(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let inner = Rect { x: area.x + 1, width: area.width.saturating_sub(2), ..area };
    let band_h = (inner.height * 46 / 100).clamp(12, 28);
    let input_h = if listening(state) { 3 } else { 1 };
    let [head, band, chat, input, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(band_h),
        Constraint::Min(5),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .areas(inner);
    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x + 1, head.y, "◆ IRIS", Style::new().fg(theme::accent()).bold(), 8);
        put(buf, x + 2, head.y, "tablero de la sesión", Style::new().fg(theme::faint()), 22);
        put_right(buf, head.right() - 1, head.y, &format!("{} · {} · {}", model_short(app), ses8(app), now_hhmmss()), Style::new().fg(theme::faint()));
    }

    let [core_t, _, right] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Length(1), Constraint::Min(30)]).areas(band);
    let [r_top, r_bot] = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(right);
    let [ctx_t, _, cost_t] = Layout::horizontal([Constraint::Percentage(50), Constraint::Length(1), Constraint::Min(10)]).areas(r_top);
    let [tools_t, _, ses_t] = Layout::horizontal([Constraint::Percentage(50), Constraint::Length(1), Constraint::Min(10)]).areas(r_bot);

    let dim = Style::new().fg(theme::dim());
    // Núcleo.
    {
        let buf = f.buffer_mut();
        corners(buf, core_t, dim, Some(&spaced("NÚCLEO")));
    }
    let core = Rect { x: core_t.x + 1, y: core_t.y + 1, width: core_t.width.saturating_sub(2), height: core_t.height.saturating_sub(4) };
    core_draw(app, core, f.buffer_mut());
    let buf = f.buffer_mut();
    let (title, detail) = core_label(app, t, core_t.width as usize);
    put_center(buf, core_t, core_t.bottom().saturating_sub(3), &spaced_title(&title), Style::new().fg(tone).bold());
    put_center(buf, core_t, core_t.bottom().saturating_sub(2), &truncate(&detail, core_t.width as usize - 2), Style::new().fg(theme::faint()));

    // Contexto: el número grande y el medidor.
    corners(buf, ctx_t, dim, Some(&spaced("CONTEXTO")));
    let pct = ctx_pct(app);
    let big_row = |buf: &mut Buffer, r: Rect, s: &str, col: Color| {
        match big_text(s, r.width.saturating_sub(4) as usize) {
            Some(rows) if r.height >= 6 => {
                for (k, row) in rows.iter().enumerate() {
                    put(buf, r.x + 3, r.y + 2 + k as u16, row, Style::new().fg(col).bold(), r.width.saturating_sub(4));
                }
            }
            _ => {
                put(buf, r.x + 3, r.y + 2, s, Style::new().fg(col).bold(), r.width.saturating_sub(4));
            }
        }
    };
    big_row(buf, ctx_t, &format!("{pct:.0}%"), theme::text());
    if ctx_t.height >= 7 {
        let y = ctx_t.bottom() - 2;
        let w = ctx_t.width.saturating_sub(6);
        meter(buf, ctx_t.x + 3, y, w, pct / 100.0, tone);
    }

    // Costo y turnos.
    corners(buf, cost_t, dim, Some(&spaced("COSTO")));
    big_row(buf, cost_t, &format!("${:.2}", app.cost), theme::text());
    if cost_t.height >= 7 {
        put(buf, cost_t.x + 3, cost_t.bottom() - 2, &turns(app), Style::new().fg(theme::faint()), cost_t.width.saturating_sub(4));
    }

    // Herramientas: cuántas veces cada una, en barras.
    corners(buf, tools_t, dim, Some(&spaced("HERRAMIENTAS")));
    let mut counts: Vec<(String, usize, usize)> = Vec::new();
    for a in &app.activity {
        match counts.iter_mut().find(|c| c.0 == a.name) {
            Some(c) => {
                c.1 += 1;
                c.2 += (a.status == ToolStatus::Err) as usize;
            }
            None => counts.push((a.name.clone(), 1, (a.status == ToolStatus::Err) as usize)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let rows = tools_t.height.saturating_sub(3) as usize;
    if counts.is_empty() {
        put(buf, tools_t.x + 3, tools_t.y + 2, "— ninguna todavía —", Style::new().fg(theme::dim()), tools_t.width);
    }
    let max = counts.first().map_or(1, |c| c.1).max(1);
    let bar_w = tools_t.width.saturating_sub(22) as usize;
    for (i, (name, n, err)) in counts.iter().take(rows).enumerate() {
        let y = tools_t.y + 2 + i as u16;
        let x = put(buf, tools_t.x + 3, y, &format!("{:<7}", truncate(name, 7)), Style::new().fg(theme::text()), 7);
        let len = (n * bar_w / max).max(1);
        let x = put(buf, x + 1, y, &"█".repeat(len), Style::new().fg(if *err > 0 && *err == *n { RED } else { tone }), len as u16);
        let tail = if *err > 0 { format!("{n} · {err} err") } else { n.to_string() };
        put(buf, x + 1, y, &tail, Style::new().fg(theme::faint()), 12);
    }

    // Sesión.
    corners(buf, ses_t, dim, Some(&spaced("SESIÓN")));
    let items = [("modelo", model_short(app)), ("voz", voice_value(app)), ("en línea", uptime(app)), ("sesión", ses8(app))];
    for (i, (k, v)) in items.iter().enumerate() {
        let y = ses_t.y + 2 + i as u16;
        if y + 1 >= ses_t.bottom() {
            break;
        }
        put(buf, ses_t.x + 3, y, k, Style::new().fg(theme::faint()), 10);
        put(buf, ses_t.x + 13, y, v, Style::new().fg(theme::text()), ses_t.width.saturating_sub(15));
    }

    // Conversación.
    corners(buf, chat, dim, Some(&spaced("CONVERSACIÓN")));
    let chat_in = Rect { x: chat.x + 2, y: chat.y + 1, width: chat.width.saturating_sub(4), height: chat.height.saturating_sub(2) };
    if app.messages.is_empty() {
        *app.view.borrow_mut() = Default::default();
        put(buf, chat_in.x + GUION as u16, chat_in.y + 1, &format!("{}. ¿En qué trabajamos?", greeting()), Style::new().fg(theme::text()).bold(), chat_in.width);
    } else {
        render_chat(f, chat_in, app, chat_rows(app, chat_in.width as usize, Voz::Guion));
    }

    if listening(state) {
        wave(f, Rect { x: input.x + 1, width: input.width.saturating_sub(2), ..input }, app, t, tone, theme::dim());
    } else {
        orden_line(f, Rect { x: input.x + 1, ..input }, Rect { y: chat.bottom().saturating_sub(1), height: 1, ..chat }, app, state, t, "› ", theme::accent());
    }
    let kc = if listening(state) { tone } else { theme::accent() };
    foot_keys(f.buffer_mut(), Rect { x: hints.x + 1, ..hints }, app, state, kc, false);
}
