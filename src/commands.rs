//! Menú de comandos: aparece en cuanto la orden empieza con `/`.

use std::path::Path;

pub struct Command {
    pub name: String,
    pub desc: String,
    /// Lo resuelve Jarvis; si no, la orden se le pasa tal cual al motor.
    pub local: bool,
}

/// Los que `claude -p` no entiende y Jarvis resuelve él mismo.
const LOCAL: &[(&str, &str)] = &[
    ("resume", "retomar una sesión anterior · /resume N"),
    ("model", "cambiar de modelo · /model sonnet"),
    ("copy", "copiar la última respuesta (o arrastra con el mouse en el chat)"),
    ("clear", "conversación nueva; la anterior queda en /resume (^L solo limpia la pantalla)"),
    ("restart", "reiniciar el motor con una sesión nueva (^R)"),
    ("calma", "menos movimiento en el núcleo (alterna)"),
    ("voz", "cuándo contesta hablando: auto (si le hablas), siempre o nunca"),
    ("theme", "elegir el estilo de la pantalla · /theme cabina"),
    ("quit", "salir de Jarvis (^C)"),
];

/// Comandos propios más los skills del usuario. Los que traen los plugins solo se conocen
/// cuando el motor manda su `init`, en el primer turno; esos se suman con `add_skills`.
pub fn load() -> Vec<Command> {
    let mut out: Vec<Command> = LOCAL
        .iter()
        .map(|(n, d)| Command { name: n.to_string(), desc: d.to_string(), local: true })
        .collect();
    // Este sí lo entiende `claude -p`; va aquí para que salga en el menú desde el arranque.
    out.push(Command {
        name: "compact".into(),
        desc: "resumir la conversación para liberar contexto".into(),
        local: false,
    });
    let home = std::env::var("HOME").unwrap_or_default();
    for dir in [format!("{home}/.claude/skills"), ".claude/skills".into()] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            if let Some((name, desc)) = skill_meta(&e.path().join("SKILL.md")) {
                if !out.iter().any(|c| c.name == name) {
                    out.push(Command { name, desc, local: false });
                }
            }
        }
    }
    out
}

/// (id, nombre, para qué)
pub const MODELS: &[(&str, &str, &str)] = &[
    ("claude-opus-5-5", "Opus 5.5", ""),
    ("claude-fable-5-1", "Fable 5.1", ""),
    ("claude-sonnet-5", "Sonnet 5", "equilibrado"),
    ("claude-haiku-4-5-20251001", "Haiku 4.5", "el más rápido y barato"),
];

pub fn add_skills(cmds: &mut Vec<Command>, names: &[String]) {
    for n in names {
        if !cmds.iter().any(|c| &c.name == n) {
            cmds.push(Command { name: n.clone(), desc: "skill".into(), local: false });
        }
    }
}

/// `name` y la primera línea de `description` del frontmatter (admite `description: >`).
fn skill_meta(path: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines().skip_while(|l| l.trim() != "---").skip(1);
    let (mut name, mut desc) = (None, String::new());
    while let Some(l) = lines.next() {
        if l.trim() == "---" {
            break;
        }
        if let Some(v) = l.strip_prefix("name:") {
            name = Some(v.trim().to_string());
        } else if let Some(v) = l.strip_prefix("description:") {
            let v = v.trim();
            desc = if matches!(v, "" | ">" | "|" | ">-" | "|-") {
                lines.next().unwrap_or("").trim().to_string()
            } else {
                v.trim_matches('"').to_string()
            };
        }
    }
    Some((name?, desc))
}

/// Lo que cabe con lo escrito: primero los que empiezan así, luego los que lo contienen.
/// Solo mientras se escribe el nombre; con el primer espacio el menú se cierra.
pub fn matches<'a>(cmds: &'a [Command], input: &str) -> Vec<&'a Command> {
    let Some(q) = input.strip_prefix('/') else { return vec![] };
    if q.contains(char::is_whitespace) {
        return vec![];
    }
    let q = q.to_lowercase();
    let mut out: Vec<&Command> = cmds.iter().filter(|c| c.name.starts_with(&q)).collect();
    out.extend(cmds.iter().filter(|c| !c.name.starts_with(&q) && c.name.contains(&q)));
    out
}
