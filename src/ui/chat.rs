//! La conversación: filas de mensajes y herramientas, con su markdown, y cómo se pinta en un panel.

use super::*;

pub(in crate::ui) fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let block = panel("CONVERSACIÓN");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let width = inner.width.saturating_sub(4).max(10) as usize;
    let rows = chat_rows(app, width, Voz::Clasico);
    render_chat(f, inner, app, rows);
    let scroll = app.scroll.min(app.view.borrow().rows.len().saturating_sub(inner.height as usize));
    if scroll > 0 {
        let tag = Span::styled(format!(" ↑ {scroll} líneas · End para volver "), Style::new().fg(Color::Black).bg(theme::dim()));
        let r = Rect { y: area.y + area.height - 1, height: 1, ..area };
        f.render_widget(Paragraph::new(tag).alignment(Alignment::Right), r);
    }
}

/// Cómo se rotula quién habla en la conversación.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::ui) enum Voz {
    /// «TÚ» / «JARVIS» en su propia fila.
    Clasico,
    /// Un margen de 10 columnas con el nombre alineado a la derecha: un guion.
    Guion,
    /// «▸ TÚ» / «◆ JARVIS», como un canal de radio.
    Canal,
}

pub(in crate::ui) const GUION: usize = 10;

/// Las filas de la conversación. Cada una lleva cuántas columnas de decoración tiene al
/// inicio y si sigue a la anterior por el ajuste de línea: al copiar se quita lo primero y
/// se vuelven a unir los párrafos.
pub(in crate::ui) fn chat_rows(app: &App, width: usize, voz: Voz) -> Vec<(Line<'static>, usize, bool)> {
    let mut rows: Vec<(Line<'static>, usize, bool)> = Vec::new();
    let blank = || (Line::default(), 0, false);
    let pad = if voz == Voz::Guion { GUION } else { 2 };
    // En el guion, el margen se come columnas del texto.
    let width = if voz == Voz::Guion { width.saturating_sub(GUION - 2).max(10) } else { width };
    if app.messages.is_empty() && voz != Voz::Guion {
        rows.push(blank());
        rows.push((
            Line::from(Span::styled(format!("  {}. ¿En qué trabajamos?", greeting()), Style::new().fg(theme::text()).bold())),
            2,
            false,
        ));
        rows.push((
            Line::from(Span::styled("  Escribe, o mantén espacio y háblame.", Style::new().fg(theme::faint()))),
            2,
            false,
        ));
    }
    let gutter = |name: &str, st: Style| Span::styled(format!("{name:>w$}  ", w = GUION - 2), st);
    let indent = " ".repeat(pad);

    for (i, m) in app.messages.iter().enumerate() {
        match m.role {
            Role::User => {
                rows.push(blank());
                let fg = if m.waiting.is_some() { theme::faint() } else { theme::text() };
                let who = Style::new().fg(if voz == Voz::Clasico { WARM } else { theme::text() }).bold();
                let lines = wrap_cont(&m.text, width);
                match voz {
                    Voz::Guion => {
                        for (k, (l, cont)) in lines.into_iter().enumerate() {
                            let head = if k == 0 { gutter("tú", who) } else { Span::raw(" ".repeat(GUION)) };
                            rows.push((Line::from(vec![head, Span::styled(l, Style::new().fg(fg))]), GUION, cont));
                        }
                        if m.waiting.is_some() {
                            rows.push((
                                Line::from(vec![Span::raw(" ".repeat(GUION)), Span::styled("◷ en cola · lo lee al terminar", Style::new().fg(theme::dim()))]),
                                GUION,
                                false,
                            ));
                        }
                    }
                    _ => {
                        let name = if voz == Voz::Canal { "  ▸ TÚ" } else { "  TÚ" };
                        let mut head = vec![Span::styled(name, who)];
                        if m.waiting.is_some() {
                            head.push(Span::styled("  ◷ en espera", Style::new().fg(theme::faint())));
                            head.push(Span::styled(" — Claude lo lee al terminar lo que está haciendo", Style::new().fg(theme::dim())));
                        }
                        rows.push((Line::from(head), 2, false));
                        for (l, cont) in lines {
                            rows.push((Line::from(Span::styled(format!("{indent}{l}"), Style::new().fg(fg))), 2, cont));
                        }
                    }
                }
                // Las imágenes que mandaste: se reserva su lugar y la app las dibuja encima.
                if let Some(cell) = app.sixel_cell {
                    let x = if voz == Voz::Guion { GUION } else { pad } as u16;
                    for t in &m.images {
                        let (cols, h) = t.cells(cell, (width as u16).saturating_sub(2));
                        rows.push(blank());
                        app.thumb_slots.borrow_mut().push((rows.len(), x, cols, h, t.clone()));
                        for _ in 0..h {
                            rows.push(blank());
                        }
                    }
                }
            }
            Role::Assistant => {
                rows.push(blank());
                let who = Style::new().fg(theme::accent()).bold();
                let md = app.md_rows(i, &m.text, width);
                match voz {
                    Voz::Guion => {
                        // Las filas del markdown traen su sangría de 2; delante va el margen.
                        for (k, (line, skip, cont)) in md.into_iter().enumerate() {
                            let head = if k == 0 {
                                Span::styled(format!("{:>w$}", "jarvis", w = GUION - 2), who)
                            } else {
                                Span::raw(" ".repeat(GUION - 2))
                            };
                            let mut spans = vec![head];
                            spans.extend(line.spans);
                            // Las filas de pura decoración marcan `usize::MAX`: siguen sin copiarse.
                            rows.push((Line::from(spans), skip.saturating_add(GUION - 2), cont));
                        }
                    }
                    _ => {
                        let name = if voz == Voz::Canal { "  ◆ JARVIS" } else { "  JARVIS" };
                        rows.push((Line::from(Span::styled(name, who)), 2, false));
                        rows.extend(md);
                    }
                }
                // Los archivos que entrega: un clic los copia al portapapeles.
                let adjuntos = app.adjuntos(i, &m.text);
                if !adjuntos.is_empty() {
                    rows.push(blank());
                }
                let x = if voz == Voz::Guion { GUION } else { pad };
                for a in adjuntos {
                    let label = truncate(&format!(" ⎘ {} ", a.label()), width.saturating_sub(18).max(8));
                    let w = label.chars().count() + 1 + "clic para copiar".len();
                    let line = Line::from(vec![
                        Span::raw(" ".repeat(x)),
                        Span::styled(label, Style::new().fg(Color::Black).bg(theme::accent()).bold()),
                        Span::styled(" clic para copiar", Style::new().fg(theme::faint())),
                    ]);
                    app.file_slots.borrow_mut().push((rows.len(), x as u16, w as u16, a));
                    rows.push((line, usize::MAX, false));
                }
            }
            Role::System => {
                for (k, (l, cont)) in wrap_cont(&m.text, width).into_iter().enumerate() {
                    let mark = if k == 0 { "·" } else { " " };
                    rows.push((Line::from(Span::styled(format!("{indent}{mark} {l}"), Style::new().fg(theme::faint()))), pad + 2, cont));
                }
            }
            Role::Error => {
                for (l, cont) in wrap_cont(&m.text, width) {
                    rows.push((Line::from(Span::styled(format!("{indent}! {l}"), Style::new().fg(RED))), pad + 2, cont));
                }
            }
            Role::Tool => {
                let Some(a) = m.tool.and_then(|t| app.activity.get(t)) else { continue };
                rows.extend(tool_rows(a, &indent, pad, width));
            }
        }
    }
    rows
}

/// Una herramienta en el chat: su línea (estado, nombre, qué hizo, cuánto tardó) y, para
/// Bash, las primeras líneas de la salida. El detalle completo, con Ctrl+G.
pub(in crate::ui) fn tool_rows(a: &crate::Activity, indent: &str, pad: usize, width: usize) -> Vec<(Line<'static>, usize, bool)> {
    use crate::ToolStatus;
    let (icon, col) = match a.status {
        ToolStatus::Running => ("◐", WARM),
        ToolStatus::Ok => ("✓", GREEN),
        ToolStatus::Err => ("✗", RED),
    };
    let secs = a.took.unwrap_or_else(|| a.started.elapsed()).as_secs_f64();
    let what = tool_summary(a);
    let room = width.saturating_sub(a.name.chars().count() + 12);
    let mut out = vec![(
        Line::from(vec![
            Span::raw(format!("{indent}")),
            Span::styled(format!("{icon} "), Style::new().fg(col)),
            Span::styled(a.name.clone(), Style::new().fg(theme::text()).bold()),
            Span::styled(format!("  {}", truncate(&what, room)), Style::new().fg(theme::faint())),
            Span::styled(format!("  {secs:.1}s"), Style::new().fg(theme::dim())),
        ]),
        pad,
        false,
    )];
    // Bash: un vistazo a la salida (o al error).
    if a.name == "Bash" && !a.output.trim().is_empty() {
        let lines: Vec<&str> = a.output.lines().filter(|l| !l.trim().is_empty()).collect();
        for (k, l) in lines.iter().take(3).enumerate() {
            let mark = if k == 0 { "⎿ " } else { "  " };
            let st = Style::new().fg(if a.status == ToolStatus::Err { RED } else { theme::dim() });
            out.push((Line::from(Span::styled(format!("{indent}  {mark}{}", truncate(l, width.saturating_sub(6))), st)), pad + 4, false));
        }
        if lines.len() > 3 {
            out.push((
                Line::from(Span::styled(format!("{indent}    … {} líneas más · ctrl+g", lines.len() - 3), Style::new().fg(theme::dim()))),
                usize::MAX,
                false,
            ));
        }
    }
    out
}

/// Lo que hizo una herramienta, en una línea: el comando, el archivo y el cambio, lo buscado…
pub(crate) fn tool_summary(a: &crate::Activity) -> String {
    let s = |k: &str| a.input.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let home = std::env::var("HOME").unwrap_or_default();
    let short = |p: String| if home.is_empty() { p } else { p.replace(&home, "~") };
    match a.name.as_str() {
        "Bash" => s("command").lines().next().unwrap_or("").to_string(),
        "Edit" | "MultiEdit" => {
            let (minus, plus) = (s("old_string").lines().count(), s("new_string").lines().count());
            format!("{}  −{minus} +{plus}", short(s("file_path")))
        }
        "Write" => format!("{}  {} líneas", short(s("file_path")), s("content").lines().count()),
        "Read" => short(s("file_path")),
        "Grep" => format!("«{}» {}", s("pattern"), short(s("path"))),
        "Glob" => s("pattern"),
        "WebFetch" => s("url"),
        "WebSearch" => s("query"),
        _ => a.detail.clone(),
    }
}

/// Dibuja las filas que caben (respetando el desplazamiento), pinta la selección y deja la
/// vista para que el mouse sepa qué texto hay debajo.
pub(in crate::ui) fn render_chat(f: &mut Frame, inner: Rect, app: &App, rows: Vec<(Line<'static>, usize, bool)>) {
    let h = inner.height as usize;
    let max_scroll = rows.len().saturating_sub(h);
    let scroll = app.scroll.min(max_scroll);
    let start = rows.len().saturating_sub(h + scroll);
    let mut view = crate::select::View { area: inner, rows: Vec::with_capacity(rows.len()), first: start };
    let mut visible = Vec::with_capacity(h);
    for (i, (line, skip, cont)) in rows.into_iter().enumerate() {
        let text = line.spans.iter().map(|s| s.content.as_ref()).collect();
        view.rows.push(crate::select::Row { text, skip, cont });
        if i >= start && visible.len() < h {
            visible.push(line);
        }
    }
    f.render_widget(Paragraph::new(visible), inner);

    // Las miniaturas que entran enteras en lo visible (las cortadas se ven al desplazar).
    // Con un menú o la lista de comandos encima, ninguna: se pintarían sobre el menú.
    let menu = app.modal.is_some() || app.tools_view.is_some() || !crate::commands::matches(&app.commands, &app.input).is_empty();
    if !menu {
        let end = start + h_rows(inner);
        for (row, x, cols, h, t) in app.thumb_slots.borrow().iter() {
            // La parte de la imagen que cae en lo visible (puede estar cortada arriba o abajo).
            let (top, bottom) = ((*row).max(start), (row + *h as usize).min(end));
            if top < bottom {
                let r = Rect { x: inner.x + x, y: inner.y + (top - start) as u16, width: *cols, height: (bottom - top) as u16 };
                let crop = ((top - row) as u16, (bottom - row) as u16, *h);
                app.thumb_targets.borrow_mut().push((r, t.clone(), crop));
            }
        }
    }

    for (row, x, w, a) in app.file_slots.borrow().iter() {
        if *row >= start && *row < start + h {
            let r = Rect { x: inner.x + x, y: inner.y + (row - start) as u16, width: (*w).min(inner.width.saturating_sub(*x)), height: 1 };
            app.file_targets.borrow_mut().push((r, a.clone()));
        }
    }

    if let Some(sel) = &app.sel {
        let buf = f.buffer_mut();
        for (x, y) in sel.cells(&view) {
            buf[(x, y)].set_bg(theme::dim()).set_fg(Color::White);
        }
    }
    *app.view.borrow_mut() = view;
}

/// Markdown aplanado para leerlo en una línea: sin asteriscos, comillas invertidas ni almohadillas.
pub(in crate::ui) fn plain(s: &str) -> String {
    // Los bloques de código no se leen en voz alta: se van enteros.
    let mut code = false;
    s.lines()
        .filter(|l| {
            if l.trim_start().starts_with("```") {
                code = !code;
                return false;
            }
            !code
        })
        .map(|l| l.trim_start_matches('#').trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        // Negrita y cursiva (** y *); los guiones bajos se quedan: son parte de nombres como ghub_palette.
        .replace('*', "")
        .replace('`', "")
}
