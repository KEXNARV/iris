//! El estilo Radar de /theme.

use crate::ui::*;

pub(in crate::ui) const SECTORES: [&str; 8] = ["LEER", "BUSCAR", "EDITAR", "GIT", "PROBAR", "EJECUTAR", "RED", "AGENTES"];

pub(in crate::ui) fn sector(a: &Activity) -> usize {
    match crate::nucleo::tool_state(&a.name, &a.detail) {
        State::Reading => 0,
        State::Searching => 1,
        State::Editing => 2,
        State::Git => 3,
        State::Testing => 4,
        State::Web => 6,
        State::Delegating | State::Planning => 7,
        _ => 5,
    }
}

pub(in crate::ui) fn radar(f: &mut Frame, app: &App) {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, TAU};
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let inner = Rect { x: area.x + 2, width: area.width.saturating_sub(4), ..area };
    let input_h = if listening(state) { 4 } else { 2 };
    let [head, rule, body, input, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .areas(inner);
    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x, head.y, "◎ IRIS", Style::new().fg(theme::accent()).bold(), head.width);
        put(buf, x + 3, head.y, &model_short(app), Style::new().fg(theme::text()), head.width / 3);
        let clock = now_hhmmss();
        put_right(buf, head.right() - 1, head.y, &clock, Style::new().fg(theme::text()));
        put_right(buf, head.right() - 1 - clock.len() as u16 - 3, head.y, &format!("{}   {}", cwd(), ses8(app)), Style::new().fg(theme::faint()));
        hline(buf, rule.x, rule.right() - 1, rule.y, "─", Style::new().fg(theme::dim()));
    }

    let side_w = (body.width * 46 / 100).clamp(40, 76);
    let narrow = body.width < 100;
    let (chat, side) = if narrow {
        (body, Rect::default())
    } else {
        let [c, _, s] = Layout::horizontal([Constraint::Min(30), Constraint::Length(2), Constraint::Length(side_w)]).areas(body);
        (c, s)
    };
    let chat_in = Rect { y: chat.y + 1, height: chat.height.saturating_sub(1), ..chat };
    if app.messages.is_empty() {
        let buf = f.buffer_mut();
        let y = chat_in.y + chat_in.height / 3;
        put(buf, chat_in.x + GUION as u16, y, &format!("{}.", greeting()), Style::new().fg(theme::text()).bold(), chat_in.width);
        put(buf, chat_in.x + GUION as u16, y + 1, "¿En qué trabajamos?", Style::new().fg(theme::accent()).bold(), chat_in.width);
        *app.view.borrow_mut() = Default::default();
    } else {
        render_chat(f, chat_in, app, chat_rows(app, chat_in.width as usize, Voz::Guion));
    }

    if !narrow {
        let disc_h = side.height.saturating_sub(5);
        // Radio en filas; en columnas es el doble, porque las celdas son altas.
        let r = ((disc_h as f64 / 2.0) - 1.5).min(side.width as f64 / 4.0 - 4.0).max(4.0);
        let cx = side.x as f64 + side.width as f64 / 2.0;
        let cy = side.y as f64 + disc_h as f64 / 2.0;
        let at = |a: f64, rr: f64| ((cx + a.cos() * rr * 2.0).round(), (cy - a.sin() * rr).round());
        let ok = |x: f64, y: f64| x >= side.x as f64 && x < side.right() as f64 && y >= side.y as f64 && y < (side.y + disc_h) as f64;
        {
            let buf = f.buffer_mut();
            let dim = Style::new().fg(theme::dim());
            // Anillos y divisiones de los sectores.
            for (rr, step) in [(r, 0.03), (r * 0.66, 0.05), (r * 0.33, 0.09)] {
                let mut a = 0.0;
                while a < TAU {
                    let (x, y) = at(a, rr);
                    if ok(x, y) {
                        put(buf, x as u16, y as u16, "·", dim, 1);
                    }
                    a += step;
                }
            }
            for k in 0..8 {
                let a = FRAC_PI_2 - k as f64 * FRAC_PI_4 + FRAC_PI_4 / 2.0;
                let mut rr = r * 0.4;
                while rr <= r {
                    let (x, y) = at(a, rr);
                    if ok(x, y) {
                        put(buf, x as u16, y as u16, "·", dim, 1);
                    }
                    rr += 0.9;
                }
            }
            // El barrido, con su estela.
            let speed = if app.busy { 2.2 } else { 0.8 };
            let sweep = -t * speed;
            for (j, c) in [(0, tone), (1, mix(tone, theme::dim(), 0.5)), (2, theme::dim())] {
                let a = sweep + j as f64 * 0.08;
                let mut rr = r * 0.36;
                while rr <= r {
                    let (x, y) = at(a, rr);
                    if ok(x, y) {
                        put(buf, x as u16, y as u16, if j == 0 { "•" } else { "·" }, Style::new().fg(c), 1);
                    }
                    rr += 0.5;
                }
            }
            for (k, name) in SECTORES.iter().enumerate() {
                let a = FRAC_PI_2 - k as f64 * FRAC_PI_4;
                let (x, y) = at(a, r + 1.3);
                let w = name.len() as f64;
                let x0 = (x - w / 2.0).max(side.x as f64).min(side.right() as f64 - w);
                if y >= side.y as f64 && y < (side.y + disc_h) as f64 {
                    put(buf, x0 as u16, y as u16, name, Style::new().fg(theme::faint()), name.len() as u16);
                }
            }
        }
        // El núcleo en el centro.
        let cw = ((r * 0.62 * 2.0) as u16).max(6);
        let ch = ((r * 0.62) as u16).max(3);
        let core = Rect { x: (cx as u16).saturating_sub(cw / 2), y: (cy as u16).saturating_sub(ch / 2), width: cw, height: ch };
        core_draw(app, core, f.buffer_mut());
        let buf = f.buffer_mut();
        // Las herramientas como blips en su sector: las más nuevas, más afuera.
        let recent: Vec<&Activity> = app.activity.iter().rev().take(8).collect();
        for (i, a) in recent.iter().enumerate() {
            let s = sector(a);
            let jitter = (i % 3) as f64 - 1.0;
            let ang = FRAC_PI_2 - s as f64 * FRAC_PI_4 + jitter * 0.22;
            let rr = r * (0.9 - 0.1 * (i as f64 / 2.0).floor()).max(0.55);
            let (x, y) = at(ang, rr);
            if !ok(x, y) {
                continue;
            }
            let (icon, c) = act_icon(a, t);
            let mark = if a.status == ToolStatus::Running { icon } else { "◆" };
            put(buf, x as u16, y as u16, mark, Style::new().fg(c).bold(), 1);
            let label = format!("{} {:.1}s", a.name, act_secs(a));
            let room = side.right().saturating_sub(x as u16 + 2);
            put(buf, x as u16 + 2, y as u16, &label, Style::new().fg(if i == 0 { theme::text() } else { theme::faint() }), room);
        }
        let (title, detail) = core_label(app, t, side.width as usize);
        let y = side.y + disc_h;
        put_center(buf, side, y + 1, &spaced_title(&title), Style::new().fg(tone).bold());
        put_center(buf, side, y + 2, &detail, Style::new().fg(theme::faint()));
        let kx = side.x + side.width / 2 - 18;
        put(buf, kx, y + 4, "contexto", Style::new().fg(theme::faint()), 10);
        let x = meter(buf, kx + 10, y + 4, 20, ctx_pct(app) / 100.0, tone);
        put(buf, x + 2, y + 4, &format!("{:.0}%", ctx_pct(app)), Style::new().fg(theme::text()), 5);
    }

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
        orden_line(f, line, Rect { y: input_w.y, height: 1, ..input_w }, app, state, t, "› ", theme::accent());
    }
    let kc = if listening(state) { tone } else { theme::accent() };
    foot_keys(f.buffer_mut(), hints, app, state, kc, false);
}
