//! El estilo Zen de /theme.

use crate::ui::*;

pub(in crate::ui) fn zen(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let inner = Rect { x: area.x + 3, width: area.width.saturating_sub(6), ..area };
    let input_h = if listening(state) { 3 } else { 1 };
    let top_h = inner.height * 40 / 100;
    let [top, chat, _, input, _] = Layout::vertical([
        Constraint::Length(top_h),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .areas(inner);

    // El punto: late más rápido cuanto más hace.
    {
        let buf = f.buffer_mut();
        let speed = if app.busy { 4.0 } else if listening(state) { 6.0 } else { 1.2 };
        let p = (t * speed).sin();
        let dot = if p > 0.35 {
            "●"
        } else if p > -0.35 {
            "•"
        } else {
            "·"
        };
        let y = top.y + top.height / 2;
        put_center(buf, top, y, dot, Style::new().fg(tone).bold());
        let (title, _) = core_label(app, t, top.width as usize);
        put_center(buf, top, y + 2, &title.to_lowercase(), Style::new().fg(theme::dim()));
    }
    if app.messages.is_empty() {
        *app.view.borrow_mut() = Default::default();
    } else {
        render_chat(f, chat, app, chat_rows(app, chat.width as usize, Voz::Canal));
    }

    let stats = format!("{} · {:.0}% · ${:.2}", model_short(app), ctx_pct(app), app.cost);
    let sw = stats.width() as u16;
    if listening(state) {
        wave(f, input, app, t, tone, theme::dim());
    } else {
        let line = Rect { width: input.width.saturating_sub(sw + 2), ..input };
        orden_line(f, line, Rect { y: input.y.saturating_sub(1), height: 1, ..line }, app, state, t, "› ", theme::accent());
        let buf = f.buffer_mut();
        put_right(buf, input.right() - 1, input.y, &stats, Style::new().fg(theme::dim()));
    }
    if let Some(l) = flash(app) {
        let buf = f.buffer_mut();
        let y = area.bottom() - 1;
        buf.set_line(inner.x, y, &l, inner.width);
    }
}
