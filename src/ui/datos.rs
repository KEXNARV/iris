//! Lo que se muestra de la sesión, ya en texto: contexto, costo, voz, turnos, atajos y avisos.

use super::*;

/// El aviso breve («copiado…», «estilo Cine») mientras dura.
pub(in crate::ui) fn flash(app: &App) -> Option<Line<'static>> {
    match &app.flash {
        Some((msg, at)) if at.elapsed().as_secs_f64() < 2.5 => {
            Some(Line::from(Span::styled(format!(" ✓ {msg}"), Style::new().fg(GREEN).bold())))
        }
        _ => None,
    }
}

pub(in crate::ui) fn voice_label(app: &App) -> String {
    match app.voice {
        VoiceState::Off => "voz off".to_string(),
        VoiceState::Loading => "voz cargando".to_string(),
        _ => format!("voz {}", app.voice_model),
    }
}

pub(in crate::ui) fn ctx_pct(app: &App) -> f64 {
    app.ctx_used as f64 * 100.0 / app.ctx_window.max(1) as f64
}

pub(in crate::ui) fn uptime(app: &App) -> String {
    let up = app.started.elapsed().as_secs();
    format!("{:02}:{:02}", up / 3600, up / 60 % 60)
}

pub(in crate::ui) fn turns(app: &App) -> String {
    format!("{} turno{}", app.turns, if app.turns == 1 { "" } else { "s" })
}

pub(in crate::ui) fn stats(app: &App) -> String {
    let ctx = match app.ctx_used {
        0 => String::new(),
        _ => format!("contexto {:.0}% · ", ctx_pct(app)),
    };
    format!("{} · {ctx}{} · ${:.2} · {}", voice_label(app), turns(app), app.cost, uptime(app))
}

pub(in crate::ui) fn cwd() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    std::env::current_dir()
        .map(|p| p.display().to_string().replacen(&home, "~", 1))
        .unwrap_or_default()
}

/// La voz sin la palabra «voz», para cuando ya va rotulada.
pub(in crate::ui) fn voice_value(app: &App) -> String {
    match app.voice {
        VoiceState::Off => "apagada".into(),
        VoiceState::Loading => "cargando…".into(),
        VoiceState::Listening => format!("{} · escuchando", app.voice_model),
        VoiceState::Transcribing => format!("{} · transcribiendo", app.voice_model),
        VoiceState::Ready => format!("{} · lista", app.voice_model),
    }
}

/// Las herramientas del turno en curso.
pub(in crate::ui) fn turn_acts(app: &App) -> &[Activity] {
    &app.activity[app.turn_from.min(app.activity.len())..]
}

pub(in crate::ui) fn hints_for(app: &App, state: State) -> Vec<(&'static str, &'static str)> {
    if listening(state) {
        vec![("suelta espacio", "enviar"), ("esc", "cancelar")]
    } else if app.busy {
        vec![("esc", "interrumpir"), ("espacio", "hablar"), ("pgup/pgdn", "desplazar")]
    } else {
        vec![("espacio", "hablar"), ("enter", "enviar"), ("/", "comandos"), ("/theme", "estilo"), ("^C", "salir")]
    }
}

pub(in crate::ui) fn ses8(app: &App) -> String {
    app.session.get(..8).unwrap_or("········").to_string()
}

pub(in crate::ui) fn model_short(app: &App) -> String {
    app.model.trim_start_matches("claude-").to_string()
}
