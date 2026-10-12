//! Lumi: a chubby firefly whose belly-lantern shines brighter the more Iris works. Drawn from
//! shapes every frame (wings, lantern, head with its little orange cap, antennae with glowing
//! tips). What it does:
//! - waiting: the lantern breathes softly, the antennae sway, the wings shiver now and then;
//! - listening: antennae straight up, the lantern follows the voice;
//! - thinking: a light hops from one antenna tip to the other, the lantern pulses slowly;
//! - speaking: the lantern pulses with the voice, the mouth too;
//! - reading, searching, web: it drinks the light: motes spiral into its lantern;
//! - working: the longer it works, the brighter it gets (it ramps up to white-hot); running and
//!   testing: it flies in place, wings a blur, leaving a trail of embers;
//! - git: lights run along a little branch beside it; a merge: two lights become one;
//! - done: it bursts into light, a halo that lights up everything around;
//! - error: the lantern blows out with a spark and a puff, then flickers back on;
//! - a key: a spark off an antenna; copying: other fireflies come to keep it company;
//! - asking: one antenna curls, a little arm up; asleep: wings folded, the light a slow
//!   heartbeat; offline: dark; booting: it flickers on like a tube light; little context left:
//!   a low-battery flicker and a sweat drop; music: it dances and flashes on the beat.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Lumi";
pub const SCENE: &str = "cave";
pub const DESCRIPTION: &str = "Lumi, una luciérnaga regordeta: su farolito brilla más cuanto más trabaja Iris, estalla en un halo de luz al terminar, se apaga con un chispazo en los errores y llama a sus amigas al copiar";

const LINE: u8 = 1;
const BODY: u8 = 2;
const BSHADE: u8 = 3;
const BLIGHT: u8 = 4;
const HOOD: u8 = 5;
const DIM: u8 = 6;
const GLOW2: u8 = 7;
const GLOW: u8 = 8;
const HOT: u8 = 9;
const EYE: u8 = 10;
const WHITE: u8 = 11;
const WING: u8 = 12;
const CHEEK: u8 = 13;
const MOUTH: u8 = 14;
const SPARK: u8 = 15;
const DROP: u8 = 16;

/// The lantern's colors, dark to white-hot.
const LADDER: [u8; 4] = [DIM, GLOW2, GLOW, HOT];

pub struct Char {
    /// The next wing shiver while waiting, and the last one.
    shiver: f64,
    next_shiver: f64,
}

impl Char {
    /// How bright the lantern is (0: off, 1: white-hot).
    fn level(&self, f: &Frame) -> f64 {
        let t = f.t;
        // An error: a spark, dark, then flickering back on.
        if let Some(u) = f.progress(f.times.error, 1.4) {
            return if u < 0.08 {
                1.0
            } else if u < 0.45 {
                0.0
            } else if f.rand((t * 14.0) as u64) < 0.25 + u * 0.6 {
                0.5
            } else {
                0.05
            };
        }
        if f.happy() {
            return 1.0;
        }
        let mut l = match f.state {
            State::Offline => 0.0,
            State::Sleeping => {
                // lub-dub, every 2.6 s.
                let p = (t % 2.6) / 2.6;
                let beat = (-((p - 0.1) * 30.0).powi(2)).exp() + 0.7 * (-((p - 0.22) * 30.0).powi(2)).exp();
                0.1 + 0.45 * beat
            }
            State::Booting => {
                if f.st > 1.4 {
                    0.6
                } else if f.rand((t * 12.0) as u64) < f.st / 1.4 {
                    0.7
                } else {
                    0.05
                }
            }
            State::Speaking => 0.4 + (f.voice * 5.0).min(0.6),
            State::Listening | State::Transcribing => 0.35 + (f.voice * 3.0).min(0.45),
            State::Thinking | State::Planning => 0.45 + 0.25 * f.wave(2.0, 0.0),
            State::Compacting => 0.85 + 0.15 * f.wave(6.0, 0.0),
            State::Testing => {
                // blink-blink, pause.
                let p = (t * 1.6) % 1.0;
                if p < 0.12 || (0.25..0.37).contains(&p) { 0.95 } else { 0.5 }
            }
            _ if f.working() || f.state == State::Delegating => 0.45 + 0.55 * (f.st / 8.0).min(1.0) + 0.05 * f.wave(17.0, 0.0),
            _ if f.reading() => 0.55 + 0.2 * f.wave(5.0, 0.0),
            State::Asking => 0.6 + 0.15 * f.wave(7.0, 0.0),
            _ => 0.45 + 0.08 * f.wave(1.5, 0.0),
        };
        if f.groove > 0.1 {
            l = l.max(0.35 + 0.65 * f.beat * f.groove);
        }
        if f.within(f.times.key, 0.15) {
            l += 0.3;
        }
        // Little context left: a tired battery that dips now and then.
        if f.context > 0.85 && !f.asleep() {
            l *= 0.6;
            if f.rand((t * 6.0) as u64 + 99) < 0.2 {
                l *= 0.2;
            }
        }
        l.clamp(0.0, 1.0)
    }
}

fn step_of(l: f64) -> usize {
    if l < 0.15 {
        0
    } else if l < 0.45 {
        1
    } else if l < 0.8 {
        2
    } else {
        3
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#7a6ee0", "#a9a6ff", "#8a84ee", "#d8d6ff", "#ff9a4a", "#4f7a3a", "#9bdc3a", "#dcff5a", "#fbffd8", "#140c24", "#ffffff",
        "#c8f2ff", "#ff86b4", "#8a2a50", "#fff07a", "#7fd0ff",
    ];
    const W: usize = 34;
    const H: usize = 37;

    fn new(seed: u64) -> Self {
        Char { shiver: -99.0, next_shiver: 2.0 + hash(seed) * 3.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.t > self.next_shiver {
            self.shiver = f.t;
            self.next_shiver = f.t + 3.0 + f.rand(f.t as u64 + 5) * 4.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let hurt = f.hurt();
        let happy = f.happy();
        let flying = f.is(&[State::Running, State::Testing]);
        let lvl = self.level(f);
        let s = step_of(lvl);

        // The wings, behind: tips sway; a blur flying; spread wide when done; folded asleep.
        let flap = if flying || happy {
            if ((t * 24.0) as i32) % 2 == 0 { 1.0 } else { -1.0 }
        } else if f.groove > 0.1 {
            f.beat * 2.0 - 1.0
        } else if let Some(u) = f.progress(self.shiver, 0.4) {
            ((u * 20.0) as i32 % 2) as f64 * 2.0 - 1.0
        } else {
            0.25 * f.wave(1.3, 0.0)
        };
        let (lw, rw) = if f.asleep() {
            ((6.0, 26.0), (28.0, 26.0))
        } else if happy || f.is(&[State::Listening, State::Transcribing]) {
            ((1.0, 8.0 + flap * 2.0), (33.0, 8.0 + flap * 2.0))
        } else {
            ((2.0, 14.0 + flap * 2.5), (32.0, 14.0 + flap * 2.5))
        };
        c.thick((12.0, 22.0), lw, 5.0, WING);
        c.thick((22.0, 22.0), rw, 5.0, WING);

        // The lantern (abdomen), its belly arms and feet.
        let fill = LADDER[s];
        let dangle = if flying { f.wave(10.0, 0.0) } else { 0.0 };
        c.ellipse(13.0, 35.0 + dangle.max(0.0), 2.4, 1.5, BODY);
        c.ellipse(21.0, 35.0 + (-dangle).max(0.0), 2.4, 1.5, BODY);
        c.ellipse(17.0, 28.5, 9.5, 7.0, fill);

        // Arms: hugging the lantern; up cheering or hurt; one waving when asking; tapping
        // while editing.
        let (mut la, mut ra) = (26.0, 26.0);
        if happy || hurt {
            la = 21.5;
            ra = 21.5;
        } else if f.state == State::Asking {
            ra = 20.5 + f.wave(9.0, 0.0);
        } else if f.state == State::Editing {
            la += f.wave(12.0, 0.0).max(0.0) * 1.5;
            ra += f.wave(12.0, PI).max(0.0) * 1.5;
        }
        c.ellipse(7.5, la, 1.6, 2.2, BODY);
        c.ellipse(26.5, ra, 1.6, 2.2, BODY);

        // The head and its orange cap.
        c.ellipse(17.0, 16.0, 9.5, 8.0, BODY);
        c.ellipse(17.0, 9.5, 7.0, 3.2, HOOD);

        c.outline(LINE, &[]);
        c.shade(BODY, BSHADE, LINE);
        c.light(BODY, BLIGHT, LINE);
        c.shade(fill, LADDER[s.saturating_sub(1)], LINE);
        // A vein down each wing, and a glint on it.
        for (b, tp) in [((11.0, 21.0), lw), ((23.0, 21.0), rw)] {
            let v = |k: f64| ((b.0 + (tp.0 - b.0) * k).round() as i32, (b.1 + (tp.1 - b.1) * k).round() as i32);
            let (p, q) = (v(0.35), v(0.85));
            c.line(p.0, p.1, q.0, q.1, BLIGHT);
            let g = v(0.7);
            if c.get(g.0 + 1, g.1) == WING {
                c.set(g.0 + 1, g.1, WHITE);
            }
        }
        // A brighter heart in the lantern, and white-hot at the top.
        if s >= 1 {
            c.ellipse(17.0, 30.5, 5.5, 3.5, LADDER[(s + 1).min(3)]);
        }
        if s >= 2 {
            c.ellipse(17.0, 31.0, 2.5, 1.6, HOT);
        }
        // Its segments: two curved bands across the lantern.
        let band = if s == 0 { BSHADE } else { LADDER[s - 1] };
        for y0 in [26.0, 33.0] {
            for x in 8..27 {
                let y = (y0 + ((x as f64 - 17.0).powi(2)) / 60.0 * if y0 > 28.0 { -1.0 } else { 1.0 }).round() as i32;
                if LADDER.contains(&c.get(x, y)) {
                    c.set(x, y, band);
                }
            }
        }
        // A highlight on the cap.
        c.set(13, 8, WHITE);
        c.set(14, 7, WHITE);

        // The antennae: from the cap, two thin stalks with glowing tips.
        let sw = 0.8 * f.wave(1.7, 0.0);
        let (mut lt, mut rt) = ((7.0 + sw, 1.0), (27.0 + sw, 1.0));
        let (mut lm, mut rm) = ((11.0, 3.0), (23.0, 3.0));
        if f.asleep() || f.within(f.times.nope, 0.8) {
            lt = (4.0, 9.0);
            rt = (30.0, 9.0);
            lm = (9.0, 5.0);
            rm = (25.0, 5.0);
        } else if f.is(&[State::Listening, State::Transcribing]) || happy {
            lt = (10.0, 1.0);
            rt = (24.0, 1.0);
            lm = (12.0, 3.5);
            rm = (22.0, 3.5);
        } else if flying {
            lt = (3.0, 5.0 + dangle * 0.5);
            rt = (31.0, 5.0 - dangle * 0.5);
            lm = (9.0, 4.0);
            rm = (25.0, 4.0);
        } else if f.groove > 0.1 {
            let b = f.wave(PI * 2.2, 0.0) * 2.0 * f.groove;
            lt.0 += b;
            rt.0 += b;
        }
        let tip = |c: &mut Canvas, base: (i32, i32), mid: (f64, f64), tp: (f64, f64), col: u8| {
            c.line(base.0, base.1, mid.0.round() as i32, mid.1.round() as i32, LINE);
            c.line(mid.0.round() as i32, mid.1.round() as i32, tp.0.round() as i32, tp.1.round() as i32, LINE);
            c.ellipse(tp.0 + 0.5, tp.1 + 0.5, 1.3, 1.3, col);
        };
        // The tips glow with the lantern; thinking, a light hops between them.
        let (mut lc, mut rc) = (LADDER[s.max(1)], LADDER[s.max(1)]);
        if f.thinking() {
            let left = f.wave(3.0, 0.0) > 0.0;
            lc = if left { HOT } else { DIM };
            rc = if left { DIM } else { HOT };
        }
        if hurt && f.progress(f.times.error, 0.6).is_some() {
            // Frazzled: zigzag stalks.
            for (b, d) in [((14, 7), -1), ((20, 7), 1)] {
                let mut p = b;
                for k in 0..4 {
                    let q = (p.0 + d * 2, p.1 - if k % 2 == 0 { 2 } else { 0 });
                    c.line(p.0, p.1, q.0, q.1, LINE);
                    p = q;
                }
                c.ellipse(p.0 as f64 + 0.5, p.1 as f64 + 0.5, 1.2, 1.2, if f.wave(30.0, 0.0) > 0.0 { SPARK } else { DIM });
            }
        } else if f.state == State::Asking {
            tip(c, (14, 7), lm, lt, lc);
            // One curls like a hook.
            for (x, y) in [(20, 7), (21, 5), (22, 3), (23, 2), (24, 1), (25, 1), (26, 1), (27, 2), (28, 3), (28, 4), (27, 5), (26, 6)] {
                c.set(x, y, LINE);
            }
            c.ellipse(26.5, 7.5, 1.2, 1.2, rc);
        } else {
            tip(c, (14, 7), lm, lt, lc);
            tip(c, (20, 7), rm, rt, rc);
        }

        // The face.
        let eyes = if f.thinking() { Eyes::Open } else { f.eyes() };
        let look = f.look();
        c.eye(11, 13, 3, 4, eyes, look, EYE, WHITE, BODY, false);
        c.eye(21, 13, 3, 4, eyes, look, EYE, WHITE, BODY, true);
        // Cheeks glow a little with the lantern.
        let cheek = if s == 3 { SPARK } else { CHEEK };
        c.rect(8, 18, 2, 1, cheek);
        c.rect(25, 18, 2, 1, cheek);
        let open = f.mouth();
        if happy {
            c.mouth(17, 18, 0.9, 5, 3, MOUTH, CHEEK);
        } else if open > 0.08 {
            c.mouth(17, 18, open, 4, 3, MOUTH, CHEEK);
        } else if f.state == State::Sleeping {
            c.set(17, 19, MOUTH);
        } else {
            c.smile(17, 18, 3, MOUTH);
        }

        if f.state == State::Offline {
            c.swap(HOOD, BSHADE);
            c.swap(WING, BLIGHT);
        }
        let _ = t;
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        let t = f.t;
        // Flying in place: it hovers up off the ground and bobs.
        if f.is(&[State::Running, State::Testing]) {
            m.dy += 0.14 + 0.04 * (t * 7.0).sin();
        } else if f.thinking() {
            m.dy += 0.03 * (t * 1.4).sin();
        }
        if f.state == State::Sleeping {
            m.sy *= 0.95;
            m.sx *= 1.03;
        }
        // The spark knocks it back a little.
        if let Some(u) = f.progress(f.times.error, 0.4) {
            m.dy += 0.06 * (1.0 - u);
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let lvl = self.level(f);
        let (cx, cy) = (17.0, 29.0);

        // Its own light on what's around it: a faint glow ring, wider the brighter it shines.
        if lvl > 0.62 && !f.happy() {
            let r = 11.0 + lvl * 4.0;
            for k in 0..12 {
                let a = k as f64 * PI / 6.0 + t * 0.3;
                if (t * 3.0 + k as f64 * 1.7).sin() > -0.2 {
                    w.px_dim(cx + a.cos() * r * 1.2, cy - 4.0 + a.sin() * r, if lvl > 0.85 { GLOW } else { GLOW2 });
                }
            }
        }

        // Reading: it drinks the light, motes spiralling into the lantern.
        if f.reading() {
            for k in 0..7 {
                let p = (t * 0.55 + k as f64 / 7.0) % 1.0;
                let r = 22.0 * (1.0 - p);
                let a = t * 1.8 + k as f64 * 2.4 + p * 3.0;
                let col = if p > 0.75 { HOT } else if p > 0.4 { GLOW } else { GLOW2 };
                w.px(cx + a.cos() * r, cy - 3.0 + a.sin() * r * 0.8, col);
            }
        }

        // Flying: a trail of embers drifting down behind it.
        if f.is(&[State::Running, State::Testing]) {
            for k in 0..6 {
                let p = (t * 1.2 + k as f64 / 6.0) % 1.0;
                let x = cx + (k as f64 * 3.7) % 7.0 - 3.5 + (p * 9.0 + k as f64).sin() * 1.5;
                let y = 36.0 + p * 7.0;
                if p < 0.5 { w.px(x, y, GLOW) } else { w.px_dim(x, y, GLOW2) }
            }
        }

        // Git: lights running along a little branch beside it; a merge, two lights become one.
        if f.state == State::Git {
            let path = |u: f64| -> (f64, f64) { (36.0 + u * 10.0, 29.0) };
            let fork = |u: f64| -> (f64, f64) {
                let x = 38.0 + u * 6.0;
                (x, 29.0 - (u * PI).sin() * 5.0)
            };
            for k in 0..6 {
                let u = k as f64 / 5.0;
                let (x, y) = path(u);
                w.px_dim(x, y, DIM);
                let (fx, fy) = fork(u);
                w.px_dim(fx, fy, DIM);
            }
            let u = (t * 0.7) % 1.0;
            let (x, y) = path(u);
            w.px(x, y, HOT);
            let (fx, fy) = fork(u);
            w.px(fx, fy, GLOW);
        }
        if let Some(u) = f.progress(f.times.merge, 1.0) {
            let d = 10.0 * (1.0 - u);
            if u < 0.75 {
                w.px(40.0 - d, 15.0 - d * 0.4, GLOW);
                w.px(40.0 + d, 15.0 - d * 0.4, GLOW2);
            } else {
                for k in 0..6 {
                    let a = k as f64 * PI / 3.0;
                    let r = (u - 0.75) * 16.0;
                    w.px(40.0 + a.cos() * r, 15.0 + a.sin() * r, HOT);
                }
            }
        }

        // Done: it bursts into light, a ring that lights up everything, rays, then a halo.
        if let Some(u) = f.progress(f.times.done, 1.6) {
            let r = 4.0 + u * 30.0;
            let col = if u < 0.3 { HOT } else if u < 0.6 { GLOW } else { GLOW2 };
            for k in 0..28 {
                let a = k as f64 * PI / 14.0;
                w.px(cx + a.cos() * r, cy - 8.0 + a.sin() * r * 0.9, col);
            }
            if u < 0.7 {
                for k in 0..8 {
                    let a = k as f64 * PI / 4.0 + PI / 8.0;
                    for j in 0..4 {
                        let rr = 13.0 + u * 8.0 + j as f64 * 1.5;
                        w.px(cx + a.cos() * rr, cy - 8.0 + a.sin() * rr, if j == 0 { HOT } else { SPARK });
                    }
                }
            }
            // The cave lit up: a soft halo that stays a while.
            for k in 0..20 {
                let a = k as f64 * PI / 10.0;
                w.px_dim(cx + a.cos() * 19.0, cy - 8.0 + a.sin() * 17.0, GLOW2);
            }
        }

        // Error: the lantern blows out with a spark and a little puff of smoke.
        if let Some(u) = f.progress(f.times.error, 1.2) {
            if u < 0.25 {
                for k in 0..5 {
                    let a = -PI / 2.0 + (k as f64 - 2.0) * 0.6;
                    let mut p = (cx, cy);
                    for j in 0..4 {
                        let jig = if (j + k) % 2 == 0 { 1.0 } else { -1.0 };
                        p = (p.0 + a.cos() * 2.5 + jig * 0.7, p.1 + a.sin() * 2.5);
                        w.px(p.0, p.1, if j == 0 { HOT } else { SPARK });
                    }
                }
            } else {
                let rise = (u - 0.25) * 14.0;
                for (dx, dy) in [(0.0, 0.0), (1.0, -1.0), (-1.0, -2.0), (0.0, -3.0), (2.0, -3.0)] {
                    w.px_dim(31.0 + dx + (rise * 0.8).sin(), 25.0 + dy - rise, BLIGHT);
                }
            }
        }

        // A key: a spark off an antenna tip.
        if f.within(f.times.key, 0.18) {
            let left = (f.times.key * 10.0) as u64 % 2 == 0;
            let (x, d) = if left { (8.0, -1.0) } else { (26.0, 1.0) };
            w.px(x + d, 0.0, SPARK);
            w.px(x + d * 2.0, -1.0, HOT);
            w.px(x + d * 2.0, 1.0, SPARK);
        }

        // Copying: its friends fly in from the dark and dance around it, blinking.
        if let Some(u) = f.progress(f.times.copy, 3.2) {
            for k in 0..4 {
                let side = if k % 2 == 0 { -1.0 } else { 1.0 };
                let a = t * (1.6 + k as f64 * 0.3) + k as f64 * PI / 2.0;
                let orbit = (cx + a.cos() * (19.0 + k as f64), 15.0 + a.sin() * 9.0 + k as f64 * 2.0);
                let start = (cx + side * 44.0, -10.0 + k as f64 * 12.0);
                let arrive = (u * 4.0 - k as f64 * 0.25).clamp(0.0, 1.0);
                let leave = ((u - 0.85) / 0.15).clamp(0.0, 1.0);
                let e = 1.0 - (1.0 - arrive).powi(2);
                let mut p = (start.0 + (orbit.0 - start.0) * e, start.1 + (orbit.1 - start.1) * e);
                p = (p.0 + (start.0 - p.0) * leave, p.1 + (start.1 - p.1) * leave);
                w.px(p.0, p.1, BODY);
                w.px(p.0, p.1 + 1.0, if (t * 6.0 + k as f64).sin() > -0.3 { HOT } else { GLOW2 });
                w.speck(p.0 - 0.6, p.1 - 0.6, WING);
                w.speck(p.0 + 0.6, p.1 - 0.6, WING);
            }
        }

        // Delegating: a little firefly flies off with the job, again and again.
        if f.state == State::Delegating {
            let p = (t * 0.5) % 1.0;
            let (x, y) = (cx + 6.0 + p * 30.0, 15.0 - p * 16.0 + (p * 18.0).sin() * 2.0);
            w.px(x, y, BODY);
            w.px(x, y + 1.0, if (t * 8.0).sin() > 0.0 { HOT } else { GLOW });
        }

        // Music: little flashes popping around it on the beat.
        if f.groove > 0.1 && f.beat > 0.6 {
            for k in 0..5 {
                let a = f.rand((t * 2.0) as u64 * 7 + k) * PI * 2.0;
                let r = 14.0 + f.rand((t * 2.0) as u64 * 11 + k) * 8.0;
                w.px(cx + a.cos() * r, cy - 8.0 + a.sin() * r, if k % 2 == 0 { GLOW } else { CHEEK });
            }
        }

        // Booting: specks of light gathering into it.
        if f.state == State::Booting && f.st < 1.4 {
            for k in 0..8 {
                let a = k as f64 * PI / 4.0;
                let r = 24.0 * (1.0 - f.st / 1.4);
                w.px(cx + a.cos() * r, cy - 6.0 + a.sin() * r, GLOW2);
            }
        }

        // Little context left: a sweat drop on its cap.
        if f.context > 0.85 && !f.asleep() {
            let y = 7.0 + ((t * 0.8) % 1.0) * 4.0;
            w.px(26.0, y, DROP);
            w.px(26.0, y + 1.0, DROP);
            w.px(25.0, y + 1.0, DROP);
        }
    }
}
