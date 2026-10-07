//! El estilo Clásico de /theme.

use crate::ui::*;

pub(in crate::ui) fn clasico(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();

    let [header, body, bottom, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(if listening(state) { 5 } else { 3 }),
        Constraint::Length(1),
    ])
    .areas(f.area());

    draw_header(f, header, app);

    let side_w = (body.width / 3).clamp(30, 46);
    let [chat, side] = Layout::horizontal([Constraint::Min(30), Constraint::Length(side_w)]).areas(body);
    draw_chat(f, chat, app);

    let core_h = (side.width / 2 + 3).min(side.height / 2).max(10);
    let [core, act] = Layout::vertical([Constraint::Length(core_h), Constraint::Min(4)]).areas(side);
    draw_core(f, core, app, t);
    draw_activity(f, act, app, t);

    if listening(state) {
        draw_wave(f, bottom, app, t);
    } else {
        draw_input(f, bottom, app, state, t);
        draw_modal(f, Rect { width: chat.width, ..bottom }, app);
    }
    draw_footer(f, footer, app);
}
