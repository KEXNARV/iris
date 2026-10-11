//! What comes in from Claude (text, tools, subagents, cost) and how it changes the state.

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
                    for sentence in self.reader.push(&t) {
                        self.speech.say(&sentence);
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
                // In the chat, at its place in the conversation.
                let idx = self.activity.len() - 1;
                self.messages.push(Msg { role: Role::Tool, text: String::new(), waiting: None, images: vec![], tool: Some(idx) });
            }
            ClaudeEvent::ToolResult { id, is_error, output } => {
                if let Some(a) = self.activity.iter_mut().rev().find(|a| a.id == id) {
                    a.output = output;
                    a.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                    a.took = Some(a.started.elapsed());
                    let kind = core::tool_state(&a.name, &a.detail);
                    if is_error {
                        self.core.fire(Gesture::Error);
                    } else if kind == State::Testing {
                        self.core.fire(Gesture::Pass);
                    } else if kind == State::Delegating && !self.agents.iter().any(|g| g.tool == id) {
                        // No kid in the core (an engine that doesn't report tasks): the usual drop.
                        self.core.fire(Gesture::Merge);
                    }
                }
                self.thinking = true;
            }
            ClaudeEvent::Done { cost, secs, is_error, window } => {
                if self.floating {
                    self.hide_at = Some(Instant::now() + Duration::from_secs(4));
                } else if !self.focused && !self.interrupted {
                    // In another window: let them know it finished.
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
                    for sentence in self.reader.finish() {
                        self.speech.say(&sentence);
                    }
                }
                if let Some(w) = window {
                    self.ctx_window = w;
                }
                self.compacting = false;
                if is_error && !self.interrupted {
                    self.core.fire(Gesture::Error);
                } else if !is_error {
                    self.core.fire(Gesture::Done);
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
            ClaudeEvent::Ask { request_id, tool, input, suggestions } => {
                if !self.focused && !self.floating {
                    notify("Iris te pregunta algo", "Necesita tu respuesta para seguir.");
                }
                self.modal = Some(Modal::Ask(Ask::new(request_id, tool, input, suggestions)));
            }
            ClaudeEvent::Taken(id) => {
                for m in self.messages.iter_mut().filter(|m| m.waiting.as_deref() == Some(&id)) {
                    m.waiting = None;
                }
            }
            ClaudeEvent::Usage(n) => {
                self.ctx_used = n;
                // The real window only arrives at the end of the turn; until then it's assumed 200k,
                // and a resumed session can already exceed it (it showed «235%»): then it is 1M.
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
                self.core.kid_born(kid);
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
                    a.log.push(Step::Tool(Activity {
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
                    self.core.kid_pulse(a.kid, false);
                }
            }
            ClaudeEvent::AgentToolResult { tool, id, is_error, output } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    let step = a.log.iter_mut().rev().find_map(|p| match p {
                        Step::Tool(t) if t.id == id => Some(t),
                        _ => None,
                    });
                    if let Some(t) = step {
                        t.status = if is_error { ToolStatus::Err } else { ToolStatus::Ok };
                        t.took = Some(t.started.elapsed());
                        t.output = output;
                    }
                    if let Some((name, detail)) = a.current.take() {
                        a.last = Some((name, detail, Instant::now()));
                    }
                    self.core.kid_pulse(a.kid, is_error);
                }
            }
            ClaudeEvent::AgentText { tool, text } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.log.push(Step::Text(text));
                }
            }
            ClaudeEvent::AgentDone { tool, ok } => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.tool == tool && a.ended.is_none()) {
                    a.ended = Some((Instant::now(), ok));
                    a.current = None;
                    self.core.kid_end(a.kid, ok);
                }
            }
            ClaudeEvent::Stderr(line) => self.push(Role::Error, line),
            ClaudeEvent::Exited => {
                self.agents.clear();
                self.core.kids_clear();
                self.modal = None;
                self.drop_waiting();
                self.alive = false;
                self.busy = false;
                self.push(Role::Error, "el proceso de claude terminó. Ctrl+R para reiniciar.");
            }
        }
    }
}
