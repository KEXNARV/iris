//! KIT: the commands for making and publishing pieces and buddies, outside the chat.
//!
//! ```text
//! iris login                     log in to the store (GitHub or an Iris account)
//! iris logout
//! iris piece new <name> [--shape | --wasm | --buddy]
//! iris piece try [folder]        the core full screen with it inside; reloads on every change
//! iris piece pack [folder]       writes <name>-<version>.iris
//! iris piece publish [folder]    packs and uploads it (it goes into review)
//! iris piece status              what you uploaded and how it's going
//! ```
//!
//! Pieces made in Iris Studio don't need any of this: they're published from there.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crossterm::event::{self, Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers};

use crate::core::recipe::Recipe;
use crate::core::{Core, Event, Signals, State};

/// Runs a kit command if the arguments are one; `None` if they're for the chat.
pub fn run(args: &[String]) -> Option<i32> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let r = match a.as_slice() {
        ["login", rest @ ..] => login(rest.contains(&"--new")),
        ["logout"] => {
            crate::store::forget_token();
            println!("Sesión cerrada.");
            Ok(())
        }
        ["piece", "new", name, rest @ ..] => new(name, rest),
        ["piece", "try", rest @ ..] => try_it(&folder(rest)),
        ["piece", "pack", rest @ ..] => pack(&folder(rest)).map(|(path, _)| println!("Listo: {}", path.display())),
        ["piece", "publish", rest @ ..] => publish(&folder(rest)),
        ["piece", "status"] => status(),
        ["piece", ..] => Err("uso: iris piece new <nombre> | try | pack | publish | status".into()),
        _ => return None,
    };
    Some(match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    })
}

fn folder(rest: &[&str]) -> PathBuf {
    rest.iter().find(|a| !a.starts_with("--")).map_or_else(|| PathBuf::from("."), PathBuf::from)
}

// ─── Account ─────────────────────────────────────────────────────────────────────────────────

fn ask(q: &str) -> String {
    print!("{q}");
    let _ = std::io::stdout().flush();
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
    s.trim().to_string()
}

/// Asks for a password without showing it.
fn ask_hidden(q: &str) -> String {
    print!("{q}");
    let _ = std::io::stdout().flush();
    let _ = crossterm::terminal::enable_raw_mode();
    let mut s = String::new();
    loop {
        match event::read() {
            Ok(TermEvent::Key(k)) if k.kind != KeyEventKind::Release => match k.code {
                KeyCode::Enter => break,
                KeyCode::Backspace => {
                    s.pop();
                }
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    s.clear();
                    break;
                }
                KeyCode::Char(c) => s.push(c),
                _ => {}
            },
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = crossterm::terminal::disable_raw_mode();
    println!();
    s
}

fn login(new: bool) -> Result<(), String> {
    let github = crate::net::json("GET", "/api/store/config", None, None).map(|(_, v)| v.get("github").and_then(|g| g.as_bool()).unwrap_or(false)).unwrap_or(false);
    let choice = if new {
        "3".to_string()
    } else if github {
        ask("Entrar a la tienda de Iris con:\n  1) GitHub\n  2) tu cuenta de Iris\n  3) crear una cuenta de Iris\n> ")
    } else {
        ask("Entrar a la tienda de Iris:\n  1) con tu cuenta de Iris\n  2) crear una cuenta\n> ").replace('2', "3").replace('1', "2")
    };
    let answer = match choice.as_str() {
        "1" => github_flow()?,
        "3" => {
            let username = ask("Nombre de usuario (minúsculas, números y guiones): ");
            let email = ask("Correo: ");
            let password = ask_hidden("Contraseña (8 o más): ");
            crate::net::json("POST", "/api/auth/register", None, Some(&serde_json::json!({"username": username, "email": email, "password": password})))?.1
        }
        _ => {
            let login = ask("Usuario o correo: ");
            let password = ask_hidden("Contraseña: ");
            crate::net::json("POST", "/api/auth/login", None, Some(&serde_json::json!({"login": login, "password": password})))?.1
        }
    };
    let token = answer.get("token").and_then(|t| t.as_str()).ok_or("la tienda no devolvió una sesión")?;
    crate::store::save_token(token).map_err(|e| e.to_string())?;
    println!("Listo, entraste como {}.", answer.get("username").and_then(|u| u.as_str()).unwrap_or("?"));
    Ok(())
}

fn github_flow() -> Result<serde_json::Value, String> {
    let (_, start) = crate::net::json("POST", "/api/auth/github/start", None, Some(&serde_json::json!({})))?;
    let s = |k: &str| start.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let interval = start.get("interval").and_then(|v| v.as_u64()).unwrap_or(5).max(2);
    println!("Abre {} y escribe este código: {}", s("verification_uri"), s("user_code"));
    let _ = crate::link::open(&s("verification_uri"));
    loop {
        std::thread::sleep(Duration::from_secs(interval));
        let (code, v) = crate::net::json("POST", "/api/auth/github/poll", None, Some(&serde_json::json!({"device_code": s("device_code")})))?;
        if code == 200 {
            return Ok(v);
        }
    }
}

fn need_token() -> Result<String, String> {
    crate::store::token().ok_or_else(|| "primero entra a la tienda: iris login".into())
}

// ─── Making ──────────────────────────────────────────────────────────────────────────────────

/// What's in a project folder, from its `iris.toml`.
struct Project {
    dir: PathBuf,
    name: String,
    version: String,
    kind: String,
    format: String,
    /// While trying it: which part of the original it takes the place of (`body`), if any.
    replaces: Option<String>,
}

fn project(dir: &Path) -> Result<Project, String> {
    let text = std::fs::read_to_string(dir.join("iris.toml")).map_err(|_| format!("en {} no hay ningún iris.toml (¿iris piece new?)", dir.display()))?;
    let t: toml::Table = text.parse().map_err(|e: toml::de::Error| format!("iris.toml: {e}"))?;
    let p = t.get("package").and_then(|p| p.as_table()).ok_or("iris.toml: falta [package]")?;
    let s = |k: &str| p.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let replaces = t.get("preview").and_then(|p| p.get("replaces")).and_then(|r| r.as_str()).map(str::to_string);
    Ok(Project { dir: dir.to_path_buf(), name: s("name"), version: s("version"), kind: s("kind"), format: s("format"), replaces })
}

fn valid_name(n: &str) -> bool {
    (2..=40).contains(&n.len()) && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !n.starts_with('-')
}

fn new(name: &str, rest: &[&str]) -> Result<(), String> {
    if !valid_name(name) {
        return Err("el nombre va en minúsculas, números y guiones, de 2 a 40 letras".into());
    }
    let dir = PathBuf::from(name);
    if dir.exists() {
        return Err(format!("ya existe una carpeta «{name}»"));
    }
    let write = |rel: &str, body: &str| -> Result<(), String> {
        let p = dir.join(rel);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(p, body).map_err(|e| e.to_string())
    };
    let manifest = |kind: &str, format: Option<&str>, extra: &str| {
        let format = format.map(|f| format!("format = \"{f}\"\n")).unwrap_or_default();
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\n{format}version = \"1.0.0\"\ndescription = \"\"\nlicense = \"MIT\"\n{extra}")
    };
    if rest.contains(&"--buddy") {
        write("iris.toml", &manifest("buddy", None, ""))?;
        write("recipe.toml", &include_str!("core/recipe/original.toml").replacen("name = \"original\"", &format!("name = \"{name}\""), 1))?;
        println!("Listo: {name}/ es un buddy hecho receta (una copia del original).");
        println!("Edita {name}/recipe.toml y míralo con: iris piece try {name}");
    } else if rest.contains(&"--wasm") {
        write("iris.toml", &manifest("piece", Some("wasm"), ""))?;
        write(
            "Cargo.toml",
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nedition = \"2024\"\npublish = false\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\niris-piece = {{ path = \"iris-piece\" }}\n\n[profile.release]\nopt-level = \"s\"\nlto = true\npanic = \"abort\"\nstrip = true\n\n[workspace]\n"
            ),
        )?;
        write("src/lib.rs", WASM_TEMPLATE)?;
        write("iris-piece/Cargo.toml", "[package]\nname = \"iris-piece\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n")?;
        write("iris-piece/src/lib.rs", include_str!("../crates/iris-piece/src/lib.rs"))?;
        write("iris-piece/src/abi.rs", include_str!("../crates/iris-piece/src/abi.rs"))?;
        write("iris-piece/src/part.rs", include_str!("../crates/iris-piece/src/part.rs"))?;
        write(".gitignore", "target/\n")?;
        println!("Listo: {name}/ es una pieza en Rust (una antena que chispea con los errores).");
        println!("Necesitas el destino de WebAssembly: rustup target add wasm32-unknown-unknown");
        println!("Mírala mientras la escribes: iris piece try {name}");
    } else {
        write("iris.toml", &manifest("piece", Some("shape"), ""))?;
        write("shape.toml", SHAPE_TEMPLATE)?;
        println!("Listo: {name}/ es una pieza de formas (una antena que chispea con los errores).");
        println!("Edita {name}/shape.toml y mírala con: iris piece try {name}");
        println!("Si va en lugar del cuerpo, agrega a iris.toml:  [preview]  replaces = \"body\"");
    }
    Ok(())
}

const SHAPE_TEMPLATE: &str = r#"# A shape piece: layers of shapes whose properties are formulas over the buddy's channels.
# Variables: t dt st state calm voice music context du · s.<state> · ev.<event> (seconds since it)
# Channels: body.x body.y body.radius breath gaze.x … (see docs/channels.md)

[[settings]]
name = "height"
default = 1.5
min = 1.1
max = 3.0

# The stem.
[[layer]]
shape = "curve"
x = "body.x"
y = "body.y"
points = [[0, "body.radius"], [0, "body.radius * height"]]
smooth = false
tone = 2

# The tip: it sparks on errors and when Iris speaks.
[[layer]]
shape = "circle"
x = "body.x"
y = "body.y + body.radius * height + 0.03"
radius = "0.03 + pulse(ev.error, 0.6) * 0.05 + voice * 0.02"
fill = "solid"
ink = "if(pulse(ev.error, 0.6) > 0, 2, 4)"
"#;

const WASM_TEMPLATE: &str = r#"//! An antenna that sparks on errors: a recipe part in WebAssembly (see iris-piece/src/part.rs).

use iris_piece::export_part;
use iris_piece::part::*;

struct Antenna {
    spark: f64,
}

impl Part for Antenna {
    const INPUTS: &'static [&'static str] = &["body.x", "body.y", "body.radius"];
    const SETTINGS: &'static [Setting] = &[Setting::new("height", 1.5, 1.1, 3.0)];

    fn new(_io: &Io, _rand: &mut Rand) -> Self {
        Antenna { spark: 0.0 }
    }

    fn step(&mut self, io: &mut Io) {
        self.spark = (self.spark - io.dt() * 2.0).max(0.0);
    }

    fn event(&mut self, h: Happening, _io: &mut Io) {
        if h == Happening::Event(Event::Error) {
            self.spark = 1.0;
        }
    }

    fn paint(&self, g: &mut Grid, io: &Io) {
        let (x, y, r, h) = (io.input(0), io.input(1), io.input(2), io.setting(0));
        let steps = 12;
        for k in 0..=steps {
            g.plot(x, y + r + (r * h - r) * k as f64 / steps as f64, 2, INK_ACCENT);
        }
        let ink = if self.spark > 0.3 { INK_RED } else { INK_ACCENT };
        for k in 0..16 {
            let a = k as f64 / 16.0 * std::f64::consts::TAU;
            let rr = 0.03 + self.spark * 0.05;
            g.plot(x + rr * a.cos(), y + r * h + 0.03 + rr * a.sin(), 3, ink);
        }
    }
}

export_part!(Antenna);
"#;

/// Compiles a WASM project and returns its module.
fn build_wasm(p: &Project) -> Result<PathBuf, String> {
    let out = std::process::Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(&p.dir)
        .output()
        .map_err(|e| format!("no pude correr cargo: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let hint = if err.contains("wasm32-unknown-unknown") && err.contains("target") { "\n(¿rustup target add wasm32-unknown-unknown?)" } else { "" };
        return Err(format!("no compila:\n{}{hint}", err.lines().filter(|l| !l.trim_start().starts_with("Compiling")).collect::<Vec<_>>().join("\n")));
    }
    let crate_name = std::fs::read_to_string(p.dir.join("Cargo.toml"))
        .ok()
        .and_then(|t| t.parse::<toml::Table>().ok())
        .and_then(|t| Some(t.get("package")?.get("name")?.as_str()?.replace('-', "_")))
        .ok_or("Cargo.toml sin [package] name")?;
    Ok(p.dir.join("target/wasm32-unknown-unknown/release").join(format!("{crate_name}.wasm")))
}

/// The recipe to try a project in: a buddy's own, or the original with the piece in it.
fn trial_recipe(p: &Project, wasm: Option<&Path>) -> Result<Recipe, String> {
    if p.kind == "buddy" {
        return Recipe::load_dir(&p.dir);
    }
    let uses = match wasm {
        Some(w) => format!("file:{}", std::fs::canonicalize(w).map_err(|e| format!("{}: {e}", w.display()))?.display()),
        None => "shape:shape.toml".to_string(),
    };
    let mut src = include_str!("core/recipe/original.toml").to_string();
    match p.replaces.as_deref() {
        Some("body") => src = src.replacen("use = \"builtin:blob\"", &format!("use = \"{uses}\""), 1),
        Some(other) => return Err(format!("[preview] replaces: por ahora solo «body», no «{other}»")),
        None => {
            src = src.replacen("[[piece]]\nuse = \"builtin:tasks\"", &format!("[[piece]]\nuse = \"{uses}\"\nid = \"preview\"\n\n[[piece]]\nuse = \"builtin:tasks\""), 1);
            src = src.replacen("\"body\", \"kids\"", "\"body\", \"preview\", \"kids\"", 1);
        }
    }
    let mut r = Recipe::parse(&src)?;
    r.files = Some(std::sync::Arc::new(crate::core::recipe::files::Disk { dir: p.dir.clone() }));
    Ok(r)
}

/// When the project's files last changed (not counting what cargo builds).
fn stamp(dir: &Path) -> SystemTime {
    fn walk(d: &Path, best: &mut SystemTime) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
                continue;
            }
            if p.is_dir() {
                walk(&p, best);
            } else if let Some(m) = e.metadata().ok().and_then(|m| m.modified().ok()) {
                *best = (*best).max(m);
            }
        }
    }
    let mut best = SystemTime::UNIX_EPOCH;
    walk(dir, &mut best);
    best
}

const STATES: [(State, &str); 21] = [
    (State::Idle, "en espera"),
    (State::Listening, "escuchando"),
    (State::Transcribing, "transcribiendo"),
    (State::Thinking, "pensando"),
    (State::Planning, "planeando"),
    (State::Reading, "leyendo"),
    (State::Searching, "buscando"),
    (State::Editing, "editando"),
    (State::Running, "corriendo"),
    (State::Testing, "probando"),
    (State::Git, "git"),
    (State::Web, "web"),
    (State::Delegating, "delegando"),
    (State::Speaking, "hablando"),
    (State::Asking, "preguntando"),
    (State::Compacting, "compactando"),
    (State::Sleeping, "durmiendo"),
    (State::Typing, "escribiendo"),
    (State::NoVoice, "sin voz"),
    (State::Booting, "arrancando"),
    (State::Offline, "desconectado"),
];

const EVENTS: [(Event, &str); 8] = [
    (Event::Key, "tecla"),
    (Event::Error, "error"),
    (Event::Done, "listo"),
    (Event::Cancel, "cancela"),
    (Event::Nope, "descarta"),
    (Event::Merge, "vuelve"),
    (Event::Copy, "copia"),
    (Event::Pass, "pasa"),
];

fn try_it(dir: &Path) -> Result<(), String> {
    let p = project(dir)?;
    let load = |p: &Project| -> Result<Core, String> {
        let wasm = if p.format == "wasm" { Some(build_wasm(p)?) } else { None };
        Core::from_recipe(&trial_recipe(p, wasm.as_deref())?)
    };
    let mut core = load(&p);
    let mut term = ratatui::init();
    let (mut st, mut voice, mut kids, mut seen) = (0usize, false, 0u64, stamp(&p.dir));
    let (mut last, start) = (Instant::now(), Instant::now());
    let mut problem: Option<String> = core.as_ref().err().cloned();
    let mut next_check = Instant::now();
    let out = loop {
        let dt = last.elapsed().as_secs_f64().min(0.1);
        last = Instant::now();
        if let Ok(c) = core.as_mut() {
            let t = start.elapsed().as_secs_f32();
            let sig = Signals { level: if voice { 0.08 + 0.06 * (t * 7.3).sin() * (t * 2.1).sin() } else { 0.0 }, ..Default::default() };
            c.step(dt, STATES[st].0, &sig);
            for (part, why) in c.take_broken() {
                problem = Some(format!("«{part}» se rompió: {why}"));
            }
        }
        if Instant::now() >= next_check {
            next_check = Instant::now() + Duration::from_millis(400);
            let now = stamp(&p.dir);
            if now != seen {
                seen = now;
                match project(&p.dir).and_then(|p| load(&p)) {
                    Ok(c) => {
                        core = Ok(c);
                        problem = None;
                    }
                    Err(e) => problem = Some(e),
                }
            }
        }
        let help = format!(
            " {} · ←→ estado ({}) · 1-8 evento · k hijo · v voz · q salir ",
            p.name,
            STATES[st].1
        );
        let drawn = term.draw(|f| {
            let a = f.area();
            let (body, foot) = (ratatui::layout::Rect { height: a.height.saturating_sub(2), ..a }, ratatui::layout::Rect { y: a.y + a.height.saturating_sub(2), height: 2.min(a.height), ..a });
            if let Ok(c) = &core {
                c.draw(body, f.buffer_mut());
            }
            let text = match &problem {
                Some(e) => format!("{help}\n {}", e.lines().next().unwrap_or("")),
                None => help.clone(),
            };
            f.render_widget(ratatui::widgets::Paragraph::new(text), foot);
        });
        if drawn.is_err() {
            break Err("no pude dibujar".to_string());
        }
        if event::poll(Duration::from_millis(33)).unwrap_or(false) {
            if let Ok(TermEvent::Key(k)) = event::read() {
                if k.kind == KeyEventKind::Release {
                    continue;
                }
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break Ok(()),
                    KeyCode::Right => st = (st + 1) % STATES.len(),
                    KeyCode::Left => st = (st + STATES.len() - 1) % STATES.len(),
                    KeyCode::Char('v') => voice = !voice,
                    KeyCode::Char(d @ '1'..='8') => {
                        if let Ok(c) = core.as_mut() {
                            c.fire(EVENTS[(d as u8 - b'1') as usize].0);
                        }
                    }
                    KeyCode::Char('k') => {
                        if let Ok(c) = core.as_mut() {
                            kids += 1;
                            c.kid_born(kids);
                            if kids > 3 {
                                c.kid_end(kids - 3, true);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    };
    ratatui::restore();
    out
}

/// Builds the `.iris` of a project: (where it was written, its bytes).
fn pack(dir: &Path) -> Result<(PathBuf, Vec<u8>), String> {
    let p = project(dir)?;
    if !valid_name(&p.name) {
        return Err("iris.toml: «name» va en minúsculas, números y guiones, de 2 a 40 letras".into());
    }
    if p.version.split('.').filter(|x| x.parse::<u64>().is_ok()).count() != 3 {
        return Err("iris.toml: «version» va como 1.0.0".into());
    }
    let files: Vec<(&str, Vec<u8>)> = match (p.kind.as_str(), p.format.as_str()) {
        ("buddy", _) => {
            let src = std::fs::read_to_string(p.dir.join("recipe.toml")).map_err(|_| "falta recipe.toml")?;
            let r = Recipe::parse(&src)?;
            if let Some(bad) = r.pieces.iter().find(|x| !x.uses.starts_with("builtin:") && !x.uses.contains('@')) {
                return Err(format!(
                    "recipe.toml usa «{}»: en la tienda un buddy solo usa piezas de fábrica o publicadas (nombre@1); publica esa pieza primero",
                    bad.uses
                ));
            }
            vec![("recipe.toml", src.into_bytes())]
        }
        ("piece", "shape") => {
            let src = std::fs::read_to_string(p.dir.join("shape.toml")).map_err(|_| "falta shape.toml")?;
            iris_shape::Shape::parse(&src).map_err(|e| format!("shape.toml: {e}"))?;
            vec![("shape.toml", src.into_bytes())]
        }
        ("piece", "wasm") => vec![("piece.wasm", std::fs::read(build_wasm(&p)?).map_err(|e| e.to_string())?)],
        (k, f) => return Err(format!("iris.toml: no sé empaquetar kind «{k}» con format «{f}»")),
    };
    let bytes = crate::store::pack(&p.dir, &files)?;
    let out = p.dir.join(format!("{}-{}.iris", p.name, p.version));
    std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
    Ok((out, bytes))
}

fn publish(dir: &Path) -> Result<(), String> {
    let token = need_token()?;
    let (path, bytes) = pack(dir)?;
    let (code, body) = crate::net::call("POST", "/api/pieces", Some(&token), Some(("application/zip", &bytes)))?;
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
    if code >= 400 {
        return Err(format!("no se publicó: {}", crate::net::error_of(code, &v)));
    }
    println!("Subido {} ({}). Queda en revisión: cuando se apruebe, cualquiera lo instala con /store.", path.display(), v.get("version").and_then(|x| x.as_str()).unwrap_or(""));
    println!("Míralo con: iris piece status");
    Ok(())
}

fn status() -> Result<(), String> {
    let token = need_token()?;
    let (_, v) = crate::net::json("GET", "/api/me/pieces", Some(&token), None)?;
    let list = v.as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        println!("Todavía no subiste nada.");
    }
    for e in list {
        let s = |k: &str| e.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let what = match s("status").as_str() {
            "in_review" => "en revisión".to_string(),
            "approved" => "aprobado (sale con el próximo índice)".to_string(),
            "signed" => "publicado".to_string(),
            "rejected" => format!("rechazado: {}", s("reason")),
            "revoked" => format!("retirado: {}", s("reason")),
            o => o.to_string(),
        };
        println!("{:<24} {:<8} {}", s("name"), s("version"), what);
    }
    Ok(())
}
