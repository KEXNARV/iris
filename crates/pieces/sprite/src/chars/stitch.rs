//! Stitch (Experimento 626): drawn from shapes every frame (huge ears, wide head, big glossy
//! eyes, a fat nose, a small body), so everything on it moves. What it does:
//! - waiting: ears twitching and swaying, a blink; now and then a mischievous giggle, claws at
//!   its mouth, shoulders shaking; music: Elvis — a pompadour and arms pumping;
//! - listening: ears straight up, eyes wide, sound rings reaching the ear tips;
//! - asking: its head tilts, one ear down, a claw at its chin;
//! - thinking, planning: a claw scratching its head, eyes up, its little red cruiser circling;
//! - speaking: a big toothy mouth with the voice, ears flapping;
//! - reading: "El patito feo" held up in its claws; searching, web: alien mode, antennae out,
//!   radar rings;
//! - editing, running, testing, git: alien mode — two extra arms and antennae out, all four
//!   arms busy; running in place;
//! - done: tongue out, arms up, a burst of hibiscus flowers;
//! - error: its ears fall, huge puppy eyes, tears;
//! - a key: an ear flick; copying: it pulls out a surfboard and throws a shaka;
//! - no / cancel: a raspberry, spit flying;
//! - asleep: ears flopped down, a snot bubble growing and popping; offline: pale and flat;
//! - booting: it crash-lands from the sky, a fiery trail above;
//! - little context left: ears at half-mast, heavy eyelids.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Stitch";
pub const SCENE: &str = "ocean";
pub const DESCRIPTION: &str = "Stitch en pixel art: baja las orejas con los errores, saca sus brazos extra y antenas al trabajar, saca la lengua al terminar y aparece con su tabla de surf al copiar";

const LINE: u8 = 1;
const BLUE: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const PALE: u8 = 5;
const EYE: u8 = 6;
const WHITE: u8 = 7;
const NOSE: u8 = 8;
const PINK: u8 = 9;
const MOUTH: u8 = 10;
const TONGUE: u8 = 11;
const RED: u8 = 12;
const YELLOW: u8 = 13;
const ORANGE: u8 = 14;
const GREEN: u8 = 15;
const WATER: u8 = 16;

pub struct Char {
    twitch: f64,
    twitch_left: bool,
    next_twitch: f64,
    giggle: f64,
    next_giggle: f64,
}

/// How its big eyes look this frame.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Open,
    Wide,
    Puppy,
    Shut,
    Asleep,
    Happy,
    Squint,
    Cross,
    Heavy,
}

fn big_eye(c: &mut Canvas, cx: i32, cy: i32, e: Look, look: (i32, i32), right: bool) {
    let (cxf, cyf) = (cx as f64, cy as f64);
    match e {
        Look::Open | Look::Wide | Look::Puppy | Look::Heavy => {
            let (rx, ry) = match e {
                Look::Wide => (2.4, 3.3),
                Look::Puppy => (2.8, 3.4),
                _ => (2.2, 2.9),
            };
            let (lx, ly) = (look.0 as f64 * 0.6, look.1 as f64 * 0.6);
            c.ellipse(cxf + lx, cyf + ly, rx, ry, EYE);
            // Its gloss: a big glint up top and a little one below.
            let gx = if right { cx + 1 } else { cx - 2 };
            c.set(gx + look.0, cy - 2 + look.1, WHITE);
            c.set(gx + 1 + look.0, cy - 2 + look.1, WHITE);
            c.set(gx + look.0, cy - 1 + look.1, WHITE);
            c.set(if right { cx - 1 } else { cx + 1 } + look.0, cy + 1 + look.1, WHITE);
            if e == Look::Puppy {
                c.set(if right { cx - 1 } else { cx }, cy + 2, WATER);
                c.set(if right { cx } else { cx - 1 }, cy + 2, WATER);
            }
            if e == Look::Heavy {
                // Eyelids halfway down.
                for y in cy - 4..cy {
                    for x in cx - 3..=cx + 3 {
                        if c.get(x, y) == EYE || c.get(x, y) == WHITE {
                            c.set(x, y, BLUE);
                        }
                    }
                }
                c.line(cx - 2, cy, cx + 2, cy, LINE);
            }
        }
        Look::Shut => c.line(cx - 2, cy + 1, cx + 2, cy + 1, EYE),
        Look::Asleep => {
            c.line(cx - 1, cy + 1, cx + 1, cy + 1, EYE);
            c.set(cx - 2, cy, EYE);
            c.set(cx + 2, cy, EYE);
        }
        Look::Happy => {
            c.set(cx - 2, cy + 1, EYE);
            c.set(cx - 1, cy, EYE);
            c.set(cx, cy - 1, EYE);
            c.set(cx + 1, cy, EYE);
            c.set(cx + 2, cy + 1, EYE);
        }
        Look::Squint => {
            let s = if right { -1 } else { 1 };
            c.set(cx - 2 * s, cy - 2, EYE);
            c.set(cx - s, cy - 1, EYE);
            c.set(cx, cy, EYE);
            c.set(cx - s, cy + 1, EYE);
            c.set(cx - 2 * s, cy + 2, EYE);
        }
        Look::Cross => {
            for k in -2..=2 {
                c.set(cx + k, cy + k, EYE);
                c.set(cx + k, cy - k, EYE);
            }
        }
    }
}

impl Char {
    fn alien(f: &Frame) -> bool {
        f.working() || f.is(&[State::Searching, State::Web, State::Typing])
    }

    fn giggling(&self, f: &Frame) -> bool {
        f.state == State::Idle && f.groove < 0.1 && f.within(self.giggle, 1.6) && !f.hurt() && !f.happy()
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#3550c8", "#5288f0", "#3d68d2", "#92bcff", "#b4dcff", "#0d1030", "#ffffff", "#3a2e96", "#e88fd0", "#8c1d4e", "#ff7aa8", "#ff4458",
        "#ffd84a", "#f08a34", "#6ad070", "#8fe8ff",
    ];
    const W: usize = 40;
    const H: usize = 40;

    fn new(seed: u64) -> Self {
        Char { twitch: -99.0, twitch_left: false, next_twitch: 1.5 + hash(seed) * 3.0, giggle: -99.0, next_giggle: 4.0 + hash(seed ^ 5) * 5.0 }
    }

    fn step(&mut self, f: &Frame) {
        let f = &dbg(f); // AG2DBG
        if f.t > self.next_twitch {
            self.twitch = f.t;
            self.twitch_left = f.rand(f.t as u64) < 0.5;
            self.next_twitch = f.t + 2.0 + f.rand(f.t as u64 + 7) * 3.5;
        }
        if f.t > self.next_giggle {
            self.giggle = f.t;
            self.next_giggle = f.t + 7.0 + f.rand(f.t as u64 + 11) * 6.0;
        }
    }

    fn event(&mut self, h: Happening, f: &Frame) {
        if let Happening::Event(Event::Key) = h {
            self.twitch = f.t;
            self.twitch_left = f.rand(f.t as u64 * 3 + 1) < 0.5;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        let f = &dbg(f); // AG2DBG
        let hurt = f.hurt();
        let happy = f.happy();
        let alien = Self::alien(f) && !hurt;
        let running = f.is(&[State::Running, State::Testing]);
        let giggle = self.giggling(f);
        let raspberry = f.within(f.times.nope, 1.0);
        let copying = f.within(f.times.copy, 2.6);
        let dance = f.groove > 0.1 && !hurt;
        let tired = f.context > 0.85 && !f.asleep();

        // Ears: their tips (the base stays on the head).
        let (mut lt, mut rt) = ((1.5, 3.0), (38.5, 3.0));
        if f.asleep() || hurt {
            lt = (0.5, 24.0);
            rt = (39.5, 24.0);
        } else if f.is(&[State::Listening, State::Transcribing]) {
            lt = (7.0, -1.0);
            rt = (33.0, -1.0);
        } else if running {
            lt = (0.0, 9.0);
            rt = (40.0, 9.0);
        } else if f.state == State::Asking {
            rt = (39.5, 18.0);
        } else if tired {
            lt = (0.5, 12.0);
            rt = (39.5, 12.0);
        } else if happy {
            let k = f.wave(22.0, 0.0) * 3.0;
            lt = (1.0, 3.0 + k);
            rt = (39.0, 3.0 - k);
        }
        if f.state == State::Speaking && !hurt {
            let k = (f.voice * 20.0).min(2.5) * f.wave(13.0, 0.0);
            lt.1 += k;
            rt.1 -= k;
        }
        if f.state == State::Idle && !hurt && !happy {
            let s = f.wave(1.3, 0.0) * 1.2;
            lt.1 += s;
            rt.1 -= s;
        }
        if let Some(u) = f.progress(self.twitch, 0.3) {
            let k = (u * PI).sin() * 4.0;
            if self.twitch_left { lt.1 += k } else { rt.1 += k }
        }
        if dance {
            let b = f.wave(PI * 2.2, 0.0) * 2.0 * f.groove;
            lt.1 += b;
            rt.1 -= b;
        }

        // Antennae (alien mode), behind the head.
        if alien {
            let w1 = f.wave(7.0, 0.0);
            let w2 = f.wave(7.0, 1.7);
            c.thick((16.0, 9.0), (13.0 + w1, 1.0), 1.2, SHADE);
            c.thick((24.0, 9.0), (27.0 + w2, 1.0), 1.2, SHADE);
            c.ellipse(13.0 + w1, 1.0, 1.3, 1.3, GREEN);
            c.ellipse(27.0 + w2, 1.0, 1.3, 1.3, GREEN);
        }

        // Ears behind the head: a big blue triangle, pink inside, the left one notched.
        let ear = |c: &mut Canvas, tip: (f64, f64), a: (f64, f64), b: (f64, f64)| {
            c.triangle(tip, a, b, BLUE);
            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let k = 0.3;
            let ia = (a.0 + (tip.0 - a.0) * k + (mid.0 - a.0) * 0.4, a.1 + (tip.1 - a.1) * k + (mid.1 - a.1) * 0.4);
            let ib = (b.0 + (tip.0 - b.0) * k + (mid.0 - b.0) * 0.4, b.1 + (tip.1 - b.1) * k + (mid.1 - b.1) * 0.4);
            let it = (tip.0 + (mid.0 - tip.0) * 0.22, tip.1 + (mid.1 - tip.1) * 0.22);
            c.triangle(it, ia, ib, PINK);
        };
        ear(c, lt, (14.0, 9.0), (11.0, 17.0));
        ear(c, rt, (26.0, 9.0), (29.0, 17.0));
        // The notches on the left ear: two bites near its tip.
        for k in [0.25, 0.45] {
            let (x, y) = (lt.0 + (14.0 - lt.0) * k, lt.1 + (9.0 - lt.1) * k);
            let (x, y) = (x.round() as i32, y.round() as i32);
            let off = if lt.1 > 12.0 { (1, 0) } else { (0, -1) };
            c.clear(x + off.0, y + off.1);
            c.clear(x + off.0 * 2, y + off.1 * 2);
        }

        // Extra arms (alien mode), behind the body.
        let step = if running { f.wave(16.0, 0.0) } else { 0.0 };
        if alien {
            let fl = if f.is(&[State::Typing, State::Editing]) { f.wave(24.0, 0.0) * 1.5 } else { f.wave(10.0, 0.0) * 2.5 };
            c.thick((14.0, 29.0), (7.0, 28.0 - fl), 2.4, BLUE);
            c.thick((26.0, 29.0), (33.0, 28.0 + fl), 2.4, BLUE);
            c.ellipse(6.5, 28.0 - fl, 1.4, 1.4, BLUE);
            c.ellipse(33.5, 28.0 + fl, 1.4, 1.4, BLUE);
        }

        // Legs, body, head.
        c.ellipse(15.5, 36.6 - step.max(0.0) * 2.0, 3.0, 2.4, BLUE);
        c.ellipse(24.5, 36.6 - (-step).max(0.0) * 2.0, 3.0, 2.4, BLUE);
        c.ellipse(20.0, 29.0, 7.5, 6.5, BLUE);
        c.ellipse(20.0, 15.5, 11.0, 8.0, BLUE);
        // The tuft on its head (an Elvis pompadour when dancing).
        if dance {
            c.ellipse(20.0 + f.wave(4.4, 0.0), 7.5, 3.5, 1.8, BLUE);
            c.triangle((17.0, 7.0), (25.0 + f.wave(4.4, 0.0) * 2.0, 4.0), (23.0, 8.0), BLUE);
        } else {
            let s = f.wave(2.0, 0.0) * 0.6;
            c.thick((19.0, 8.0), (18.0 + s, 5.0), 1.0, BLUE);
            c.thick((20.5, 8.0), (21.0 + s, 4.5), 1.0, BLUE);
        }

        // Arms: down at its sides; up when done; claws at the mouth giggling; scratching its
        // head thinking; holding a book reading; one at the chin asking; up in a shaka copying;
        // pumping dancing; flailing running.
        let (mut lh, mut rh) = ((11.5, 31.0), (28.5, 31.0));
        if happy {
            lh = (8.0, 21.0);
            rh = (32.0, 21.0);
        } else if hurt {
            lh = (14.0, 32.5);
            rh = (26.0, 32.5);
        } else if giggle || raspberry {
            lh = (16.5, 23.5);
            rh = (23.5, 23.5);
        } else if f.state == State::Reading {
            lh = (14.0, 27.0);
            rh = (26.0, 27.0);
        } else if f.thinking() {
            rh = (32.0, 12.0 + f.wave(16.0, 0.0));
        } else if f.state == State::Asking {
            rh = (24.0, 23.0);
        } else if copying {
            rh = (33.0, 24.0);
        } else if dance {
            let k = f.wave(PI * 2.2, 0.0);
            lh = (9.5, 26.0 - k * 4.0);
            rh = (30.5, 26.0 + k * 4.0);
        } else if running || alien {
            lh = (10.0, 29.0 + step * 2.5 + f.wave(20.0, 0.0));
            rh = (30.0, 29.0 - step * 2.5 - f.wave(20.0, 0.5));
        }
        // Arms in front of it (at its mouth, its chin, its head) are drawn last, outlined.
        let front_l = !happy && !hurt && (giggle || raspberry);
        let front_r = front_l || (!happy && !hurt && (f.thinking() || f.state == State::Asking));
        if !front_l {
            c.thick((14.5, 25.5), lh, 2.6, BLUE);
        }
        if !front_r {
            c.thick((25.5, 25.5), rh, 2.6, BLUE);
        }

        c.outline(LINE, &[]);
        c.shade(BLUE, SHADE, LINE);
        c.light(BLUE, LIGHT, LINE);

        // Claws: little white tips on the hands and the feet.
        for (hx, hy) in [lh, rh] {
            c.set(hx.round() as i32, (hy + 1.3).round() as i32, WHITE);
        }
        if alien {
            let fl = if f.is(&[State::Typing, State::Editing]) { f.wave(24.0, 0.0) * 1.5 } else { f.wave(10.0, 0.0) * 2.5 };
            c.set(5, (28.0 - fl).round() as i32, WHITE);
            c.set(34, (28.0 + fl).round() as i32, WHITE);
        }
        c.set(14, 38 - (step.max(0.0) * 2.0) as i32, WHITE);
        c.set(16, 38 - (step.max(0.0) * 2.0) as i32, WHITE);
        c.set(24, 38 - ((-step).max(0.0) * 2.0) as i32, WHITE);
        c.set(26, 38 - ((-step).max(0.0) * 2.0) as i32, WHITE);

        // The light blue belly and the patches of its face (forehead stripe and around the
        // eyes).
        c.ellipse(20.0, 30.0, 4.5, 4.2, PALE);
        c.ellipse(14.5, 15.5, 4.0, 4.2, PALE);
        c.ellipse(25.5, 15.5, 4.0, 4.2, PALE);
        c.triangle((17.0, 8.5), (23.0, 8.5), (20.0, 15.0), PALE);
        // A chest tuft.
        c.set(20, 27, LIGHT);
        c.set(19, 28, LIGHT);
        c.set(21, 28, LIGHT);

        // The eyes.
        let base = f.eyes();
        let mut eyes = match base {
            Eyes::Open => Look::Open,
            Eyes::Wide => Look::Wide,
            Eyes::Shut => Look::Shut,
            Eyes::Asleep => Look::Asleep,
            Eyes::Happy => Look::Happy,
            Eyes::Squint => Look::Squint,
            Eyes::Cross => Look::Cross,
        };
        if hurt {
            // A moment of shock, then the puppy eyes.
            eyes = if f.within(f.times.error, 0.35) { Look::Cross } else { Look::Puppy };
        } else if giggle || raspberry {
            eyes = Look::Happy;
        } else if f.state == State::Reading {
            eyes = Look::Open;
        } else if tired && eyes == Look::Open {
            eyes = Look::Heavy;
        }
        let look = if f.state == State::Reading { (0, 1) } else { f.look() };
        big_eye(c, 14, 16, eyes, look, false);
        big_eye(c, 26, 16, eyes, look, true);

        // The nose: big and dark, with a shine.
        c.ellipse(20.0, 19.5, 2.6, 1.7, NOSE);
        c.set(19, 19, LIGHT);

        // The mouth: a wide grin closed; toothy open with the voice; tongue out when done; a
        // raspberry; a giggle with its teeth; a wobbly frown hurt.
        let open = f.mouth();
        if happy {
            c.ellipse(20.0, 22.5, 4.5, 1.6, MOUTH);
            c.line(17, 22, 23, 22, WHITE);
            let lick = 3.0 + f.wave(14.0, 0.0) * 1.0;
            c.thick((21.0, 23.0), (22.0 + f.wave(9.0, 0.0), 23.0 + lick), 2.2, TONGUE);
        } else if raspberry {
            c.line(16, 22, 24, 22, NOSE);
            c.thick((20.0, 22.5), (24.0, 24.5), 2.0, TONGUE);
        } else if giggle {
            c.line(15, 22, 25, 22, MOUTH);
            c.line(16, 23, 24, 23, MOUTH);
            for x in (16..=24).step_by(2) {
                c.set(x, 22, WHITE);
            }
        } else if hurt {
            for (x, y) in [(16, 23), (17, 22), (18, 22), (19, 23), (20, 23), (21, 22), (22, 22), (23, 23)] {
                c.set(x, y, NOSE);
            }
        } else if open > 0.08 {
            let h = (1.0 + open * 3.0).round() as i32;
            c.rect(16, 22, 9, h, MOUTH);
            c.line(16, 22, 24, 22, WHITE);
            if h >= 3 {
                c.line(18, 22 + h - 1, 22, 22 + h - 1, TONGUE);
            }
        } else {
            c.line(16, 22, 24, 22, NOSE);
            c.set(15, 21, NOSE);
            c.set(25, 21, NOSE);
        }

        for (on, sh, hand) in [(front_l, (14.5, 25.5), lh), (front_r, (25.5, 25.5), rh)] {
            if on {
                c.thick(sh, hand, 4.4, LINE);
                c.thick(sh, hand, 2.4, BLUE);
                c.set(hand.0.round() as i32, hand.1.round() as i32 - 1, LIGHT);
                c.set(hand.0.round() as i32 - 1, (hand.1 - 1.8).round() as i32, WHITE);
                c.set(hand.0.round() as i32 + 1, (hand.1 - 1.8).round() as i32, WHITE);
            }
        }

        if f.state == State::Offline {
            for col in [BLUE, SHADE, LIGHT, PINK] {
                c.swap(col, PALE);
            }
            c.swap(LINE, SHADE);
        }
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        let f = &dbg(f); // AG2DBG
        if f.state == State::Asking {
            m.lean -= 0.05;
        }
        // A giggle: shoulders shaking.
        if self.giggling(f) {
            m.dy += 0.025 * (f.t * 30.0).sin().abs();
        }
        // Hurt: it sinks a little.
        if let Some(u) = f.progress(f.times.error, 1.3) {
            m.sy *= 1.0 - 0.06 * (u * PI).sin();
        }
        // Booting: a hard landing, squashed for a moment.
        if f.state == State::Booting && (1.1..1.5).contains(&f.st) {
            let u = (f.st - 1.1) / 0.4;
            m.sy *= 1.0 - 0.25 * (u * PI).sin();
            m.sx *= 1.0 + 0.2 * (u * PI).sin();
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let f = &dbg(f); // AG2DBG
        let t = f.t;
        // A hibiscus: red petals around a yellow middle.
        let flower = |w: &mut World, x: f64, y: f64| {
            for (dx, dy) in [(0.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (1.0, 1.0)] {
                w.px(x + dx, y + dy, RED);
            }
            w.px(x, y, YELLOW);
        };

        // Done: hibiscus bursting out of it, spinning, and drops of sea.
        if let Some(u) = f.progress(f.times.done, 1.6) {
            let r = 15.0 + u * 20.0;
            for k in 0..8 {
                let a = k as f64 * PI / 4.0 + u * 2.0;
                let (x, y) = (20.0 + a.cos() * r, 18.0 + a.sin() * r * 0.75 - u * 6.0);
                if k % 2 == 0 {
                    flower(w, x, y);
                } else {
                    w.px(x, y, GREEN);
                    w.px(x + 1.0, y - 1.0, GREEN);
                }
            }
            for k in 0..6 {
                let a = PI + k as f64 * PI / 5.0;
                let rr = u * 18.0;
                w.px_dim(20.0 + a.cos() * rr, 36.0 + a.sin() * rr * 0.6 + u * u * 20.0, WATER);
            }
        }

        // Error: tears falling from its puppy eyes.
        if f.hurt() && !f.within(f.times.error, 0.35) {
            for (k, x) in [(0u64, 11.0), (1, 29.0)] {
                let ph = ((t * 1.6 + k as f64 * 0.5) % 1.0) * 12.0;
                w.px(x, 19.0 + ph, WATER);
                if ph > 1.0 {
                    w.px_dim(x, 18.0 + ph, WATER);
                }
            }
        }

        // Thinking: its little red police cruiser circling over its head.
        if f.thinking() {
            let a = t * 1.8;
            let (x, y) = (20.0 + a.cos() * 18.0, -4.0 + a.sin() * 3.5);
            let dir = if a.sin() > 0.0 { -1.0 } else { 1.0 };
            for dx in -2..=2 {
                w.px(x + dx as f64, y, RED);
            }
            w.px(x - 1.0, y - 1.0, WATER);
            w.px(x, y - 1.0, WATER);
            w.px(x + dir * 3.0, y, YELLOW);
            if f.wave(30.0, 0.0) > 0.0 {
                w.px(x - dir * 3.0, y, ORANGE);
            }
        }

        // Reading: "The Ugly Duckling", held up in its claws.
        if f.state == State::Reading {
            let flip = f.wave(1.5, 0.0) > 0.85;
            for y in 0..7 {
                for x in 0..10 {
                    let col = if x == 5 { WHITE } else if y == 0 || y == 6 || x == 0 || x == 9 { ORANGE } else { GREEN };
                    w.px(15.0 + x as f64, 24.0 + y as f64, col);
                }
            }
            // The duckling on its cover.
            for (x, y) in [(2.0, 2.0), (2.0, 3.0), (3.0, 3.0), (3.0, 4.0), (4.0, 4.0), (1.0, 2.0)] {
                w.px(15.0 + x, 24.0 + y, YELLOW);
            }
            w.px(16.0, 26.0, ORANGE);
            if flip {
                w.px(25.0, 23.0, WHITE);
                w.px(24.0, 23.0, WHITE);
            }
        }

        // Searching, on the web: radar rings from its antennae.
        if f.is(&[State::Searching, State::Web]) {
            for (k, x) in [(0u64, 13.0), (1, 27.0)] {
                let r = ((t * 1.4 + k as f64 * 0.5) % 1.0) * 7.0;
                for s in 0..5 {
                    let a = -PI / 2.0 + (s as f64 - 2.0) * 0.4;
                    w.px_dim(x + a.cos() * r, 1.0 + a.sin() * r, GREEN);
                }
            }
        }

        // Listening: sound rings reaching its ear tips.
        if f.is(&[State::Listening, State::Transcribing]) {
            let r = (t * 1.5 % 1.0) * 5.0 + 1.0;
            for (x, dir) in [(4.0, -1.0), (36.0, 1.0)] {
                for s in -1..=1 {
                    w.px_dim(x + dir * r, -2.0 + s as f64 * 1.5 + 1.0, WATER);
                }
            }
        }

        // Giggling: little "he" marks bobbing at its sides.
        if self.giggling(f) {
            for (k, x) in [(0u64, 3.0), (1, 34.0)] {
                if (t * 6.0 + k as f64).sin() > -0.2 {
                    let y = 18.0 - ((t * 2.0 + k as f64 * 0.4) % 1.0) * 6.0;
                    // "h"
                    for dy in 0..4 {
                        w.px(x, y + dy as f64, YELLOW);
                    }
                    w.px(x + 1.0, y + 2.0, YELLOW);
                    w.px(x + 2.0, y + 3.0, YELLOW);
                }
            }
        }

        // No, cancel: a raspberry, spit flying.
        if let Some(u) = f.progress(f.times.nope, 1.0) {
            for k in 0..4 {
                let d = u * 14.0 + k as f64 * 2.0;
                w.px_dim(25.0 + d, 24.0 + (k as f64 - 1.5) * u * 4.0 + u * u * 4.0, WATER);
            }
        }

        // Copying: its surfboard pops up beside it, and a shaka.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let rise = (u * 5.0).min(1.0);
            let (bx, by) = (34.0, 37.0 - rise * 22.0);
            for y in 0..22 {
                let half = if y < 3 { [0.5, 1.0, 1.5][y as usize] } else if y > 19 { 1.5 - (y - 19) as f64 * 0.5 } else { 2.0 };
                let mut x = -half;
                while x <= half {
                    let col = if x.abs() < 0.6 { RED } else { ORANGE };
                    w.px(bx + x, by + y as f64, col);
                    x += 1.0;
                }
            }
            // A hibiscus on the board.
            flower(w, bx, by + 8.0);
            // Splashes where it popped up.
            if u < 0.3 {
                for k in 0..5 {
                    w.px(bx - 4.0 + k as f64 * 2.0, 36.0 - (u * 30.0) * (1.0 - (k as f64 - 2.0).abs() * 0.25), WATER);
                }
            }
        }

        // Merge: ohana — two little hearts rising.
        if let Some(u) = f.progress(f.times.merge, 1.6) {
            for (k, x) in [(0u64, 6.0), (1, 33.0)] {
                let y = 10.0 - u * 10.0 - k as f64 * 2.0;
                for (dx, dy) in [(0.0, 0.0), (2.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (0.0, 2.0), (1.0, 2.0), (2.0, 2.0), (1.0, 3.0)] {
                    w.px(x + dx, y + dy, PINK);
                }
            }
        }

        // Asleep: a snot bubble from its nose, growing and popping.
        if f.state == State::Sleeping {
            let u = (t * 0.35) % 1.0;
            if u < 0.85 {
                let r = 0.8 + u * 3.2;
                let (cx, cy) = (23.5 + r * 0.6, 20.5);
                for k in 0..12 {
                    let a = k as f64 * PI / 6.0;
                    w.px_dim(cx + a.cos() * r, cy + a.sin() * r, WATER);
                }
                w.px(cx - r * 0.4, cy - r * 0.5, WHITE);
            } else {
                for k in 0..6 {
                    let a = k as f64 * PI / 3.0;
                    w.px_dim(26.0 + a.cos() * 5.0, 20.5 + a.sin() * 4.0, WATER);
                }
            }
        }

        // Booting: it falls from the sky like a meteor, a fiery trail above it.
        if f.state == State::Booting && f.st < 1.3 {
            let fall = (1.0 - (f.st / 1.2).min(1.0)).max(0.0);
            for k in 0..10 {
                let y = -2.0 - k as f64 * 3.0 - fall * 6.0;
                let jig = ((t * 20.0 + k as f64).sin()) * 2.0;
                let col = if k < 3 { YELLOW } else if k < 6 { ORANGE } else { RED };
                w.px(16.0 + jig, y, col);
                w.px(24.0 - jig, y + 1.0, col);
                if k % 2 == 0 {
                    w.px(20.0, y - 1.0, col);
                }
            }
        }
        // Booting, landed: a splash of sand.
        if f.state == State::Booting && (1.1..1.8).contains(&f.st) {
            let u = (f.st - 1.1) / 0.7;
            for k in 0..8 {
                let a = PI + k as f64 * PI / 7.0;
                w.px_dim(20.0 + a.cos() * (6.0 + u * 14.0), 39.0 + a.sin() * u * 8.0, YELLOW);
            }
        }
    }
}

// AG2DBG-BEGIN
fn dbg(f: &Frame) -> Frame {
    let mut g = Frame { t: f.t, dt: f.dt, state: f.state, st: f.st, voice: f.voice, music: f.music, groove: f.groove, beat: f.beat, gaze: f.gaze, blink: f.blink, yawn: f.yawn, red: f.red, context: f.context, times: f.times, seed: f.seed };
    if option_env!("AG2_DBG").is_none() {
        return g;
    }
    let done = f.times.done;
    let err = f.times.error;
    match f.state {
        State::Idle if done > 0.0 => { g.times.done = -99.0; g.times.copy = done; }
        State::Idle if err > 0.0 => { g.times.error = -99.0; g.times.nope = err; g.red = 0.0; }
        State::Idle => { g.state = State::Booting; g.st = 0.7; }
        State::Thinking => g.state = State::Listening,
        State::Speaking => { g.state = State::Asking; g.voice = 0.0; }
        State::Reading => { g.state = State::Idle; g.groove = 1.0; g.music = 1.0; }
        State::Running => { g.state = State::Idle; g.context = 0.95; g.times.key = f.t - 0.05; }
        State::Sleeping => g.state = State::Offline,
        _ => {}
    }
    g
}
// AG2DBG-END
