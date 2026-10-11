//! Voice: microphone capture with cpal and local transcription with whisper.cpp.
//!
//! Everything lives on its own thread: cpal's `Stream` is not `Send` and whisper blocks for
//! several seconds, so the UI only sends commands and receives events.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::AppEvent;

const WHISPER_RATE: u32 = 16_000;

pub enum VoiceCmd {
    Start,
    Stop,
    Cancel,
}

pub enum VoiceEvent {
    Ready(String),
    Unavailable(String),
    Listening,
    Transcribing,
    Transcript(String),
    /// Whisper returned one of its made-up phrases over noise; it was discarded.
    Discarded(String),
    Cancelled,
    Error(String),
}

/// Microphone RMS level (f32 as bits) to draw the wave without going through the channel.
pub type Level = Arc<AtomicU32>;

pub fn level_get(l: &Level) -> f32 {
    f32::from_bits(l.load(Ordering::Relaxed))
}

pub fn spawn(tx: Sender<AppEvent>) -> (Sender<VoiceCmd>, Level) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let level: Level = Arc::new(AtomicU32::new(0));
    let lvl = level.clone();
    thread::spawn(move || {
        let send = |e| {
            let _ = tx.send(AppEvent::Voice(e));
        };
        let (ctx, name) = match load_model() {
            Ok(m) => m,
            Err(e) => return send(VoiceEvent::Unavailable(e.to_string())),
        };
        let _ = transcribe(&ctx, &vec![0.0; WHISPER_RATE as usize]); // compiles shaders now instead of on first use
        send(VoiceEvent::Ready(name));
        run(&ctx, cmd_rx, lvl, &send);
    });
    (cmd_tx, level)
}

/// On hybrid laptops ggml picks the integrated GPU (the first in Vulkan's list) and is
/// ~15× slower. It is restricted to the first discrete GPU. Call before creating threads.
pub fn prefer_discrete_gpu() {
    if std::env::var_os("GGML_VK_VISIBLE_DEVICES").is_some() {
        return;
    }
    let Ok(out) = std::process::Command::new("vulkaninfo").arg("--summary").output() else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let discrete = text
        .lines()
        .filter(|l| l.contains("deviceType"))
        .position(|l| l.contains("DISCRETE_GPU"));
    if let Some(i) = discrete {
        // SAFETY: called at the start of main, before spawning any thread.
        unsafe { std::env::set_var("GGML_VK_VISIBLE_DEVICES", i.to_string()) };
    }
}

fn models_dir() -> Result<PathBuf> {
    Ok(crate::paths::data().ok_or_else(|| anyhow::anyhow!("sin carpeta de datos"))?.join("models"))
}

fn model_path() -> Result<PathBuf> {
    if let Ok(p) = crate::paths::var("MODEL").ok_or(()) {
        return Ok(PathBuf::from(p));
    }
    let dir = models_dir()?;
    ["ggml-large-v3-turbo-q5_0.bin", "ggml-small-q5_1.bin"]
        .iter()
        .map(|f| dir.join(f))
        .find(|p| p.exists())
        .ok_or_else(|| anyhow!("no hay modelo whisper en {}", dir.display()))
}

fn load_model() -> Result<(WhisperContext, String)> {
    whisper_rs::install_logging_hooks(); // silences whisper.cpp; otherwise it garbles the TUI
    let path = model_path()?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("whisper")
        .trim_start_matches("ggml-")
        .to_string();
    let ctx = WhisperContext::new_with_params(
        path.to_str().ok_or_else(|| anyhow!("ruta no UTF-8"))?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow!("no pude cargar el modelo: {e:?}"))?;
    Ok((ctx, name))
}

fn run(ctx: &WhisperContext, rx: Receiver<VoiceCmd>, level: Level, send: &dyn Fn(VoiceEvent)) {
    let mut rec: Option<Recording> = None;
    for cmd in rx {
        match cmd {
            VoiceCmd::Start if rec.is_none() => match Recording::start(level.clone()) {
                Ok(r) => {
                    rec = Some(r);
                    send(VoiceEvent::Listening);
                }
                Err(e) => send(VoiceEvent::Error(format!("micrófono: {e}"))),
            },
            VoiceCmd::Stop => {
                let Some(r) = rec.take() else { continue };
                let audio = r.finish();
                level.store(0, Ordering::Relaxed);
                send(VoiceEvent::Transcribing);
                match transcribe(ctx, &audio) {
                    Ok(text) if is_hallucination(&text) => send(VoiceEvent::Discarded(text)),
                    Ok(text) => send(VoiceEvent::Transcript(text)),
                    Err(e) => send(VoiceEvent::Error(format!("whisper: {e}"))),
                }
            }
            VoiceCmd::Cancel => {
                rec = None;
                level.store(0, Ordering::Relaxed);
                send(VoiceEvent::Cancelled);
            }
            VoiceCmd::Start => {}
        }
    }
}

struct Recording {
    _stream: cpal::Stream,
    buf: Arc<Mutex<Vec<f32>>>,
    rate: u32,
}

impl Recording {
    fn start(level: Level) -> Result<Self> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or_else(|| anyhow!("no hay dispositivo de entrada"))?;
        let supported = device.default_input_config()?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let channels = config.channels as usize;
        let rate = config.sample_rate;
        let buf = Arc::new(Mutex::new(Vec::new()));

        // Mixes down to mono and updates the level on every block.
        let sink = {
            let buf = buf.clone();
            move |mono: &mut dyn Iterator<Item = f32>| {
                let block: Vec<f32> = mono.collect();
                if block.is_empty() {
                    return;
                }
                let rms = (block.iter().map(|x| x * x).sum::<f32>() / block.len() as f32).sqrt();
                level.store(rms.to_bits(), Ordering::Relaxed);
                buf.lock().unwrap().extend_from_slice(&block);
            }
        };
        let err = |_| {};
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                config,
                move |d: &[f32], _: &_| {
                    sink(&mut d.chunks(channels).map(|c| c.iter().sum::<f32>() / channels as f32))
                },
                err,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                config,
                move |d: &[i16], _: &_| {
                    sink(&mut d.chunks(channels).map(|c| {
                        c.iter().map(|&s| s as f32 / i16::MAX as f32).sum::<f32>() / channels as f32
                    }))
                },
                err,
                None,
            )?,
            f => return Err(anyhow!("formato de audio no soportado: {f:?}")),
        };
        stream.play()?;
        Ok(Self { _stream: stream, buf, rate })
    }

    /// Stops the capture and returns mono audio at 16 kHz.
    fn finish(self) -> Vec<f32> {
        let rate = self.rate;
        let samples = std::mem::take(&mut *self.buf.lock().unwrap());
        drop(self);
        // Opening the internal mic produces a clipping pop; nobody speaks that soon.
        let skip = (rate as usize / 5).min(samples.len());
        normalize(resample(&samples[skip..], rate, WHISPER_RATE))
    }
}

/// Brings the recording up to an even level before whisper. The mic gain has to stay low
/// (at 97% it clipped and whisper made up phrases; at 35% it doesn't, but the voice comes in
/// weak and you have to lean in and speak loudly): what's missing is raised here, without clipping.
fn normalize(mut audio: Vec<f32>) -> Vec<f32> {
    if audio.is_empty() {
        return audio;
    }
    // The «peak» is what only 0.1% of the samples exceed: a stray pop doesn't hold back the gain.
    let mut mags: Vec<f32> = audio.iter().map(|x| x.abs()).collect();
    let k = ((mags.len() as f32 * 0.999) as usize).min(mags.len() - 1);
    let peak = *mags.select_nth_unstable_by(k, f32::total_cmp).1;
    // Almost nothing recorded: don't make up a voice by raising the background noise.
    if peak < 0.003 {
        return audio;
    }
    // Without raising the noise: the internal mic has an RMS floor of ~0.04 at 40%, and amplified
    // whisper makes up phrases over it («¿Qué es el baile?»). The floor is the quietest 50 ms
    // stretch (10th percentile); after the gain it must stay below 0.03.
    let mut rms: Vec<f32> = audio
        .chunks(800)
        .map(|c| (c.iter().map(|x| x * x).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let floor = if rms.len() >= 4 {
        let k = rms.len() / 10;
        *rms.select_nth_unstable_by(k, f32::total_cmp).1
    } else {
        0.0
    };
    let quiet = if floor > 0.0 { 0.03 / floor } else { 10.0 };
    let gain = (0.9 / peak).min(quiet).clamp(1.0, 10.0);
    for x in &mut audio {
        *x = (*x * gain).clamp(-1.0, 1.0);
    }
    audio
}

fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let n = (input.len() as f64 / ratio) as usize;
    (0..n)
        .map(|i| {
            let pos = i as f64 * ratio;
            let j = pos as usize;
            let frac = (pos - j as f64) as f32;
            let a = input[j];
            let b = *input.get(j + 1).unwrap_or(&a);
            a + (b - a) * frac
        })
        .collect()
}

fn transcribe(ctx: &WhisperContext, audio: &[f32]) -> Result<String> {
    // Less than ~0.3 s is an accidental tap.
    if audio.len() < WHISPER_RATE as usize * 3 / 10 {
        return Ok(String::new());
    }
    let mut state = ctx.create_state().map_err(|e| anyhow!("{e:?}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let lang = crate::paths::var("LANG").ok_or(()).unwrap_or_else(|_| "es".into());
    params.set_language(Some(&lang));
    params.set_n_threads(threads());
    params.set_no_context(true);
    params.set_suppress_blank(true);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);

    // Silero VAD: whisper only sees the stretches with voice. Without it, over background noise
    // it makes up phrases ("¡Hasta la vuelta!", "Gracias por ver el video").
    let vad = models_dir().ok().map(|d| d.join("ggml-silero-v5.1.2.bin")).filter(|p| p.exists());
    let vad = vad.as_deref().and_then(|p| p.to_str());
    if vad.is_some() {
        let mut vp = whisper_rs::WhisperVadParams::new();
        vp.set_threshold(0.6);
        vp.set_min_speech_duration(300);
        params.set_vad_model_path(vad);
        params.set_vad_params(vp);
        params.enable_vad(true);
    }

    // whisper expects at least 1 s; it is padded with silence.
    let mut padded = audio.to_vec();
    padded.resize(padded.len().max(WHISPER_RATE as usize + 1600), 0.0);
    state.full(params, &padded).map_err(|e| anyhow!("{e:?}"))?;

    let text: String = state
        .as_iter()
        .filter(|s| s.no_speech_probability() < 0.6)
        .filter_map(|s| s.to_str_lossy().ok().map(|t| t.into_owned()))
        .collect();
    // Hallucinations are filtered by the caller (`is_hallucination`), so it can notify.
    Ok(clean(&text))
}

/// Phrases whisper produces over noise or silence (they come from YouTube subtitles in its
/// training). They are only discarded if they are the whole transcript.
fn is_hallucination(text: &str) -> bool {
    const KNOWN: &[&str] = &[
        "gracias",
        "muchas gracias",
        "gracias por ver",
        "gracias por ver el video",
        "hasta la vuelta",
        "hasta la próxima",
        "suscríbete",
        "subtítulos realizados por la comunidad de amara.org",
        "thank you",
        "thanks for watching",
        "you",
    ];
    let norm: String = text
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.')
        .collect::<String>()
        .trim_end_matches('.')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // "¡Hasta la vuelta! ¡Hasta la vuelta!" → it also counts if it's the same phrase repeated.
    let parts: Vec<&str> = norm.split('.').map(str::trim).filter(|p| !p.is_empty()).collect();
    !parts.is_empty() && parts.iter().all(|p| KNOWN.contains(p))
        || KNOWN.iter().any(|k| {
            let rep = norm.replace(k, "");
            !norm.is_empty() && rep.trim().is_empty()
        })
}

/// Strips the marks whisper makes up over silence ("[BLANK_AUDIO]", "(música)", …).
fn clean(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '[' | '(' | '*' if depth == 0 => depth = 1,
            ']' | ')' | '*' if depth == 1 => depth = 0,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    if !out.chars().any(char::is_alphanumeric) {
        return String::new();
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn threads() -> i32 {
    thread::available_parallelism().map_or(4, |n| n.get().min(12) as i32)
}

/// `iris --transcribe audio.wav`: tests transcription without the UI.
pub fn transcribe_file(path: &str) -> Result<()> {
    let out = std::process::Command::new("ffmpeg")
        .args(["-loglevel", "error", "-i", path, "-f", "f32le", "-ac", "1", "-ar", "16000", "-"])
        .output()?;
    let audio: Vec<f32> = out
        .stdout
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let audio = normalize(audio); // same as a recording from the UI
    let t0 = std::time::Instant::now();
    let (ctx, name) = load_model()?;
    let load = t0.elapsed();
    transcribe(&ctx, &audio)?; // warm-up: the first pass compiles shaders
    let t1 = std::time::Instant::now();
    let text = transcribe(&ctx, &audio)?;
    println!(
        "{name}: carga {:.1}s, {:.1}s de audio en {:.1}s\n{text}{}",
        load.as_secs_f32(),
        audio.len() as f32 / WHISPER_RATE as f32,
        t1.elapsed().as_secs_f32(),
        if is_hallucination(&text) { "  (alucinación: Iris la descartaría)" } else { "" }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_quiet_voice_without_clipping() {
        // Quiet voice (peak 0.1) between silences, like a real recording, with a stray pop.
        let mut a: Vec<f32> =
            (0..16000).map(|i| if (4000..12000).contains(&i) { 0.1 * (i as f32 * 0.05).sin() } else { 0.0005 }).collect();
        a[10] = 0.8;
        let out = normalize(a);
        let peak = out.iter().skip(100).fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.8 && peak <= 1.0, "{peak}");
        // Silence stays as it is.
        assert_eq!(normalize(vec![0.001; 1000]), vec![0.001; 1000]);
        // With loud background noise it isn't raised: amplified, whisper makes up phrases.
        let noisy: Vec<f32> = (0..16000)
            .map(|i| 0.04 * ((i * 7919 % 101) as f32 / 50.0 - 1.0) + if i > 8000 { 0.15 * (i as f32 * 0.05).sin() } else { 0.0 })
            .collect();
        let out = normalize(noisy.clone());
        let g = out[100] / noisy[100];
        assert!(g < 1.3, "subió el ruido ×{g}");
        // What already comes in loud is left untouched.
        let loud: Vec<f32> = (0..1000).map(|i| 0.95 * (i as f32 * 0.1).sin()).collect();
        assert_eq!(normalize(loud.clone()), loud);
    }

    #[test]
    fn hallucinations() {
        for t in ["¡Gracias!", "Gracias.", "¡Hasta la vuelta! ¡Hasta la vuelta!", "Gracias por ver el video."] {
            assert!(is_hallucination(t), "{t}");
        }
        for t in ["lista los archivos", "gracias, ahora corre los tests", "hasta la vuelta del bucle suma uno"] {
            assert!(!is_hallucination(t), "{t}");
        }
    }

    #[test]
    fn strips_marks() {
        assert_eq!(clean(" [BLANK_AUDIO] "), "");
        assert_eq!(clean(" (música) hola mundo"), "hola mundo");
        assert_eq!(clean("."), "");
    }
}
