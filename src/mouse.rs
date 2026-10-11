//! The mouse: wheel, clicks on attachments, thumbnails and links, and dragging to copy from the chat.

use crate::*;

pub(crate) fn on_mouse(app: &mut App, m: crossterm::event::MouseEvent) {
    let at = (m.column, m.row);
    let area = app.view.borrow().area;
    let inside = area.contains(ratatui::layout::Position::new(m.column, m.row));
    let dragging = app.sel.as_ref().is_some_and(|s| s.pointer.is_some());
    match m.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let hit = app.file_targets.borrow().iter().find(|(r, _)| r.contains(ratatui::layout::Position::new(m.column, m.row))).map(|(_, a)| a.clone());
            if let Some(a) = hit {
                app.sel = None;
                let msg = if attachment::copy(&a) {
                    app.core.fire(Gesture::Copy);
                    "archivo copiado · pégalo donde quieras".to_string()
                } else {
                    "no pude copiar el archivo al portapapeles".to_string()
                };
                app.flash = Some((msg, Instant::now()));
                return;
            }
            app.sel = inside.then(|| {
                let p = app.view.borrow().locate(at);
                select::Selection { anchor: p, head: p, pointer: Some(at) }
            });
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(s) = &mut app.sel {
                s.pointer = Some(at);
                s.head = app.view.borrow().locate(at);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            let Some(s) = &mut app.sel else { return };
            s.pointer = None;
            if s.is_empty() {
                app.sel = None;
                // A click without dragging on a URL opens it in the browser.
                open_link(app, at);
                return;
            }
            // It stays highlighted until the next click or key, to see what was copied.
            let text = s.text(&app.view.borrow());
            copied(app, &text);
        }
        // With the wheel the selection can keep stretching beyond what is visible.
        MouseEventKind::ScrollUp if app.reading.get() => scroll(app, true, 3),
        MouseEventKind::ScrollDown if app.reading.get() => scroll(app, false, 3),
        MouseEventKind::ScrollUp if inside || dragging => {
            app.scroll = (app.scroll + 3).min(app.view.borrow().max_scroll());
        }
        MouseEventKind::ScrollDown if inside || dragging => app.scroll = app.scroll.saturating_sub(3),
        _ => {}
    }
}

/// Every frame, while dragging: the tip follows the pointer over the text that is now
/// underneath, and dragging above or below the chat scrolls it on its own.
pub(crate) fn follow_drag(app: &mut App) {
    let Some(p) = app.sel.as_ref().and_then(|s| s.pointer) else { return };
    let area = app.view.borrow().area;
    if p.1 < area.y {
        app.scroll = (app.scroll + 1).min(app.view.borrow().max_scroll());
    } else if p.1 >= area.bottom() {
        app.scroll = app.scroll.saturating_sub(1);
    }
    let head = app.view.borrow().locate(p);
    if let Some(s) = &mut app.sel {
        s.head = head;
    }
}

/// `/copy`: the whole last reply, with its markdown.
pub(crate) fn copy_last(app: &mut App) {
    match app.messages.iter().rev().find(|m| m.role == Role::Assistant) {
        Some(m) => {
            let text = m.text.clone();
            copied(app, &text);
        }
        None => app.flash = Some(("todavía no hay respuesta que copiar".into(), Instant::now())),
    }
}

pub(crate) fn copied(app: &mut App, text: &str) {
    let n = text.chars().count();
    let msg = if text.is_empty() {
        "nada que copiar ahí".to_string()
    } else if select::copy(text) {
        app.core.fire(Gesture::Copy);
        format!("copiado · {n} caracteres")
    } else {
        "no pude copiar al portapapeles".to_string()
    };
    app.flash = Some((msg, Instant::now()));
}

/// If there is a conversation URL under the pointer, opens it in the browser.
fn open_link(app: &mut App, at: (u16, u16)) {
    let urls: Vec<String> = app.messages.iter().flat_map(|m| link::urls(&m.text)).collect();
    if urls.is_empty() {
        return;
    }
    let view = app.view.borrow();
    if !view.area.contains(ratatui::layout::Position::new(at.0, at.1)) {
        return;
    }
    let Some(url) = link::under(&view, view.locate(at), &urls) else { return };
    drop(view);
    let msg = if link::open(&url) {
        format!("abriendo {}", link::domain(&url))
    } else {
        "no pude abrir el enlace".to_string()
    };
    app.flash = Some((msg, Instant::now()));
}
