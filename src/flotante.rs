//! El modo flotante (la ventana que se llama con un atajo) y las notificaciones del escritorio.

use crate::*;

pub(crate) const FLOTANTE_PROMPT: &str = "Modo flotante: Kevin te llamó con un atajo desde el escritorio y te habla por voz; \
tu respuesta se lee en voz alta. Si pide una acción del sistema (volumen, brillo, tema, abrir o cerrar apps, \
música, capturas, recordatorios), hazla directo con los comandos de Omarchy, Hyprland o wpctl, sin pedir \
confirmación salvo que sea destructiva, y contesta en UNA frase corta en español, sin markdown. Para preguntas, \
contesta breve.";

/// Modo flotante: corta la escucha cuando te callas (o si no dijiste nada) y se esconde un
/// rato después de terminar de contestar.
pub(crate) fn flotante_tick(app: &mut App, voice_tx: &Sender<VoiceCmd>) {
    if !app.flotante {
        return;
    }
    if let Some(start) = app.hands_free {
        if app.voice != VoiceState::Listening {
            if start.elapsed() > Duration::from_secs(3) {
                app.hands_free = None; // ya terminó por otro lado
            }
        } else if app.heard && app.last_voice.elapsed() > Duration::from_millis(1300) {
            let _ = voice_tx.send(VoiceCmd::Stop);
            app.hands_free = None;
        } else if !app.heard && start.elapsed() > Duration::from_secs(7) {
            let _ = voice_tx.send(VoiceCmd::Cancel);
            app.hands_free = None;
            app.hide_at = Some(Instant::now());
        } else if start.elapsed() > Duration::from_secs(20) {
            let _ = voice_tx.send(VoiceCmd::Stop);
            app.hands_free = None;
        }
    }
    let quiet = !app.busy && !app.talking && app.voice != VoiceState::Listening && app.voice != VoiceState::Transcribing;
    if let Some(at) = app.hide_at {
        if quiet && app.focused && Instant::now() >= at {
            app.hide_at = None;
            hide_flotante();
        } else if !quiet {
            app.hide_at = None;
        }
    }
}

pub(crate) fn hide_flotante() {
    let _ = std::process::Command::new("hyprctl")
        // Con la config en Lua, «dispatch» recibe una llamada de hl.dsp (la sintaxis vieja falla).
        .args(["dispatch", "hl.dsp.workspace.toggle_special(\"jarvis\")"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Una notificación del escritorio (notify-send); si no está, no pasa nada.
pub(crate) fn notify(title: &str, body: &str) {
    let body: String = body.chars().take(140).collect();
    let _ = std::process::Command::new("notify-send")
        .args(["-a", "JARVIS", title, &body])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
