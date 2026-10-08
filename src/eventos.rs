//! Lo que llega de Claude (texto, herramientas, subagentes, costo) y cómo cambia el estado.

use crate::*;

impl App {
    pub(crate) fn on_claude(&mut self, ev: ClaudeEvent) {
        self.last_activity = Instant::now();
        match ev {
            ClaudeEvent::Init { model, session, skills } => {
                self.model = model;
                self.session = session;
                commands::add_skills(&mut self.commands, &skills);
            }
            ClaudeEvent::Thinking => self.thinking = true,
            ClaudeEvent::Text(t) => {
                self.thinking = false;
                if self.speak_turn {
                    for frase in self.lector.push(&t) {
                        self.habla.say(&frase);
                    }
                }
                if self.assistant_open {
                    self.messages.last_mut().unwrap().text.push_str(&t);
                } else {
                    self.push(Role::Assistant, t.trim_start());
                    self.assistant_open = true;
                }
            }
            ClaudeEvent::ToolUse { id, name, detail, input } => {
                self.thinking = false;
                self.assistant_open = false;
                if name == "TodoWrite" {
                    let todos = input["todos"].as_array().map(Vec::as_slice).unwrap_or_default();
                    let done = todos.iter().filter(|t| t["status"] == "completed").count();
                    self.todos = (todos.len(), done);
                }
                self.activity.push(Activity {
                    id,
                    name,
                    detail,
                    status: ToolStatus::Running,
                    started: Instant::now(),
                    took: None,
                    input,
                    output: String::new(),
                });
                // En el chat, en su lugar de la conversación.
                let idx = self.activity.len() - 1;
                self.messages.push(Msg { role: Role::Tool, text: String::new(), waiting: None, images: vec![], tool: Some(idx) });
            }
            ClaudeEvent::ToolResult { id, is_error, output } => {
                if let Some(a) = self.activity.iter_mut().rev().find(|a| a.id == id) {
                    a.output = output;
                    a.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                    a.took = Some(a.started.elapsed());
                    let kind = nucleo::tool_state(&a.name, &a.detail);
                    if is_error {
                        self.nucleo.fire(Gesto::Error);
                    } else if kind == State::Testing {
                        self.nucleo.fire(Gesto::Pass);
                    } else if kind == State::Delegating && !self.agents.iter().any(|g| g.tool == id) {
                        // Sin hijo en el núcleo (un motor que no avisa de sus tareas): la gota de siempre.
                        self.nucleo.fire(Gesto::Merge);
                    }
                }
                self.thinking = true;
            }
            ClaudeEvent::Done { cost, secs, is_error, window } => {
                if self.flotante {
                    self.hide_at = Some(Instant::now() + Duration::from_secs(4));
                } else if !self.focused && !self.interrupted {
                    // En otra ventana: que se entere de que terminó.
                    let first = self
                        .messages
                        .iter()
                        .rev()
                        .find(|m| m.role == Role::Assistant)
                        .and_then(|m| m.text.lines().find(|l| !l.trim().is_empty()))
                        .unwrap_or("")
                        .replace(['*', '`', '#'], "");
                    let title = if is_error { "Iris: el turno terminó con error" } else { "Iris terminó" };
                    notify(title, &first);
                }
                if std::mem::take(&mut self.speak_turn) {
                    for frase in self.lector.finish() {
                        self.habla.say(&frase);
                    }
                }
                if let Some(w) = window {
                    self.ctx_window = w;
                }
                self.compacting = false;
                if is_error && !self.interrupted {
                    self.nucleo.fire(Gesto::Error);
                } else if !is_error {
                    self.nucleo.fire(Gesto::Done);
                }
                self.busy = false;
                self.thinking = false;
                self.assistant_open = false;
                self.cost = cost.max(self.cost);
                self.turns += 1;
                for a in self.activity.iter_mut().filter(|a| a.status == ToolStatus::Running) {
                    a.status = ToolStatus::Err;
                }
                if is_error && !std::mem::take(&mut self.interrupted) {
                    self.push(Role::Error, format!("el turno terminó con error ({secs:.1}s)"));
                }
            }
            ClaudeEvent::Ask { request_id, tool, input } => {
                if !self.focused && !self.flotante {
                    notify("Iris te pregunta algo", "Necesita tu respuesta para seguir.");
                }
                self.modal = Some(Modal::Ask(Ask::new(request_id, tool, input)));
            }
            ClaudeEvent::Taken(id) => {
                for m in self.messages.iter_mut().filter(|m| m.waiting.as_deref() == Some(&id)) {
                    m.waiting = None;
                }
            }
            ClaudeEvent::Usage(n) => {
                self.ctx_used = n;
                // La ventana real llega recién al terminar el turno; mientras tanto se supone de
                // 200k, y una sesión retomada ya puede pasarla (salía «235%»): entonces es de 1M.
                if n > self.ctx_window {
                    self.ctx_window = 1_000_000;
                }
            }
            ClaudeEvent::Compacted { pre, post } => {
                self.compacting = false;
                self.ctx_used = post;
                let k = |n: u64| format!("{:.1}k", n as f64 / 1000.0);
                self.push(Role::System, format!("contexto compactado: {} → {} tokens", k(pre), k(post)));
            }
            ClaudeEvent::AgentStarted { tool, description, kind } => {
                let kid = self.next_kid;
                self.next_kid += 1;
                self.nucleo.kid_born(kid);
                self.agents.push(Agent {
                    tool,
                    kid,
                    description,
                    kind,
                    started: Instant::now(),
                    current: None,
                    last: None,
                    tools: 0,
                    ended: None,
                    log: vec![],
                });
            }
            ClaudeEvent::AgentTool { tool, id, name, detail, input } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.log.push(Paso::Herramienta(Activity {
                        id,
                        name: name.clone(),
                        detail: detail.clone(),
                        status: ToolStatus::Running,
                        started: Instant::now(),
                        took: None,
                        input,
                        output: String::new(),
                    }));
                    a.current = Some((name, detail));
                    a.tools += 1;
                    self.nucleo.kid_pulse(a.kid, false);
                }
            }
            ClaudeEvent::AgentToolResult { tool, id, is_error, output } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    let paso = a.log.iter_mut().rev().find_map(|p| match p {
                        Paso::Herramienta(t) if t.id == id => Some(t),
                        _ => None,
                    });
                    if let Some(t) = paso {
                        t.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                        t.took = Some(t.started.elapsed());
                        t.output = output;
                    }
                    if let Some((name, detail)) = a.current.take() {
                        a.last = Some((name, detail, Instant::now()));
                    }
                    self.nucleo.kid_pulse(a.kid, is_error);
                }
            }
            ClaudeEvent::AgentText { tool, text } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.log.push(Paso::Texto(text));
                }
            }
            ClaudeEvent::AgentDone { tool, ok } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.ended = Some((Instant::now(), ok));
                    a.current = None;
                    self.nucleo.kid_end(a.kid, ok);
                }
            }
            ClaudeEvent::Stderr(line) => self.push(Role::Error, line),
            ClaudeEvent::Exited => {
                self.agents.clear();
                self.nucleo.kids_clear();
                self.modal = None;
                self.drop_waiting();
                self.alive = false;
                self.busy = false;
                self.push(Role::Error, "el proceso de claude terminó. Ctrl+R para reiniciar.");
            }
        }
    }
}
