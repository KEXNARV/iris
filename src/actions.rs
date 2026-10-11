//! The slash commands that change something in Iris: /model, /theme, /buddy, /core, /resume…

use crate::*;

pub(crate) fn clear(app: &mut App) -> Flow {
    app.messages.clear();
    app.md_cache.borrow_mut().clear();
    app.attachments_cache.borrow_mut().clear();
    app.activity.clear();
    app.turn_from = 0;
    Flow::Redraw
}

/// With no argument, opens the list; with one (id, alias like `sonnet` or list name), requests it.
pub(crate) fn model(app: &mut App, claude: &mut Option<Claude>, arg: &str) {
    if arg.is_empty() {
        let sel = commands::MODELS.iter().position(|m| m.0 == app.model).unwrap_or(0);
        app.modal = Some(Modal::Model { sel });
        return;
    }
    let q = arg.to_lowercase();
    let id = commands::MODELS
        .iter()
        .find(|m| m.0 == q || m.1.to_lowercase().starts_with(&q))
        .map_or(arg, |m| m.0);
    let Some(c) = claude.as_mut().filter(|_| app.alive) else {
        app.push(Role::Error, "claude no está corriendo (Ctrl+R)");
        return;
    };
    match c.set_model(id) {
        Ok(()) => {
            app.model = id.to_string();
            let when = if app.busy { "desde el próximo turno" } else { "listo" };
            app.push(Role::System, format!("modelo → {id} ({when})"));
        }
        Err(e) => app.push(Role::Error, format!("no pude cambiar de modelo: {e}")),
    }
}

/// With no argument, opens the list; with one, requests it. The engine accepts any text without
/// complaining, so the level is validated here.
pub(crate) fn effort(app: &mut App, claude: &mut Option<Claude>, arg: &str) {
    let q = arg.to_lowercase();
    if q.is_empty() {
        let sel = commands::EFFORTS.iter().position(|e| e.0 == app.effort).unwrap_or(0);
        app.modal = Some(Modal::Effort { sel });
        return;
    }
    let level = match q.as_str() {
        "auto" | "default" => None,
        l if commands::EFFORTS.iter().any(|e| e.0 == l) => Some(l),
        _ => {
            let all: Vec<_> = commands::EFFORTS.iter().map(|e| e.0).collect();
            app.push(Role::Error, format!("effort desconocido: {arg} (usa {})", all.join(", ")));
            return;
        }
    };
    let Some(c) = claude.as_mut().filter(|_| app.alive) else {
        app.push(Role::Error, "claude no está corriendo (Ctrl+R)");
        return;
    };
    match c.set_effort(level) {
        Ok(()) => {
            app.effort = level.unwrap_or("auto").to_string();
            let when = if app.busy { "desde el próximo turno" } else { "listo" };
            app.push(Role::System, format!("effort → {} ({when})", app.effort));
        }
        Err(e) => app.push(Role::Error, format!("no pude cambiar el effort: {e}")),
    }
}

/// `/theme`: with no argument, opens the list with preview; with one (`cabina`, `cine`…), applies it.
pub(crate) fn theme(app: &mut App, arg: &str) {
    if arg.is_empty() {
        let sel = style::ALL.iter().position(|e| e.0 == app.style).unwrap_or(0);
        app.modal = Some(Modal::UiStyle { sel, before: app.style });
        return;
    }
    match style::UiStyle::find(arg) {
        Some(e) => set_style(app, e),
        None => {
            let ids: Vec<&str> = style::ALL.iter().map(|e| e.1).collect();
            app.push(Role::Error, format!("no conozco el estilo «{arg}» · hay {}", ids.join(", ")));
        }
    }
}

/// `/buddy`: with no argument, opens the list with preview; with one (`baymax`, `original`), applies it.
pub(crate) fn choose_buddy(app: &mut App, arg: &str) {
    if arg.is_empty() {
        let list = buddy::all();
        let sel = list.iter().position(|e| e.buddy == app.buddy).unwrap_or(0);
        app.modal = Some(Modal::Buddy { sel, before: app.buddy.clone(), list });
        return;
    }
    match buddy::Buddy::find(arg) {
        Some(b) => set_buddy(app, b),
        None => {
            let names: Vec<String> = buddy::all().into_iter().map(|e| e.buddy.id()).collect();
            let ids: Vec<&str> = names.iter().map(String::as_str).collect();
            app.push(Role::Error, format!("no conozco el buddy «{arg}» · hay {}", ids.join(", ")));
        }
    }
}

pub(crate) fn set_buddy(app: &mut App, b: buddy::Buddy) {
    let msg = match buddy::save(&b) {
        Ok(()) => format!("buddy {}", b.name()),
        Err(err) => format!("buddy {} (no se guardó: {err})", b.name()),
    };
    app.buddy = b;
    app.flash = Some((msg, Instant::now()));
}

/// `/core`: with no argument, the list with preview; with one (`auto`, `rosa`, `#ffd84a`), applies
/// it.
pub(crate) fn color(app: &mut App, arg: &str) {
    if app.buddy.palette().is_none() {
        app.push(Role::Error, String::from("los colores del núcleo son de Baymax · /buddy baymax"));
        return;
    }
    if arg.is_empty() {
        let before = baymax::core();
        let sel = baymax::OPTIONS.iter().position(|o| o.2 == before).unwrap_or(0);
        app.modal = Some(Modal::Color { sel, before });
        return;
    }
    match baymax::Mode::find(arg) {
        Some(m) => set_color(app, m),
        None => {
            let ids: Vec<&str> = baymax::OPTIONS.iter().map(|o| o.0).collect();
            app.push(Role::Error, format!("no conozco el color «{arg}» · hay {} o un #rrggbb", ids.join(", ")));
        }
    }
}

pub(crate) fn set_color(app: &mut App, m: baymax::Mode) {
    baymax::set_core(m);
    let msg = match theme::save("nucleo", &m.text()) {
        Ok(()) => format!("núcleo {}", m.name()),
        Err(err) => format!("núcleo {} (no se guardó: {err})", m.name()),
    };
    app.flash = Some((msg, Instant::now()));
}

pub(crate) fn set_style(app: &mut App, e: style::UiStyle) {
    app.style = e;
    let msg = match style::save(e) {
        Ok(()) => format!("estilo {}", e.name()),
        Err(err) => format!("estilo {} (no se guardó: {err})", e.name()),
    };
    app.flash = Some((msg, Instant::now()));
}

/// Answers the current question (typed, dictated or picked) and, if it was the last, sends it.
pub(crate) fn answer(app: &mut App, claude: &mut Option<Claude>, typed: &str) {
    let Some(Modal::Ask(a)) = app.modal.as_mut() else { return };
    let q = a.current().text.clone();
    let outcome = a.answer(typed);
    let id = a.request_id.clone();
    let res = match outcome {
        Outcome::Next => return,
        Outcome::Allow(input) => {
            if let Some(ans) = input["answers"].as_object() {
                let mut text = String::new();
                for (q, v) in ans {
                    text.push_str(&format!("{q} → {}\n", v.as_str().unwrap_or("")));
                }
                app.push(Role::System, text.trim_end().to_string());
            } else {
                app.push(Role::System, format!("permitido: {q}"));
            }
            claude.as_mut().map(|c| c.allow(&id, input))
        }
        Outcome::Always(input, rules) => {
            app.push(Role::System, format!("permitido en esta sesión: {q}"));
            claude.as_mut().map(|c| c.allow_always(&id, input, rules))
        }
        Outcome::Deny(why) => {
            app.push(Role::System, format!("rechazado: {q}"));
            claude.as_mut().map(|c| c.deny(&id, &why))
        }
    };
    app.modal = None;
    if let Some(Err(e)) = res {
        app.push(Role::Error, format!("no pude responder: {e}"));
    }
}

/// `claude -p` doesn't understand `/resume`: with no argument, opens the session list; with `N` or
/// an id prefix, relaunches the engine on that session.
pub(crate) fn resume(app: &mut App, arg: &str) -> Flow {
    if app.busy {
        app.push(Role::Error, "espera a que termine el turno (o Esc) antes de cambiar de sesión");
        return Flow::Go;
    }
    if arg.is_empty() {
        let list = sessions::list(&app.session, 30);
        if list.is_empty() {
            app.push(Role::System, "no hay sesiones anteriores en este directorio");
        } else {
            app.modal = Some(Modal::Sessions { list, sel: 0 });
        }
        return Flow::Go;
    }
    match sessions::find(&app.session, arg) {
        Some(id) => Flow::Resume(id),
        None => {
            app.push(Role::Error, format!("no encontré la sesión «{arg}»; `/resume` para ver la lista"));
            Flow::Go
        }
    }
}

/// Keeps the core showing the chosen recipe, and reloads it when its files change (edited by
/// hand, or a draft arriving from Iris Studio). A recipe that fails says why and leaves the core
/// as it was.
pub(crate) fn sync_recipe(app: &mut App) {
    let name = match &app.buddy {
        buddy::Buddy::Recipe(n) => n.clone(),
        _ => {
            if app.shown_recipe.take().is_some() {
                let _ = app.core.set_recipe(None);
            }
            return;
        }
    };
    let same = app.shown_recipe.as_ref().is_some_and(|s| s.0 == name);
    if same && app.recipe_checked.elapsed() < Duration::from_millis(500) {
        return;
    }
    app.recipe_checked = Instant::now();
    let Some(dir) = buddy::recipe_dir(&name) else {
        app.push(Role::Error, format!("no encuentro el buddy «{name}»: vuelve el original"));
        app.buddy = buddy::Buddy::Original;
        return;
    };
    let stamp = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .max()
        .unwrap_or(std::time::UNIX_EPOCH);
    if app.shown_recipe.as_ref().is_some_and(|s| s.0 == name && s.1 == stamp) {
        return;
    }
    let reload = same;
    app.shown_recipe = Some((name.clone(), stamp));
    match core::recipe::Recipe::load_dir(&dir).and_then(|r| app.core.set_recipe(Some(&r))) {
        Ok(()) if reload => app.flash = Some((format!("{name} se actualizó"), Instant::now())),
        Ok(()) => {}
        Err(e) => app.push(Role::Error, format!("el buddy «{name}» no se pudo armar: {e}")),
    }
}

/// A part of the recipe in the core stopped working: it's taken out (the rest goes on) and the
/// user is told what happened, in plain words. A store piece that fails three times is turned
/// off until a new version comes out.
pub(crate) fn broken_parts(app: &mut App) {
    for (part, why) in app.core.take_broken() {
        let name = part.split('@').next().unwrap_or(&part).to_string();
        let msg = if part.contains('@') {
            match store::failed(&name) {
                n if n >= 3 => format!("«{name}» falló tres veces y quedó desactivada hasta que salga una versión nueva · /store remove {name} para quitarla"),
                _ => format!("«{name}» dejó de responder y la quité de tu buddy; lo demás sigue igual · se reactiva al volver a elegir el buddy"),
            }
        } else {
            format!("«{name}» dejó de responder y la quité de tu buddy; lo demás sigue igual")
        };
        app.push(Role::System, format!("{msg} ({why})"));
    }
}

/// `/buddy edit`: the current buddy in Iris Studio, on the web. With a store session it goes up
/// as a draft and comes back live as it's edited; without one, the recipe's folder opens to edit
/// by hand (Iris reloads it on every save).
pub(crate) fn edit_buddy(app: &mut App) {
    let name = match &app.buddy {
        buddy::Buddy::Recipe(n) => n.clone(),
        b => {
            // A built-in one: a copy of it becomes the user's.
            let mine = format!("mi-{}", b.id());
            let Some(dir) = crate::paths::config().map(|c| c.join("buddies").join(&mine)) else { return };
            let src = if *b == buddy::Buddy::Baymax { include_str!("core/recipe/baymax.toml") } else { include_str!("core/recipe/original.toml") };
            if !dir.join("recipe.toml").exists() {
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("recipe.toml"), src.replacen(&format!("name = \"{}\"", b.id()), &format!("name = \"{mine}\""), 1));
            }
            set_buddy(app, buddy::Buddy::Recipe(mine.clone()));
            mine
        }
    };
    let Some(dir) = buddy::recipe_dir(&name) else { return };
    let Some(token) = store::token() else {
        crate::link::open(&dir.display().to_string());
        app.push(Role::System, format!("tu buddy está en {} · edita recipe.toml con lo que quieras y se recarga solo · para Iris Studio: iris login", dir.display()));
        return;
    };
    let files: serde_json::Map<String, serde_json::Value> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "toml") && e.file_name() != "iris.toml")
        .filter_map(|e| Some((e.file_name().to_string_lossy().to_string(), serde_json::Value::String(std::fs::read_to_string(e.path()).ok()?))))
        .collect();
    let body = serde_json::json!({ "files": files });
    match crate::net::json("PUT", &format!("/api/me/drafts/{name}"), Some(&token), Some(&body)) {
        Ok(_) => {
            let url = format!("{}/studio?draft={name}", crate::net::base());
            crate::link::open(&url);
            app.push(Role::System, format!("«{name}» abierto en Iris Studio ({url}): lo que cambies ahí aparece aquí solo"));
        }
        Err(e) => app.push(Role::Error, format!("no pude abrirlo en Iris Studio: {e}")),
    }
}
