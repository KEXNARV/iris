//! Questions the engine asks the user (`AskUserQuestion`, or a permission) that wait for an
//! answer before continuing.

use serde_json::{Map, Value};

pub struct Question {
    pub header: String,
    pub text: String,
    pub multi: bool,
    /// (label, description)
    pub options: Vec<(String, String)>,
}

pub struct Ask {
    pub request_id: String,
    /// A permission for another tool, not an `AskUserQuestion`.
    pub permission: bool,
    input: Value,
    /// The rules proposed by the engine, already scoped to this session; empty if it proposed none.
    rules: Value,
    pub questions: Vec<Question>,
    pub step: usize,
    pub sel: usize,
    pub checked: Vec<bool>,
    answers: Map<String, Value>,
}

pub enum Outcome {
    Next,
    Allow(Value),
    /// Allow and don't ask again about the same thing in this session: (input, rules).
    Always(Value, Value),
    Deny(String),
}

const ALLOW: &str = "Permitir";
const ALWAYS: &str = "Siempre";

impl Ask {
    pub fn new(request_id: String, tool: String, input: Value, suggestions: Value) -> Self {
        let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").to_string();
        let permission = tool != "AskUserQuestion";
        let rules = if permission { session_rules(&suggestions) } else { Value::Null };
        let questions = if permission {
            let mut options = vec![(ALLOW.into(), "una vez".into())];
            if !rules.is_null() {
                options.push((ALWAYS.into(), format!("en esta sesión: {}", summary(&rules))));
            }
            options.push(("Rechazar".into(), "Claude sigue sin esta herramienta".into()));
            vec![Question {
                header: "Permiso".into(),
                text: format!("¿Dejo correr {tool}? {}", crate::claude::tool_detail(&input)),
                multi: false,
                options,
            }]
        } else {
            input["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|q| Question {
                    header: s(q, "header"),
                    text: s(q, "question"),
                    multi: q["multiSelect"].as_bool().unwrap_or(false),
                    options: q["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|o| (s(o, "label"), s(o, "description")))
                        .collect(),
                })
                .collect()
        };
        let n = questions.first().map_or(0, |q| q.options.len());
        Self {
            request_id,
            permission,
            input,
            rules,
            questions,
            step: 0,
            sel: 0,
            checked: vec![false; n],
            answers: Map::new(),
        }
    }

    pub fn current(&self) -> &Question {
        &self.questions[self.step]
    }

    pub fn move_sel(&mut self, down: bool) {
        let n = self.current().options.len().max(1);
        self.sel = if down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
    }

    pub fn toggle(&mut self) {
        if let Some(c) = self.checked.get_mut(self.sel) {
            *c = !*c;
        }
    }

    /// Answers the current question: with `typed` if something was typed, otherwise with the selection.
    pub fn answer(&mut self, typed: &str) -> Outcome {
        let q = self.current();
        let answer = if !typed.trim().is_empty() {
            typed.trim().to_string()
        } else if q.multi && self.checked.contains(&true) {
            q.options
                .iter()
                .zip(&self.checked)
                .filter(|(_, c)| **c)
                .map(|(o, _)| o.0.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            q.options.get(self.sel).map(|o| o.0.clone()).unwrap_or_default()
        };

        if self.permission {
            return if answer == ALLOW {
                Outcome::Allow(self.input.clone())
            } else if answer == ALWAYS {
                Outcome::Always(self.input.clone(), self.rules.clone())
            } else {
                Outcome::Deny(format!("el usuario no lo permitió: {answer}"))
            };
        }

        self.answers.insert(q.text.clone(), Value::String(answer));
        self.step += 1;
        if self.step < self.questions.len() {
            self.sel = 0;
            self.checked = vec![false; self.current().options.len()];
            return Outcome::Next;
        }
        let mut input = self.input.clone();
        input["answers"] = Value::Object(std::mem::take(&mut self.answers));
        Outcome::Allow(input)
    }
}

/// The rules the engine proposes, but only for this session: the suggestions come meant to be
/// saved in `.claude/settings.local.json` and here we don't want to touch any file.
/// `Null` if there are none.
fn session_rules(suggestions: &Value) -> Value {
    let rules: Vec<Value> = suggestions
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let mut r = r.clone();
            r["destination"] = Value::String("session".into());
            r
        })
        .collect();
    if rules.is_empty() { Value::Null } else { Value::Array(rules) }
}

/// «Bash(touch /tmp/x), carpeta /tmp»: what stays allowed, for the menu option.
fn summary(rules: &Value) -> String {
    let mut parts = vec![];
    for r in rules.as_array().into_iter().flatten() {
        for x in r["rules"].as_array().into_iter().flatten() {
            let tool = x["toolName"].as_str().unwrap_or("");
            match x["ruleContent"].as_str() {
                Some(c) => parts.push(format!("{tool}({c})")),
                None => parts.push(tool.to_string()),
            }
        }
        for d in r["directories"].as_array().into_iter().flatten() {
            parts.push(format!("carpeta {}", d.as_str().unwrap_or("")));
        }
        if let Some(m) = r["mode"].as_str() {
            parts.push(format!("modo {m}"));
        }
    }
    parts.join(", ")
}
