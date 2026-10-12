//! `/store` inside Iris: the catalog, with the core wearing what's selected; enter installs it.
//! Also `/store remove <name>`, `/store update`, and the drafts that arrive from Iris Studio.
//!
//! The network runs on threads; what they find comes back as `AppEvent::Store`.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppEvent, Modal, Role};
use crate::buddy::{self, Buddy};
use crate::store::{self, Entry, Index};

pub enum StoreEvent {
    /// The index arrived (or didn't), with what it turned off; and the search the user typed.
    Index(Result<(Index, Vec<String>), String>, String),
    /// A package was downloaded: to look at it, or installed for good.
    Ready { name: String, preview: bool, result: Result<(), String> },
    /// A draft from Iris Studio was updated on disk.
    Draft(String),
    Published(Result<String, String>),
    /// New versions of installed packages, from the background check.
    Updates(Index, Vec<crate::store::Entry>),
    /// An installed package was updated (or not).
    Updated { name: String, version: String, result: Result<(), String> },
    /// A like went through: the package and how many it has now.
    Liked(Result<(String, i64), String>),
    /// Logging in (or creating the account) from the form ended: your username, or why not.
    LoggedIn(Result<String, String>),
}

/// The form to log in to the store (or create the account) without leaving Iris. `/store`
/// opens it when there's no session: the store only gives out its catalog to someone logged in.
pub struct StoreLogin {
    pub creating: bool,
    pub field: usize,
    pub username: String,
    /// Username or email to log in; email to create the account.
    pub login: String,
    pub password: String,
    pub error: Option<String>,
    pub busy: bool,
    /// What had been asked of the store: it goes on there once you're in.
    pub then: String,
}

impl StoreLogin {
    pub fn new(then: String) -> Self {
        Self { creating: false, field: 0, username: String::new(), login: String::new(), password: String::new(), error: None, busy: false, then }
    }

    /// (label, value, secret) of each field, in order.
    pub fn fields(&self) -> Vec<(&'static str, &str, bool)> {
        if self.creating {
            vec![("Usuario", &self.username, false), ("Correo", &self.login, false), ("Contraseña", &self.password, true)]
        } else {
            vec![("Usuario o correo", &self.login, false), ("Contraseña", &self.password, true)]
        }
    }

    fn value(&mut self) -> &mut String {
        match (self.creating, self.field) {
            (true, 0) => &mut self.username,
            (true, 1) | (false, 0) => &mut self.login,
            _ => &mut self.password,
        }
    }
}

/// Opens the form; when you're in, the store opens with `then`.
pub fn ask_login(app: &mut App, then: String) {
    app.modal = Some(Modal::StoreLogin(StoreLogin::new(then)));
}

pub fn login_key(app: &mut App, k: KeyEvent) {
    let Some(Modal::StoreLogin(l)) = app.modal.as_mut() else { return };
    let n = l.fields().len();
    match k.code {
        KeyCode::Esc => app.modal = None,
        _ if l.busy => {}
        KeyCode::Char('n') if k.modifiers.contains(KeyModifiers::CONTROL) => {
            l.creating = !l.creating;
            l.field = 0;
            l.error = None;
        }
        KeyCode::Tab | KeyCode::Down => l.field = (l.field + 1) % n,
        KeyCode::BackTab | KeyCode::Up => l.field = (l.field + n - 1) % n,
        KeyCode::Backspace => {
            l.value().pop();
        }
        KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => l.value().push(c),
        // Enter on a field that isn't the last moves on, like a form.
        KeyCode::Enter if l.field + 1 < n => l.field += 1,
        KeyCode::Enter => {
            if l.fields().iter().any(|(_, v, _)| v.trim().is_empty()) {
                l.error = Some("falta llenar algo".into());
                return;
            }
            l.busy = true;
            l.error = None;
            let body = if l.creating {
                serde_json::json!({ "username": l.username.trim(), "email": l.login.trim(), "password": l.password })
            } else {
                serde_json::json!({ "login": l.login.trim(), "password": l.password })
            };
            let path = if l.creating { "/api/auth/register" } else { "/api/auth/login" };
            let tx = app.events.clone();
            std::thread::spawn(move || {
                let result = crate::net::json("POST", path, None, Some(&body)).and_then(|(code, v)| {
                    if code != 200 {
                        return Err(crate::net::error_of(code, &v));
                    }
                    let token = v.get("token").and_then(|t| t.as_str()).ok_or("la tienda no devolvió una sesión")?;
                    store::save_token(token).map_err(|e| e.to_string())?;
                    Ok(v.get("username").and_then(|u| u.as_str()).unwrap_or("?").to_string())
                });
                let _ = tx.send(AppEvent::Store(StoreEvent::LoggedIn(result)));
            });
        }
        _ => {}
    }
}

pub struct StoreModal {
    pub index: Index,
    /// 0 buddies, 1 pieces.
    pub tab: usize,
    pub sel: usize,
    pub query: String,
    /// Who was in the core before opening the store, to put back on Esc.
    pub before: Buddy,
    /// What's being downloaded right now.
    pub busy: Option<String>,
}

impl StoreModal {
    pub fn list(&self) -> Vec<Entry> {
        let kind = if self.tab == 0 { "buddy" } else { "piece" };
        let q = self.query.to_lowercase();
        self.index
            .latest()
            .into_iter()
            .filter(|e| e.kind == kind && (q.is_empty() || e.name.contains(&q) || e.description.to_lowercase().contains(&q)))
            .cloned()
            .collect()
    }
}

/// `/store [search]`, `/store remove <name>`, `/store update`.
pub fn command(app: &mut App, arg: &str) {
    let (verb, rest) = arg.split_once(' ').unwrap_or((arg, ""));
    match verb {
        "remove" | "quitar" => {
            let name = rest.trim();
            let msg = match store::remove(name) {
                Ok(()) => {
                    if app.buddy == Buddy::Recipe(name.to_string()) {
                        crate::actions::set_buddy(app, Buddy::Original);
                    }
                    format!("«{name}» desinstalado")
                }
                Err(e) => e,
            };
            app.push(Role::System, msg);
        }
        // Without a session, first the form; then the store goes on with what you asked.
        _ if store::token().is_none() => ask_login(app, if verb == "update" || verb == "actualizar" { "\u{0}update".into() } else { arg.to_string() }),
        "update" | "actualizar" => fetch_index(app, "\u{0}update".into()),
        q => fetch_index(app, q.to_string()),
    }
}

fn fetch_index(app: &mut App, query: String) {
    app.flash = Some(("abriendo la tienda…".into(), Instant::now()));
    let tx = app.events.clone();
    std::thread::spawn(move || {
        let _ = tx.send(AppEvent::Store(StoreEvent::Index(store::refresh(), query)));
    });
}

/// Installs new versions of what's already installed: as is, without putting anything on the
/// core (an update of a buddy's piece is not a piece to try on).
pub fn install_updates(app: &mut App, index: Index, list: Vec<Entry>) {
    for e in list {
        app.push(Role::System, format!("actualizando «{}» a la {}…", e.name, e.version));
        let (tx, index) = (app.events.clone(), index.clone());
        std::thread::spawn(move || {
            let result = store::install(&index, &e, false).map(|_| ());
            let _ = tx.send(AppEvent::Store(StoreEvent::Updated { name: e.name.clone(), version: e.version.clone(), result }));
        });
    }
}

/// Checks the store in the background (a little after startup, then every hour) for new
/// versions of what's installed, without the ones already turned down.
pub fn check_updates(tx: Sender<AppEvent>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(20));
        let mut offered: Vec<String> = vec![];
        loop {
            if let Ok((index, _)) = store::refresh() {
                let no = store::dismissed();
                let list: Vec<Entry> = store::updates(&index).into_iter().filter(|e| !no.contains(&format!("{} {}", e.name, e.version))).collect();
                let keys: Vec<String> = list.iter().map(|e| format!("{} {}", e.name, e.version)).collect();
                if !list.is_empty() && keys != offered {
                    offered = keys;
                    if tx.send(AppEvent::Store(StoreEvent::Updates(index, list))).is_err() {
                        return;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    });
}

fn download(tx: Sender<AppEvent>, index: Index, e: Entry, preview: bool) {
    std::thread::spawn(move || {
        let result = store::install(&index, &e, preview).map(|_| ());
        let _ = tx.send(AppEvent::Store(StoreEvent::Ready { name: e.name.clone(), preview, result }));
    });
}

/// The hidden recipe used to look at a piece on the current buddy.
fn preview_dir() -> Option<PathBuf> {
    Some(crate::paths::config()?.join("buddies").join(".preview"))
}

/// The current buddy's recipe, as text, with the folder its files come from.
fn current_recipe(b: &Buddy) -> (String, Option<PathBuf>) {
    match b {
        Buddy::Baymax => (include_str!("core/recipe/baymax.toml").to_string(), None),
        Buddy::Recipe(n) => match buddy::recipe_dir(n).and_then(|d| Some((std::fs::read_to_string(d.join("recipe.toml")).ok()?, d))) {
            Some((text, dir)) => (text, Some(dir)),
            None => (include_str!("core/recipe/original.toml").to_string(), None),
        },
        Buddy::Original => (include_str!("core/recipe/original.toml").to_string(), None),
    }
}

/// A copy of the current buddy with a store piece added, written to `dir`.
fn with_piece(base: &Buddy, piece: &Entry, dir: &std::path::Path, name: &str) -> Result<(), String> {
    let (text, from) = current_recipe(base);
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if let Some(from) = from {
        for f in std::fs::read_dir(from).into_iter().flatten().flatten().filter(|f| f.path().is_file()) {
            let _ = std::fs::copy(f.path(), dir.join(f.file_name()));
        }
    }
    let uses = format!("{}@{}", piece.name, piece.major());
    let mut text = text.replacen(
        "[[piece]]\nuse = \"builtin:tasks\"",
        &format!("[[piece]]\nuse = \"{uses}\"\nid = \"{}\"\n\n[[piece]]\nuse = \"builtin:tasks\"", piece.name),
        1,
    );
    text = text.replacen("\"body\", \"kids\"", &format!("\"body\", \"{}\", \"kids\"", piece.name), 1);
    if let Some(i) = text.find("name = \"") {
        let end = text[i + 8..].find('"').map_or(i + 8, |e| i + 8 + e);
        text.replace_range(i..=end, &format!("name = \"{name}\""));
    }
    std::fs::write(dir.join("recipe.toml"), text).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(dir.join(".draft"));
    let _ = std::fs::remove_file(dir.join("iris.toml"));
    Ok(())
}

/// Puts what's selected on the core (downloading it first if needed).
fn preview(app: &mut App) {
    let Some(Modal::Store(m)) = &mut app.modal else { return };
    let Some(e) = m.list().get(m.sel).cloned() else { return };
    m.busy = Some(e.name.clone());
    download(app.events.clone(), m.index.clone(), e, true);
}

pub fn on_event(app: &mut App, ev: StoreEvent) {
    match ev {
        // The session expired (or it was closed elsewhere): log in again and go on.
        StoreEvent::Index(Err(e), query) if e == store::NEED_LOGIN => ask_login(app, query),
        StoreEvent::Index(Err(e), _) => app.push(Role::Error, format!("la tienda: {e}")),
        StoreEvent::LoggedIn(Ok(user)) => {
            let then = match app.modal.take() {
                Some(Modal::StoreLogin(l)) => l.then,
                other => {
                    app.modal = other;
                    String::new()
                }
            };
            app.push(Role::System, format!("entraste a la tienda como {user}"));
            fetch_index(app, then);
        }
        StoreEvent::LoggedIn(Err(e)) => {
            if let Some(Modal::StoreLogin(l)) = app.modal.as_mut() {
                l.busy = false;
                l.error = Some(e);
            }
        }
        StoreEvent::Index(Ok((index, off)), query) => {
            for name in off {
                app.push(Role::System, format!("«{name}» fue retirado de la tienda: lo desactivé · /store remove {name} para borrarlo"));
            }
            if query == "\u{0}update" {
                let ups = store::updates(&index);
                if ups.is_empty() {
                    app.push(Role::System, "todo lo de la tienda está al día");
                }
                install_updates(app, index, ups);
                return;
            }
            if index.packages.is_empty() {
                app.push(Role::System, "la tienda está vacía todavía · crea el primero en https://iris.knarvaez.com/studio");
                return;
            }
            let mut m = StoreModal { index, tab: 0, sel: 0, query, before: app.buddy.clone(), busy: None };
            if m.list().is_empty() {
                m.tab = 1;
            }
            app.modal = Some(Modal::Store(m));
            preview(app);
        }
        StoreEvent::Ready { name, preview: true, result } => {
            let Some(Modal::Store(m)) = &mut app.modal else { return };
            if m.busy.as_deref() == Some(&name) {
                m.busy = None;
            }
            let Some(e) = m.list().get(m.sel).cloned() else { return };
            if e.name != name {
                return;
            }
            match result {
                Err(why) => app.flash = Some((why, Instant::now())),
                Ok(()) if e.kind == "buddy" => app.buddy = Buddy::Recipe(name),
                Ok(()) => {
                    let before = m.before.clone();
                    if let Some(dir) = preview_dir() {
                        if with_piece(&before, &e, &dir, ".preview").is_ok() {
                            app.buddy = Buddy::Recipe(".preview".into());
                        }
                    }
                }
            }
        }
        StoreEvent::Ready { result: Err(why), .. } if why == store::NEED_LOGIN => ask_login(app, String::new()),
        StoreEvent::Ready { name, preview: false, result: Err(why) } => app.push(Role::Error, format!("no se instaló «{name}»: {why}")),
        StoreEvent::Ready { name, preview: false, result: Ok(()) } => {
            let kind = store::installed().into_iter().find(|i| i.0 == name).map(|i| i.3).unwrap_or_default();
            if kind == "buddy" {
                crate::actions::set_buddy(app, Buddy::Recipe(name.clone()));
                app.push(Role::System, format!("«{name}» instalado y puesto · /buddy para cambiar"));
            } else {
                let base = match &app.buddy {
                    Buddy::Recipe(n) if n == ".preview" => Buddy::Original,
                    b => b.clone(),
                };
                let mine = format!("{}-con-{name}", base.id());
                let entry = store::cached().and_then(|i| i.find(&name, None).cloned());
                let ok = match (crate::paths::config(), entry) {
                    (Some(c), Some(e)) => with_piece(&base, &e, &c.join("buddies").join(&mine), &mine),
                    _ => Err("no encuentro la pieza en el índice".into()),
                };
                match ok {
                    Ok(()) => {
                        crate::actions::set_buddy(app, Buddy::Recipe(mine.clone()));
                        app.push(Role::System, format!("«{name}» instalada: tu buddy ahora es «{mine}» (una copia; el de antes sigue en /buddy)"));
                    }
                    Err(e) => app.push(Role::Error, format!("«{name}» se instaló pero no pude ponerla: {e}")),
                }
            }
        }
        StoreEvent::Draft(name) => {
            if app.buddy != Buddy::Recipe(name.clone()) {
                app.flash = Some((format!("borrador «{name}» actualizado · /buddy {name}"), Instant::now()));
            }
        }
        StoreEvent::Updates(index, list) => app.store_updates = Some((index, list)),
        StoreEvent::Updated { name, version, result: Ok(()) } => {
            app.push(Role::System, format!("«{name}» actualizado a la {version}"));
            // The buddy is read again, with the new version of whatever it uses.
            app.shown_recipe = None;
        }
        StoreEvent::Updated { name, result: Err(why), .. } => app.push(Role::Error, format!("no se actualizó «{name}»: {why}")),
        StoreEvent::Published(Ok(msg)) => app.push(Role::System, msg),
        StoreEvent::Published(Err(e)) => app.push(Role::Error, format!("no se publicó: {e}")),
        StoreEvent::Liked(Ok((name, n))) => app.flash = Some((format!("♥ le diste like a «{name}» · ya van {n}"), Instant::now())),
        StoreEvent::Liked(Err(e)) => app.flash = Some((format!("no se pudo dar like: {e}"), Instant::now())),
    }
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let Some(Modal::Store(m)) = &mut app.modal else { return };
    let n = m.list().len();
    match k.code {
        KeyCode::Tab | KeyCode::BackTab => {
            m.tab = 1 - m.tab;
            m.sel = 0;
        }
        KeyCode::Up if n > 0 => m.sel = (m.sel + n - 1) % n,
        KeyCode::Down if n > 0 => m.sel = (m.sel + 1) % n,
        KeyCode::Esc => {
            let before = m.before.clone();
            app.modal = None;
            app.buddy = before;
            if let Some(d) = preview_dir() {
                let _ = std::fs::remove_dir_all(d);
            }
            return;
        }
        KeyCode::Enter => {
            let Some(e) = m.list().get(m.sel).cloned() else { return };
            let (index, before) = (m.index.clone(), m.before.clone());
            app.modal = None;
            app.buddy = before;
            app.push(Role::System, format!("instalando «{}» {}…", e.name, e.version));
            download(app.events.clone(), index, e, false);
            return;
        }
        KeyCode::Char('l') => {
            let Some(e) = m.list().get(m.sel).cloned() else { return };
            let Some(token) = store::token() else {
                app.flash = Some(("para dar like entra a la tienda: iris login (en otra terminal)".into(), Instant::now()));
                return;
            };
            let tx = app.events.clone();
            std::thread::spawn(move || {
                let r = crate::net::json("PUT", &format!("/api/pieces/{}/like", e.name), Some(&token), None).and_then(|(code, v)| {
                    if code >= 400 {
                        return Err(crate::net::error_of(code, &v));
                    }
                    Ok((e.name.clone(), v.get("likes").and_then(|n| n.as_i64()).unwrap_or(0)))
                });
                let _ = tx.send(AppEvent::Store(StoreEvent::Liked(r)));
            });
            return;
        }
        KeyCode::Char('r') => {
            if let Some(e) = m.list().get(m.sel).cloned() {
                let name = e.name.clone();
                std::thread::spawn(move || {
                    let _ = crate::net::json("POST", &format!("/api/pieces/{name}/report"), None, Some(&serde_json::json!({"reason": "reportado desde Iris"})));
                });
                app.flash = Some((format!("«{}» reportado; Kevin lo revisa", e.name), Instant::now()));
            }
            return;
        }
        _ => return,
    }
    preview(app);
}

/// Every few seconds, with a store session, brings the drafts from Iris Studio into
/// `~/.config/iris/buddies/<name>/` (the core reloads them on its own when they change).
pub fn sync_drafts(tx: Sender<AppEvent>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(3));
        let (Some(token), Some(root)) = (store::token(), crate::paths::config().map(|c| c.join("buddies"))) else { continue };
        let Ok((_, list)) = crate::net::json("GET", "/api/me/drafts", Some(&token), None) else { continue };
        for d in list.as_array().into_iter().flatten() {
            let (Some(name), Some(at)) = (d.get("name").and_then(|v| v.as_str()), d.get("updated_at").and_then(|v| v.as_str())) else { continue };
            if name.starts_with('.') || name.contains('/') || name.contains('\\') {
                continue;
            }
            let dir = root.join(name);
            if std::fs::read_to_string(dir.join(".draft")).is_ok_and(|s| s.trim() == at) {
                continue;
            }
            let Ok((_, full)) = crate::net::json("GET", &format!("/api/me/drafts/{name}"), Some(&token), None) else { continue };
            let Some(files) = full.get("files").and_then(|f| f.as_object()) else { continue };
            let _ = std::fs::create_dir_all(&dir);
            for (f, body) in files {
                if f.contains('/') || f.contains('\\') || f.starts_with('.') {
                    continue;
                }
                let Some(text) = body.as_str() else { continue };
                // Binary files (a part's .wasm) come in base64, as `<name>.b64`.
                if let Some(bin) = f.strip_suffix(".b64") {
                    use base64::Engine;
                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(text.trim()) {
                        let _ = std::fs::write(dir.join(bin), bytes);
                    }
                    continue;
                }
                let _ = std::fs::write(dir.join(f), text);
            }
            let _ = std::fs::write(dir.join(".draft"), at);
            let _ = tx.send(AppEvent::Store(StoreEvent::Draft(name.to_string())));
        }
    });
}

/// `/buddy publish`: the current buddy, as a store package (it goes into review).
pub fn publish_buddy(app: &mut App) {
    let Buddy::Recipe(name) = app.buddy.clone() else {
        app.push(Role::Error, "los buddies de fábrica ya están en Iris · haz el tuyo en Iris Studio o con iris piece new <nombre> --buddy");
        return;
    };
    let Some(dir) = buddy::recipe_dir(&name) else { return };
    let tx = app.events.clone();
    app.push(Role::System, format!("publicando «{name}»…"));
    std::thread::spawn(move || {
        let r = (|| -> Result<String, String> {
            let token = store::token().ok_or("primero entra a la tienda: iris login (en otra terminal)")?;
            if !dir.join("iris.toml").exists() {
                std::fs::write(dir.join("iris.toml"), format!("[package]\nname = \"{name}\"\nkind = \"buddy\"\nversion = \"1.0.0\"\ndescription = \"{name}, un buddy hecho en Iris\"\nlicense = \"MIT\"\n")).map_err(|e| e.to_string())?;
            }
            let src = std::fs::read_to_string(dir.join("recipe.toml")).map_err(|e| e.to_string())?;
            let bytes = store::pack(&dir, &[("recipe.toml", src.into_bytes())])?;
            let (code, body) = crate::net::call("POST", "/api/pieces", Some(&token), Some(("application/zip", &bytes)))?;
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            if code >= 400 {
                return Err(crate::net::error_of(code, &v));
            }
            Ok(format!("«{name}» subido: queda en revisión"))
        })();
        let _ = tx.send(AppEvent::Store(StoreEvent::Published(r)));
    });
}
