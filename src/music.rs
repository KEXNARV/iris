//! What's playing on the machine, so the core dances while idle. Same as the keyboard
//! (ghubd): `cava` over PipeWire in raw mode, 33 bands from 0 to 100 per line, and the level is
//! the larger of the bass average and the mids average.

use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// The same config ghubd uses: with `noise_reduction` at 77 (the default) the meter lagged
/// behind the beats.
const CAVA_CONF: &str = "[general]
bars = 33
framerate = 60
[input]
method = pipewire
source = auto
[output]
method = raw
channels = mono
raw_target = /dev/stdout
data_format = ascii
ascii_max_range = 100
bar_delimiter = 59
frame_delimiter = 10
[smoothing]
noise_reduction = 20
";

/// cava bands (33 of them, logarithmic from 50 Hz to 10 kHz): bass 0-4, mids 5-15.
const BASS: std::ops::Range<usize> = 0..5;
const MID: std::ops::Range<usize> = 5..16;
/// The keyboard's `bar_gain`, so both hit red on the same beats.
const GAIN: f32 = 1.5;
/// If cava dies (at login it may start before PipeWire), it's relaunched.
const RESPAWN: Duration = Duration::from_secs(2);

#[derive(Clone, Default)]
pub struct Music {
    levels: Arc<Mutex<Vec<f32>>>,
}

impl Music {
    /// Launches cava in the background. Without cava nothing happens: the level stays at zero.
    pub fn spawn() -> Self {
        let m = Music::default();
        let levels = m.levels.clone();
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let conf = dir.join("iris-cava.conf");
        if std::fs::write(&conf, CAVA_CONF).is_err() {
            return m;
        }
        thread::spawn(move || loop {
            let child = Command::new("cava")
                .arg("-p")
                .arg(&conf)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();
            let Ok(mut child) = child else { return }; // not installed
            if let Some(out) = child.stdout.take() {
                for line in std::io::BufReader::new(out).lines() {
                    let Ok(line) = line else { break };
                    let v = line.split(';').filter_map(|s| s.parse::<f32>().ok()).map(|x| x / 100.0).collect();
                    *levels.lock().unwrap() = v;
                }
            }
            let _ = child.wait();
            levels.lock().unwrap().clear();
            thread::sleep(RESPAWN);
        });
        m
    }

    /// Level 0..1 (may go slightly over due to the gain), computed like the keyboard does.
    pub fn level(&self) -> f32 {
        let l = self.levels.lock().unwrap();
        (mean(&l, BASS).max(mean(&l, MID)) * GAIN).min(1.2)
    }
}

fn mean(l: &[f32], r: std::ops::Range<usize>) -> f32 {
    let r = r.start.min(l.len())..r.end.min(l.len());
    if r.is_empty() { 0.0 } else { l[r.clone()].iter().sum::<f32>() / r.len() as f32 }
}
