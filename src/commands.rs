//! Command menu: shows up as soon as the input starts with `/`.

use std::path::Path;

pub struct Command {
    pub name: String,
    pub desc: String,
    /// Handled by Iris; otherwise the command is passed as-is to the engine.
    pub local: bool,
}

/// The ones `claude -p` doesn't understand and Iris handles itself.
const LOCAL: &[(&str, &str)] = &[
    ("resume", "retomar una sesión anterior · /resume N"),
    ("model", "cambiar de modelo · /model sonnet"),
    ("effort", "cuánto piensa: low, medium, high, xhigh o max · /effort high"),
    ("copy", "copiar la última respuesta (o arrastra con el mouse en el chat)"),
    ("clear", "conversación nueva; la anterior queda en /resume (^L solo limpia la pantalla)"),
    ("welcome", "repetir la bienvenida: permisos, estilo, voz, modelo y el recorrido"),
    ("login", "iniciar sesión en tu cuenta de Claude (abre el navegador)"),
    ("restart", "reiniciar el motor con una sesión nueva (^R)"),
    ("calm", "menos movimiento en el núcleo (alterna)"),
    ("voice", "cuándo contesta hablando: auto (si le hablas), always o never"),
    ("theme", "elegir el estilo de la pantalla · /theme cabina"),
    ("buddy", "quién vive en el núcleo · /buddy kirby · /buddy edit · /buddy publish"),
    ("store", "la tienda: buddies y piezas · /store kirby · /store update · /store remove kirby"),
    ("core", "color de la cara de Baymax en el núcleo · /core blanco"),
    ("agents", "qué hace cada subagente, paso a paso (también ^G y Tab)"),
    ("quit", "salir de Iris (^C)"),
];

/// Built-in commands plus the user's skills. Those from plugins are only known when the engine
/// sends its `init`, on the first turn; they are added with `add_skills`.
pub fn load() -> Vec<Command> {
    let mut out: Vec<Command> = LOCAL
        .iter()
        .map(|(n, d)| Command { name: n.to_string(), desc: d.to_string(), local: true })
        .collect();
    // `claude -p` does understand this one; it goes here so it shows in the menu from startup.
    out.push(Command {
        name: "compact".into(),
        desc: "resumir la conversación para liberar contexto".into(),
        local: false,
    });
    let home = crate::paths::home().unwrap_or_default();
    for dir in [home.join(".claude").join("skills"), ".claude/skills".into()] {
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

/// (level, what for); `auto` keeps the model's default.
pub const EFFORTS: &[(&str, &str)] = &[
    ("auto", "el que trae el modelo"),
    ("low", "rápido, piensa poco"),
    ("medium", ""),
    ("high", ""),
    ("xhigh", ""),
    ("max", "piensa todo lo que haga falta"),
];

/// (id, name, what for)
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

/// `name` and the first line of `description` from the frontmatter (supports `description: >`).
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

/// What matches the input: first those that start with it, then those that contain it.
/// Only while typing the name; the first space closes the menu.
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
