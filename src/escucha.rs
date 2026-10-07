//! Lo que llega del dictado (whisper) y cuándo Iris te escucha.

use crate::*;

pub(crate) fn on_voice(app: &mut App, ev: VoiceEvent, claude: &mut Option<Claude>) {
    match ev {
        VoiceEvent::Ready(name) => {
            app.voice = VoiceState::Ready;
            app.voice_model = name;
        }
        VoiceEvent::Unavailable(e) => {
            app.voice = VoiceState::Off;
            app.push(Role::Error, format!("voz desactivada: {e}"));
        }
        VoiceEvent::Listening => {
            app.voice = VoiceState::Listening;
            app.last_voice = Instant::now();
            app.noise_floor = 0.0;
        }
        VoiceEvent::Discarded(t) => {
            app.voice = VoiceState::Ready;
            app.nucleo.fire(Gesto::Nope);
            app.push(Role::System, format!("descarté «{t}»: whisper lo inventa sobre el ruido"));
        }
        VoiceEvent::Transcribing => app.voice = VoiceState::Transcribing,
        VoiceEvent::Transcript(t) if t.is_empty() => {
            app.voice = VoiceState::Ready;
            app.push(Role::System, "no escuché nada");
        }
        VoiceEvent::Transcript(t) if matches!(app.modal, Some(Modal::Ask(_))) => {
            app.voice = VoiceState::Ready;
            answer(app, claude, &t);
        }
        VoiceEvent::Transcript(t) => {
            app.voice = VoiceState::Ready;
            app.spoken_next = true;
            submit(app, claude, t);
        }
        VoiceEvent::Cancelled => app.voice = VoiceState::Ready,
        VoiceEvent::Error(e) => {
            app.voice = VoiceState::Ready;
            app.push(Role::Error, e);
        }
    }
}

/// Sigue el piso de ruido mientras escucha y anota cuándo hubo voz por encima. Un umbral fijo
/// no sirve: en una pieza con ruido de fondo el silencio ya marca tanto como la voz baja.
pub(crate) fn hear(app: &mut App, lvl: f32) {
    let floor = &mut app.noise_floor;
    // Baja de golpe con el silencio y sube muy despacio con la voz.
    *floor = if *floor == 0.0 || lvl < *floor { lvl } else { *floor + (lvl - *floor) * 0.002 };
    if lvl > (*floor * 1.8).max(*floor + 0.01) {
        app.last_voice = Instant::now();
        app.heard = true;
    }
}
