//! Preguntas que el motor le hace al usuario (`AskUserQuestion`, o un permiso) y que esperan
//! respuesta antes de seguir.

use serde_json::{Map, Value};

pub struct Question {
    pub header: String,
    pub text: String,
    pub multi: bool,
    /// (etiqueta, descripción)
    pub options: Vec<(String, String)>,
}

pub struct Ask {
    pub request_id: String,
    /// Un permiso para otra herramienta, no una `AskUserQuestion`.
    pub permission: bool,
    input: Value,
    pub questions: Vec<Question>,
    pub step: usize,
    pub sel: usize,
    pub checked: Vec<bool>,
    answers: Map<String, Value>,
}

pub enum Outcome {
    Next,
    Allow(Value),
    Deny(String),
}

const ALLOW: &str = "Permitir";

impl Ask {
    pub fn new(request_id: String, tool: String, input: Value) -> Self {
        let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").to_string();
        let permission = tool != "AskUserQuestion";
        let questions = if permission {
            vec![Question {
                header: "Permiso".into(),
                text: format!("¿Dejo correr {tool}? {}", crate::claude::tool_detail(&input)),
                multi: false,
                options: vec![
                    (ALLOW.into(), "una vez".into()),
                    ("Rechazar".into(), "Claude sigue sin esta herramienta".into()),
                ],
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

    /// Responde la pregunta en curso: con `typed` si escribió algo, si no con lo elegido.
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
