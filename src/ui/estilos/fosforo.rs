//! El estilo Fósforo de /theme.

use crate::ui::*;

pub(in crate::ui) fn fosforo(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let ph = Style::new().fg(theme::accent());
    {
        let buf = f.buffer_mut();
        let (x0, x1, y0, y1) = (area.x, area.right() - 1, area.y, area.bottom() - 1);
        put(buf, x0, y0, "╔", ph, 1);
        put(buf, x1, y0, "╗", ph, 1);
        put(buf, x0, y1, "╚", ph, 1);
        put(buf, x1, y1, "╝", ph, 1);
        hline(buf, x0 + 1, x1 - 1, y0, "═", ph);
        hline(buf, x0 + 1, x1 - 1, y1, "═", ph);
        for y in y0 + 1..y1 {
            put(buf, x0, y, "║", ph, 1);
            put(buf, x1, y, "║", ph, 1);
        }
    }
    let inner = Rect { x: area.x + 2, y: area.y + 1, width: area.width.saturating_sub(4), height: area.height.saturating_sub(2) };
    let input_h = if listening(state) { 3 } else { 1 };
    let [head, rule, body, rule2, input, _, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(input_h),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    {
        let buf = f.buffer_mut();
        let hi = Style::new().fg(theme::text()).bold();
        let lo = Style::new().fg(theme::faint());
        let mut x = put(buf, head.x, head.y, "IRIS-OS", hi, head.width);
        for v in [model_short(app), cwd(), format!("SES {}", ses8(app))] {
            x = put(buf, x + 3, head.y, &v.to_uppercase(), lo, head.right().saturating_sub(x + 12));
        }
        put_right(buf, head.right() - 1, head.y, &now_hhmmss(), hi);
        hline(buf, rule.x, rule.right() - 1, rule.y, "─", Style::new().fg(theme::dim()));
    }

    let side_w = (body.width * 40 / 100).clamp(30, 60);
    let narrow = body.width < 90;
    let (chat, side) = if narrow {
        (body, Rect::default())
    } else {
        let [c, _, s] = Layout::horizontal([Constraint::Min(30), Constraint::Length(3), Constraint::Length(side_w)]).areas(body);
        (c, s)
    };
    let chat_in = Rect { y: chat.y + 1, height: chat.height.saturating_sub(1), ..chat };
    render_chat(f, chat_in, app, chat_rows(app, chat_in.width as usize, Voz::Canal));

    // El osciloscopio: retícula, el núcleo encima y las lecturas debajo.
    if !narrow {
        let core_h = (side.width / 2).min(side.height.saturating_sub(8)).max(6);
        let frame = Rect { height: core_h + 2, ..side };
        {
            let buf = f.buffer_mut();
            corners(buf, frame, Style::new().fg(theme::dim()), Some("CANAL A ─ 2V/DIV"));
            for y in (frame.y + 2..frame.bottom().saturating_sub(1)).step_by(3) {
                for x in (frame.x + 2..frame.right().saturating_sub(2)).step_by(6) {
                    put(buf, x, y, "·", Style::new().fg(theme::dim()), 1);
                }
            }
        }
        let core = Rect { x: frame.x + 1, y: frame.y + 1, width: frame.width.saturating_sub(2), height: frame.height.saturating_sub(2) };
        core_draw(app, core, f.buffer_mut());
        let (title, detail) = core_label(app, t, side.width as usize);
        let buf = f.buffer_mut();
        let hi = Style::new().fg(theme::text()).bold();
        let lo = Style::new().fg(theme::faint());
        let mut y = frame.bottom() + 1;
        put(buf, side.x + 1, y, &format!("ESTADO: {}", title.to_uppercase()), hi, side.width);
        y += 1;
        put(buf, side.x + 1, y, &detail.to_uppercase(), lo, side.width);
        y += 2;
        let n = (ctx_pct(app) / 5.0).round().clamp(0.0, 20.0) as usize;
        let rows = [
            format!("CTX   [{}{}]  {:.0}%", "#".repeat(n), ".".repeat(20 - n), ctx_pct(app)),
            format!("COSTO ${:.2}", app.cost),
            format!("TURNO {}", app.turns),
            format!("VOZ   {}", voice_value(app).to_uppercase()),
        ];
        for r in rows {
            if y >= side.bottom() {
                break;
            }
            put(buf, side.x + 1, y, &r, Style::new().fg(theme::text()), side.width);
            y += 1;
        }
    }

    {
        let buf = f.buffer_mut();
        hline(buf, chat.x, chat.right() - 1, rule2.y, "─", Style::new().fg(theme::dim()));
    }
    let input_w = Rect { width: chat.width, ..input };
    if listening(state) {
        wave(f, input_w, app, t, theme::accent(), theme::dim());
    } else {
        orden_line(f, input_w, Rect { width: chat.width, ..rule2 }, app, state, t, "IRIS> ", theme::accent());
    }
    {
        let buf = f.buffer_mut();
        match flash(app) {
            Some(l) => {
                buf.set_line(hints.x, hints.y, &l, hints.width);
            }
            None => {
                let up: Vec<(String, String)> = hints_for(app, state).into_iter().map(|(k, d)| (k.to_uppercase(), d.to_uppercase())).collect();
                let refs: Vec<(&str, &str)> = up.iter().map(|(k, d)| (k.as_str(), d.as_str())).collect();
                keys(buf, hints.x, hints.y, hints.right(), &refs, theme::text(), false);
            }
        }
        // Todo pasa a la rampa del fósforo; una fila más brillante baja como el barrido.
        let scan = area.y + ((t * 7.0) as u16 % area.height.max(1));
        phosphor(buf, area, scan);
    }
}

/// Pasa cada celda a la rampa de un solo color según lo brillante que era.
pub(in crate::ui) fn phosphor(buf: &mut Buffer, area: Rect, scan: u16) {
    let ramp = [theme::dim(), mix(theme::dim(), theme::accent(), 0.55), theme::accent(), rgb3(theme::accent_toward_white(0.55))];
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            if !matches!(cell.bg, Color::Reset) {
                cell.set_bg(theme::accent()).set_fg(Color::Black);
                continue;
            }
            let v = match cell.fg {
                Color::Rgb(r, g, b) => r.max(g).max(b) as f64 / 255.0,
                Color::Black => 0.0,
                Color::White => 1.0,
                _ => 0.75,
            };
            let mut k = if v < 0.45 {
                0
            } else if v < 0.6 {
                1
            } else if v < 0.93 {
                2
            } else {
                3
            };
            if y == scan {
                k = (k + 1).min(3);
            }
            cell.set_fg(ramp[k]);
        }
    }
}
