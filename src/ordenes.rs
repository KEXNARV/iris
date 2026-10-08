//! Las órdenes con barra que cambian algo de Iris: /model, /theme, /buddy, /core, /resume…

use crate::*;

pub(crate) fn clear(app: &mut App) -> Flow {
    app.messages.clear();
    app.md_cache.borrow_mut().clear();
    app.adjuntos_cache.borrow_mut().clear();
    app.activity.clear();
    app.turn_from = 0;
    Flow::Redraw
}

/// Sin argumento abre la lista; con uno (id, alias como `sonnet` o nombre de la lista) lo pide.
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

/// Sin argumento abre la lista; con uno lo pide. El motor acepta cualquier texto sin quejarse,
/// así que el nivel se valida aquí.
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

/// `/theme`: sin argumento abre la lista con vista previa; con uno (`cabina`, `cine`…) lo aplica.
pub(crate) fn theme(app: &mut App, arg: &str) {
    if arg.is_empty() {
        let sel = estilo::TODOS.iter().position(|e| e.0 == app.estilo).unwrap_or(0);
        app.modal = Some(Modal::Estilo { sel, antes: app.estilo });
        return;
    }
    match estilo::Estilo::buscar(arg) {
        Some(e) => set_estilo(app, e),
        None => {
            let ids: Vec<&str> = estilo::TODOS.iter().map(|e| e.1).collect();
            app.push(Role::Error, format!("no conozco el estilo «{arg}» · hay {}", ids.join(", ")));
        }
    }
}

/// `/buddy`: sin argumento abre la lista con vista previa; con uno (`baymax`, `original`) lo aplica.
pub(crate) fn elegir_buddy(app: &mut App, arg: &str) {
    if arg.is_empty() {
        let sel = buddy::TODOS.iter().position(|b| b.0 == app.buddy).unwrap_or(0);
        app.modal = Some(Modal::Buddy { sel, antes: app.buddy });
        return;
    }
    match buddy::Buddy::buscar(arg) {
        Some(b) => set_buddy(app, b),
        None => {
            let ids: Vec<&str> = buddy::TODOS.iter().map(|b| b.1).collect();
            app.push(Role::Error, format!("no conozco el buddy «{arg}» · hay {}", ids.join(", ")));
        }
    }
}

pub(crate) fn set_buddy(app: &mut App, b: buddy::Buddy) {
    app.buddy = b;
    let msg = match buddy::guardar(b) {
        Ok(()) => format!("buddy {}", b.nombre()),
        Err(err) => format!("buddy {} (no se guardó: {err})", b.nombre()),
    };
    app.flash = Some((msg, Instant::now()));
}

/// `/core`: sin argumento, la lista con vista previa; con uno (`auto`, `rosa`, `#ffd84a`) lo
/// aplica.
pub(crate) fn color(app: &mut App, arg: &str) {
    if app.buddy.paleta().is_none() {
        app.push(Role::Error, String::from("los colores del núcleo son de Baymax · /buddy baymax"));
        return;
    }
    if arg.is_empty() {
        let antes = baymax::nucleo();
        let sel = baymax::OPCIONES.iter().position(|o| o.2 == antes).unwrap_or(0);
        app.modal = Some(Modal::Color { sel, antes });
        return;
    }
    match baymax::Modo::buscar(arg) {
        Some(m) => set_color(app, m),
        None => {
            let ids: Vec<&str> = baymax::OPCIONES.iter().map(|o| o.0).collect();
            app.push(Role::Error, format!("no conozco el color «{arg}» · hay {} o un #rrggbb", ids.join(", ")));
        }
    }
}

pub(crate) fn set_color(app: &mut App, m: baymax::Modo) {
    baymax::poner_nucleo(m);
    let msg = match theme::guardar("nucleo", &m.texto()) {
        Ok(()) => format!("núcleo {}", m.nombre()),
        Err(err) => format!("núcleo {} (no se guardó: {err})", m.nombre()),
    };
    app.flash = Some((msg, Instant::now()));
}

pub(crate) fn set_estilo(app: &mut App, e: estilo::Estilo) {
    app.estilo = e;
    let msg = match estilo::guardar(e) {
        Ok(()) => format!("estilo {}", e.nombre()),
        Err(err) => format!("estilo {} (no se guardó: {err})", e.nombre()),
    };
    app.flash = Some((msg, Instant::now()));
}

/// Responde la pregunta en curso (escrita, dictada o elegida) y, si era la última, la envía.
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

/// `claude -p` no entiende `/resume`: sin argumento abre la lista de sesiones, con `N` o un prefijo
/// del id relanza el motor sobre esa sesión.
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
