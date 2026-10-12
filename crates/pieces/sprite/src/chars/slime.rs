//! Slime (Dragon Quest): a drop of jelly with its little point on top and its wide open smile,
//! drawn from a row-by-row profile every frame, so it squashes and stretches like jelly. What it
//! does:
//! - waiting: breathing jelly, its point swaying, a wobble now and then; music: bounces to the
//!   groove, its point whipping side to side;
//! - listening: stretched tall, eyes wide; asking: its point curls into a hook, head tilted;
//! - thinking: slow side-to-side sway, eyes up, bubbles rising inside it;
//! - speaking: the smile with the voice, the jelly pulsing with every syllable;
//! - reading, searching, on the web: it absorbs bits of text flying into it, which dissolve in
//!   its body;
//! - running, testing: hopping hard in place (squash on landing, stretch in the air, splashes);
//!   editing: kneading itself from side to side; git: a bud of jelly grows out and goes back;
//! - done: LEVEL UP! — golden letters and sparkles, a crown falls on it: it's the King Slime;
//! - error: it melts into a puddle with x x, splashes drops, and pulls itself back together;
//! - a key: a jiggle; copying: mini slimes pop out of it and hop away; merge: one hops in;
//! - asleep: flat and soft, a nose bubble growing and popping; offline: a pale puddle;
//! - booting: falls from the sky and splats; little context left: droopy, sweating.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Slime";
pub const SCENE: &str = "meadow";
pub const DESCRIPTION: &str = "El Slime de Dragon Quest en pixel art: rebota como gelatina al trabajar, sube de nivel y se corona Rey Slime al terminar, se derrite en un charco con los errores y se multiplica al copiar";

const LINE: u8 = 1;
const BODY: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const WHITE: u8 = 5;
const EYE: u8 = 6;
const MOUTH: u8 = 7;
const TONGUE: u8 = 8;
const GOLD: u8 = 9;
const GOLDSH: u8 = 10;
const SPARK: u8 = 11;
const PALE: u8 = 12;
const PALESH: u8 = 13;
const DEEP: u8 = 14;
const GEM: u8 = 15;

const CX: f64 = 19.0;
const BOTTOM: f64 = 37.0;
const HEIGHT: f64 = 29.0;
const HALF: f64 = 13.5;

/// The jelly this frame: its height and half width, where its point leans and how it curls.
#[derive(Clone, Copy)]
struct Shape {
    h: f64,
    w: f64,
    tip: f64,
    curl: f64,
    /// How much of a puddle it is (0: none, 1: all melted).
    melt: f64,
}

impl Shape {
    fn top(&self) -> f64 {
        BOTTOM - self.h
    }
}

/// The width of the drop at `u` (0 at the point, 1 at the bottom), from 0 to 1.
fn profile(u: f64) -> f64 {
    let a = 0.4;
    if u < a {
        0.6 * (u / a).powf(1.5)
    } else {
        let v = (u - a) / (1.0 - a);
        let (vc, r) = (0.571, 0.714);
        let k = (v - vc) / r;
        let w = (1.0 - k * k).max(0.0).sqrt();
        if u > 0.95 { w * 0.94 } else { w }
    }
}

/// A damped wobble starting at `at`: how far the jelly is pushed, -1..1.
fn jiggle(f: &Frame, at: f64, decay: f64, speed: f64) -> f64 {
    let d = f.since(at);
    if !(0.0..3.0).contains(&d) {
        return 0.0;
    }
    (-d * decay).exp() * (d * speed).sin()
}

/// Back out with a bounce: 0..1 → 0..1, overshooting a little.
fn elastic(u: f64) -> f64 {
    if u <= 0.0 {
        return 0.0;
    }
    if u >= 1.0 {
        return 1.0;
    }
    1.0 - (-6.0 * u).exp() * (u * 14.0).cos()
}


// DEBUG-REMAP-START
const DEBUG: u8 = 0;
fn remap(f: &Frame) -> Frame {
    let mut times = f.times;
    let (mut state, mut st, mut groove, mut context) = (f.state, f.st, f.groove, f.context);
    if DEBUG == 1 {
        state = match f.state {
            State::Idle if f.within(f.times.done, 3.0) => { times.copy = times.done; times.done = -99.0; State::Idle }
            State::Idle if f.within(f.times.error, 3.0) => { times.error = -99.0; context = 0.95; State::Idle }
            State::Idle => State::Listening,
            State::Thinking => State::Asking,
            State::Speaking => State::Git,
            State::Reading => State::Delegating,
            State::Running => State::Offline,
            State::Sleeping => { st = 1.15; State::Booting }
            s => s,
        };
    } else if DEBUG == 2 {
        state = match f.state {
            State::Idle if f.within(f.times.done, 3.0) => { times.merge = times.done - 0.35; times.done = -99.0; State::Idle }
            State::Idle if f.within(f.times.error, 3.0) => { times.error -= 0.6; State::Idle }
            State::Idle => { groove = 1.0; State::Idle }
            State::Thinking => State::Editing,
            State::Speaking => State::Compacting,
            State::Reading => State::Web,
            State::Running => { st = 0.5; State::Booting }
            State::Sleeping => { times.error -= 0.0; groove = 1.0; State::Thinking }
            s => s,
        };
    }
    Frame { t: f.t, dt: f.dt, state, st, voice: f.voice, music: f.music, groove, beat: f.beat, gaze: f.gaze, blink: f.blink, yawn: f.yawn, red: f.red, context, times, seed: f.seed }
}
// DEBUG-REMAP-END

pub struct Char {
    next_wobble: f64,
    wobble: f64,
}

impl Char {
    fn running(f: &Frame) -> bool {
        f.is(&[State::Running, State::Testing])
    }

    /// Where it is in its hop (0 on the ground, 1 at the top), running.
    fn hop(f: &Frame) -> f64 {
        (PI * ((f.t * 2.6) % 1.0)).sin()
    }

    fn shape(&self, f: &Frame) -> Shape {
        let t = f.t;
        let (mut hf, mut wf) = (1.0, 1.0);
        let mut tip = f.wave(1.3, 0.0) * 1.2;
        let mut curl = 0.35 + 0.15 * f.wave(0.9, 1.0);
        let mut melt: f64 = 0.0;

        // Idle wobbles, keys and music.
        let wob = jiggle(f, self.wobble, 3.0, 16.0) * 0.08 + jiggle(f, f.times.key, 5.0, 22.0) * 0.06;
        hf += wob;
        tip += wob * 25.0;

        if f.groove > 0.1 {
            let b = (t * PI * 2.0 * 0.55 * 2.0).sin();
            hf -= 0.1 * f.groove * b.abs();
            tip += (t * PI * 2.0 * 0.55).sin() * 4.0 * f.groove;
            curl = 0.8 * (t * PI * 2.0 * 0.55).sin();
        }

        match f.state {
            State::Listening | State::Transcribing => {
                hf *= 1.08;
                tip *= 0.4;
            }
            State::Thinking | State::Planning => {
                tip = f.wave(1.6, 0.0) * 3.0;
                curl = 0.6 * f.wave(1.6, 0.6);
            }
            State::Speaking => {
                hf *= 1.0 + f.voice * 0.9 * (0.6 + 0.4 * f.wave(17.0, 0.0));
            }
            State::Asking => {
                curl = 1.0;
                tip = 2.0;
            }
            State::Editing => {
                tip = f.wave(5.0, 0.0) * 3.5;
                hf *= 1.0 - 0.05 * f.wave(10.0, 0.0).abs();
            }
            State::Running | State::Testing => {
                let g = Self::hop(f);
                hf *= 0.72 + 1.3 * g - 1.0 * g * g;
                tip *= 0.5;
                curl = 0.2 - g * 0.6;
            }
            State::Sleeping => {
                hf *= 0.8 + 0.03 * (t * 0.9).sin();
                wf *= 1.1;
                tip = 2.5;
                curl = 0.9;
            }
            State::Offline => {
                melt = 0.6;
            }
            State::Booting => {
                if f.st < 1.0 {
                    hf *= 1.2;
                    wf *= 0.85;
                } else {
                    hf *= 0.68 + 0.32 * elastic(((f.st - 1.0) / 0.7).min(1.0));
                }
            }
            _ => {}
        }
        if f.reading() {
            hf *= 1.0 + 0.04 * f.wave(6.0, 0.0);
        }
        if f.context > 0.85 && !f.asleep() {
            hf *= 0.9;
            wf *= 1.05;
            curl = -0.5;
        }

        // An error: it melts into a puddle and pulls itself back up.
        let since = f.since(f.times.error);
        if (0.0..1.8).contains(&since) {
            let m = if since < 0.4 {
                1.0 - (1.0 - since / 0.4).powi(2)
            } else if since < 0.95 {
                1.0
            } else {
                1.0 - elastic((since - 0.95) / 0.85)
            };
            melt = melt.max(m);
        }
        hf *= 1.0 - 0.72 * melt;
        wf *= 1.0 + 0.3 * melt;

        // Done: it grows into the King Slime.
        if let Some(u) = f.progress(f.times.done, 2.8) {
            let k = elastic((u * 4.0).min(1.0)) * if u > 0.85 { (1.0 - u) / 0.15 } else { 1.0 };
            hf *= 1.0 + 0.12 * k;
            wf *= 1.0 + 0.08 * k;
        }
        // Copying: it bulges before the minis pop out.
        if let Some(u) = f.progress(f.times.copy, 0.5) {
            let k = (u * PI).sin();
            hf *= 1.0 - 0.15 * k;
            wf *= 1.0 + 0.12 * k;
        }

        let wf = wf / hf.max(0.3).powf(0.45);
        Shape { h: (HEIGHT * hf).clamp(5.0, 35.0), w: (HALF * wf).clamp(6.0, 18.5), tip: tip.clamp(-6.0, 6.0), curl: curl.clamp(-1.0, 1.0), melt }
    }
}

/// The drop, row by row.
fn jelly(c: &mut Canvas, s: &Shape) {
    let top = s.top();
    let y0 = top.floor() as i32;
    for y in y0..BOTTOM as i32 {
        let u = (y as f64 + 0.5 - top) / s.h;
        if u < 0.0 {
            continue;
        }
        let w = (profile(u) * s.w).max(0.7);
        let mid = CX + s.tip * (1.0 - u).powf(2.5);
        for x in 0..c.w as i32 {
            if (x as f64 + 0.5 - mid).abs() <= w {
                c.set(x, y, BODY);
            }
        }
    }
    // The point's little curl.
    let tx = CX + s.tip;
    let k = s.curl;
    if s.melt < 0.5 {
        let a = (tx, top + 1.0);
        let b = (tx + 2.2 * k, top - 1.0 - 0.6 * k.abs());
        c.thick(a, b, 1.6, BODY);
        if k.abs() > 0.6 {
            let d = (tx + 3.8 * k.signum(), top + 0.6);
            c.thick(b, d, 1.4, BODY);
        }
    }
}

/// The DQ smile: flat on top, round below; `h` rows deep, `hw` half wide.
fn smile_open(c: &mut Canvas, cx: f64, y: i32, hw: f64, h: i32) {
    for k in 0..h {
        let v = (k as f64 + 0.5) / h as f64;
        let w = hw * (1.0 - v * v).max(0.0).sqrt();
        let col = if h >= 3 && k == h - 1 { TONGUE } else if h >= 4 && k == h - 2 && v > 0.5 { TONGUE } else { MOUTH };
        for x in (cx - w - 0.5).round() as i32..=(cx + w - 0.5).round() as i32 {
            c.set(x, y + k, col);
        }
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#6a5cf0", "#5cb4ff", "#3e8ef0", "#bce8ff", "#ffffff", "#16123a", "#d22c4a", "#ff8fa6", "#ffd23a", "#e08a12", "#fff6b0", "#c3cfe4",
        "#8e9cc0", "#3577e6", "#ff4060",
    ];
    const W: usize = 38;
    const H: usize = 38;

    fn new(seed: u64) -> Self {
        Char { next_wobble: 2.0 + hash(seed) * 2.0, wobble: -99.0 }
    }

    fn step(&mut self, f: &Frame) {
        let f = &remap(f);
        if f.t > self.next_wobble {
            self.next_wobble = f.t + 3.0 + f.rand(f.t as u64) * 4.0;
            if f.state == State::Idle {
                self.wobble = f.t;
            }
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let f = &remap(f);
        let s = self.shape(f);
        let top = s.top();
        jelly(c, &s);

        // Git: a bud of jelly grows out of its side and is taken back.
        if f.state == State::Git {
            let u = (f.st % 3.0) / 3.0;
            let g = (u * PI).sin();
            if g > 0.1 {
                let bx = CX + s.w * 0.7 + g * 6.0;
                c.thick((CX + s.w * 0.5, BOTTOM - 4.0), (bx, BOTTOM - 3.0), 2.0 + 2.0 * (1.0 - g), BODY);
                c.ellipse(bx, BOTTOM - 3.0, 3.0 * g + 1.0, 2.6 * g + 0.8, BODY);
            }
        }

        c.outline(LINE, &[]);
        // Deep shading on the right and bottom, then the rim light.
        let old = c.px;
        let at = |x: i32, y: i32| if x < 0 || y < 0 || x >= 48 || y >= 48 { 0 } else { old[y as usize * 48 + x as usize] };
        for y in 0..c.h as i32 {
            for x in 0..c.w as i32 {
                if at(x, y) == BODY && (at(x + 2, y) == LINE || at(x + 3, y) == LINE || at(x, y + 2) == LINE || at(x + 2, y + 2) == LINE) {
                    c.set(x, y, SHADE);
                }
            }
        }
        c.shade(BODY, DEEP, LINE);
        c.shade(SHADE, DEEP, LINE);
        c.light(BODY, LIGHT, LINE);

        // The gloss on its upper left.
        if s.melt < 0.7 {
            let gy = top + s.h * 0.52;
            let gx = CX - s.w * 0.62 + s.tip * 0.2;
            c.ellipse(gx, gy, 1.4, 2.2, LIGHT);
            c.ellipse(gx - 0.3, gy - 0.5, 0.8, 1.3, WHITE);
            c.set((CX + s.tip * 0.5 - 1.5) as i32, (top + s.h * 0.28) as i32, LIGHT);
        }

        // Reading: bits dissolving inside it, sinking.
        if f.reading() {
            for k in 0..5u64 {
                let ph = (f.t * 0.5 + k as f64 * 0.2) % 1.0;
                let x = CX - 6.0 + f.rand(k) * 12.0 + (f.t * 3.0 + k as f64).sin();
                let y = top + s.h * (0.45 + 0.45 * ph);
                if ph < 0.85 {
                    c.set(x as i32, y as i32, if ph < 0.4 { WHITE } else { LIGHT });
                }
            }
        }
        // Thinking: bubbles rising inside.
        if f.thinking() {
            for k in 0..3u64 {
                let ph = (f.t * 0.6 + k as f64 / 3.0) % 1.0;
                let x = CX + 5.0 - k as f64 * 3.0 + (f.t * 4.0 + k as f64).sin() * 0.8;
                let y = top + s.h * (0.95 - 0.6 * ph);
                c.set(x as i32, y as i32, LIGHT);
            }
        }

        // The face: on the puddle it stays on top of it.
        let melted = s.melt > 0.5;
        let ey = if melted { (top + s.h * 0.35).round() as i32 } else { (top + s.h * 0.56).round() as i32 - 2 };
        let ex = s.w * 0.33;
        let dx = (s.tip * 0.25).round();
        let (lx, rx) = ((CX - ex + dx - 1.5).round() as i32, (CX + ex + dx - 1.5).round() as i32);
        let mut eyes = f.eyes();
        if f.thinking() {
            eyes = if f.blink { Eyes::Shut } else { Eyes::Open };
        }
        if f.state == State::Offline {
            eyes = Eyes::Shut;
        }
        let look = f.look();
        let tired = f.context > 0.85 && !f.asleep();
        if melted && f.within(f.times.error, 1.8) {
            c.eye(lx, ey, 3, 3, Eyes::Cross, look, EYE, 0, 0, false);
            c.eye(rx, ey, 3, 3, Eyes::Cross, look, EYE, 0, 0, true);
        } else if matches!(eyes, Eyes::Open | Eyes::Wide) {
            let tall = if eyes == Eyes::Wide { 3.3 } else { 2.8 };
            for (x, right) in [(lx, false), (rx, true)] {
                let (cx, cy) = (x as f64 + 1.5 + look.0 as f64 * 0.7, ey as f64 + 2.0 + look.1 as f64 * 0.7);
                c.ellipse(cx, cy, 1.7, tall, EYE);
                c.set(cx as i32 - 1, (cy - tall + 1.4) as i32, WHITE);
                c.set(cx as i32 - 1, (cy - tall + 2.4) as i32, WHITE);
                if !right || tall > 2.5 {
                    c.set(cx as i32, (cy + tall - 1.2) as i32, LIGHT);
                }
                if tired {
                    // Heavy lids.
                    c.rect(cx as i32 - 2, (cy - tall) as i32, 4, 2, BODY);
                    c.line(cx as i32 - 2, (cy - tall) as i32 + 2, cx as i32 + 1, (cy - tall) as i32 + 2, LINE);
                }
            }
        } else {
            c.eye(lx, ey + 1, 3, 3, eyes, look, EYE, WHITE, 0, false);
            c.eye(rx, ey + 1, 3, 3, eyes, look, EYE, WHITE, 0, true);
        }

        // The mouth.
        let my = ey + if melted { 3 } else { 6 };
        let mx = CX + dx;
        let hw = (s.w * 0.42).clamp(3.0, 6.5);
        if f.hurt() || melted {
            // A wobbly ~ line.
            for k in -3..=3 {
                c.set(mx as i32 + k, my + if k.rem_euclid(2) == 0 { 0 } else { 1 }, MOUTH);
            }
        } else if f.asleep() {
            c.smile(mx as i32, my, 3, MOUTH);
        } else if f.state == State::Speaking || f.is(&[State::Listening, State::Transcribing]) || f.reading() {
            let open = f.mouth();
            smile_open(c, mx, my, hw * (0.75 + 0.25 * open), 1 + (open * 4.0).round() as i32);
        } else if f.happy() {
            smile_open(c, mx, my, hw + 0.5, 5);
        } else if tired {
            smile_open(c, mx, my, hw * 0.6, 2);
        } else if f.state == State::Asking {
            smile_open(c, mx, my, hw * 0.55, 3);
        } else {
            smile_open(c, mx, my, hw, 4 + if f.groove > 0.1 { 1 } else { 0 });
        }

        if f.state == State::Offline {
            for col in [BODY, SHADE, LIGHT, DEEP] {
                c.swap(col, PALE);
            }
            c.swap(LINE, PALESH);
            c.swap(MOUTH, PALESH);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        let f = &remap(f);
        if Self::running(f) {
            m.dy -= (f.t * 12.0).sin().abs() * 0.07;
            let g = Self::hop(f);
            m.dy += 0.28 * g;
        }
        if f.state == State::Asking {
            m.lean += 0.05;
        }
        // Music: hops on the beat.
        if f.groove > 0.1 {
            m.dy += 0.08 * f.groove * (f.t * PI * 2.0 * 0.55 * 2.0).sin().abs();
        }
        // Melted, it doesn't shake: it spreads.
        if f.within(f.times.error, 1.8) {
            m.dx *= 0.3;
        }
        // Booting: it falls stretched, and the drop ends in a splat (drawn).
        if f.state == State::Booting && f.st > 1.0 {
            m.dy *= 0.5;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let f = &remap(f);
        let t = f.t;
        let s = self.shape(f);
        let top = s.top();
        const MINI: [&str; 7] = ["...a...", "..aba..", ".abbba.", "abwbwba", "abebeba", "abbmbba", ".aaaaa."];
        let draw_mini = |w: &mut World, x: f64, y: f64, dim: bool| {
            for (sy, row) in MINI.iter().enumerate() {
                for (sx, ch) in row.chars().enumerate() {
                    let col = match ch {
                        'a' => LINE,
                        'b' => BODY,
                        'w' => LIGHT,
                        'e' => EYE,
                        'm' => MOUTH,
                        _ => 0,
                    };
                    let (px, py) = (x + sx as f64 - 3.0, y + sy as f64 - 7.0);
                    if dim { w.px_dim(px, py, col) } else { w.px(px, py, col) }
                }
            }
        };

        // A sparkle: a plus with a bright middle.
        let twinkle = |w: &mut World, x: f64, y: f64, big: bool| {
            w.px(x, y, WHITE);
            w.px(x - 1.0, y, SPARK);
            w.px(x + 1.0, y, SPARK);
            w.px(x, y - 1.0, SPARK);
            w.px(x, y + 1.0, SPARK);
            if big {
                w.px(x - 2.0, y, GOLD);
                w.px(x + 2.0, y, GOLD);
                w.px(x, y - 2.0, GOLD);
                w.px(x, y + 2.0, GOLD);
            }
        };

        // Done: LEVEL UP! and the crown of the King Slime.
        if let Some(u) = f.progress(f.times.done, 2.8) {
            let d = u * 2.8;
            // The letters, rising a little and flashing.
            let glyphs: [(u16, u8); 9] = [
                (0b100_100_100_100_111, 3),
                (0b111_100_110_100_111, 3),
                (0b101_101_101_101_010, 3),
                (0b111_100_110_100_111, 3),
                (0b100_100_100_100_111, 3),
                (0, 1),
                (0b101_101_101_101_111, 3),
                (0b110_101_110_100_100, 3),
                (0b010_010_010_000_010, 3),
            ];
            let rise = (d * 3.0).min(1.0);
            let ty = top - 19.0 - rise * 2.0;
            let mut x = CX - 17.0;
            let col = if (d * 8.0) as i32 % 2 == 0 { GOLD } else { SPARK };
            for (k, (bits, wid)) in glyphs.iter().enumerate() {
                if d > 0.02 * k as f64 {
                    for row in 0..5 {
                        for colx in 0..3 {
                            if bits >> ((4 - row) * 3 + (2 - colx)) & 1 == 1 {
                                w.px(x + colx as f64, ty + row as f64, col);
                            }
                        }
                    }
                }
                x += *wid as f64 + 1.0;
            }
            // The crown falls on its point.
            let fall = (d / 0.35).min(1.0);
            let cxr = CX + s.tip;
            let cy = top - 4.5 - (1.0 - fall * fall) * 12.0;
            const CROWN: [&str; 6] = ["g...g...g", "gg.gwg.gg", "ggggggggg", "grgggggrg", "ggggggggg", "sssssssss"];
            for (sy, row) in CROWN.iter().enumerate() {
                for (sx, ch) in row.chars().enumerate() {
                    let col = match ch {
                        'g' => GOLD,
                        's' => GOLDSH,
                        'r' => GEM,
                        'w' => SPARK,
                        _ => 0,
                    };
                    if col > 0 {
                        w.px(cxr - 4.5 + sx as f64, cy + sy as f64, col);
                    }
                }
            }
            // Sparkles bursting out, then twinkling round it.
            for k in 0..8 {
                let a = k as f64 * PI / 4.0 + d * 0.8;
                let r = 12.0 + d.min(0.6) * 18.0;
                if (t * 7.0 + k as f64).sin() > -0.2 {
                    twinkle(w, CX + a.cos() * r, top + s.h * 0.5 + a.sin() * r * 0.75, k % 3 == 0);
                }
            }
        }

        // Error: drops splashed out, falling back.
        if let Some(u) = f.progress(f.times.error, 1.2) {
            for k in 0..6 {
                let dir = if k % 2 == 0 { -1.0 } else { 1.0 };
                let sp = 6.0 + (k / 2) as f64 * 5.0;
                let x = CX + dir * (s.w * 0.6 + u * sp * 1.6);
                let y = BOTTOM - 4.0 - (u * PI).sin() * (6.0 + (k / 2) as f64 * 3.0) + u * 4.0;
                w.px(x, y, BODY);
                w.px(x, y - 1.0, LIGHT);
            }
        }

        // Running: splashes when it lands.
        if Self::running(f) {
            let p = (t * 2.6) % 1.0;
            if p < 0.3 {
                let u = p / 0.3;
                for k in 0..4 {
                    let dir = if k % 2 == 0 { -1.0 } else { 1.0 };
                    let x = CX + dir * (s.w + 1.0 + u * (4.0 + (k / 2) as f64 * 3.0));
                    let y = BOTTOM - 1.0 - (u * PI).sin() * (2.0 + (k / 2) as f64 * 2.0);
                    w.px(x, y, LIGHT);
                }
            }
        }

        // Reading, searching, on the web: bits of text flying into it.
        if f.reading() {
            for k in 0..6u64 {
                let ph = (t * 0.7 + k as f64 / 6.0) % 1.0;
                let a = k as f64 * PI / 3.0 + 0.4 + t * 0.3;
                let r = 24.0 * (1.0 - ph) + s.w * 0.6;
                let (x, y) = (CX + a.cos() * r, top + s.h * 0.55 + a.sin() * r * 0.7);
                let col = if k % 2 == 0 { WHITE } else { SPARK };
                w.px(x, y, col);
                if ph < 0.75 {
                    // A tiny glyph: two bars of text.
                    w.px(x + 1.0, y, col);
                    w.px(x + 2.0, y, col);
                    w.px(x, y + 2.0, LIGHT);
                    w.px(x + 1.0, y + 2.0, LIGHT);
                }
            }
        }

        // Copying (or more helpers): mini slimes pop out and hop away.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let d = u * 2.6;
            if d > 0.35 {
                for k in 0..3 {
                    let dd = d - 0.35 - k as f64 * 0.15;
                    if dd < 0.0 {
                        continue;
                    }
                    let dir = [-1.0, 1.0, -1.0][k];
                    let reach = [14.0, 15.0, 23.0][k];
                    let x = CX + dir * (s.w * 0.4 + (dd / 1.2).min(1.0) * reach);
                    let hop = (dd * PI * 2.2).sin().abs() * (5.0 - (dd * 2.0).min(4.0));
                    draw_mini(w, x, BOTTOM - hop, u > 0.85);
                }
            }
        }
        // Merge: one hops in from the side and goes into it.
        if let Some(u) = f.progress(f.times.merge, 1.2) {
            let x = CX - 28.0 + u * (28.0 - s.w * 0.5);
            let hop = (u * PI * 3.0).sin().abs() * 5.0;
            draw_mini(w, x, BOTTOM - hop, false);
        }
        // Delegating: a messenger slime hopping off to the side, again and again.
        if f.state == State::Delegating {
            let u = (f.st % 2.5) / 2.5;
            let x = CX + s.w * 0.6 + u * 18.0;
            let hop = (u * PI * 4.0).sin().abs() * 4.0;
            draw_mini(w, x, BOTTOM - hop, u > 0.75);
        }

        // Asleep: a nose bubble that grows and pops.
        if f.state == State::Sleeping {
            let ph = (t / 3.2) % 1.0;
            let (bx, by) = (CX + s.w * 0.55, top + s.h * 0.62);
            if ph < 0.85 {
                let r = 1.0 + ph / 0.85 * 3.5;
                let n = 14;
                for k in 0..n {
                    let a = k as f64 * PI * 2.0 / n as f64;
                    w.px(bx + r + a.cos() * r, by - r * 0.6 + a.sin() * r, if k == 9 { WHITE } else { LIGHT });
                }
            } else {
                let r = 6.0 + (ph - 0.85) * 30.0;
                for k in 0..6 {
                    let a = k as f64 * PI / 3.0;
                    w.px(bx + 4.0 + a.cos() * r, by - 2.0 + a.sin() * r, LIGHT);
                }
            }
        }

        // Booting: the splat when it lands.
        if f.state == State::Booting && f.st > 1.0 && f.st < 1.6 {
            let u = (f.st - 1.0) / 0.6;
            for k in 0..6 {
                let dir = if k % 2 == 0 { -1.0 } else { 1.0 };
                let x = CX + dir * (s.w + 1.0 + u * (3.0 + (k / 2) as f64 * 4.0));
                let y = BOTTOM - 1.0 - (u * PI).sin() * (3.0 + (k / 2) as f64 * 2.5);
                w.px(x, y, BODY);
            }
        }

        // Little context left: a sweat drop sliding down its side.
        if f.context > 0.85 && !f.asleep() {
            let ph = (t * 0.7) % 1.0;
            let (x, y) = (CX + s.w * 0.75, top + s.h * 0.3 + ph * 6.0);
            w.px(x, y, LIGHT);
            w.px(x, y + 1.0, WHITE);
            w.px(x - 1.0, y + 1.0, LIGHT);
        }
    }
}
