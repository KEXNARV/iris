//! What arrives from dictation (whisper) and when Iris is listening to you.

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
            app.core.fire(Gesture::Nope);
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

/// Tracks the noise floor while listening and notes when there was voice above it. A fixed threshold
/// doesn't work: in a room with background noise, silence already reads as high as a soft voice.
pub(crate) fn hear(app: &mut App, lvl: f32) {
    let floor = &mut app.noise_floor;
    // Drops at once with silence and rises very slowly with the voice.
    *floor = if *floor == 0.0 || lvl < *floor { lvl } else { *floor + (lvl - *floor) * 0.002 };
    if lvl > (*floor * 1.8).max(*floor + 0.01) {
        app.last_voice = Instant::now();
        app.heard = true;
    }
}
