use std::f64::consts::TAU;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Circle, Points};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::{App, Modal, Role, ToolStatus, VoiceState};

const ACCENT: Color = Color::Rgb(0, 200, 255);
const DIM: Color = Color::Rgb(40, 90, 120);
const FAINT: Color = Color::Rgb(90, 110, 125);
const TEXT: Color = Color::Rgb(205, 225, 235);
const WARM: Color = Color::Rgb(255, 170, 40);
const MAGENTA: Color = Color::Rgb(200, 110, 255);
const RED: Color = Color::Rgb(255, 90, 90);
const GREEN: Color = Color::Rgb(90, 230, 150);

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Idle,
    Listening,
    Transcribing,
    Thinking,
    Working,
    Speaking,
    Asking,
    Offline,
}

impl State {
    fn label(self) -> &'static str {
        match self {
            State::Idle => "EN ESPERA",
            State::Listening => "ESCUCHANDO",
            State::Transcribing => "TRANSCRIBIENDO",
            State::Thinking => "PENSANDO",
            State::Working => "TRABAJANDO",
            State::Speaking => "RESPONDIENDO",
            State::Asking => "ESPERANDO RESPUESTA",
            State::Offline => "DESCONECTADO",
        }
    }

    fn color(self) -> Color {
        match self {
            State::Idle => ACCENT,
            State::Listening => WARM,
            State::Transcribing => MAGENTA,
            State::Thinking => Color::Rgb(90, 150, 255),
            State::Working => ACCENT,
            State::Speaking => Color::Rgb(160, 235, 255),
            State::Asking => WARM,
            State::Offline => FAINT,
        }
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let t = app.started.elapsed().as_secs_f64();
    let state = app.state();

    let [header, body, bottom, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(if state == State::Listening { 5 } else { 3 }),
        Constraint::Length(1),
    ])
    .areas(f.area());

    draw_header(f, header, app);

    let side_w = (body.width / 3).clamp(30, 46);
    let [chat, side] = Layout::horizontal([Constraint::Min(30), Constraint::Length(side_w)]).areas(body);
    draw_chat(f, chat, app);

    let core_h = (side.width / 2 + 3).min(side.height / 2).max(10);
    let [core, act] = Layout::vertical([Constraint::Length(core_h), Constraint::Min(4)]).areas(side);
    draw_core(f, core, app, state, t);
    draw_activity(f, act, app, t);

    if state == State::Listening {
        draw_wave(f, bottom, app, t);
    } else {
        draw_input(f, bottom, app, state, t);
        let anchor = Rect { width: chat.width, ..bottom };
        match &app.modal {
            Some(Modal::Sessions { list, sel }) => draw_sessions(f, anchor, list, *sel),
            Some(Modal::Ask(a)) => draw_ask(f, anchor, a),
            Some(Modal::Model { sel }) => draw_models(f, anchor, app, *sel),
            None => draw_menu(f, anchor, app),
        }
    }
    draw_footer(f, footer, app);
}

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(DIM))
        .title(Span::styled(format!(" {title} "), Style::new().fg(ACCENT).bold()))
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let home = std::env::var("HOME").unwrap_or_default();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string().replacen(&home, "~", 1))
        .unwrap_or_default();
    let model = app.model.trim_start_matches("claude-");
    let sess = app.session.get(..8).unwrap_or("········");
    let left = Line::from(vec![
        Span::styled(" ◆ J.A.R.V.I.S ", Style::new().fg(Color::Black).bg(ACCENT).bold()),
        Span::styled(format!("  {model}"), Style::new().fg(TEXT)),
        Span::styled(format!("  ·  {cwd}  ·  sesión {sess}"), Style::new().fg(FAINT)),
    ]);
    f.render_widget(Paragraph::new(left), area);
    let clock = Line::from(Span::styled(format!("{} ", now_hhmmss()), Style::new().fg(ACCENT)));
    f.render_widget(Paragraph::new(clock).alignment(Alignment::Right), area);
}

fn draw_core(f: &mut Frame, area: Rect, app: &App, state: State, t: f64) {
    let block = panel("NÚCLEO");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    let [ring, label] = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(inner);

    // Celdas ~1:2 y braille 2×4 → puntos casi cuadrados; se corrige el aspecto para que el círculo sea redondo.
    let aspect = ring.width as f64 / (ring.height as f64 * 2.0);
    let color = state.color();
    let level = crate::voice::level_get(&app.level) as f64;

    let (speed, pulse) = match state {
        State::Idle => (0.25, 0.03 * (t * 1.2).sin()),
        State::Listening => (0.6, (level * 6.0).min(0.25)),
        State::Transcribing => (2.2, 0.04 * (t * 6.0).sin()),
        State::Thinking => (1.4, 0.05 * (t * 3.0).sin()),
        State::Working => (2.0, 0.04 * (t * 4.0).sin()),
        State::Speaking => (0.9, 0.06 * (t * 5.0).sin().abs()),
        State::Asking => (0.15, 0.05 * (t * 2.0).sin()),
        State::Offline => (0.0, 0.0),
    };
    let rot = t * speed;

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([-aspect, aspect])
        .y_bounds([-1.0, 1.0])
        .paint(move |ctx| {
            ctx.draw(&Circle { x: 0.0, y: 0.0, radius: 0.94, color: DIM });

            // Arcos que giran en sentidos opuestos.
            let arc = |r: f64, start: f64, len: f64, n: usize| -> Vec<(f64, f64)> {
                (0..n)
                    .map(|i| {
                        let a = start + len * i as f64 / n as f64;
                        (r * a.cos(), r * a.sin())
                    })
                    .collect()
            };
            for k in 0..3 {
                let s = rot + k as f64 * TAU / 3.0;
                ctx.draw(&Points { coords: &arc(0.82, s, TAU / 5.0, 60), color });
            }
            for k in 0..6 {
                let s = -rot * 1.6 + k as f64 * TAU / 6.0;
                ctx.draw(&Points { coords: &arc(0.66 + pulse, s, TAU / 14.0, 20), color });
            }

            // Marcas de escala fijas.
            let ticks: Vec<(f64, f64)> = (0..24)
                .flat_map(|i| {
                    let a = i as f64 * TAU / 24.0;
                    [(0.94 * a.cos(), 0.94 * a.sin()), (0.88 * a.cos(), 0.88 * a.sin())]
                })
                .collect();
            ctx.draw(&Points { coords: &ticks, color: DIM });

            // Núcleo.
            let core = 0.30 + pulse * 1.5;
            for i in 0..5 {
                ctx.draw(&Circle { x: 0.0, y: 0.0, radius: core * (1.0 - i as f64 * 0.2), color });
            }
            ctx.draw(&Circle { x: 0.0, y: 0.0, radius: core + 0.1, color: DIM });
        });
    f.render_widget(canvas, ring);

    let detail = match state {
        State::Working => app
            .current_tool()
            .map(|a| format!("{} · {:.0}s", a.name, a.started.elapsed().as_secs_f64()))
            .unwrap_or_default(),
        State::Listening if app.ptt.is_some() => "suelta espacio para enviar · esc cancela".into(),
        State::Listening => "espacio para enviar · esc cancela".into(),
        State::Asking => "elige con ↑↓ y enter".into(),
        State::Idle if app.voice == VoiceState::Ready => "mantén o toca espacio para hablar".into(),
        State::Idle if app.voice == VoiceState::Loading => "cargando voz…".into(),
        _ => String::new(),
    };
    let dots = if matches!(state, State::Idle | State::Offline) {
        ""
    } else {
        ["   ", ".  ", ".. ", "..."][(t * 3.0) as usize % 4]
    };
    let lines = vec![
        Line::from(Span::styled(format!("{}{dots}", state.label()), Style::new().fg(color).bold())),
        Line::from(Span::styled(detail, Style::new().fg(FAINT))),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), label);
}

fn draw_activity(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("ACTIVIDAD");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width as usize;
    let spinner = ["◐", "◓", "◑", "◒"][(t * 8.0) as usize % 4];

    let lines: Vec<Line> = app
        .activity
        .iter()
        .rev()
        .take(inner.height as usize)
        .map(|a| {
            let (icon, c) = match a.status {
                ToolStatus::Running => (spinner, WARM),
                ToolStatus::Ok => ("✓", GREEN),
                ToolStatus::Err => ("✗", RED),
            };
            let secs = a.took.unwrap_or_else(|| a.started.elapsed()).as_secs_f64();
            let time = format!(" {secs:.1}s");
            let head = format!("{icon} {} ", a.name);
            let room = w.saturating_sub(head.chars().count() + time.len());
            Line::from(vec![
                Span::styled(format!("{icon} "), Style::new().fg(c)),
                Span::styled(format!("{} ", a.name), Style::new().fg(TEXT).bold()),
                Span::styled(format!("{:<room$}", truncate(&a.detail, room)), Style::new().fg(FAINT)),
                Span::styled(time, Style::new().fg(DIM)),
            ])
        })
        .collect();

    if lines.is_empty() {
        let p = Paragraph::new(Span::styled("sin actividad todavía", Style::new().fg(DIM)));
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

    let mut lines: Vec<Line> = Vec::new();
    if app.messages.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("  {}. ¿En qué trabajamos?", greeting()),
            Style::new().fg(TEXT).bold(),
        )));
        lines.push(Line::from(Span::styled(
            "  Escribe, o presiona espacio y háblame.",
            Style::new().fg(FAINT),
        )));
    }

    for m in &app.messages {
        match m.role {
            Role::User => {
                lines.push(Line::default());
                let mut head = vec![Span::styled("  TÚ", Style::new().fg(WARM).bold())];
                if m.waiting.is_some() {
                    head.push(Span::styled("  ◷ en espera", Style::new().fg(FAINT)));
                    head.push(Span::styled(" — Claude lo lee al terminar lo que está haciendo", Style::new().fg(DIM)));
                }
                lines.push(Line::from(head));
                let fg = if m.waiting.is_some() { FAINT } else { TEXT };
                for l in wrap(&m.text, width) {
                    lines.push(Line::from(Span::styled(format!("  {l}"), Style::new().fg(fg))));
                }
            }
            Role::Assistant => {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled("  JARVIS", Style::new().fg(ACCENT).bold())));
                lines.extend(markdown(&m.text, width));
            }
            Role::System => {
                for (i, l) in m.text.lines().flat_map(|l| wrap(l, width)).enumerate() {
                    let mark = if i == 0 { "·" } else { " " };
                    lines.push(Line::from(Span::styled(format!("  {mark} {l}"), Style::new().fg(FAINT))));
                }
            }
            Role::Error => {
                for l in wrap(&m.text, width) {
                    lines.push(Line::from(Span::styled(format!("  ! {l}"), Style::new().fg(RED))));
                }
            }
        }
    }

    let h = inner.height as usize;
    let max_scroll = lines.len().saturating_sub(h);
    let scroll = app.scroll.min(max_scroll);
    let start = lines.len().saturating_sub(h + scroll);
    let visible: Vec<Line> = lines.into_iter().skip(start).take(h).collect();
    f.render_widget(Paragraph::new(visible), inner);

    if scroll > 0 {
        let tag = Span::styled(format!(" ↑ {scroll} líneas · End para volver "), Style::new().fg(Color::Black).bg(DIM));
        let r = Rect { y: area.y + area.height - 1, height: 1, ..area };
        f.render_widget(Paragraph::new(tag).alignment(Alignment::Right), r);
    }
}

/// Markdown mínimo: bloques de código, títulos, viñetas, **negrita** y `código`.
fn markdown(text: &str, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_code = false;
    for raw in text.lines() {
        if raw.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            out.push(Line::from(vec![
                Span::styled("  │ ", Style::new().fg(DIM)),
                Span::styled(truncate(raw, width.saturating_sub(2)), Style::new().fg(GREEN)),
            ]));
            continue;
        }
        let t = raw.trim_start();
        if let Some(h) = t.strip_prefix("### ").or(t.strip_prefix("## ")).or(t.strip_prefix("# ")) {
            out.push(Line::from(Span::styled(format!("  {h}"), Style::new().fg(ACCENT).bold())));
            continue;
        }
        let (bullet, body) = match t.strip_prefix("- ").or(t.strip_prefix("* ")) {
            Some(b) => ("• ", b),
            None => ("", raw),
        };
        let indent = if bullet.is_empty() { "  " } else { "    " };
        let mut bold = false;
        let mut code = false;
        for (i, piece) in wrap(body, width.saturating_sub(bullet.len())).into_iter().enumerate() {
            let prefix = if i == 0 && !bullet.is_empty() { format!("  {bullet}") } else { indent.to_string() };
            let mut spans = vec![Span::styled(prefix, Style::new().fg(ACCENT))];
            spans.extend(inline(&piece, &mut bold, &mut code));
            out.push(Line::from(spans));
        }
    }
    out
}

fn inline(s: &str, bold: &mut bool, code: &mut bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let style = |b: bool, c: bool| {
        let mut st = Style::new().fg(if c { GREEN } else { TEXT });
        if b {
            st = st.add_modifier(Modifier::BOLD).fg(Color::White);
        }
        st
    };
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        let toggle = match ch {
            '*' if chars.peek() == Some(&'*') && !*code => {
                chars.next();
                Some(true)
            }
            '`' => Some(false),
            _ => None,
        };
        match toggle {
            Some(is_bold) => {
                if !buf.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut buf), style(*bold, *code)));
                }
                if is_bold {
                    *bold = !*bold;
                } else {
                    *code = !*code;
                }
            }
            None => buf.push(ch),
        }
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, style(*bold, *code)));
    }
    spans
}

fn draw_wave(f: &mut Frame, area: Rect, app: &App, t: f64) {
    let block = panel("ESCUCHANDO").border_style(Style::new().fg(WARM));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Onda espejada: cada columna es un instante; la altura sale del RMS.
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
                    let c = if dist < 0.6 { WARM } else { Color::Rgb(200, 120, 30) };
                    Span::styled(bars[fill].to_string(), Style::new().fg(c))
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(rows), inner);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App, state: State, t: f64) {
    let border = if app.busy { DIM } else { ACCENT };
    let block = panel("ORDEN").border_style(Style::new().fg(border));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cursor = if (t * 2.0) as usize % 2 == 0 { "▏" } else { " " };
    let w = inner.width.saturating_sub(4) as usize;
    let shown: String = {
        let n = app.input.chars().count();
        app.input.chars().skip(n.saturating_sub(w)).collect()
    };
    let mut spans = vec![Span::styled(" › ", Style::new().fg(ACCENT).bold())];
    if app.input.is_empty() {
        spans.push(Span::styled(cursor, Style::new().fg(ACCENT)));
        let hint = match (state, &app.voice) {
            (State::Transcribing, _) => "transcribiendo…",
            (State::Asking, _) => "elige arriba, o escribe aquí otra respuesta",
            (_, _) if app.busy => "trabajando — esc para interrumpir",
            (_, VoiceState::Ready) => "escribe, o espacio para hablar",
            _ => "escribe una orden",
        };
        spans.push(Span::styled(hint, Style::new().fg(DIM)));
    } else {
        spans.push(Span::styled(shown, Style::new().fg(TEXT)));
        spans.push(Span::styled(cursor, Style::new().fg(ACCENT)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), inner);
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
        foot.push(Span::styled(format!("{d} "), Style::new().fg(FAINT)));
    }
    let block = panel(title).border_style(Style::new().fg(color)).title_bottom(Line::from(foot));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let room = (inner.height as usize).saturating_sub(head.len());
    let first = (sel + 1).saturating_sub(room);
    let mut lines = head;
    lines.extend(items.into_iter().skip(first).take(room));
    f.render_widget(Paragraph::new(lines), inner);
}

/// Una fila elegible: marca, nombre a ancho fijo y una descripción que se recorta.
fn item(on: bool, mark: &str, name: &str, name_w: usize, desc: &str, w: usize) -> Line<'static> {
    let bg = if on { Style::new().bg(Color::Rgb(15, 45, 65)) } else { Style::new() };
    let cursor = if on { " › " } else { "   " };
    let name = format!("{mark}{:<name_w$}", truncate(name, name_w));
    let room = w.saturating_sub(3 + name.chars().count() + 2);
    Line::from(vec![
        Span::styled(cursor, bg.fg(ACCENT).bold()),
        Span::styled(name, bg.fg(if on { Color::White } else { ACCENT }).bold()),
        Span::styled(format!("  {:<room$}", truncate(desc, room)), bg.fg(if on { TEXT } else { FAINT })),
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
    overlay(f, anchor, "COMANDOS", ACCENT, vec![], items, sel, 8, &hints);
}

fn draw_sessions(f: &mut Frame, anchor: Rect, list: &[crate::sessions::Session], sel: usize) {
    let w = anchor.width.saturating_sub(2) as usize;
    let items = list
        .iter()
        .enumerate()
        .map(|(i, s)| item(i == sel, "", &s.age, 11, &format!("{}  {}", &s.id[..8], s.title), w))
        .collect();
    let hints = [("↑↓", "elegir"), ("enter", "retomar"), ("esc", "cerrar")];
    overlay(f, anchor, "SESIONES", ACCENT, vec![], items, sel, 12, &hints);
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
    overlay(f, anchor, "MODELO", ACCENT, vec![], items, sel, 8, &hints);
}

fn draw_ask(f: &mut Frame, anchor: Rect, a: &crate::ask::Ask) {
    let q = a.current();
    let w = anchor.width.saturating_sub(2) as usize;
    let mut head = vec![];
    let step = if a.questions.len() > 1 { format!("  {}/{}", a.step + 1, a.questions.len()) } else { String::new() };
    head.push(Line::from(vec![
        Span::styled(format!(" {} ", q.header.to_uppercase()), Style::new().fg(Color::Black).bg(WARM).bold()),
        Span::styled(step, Style::new().fg(FAINT)),
    ]));
    for l in wrap(&q.text, w.saturating_sub(2)) {
        head.push(Line::from(Span::styled(format!(" {l}"), Style::new().fg(TEXT).bold())));
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
    let k = |s: &'static str| Span::styled(s, Style::new().fg(ACCENT));
    let d = |s: &'static str| Span::styled(s, Style::new().fg(FAINT));
    let hints = Line::from(vec![
        k(" espacio"), d(" hablar  "),
        k("enter"), d(" enviar  "),
        k("esc"), d(" cancelar  "),
        k("pgup/pgdn"), d(" desplazar  "),
        k("^L"), d(" limpiar  "),
        k("^R"), d(" reiniciar  "),
        k("^C"), d(" salir"),
    ]);
    let voice = match app.voice {
        VoiceState::Off => "voz off".to_string(),
        VoiceState::Loading => "voz cargando".to_string(),
        _ => format!("voz {}", app.voice_model),
    };
    let up = app.started.elapsed().as_secs();
    let stats = format!(
        "   {voice} · {} turno{} · ${:.2} · {:02}:{:02} ",
        app.turns,
        if app.turns == 1 { "" } else { "s" },
        app.cost,
        up / 3600,
        up / 60 % 60
    );
    let sw = stats.chars().count() as u16;
    let [left, right] = Layout::horizontal([Constraint::Min(0), Constraint::Length(sw)]).areas(area);
    f.render_widget(Paragraph::new(hints), left);
    f.render_widget(Paragraph::new(Span::styled(stats, Style::new().fg(DIM))), right);
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
