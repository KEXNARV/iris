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
    /// Las reglas propuestas por el motor, ya para esta sesión; vacío si no propuso ninguna.
    reglas: Value,
    pub questions: Vec<Question>,
    pub step: usize,
    pub sel: usize,
    pub checked: Vec<bool>,
    answers: Map<String, Value>,
}

pub enum Outcome {
    Next,
    Allow(Value),
    /// Permitir y no volver a preguntar por lo mismo en esta sesión: (input, reglas).
    Siempre(Value, Value),
    Deny(String),
}

const ALLOW: &str = "Permitir";
const SIEMPRE: &str = "Siempre";

impl Ask {
    pub fn new(request_id: String, tool: String, input: Value, suggestions: Value) -> Self {
        let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").to_string();
        let permission = tool != "AskUserQuestion";
        let reglas = if permission { para_la_sesion(&suggestions) } else { Value::Null };
        let questions = if permission {
            let mut options = vec![(ALLOW.into(), "una vez".into())];
            if !reglas.is_null() {
                options.push((SIEMPRE.into(), format!("en esta sesión: {}", resumen(&reglas))));
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
            reglas,
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
            } else if answer == SIEMPRE {
                Outcome::Siempre(self.input.clone(), self.reglas.clone())
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

/// Las reglas que propone el motor, pero solo para esta sesión: las sugerencias vienen para
/// guardarse en `.claude/settings.local.json` y aquí no se quiere tocar ningún archivo.
/// `Null` si no hay ninguna.
fn para_la_sesion(sugerencias: &Value) -> Value {
    let reglas: Vec<Value> = sugerencias
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let mut r = r.clone();
            r["destination"] = Value::String("session".into());
            r
        })
        .collect();
    if reglas.is_empty() { Value::Null } else { Value::Array(reglas) }
}

/// «Bash(touch /tmp/x), carpeta /tmp»: lo que queda permitido, para la opción del menú.
fn resumen(reglas: &Value) -> String {
    let mut partes = vec![];
    for r in reglas.as_array().into_iter().flatten() {
        for x in r["rules"].as_array().into_iter().flatten() {
            let tool = x["toolName"].as_str().unwrap_or("");
            match x["ruleContent"].as_str() {
                Some(c) => partes.push(format!("{tool}({c})")),
                None => partes.push(tool.to_string()),
            }
        }
        for d in r["directories"].as_array().into_iter().flatten() {
            partes.push(format!("carpeta {}", d.as_str().unwrap_or("")));
        }
        if let Some(m) = r["mode"].as_str() {
            partes.push(format!("modo {m}"));
        }
    }
    partes.join(", ")
}
