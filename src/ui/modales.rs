//! Las listas que se despliegan sobre la orden: comandos, sesiones, modelo, estilo, buddy, color,
//! actualización y las preguntas de Claude.

use super::*;

/// Lo que se superpone sobre la orden: preguntas, listas o el menú de comandos.
pub(in crate::ui) fn draw_modal(f: &mut Frame, anchor: Rect, app: &App) {
    match &app.modal {
        Some(Modal::Sessions { list, sel }) => draw_sessions(f, anchor, list, *sel),
        Some(Modal::Ask(a)) => draw_ask(f, anchor, a),
        Some(Modal::Model { sel }) => draw_models(f, anchor, app, *sel),
        Some(Modal::Effort { sel }) => draw_efforts(f, anchor, app, *sel),
        Some(Modal::Estilo { sel, .. }) => draw_estilos(f, anchor, *sel),
        Some(Modal::Color { sel, .. }) => draw_colores(f, anchor, *sel),
        Some(Modal::Buddy { sel, .. }) => draw_buddies(f, anchor, *sel),
        Some(Modal::Update(info)) => draw_update(f, anchor, info),
        None => draw_menu(f, anchor, app),
    }
}

/// Panel que se despliega hacia arriba desde la orden, tapando el final de la conversación.
/// `head` queda fijo arriba; `items` se desplaza para que `sel` siempre se vea.
pub(in crate::ui) fn overlay(
    f: &mut Frame,
    anchor: Rect,
    title: &str,
    color: Color,
    head: Vec<Line<'static>>,
    items: Vec<Line<'static>>,
    sel: usize,
    max_rows: usize,
    hints: &[(&'static str, &'static str)],
) {
    let rows = head.len() + items.len().min(max_rows);
    let h = (rows as u16 + 2).min(anchor.y);
    let area = Rect { y: anchor.y - h, height: h, ..anchor };
    let mut foot = vec![];
    for (k, d) in hints {
        foot.push(Span::styled(format!(" {k} "), Style::new().fg(color)));
        foot.push(Span::styled(format!("{d} "), Style::new().fg(theme::faint())));
    }
    let block = panel(title).border_style(Style::new().fg(color)).title_bottom(Line::from(foot));
    let inner = block.inner(area);
    limpiar(f, area);
    f.render_widget(block, area);

    let room = (inner.height as usize).saturating_sub(head.len());
    let first = (sel + 1).saturating_sub(room);
    let mut lines = head;
    lines.extend(items.into_iter().skip(first).take(room));
    f.render_widget(Paragraph::new(lines), inner);
}

/// Una fila elegible: marca, nombre a ancho fijo y una descripción que se recorta.
pub(in crate::ui) fn item(on: bool, mark: &str, name: &str, name_w: usize, desc: &str, w: usize) -> Line<'static> {
    let bg = if on { Style::new().bg(theme::seleccion()) } else { Style::new() };
    let cursor = if on { " › " } else { "   " };
    let name = format!("{mark}{:<name_w$}", truncate(name, name_w));
    let room = w.saturating_sub(3 + name.chars().count() + 2);
    Line::from(vec![
        Span::styled(cursor, bg.fg(theme::accent()).bold()),
        Span::styled(name, bg.fg(if on { Color::White } else { theme::accent() }).bold()),
        Span::styled(format!("  {:<room$}", truncate(desc, room)), bg.fg(if on { theme::text() } else { theme::faint() })),
    ])
}

pub(in crate::ui) fn draw_menu(f: &mut Frame, anchor: Rect, app: &App) {
    let cmds = crate::commands::matches(&app.commands, &app.input);
    if cmds.is_empty() {
        return;
    }
    let sel = app.menu.min(cmds.len() - 1);
    let name_w = cmds.iter().map(|c| c.name.chars().count()).max().unwrap_or(0).min(28) + 1;
    let w = anchor.width.saturating_sub(2) as usize;
    let items = cmds
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let desc = if c.local { format!("iris {}", c.desc) } else { c.desc.clone() };
            item(i == sel, "/", &c.name, name_w, &desc, w)
        })
        .collect();
    let hints = [("↑↓", "elegir"), ("tab", "completar"), ("enter", "ejecutar")];
    overlay(f, anchor, "COMANDOS", theme::accent(), vec![], items, sel, 8, &hints);
}

pub(in crate::ui) fn draw_sessions(f: &mut Frame, anchor: Rect, list: &[crate::sessions::Session], sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = list
        .iter()
        .enumerate()
        .map(|(i, s)| item(i == sel, "", &s.age, 11, &format!("{}  {}", &s.id[..8], s.title), w))
        .collect();
    let hints = [("↑↓", "elegir"), ("enter", "retomar"), ("esc", "cerrar")];
    overlay(f, anchor, "SESIONES", theme::accent(), vec![], items, sel, 12, &hints);
}

pub(in crate::ui) fn draw_models(f: &mut Frame, anchor: Rect, app: &App, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = crate::commands::MODELS
        .iter()
        .enumerate()
        .map(|(i, (id, name, what))| {
            let now = if *id == app.model { "en uso" } else { "" };
            let desc = [*id, now, what].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · ");
            item(i == sel, "", name, 10, &desc, w)
        })
        .collect();
    let hints = [("↑↓", "elegir"), ("enter", "cambiar"), ("esc", "cerrar")];
    overlay(f, anchor, "MODELO", theme::accent(), vec![], items, sel, 8, &hints);
}

pub(in crate::ui) fn draw_efforts(f: &mut Frame, anchor: Rect, app: &App, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = crate::commands::EFFORTS
        .iter()
        .enumerate()
        .map(|(i, (level, what))| {
            let now = if *level == app.effort { "en uso" } else { "" };
            let desc = [now, what].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · ");
            item(i == sel, "", level, 10, &desc, w)
        })
        .collect();
    let hints = [("↑↓", "elegir"), ("enter", "cambiar"), ("esc", "cerrar")];
    overlay(f, anchor, "EFFORT", theme::accent(), vec![], items, sel, 8, &hints);
}

pub(in crate::ui) fn draw_estilos(f: &mut Frame, anchor: Rect, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = crate::estilo::TODOS
        .iter()
        .enumerate()
        .map(|(i, (_, _, name, what))| item(i == sel, &format!("{} ", i + 1), name, 10, what, w))
        .collect();
    let hints = [("↑↓", "probar"), ("enter", "usar"), ("esc", "volver")];
    overlay(f, anchor, "ESTILO", theme::accent(), vec![], items, sel, 8, &hints);
}

pub(in crate::ui) fn draw_buddies(f: &mut Frame, anchor: Rect, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = crate::buddy::TODOS
        .iter()
        .enumerate()
        .map(|(i, (_, _, name, what))| item(i == sel, &format!("{} ", i + 1), name, 10, what, w))
        .collect();
    let hints = [("↑↓", "probar"), ("enter", "usar"), ("esc", "volver")];
    overlay(f, anchor, "BUDDY", theme::accent(), vec![], items, sel, 8, &hints);
}

pub(in crate::ui) fn draw_colores(f: &mut Frame, anchor: Rect, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let acento = theme::acento_u32();
    let items = crate::baymax::OPCIONES
        .iter()
        .enumerate()
        .map(|(i, (_, nombre, m))| {
            let mut l = item(i == sel, &format!("{} ", i + 1), nombre, 22, "", w.saturating_sub(3));
            // La muestra del color, entre la marca y el nombre.
            let c = m.color(acento);
            l.spans.insert(1, Span::styled("██ ", Style::new().fg(Color::Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8))));
            l
        })
        .collect();
    let hints = [("↑↓", "probar"), ("enter", "usar"), ("esc", "volver")];
    overlay(f, anchor, "NÚCLEO BAYMAX", theme::accent(), vec![], items, sel, 8, &hints);
}

pub(in crate::ui) fn draw_update(f: &mut Frame, anchor: Rect, info: &crate::update::Info) {
    let w = anchor.width.saturating_sub(2) as usize;
    let what = format!("Iris {} está disponible (tienes la {}).", info.version, env!("CARGO_PKG_VERSION"));
    let mut head = vec![];
    for l in wrap(&what, w.saturating_sub(2)) {
        head.push(Line::from(Span::styled(format!(" {l}"), Style::new().fg(theme::text()).bold())));
    }
    for l in wrap("Se baja de iris.knarvaez.com, se instala e Iris se vuelve a abrir en esta misma sesión.", w.saturating_sub(2)) {
        head.push(Line::from(Span::styled(format!(" {l}"), Style::new().fg(theme::faint()))));
    }
    if !info.notas.is_empty() {
        head.push(Line::default());
    }
    let items = info
        .notas
        .iter()
        .map(|c| Line::from(Span::styled(format!("  · {}", truncate(c, w.saturating_sub(4))), Style::new().fg(theme::text()))))
        .collect();
    let hints = [("enter", "actualizar"), ("esc", "ahora no")];
    overlay(f, anchor, "ACTUALIZACIÓN", theme::accent(), head, items, 0, 8, &hints);
}

pub(in crate::ui) fn draw_ask(f: &mut Frame, anchor: Rect, a: &crate::ask::Ask) {
    let q = a.current();
    let w = anchor.width.saturating_sub(2) as usize;
    let mut head = vec![];
    let step = if a.questions.len() > 1 { format!("  {}/{}", a.step + 1, a.questions.len()) } else { String::new() };
    head.push(Line::from(vec![
        Span::styled(format!(" {} ", q.header.to_uppercase()), Style::new().fg(Color::Black).bg(WARM).bold()),
        Span::styled(step, Style::new().fg(theme::faint())),
    ]));
    for l in wrap(&q.text, w.saturating_sub(2)) {
        head.push(Line::from(Span::styled(format!(" {l}"), Style::new().fg(theme::text()).bold())));
    }
    head.push(Line::default());

    let name_w = q.options.iter().map(|o| o.0.chars().count()).max().unwrap_or(0).min(30);
    let items = q
        .options
        .iter()
        .enumerate()
        .map(|(i, (label, desc))| {
            let mark = match (q.multi, a.checked.get(i)) {
                (true, Some(true)) => "◉ ",
                (true, _) => "○ ",
                _ => "",
            };
            item(i == a.sel, mark, label, name_w, desc, w)
        })
        .collect();
    let hints: &[(&str, &str)] = if q.multi {
        &[("↑↓", "elegir"), ("espacio", "marcar"), ("enter", "responder"), ("o escribe", "otra cosa"), ("esc", "descartar")]
    } else {
        &[("↑↓", "elegir"), ("enter", "responder"), ("o escribe/habla", "otra cosa"), ("esc", "descartar")]
    };
    let title = if a.permission { "PERMISO" } else { "PREGUNTA" };
    overlay(f, anchor, title, WARM, head, items, a.sel, 8, hints);
}
