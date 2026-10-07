//! Lo que se escribe a la terminal fuera de ratatui: el núcleo en Sixel y las miniaturas.

use crate::*;

/// ¿Se puede dibujar el núcleo como imagen? Hace falta una terminal con Sixel (foot) y saber
/// cuántos píxeles mide una celda. `JARVIS_SIXEL=0` lo apaga; `=1` lo fuerza en otra terminal.
pub(crate) fn sixel_cell() -> Option<(u16, u16)> {
    let want = std::env::var("JARVIS_SIXEL").ok();
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

/// Manda la imagen del núcleo a 30 fps (uno de cada dos cuadros). Si el panel se movió o el
/// núcleo volvió a braille (menú abierto), limpia la pantalla: la imagen vieja no se borra sola.
/// Devuelve si limpió la pantalla.
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
    // El tamaño de la celda cambia con el zoom de la fuente; se vuelve a medir.
    let Some((cw, ch)) = cell_px().or(app.sixel_cell) else { return Ok(cleared) };
    app.sixel_cell = Some((cw, ch));
    let sp = std::env::var("JARVIS_DOT").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let img = app.nucleo.sixel(r.width as usize * cw as usize, r.height as usize * ch as usize, sp);
    // Cada imagen tiene fondo transparente y foot deja ver por ahí la anterior: sin borrar el
    // panel, los cuadros viejos se acumulan. Borrar las celdas (ECH) elimina la imagen de abajo;
    // dentro de una actualización sincronizada (modo 2026) borrar y dibujar se ven juntos.
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

/// Las miniaturas del chat: se dibujan solo cuando cambian de lugar (desplazar, texto nuevo).
/// Donde estaban, se reescribe el texto de ese cuadro: escribir encima borra la imagen vieja
/// (foot no la borra solo) y deja el texto correcto. Borrar las celdas en blanco se comía el
/// texto que había ahí, y ratatui no lo volvía a escribir porque lo creía dibujado.
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
