//! What gets written to the terminal outside ratatui: the core in Sixel and the thumbnails.

use crate::*;

/// Can the core be drawn as an image? It needs a terminal with Sixel (foot) and to know how
/// many pixels a cell measures. `IRIS_SIXEL=0` turns it off; `=1` forces it in another terminal.
pub(crate) fn sixel_cell() -> Option<(u16, u16)> {
    let want = crate::paths::var("SIXEL");
    let foot = std::env::var("TERM").is_ok_and(|t| t.starts_with("foot"));
    if want.as_deref() == Some("0") || (!foot && want.as_deref() != Some("1")) {
        return None;
    }
    cell_px()
}

pub(crate) fn cell_px() -> Option<(u16, u16)> {
    let ws = crossterm::terminal::window_size().ok()?;
    (ws.width > 0 && ws.columns > 0 && ws.rows > 0).then(|| (ws.width / ws.columns, ws.height / ws.rows))
}

/// Sends the core's image at 30 fps (one of every two frames). If the panel moved or the core
/// went back to braille (menu open), it clears the screen: the old image doesn't erase itself.
/// Returns whether it cleared the screen.
pub(crate) fn sixel_frame(term: &mut ratatui::DefaultTerminal, app: &mut App, frames: &mut u64) -> Result<bool> {
    use std::io::Write;
    let target = app.core_rect.get();
    let mut cleared = false;
    if app.sixel_shown.is_some() && app.sixel_shown != target {
        app.sixel_shown = None;
        term.clear()?;
        cleared = true;
        if target.is_none() {
            app.thumb_slots.borrow_mut().clear();
            app.thumb_targets.borrow_mut().clear();
            app.file_slots.borrow_mut().clear();
            app.file_targets.borrow_mut().clear();
            term.draw(|f| ui::draw(f, app))?;
        }
    }
    let Some(r) = target else { return Ok(cleared) };
    *frames += 1;
    if app.sixel_shown == Some(r) && *frames % 2 == 1 {
        return Ok(cleared);
    }
    // The cell size changes with the font zoom; it is measured again.
    let Some((cw, ch)) = cell_px().or(app.sixel_cell) else { return Ok(cleared) };
    app.sixel_cell = Some((cw, ch));
    let sp = crate::paths::var("DOT").and_then(|v| v.parse().ok()).unwrap_or(4);
    let img = app.core.sixel(r.width as usize * cw as usize, r.height as usize * ch as usize, sp);
    // Each image has a transparent background and foot shows the previous one through it: without
    // clearing the panel, old frames pile up. Erasing the cells (ECH) removes the image below;
    // inside a synchronized update (mode 2026) erasing and drawing show up together.
    let mut seq = String::with_capacity(img.len() + 32 * r.height as usize);
    seq.push_str("\x1b[?2026h\x1b7");
    for y in r.y..r.bottom() {
        seq.push_str(&format!("\x1b[{};{}H\x1b[{}X", y + 1, r.x + 1, r.width));
    }
    seq.push_str("\x1b[49m");
    seq.push_str(&format!("\x1b[{};{}H", r.y + 1, r.x + 1));
    seq.push_str(&img);
    seq.push_str("\x1b8\x1b[?2026l");
    let mut out = std::io::stdout().lock();
    out.write_all(seq.as_bytes())?;
    out.flush()?;
    app.sixel_shown = Some(r);
    Ok(cleared)
}

/// The chat thumbnails: they are drawn only when they change place (scrolling, new text).
/// Where they were, that frame's text is rewritten: writing over it erases the old image
/// (foot doesn't erase it by itself) and leaves the right text. Blanking the cells ate the
/// text that was there, and ratatui didn't rewrite it because it thought it was drawn.
pub(crate) fn thumbs_frame(term: &mut ratatui::DefaultTerminal, app: &mut App, snap: &ratatui::buffer::Buffer) -> Result<()> {
    use std::io::Write;
    let targets = std::mem::take(&mut *app.thumb_targets.borrow_mut());
    let now: Vec<(ratatui::layout::Rect, usize, (u16, u16, u16))> =
        targets.iter().map(|(r, t, c)| (*r, std::rc::Rc::as_ptr(t) as usize, *c)).collect();
    if now == app.thumbs_shown {
        return Ok(());
    }
    let Some(cell) = app.sixel_cell else { return Ok(()) };
    {
        use ratatui::backend::Backend;
        let area = snap.area;
        let mut cells = vec![];
        for (r, _, _) in &app.thumbs_shown {
            for y in r.y..r.bottom().min(area.bottom()) {
                for x in r.x..r.right().min(area.right()) {
                    cells.push((x, y, &snap[(x, y)]));
                }
            }
        }
        let b = term.backend_mut();
        b.draw(cells.into_iter())?;
        Backend::flush(b)?;
    }
    let mut seq = String::from("\x1b[?2026h\x1b7");
    for (r, t, (from, to, total)) in &targets {
        seq.push_str(&format!("\x1b[{};{}H", r.y + 1, r.x + 1));
        seq.push_str(&t.sixel(cell, r.width, *total, *from, *to));
    }
    seq.push_str("\x1b8\x1b[?2026l");
    let mut out = std::io::stdout().lock();
    out.write_all(seq.as_bytes())?;
    out.flush()?;
    app.thumbs_shown = now;
    Ok(())
}
