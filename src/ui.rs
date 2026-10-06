
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::estilo::Estilo;
use crate::{theme, Activity, App, Modal, Role, ToolStatus, VoiceState};

const WARM: Color = Color::Rgb(255, 170, 40);
const RED: Color = Color::Rgb(255, 90, 90);
pub(crate) const GREEN: Color = Color::Rgb(90, 230, 150);
const SPIN: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub use crate::nucleo::State;

pub fn draw(f: &mut Frame, app: &App) {
    theme::usar(app.buddy.paleta());
    if let Some(bg) = theme::fondo() {
        f.render_widget(Block::new().style(Style::new().bg(bg)), f.area());
    }
    // Los estilos sin paneles necesitan aire; en una terminal chica se ve el clásico.
    let a = f.area();
    let fits = |w: u16, h: u16| a.width >= w && a.height >= h;
    match app.estilo {
        Estilo::Propuesta if fits(60, 16) => propuesta(f, app),
        Estilo::Cabina if fits(70, 18) => cabina(f, app),
        Estilo::Cine if fits(64, 22) => cine(f, app),
        Estilo::Fosforo if fits(70, 18) => fosforo(f, app),
        Estilo::Radar if fits(70, 20) => radar(f, app),
        Estilo::Editorial if fits(70, 20) => editorial(f, app),
        Estilo::Zen if fits(40, 12) => zen(f, app),
        Estilo::Bitacora if fits(60, 12) => bitacora(f, app),
        Estilo::Tablero if fits(80, 24) => tablero(f, app),
        _ => clasico(f, app),
    }
    if app.tools_view.is_some() {
        tools_overlay(f, app);
    }
}

/// Borra lo que haya debajo de una ventana; con paleta propia, deja su fondo y no el de la terminal.
fn limpiar(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
    if let Some(bg) = theme::fondo() {
        f.render_widget(Block::new().style(Style::new().bg(bg)), area);
    }
}

/// Ctrl+G: la lista de herramientas a la izquierda y la elegida entera a la derecha.
fn tools_overlay(f: &mut Frame, app: &App) {
    use crate::ToolStatus;
    let Some((sel, scroll)) = app.tools_view else { return };
    let a = f.area();
    let area = Rect { x: a.x + 2, y: a.y + 1, width: a.width.saturating_sub(4), height: a.height.saturating_sub(2) };
    limpiar(f, area);
    let block = panel("HERRAMIENTAS · ↑↓ elegir · pgup/pgdn recorrer · esc cierra");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let list_w = (inner.width / 3).clamp(28, 44);
    let [list, _, detail] =
        Layout::horizontal([Constraint::Length(list_w), Constraint::Length(2), Constraint::Min(20)]).areas(inner);

    // La lista, con la elegida siempre a la vista.
    let n = app.activity.len();
    let sel = sel.min(n.saturating_sub(1));
    let h = list.height as usize;
    let first = (sel + 1).saturating_sub(h);
    let rows: Vec<Line> = app
        .activity
        .iter()
        .enumerate()
        .skip(first)
        .take(h)
        .map(|(i, t)| {
            let (icon, col) = match t.status {
                ToolStatus::Running => ("◐", WARM),
                ToolStatus::Ok => ("✓", GREEN),
                ToolStatus::Err => ("✗", RED),
            };
            let secs = t.took.unwrap_or_else(|| t.started.elapsed()).as_secs_f64();
            let label = truncate(&format!("{} {}", t.name, tool_summary(t)), (list.width as usize).saturating_sub(10));
            let st = if i == sel { Style::new().fg(Color::Black).bg(theme::accent()) } else { Style::new().fg(theme::text()) };
            Line::from(vec![
                Span::styled(format!("{icon} "), if i == sel { st } else { Style::new().fg(col) }),
                Span::styled(format!("{label:<w$}", w = (list.width as usize).saturating_sub(9)), st),
                Span::styled(format!("{secs:>5.1}s"), if i == sel { st } else { Style::new().fg(theme::dim()) }),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(rows), list);

    // El detalle de la elegida.
    let Some(t) = app.activity.get(sel) else { return };
    let w = detail.width as usize;
    let mut lines: Vec<Line> = vec![];
    let head = |s: &str| Line::from(Span::styled(s.to_string(), Style::new().fg(theme::accent()).bold()));
    let (state, col) = match t.status {
        ToolStatus::Running => ("en curso", WARM),
        ToolStatus::Ok => ("terminó bien", GREEN),
        ToolStatus::Err => ("terminó con error", RED),
    };
    let secs = t.took.unwrap_or_else(|| t.started.elapsed()).as_secs_f64();
    lines.push(Line::from(vec![
        Span::styled(t.name.clone(), Style::new().fg(theme::text()).bold()),
        Span::styled(format!("  ·  {state}  ·  {secs:.1}s"), Style::new().fg(col)),
    ]));
    lines.push(Line::default());
    let push_wrapped = |lines: &mut Vec<Line>, text: &str, st: Style, prefix: &str| {
        for raw in text.lines() {
            let raw = raw.replace('\t', "    ");
            for piece in wrap(&raw, w.saturating_sub(prefix.chars().count()).max(10)) {
                lines.push(Line::from(Span::styled(format!("{prefix}{piece}"), st)));
            }
            if raw.is_empty() {
                lines.push(Line::from(Span::styled(prefix.to_string(), st)));
            }
        }
    };
    let s = |k: &str| t.input.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    lines.push(head("ENTRADA"));
    match t.name.as_str() {
        "Bash" => {
            if !s("description").is_empty() {
                push_wrapped(&mut lines, &s("description"), Style::new().fg(theme::faint()), "");
            }
            push_wrapped(&mut lines, &s("command"), Style::new().fg(GREEN), "$ ");
        }
        "Edit" | "MultiEdit" => {
            push_wrapped(&mut lines, &s("file_path"), Style::new().fg(theme::faint()), "");
            let edits: Vec<(String, String)> = match t.input.get("edits").and_then(|e| e.as_array()) {
                Some(es) => es
                    .iter()
                    .map(|e| {
                        let g = |k: &str| e.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        (g("old_string"), g("new_string"))
                    })
                    .collect(),
                None => vec![(s("old_string"), s("new_string"))],
            };
            for (old, new) in edits {
                lines.push(Line::default());
                push_wrapped(&mut lines, &old, Style::new().fg(RED), "- ");
                push_wrapped(&mut lines, &new, Style::new().fg(GREEN), "+ ");
            }
        }
        "Write" => {
            push_wrapped(&mut lines, &s("file_path"), Style::new().fg(theme::faint()), "");
            push_wrapped(&mut lines, &s("content"), Style::new().fg(GREEN), "+ ");
        }
        _ => {
            let pretty = serde_json::to_string_pretty(&t.input).unwrap_or_default();
            push_wrapped(&mut lines, &pretty, Style::new().fg(theme::faint()), "");
        }
    }
    lines.push(Line::default());
    lines.push(head("SALIDA"));
    if t.output.trim().is_empty() {
        let none = if t.status == ToolStatus::Running { "todavía corriendo…" } else { "(sin salida)" };
        lines.push(Line::from(Span::styled(none, Style::new().fg(theme::dim()))));
    } else {
        let st = Style::new().fg(if t.status == ToolStatus::Err { RED } else { theme::text() });
        push_wrapped(&mut lines, &t.output, st, "");
    }
    let dh = detail.height as usize;
    let max = lines.len().saturating_sub(dh);
    let top = scroll.min(max);
    let shown: Vec<Line> = lines.into_iter().skip(top).take(dh).collect();
    f.render_widget(Paragraph::new(shown), detail);
    if top < max {
        let tag = Span::styled(format!(" ↓ pgdn · {}% ", (top + dh) * 100 / (max + dh).max(1)), Style::new().fg(Color::Black).bg(theme::dim()));
        let r = Rect { y: detail.bottom().saturating_sub(1), height: 1, ..detail };
        f.render_widget(Paragraph::new(tag).alignment(Alignment::Right), r);
    }
}

fn listening(state: State) -> bool {
    matches!(state, State::Listening | State::NoVoice)
}

// ── Clásico ─────────────────────────────────────────────────────────────────────────────

fn clasico(f: &mut Frame, app: &App) {
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

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme::dim()))
        .title(Span::styled(format!(" {title} "), Style::new().fg(theme::accent()).bold()))
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let model = app.model.trim_start_matches("claude-");
    let sess = app.session.get(..8).unwrap_or("········");
    let left = Line::from(vec![
        Span::styled(" ◆ J.A.R.V.I.S ", Style::new().fg(Color::Black).bg(theme::accent()).bold()),
        Span::styled(format!("  {model}"), Style::new().fg(theme::text())),
        Span::styled(format!("  ·  {}  ·  sesión {sess}", cwd()), Style::new().fg(theme::faint())),
    ]);
    f.render_widget(Paragraph::new(left), area);
    let clock = Line::from(Span::styled(format!("{} ", now_hhmmss()), Style::new().fg(theme::accent())));
    f.render_widget(Paragraph::new(clock).alignment(Alignment::Right), area);
}

fn draw_core(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("NÚCLEO");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    let [ring, label] = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(inner);
    core_draw(app, ring, f.buffer_mut());
    let (title, detail) = core_label(app, t, label.width as usize);
    let lines = vec![
        Line::from(Span::styled(title, Style::new().fg(app.nucleo.color()).bold())),
        Line::from(Span::styled(detail, Style::new().fg(theme::faint()))),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), label);
}

/// El rótulo del núcleo: el estado que se ve (no el pedido, así no se adelanta a la
/// animación) y un detalle de lo que pasa.
fn core_label(app: &App, t: f64, width: usize) -> (String, String) {
    let state = app.nucleo.state();
    let detail = match state {
        State::Planning if app.todos.0 > 0 => format!("{}/{} tareas", app.todos.1, app.todos.0),
        s if app.current_tool().is_some() && matches!(s,
            State::Searching | State::Reading | State::Editing | State::Running | State::Testing
            | State::Git | State::Web | State::Delegating | State::Planning) =>
        {
            let a = app.current_tool().unwrap();
            let room = width.saturating_sub(a.name.len() + 10);
            let what = truncate(&a.detail, room);
            let secs = a.started.elapsed().as_secs_f64();
            if what.is_empty() { format!("{} · {secs:.0}s", a.name) } else { format!("{} · {what} · {secs:.0}s", a.name) }
        }
        State::Listening if app.ptt.is_some() => "suelta espacio para enviar · esc cancela".into(),
        State::Listening => "espacio para enviar · esc cancela".into(),
        State::NoVoice => "no llega tu voz · ¿el micrófono correcto?".into(),
        State::Asking => "elige con ↑↓ y enter".into(),
        State::Compacting => "resumiendo la conversación".into(),
        State::Sleeping => "toca una tecla o espacio para hablar".into(),
        State::Booting | State::Idle if app.voice == VoiceState::Loading => "cargando voz…".into(),
        State::Idle if app.voice == VoiceState::Ready => "mantén o toca espacio para hablar".into(),
        _ => String::new(),
    };
    let dots = if matches!(state, State::Idle | State::Offline | State::Sleeping) {
        ""
    } else {
        ["   ", ".  ", ".. ", "..."][(t * 3.0) as usize % 4]
    };
    (format!("{}{dots}", state.label()), detail)
}

fn act_icon(a: &Activity, t: f64) -> (&'static str, Color) {
    match a.status {
        ToolStatus::Running => (SPIN[(t * 8.0) as usize % 4], WARM),
        ToolStatus::Ok => ("✓", GREEN),
        ToolStatus::Err => ("✗", RED),
    }
}

fn act_secs(a: &Activity) -> f64 {
    a.took.unwrap_or_else(|| a.started.elapsed()).as_secs_f64()
}

fn draw_activity(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("ACTIVIDAD");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width as usize;

    // Arriba, los subagentes: qué es cada hijo del núcleo y qué está haciendo.
    let mut lines: Vec<Line> = app
        .agents
        .iter()
        .map(|g| {
            let (icon, c) = match g.ended {
                None => ("◉", app.nucleo.color()),
                Some((_, true)) => ("✓", GREEN),
                Some((_, false)) => ("✗", RED),
            };
            let tool = g.current.as_ref().map(|(n, d)| (n, d)).or(g.last.as_ref().map(|(n, d, _)| (n, d)).filter(|_| g.state() != State::Thinking));
            let what = match (&g.ended, tool) {
                (Some(_), _) => String::new(),
                (None, Some((name, detail))) => format!("{} · {name} {detail}", g.state().label().to_lowercase()),
                (None, None) => g.state().label().to_lowercase(),
            };
            let time = format!(" {:.0}s", g.ended.map_or(g.started.elapsed(), |(t, _)| t - g.started).as_secs_f64());
            let desc = format!("{} ", truncate(&g.description, w.saturating_sub(time.len() + 6)));
            let room = w.saturating_sub(desc.chars().count() + 2 + time.len());
            Line::from(vec![
                Span::styled(format!("{icon} "), Style::new().fg(c)),
                Span::styled(desc, Style::new().fg(theme::text()).bold()),
                Span::styled(format!("{:<room$}", truncate(&what, room)), Style::new().fg(theme::faint())),
                Span::styled(time, Style::new().fg(theme::dim())),
            ])
        })
        .collect();
    let room = (inner.height as usize).saturating_sub(lines.len());

    lines.extend(app
        .activity
        .iter()
        .rev()
        .take(room)
        .map(|a| {
            let (icon, c) = act_icon(a, t);
            let time = format!(" {:.1}s", act_secs(a));
            let head = format!("{icon} {} ", a.name);
            let room = w.saturating_sub(head.chars().count() + time.len());
            Line::from(vec![
                Span::styled(format!("{icon} "), Style::new().fg(c)),
                Span::styled(format!("{} ", a.name), Style::new().fg(theme::text()).bold()),
                Span::styled(format!("{:<room$}", truncate(&a.detail, room)), Style::new().fg(theme::faint())),
                Span::styled(time, Style::new().fg(theme::dim())),
            ])
        }));

    if lines.is_empty() {
        let p = Paragraph::new(Span::styled("sin actividad todavía", Style::new().fg(theme::dim())));
        f.render_widget(p, inner);
    } else {
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
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
enum Voz {
    /// «TÚ» / «JARVIS» en su propia fila.
    Clasico,
    /// Un margen de 10 columnas con el nombre alineado a la derecha: un guion.
    Guion,
    /// «▸ TÚ» / «◆ JARVIS», como un canal de radio.
    Canal,
}

const GUION: usize = 10;

/// Las filas de la conversación. Cada una lleva cuántas columnas de decoración tiene al
/// inicio y si sigue a la anterior por el ajuste de línea: al copiar se quita lo primero y
/// se vuelven a unir los párrafos.
fn chat_rows(app: &App, width: usize, voz: Voz) -> Vec<(Line<'static>, usize, bool)> {
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
fn tool_rows(a: &crate::Activity, indent: &str, pad: usize, width: usize) -> Vec<(Line<'static>, usize, bool)> {
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
fn render_chat(f: &mut Frame, inner: Rect, app: &App, rows: Vec<(Line<'static>, usize, bool)>) {
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

    if let Some(sel) = &app.sel {
        let buf = f.buffer_mut();
        for (x, y) in sel.cells(&view) {
            buf[(x, y)].set_bg(theme::dim()).set_fg(Color::White);
        }
    }
    *app.view.borrow_mut() = view;
}

fn draw_wave(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("ESCUCHANDO").border_style(Style::new().fg(WARM));
    let inner = block.inner(area);
    f.render_widget(block, area);
    wave(f, inner, app, t, WARM, Color::Rgb(200, 120, 30));
}

/// Onda espejada: cada columna es un instante; la altura sale del RMS.
fn wave(f: &mut Frame, inner: Rect, app: &App, t: f64, mid_c: Color, edge_c: Color) {
    let w = inner.width as usize;
    let h = inner.height as usize;
    let hist = &app.levels[app.levels.len().saturating_sub(w)..];
    let bars = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mid = h as f64 / 2.0;
    let rows: Vec<Line> = (0..h)
        .map(|row| {
            let spans: Vec<Span> = hist
                .iter()
                .enumerate()
                .map(|(i, &l)| {
                    let shimmer = 0.015 * ((i as f64 * 0.35 + t * 6.0).sin() + 1.0);
                    let amp = ((l as f64 * 5.0).sqrt().min(1.0) + shimmer) * mid;
                    let dist = (row as f64 + 0.5 - mid).abs();
                    let fill = ((amp - dist + 0.5) * 8.0).clamp(0.0, 8.0) as usize;
                    let c = if dist < 0.6 { mid_c } else { edge_c };
                    Span::styled(bars[fill].to_string(), Style::new().fg(c))
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(rows), inner);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App, state: State, t: f64) {
    let border = if app.busy { theme::dim() } else { theme::accent() };
    let block = panel("ORDEN").border_style(Style::new().fg(border));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut spans = vec![Span::styled(" › ", Style::new().fg(theme::accent()).bold())];
    spans.extend(input_spans(app, state, t, (inner.width as usize).saturating_sub(4), theme::accent()));
    f.render_widget(Paragraph::new(Line::from(spans)), inner);
}

/// Lo escrito (o la pista, si no hay nada) con el cursor, en `w` columnas.
fn input_spans(app: &App, state: State, t: f64, w: usize, accent: Color) -> Vec<Span<'static>> {
    let cursor = if (t * 2.0) as usize % 2 == 0 { "▏" } else { " " };
    let mut spans = vec![];
    // Las imágenes adjuntas van como fichas delante del texto.
    let mut used = 0;
    for (i, img) in app.images.iter().enumerate() {
        let chip = format!(" ▣ {} ", img.label(i + 1));
        used += chip.chars().count() + 1;
        spans.push(Span::styled(chip, Style::new().fg(Color::Black).bg(accent)));
        spans.push(Span::raw(" "));
    }
    let w = w.saturating_sub(used + 1).max(4);
    // Los saltos de línea se ven como «↵»; se envían tal cual. La ventana de texto se corre
    // para que el cursor siempre quede a la vista.
    let chars: Vec<char> = app.input.chars().map(|c| if c == '\n' { '↵' } else { c }).collect();
    let cur = app.cur.min(chars.len());
    let start = if chars.len() < w { 0 } else { (cur + 1).saturating_sub(w).min(chars.len() + 1 - w) };
    let end = (start + w).min(chars.len());
    let before: String = chars[start..cur].iter().collect();
    let at: Option<char> = chars.get(cur).copied().filter(|_| cur < end);
    let after: String = chars.get(cur + 1..end).map(|c| c.iter().collect()).unwrap_or_default();
    if app.input.is_empty() && !app.images.is_empty() {
        spans.push(Span::styled(cursor, Style::new().fg(accent)));
        spans.push(Span::styled("escribe algo para acompañarla, o enter · ⌫ quita", Style::new().fg(theme::dim())));
    } else if app.input.is_empty() {
        spans.push(Span::styled(cursor, Style::new().fg(accent)));
        let hint = match (state, &app.voice) {
            (State::Transcribing, _) => "transcribiendo…",
            (State::Asking, _) => "elige arriba, o escribe aquí otra respuesta",
            (_, _) if app.busy => "trabajando — esc para interrumpir",
            (_, VoiceState::Ready) => "escribe, espacio para hablar, ^V pega una imagen",
            _ => "escribe una orden",
        };
        spans.push(Span::styled(hint, Style::new().fg(theme::dim())));
    } else {
        spans.push(Span::styled(before, Style::new().fg(theme::text())));
        match at {
            // En medio del texto: un bloque sobre la letra, fijo (si parpadeara, la letra se perdería).
            Some(c) => spans.push(Span::styled(c.to_string(), Style::new().fg(Color::Black).bg(accent))),
            None => spans.push(Span::styled(cursor, Style::new().fg(accent))),
        }
        spans.push(Span::styled(after, Style::new().fg(theme::text())));
    }
    spans
}

/// Lo que se superpone sobre la orden: preguntas, listas o el menú de comandos.
fn draw_modal(f: &mut Frame, anchor: Rect, app: &App) {
    match &app.modal {
        Some(Modal::Sessions { list, sel }) => draw_sessions(f, anchor, list, *sel),
        Some(Modal::Ask(a)) => draw_ask(f, anchor, a),
        Some(Modal::Model { sel }) => draw_models(f, anchor, app, *sel),
        Some(Modal::Estilo { sel, .. }) => draw_estilos(f, anchor, *sel),
        Some(Modal::Fondo { sel, .. }) => draw_fondos(f, anchor, *sel),
        Some(Modal::Color { sel, .. }) => draw_colores(f, anchor, *sel),
        Some(Modal::Buddy { sel, .. }) => draw_buddies(f, anchor, *sel),
        Some(Modal::Update(info)) => draw_update(f, anchor, info),
        None => draw_menu(f, anchor, app),
    }
}

/// Panel que se despliega hacia arriba desde la orden, tapando el final de la conversación.
/// `head` queda fijo arriba; `items` se desplaza para que `sel` siempre se vea.
fn overlay(
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
fn item(on: bool, mark: &str, name: &str, name_w: usize, desc: &str, w: usize) -> Line<'static> {
    let bg = if on { Style::new().bg(theme::seleccion()) } else { Style::new() };
    let cursor = if on { " › " } else { "   " };
    let name = format!("{mark}{:<name_w$}", truncate(name, name_w));
    let room = w.saturating_sub(3 + name.chars().count() + 2);
    Line::from(vec![
        Span::styled(cursor, bg.fg(theme::accent()).bold()),
        Span::styled(name, bg.fg(if !on { theme::accent() } else if theme::claro() { theme::text() } else { Color::White }).bold()),
        Span::styled(format!("  {:<room$}", truncate(desc, room)), bg.fg(if on { theme::text() } else { theme::faint() })),
    ])
}

fn draw_menu(f: &mut Frame, anchor: Rect, app: &App) {
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
            let desc = if c.local { format!("jarvis {}", c.desc) } else { c.desc.clone() };
            item(i == sel, "/", &c.name, name_w, &desc, w)
        })
        .collect();
    let hints = [("↑↓", "elegir"), ("tab", "completar"), ("enter", "ejecutar")];
    overlay(f, anchor, "COMANDOS", theme::accent(), vec![], items, sel, 8, &hints);
}

fn draw_sessions(f: &mut Frame, anchor: Rect, list: &[crate::sessions::Session], sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = list
        .iter()
        .enumerate()
        .map(|(i, s)| item(i == sel, "", &s.age, 11, &format!("{}  {}", &s.id[..8], s.title), w))
        .collect();
    let hints = [("↑↓", "elegir"), ("enter", "retomar"), ("esc", "cerrar")];
    overlay(f, anchor, "SESIONES", theme::accent(), vec![], items, sel, 12, &hints);
}

fn draw_models(f: &mut Frame, anchor: Rect, app: &App, sel: usize) {
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

fn draw_estilos(f: &mut Frame, anchor: Rect, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = crate::estilo::TODOS
        .iter()
        .enumerate()
        .map(|(i, (_, _, name, what))| item(i == sel, &format!("{} ", i + 1), name, 10, what, w))
        .collect();
    let hints = [("↑↓", "probar"), ("enter", "usar"), ("esc", "volver")];
    overlay(f, anchor, "ESTILO", theme::accent(), vec![], items, sel, 8, &hints);
}

fn draw_buddies(f: &mut Frame, anchor: Rect, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = crate::buddy::TODOS
        .iter()
        .enumerate()
        .map(|(i, (_, _, name, what))| item(i == sel, &format!("{} ", i + 1), name, 10, what, w))
        .collect();
    let hints = [("↑↓", "probar"), ("enter", "usar"), ("esc", "volver")];
    overlay(f, anchor, "BUDDY", theme::accent(), vec![], items, sel, 8, &hints);
}

fn draw_fondos(f: &mut Frame, anchor: Rect, sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    // Primero, los colores del sistema: sin muestra, que cambian con el tema.
    let sistema = [("Del sistema", "cambia con el tema de Omarchy", "◐  "), ("Del sistema, transparente", "se ve el fondo de pantalla", "◌  ")]
        .into_iter()
        .enumerate()
        .map(|(i, (name, desc, marca))| {
            let mut l = item(sel == i, &format!("{} ", i + 1), name, 26, desc, w.saturating_sub(3));
            l.spans.insert(1, Span::styled(marca, Style::new().fg(theme::accent())));
            l
        });
    let items = sistema
        .chain(crate::estilo::FONDOS.iter().enumerate().map(|(i, (_, name, c))| {
            let i = i + 2;
            let mut l = item(i == sel, &format!("{} ", i + 1), name, 26, &format!("#{c:06X}"), w.saturating_sub(3));
            // La muestra del color, entre la marca y el nombre.
            let muestra = Color::Rgb((c >> 16) as u8, (c >> 8) as u8, *c as u8);
            l.spans.insert(1, Span::styled("██ ", Style::new().fg(muestra)));
            l
        }))
        .collect();
    let hints = [("↑↓", "probar"), ("enter", "usar"), ("esc", "volver")];
    overlay(f, anchor, "FONDOS BAYMAX", theme::accent(), vec![], items, sel, 8, &hints);
}

fn draw_colores(f: &mut Frame, anchor: Rect, sel: usize) {
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

fn draw_update(f: &mut Frame, anchor: Rect, info: &crate::update::Info) {
    let w = anchor.width.saturating_sub(2) as usize;
    let what = match info.commits.len() {
        0 => format!("Hay una versión nueva en GitHub ({}).", info.commit),
        1 => format!("Hay una versión nueva en GitHub ({}): 1 cambio.", info.commit),
        n => format!("Hay una versión nueva en GitHub ({}): {n} cambios.", info.commit),
    };
    let mut head = vec![];
    for l in wrap(&what, w.saturating_sub(2)) {
        head.push(Line::from(Span::styled(format!(" {l}"), Style::new().fg(theme::text()).bold())));
    }
    for l in wrap("Se baja, se compila y Jarvis se vuelve a abrir en esta misma sesión.", w.saturating_sub(2)) {
        head.push(Line::from(Span::styled(format!(" {l}"), Style::new().fg(theme::faint()))));
    }
    if !info.commits.is_empty() {
        head.push(Line::default());
    }
    let items = info
        .commits
        .iter()
        .map(|c| Line::from(Span::styled(format!("  · {}", truncate(c, w.saturating_sub(4))), Style::new().fg(theme::text()))))
        .collect();
    let hints = [("enter", "actualizar"), ("esc", "ahora no")];
    overlay(f, anchor, "ACTUALIZACIÓN", theme::accent(), head, items, 0, 8, &hints);
}

fn draw_ask(f: &mut Frame, anchor: Rect, a: &crate::ask::Ask) {
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

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let k = |s: &'static str| Span::styled(s, Style::new().fg(theme::accent()));
    let d = |s: &'static str| Span::styled(s, Style::new().fg(theme::faint()));
    let hints = Line::from(vec![
        k(" espacio"), d(" hablar  "),
        k("enter"), d(" enviar  "),
        k("esc"), d(" cancelar  "),
        k("pgup/pgdn"), d(" desplazar  "),
        k("^L"), d(" limpiar  "),
        k("^R"), d(" reiniciar  "),
        k("^C"), d(" salir"),
    ]);
    let hints = flash(app).unwrap_or(hints);
    let stats = format!("   {} ", stats(app));
    let sw = stats.chars().count() as u16;
    let [left, right] = Layout::horizontal([Constraint::Min(0), Constraint::Length(sw)]).areas(area);
    f.render_widget(Paragraph::new(hints), left);
    f.render_widget(Paragraph::new(Span::styled(stats, Style::new().fg(theme::dim()))), right);
}

/// El aviso breve («copiado…», «estilo Cine») mientras dura.
fn flash(app: &App) -> Option<Line<'static>> {
    match &app.flash {
        Some((msg, at)) if at.elapsed().as_secs_f64() < 2.5 => {
            Some(Line::from(Span::styled(format!(" ✓ {msg}"), Style::new().fg(GREEN).bold())))
        }
        _ => None,
    }
}

fn voice_label(app: &App) -> String {
    match app.voice {
        VoiceState::Off => "voz off".to_string(),
        VoiceState::Loading => "voz cargando".to_string(),
        _ => format!("voz {}", app.voice_model),
    }
}

fn ctx_pct(app: &App) -> f64 {
    app.ctx_used as f64 * 100.0 / app.ctx_window.max(1) as f64
}

fn uptime(app: &App) -> String {
    let up = app.started.elapsed().as_secs();
    format!("{:02}:{:02}", up / 3600, up / 60 % 60)
}

fn turns(app: &App) -> String {
    format!("{} turno{}", app.turns, if app.turns == 1 { "" } else { "s" })
}

fn stats(app: &App) -> String {
    let ctx = match app.ctx_used {
        0 => String::new(),
        _ => format!("contexto {:.0}% · ", ctx_pct(app)),
    };
    format!("{} · {ctx}{} · ${:.2} · {}", voice_label(app), turns(app), app.cost, uptime(app))
}

fn cwd() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    std::env::current_dir()
        .map(|p| p.display().to_string().replacen(&home, "~", 1))
        .unwrap_or_default()
}

// ── Piezas sueltas para los estilos sin paneles ─────────────────────────────────────────

/// Escribe recortando a `max` columnas y al borde del buffer; devuelve dónde terminó.
fn put(buf: &mut Buffer, x: u16, y: u16, s: &str, st: Style, max: u16) -> u16 {
    let a = buf.area;
    if y < a.y || y >= a.bottom() || x < a.x || x >= a.right() || max == 0 {
        return x;
    }
    buf.set_stringn(x, y, s, max as usize, st).0
}

/// Alineado a la derecha terminando en `xr` (incluido).
fn put_right(buf: &mut Buffer, xr: u16, y: u16, s: &str, st: Style) {
    let w = s.width() as u16;
    put(buf, (xr + 1).saturating_sub(w), y, s, st, w);
}

fn put_center(buf: &mut Buffer, r: Rect, y: u16, s: &str, st: Style) {
    let w = (s.width() as u16).min(r.width);
    put(buf, r.x + (r.width - w) / 2, y, s, st, r.width);
}

fn put_line_center(buf: &mut Buffer, r: Rect, y: u16, spans: &[(String, Style)]) {
    let w: u16 = spans.iter().map(|(s, _)| s.width() as u16).sum::<u16>().min(r.width);
    let mut x = r.x + (r.width - w) / 2;
    for (s, st) in spans {
        x = put(buf, x, y, s, *st, r.right().saturating_sub(x));
    }
}

fn hline(buf: &mut Buffer, x0: u16, x1: u16, y: u16, ch: &str, st: Style) {
    for x in x0..=x1 {
        put(buf, x, y, ch, st, 1);
    }
}

/// «ESPERANDO RESPUESTA...» → «E S P E R A N D O   R E S P U E S T A...»: los puntos que
/// laten no se separan.
fn spaced_title(s: &str) -> String {
    let base = s.trim_end_matches(['.', ' ']);
    format!("{}{}", spaced(base), &s[base.len()..])
}

/// La voz sin la palabra «voz», para cuando ya va rotulada.
fn voice_value(app: &App) -> String {
    match app.voice {
        VoiceState::Off => "apagada".into(),
        VoiceState::Loading => "cargando…".into(),
        VoiceState::Listening => format!("{} · escuchando", app.voice_model),
        VoiceState::Transcribing => format!("{} · transcribiendo", app.voice_model),
        VoiceState::Ready => format!("{} · lista", app.voice_model),
    }
}

fn spaced(s: &str) -> String {
    s.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
}

/// Una regla con título a la izquierda: «A H O R A ┈┈┈┈┈».
fn section(buf: &mut Buffer, x0: u16, x1: u16, y: u16, title: &str) {
    let x = put(buf, x0, y, &spaced(title), Style::new().fg(theme::faint()).bold(), x1.saturating_sub(x0) + 1);
    if x + 1 <= x1 {
        hline(buf, x + 1, x1, y, "┈", Style::new().fg(theme::dim()));
    }
}

/// Medidor `━━━━━────` de `w` columnas.
fn meter(buf: &mut Buffer, x: u16, y: u16, w: u16, frac: f64, on: Color) -> u16 {
    let n = ((frac.clamp(0.0, 1.0) * w as f64).round()) as u16;
    let x = put(buf, x, y, &"━".repeat(n as usize), Style::new().fg(on), n);
    put(buf, x, y, &"━".repeat((w - n) as usize), Style::new().fg(theme::dim()), w - n)
}

/// Esquinas de mira en vez de una caja: `┌──   ──┐ / └──   ──┘`.
fn corners(buf: &mut Buffer, r: Rect, st: Style, label: Option<&str>) {
    if r.width < 8 || r.height < 2 {
        return;
    }
    let (x0, x1, y0, y1) = (r.x, r.right() - 1, r.y, r.bottom() - 1);
    put(buf, x0, y0, "┌───", st, 4);
    put(buf, x1 - 3, y0, "───┐", st, 4);
    put(buf, x0, y1, "└───", st, 4);
    put(buf, x1 - 3, y1, "───┘", st, 4);
    if let Some(l) = label {
        put(buf, x0 + 5, y0, &format!(" {l} "), Style::new().fg(theme::faint()).bold(), r.width.saturating_sub(10));
    }
}

/// Las herramientas del turno en curso.
fn turn_acts(app: &App) -> &[Activity] {
    &app.activity[app.turn_from.min(app.activity.len())..]
}

/// La línea del turno: un bloque por herramienta, de ancho proporcional a lo que tardó.
fn timeline(buf: &mut Buffer, r: Rect, acts: &[Activity], with_secs: bool) {
    if r.width < 6 || r.height == 0 {
        return;
    }
    if acts.is_empty() {
        put(buf, r.x, r.y, &"░".repeat(r.width as usize), Style::new().fg(theme::dim()), r.width);
        if with_secs && r.height > 1 {
            put(buf, r.x, r.y + 1, "sin turno en curso", Style::new().fg(theme::faint()), r.width);
        }
        return;
    }
    // Lo que no cabe se cae por la izquierda: las últimas importan más.
    let room = r.width as usize;
    let mut take = acts.len();
    while take > 1 && take * 3 > room {
        take -= 1;
    }
    let acts = &acts[acts.len() - take..];
    let running = acts.last().is_some_and(|a| a.status == ToolStatus::Running);
    let room = room.saturating_sub(acts.len() + running as usize);
    let mins: Vec<usize> = acts.iter().map(|a| (a.name.chars().count() + 2).min(8).max(3)).collect();
    let tot: f64 = acts.iter().map(|a| act_secs(a).max(0.05)).sum();
    let mut ws: Vec<usize> = acts
        .iter()
        .zip(&mins)
        .map(|(a, m)| ((room as f64 * act_secs(a).max(0.05) / tot).round() as usize).max(*m))
        .collect();
    while ws.iter().sum::<usize>() > room {
        let Some(k) = (0..ws.len()).filter(|&j| ws[j] > 3).max_by_key(|&j| ws[j] - mins[j].min(ws[j])) else { break };
        ws[k] -= 1;
    }
    let mut x = r.x;
    for (a, w) in acts.iter().zip(ws) {
        let c = match a.status {
            ToolStatus::Running => theme::accent(),
            ToolStatus::Ok => theme::faint(),
            ToolStatus::Err => RED,
        };
        let label = format!(" {:<w$}", truncate(&a.name.to_uppercase(), w.saturating_sub(1)), w = w.saturating_sub(1));
        put(buf, x, r.y, &label, Style::new().fg(Color::Black).bg(c).bold(), w as u16);
        if with_secs && r.height > 1 {
            put(buf, x, r.y + 1, &format!("{:.1}s", act_secs(a)), Style::new().fg(theme::dim()), w as u16);
        }
        x += w as u16 + 1;
    }
    if running {
        put(buf, x.saturating_sub(1), r.y, "▶", Style::new().fg(theme::accent()).bold(), 1);
    }
}

/// Atajos `tecla desc · tecla desc` desde `x`.
fn keys(buf: &mut Buffer, x: u16, y: u16, max_x: u16, items: &[(&str, &str)], kc: Color, brackets: bool) {
    let mut x = x;
    for (k, d) in items {
        let k = if brackets { format!("[{k}]") } else { (*k).to_string() };
        x = put(buf, x, y, &k, Style::new().fg(kc), max_x.saturating_sub(x));
        x = put(buf, x + 1, y, d, Style::new().fg(theme::faint()), max_x.saturating_sub(x + 1)) + 3;
        if x >= max_x {
            break;
        }
    }
}

fn hints_for(app: &App, state: State) -> Vec<(&'static str, &'static str)> {
    if listening(state) {
        vec![("suelta espacio", "enviar"), ("esc", "cancelar")]
    } else if app.busy {
        vec![("esc", "interrumpir"), ("espacio", "hablar"), ("pgup/pgdn", "desplazar")]
    } else {
        vec![("espacio", "hablar"), ("enter", "enviar"), ("/", "comandos"), ("/theme", "estilo"), ("^C", "salir")]
    }
}

// ── Propuesta: sin cajas, el núcleo al frente ───────────────────────────────────────────

fn propuesta(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let m = if area.width > 90 { 3 } else { 1 };
    let inner = Rect { x: area.x + m, width: area.width.saturating_sub(m * 2), ..area };
    let tone = app.nucleo.color();

    let input_h = if listening(state) { 4 } else { 2 };
    let [head, rule, body, input, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Cabecera: marca y modelo a la izquierda; carpeta, sesión y hora a la derecha.
    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x, head.y, "◆ ", Style::new().fg(theme::accent()).bold(), head.width);
        let x = put(buf, x, head.y, &spaced("JARVIS"), Style::new().fg(theme::accent()).bold(), head.width);
        put(buf, x + 3, head.y, app.model.trim_start_matches("claude-"), Style::new().fg(theme::text()), head.width / 3);
        let sess = app.session.get(..8).unwrap_or("········");
        let clock = now_hhmmss();
        put_right(buf, head.right() - 1, head.y, &clock, Style::new().fg(theme::text()));
        put_right(buf, head.right() - 1 - clock.len() as u16 - 3, head.y, &format!("{}   {sess}", cwd()), Style::new().fg(theme::faint()));
        hline(buf, rule.x, rule.right() - 1, rule.y, "─", Style::new().fg(theme::dim()));
    }

    let side_w = (body.width * 36 / 100).clamp(30, 56);
    let narrow = body.width < 80;
    let (chat, side) = if narrow {
        (body, Rect::default())
    } else {
        let [c, _, s] = Layout::horizontal([Constraint::Min(30), Constraint::Length(4), Constraint::Length(side_w)]).areas(body);
        (c, s)
    };

    // La conversación como guion.
    let chat_in = Rect { y: chat.y + 1, height: chat.height.saturating_sub(1), ..chat };
    if app.messages.is_empty() {
        let buf = f.buffer_mut();
        let y = chat_in.y + chat_in.height / 3;
        put(buf, chat_in.x + GUION as u16, y, &format!("{}.", greeting()), Style::new().fg(theme::text()).bold(), chat_in.width);
        put(buf, chat_in.x + GUION as u16, y + 1, "¿En qué trabajamos?", Style::new().fg(theme::accent()).bold(), chat_in.width);
        put(buf, chat_in.x + GUION as u16, y + 3, "escribe, o mantén espacio y háblame", Style::new().fg(theme::faint()), chat_in.width);
        *app.view.borrow_mut() = Default::default();
    } else {
        let rows = chat_rows(app, chat_in.width as usize, Voz::Guion);
        render_chat(f, chat_in, app, rows);
    }

    // El núcleo, su estado y la telemetría.
    if !narrow {
        let core_h = (side.width / 2).min(side.height.saturating_sub(12)).max(6);
        let [core, label, _, tele, _, now] = Layout::vertical([
            Constraint::Length(core_h),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(side);
        core_draw(app, core, f.buffer_mut());
        let (title, detail) = core_label(app, t, label.width as usize);
        let buf = f.buffer_mut();
        put_center(buf, label, label.y, &spaced_title(&title), Style::new().fg(tone).bold());
        put_center(buf, label, label.y + 1, &detail, Style::new().fg(theme::faint()));
        let kx = tele.x + 1;
        let vx = tele.x + 13;
        let vw = tele.right().saturating_sub(vx);
        let k = Style::new().fg(theme::faint());
        let v = Style::new().fg(theme::text());
        put(buf, kx, tele.y, "contexto", k, 12);
        let x = meter(buf, vx, tele.y, vw.saturating_sub(6).min(20), ctx_pct(app) / 100.0, tone);
        put(buf, x + 1, tele.y, &format!("{:.0}%", ctx_pct(app)), v, 5);
        put(buf, kx, tele.y + 1, "sesión", k, 12);
        put(buf, vx, tele.y + 1, &format!("{} · ${:.2}", turns(app), app.cost), v, vw);
        put(buf, kx, tele.y + 2, "voz", k, 12);
        put(buf, vx, tele.y + 2, &voice_value(app), v, vw);
        put(buf, kx, tele.y + 3, "en línea", k, 12);
        put(buf, vx, tele.y + 3, &uptime(app), v, vw);
        // Lo último que hizo, lo más nuevo arriba.
        for (i, a) in app.activity.iter().rev().take(now.height as usize).enumerate() {
            let y = now.y + i as u16;
            let (icon, c) = act_icon(a, t);
            put(buf, kx, y, icon, Style::new().fg(c), 1);
            let x = put(buf, kx + 2, y, &a.name, Style::new().fg(theme::text()).bold(), 12);
            let secs = format!("{:.1}s", act_secs(a));
            put(buf, x + 1, y, &a.detail, Style::new().fg(theme::faint()), now.right().saturating_sub(x + 2 + secs.len() as u16));
            put_right(buf, now.right() - 1, y, &secs, Style::new().fg(theme::dim()));
        }
    }

    // La orden: una regla del color del estado y la línea de escritura.
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
        let mut spans = vec![Span::styled("› ", Style::new().fg(theme::accent()).bold())];
        spans.extend(input_spans(app, state, t, line.width as usize - 2, theme::accent()));
        f.render_widget(Paragraph::new(Line::from(spans)), line);
        draw_modal(f, Rect { y: input_w.y, height: 1, ..input_w }, app);
    }

    let buf = f.buffer_mut();
    match flash(app) {
        Some(l) => {
            buf.set_line(hints.x, hints.y, &l, hints.width);
        }
        None => keys(buf, hints.x, hints.y, hints.right(), &hints_for(app, state), if listening(state) { tone } else { theme::accent() }, false),
    }
}

// ── Cabina: HUD de instrumentos ─────────────────────────────────────────────────────────

fn cabina(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let [bar, body, foot] = Layout::vertical([Constraint::Length(1), Constraint::Min(8), Constraint::Length(1)]).areas(area);

    // Barra de estado segmentada.
    {
        let buf = f.buffer_mut();
        let sep = Style::new().fg(theme::dim());
        let k = Style::new().fg(theme::faint());
        let v = Style::new().fg(theme::text());
        let mut x = put(buf, bar.x, bar.y, " ◢ JARVIS ", Style::new().fg(Color::Black).bg(theme::accent()).bold(), 10) + 2;
        for (key, val) in [
            ("MODELO", app.model.trim_start_matches("claude-").to_string()),
            ("DIR", cwd()),
            ("SES", app.session.get(..8).unwrap_or("········").to_string()),
        ] {
            x = put(buf, x, bar.y, key, k, 8);
            x = put(buf, x + 1, bar.y, &val, v, 40);
            x = put(buf, x + 1, bar.y, "│", sep, 1) + 1;
        }
        let clock = now_hhmmss();
        let pct = ctx_pct(app);
        let right = format!("CTX ▮▮▮▮▮▮▮▮▮▮ {:>2.0}% │ ${:.2} │ T{} │ {clock} ", pct, app.cost, app.turns);
        let rx = bar.right().saturating_sub(right.width() as u16);
        if rx > x {
            let mut x = put(buf, rx, bar.y, "CTX ", k, 4);
            let n = (pct / 10.0).round().clamp(0.0, 10.0) as usize;
            x = put(buf, x, bar.y, &"▮".repeat(n), Style::new().fg(tone), 10);
            x = put(buf, x, bar.y, &"▯".repeat(10 - n), sep, 10);
            x = put(buf, x + 1, bar.y, &format!("{pct:>2.0}%"), v, 4);
            x = put(buf, x + 1, bar.y, "│", sep, 1);
            x = put(buf, x + 1, bar.y, &format!("${:.2}", app.cost), v, 10);
            x = put(buf, x + 1, bar.y, "│", sep, 1);
            x = put(buf, x + 1, bar.y, "T", k, 1);
            x = put(buf, x, bar.y, &app.turns.to_string(), v, 5);
            x = put(buf, x + 1, bar.y, "│", sep, 1);
            put(buf, x + 1, bar.y, &clock, Style::new().fg(theme::accent()).bold(), 8);
        }
    }

    let left_w = (body.width * 38 / 100).clamp(30, 56);
    let narrow = body.width < 90;
    let [left, _, right] = if narrow {
        [Rect::default(), Rect::default(), body]
    } else {
        Layout::horizontal([Constraint::Length(left_w), Constraint::Length(2), Constraint::Min(30)]).areas(body)
    };

    // Columna izquierda: núcleo enmarcado, registro y línea del turno.
    if !narrow {
        let core_h = (left.width / 2 + 2).min(left.height.saturating_sub(10)).max(8);
        let [frame, _, reg, line] = Layout::vertical([
            Constraint::Length(core_h),
            Constraint::Length(1),
            Constraint::Min(2),
            Constraint::Length(3),
        ])
        .areas(Rect { x: left.x + 1, width: left.width.saturating_sub(1), ..left });
        {
            let buf = f.buffer_mut();
            corners(buf, frame, Style::new().fg(theme::dim()), Some("NÚCLEO"));
        }
        let core = Rect { x: frame.x + 1, y: frame.y + 1, width: frame.width.saturating_sub(2), height: frame.height.saturating_sub(4) };
        core_draw(app, core, f.buffer_mut());
        let (title, detail) = core_label(app, t, frame.width as usize / 2);
        let buf = f.buffer_mut();
        let ly = frame.bottom().saturating_sub(3);
        let blink = if (t * 1.5) as usize % 2 == 0 { "◉" } else { "○" };
        let x = put(buf, frame.x + 2, ly, blink, Style::new().fg(tone).bold(), 1);
        put(buf, x + 1, ly, &title, Style::new().fg(tone).bold(), frame.width / 2);
        put_right(buf, frame.right().saturating_sub(3), ly, &truncate(&detail, frame.width as usize / 2), Style::new().fg(theme::faint()));
        for x in frame.x + 2..frame.right().saturating_sub(2) {
            let ch = if (x - frame.x - 2) % 5 == 0 { "┊" } else { "┈" };
            put(buf, x, ly + 1, ch, Style::new().fg(theme::dim()), 1);
        }

        // Registro: cada herramienta con su hora.
        section(buf, reg.x, reg.right() - 1, reg.y, "REGISTRO");
        let rows = reg.height.saturating_sub(2) as usize;
        let list: Vec<&Activity> = app.activity.iter().rev().take(rows).collect();
        if list.is_empty() {
            put(buf, reg.x + 2, reg.y + 2, "— sin eventos —", Style::new().fg(theme::dim()), reg.width);
        }
        for (i, a) in list.iter().rev().enumerate() {
            let y = reg.y + 2 + i as u16;
            let (icon, c) = act_icon(a, t);
            let ago = a.started.elapsed().as_secs();
            let x = put(buf, reg.x, y, &clock_ago(ago), Style::new().fg(theme::dim()), 8);
            put(buf, x + 1, y, icon, Style::new().fg(c).bold(), 1);
            let x = put(buf, x + 3, y, &format!("{:<6}", truncate(&a.name.to_uppercase(), 6)), Style::new().fg(theme::text()).bold(), 6);
            let secs = format!("{:.1}s", act_secs(a));
            put(buf, x + 1, y, &a.detail, Style::new().fg(theme::faint()), reg.right().saturating_sub(x + 3 + secs.len() as u16));
            let sc = match a.status {
                ToolStatus::Running => theme::accent(),
                ToolStatus::Err => RED,
                ToolStatus::Ok => theme::dim(),
            };
            put_right(buf, reg.right() - 1, y, &secs, Style::new().fg(sc));
        }

        // Línea del turno.
        put(buf, line.x, line.y + 1, "LÍNEA", Style::new().fg(theme::faint()).bold(), 6);
        timeline(buf, Rect { x: line.x + 7, y: line.y + 1, width: line.width.saturating_sub(7), height: 2 }, turn_acts(app), true);
    }

    // Columna derecha: el canal y la orden.
    let orden_h = if listening(state) { 6 } else { 4 };
    let [canal, orden] = Layout::vertical([Constraint::Min(4), Constraint::Length(orden_h)]).areas(right);
    let canal = Rect { width: canal.width.saturating_sub(1), ..canal };
    {
        let buf = f.buffer_mut();
        corners(buf, canal, Style::new().fg(theme::dim()), Some("CANAL"));
    }
    let chat_in = Rect { x: canal.x + 1, y: canal.y + 2, width: canal.width.saturating_sub(3), height: canal.height.saturating_sub(3) };
    let rows = chat_rows(app, chat_in.width.saturating_sub(4) as usize, Voz::Canal);
    render_chat(f, chat_in, app, rows);

    let orden = Rect { width: orden.width.saturating_sub(1), ..orden };
    let active = !app.busy || listening(state);
    let col = if listening(state) { tone } else if active { theme::accent() } else { theme::dim() };
    {
        let buf = f.buffer_mut();
        corners(buf, orden, Style::new().fg(col), None);
        let y = orden.y + 1;
        let x = put(buf, orden.x + 3, y, "ORDEN", Style::new().fg(if active { col } else { theme::faint() }).bold(), 5);
        put(buf, x + 1, y, "│", Style::new().fg(theme::dim()), 1);
    }
    if listening(state) {
        let w = Rect { x: orden.x + 3, y: orden.y + 2, width: orden.width.saturating_sub(6), height: orden.height.saturating_sub(3) };
        wave(f, w, app, t, tone, theme::dim());
    } else {
        let line = Rect { x: orden.x + 11, y: orden.y + 1, width: orden.width.saturating_sub(14), height: 1 };
        f.render_widget(Paragraph::new(Line::from(input_spans(app, state, t, line.width as usize, col))), line);
        draw_modal(f, Rect { x: canal.x, y: orden.y, width: canal.width, height: 1 }, app);
    }

    let buf = f.buffer_mut();
    match flash(app) {
        Some(l) => {
            buf.set_line(foot.x + 1, foot.y, &l, foot.width);
        }
        None => keys(buf, foot.x + 1, foot.y, foot.right(), &hints_for(app, state), if listening(state) { tone } else { theme::accent() }, true),
    }
}

/// La hora local de hace `ago` segundos, `hh:mm:ss`.
fn clock_ago(ago: u64) -> String {
    let (h, m, s) = local_hms();
    let now = h * 3600 + m * 60 + s;
    let then = (now + 86_400 - ago % 86_400) % 86_400;
    format!("{:02}:{:02}:{:02}", then / 3600, then / 60 % 60, then % 60)
}

// ── Cine: pasado, presente y lo que hace ────────────────────────────────────────────────

fn cine(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let hear = listening(state);

    // Las cuatro esquinas.
    {
        let buf = f.buffer_mut();
        let top = area.y + 1;
        let bot = area.bottom().saturating_sub(2);
        put(buf, area.x + 3, top, &spaced("JARVIS"), Style::new().fg(theme::accent()).bold(), 20);
        put_right(buf, area.right().saturating_sub(4), top, &now_hhmmss(), Style::new().fg(theme::text()));
        let x = put(buf, area.x + 3, bot, "ctx ", Style::new().fg(theme::faint()), 4);
        let x = meter(buf, x, bot, 20, ctx_pct(app) / 100.0, tone);
        put(buf, x + 1, bot, &format!("{:.0}%", ctx_pct(app)), Style::new().fg(theme::faint()), 5);
        let right = format!("{} · ${:.2} · {}", app.model.trim_start_matches("claude-"), app.cost, turns(app));
        put_right(buf, area.right().saturating_sub(4), bot, &right, Style::new().fg(theme::faint()));
    }

    // ^T: la conversación entera como una cortina, con su markdown; la orden sigue abajo.
    if app.transcript {
        app.reading.set(false);
        let body = Rect {
            x: area.x + 2,
            y: area.y + 3,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(9),
        };
        draw_chat(f, body, app);
        let line = Rect { x: area.x + 4, y: body.bottom() + 1, width: area.width.saturating_sub(8), height: 1 };
        let mut spans = vec![Span::styled("› ", Style::new().fg(theme::accent()).bold())];
        spans.extend(input_spans(app, state, t, line.width.saturating_sub(2) as usize, theme::accent()));
        f.render_widget(Paragraph::new(Line::from(spans)), line);
        let hint = Line::from(vec![
            Span::styled("^T", Style::new().fg(theme::accent())),
            Span::styled(" o ", Style::new().fg(theme::faint())),
            Span::styled("esc", Style::new().fg(theme::accent())),
            Span::styled(" vuelve a Cine   ", Style::new().fg(theme::faint())),
            Span::styled("pgup/pgdn", Style::new().fg(theme::accent())),
            Span::styled(" recorre   ", Style::new().fg(theme::faint())),
            Span::styled("arrastra", Style::new().fg(theme::accent())),
            Span::styled(" para copiar", Style::new().fg(theme::faint())),
        ]);
        let hr = Rect { y: line.y + 1, ..line };
        f.render_widget(Paragraph::new(hint).alignment(Alignment::Center), hr);
        return;
    }

    let inner = Rect {
        x: area.x + 3,
        y: area.y + 3,
        width: area.width.saturating_sub(6),
        height: area.height.saturating_sub(6),
    };
    // La respuesta que se abre para leer: la más nueva que se ve en el historial (con pgup se
    // puede ir a una anterior). Se abre sola si terminó el turno y no cabe en el subtítulo.
    let said: Vec<(usize, &crate::Msg)> = app
        .messages
        .iter()
        .enumerate()
        .filter(|(_, m)| matches!(m.role, Role::User | Role::Assistant) && m.waiting.is_none())
        .collect();
    let skip = (app.scroll / 5).min(said.len().saturating_sub(1));
    let pick = said.iter().rev().skip(skip).find(|(_, m)| m.role == Role::Assistant).copied();
    let long = pick.is_some_and(|(_, m)| wrap(&plain(&m.text), 60).len() > 4 || m.text.contains('\n'));
    let reading = !hear && pick.is_some() && app.read_override.unwrap_or(!app.busy && long);
    app.reading.set(reading);

    let rails = inner.width >= 110;
    let (center_w, rail_w) = if reading && rails {
        // Leyendo, el centro se lleva el ancho y los rieles se angostan.
        let rail = (inner.width / 5).clamp(30, 40);
        (inner.width - 2 * (rail + 3), rail)
    } else if rails {
        let c = (inner.width * 40 / 100).clamp(52, 64);
        (c, ((inner.width - c) / 2).saturating_sub(4).min(44))
    } else {
        (inner.width.min(if reading { 110 } else { 72 }), 0)
    };
    let center = Rect { x: inner.x + (inner.width - center_w) / 2, width: center_w, ..inner };

    // Centro: núcleo, estado, subtítulo (o la respuesta entera) y la orden.
    let caption_h = if hear { 6 } else { 5 };
    let [core, label, caption, orden, keyrow] = if reading {
        Layout::vertical([
            Constraint::Length(9.min(center.height / 4).max(4)),
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(center)
    } else {
        Layout::vertical([
            Constraint::Min(6),
            Constraint::Length(2),
            Constraint::Length(caption_h),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(center)
    };
    let core_w = core.width.min(core.height * 2 + 8);
    core_draw(app, Rect { x: core.x + (core.width - core_w) / 2, width: core_w, ..core }, f.buffer_mut());
    let (title, detail) = core_label(app, t, label.width as usize);
    {
        let buf = f.buffer_mut();
        put_center(buf, label, label.y, &spaced_title(&title), Style::new().fg(tone).bold());
        put_center(buf, label, label.y + 1, &detail, Style::new().fg(theme::faint()));
    }

    if let (true, Some((i, m))) = (reading, pick) {
        read_panel(f, caption, app, i, &m.text);
    }
    let mut subtitle: Option<crate::select::View> = None;
    // El subtítulo: lo último que se dijo, con el final a la vista mientras llega.
    let last = app.messages.iter().rev().find(|m| matches!(m.role, Role::User | Role::Assistant) && m.waiting.is_none());
    let last = if reading { None } else { last };
    let cap_lines = if hear { 2 } else { (caption.height as usize).saturating_sub(1) };
    if let Some(m) = last {
        let text = plain(&m.text);
        let ls = wrap(&text, caption.width.saturating_sub(4) as usize);
        let tail = &ls[ls.len().saturating_sub(cap_lines)..];
        let st = match (m.role, hear) {
            (_, true) => Style::new().fg(theme::faint()),
            (Role::User, _) => Style::new().fg(theme::faint()),
            _ => Style::new().fg(theme::text()),
        };
        let buf = f.buffer_mut();
        for (i, l) in tail.iter().enumerate() {
            put_center(buf, caption, caption.y + 1 + i as u16, l, st);
        }
        // El subtítulo también se selecciona y se copia con el mouse.
        let area = Rect { y: caption.y + 1, height: tail.len() as u16, ..caption };
        let mut view = crate::select::View { area, rows: Vec::with_capacity(tail.len()), first: 0 };
        for (i, l) in tail.iter().enumerate() {
            let pad = (caption.width as usize).saturating_sub(l.width().min(caption.width as usize)) / 2;
            view.rows.push(crate::select::Row { text: format!("{}{l}", " ".repeat(pad)), skip: pad, cont: i > 0 });
        }
        subtitle = Some(view);
    } else if !reading {
        let buf = f.buffer_mut();
        put_center(buf, caption, caption.y + 1, &format!("{}.", greeting()), Style::new().fg(theme::text()).bold());
        put_center(buf, caption, caption.y + 2, "¿En qué trabajamos?", Style::new().fg(theme::accent()).bold());
    }
    if hear {
        let w = Rect { x: caption.x + 2, y: caption.y + 3, width: caption.width.saturating_sub(4), height: 3 };
        wave(f, w, app, t, tone, theme::dim());
    }

    // La orden: una caja que se ve, del color del estado.
    let active = !app.busy || hear;
    let col = if hear { tone } else if active { theme::accent() } else { theme::dim() };
    {
        let buf = f.buffer_mut();
        let (x0, x1) = (orden.x, orden.right() - 1);
        let st = Style::new().fg(col);
        put(buf, x0, orden.y, &format!("╭{}╮", "─".repeat(orden.width.saturating_sub(2) as usize)), st, orden.width);
        put(buf, x0, orden.y + 2, &format!("╰{}╯", "─".repeat(orden.width.saturating_sub(2) as usize)), st, orden.width);
        put(buf, x0, orden.y + 1, "│", st, 1);
        put(buf, x1, orden.y + 1, "│", st, 1);
        let lab = if active { " escribe aquí " } else { " o escribe aquí " };
        put(buf, x0 + 2, orden.y, lab, st.bold(), orden.width.saturating_sub(4));
        put(buf, x0 + 2, orden.y + 1, "›", Style::new().fg(col).bold(), 1);
    }
    let line = Rect { x: orden.x + 4, y: orden.y + 1, width: orden.width.saturating_sub(6), height: 1 };
    let spans = if hear && app.input.is_empty() {
        vec![Span::styled("te escucho… suelta espacio para enviar", Style::new().fg(theme::faint()))]
    } else {
        input_spans(app, state, t, line.width as usize, col)
    };
    f.render_widget(Paragraph::new(Line::from(spans)), line);
    {
        let buf = f.buffer_mut();
        match flash(app) {
            Some(l) => {
                let w = l.width() as u16;
                buf.set_line(keyrow.x + keyrow.width.saturating_sub(w) / 2, keyrow.y, &l, keyrow.width);
            }
            None => {
                let mut items = hints_for(app, state);
                if reading {
                    items.insert(0, ("^O", "núcleo"));
                } else if long && !hear {
                    items.insert(0, ("^O", "leer entera"));
                }
                let spans: Vec<(String, Style)> = items
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (k, d))| {
                        let sep = if i == 0 { String::new() } else { "   ".into() };
                        [(sep, Style::new()), ((*k).to_string(), Style::new().fg(col)), (format!(" {d}"), Style::new().fg(theme::faint()))]
                    })
                    .collect();
                put_line_center(buf, keyrow, keyrow.y, &spans);
            }
        }
    }

    // Lo que se puede seleccionar con el mouse: la respuesta abierta (ya puesta por
    // read_panel) o, si no, el subtítulo.
    if !reading {
        *app.view.borrow_mut() = subtitle.unwrap_or_default();
    }
    if let Some(sel) = &app.sel {
        let view = app.view.borrow();
        let buf = f.buffer_mut();
        for (x, y) in sel.cells(&view) {
            buf[(x, y)].set_bg(theme::dim()).set_fg(Color::White);
        }
    }

    if rails {
        let left = Rect { x: inner.x, width: rail_w, height: orden.bottom() - inner.y, ..inner };
        let right = Rect { x: inner.right() - rail_w, width: rail_w, height: orden.bottom() - inner.y, ..inner };
        cine_history(f.buffer_mut(), left, app);
        cine_now(f.buffer_mut(), right, app, t, tone);
    }

    if !hear {
        draw_modal(f, Rect { y: orden.y, height: 1, ..center }, app);
    }
}

/// Cine leyendo: la respuesta entera con su markdown, desde el principio, desplazable.
fn read_panel(f: &mut Frame, r: Rect, app: &App, i: usize, text: &str) {
    let rows = app.md_rows(i, text, r.width.saturating_sub(2) as usize);
    let h = r.height.saturating_sub(1) as usize;
    let max_top = rows.len().saturating_sub(h);
    // Otro mensaje: se lee desde arriba. El mismo creciendo (o desplazado): donde estaba.
    if app.read_msg.get() != i {
        app.read_msg.set(i);
        app.read_top.set(0);
    }
    let top = app.read_top.get().min(max_top);
    app.read_top.set(top);
    let body = Rect { y: r.y + 1, height: h as u16, ..r };
    let visible: Vec<Line> = rows.iter().skip(top).take(h).map(|(l, _, _)| l.clone()).collect();
    f.render_widget(Paragraph::new(visible), body);

    let buf = f.buffer_mut();
    let dim = Style::new().fg(theme::dim());
    put(buf, r.x, r.y, &"─".repeat(r.width as usize), dim, r.width);
    if top > 0 {
        put_right(buf, r.right().saturating_sub(1), r.y, " ↑ pgup ", Style::new().fg(theme::accent()));
    }
    if top < max_top {
        let pct = (top + h) * 100 / rows.len().max(1);
        put_right(buf, r.right().saturating_sub(1), r.bottom().saturating_sub(1), &format!(" ↓ pgdn · {pct}% "), Style::new().fg(theme::accent()));
    }

    // Para seleccionar y copiar con el mouse, como en la conversación del Clásico.
    let mut view = crate::select::View { area: body, rows: Vec::with_capacity(rows.len()), first: top };
    for (line, skip, cont) in &rows {
        let text = line.spans.iter().map(|s| s.content.as_ref()).collect();
        view.rows.push(crate::select::Row { text, skip: *skip, cont: *cont });
    }
    *app.view.borrow_mut() = view;
}

/// Markdown aplanado para leerlo en una línea: sin asteriscos, comillas invertidas ni almohadillas.
fn plain(s: &str) -> String {
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

/// Riel izquierdo: lo dicho, apilado desde abajo y apagándose hacia arriba.
fn cine_history(buf: &mut Buffer, r: Rect, app: &App) {
    let x1 = r.right() - 1;
    section(buf, r.x, x1, r.y, "HISTORIAL");
    let w = r.width.saturating_sub(1) as usize;
    let msgs: Vec<_> = app
        .messages
        .iter()
        .filter(|m| matches!(m.role, Role::User | Role::Assistant) && m.waiting.is_none())
        .collect();
    let bottom = r.bottom().saturating_sub(3);
    if msgs.is_empty() {
        put(buf, r.x, r.y + 2, "todavía no hay conversación", Style::new().fg(theme::dim()), r.width);
    }
    // pgup/pgdn recorren el historial: cada 10 líneas de desplazamiento, 2 mensajes.
    let skip = (app.scroll / 5).min(msgs.len().saturating_sub(1));
    let mut y = bottom;
    for (k, m) in msgs.iter().rev().skip(skip).enumerate() {
        let mut ls = wrap(&plain(&m.text), w);
        if ls.len() > 2 {
            ls.truncate(2);
            ls[1] = format!("{}…", truncate(&ls[1], w.saturating_sub(1)));
        }
        let h = ls.len() as u16 + 1;
        if y < r.y + 2 + h {
            break;
        }
        // Lo más nuevo se lee claro; hacia arriba se va apagando.
        let fade = (k as f64 / 4.0).min(1.0);
        let tone = mix(theme::text(), theme::dim(), fade);
        let (name, wc) = match m.role {
            Role::User => ("tú", if k == 0 { theme::text() } else { tone }),
            _ => ("jarvis", if k == 0 { theme::accent() } else { tone }),
        };
        let top = y + 1 - h;
        put(buf, r.x, top, name, Style::new().fg(wc).bold(), r.width);
        for (j, l) in ls.iter().enumerate() {
            put(buf, r.x + 1, top + 1 + j as u16, l, Style::new().fg(tone), r.width.saturating_sub(1));
        }
        y = top.saturating_sub(2);
    }
    let x = put(buf, r.x, r.bottom() - 1, "^T", Style::new().fg(theme::accent()), 2);
    let what = if skip > 0 { format!("ver todo · {skip} más recientes ocultos") } else { "ver todo".into() };
    put(buf, x + 1, r.bottom() - 1, &what, Style::new().fg(theme::dim()), r.width.saturating_sub(3));
}

/// Riel derecho: lo que está haciendo ahora, el turno, la línea de tiempo, el plan y la cola.
fn cine_now(buf: &mut Buffer, r: Rect, app: &App, t: f64, tone: Color) {
    let x1 = r.right() - 1;
    let mut y = r.y;
    section(buf, r.x, x1, y, "AHORA");
    y += 2;
    match app.current_tool() {
        Some(a) => {
            let (icon, c) = act_icon(a, t);
            put(buf, r.x, y, icon, Style::new().fg(c).bold(), 1);
            put(buf, r.x + 2, y, &a.name, Style::new().fg(theme::text()).bold(), r.width / 2);
            put_right(buf, x1, y, &format!("{:.1}s", act_secs(a)), Style::new().fg(theme::accent()));
            put(buf, r.x + 2, y + 1, &a.detail, Style::new().fg(theme::faint()), r.width.saturating_sub(2));
            if matches!(app.modal, Some(Modal::Ask(_))) {
                put(buf, r.x + 2, y + 2, "espera tu permiso", Style::new().fg(tone), r.width);
            }
        }
        None => {
            let what = match app.state() {
                State::Thinking => "pensando",
                State::Speaking => "respondiendo",
                State::Listening | State::NoVoice => "escuchando",
                State::Transcribing => "transcribiendo",
                State::Offline => "motor detenido · ^R",
                _ if app.busy => "trabajando",
                _ => "nada en curso",
            };
            put(buf, r.x, y, if app.busy { SPIN[(t * 8.0) as usize % 4] } else { "·" }, Style::new().fg(tone), 1);
            put(buf, r.x + 2, y, what, Style::new().fg(if app.busy { theme::text() } else { theme::faint() }), r.width);
            put(buf, r.x + 2, y + 1, &voice_label(app), Style::new().fg(theme::dim()), r.width);
        }
    }
    y += 4;

    let acts = turn_acts(app);
    section(buf, r.x, x1, y, "ESTE TURNO");
    y += 2;
    let room = 5usize;
    if acts.is_empty() {
        put(buf, r.x, y, "sin herramientas todavía", Style::new().fg(theme::dim()), r.width);
        y += 1;
    }
    for a in acts.iter().rev().take(room).collect::<Vec<_>>().into_iter().rev() {
        let (icon, c) = act_icon(a, t);
        put(buf, r.x, y, icon, Style::new().fg(c).bold(), 1);
        let x = put(buf, r.x + 2, y, &a.name, Style::new().fg(theme::text()).bold(), 10);
        let secs = format!("{:.1}s", act_secs(a));
        put(buf, x + 1, y, &a.detail, Style::new().fg(theme::dim()), x1.saturating_sub(x + 2 + secs.len() as u16));
        let sc = if a.status == ToolStatus::Err { RED } else { theme::dim() };
        put_right(buf, x1, y, &secs, Style::new().fg(sc));
        y += 1;
    }
    y += 1;
    timeline(buf, Rect { x: r.x, y, width: r.width, height: 1 }, acts, false);
    y += 3;

    if app.todos.0 > 0 && y + 2 < r.bottom() {
        section(buf, r.x, x1, y, &format!("PLAN {}/{}", app.todos.1, app.todos.0));
        y += 2;
        meter(buf, r.x, y, r.width.min(30), app.todos.1 as f64 / app.todos.0 as f64, tone);
        y += 3;
    }

    let cola: Vec<_> = app.messages.iter().filter(|m| m.waiting.is_some()).collect();
    if !cola.is_empty() && y + 2 < r.bottom() {
        section(buf, r.x, x1, y, "EN COLA");
        y += 2;
        for (i, m) in cola.iter().enumerate() {
            if y >= r.bottom() {
                break;
            }
            put(buf, r.x, y, &(i + 1).to_string(), Style::new().fg(theme::accent()).bold(), 2);
            put(buf, r.x + 2, y, &plain(&m.text), Style::new().fg(theme::faint()), r.width.saturating_sub(2));
            y += 1;
        }
    }
}

fn mix(a: Color, b: Color, f: f64) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * f).round() as u8;
            Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
        }
        _ => a,
    }
}

// ── Texto y hora ────────────────────────────────────────────────────────────────────────

/// Como `wrap`, marcando las piezas que continúan la línea anterior del texto original.
fn wrap_cont(s: &str, width: usize) -> Vec<(String, bool)> {
    s.lines()
        .flat_map(|l| wrap(l, width).into_iter().enumerate().map(|(i, p)| (p, i > 0)))
        .collect()
}

fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in s.lines() {
        if line.trim().is_empty() {
            out.push(String::new());
        } else {
            out.extend(textwrap::wrap(line, width.max(1)).into_iter().map(|c| c.into_owned()));
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut w = 0;
    let mut out = String::new();
    for c in s.chars() {
        let cw = c.width().unwrap_or(0);
        if w + cw > max {
            if max > 0 {
                out.pop();
                out.push('…');
            }
            break;
        }
        w += cw;
        out.push(c);
    }
    out
}

fn local_hms() -> (u64, u64, u64) {
    // Hora local sin dependencias: se toma el desfase de `date +%z` una vez.
    use std::sync::OnceLock;
    static OFFSET: OnceLock<i64> = OnceLock::new();
    let off = *OFFSET.get_or_init(|| {
        std::process::Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                let s = s.trim();
                let sign = if s.starts_with('-') { -1 } else { 1 };
                let n: i64 = s.get(1..)?.parse().ok()?;
                Some(sign * ((n / 100) * 3600 + (n % 100) * 60))
            })
            .unwrap_or(0)
    });
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + off;
    let s = now.rem_euclid(86_400) as u64;
    (s / 3600, s / 60 % 60, s % 60)
}

fn now_hhmmss() -> String {
    let (h, m, s) = local_hms();
    format!("{h:02}:{m:02}:{s:02}")
}

fn greeting() -> &'static str {
    match local_hms().0 {
        5..=11 => "Buenos días",
        12..=18 => "Buenas tardes",
        _ => "Buenas noches",
    }
}

/// El núcleo: como imagen de puntos (Sixel) si se puede, y si no en braille. En modo imagen
/// el texto deja el panel vacío y se anota dónde va; la app manda la imagen después del cuadro.
/// Con un menú abierto va en braille: la imagen se pintaría encima del menú.
fn core_draw(app: &App, r: Rect, buf: &mut Buffer) {
    if app.sixel_cell.is_some() && app.modal.is_none() && app.tools_view.is_none() && r.width >= 6 && r.height >= 3 {
        app.core_rect.set(Some(r));
    } else {
        app.nucleo.draw(r, buf);
    }
}

fn h_rows(r: Rect) -> usize {
    r.height as usize
}

// ── Piezas de los estilos de /theme que siguen ──────────────────────────────────────────

fn rgb3(c: [f64; 3]) -> Color {
    Color::Rgb(c[0].round() as u8, c[1].round() as u8, c[2].round() as u8)
}

/// La línea de escritura con su prompt, y lo que se despliega encima (preguntas, menús).
/// `anchor` es la fila desde la que suben los menús.
fn orden_line(f: &mut Frame, line: Rect, anchor: Rect, app: &App, state: State, t: f64, prompt: &str, col: Color) {
    if line.width < 4 || line.height == 0 {
        return;
    }
    let pw = prompt.width();
    let mut spans = vec![Span::styled(prompt.to_string(), Style::new().fg(col).bold())];
    spans.extend(input_spans(app, state, t, (line.width as usize).saturating_sub(pw + 1), col));
    f.render_widget(Paragraph::new(Line::from(spans)), Rect { height: 1, ..line });
    draw_modal(f, anchor, app);
}

/// El aviso breve si hay uno; si no, los atajos.
fn foot_keys(buf: &mut Buffer, r: Rect, app: &App, state: State, kc: Color, brackets: bool) {
    match flash(app) {
        Some(l) => {
            buf.set_line(r.x, r.y, &l, r.width);
        }
        None => keys(buf, r.x, r.y, r.right(), &hints_for(app, state), kc, brackets),
    }
}

/// Letras de bloque de 3 filas para titulares. Sin tildes; lo que no está, no se dibuja.
fn big_glyph(c: char) -> Option<[&'static str; 3]> {
    Some(match c {
        'A' => ["█▀█", "█▀█", "▀ ▀"],
        'B' => ["█▀▄", "█▀▄", "▀▀ "],
        'C' => ["█▀▀", "█  ", "▀▀▀"],
        'D' => ["█▀▄", "█ █", "▀▀ "],
        'E' => ["█▀▀", "█▀ ", "▀▀▀"],
        'F' => ["█▀▀", "█▀ ", "▀  "],
        'G' => ["█▀▀", "█ █", "▀▀▀"],
        'H' => ["█ █", "█▀█", "▀ ▀"],
        'I' => ["▀█▀", " █ ", "▀▀▀"],
        'J' => ["  █", "  █", "▀▀ "],
        'K' => ["█ █", "█▀▄", "▀ ▀"],
        'L' => ["█  ", "█  ", "▀▀▀"],
        'M' => ["█▄ ▄█", "█ ▀ █", "▀   ▀"],
        'N' => ["█▄ █", "█ ▀█", "▀  ▀"],
        'O' => ["█▀█", "█ █", "▀▀▀"],
        'P' => ["█▀█", "█▀▀", "▀  "],
        'Q' => ["█▀█", "█ █", "▀▀▄"],
        'R' => ["█▀▄", "█▀▄", "▀ ▀"],
        'S' => ["█▀▀", "▀▀█", "▀▀▀"],
        'T' => ["▀█▀", " █ ", " ▀ "],
        'U' => ["█ █", "█ █", "▀▀▀"],
        'V' => ["█ █", "█ █", " ▀ "],
        'W' => ["█   █", "█ █ █", " ▀ ▀ "],
        'X' => ["█ █", "▄▀▄", "▀ ▀"],
        'Y' => ["█ █", "▀█▀", " ▀ "],
        'Z' => ["▀▀█", "▄▀ ", "▀▀▀"],
        '0' => ["█▀█", "█ █", "▀▀▀"],
        '1' => ["▄█ ", " █ ", "▀▀▀"],
        '2' => ["▀▀█", "█▀▀", "▀▀▀"],
        '3' => ["▀▀█", " ▀█", "▀▀▀"],
        '4' => ["█ █", "▀▀█", "  ▀"],
        '5' => ["█▀▀", "▀▀█", "▀▀▀"],
        '6' => ["█▀▀", "█▀█", "▀▀▀"],
        '7' => ["▀▀█", "  █", "  ▀"],
        '8' => ["█▀█", "█▀█", "▀▀▀"],
        '9' => ["█▀█", "▀▀█", "▀▀▀"],
        '%' => ["▀ █", " █ ", "█ ▄"],
        '$' => ["▄█▀", "▀█▄", "▀▀ "],
        '.' => [" ", " ", "▀"],
        ' ' => ["  ", "  ", "  "],
        _ => return None,
    })
}

/// El texto en letras de bloque, o `None` si no entra en `max` columnas.
fn big_text(s: &str, max: usize) -> Option<[String; 3]> {
    let s: String = s
        .trim_end_matches(['.', ' '])
        .to_uppercase()
        .chars()
        .map(|c| match c {
            'Á' => 'A',
            'É' => 'E',
            'Í' => 'I',
            'Ó' => 'O',
            'Ú' | 'Ü' => 'U',
            'Ñ' => 'N',
            c => c,
        })
        .collect();
    let mut rows = [String::new(), String::new(), String::new()];
    for (i, c) in s.chars().filter_map(big_glyph).enumerate() {
        for k in 0..3 {
            if i > 0 {
                rows[k].push(' ');
            }
            rows[k].push_str(c[k]);
        }
    }
    (rows[0].width() <= max && !rows[0].is_empty()).then_some(rows)
}

/// Fecha local: (año, mes, día, día de la semana con 0 = domingo).
fn local_date() -> (i64, u32, u32, u32) {
    let utc = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // El desfase sale de comparar la hora local con la UTC del día.
    let (h, m, s) = local_hms();
    let local_sod = (h * 3600 + m * 60 + s) as i64;
    let mut off = local_sod - utc.rem_euclid(86_400);
    if off > 43_200 {
        off -= 86_400;
    } else if off < -43_200 {
        off += 86_400;
    }
    let days = (utc + off).div_euclid(86_400);
    let wd = (days + 4).rem_euclid(7) as u32;
    // Días desde 1970 → fecha civil (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + (mo <= 2) as i64;
    (y, mo, d, wd)
}

fn ses8(app: &App) -> String {
    app.session.get(..8).unwrap_or("········").to_string()
}

fn model_short(app: &App) -> String {
    app.model.trim_start_matches("claude-").to_string()
}

// ── Fósforo: un CRT de un solo color ────────────────────────────────────────────────────

fn fosforo(f: &mut Frame, app: &App) {
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
        let mut x = put(buf, head.x, head.y, "JARVIS-OS", hi, head.width);
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
        orden_line(f, input_w, Rect { width: chat.width, ..rule2 }, app, state, t, "JARVIS> ", theme::accent());
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
fn phosphor(buf: &mut Buffer, area: Rect, scan: u16) {
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

// ── Radar: cada herramienta en su sector ────────────────────────────────────────────────

const SECTORES: [&str; 8] = ["LEER", "BUSCAR", "EDITAR", "GIT", "PROBAR", "EJECUTAR", "RED", "AGENTES"];

fn sector(a: &Activity) -> usize {
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

fn radar(f: &mut Frame, app: &App) {
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
        let x = put(buf, head.x, head.y, "◎ JARVIS", Style::new().fg(theme::accent()).bold(), head.width);
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

// ── Editorial: titular, columna de datos, lectura y figura ──────────────────────────────

fn editorial(f: &mut Frame, app: &App) {
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

// ── Zen: un punto que late y la línea de escritura ──────────────────────────────────────

fn zen(f: &mut Frame, app: &App) {
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

// ── Bitácora: todo en orden, línea por línea ────────────────────────────────────────────

fn bitacora(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let inner = Rect { x: area.x + 2, width: area.width.saturating_sub(4), ..area };
    let input_h = if listening(state) { 3 } else { 1 };
    let [head, rule, cols, _, body, rule2, input, _, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(input_h),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x, head.y, "JARVIS", Style::new().fg(theme::accent()).bold(), 6);
        put(buf, x + 1, head.y, "bitácora", Style::new().fg(theme::faint()), 10);
        // El estado como una traza: el nivel de la voz al escuchar, el pulso del núcleo si no.
        let clock = now_hhmmss();
        let n = 30usize;
        let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let trace: String = (0..n)
            .map(|i| {
                let v = if listening(state) {
                    let l = app.levels[app.levels.len().saturating_sub(n) + i.min(n - 1)] as f64;
                    (l * 5.0).sqrt().min(1.0)
                } else {
                    let speed = if app.busy { 3.0 } else { 0.8 };
                    0.5 + 0.45 * ((i as f64 * 0.45) - t * speed).sin() * if app.busy { 1.0 } else { 0.4 }
                };
                bars[((v * 7.0).round() as usize).min(7)]
            })
            .collect();
        put_right(buf, head.right() - 1, head.y, &clock, Style::new().fg(theme::text()));
        let tx = head.right().saturating_sub(clock.len() as u16 + 2 + n as u16);
        put(buf, tx, head.y, &trace, Style::new().fg(tone), n as u16);
        put_right(buf, tx.saturating_sub(2), head.y, "estado", Style::new().fg(theme::faint()));
        hline(buf, rule.x, rule.right() - 1, rule.y, "─", Style::new().fg(theme::dim()));
        let h = Style::new().fg(theme::faint()).bold();
        put(buf, cols.x, cols.y, "HORA", h, 8);
        put(buf, cols.x + 10, cols.y, "TIPO", h, 8);
        put(buf, cols.x + 18, cols.y, "CONTENIDO", h, 20);
        put_right(buf, cols.right() - 1, cols.y, "DURACIÓN", h);
    }

    // Cada mensaje y cada herramienta es una entrada; lo largo sigue debajo, sangrado.
    let w = body.width as usize;
    let cw = w.saturating_sub(18 + 10).max(10);
    let mut rows: Vec<(Line<'static>, usize, bool)> = Vec::new();
    let entry = |rows: &mut Vec<(Line<'static>, usize, bool)>, time: String, tipo: &str, tc: Color, text: &str, fg: Color, dur: Option<(String, Color)>| {
        let lines = wrap_cont(text, cw);
        for (k, (l, cont)) in lines.into_iter().enumerate() {
            let mut spans = vec![
                Span::styled(format!("{:<10}", if k == 0 { time.as_str() } else { "" }), Style::new().fg(theme::dim())),
                Span::styled(format!("{:<8}", if k == 0 { tipo } else { "" }), Style::new().fg(tc).bold()),
                Span::styled(format!("{:<cw$}", l), Style::new().fg(fg)),
            ];
            if k == 0 {
                if let Some((d, c)) = &dur {
                    spans.push(Span::styled(format!("{d:>10}"), Style::new().fg(*c)));
                }
            }
            rows.push((Line::from(spans), 18, cont));
        }
    };
    entry(
        &mut rows,
        clock_ago(app.started.elapsed().as_secs()),
        "SES",
        theme::faint(),
        &format!("sesión {} · {} · {}", ses8(app), model_short(app), cwd()),
        theme::faint(),
        None,
    );
    for m in &app.messages {
        match m.role {
            Role::User if m.waiting.is_some() => entry(&mut rows, String::new(), "COLA", theme::faint(), &m.text, theme::faint(), None),
            Role::User => entry(&mut rows, String::new(), "TÚ", theme::text(), &m.text, theme::text(), None),
            Role::Assistant => entry(&mut rows, String::new(), "JARVIS", theme::accent(), &plain(&m.text), theme::text(), None),
            Role::System => entry(&mut rows, String::new(), "SYS", theme::faint(), &m.text, theme::faint(), None),
            Role::Error => entry(&mut rows, String::new(), "ERROR", RED, &m.text, RED, None),
            Role::Tool => {
                let Some(a) = m.tool.and_then(|i| app.activity.get(i)) else { continue };
                let (icon, c) = act_icon(a, t);
                let dur = Some((format!("{icon} {:.1}s", act_secs(a)), c));
                let name = truncate(&a.name.to_uppercase(), 7);
                entry(&mut rows, clock_ago(a.started.elapsed().as_secs()), &name, theme::text(), &a.detail, theme::faint(), dur);
            }
        }
    }
    if app.busy && app.current_tool().is_none() {
        let dots = ".".repeat((t * 3.0) as usize % 4 + 1);
        entry(&mut rows, String::new(), "PIENSA", tone, &dots, tone, None);
    }
    render_chat(f, body, app, rows);

    {
        let buf = f.buffer_mut();
        hline(buf, rule2.x, rule2.right() - 1, rule2.y, "─", Style::new().fg(theme::dim()));
    }
    if listening(state) {
        wave(f, input, app, t, tone, theme::dim());
    } else {
        orden_line(f, input, rule2, app, state, t, "▶ ", theme::accent());
    }
    let kc = if listening(state) { tone } else { theme::accent() };
    foot_keys(f.buffer_mut(), hints, app, state, kc, false);
}

// ── Tablero: mosaicos ───────────────────────────────────────────────────────────────────

fn tablero(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();
    let area = f.area();
    let tone = app.nucleo.color();
    let inner = Rect { x: area.x + 1, width: area.width.saturating_sub(2), ..area };
    let band_h = (inner.height * 46 / 100).clamp(12, 28);
    let input_h = if listening(state) { 3 } else { 1 };
    let [head, band, chat, input, hints] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(band_h),
        Constraint::Min(5),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .areas(inner);
    {
        let buf = f.buffer_mut();
        let x = put(buf, head.x + 1, head.y, "◆ JARVIS", Style::new().fg(theme::accent()).bold(), 8);
        put(buf, x + 2, head.y, "tablero de la sesión", Style::new().fg(theme::faint()), 22);
        put_right(buf, head.right() - 1, head.y, &format!("{} · {} · {}", model_short(app), ses8(app), now_hhmmss()), Style::new().fg(theme::faint()));
    }

    let [core_t, _, right] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Length(1), Constraint::Min(30)]).areas(band);
    let [r_top, r_bot] = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(right);
    let [ctx_t, _, cost_t] = Layout::horizontal([Constraint::Percentage(50), Constraint::Length(1), Constraint::Min(10)]).areas(r_top);
    let [tools_t, _, ses_t] = Layout::horizontal([Constraint::Percentage(50), Constraint::Length(1), Constraint::Min(10)]).areas(r_bot);

    let dim = Style::new().fg(theme::dim());
    // Núcleo.
    {
        let buf = f.buffer_mut();
        corners(buf, core_t, dim, Some(&spaced("NÚCLEO")));
    }
    let core = Rect { x: core_t.x + 1, y: core_t.y + 1, width: core_t.width.saturating_sub(2), height: core_t.height.saturating_sub(4) };
    core_draw(app, core, f.buffer_mut());
    let buf = f.buffer_mut();
    let (title, detail) = core_label(app, t, core_t.width as usize);
    put_center(buf, core_t, core_t.bottom().saturating_sub(3), &spaced_title(&title), Style::new().fg(tone).bold());
    put_center(buf, core_t, core_t.bottom().saturating_sub(2), &truncate(&detail, core_t.width as usize - 2), Style::new().fg(theme::faint()));

    // Contexto: el número grande y el medidor.
    corners(buf, ctx_t, dim, Some(&spaced("CONTEXTO")));
    let pct = ctx_pct(app);
    let big_row = |buf: &mut Buffer, r: Rect, s: &str, col: Color| {
        match big_text(s, r.width.saturating_sub(4) as usize) {
            Some(rows) if r.height >= 6 => {
                for (k, row) in rows.iter().enumerate() {
                    put(buf, r.x + 3, r.y + 2 + k as u16, row, Style::new().fg(col).bold(), r.width.saturating_sub(4));
                }
            }
            _ => {
                put(buf, r.x + 3, r.y + 2, s, Style::new().fg(col).bold(), r.width.saturating_sub(4));
            }
        }
    };
    big_row(buf, ctx_t, &format!("{pct:.0}%"), theme::text());
    if ctx_t.height >= 7 {
        let y = ctx_t.bottom() - 2;
        let w = ctx_t.width.saturating_sub(6);
        meter(buf, ctx_t.x + 3, y, w, pct / 100.0, tone);
    }

    // Costo y turnos.
    corners(buf, cost_t, dim, Some(&spaced("COSTO")));
    big_row(buf, cost_t, &format!("${:.2}", app.cost), theme::text());
    if cost_t.height >= 7 {
        put(buf, cost_t.x + 3, cost_t.bottom() - 2, &turns(app), Style::new().fg(theme::faint()), cost_t.width.saturating_sub(4));
    }

    // Herramientas: cuántas veces cada una, en barras.
    corners(buf, tools_t, dim, Some(&spaced("HERRAMIENTAS")));
    let mut counts: Vec<(String, usize, usize)> = Vec::new();
    for a in &app.activity {
        match counts.iter_mut().find(|c| c.0 == a.name) {
            Some(c) => {
                c.1 += 1;
                c.2 += (a.status == ToolStatus::Err) as usize;
            }
            None => counts.push((a.name.clone(), 1, (a.status == ToolStatus::Err) as usize)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let rows = tools_t.height.saturating_sub(3) as usize;
    if counts.is_empty() {
        put(buf, tools_t.x + 3, tools_t.y + 2, "— ninguna todavía —", Style::new().fg(theme::dim()), tools_t.width);
    }
    let max = counts.first().map_or(1, |c| c.1).max(1);
    let bar_w = tools_t.width.saturating_sub(22) as usize;
    for (i, (name, n, err)) in counts.iter().take(rows).enumerate() {
        let y = tools_t.y + 2 + i as u16;
        let x = put(buf, tools_t.x + 3, y, &format!("{:<7}", truncate(name, 7)), Style::new().fg(theme::text()), 7);
        let len = (n * bar_w / max).max(1);
        let x = put(buf, x + 1, y, &"█".repeat(len), Style::new().fg(if *err > 0 && *err == *n { RED } else { tone }), len as u16);
        let tail = if *err > 0 { format!("{n} · {err} err") } else { n.to_string() };
        put(buf, x + 1, y, &tail, Style::new().fg(theme::faint()), 12);
    }

    // Sesión.
    corners(buf, ses_t, dim, Some(&spaced("SESIÓN")));
    let items = [("modelo", model_short(app)), ("voz", voice_value(app)), ("en línea", uptime(app)), ("sesión", ses8(app))];
    for (i, (k, v)) in items.iter().enumerate() {
        let y = ses_t.y + 2 + i as u16;
        if y + 1 >= ses_t.bottom() {
            break;
        }
        put(buf, ses_t.x + 3, y, k, Style::new().fg(theme::faint()), 10);
        put(buf, ses_t.x + 13, y, v, Style::new().fg(theme::text()), ses_t.width.saturating_sub(15));
    }

    // Conversación.
    corners(buf, chat, dim, Some(&spaced("CONVERSACIÓN")));
    let chat_in = Rect { x: chat.x + 2, y: chat.y + 1, width: chat.width.saturating_sub(4), height: chat.height.saturating_sub(2) };
    if app.messages.is_empty() {
        *app.view.borrow_mut() = Default::default();
        put(buf, chat_in.x + GUION as u16, chat_in.y + 1, &format!("{}. ¿En qué trabajamos?", greeting()), Style::new().fg(theme::text()).bold(), chat_in.width);
    } else {
        render_chat(f, chat_in, app, chat_rows(app, chat_in.width as usize, Voz::Guion));
    }

    if listening(state) {
        wave(f, Rect { x: input.x + 1, width: input.width.saturating_sub(2), ..input }, app, t, tone, theme::dim());
    } else {
        orden_line(f, Rect { x: input.x + 1, ..input }, Rect { y: chat.bottom().saturating_sub(1), height: 1, ..chat }, app, state, t, "› ", theme::accent());
    }
    let kc = if listening(state) { tone } else { theme::accent() };
    foot_keys(f.buffer_mut(), Rect { x: hints.x + 1, ..hints }, app, state, kc, false);
}
