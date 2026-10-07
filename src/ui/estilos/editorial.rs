//! El estilo Editorial de /theme.

use crate::ui::*;

pub(in crate::ui) fn editorial(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let m = if area.width > 100 { 3 } else { 1 };
    let inner = Rect { x: area.x + m, width: area.width.saturating_sub(m * 2), ..area };
    let (title, detail) = core_label(app, t, inner.width as usize);
    let big = big_text(&title, inner.width as usize);
    let title_h = if big.is_some() { 3 } else { 1 };
    let input_h = if listening(state) { 3 } else { 1 };
    let [head, _, tit, _, sub, rule, _, body, rule2, input, _, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(title_h),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(1),
        Constraint::Length(input_h),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    {
        let buf = f.buffer_mut();
        const DIAS: [&str; 7] = ["domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado"];
        const MESES: [&str; 12] =
            ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
        let (_, mo, d, wd) = local_date();
        let x = put(buf, head.x, head.y, &spaced("JARVIS"), Style::new().fg(theme::accent()).bold(), head.width);
        let ed = format!("  ·  edición del {} {d} de {}  ·  nº {}", DIAS[wd as usize], MESES[(mo as usize).saturating_sub(1) % 12], app.turns);
        put(buf, x, head.y, &ed, Style::new().fg(theme::faint()), head.width.saturating_sub(x - head.x + 10));
        put_right(buf, head.right() - 1, head.y, &now_hhmmss(), Style::new().fg(theme::text()));
        match &big {
            Some(rows) => {
                for (k, r) in rows.iter().enumerate() {
                    put(buf, tit.x, tit.y + k as u16, r, Style::new().fg(tone).bold(), tit.width);
                }
            }
            None => {
                put(buf, tit.x, tit.y, &spaced_title(&title), Style::new().fg(tone).bold(), tit.width);
            }
        }
        let lead = if detail.is_empty() { format!("{}. ¿En qué trabajamos?", greeting()) } else { detail.clone() };
        put(buf, sub.x, sub.y, &lead, Style::new().fg(theme::text()), sub.width);
        hline(buf, rule.x, rule.right() - 1, rule.y, "━", Style::new().fg(theme::faint()));
    }

    // Columna de datos │ lectura │ figura.
    let col_w = 26u16;
    let narrow = body.width < 110;
    let fig_w = if narrow { 0 } else { (body.width * 34 / 100).clamp(30, 60) };
    let [data, bar, read, _, fig] = Layout::horizontal([
        Constraint::Length(col_w),
        Constraint::Length(3),
        Constraint::Min(30),
        Constraint::Length(if narrow { 0 } else { 3 }),
        Constraint::Length(fig_w),
    ])
    .areas(body);
    {
        let buf = f.buffer_mut();
        for y in bar.y..bar.bottom() {
            put(buf, bar.x + 1, y, "│", Style::new().fg(theme::dim()), 1);
        }
        let last = app.activity.last().map(|a| {
            let (icon, _) = act_icon(a, t);
            format!("{icon} {} · {:.1}s", a.name, act_secs(a))
        });
        let items = [
            ("MODELO", model_short(app)),
            ("SESIÓN", ses8(app)),
            ("CARPETA", cwd()),
            ("CONTEXTO", format!("{:.0} %", ctx_pct(app))),
            ("COSTO", format!("${:.2}", app.cost)),
            ("VOZ", voice_value(app)),
            ("AHORA", last.unwrap_or_else(|| "—".into())),
        ];
        let mut y = data.y;
        for (k, v) in items {
            if y + 1 >= data.bottom() {
                break;
            }
            put(buf, data.x, y, &spaced(k), Style::new().fg(theme::faint()).bold(), data.width);
            put(buf, data.x, y + 1, &v, Style::new().fg(theme::text()), data.width);
            y += 3;
        }
    }
    if app.messages.is_empty() {
        *app.view.borrow_mut() = Default::default();
        let buf = f.buffer_mut();
        put(buf, read.x + 2, read.y + 1, "Escribe, o mantén espacio y háblame.", Style::new().fg(theme::faint()), read.width);
    } else {
        render_chat(f, read, app, chat_rows(app, read.width as usize, Voz::Guion));
    }
    if !narrow {
        let core = Rect { height: fig.height.saturating_sub(3).min(fig.width / 2 + 2), ..fig };
        core_draw(app, core, f.buffer_mut());
        let buf = f.buffer_mut();
        let y = core.bottom() + 1;
        hline(buf, fig.x, fig.right() - 1, y, "─", Style::new().fg(theme::dim()));
        let cap = if detail.is_empty() { "a la espera".to_string() } else { detail.to_lowercase() };
        put(buf, fig.x, y + 1, &format!("Fig. 1 — El núcleo, {cap}."), Style::new().fg(theme::faint()), fig.width);
    }

    {
        let buf = f.buffer_mut();
        hline(buf, rule2.x, rule2.right() - 1, rule2.y, "─", Style::new().fg(theme::dim()));
    }
    if listening(state) {
        wave(f, input, app, t, tone, theme::dim());
    } else {
        orden_line(f, input, Rect { width: read.right() - input.x, ..rule2 }, app, state, t, "Escribe → ", theme::accent());
    }
    let kc = if listening(state) { tone } else { theme::accent() };
    foot_keys(f.buffer_mut(), hints, app, state, kc, false);
}
