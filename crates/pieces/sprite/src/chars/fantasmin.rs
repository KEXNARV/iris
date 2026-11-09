//! Fantasmín: un fantasmita tímido y travieso que nunca suelta su farolito. Drawn from shapes
//! every frame (a round sheet of a head, a wavy wisp of a tail, two little nub arms, big eyes,
//! pink cheeks and a brass lantern with a flickering flame), so everything on him moves. He
//! floats: he never touches the floor, his tail ripples behind him. What he does:
//! - waiting: bobbing in the air, the lantern swinging, the tail rippling; now and then he's
//!   shy and goes see-through from the top down (only his glowing eyes left), or he's naughty
//!   and sticks his tongue out with a wink;
//! - listening: leans in with a nub cupped by his head, eyes wide;
//! - thinking, planning: he goes invisible in patches (a dither that comes and goes), his eyes
//!   glow mint, the lantern burns ghost-green and will-o'-wisps circle his head;
//! - speaking: the mouth with the voice, little rings of ectoplasm puffing out;
//! - reading: the lantern held up to a little purple book, a warm glow on it; searching: the
//!   lantern held far out, its beam sweeping the dark; on the web: watching a little spider
//!   dangling on its thread;
//! - editing: haunting a page that writes itself; running, testing: zooming, tail streaming,
//!   ghostly speed lines; git: two wisps branch off his flame; delegating: tiny ghostlings
//!   with tiny lanterns orbit him; compacting: squeezed, eyes shut tight;
//! - done: "¡BU!": arms up, a huge grin, the lantern flaring, ectoplasm bursting out;
//! - error: he spooks HIMSELF: a jolt, eyes wide, and he bursts into loose pixels that drift
//!   back together;
//! - a key: the flame flares; copying: he duplicates as a mirror image, a glassy twin;
//! - asking: a tilt and a little nub waving; music: a wobbly dance, lantern swinging, tail
//!   whipping; asleep: a nightcap, tail curled under him, the lantern set down with a tiny
//!   flame; offline: almost gone, just two faint eyes and a cold lantern; booting: he
//!   materializes bottom to top and the lantern lights; little context left: drooping, a weak
//!   flame and a sweat drop.

use crate::kit::*;
use core::f64::consts::PI;
use iris_piece::part::*;

pub const NAME: &str = "Fantasmín";
pub const SCENE: &str = "spooky";
pub const DESCRIPTION: &str = "Fantasmín, un fantasmita tímido y travieso con su farolito: se vuelve invisible a ratos al pensar, se asusta de sí mismo y se desarma en píxeles con los errores, grita ¡BU! al terminar y se duplica como espejo al copiar";

const LINE: u8 = 1;
const BODY: u8 = 2;
const SHADE: u8 = 3;
const LIGHT: u8 = 4;
const EYE: u8 = 5;
const CHEEK: u8 = 6;
const MOUTH: u8 = 7;
const TONGUE: u8 = 8;
const FLAME: u8 = 9;
const HOT: u8 = 10;
const BRASS: u8 = 11;
const GLOW: u8 = 12;
const ECTO: u8 = 13;
const CAP: u8 = 14;
const PALE: u8 = 15;
const DROP: u8 = 16;

const W: usize = 37;
const H: usize = 40;
/// The middle of the drawing (pixel 18's center).
const CX: f64 = 18.5;

const BAYER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

pub struct Char {
    /// A quirk while waiting (shy fade or a cheeky tongue), when it started, and the next one.
    quirk: f64,
    cheeky: bool,
    next_quirk: f64,
}

/// Where things are this frame.
struct Pose {
    /// The head's center height.
    hy: f64,
    /// The hands (left holds the lantern).
    la: (f64, f64),
    ra: (f64, f64),
    /// The lantern's top-left, and how lit it is (0: out, 1: normal, 2: flaring).
    lantern: (f64, f64),
    lit: f64,
}

/// Fades the sheet (not the eyes, cheeks, lantern or props) where `amount(x, y)` says, with an
/// ordered dither: 0 solid, 1 gone.
fn fade(c: &mut Canvas, amount: impl Fn(i32, i32) -> f64) {
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            let p = c.get(x, y);
            if matches!(p, BODY | SHADE | LIGHT | LINE) {
                let a = amount(x, y);
                if a > 0.0 && (BAYER[((y & 3) * 4 + (x & 3)) as usize] as f64 + 0.5) / 16.0 < a {
                    c.clear(x, y);
                }
            }
        }
    }
}

/// Glyphs 3×5 for "¡BU!".
const GLYPHS: [&[&str; 5]; 4] = [
    &["#..", "...", "#..", "#..", "#.."],
    &["##.", "#.#", "##.", "#.#", "##."],
    &["#.#", "#.#", "#.#", "#.#", "###"],
    &["#..", "#..", "#..", "...", "#.."],
];

/// A small ghostling (5 wide) with a speck of a lantern, its top-left at (x, y).
fn ghostling(w: &mut World, x: f64, y: f64, flap: bool, lamp: bool) {
    let rows = [".bbb.", "bebeb", "bbbbb", "bbbbb", if flap { "b.b.b" } else { ".b.b." }];
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            let col = match ch {
                'b' => BODY,
                'e' => EYE,
                _ => 0,
            };
            w.px(x + i as f64, y + j as f64, col);
        }
    }
    if lamp {
        w.px(x - 1.0, y + 3.0, FLAME);
    }
}

impl Char {
    fn pose(&self, f: &Frame) -> Pose {
        let t = f.t;
        let tired = f.context > 0.85 && !f.asleep();
        let bu = f.within(f.times.done, 1.6);
        let sink = if f.state == State::Sleeping {
            6.0
        } else if f.state == State::Offline {
            3.0
        } else if tired {
            1.5
        } else {
            0.0
        };
        let hy = 14.0 + sink;
        let rest = (CX - 10.5, hy + 7.5);
        let mut la = rest;
        let mut ra = (CX + 10.5, hy + 7.5);
        let dance = f.groove > 0.1;
        if bu {
            let wig = f.wave(18.0, 0.0) * 0.8;
            la = (CX - 14.0, hy - 6.0 + wig);
            ra = (CX + 14.0, hy - 6.0 - wig);
        } else if f.hurt() {
            la = (CX - 12.5, hy + 1.0);
            ra = (CX + 12.5, hy + 1.0);
        } else if f.state == State::Sleeping {
            la = (CX - 9.0, hy + 8.5);
            ra = (CX + 9.0, hy + 8.5);
        } else if dance {
            let s = f.wave(PI * 2.2, 0.0) * 4.0 * f.groove.min(1.0);
            la = (CX - 12.5, hy + 2.0 + s);
            ra = (CX + 12.5, hy + 2.0 - s);
        } else {
            match f.state {
                State::Reading => {
                    la = (CX - 12.0, hy + 0.5);
                    ra = (CX + 7.5, hy + 11.0);
                }
                State::Searching => la = (CX - 15.0, hy + 3.0 + f.wave(2.0, 0.0)),
                State::Web => la = (CX - 12.5, hy - 1.0),
                State::Listening | State::Transcribing => ra = (CX + 11.5, hy - 1.5),
                State::Asking => ra = (CX + 13.0, hy - 5.0 + f.wave(10.0, 0.0)),
                State::Editing => ra = (CX + 11.5 + f.wave(9.0, 0.0), hy + 9.0 + f.wave(4.5, 1.0) * 0.7),
                State::Running | State::Testing => {
                    la = (CX - 11.5, hy + 8.5);
                    ra = (CX + 9.0, hy + 9.5);
                }
                _ => {}
            }
        }
        // The lantern hangs from the left hand, swinging; asleep it's set on the floor.
        let swing = if f.state == State::Sleeping {
            0.0
        } else if dance {
            f.wave(PI * 2.2, 0.6) * 2.0
        } else if f.is(&[State::Running, State::Testing]) {
            -1.5 + f.wave(12.0, 0.0) * 0.6
        } else {
            f.wave(1.9, 0.3) * 1.2
        };
        let lantern = if f.state == State::Sleeping { (1.0, 31.0) } else { (la.0 - 2.5 + swing, la.1 + 2.0) };
        let lit = if f.state == State::Offline {
            0.0
        } else if f.state == State::Booting {
            if f.st < 1.0 {
                0.0
            } else {
                ((f.st - 1.0) * 3.0).min(1.0)
            }
        } else if bu || f.within(f.times.key, 0.2) {
            2.0
        } else if f.state == State::Sleeping || tired {
            0.4
        } else {
            1.0
        };
        let _ = t;
        Pose { hy, la, ra, lantern, lit }
    }

    /// The whole of him, without the error burst.
    fn paint(&self, f: &Frame, c: &mut Canvas) {
        let t = f.t;
        let p = self.pose(f);
        let hy = p.hy;
        let bu = f.within(f.times.done, 1.6);
        let asleep = f.state == State::Sleeping;
        let running = f.is(&[State::Running, State::Testing]);
        let dance = f.groove > 0.1;
        let tired = f.context > 0.85 && !f.asleep();
        let fright = f.progress(f.times.error, 0.2).is_some();
        let shy = f.state == State::Idle && !self.cheeky && f.within(self.quirk, 2.4);
        let cheeky = f.state == State::Idle && self.cheeky && f.within(self.quirk, 1.6);

        // The tail: a wisp tapering down from the sheet, rippling (streaming back when he
        // zooms, whipping when he dances, curled under him asleep).
        let n = 14;
        let (speed, amp) = if running {
            (9.0, 0.32)
        } else if dance {
            (PI * 2.2, 0.4)
        } else {
            (2.6, 0.24)
        };
        let bottom = 37.0;
        for k in 0..=n {
            let u = k as f64 / n as f64;
            let r = 9.6 * (1.0 - u).powf(0.9) + 1.1;
            let (x, y) = if asleep {
                let x = CX + u * u * 14.0;
                let y = hy + 5.0 + (bottom - hy - 5.0) * (1.0 - (1.0 - u) * (1.0 - u)) - (u - 0.75).max(0.0) * 18.0;
                (x, y)
            } else {
                let back = if running { -u * u * 9.0 } else { 0.0 };
                let x = CX + f.wave(speed, -u * 4.2) * amp * k as f64 + back;
                let y = hy + 5.0 + u * (bottom - hy - 5.0) - if running { u * u * 3.0 } else { 0.0 };
                (x, y)
            };
            c.ellipse(x, y, r, r * 0.85, BODY);
        }

        // The sheet of a head, puffed up when he shouts "¡BU!".
        let puff = if bu { 0.8 } else { 0.0 };
        c.ellipse(CX, hy, 11.0 + puff, 11.0 + puff * 0.5, BODY);

        // The nightcap, asleep: drooping to one side, a pom at its tip.
        if asleep {
            c.triangle((CX - 8.0, hy - 6.5), (CX + 6.0, hy - 9.5), (CX + 13.0, hy - 1.0), CAP);
            c.ellipse(CX - 1.0, hy - 7.8, 7.5, 2.0, CAP);
            c.thick((CX - 8.5, hy - 6.0), (CX + 5.5, hy - 9.6), 2.0, LIGHT);
            c.ellipse(CX + 13.5, hy - 0.5, 1.6, 1.6, LIGHT);
        }

        // Arms: little nubs from the sides of the sheet.
        let arm = |c: &mut Canvas, from: (f64, f64), to: (f64, f64)| {
            c.thick(from, to, 3.0, BODY);
            c.ellipse(to.0, to.1, 1.8, 1.8, BODY);
        };
        arm(c, (CX - 7.5, hy + 5.5), p.la);
        arm(c, (CX + 7.5, hy + 5.5), p.ra);

        // A little purple book in his hand, reading.
        let reading = f.state == State::Reading && !bu && !f.hurt();
        if reading {
            let (bx, by) = (p.ra.0 - 6.0, p.ra.1 - 3.0);
            c.rect(bx as i32, by as i32, 9, 6, CAP);
        }

        c.outline(LINE, &[]);
        c.shade(BODY, SHADE, LINE);
        c.light(BODY, LIGHT, LINE);

        if reading {
            let (bx, by) = ((p.ra.0 - 6.0) as i32, (p.ra.1 - 3.0) as i32);
            c.rect(bx + 1, by + 1, 3, 4, LIGHT);
            c.rect(bx + 5, by + 1, 3, 4, BODY);
            // Lines of text, the one he's on flickering.
            let on = ((t * 1.5) as i32) % 4;
            for k in 0..2 {
                c.set(bx + 2, by + 2 + k * 2, PALE);
                c.set(bx + 6, by + 2 + k * 2, if on == k { ECTO } else { PALE });
                c.set(bx + 5, by + 2 + k * 2, PALE);
            }
        }

        // The face.
        let ey = (hy - 2.0) as i32;
        let (lx, rx) = (13, 22);
        let mut eyes = f.eyes();
        if f.thinking() {
            eyes = Eyes::Open;
        }
        if fright {
            eyes = Eyes::Wide;
        } else if f.hurt() {
            eyes = Eyes::Cross;
        } else if bu {
            eyes = Eyes::Happy;
        } else if f.state == State::Compacting {
            eyes = Eyes::Squint;
        } else if f.state == State::Offline {
            eyes = Eyes::Open;
        } else if tired && !f.blink && eyes == Eyes::Open {
            eyes = Eyes::Shut;
        }
        let mut look = f.look();
        match f.state {
            State::Searching => look = (-1, 0),
            State::Web => look = (0, -1),
            State::Reading => look = (1, 1),
            State::Editing => look = (1, 1),
            _ => {}
        }
        if shy {
            look = (0, 1);
        }
        if fright {
            look = (0, 0);
        }
        if eyes == Eyes::Squint {
            c.eye(lx, ey, 3, 4, eyes, look, EYE, 0, 0, false);
            c.eye(rx, ey, 3, 4, eyes, look, EYE, 0, 0, true);
        } else if cheeky && f.since(self.quirk) > 0.25 && f.since(self.quirk) < 1.4 {
            // A wink.
            c.eye(lx, ey, 3, 4, Eyes::Open, (0, 0), EYE, LIGHT, 0, false);
            c.eye(rx, ey, 3, 4, Eyes::Happy, (0, 0), EYE, 0, 0, true);
        } else {
            c.eye(lx, ey, 3, 4, eyes, look, EYE, LIGHT, 0, false);
            c.eye(rx, ey, 3, 4, eyes, look, EYE, LIGHT, 0, true);
            if fright {
                // Tiny pupils of terror: just a glint in the middle.
                for x in [lx, rx] {
                    c.rect(x, ey - 1, 3, 5, EYE);
                    c.set(x + 1, ey + 1, LIGHT);
                }
            }
        }
        // Cheeks: blushing harder when shy or cheering.
        let blush = if shy || bu { 2.2 } else { 1.6 };
        c.ellipse(CX - 8.5, hy + 3.2, blush, 1.0, CHEEK);
        c.ellipse(CX + 8.5, hy + 3.2, blush, 1.0, CHEEK);
        // The mouth.
        let my = (hy + 3.0) as i32;
        if bu {
            c.mouth(18, my, 1.0, 7, 5, MOUTH, TONGUE);
        } else if fright {
            c.ellipse(CX, my as f64 + 1.5, 1.6, 2.0, MOUTH);
        } else if f.state == State::Speaking || f.mouth() > 0.15 {
            c.mouth(18, my, f.mouth().max(0.25), 5, 4, MOUTH, TONGUE);
        } else if asleep {
            c.ellipse(CX, my as f64 + 0.5, 0.8, 0.8, MOUTH);
        } else if f.hurt() {
            c.line(16, my + 1, 20, my + 1, MOUTH);
            c.set(17, my, MOUTH);
            c.set(19, my, MOUTH);
        } else if tired || f.state == State::Offline {
            c.line(17, my + 1, 19, my + 1, MOUTH);
        } else {
            c.smile(18, my, 5, MOUTH);
            if cheeky && f.since(self.quirk) > 0.25 && f.since(self.quirk) < 1.4 {
                // Blep: the tongue out.
                c.rect(17, my + 1, 3, 2, TONGUE);
                c.set(18, my + 2, MOUTH);
            }
        }

        // See-through: thinking goes in patches, shy goes from the top down, booting fills in
        // from the bottom up, offline is almost gone, asleep a little faint.
        let glow_eyes;
        if f.thinking() {
            glow_eyes = true;
            let seed = f.seed;
            fade(c, |x, y| {
                let cell = ((x / 5) as u64) * 31 + (y / 5) as u64;
                let ph = t * 1.3 + hash(seed ^ cell.wrapping_mul(0x9E37_79B9)) * 9.0;
                ((0.5 + 0.5 * ph.sin() - 0.4) * 2.4).clamp(0.0, 0.95)
            });
        } else if f.state == State::Booting {
            glow_eyes = true;
            let edge = H as f64 * (1.0 - (f.st / 1.3).min(1.0));
            fade(c, |_, y| ((edge - y as f64) / 6.0 + 0.5).clamp(0.0, 1.0));
        } else if f.state == State::Offline {
            glow_eyes = true;
            fade(c, |_, y| 0.8 + 0.12 * (y as f64 * 0.45 - t * 2.0).sin());
        } else if shy {
            glow_eyes = true;
            let u = f.since(self.quirk) / 2.4;
            let k = (u * PI).sin().min(0.8) / 0.8;
            let edge = k * (H as f64 + 6.0);
            fade(c, |_, y| ((edge - y as f64) / 5.0).clamp(0.0, 0.92));
        } else if asleep {
            glow_eyes = false;
            fade(c, |_, y| if y > 30 { 0.25 } else { 0.0 });
        } else {
            glow_eyes = false;
        }
        if glow_eyes {
            c.swap(EYE, ECTO);
        }
        if f.state == State::Offline {
            c.swap(CHEEK, PALE);
            c.swap(MOUTH, PALE);
        }

        // The lantern: a brass cap and ring, glass with a flame, a base.
        self.lantern(f, c, &p);
        let _ = t;
    }

    fn lantern(&self, f: &Frame, c: &mut Canvas, p: &Pose) {
        let (x, y) = (p.lantern.0.round() as i32, p.lantern.1.round() as i32);
        // The handle up to the hand.
        if f.state != State::Sleeping {
            c.line(x + 2, y - 1, p.la.0.round() as i32, p.la.1.round() as i32 + 1, BRASS);
        } else {
            c.set(x + 1, y - 1, BRASS);
            c.set(x + 3, y - 1, BRASS);
            c.set(x + 2, y - 2, BRASS);
        }
        c.rect(x + 1, y, 3, 1, BRASS);
        c.rect(x, y + 1, 5, 1, BRASS);
        let ghostfire = f.thinking();
        let glass = if p.lit <= 0.01 { PALE } else if ghostfire { ECTO } else { GLOW };
        c.rect(x, y + 2, 5, 4, BRASS);
        c.rect(x + 1, y + 2, 3, 4, glass);
        c.rect(x, y + 6, 5, 1, BRASS);
        c.set(x + 1, y + 7, BRASS);
        c.set(x + 3, y + 7, BRASS);
        if p.lit > 0.01 {
            let flick = f.rand((f.t * 12.0) as u64) < 0.5;
            let (core, tip) = if ghostfire { (LIGHT, ECTO) } else { (HOT, FLAME) };
            if p.lit >= 1.0 {
                c.set(x + 2, y + 4, core);
                c.set(x + 2, y + 3, tip);
                c.set(x + if flick { 1 } else { 3 }, y + 4, tip);
                c.set(x + 2, y + 5, tip);
            } else {
                c.set(x + 2, y + 5, if flick { tip } else { core });
            }
            if p.lit > 1.5 {
                c.rect(x + 1, y + 2, 3, 4, FLAME);
                c.rect(x + 2, y + 3, 1, 2, LIGHT);
            }
        } else {
            c.set(x + 2, y + 5, BRASS);
        }
        // The window bars.
        c.set(x + 2, y + 2, BRASS);
    }

    fn burst(&self, f: &Frame) -> Option<f64> {
        f.progress(f.times.error, 1.25).filter(|&u| u * 1.25 > 0.2).map(|u| ((u * 1.25 - 0.2) / 1.05).clamp(0.0, 1.0))
    }
}

impl Character for Char {
    const COLORS: &'static [&'static str] = &[
        "#8b95e0", "#eaf0ff", "#b4c0f2", "#ffffff", "#1a1636", "#ff9cc4", "#a33a6e", "#ff7a9c", "#ffd84a", "#ff8a2a", "#d0903e", "#fff2a8",
        "#8cf5d8", "#a77cff", "#7c84a8", "#8fd6ff",
    ];
    const W: usize = W;
    const H: usize = H;

    fn new(seed: u64) -> Self {
        Char { quirk: -99.0, cheeky: false, next_quirk: 3.0 + hash(seed) * 4.0 }
    }

    fn step(&mut self, f: &Frame) {
        if f.state == State::Idle && f.t > self.next_quirk {
            self.quirk = f.t;
            self.cheeky = f.rand(f.t as u64 + 11) < 0.45;
            self.next_quirk = f.t + 6.0 + f.rand(f.t as u64 + 5) * 5.0;
        }
    }

    fn draw(&self, f: &Frame, c: &mut Canvas) {
        // Bursting into pixels: he's all in `around`, flying apart.
        if self.burst(f).is_some() {
            return;
        }
        self.paint(f, c);
    }

    fn motion(&self, f: &Frame, m: &mut Motion) {
        // Floating: never quite touching the floor, bobbing.
        let asleep = f.state == State::Sleeping;
        m.dy += if asleep { 0.01 } else { 0.05 + 0.04 * f.wave(1.6, 0.0) };
        m.lean += 0.02 * f.wave(1.1, 0.5);
        if f.is(&[State::Running, State::Testing]) {
            m.lean += 0.1;
            m.dy += 0.03 * f.wave(5.0, 0.0);
        }
        if f.is(&[State::Listening, State::Transcribing]) {
            m.lean -= 0.05;
        }
        if f.state == State::Asking {
            m.lean += 0.04;
        }
        if f.groove > 0.1 {
            m.lean += f.groove.min(1.0) * 0.08 * f.wave(PI * 2.2, 0.0);
            m.dy += 0.04 * f.wave(PI * 4.4, 0.0).abs();
        }
        // The jolt of a fright.
        if let Some(u) = f.progress(f.times.error, 0.2) {
            m.dy += 0.18 * (u * PI).sin();
            m.sy *= 1.06;
            m.dx = 0.0;
        }
        // "¡BU!": a puff.
        if let Some(u) = f.progress(f.times.done, 0.5) {
            let k = (u * PI).sin();
            m.sx *= 1.0 + 0.08 * k;
            m.sy *= 1.0 + 0.08 * k;
        }
    }

    fn around(&self, f: &Frame, w: &mut World) {
        let t = f.t;
        let p = self.pose(f);
        let (lx, ly) = (p.lantern.0.round() + 2.5, p.lantern.1.round() + 4.0);

        // An error: he flies apart into loose pixels that drift back together.
        if let Some(v) = self.burst(f) {
            let mut tmp = Canvas::new(W, H);
            self.paint(f, &mut tmp);
            let k = (v * PI).sin().powf(0.6);
            for y in 0..H as i32 {
                for x in 0..W as i32 {
                    let col = tmp.get(x, y);
                    if col == 0 {
                        continue;
                    }
                    let h1 = hash(f.seed ^ (x as u64 * 131 + y as u64 * 7919));
                    let h2 = hash(f.seed ^ (x as u64 * 977 + y as u64 * 37 + 5));
                    let (dx, dy) = (x as f64 + 0.5 - CX, y as f64 - 20.0);
                    let a = dy.atan2(dx) + (h2 - 0.5) * 1.4;
                    let d = (4.0 + h1 * 13.0) * k;
                    let px = x as f64 + a.cos() * d + (t * 9.0 + h1 * 6.0).sin() * k;
                    let py = y as f64 + a.sin() * d * 0.8 + k * k * 4.0;
                    if h1 > 0.75 && k > 0.3 {
                        w.px_dim(px, py, col);
                    } else {
                        w.px(px, py, col);
                    }
                }
            }
            return;
        }

        // The lantern's warm glow around it (ghost-green while thinking).
        if p.lit > 0.01 {
            let glow = if f.thinking() { ECTO } else { GLOW };
            let r = 4.5 + p.lit * 0.8 + 0.4 * f.wave(7.0, 0.0);
            let nn = if p.lit > 1.5 { 14 } else { 10 };
            for k in 0..nn {
                let a = k as f64 * PI * 2.0 / nn as f64 + t * 0.4;
                if f.rand(k + (t * 6.0) as u64 * 17) < 0.8 {
                    w.px_dim(lx + a.cos() * r, ly + a.sin() * r * 0.9, glow);
                }
            }
        }

        // Thinking: will-o'-wisps circling his head.
        if f.thinking() {
            for k in 0..2 {
                let a = t * 1.8 + k as f64 * PI;
                let (x, y) = (CX + a.cos() * 15.0, p.hy - 4.0 + a.sin() * 5.0);
                w.px(x, y, ECTO);
                w.px(x, y - 1.0, LIGHT);
                w.px_dim(x - a.sin().signum(), y + 1.0, ECTO);
            }
        }

        // Speaking: rings of ectoplasm puffing out of his mouth.
        if f.state == State::Speaking && f.voice > 0.03 {
            for k in 0..2 {
                let ph = (t * 0.9 + k as f64 * 0.5) % 1.0;
                let (x, y) = (CX + 5.0 + ph * 14.0, p.hy + 3.0 - ph * 10.0);
                let s = if ph > 0.5 { 2.0 } else { 1.0 };
                for (i, j) in [(0.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
                    if ph > 0.65 {
                        w.px_dim(x + i * s, y + j * s, ECTO);
                    } else {
                        w.px(x + i * s, y + j * s, ECTO);
                    }
                }
            }
        }

        // Searching: the lantern's beam sweeping the dark.
        if f.state == State::Searching && !f.hurt() {
            let a0 = PI * 0.95 + 0.45 * f.wave(1.4, 0.0);
            for k in 0..14 {
                let d = 4.0 + (k / 2) as f64 * 3.0;
                let spread = (k % 2) as f64 * 2.0 - 1.0;
                let a = a0 + spread * (0.05 + d * 0.008);
                let (x, y) = (lx + a.cos() * d, ly + a.sin() * d);
                w.px_dim(x, y, GLOW);
            }
            // What it finds: a glint at the end of the beam.
            if f.wave(5.0, 0.0) > 0.5 {
                w.px(lx + a0.cos() * 25.0, ly + a0.sin() * 25.0, LIGHT);
            }
        }

        // On the web: a little spider dangling on its thread above him.
        if f.state == State::Web {
            let sx = CX + 9.0 + f.wave(0.9, 0.0) * 1.5;
            let sy = -4.0 + f.wave(1.7, 0.0) * 3.0;
            let mut y = -24.0;
            while y < sy {
                w.px_dim(sx, y, PALE);
                y += 1.0;
            }
            w.px(sx, sy, CAP);
            w.px(sx + 1.0, sy, CAP);
            w.px(sx, sy + 1.0, CAP);
            w.px(sx + 1.0, sy + 1.0, CAP);
            let kick = if f.wave(8.0, 0.0) > 0.0 { 0.0 } else { 1.0 };
            for (i, j) in [(-1.0, 0.0), (2.0, 0.0), (-1.0, 2.0 - kick), (2.0, 2.0 - kick)] {
                w.px(sx + i, sy + j, PALE);
            }
            w.px(sx, sy + 1.0, ECTO);
        }

        // Editing: a page that writes itself while he haunts it.
        if f.state == State::Editing {
            let (px0, py0) = (31.0, p.hy + 5.0 + f.wave(1.3, 0.0));
            for j in 0..8 {
                for i in 0..7 {
                    w.px(px0 + i as f64, py0 + j as f64, if i == 0 || j == 0 { BODY } else { LIGHT });
                }
            }
            let written = ((f.st * 6.0) as i32) % 24;
            for k in 0..written.min(20) {
                let (row, col) = (k / 5, k % 5);
                if (row * 3 + col) % 4 != 3 {
                    w.px(px0 + 1.0 + col as f64, py0 + 1.5 + row as f64 * 1.6, PALE);
                }
            }
            let k = written.min(20);
            w.px(px0 + 1.0 + (k % 5) as f64, py0 + (k / 5) as f64 * 1.6, ECTO);
        }

        // Zooming: ghostly speed lines behind him.
        if f.is(&[State::Running, State::Testing]) {
            for k in 0..5 {
                let ph = (t * 2.5 + k as f64 * 0.37) % 1.0;
                let y = 6.0 + k as f64 * 6.5;
                let x = -3.0 - ph * 16.0;
                for i in 0..4 {
                    w.px_dim(x - i as f64, y, if k % 2 == 0 { ECTO } else { SHADE });
                }
            }
        }

        // Git: two wisps branch off his flame.
        if f.state == State::Git {
            for k in 0..2 {
                let side = if k == 0 { -1.0 } else { 1.0 };
                let ph = (t * 0.6 + k as f64 * 0.5) % 1.0;
                let up = ph * 22.0;
                let x = lx + side * (up - 8.0).max(0.0) * 0.7 + f.wave(6.0, k as f64) * 0.6;
                let y = ly - 6.0 - up;
                w.px(x, y, if k == 0 { FLAME } else { ECTO });
                w.px(x, y - 1.0, LIGHT);
                w.px_dim(x, y + 1.0, if k == 0 { HOT } else { ECTO });
                // The trunk and the fork, faint.
                for s in 0..8 {
                    let yy = ly - 6.0 - s as f64;
                    w.speck(lx, yy, PALE);
                }
                for s in 0..6 {
                    w.speck(lx + side * s as f64 * 0.7, ly - 14.0 - s as f64, PALE);
                }
            }
        }

        // Delegating: tiny ghostlings with tiny lanterns orbiting him.
        if f.state == State::Delegating {
            for k in 0..3 {
                let a = t * 0.9 + k as f64 * PI * 2.0 / 3.0;
                let (x, y) = (CX - 2.5 + a.cos() * 23.0, 14.0 + a.sin() * 10.0);
                ghostling(w, x, y, (t * 5.0 + k as f64) as i32 % 2 == 0, true);
            }
        }

        // A key: a spark jumping off the flame.
        if let Some(u) = f.progress(f.times.key, 0.35) {
            w.px(lx + u * 2.0, ly - 5.0 - u * 5.0, FLAME);
        }

        // "¡BU!": the shout over his head and ectoplasm bursting out.
        if let Some(u) = f.progress(f.times.done, 1.6) {
            if u < 0.85 {
                let jit = if u < 0.3 { f.wave(40.0, 0.0) * 0.6 } else { 0.0 };
                let mut x = CX - 11.0 + jit;
                let y = -13.0 - (u * 6.0).min(1.0) * 2.0;
                for (g, glyph) in GLYPHS.iter().enumerate() {
                    let gw = if g == 0 || g == 3 { 1 } else { 3 };
                    for (j, row) in glyph.iter().enumerate() {
                        for (i, ch) in row.chars().take(gw).enumerate() {
                            if ch == '#' {
                                for (a, b) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                                    let (px, py) = (x + i as f64 * 2.0 + a, y + j as f64 * 2.0 + b);
                                    w.px(px + 1.0, py + 1.0, CAP);
                                    w.px(px, py, ECTO);
                                }
                            }
                        }
                    }
                    x += gw as f64 * 2.0 + 2.0;
                }
            }
            let r = 6.0 + u * 26.0;
            for k in 0..12 {
                let a = k as f64 * PI / 6.0 + 0.2;
                let (x, y) = (CX + a.cos() * r * 1.1, p.hy + 4.0 + a.sin() * r * 0.8);
                if u > 0.6 {
                    w.px_dim(x, y, ECTO);
                } else {
                    w.px(x, y, if k % 3 == 0 { LIGHT } else { ECTO });
                }
            }
        }

        // Copying: he duplicates himself as a mirror image, a glassy twin beside him.
        if let Some(u) = f.progress(f.times.copy, 2.6) {
            let slide = if u < 0.15 {
                u / 0.15
            } else if u > 0.85 {
                (1.0 - u) / 0.15
            } else {
                1.0
            };
            let off = slide * 26.0;
            let mut tmp = Canvas::new(W, H);
            self.paint(f, &mut tmp);
            let sheen = (t * 14.0) as i32;
            for y in 0..H as i32 {
                for x in 0..W as i32 {
                    let col = tmp.get(x, y);
                    if col == 0 {
                        continue;
                    }
                    let mx = (W as i32 - 1 - x) as f64 + off;
                    if (x + y * 2 - sheen).rem_euclid(17) < 2 && matches!(col, BODY | SHADE | LIGHT) {
                        w.px(mx, y as f64, LIGHT);
                    } else if col == BODY || col == SHADE {
                        w.px(mx, y as f64, if col == BODY { SHADE } else { LINE });
                    } else {
                        w.px(mx, y as f64, col);
                    }
                }
            }
        }

        // A merge: two little wisps fuse into one with a flash.
        if let Some(u) = f.progress(f.times.merge, 1.0) {
            let d = (1.0 - (u * 1.6).min(1.0)) * 10.0;
            w.px(CX - d, -6.0, ECTO);
            w.px(CX + d, -6.0, FLAME);
            if d < 0.5 {
                for (i, j) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    w.px(CX + i * 2.0, -6.0 + j * 2.0, LIGHT);
                }
            }
        }

        // Music: the lantern throws light on the beat.
        if f.groove > 0.1 && (f.beat * PI * 2.0).sin() > 0.5 {
            for k in 0..6 {
                let a = k as f64 * PI / 3.0 + f.beat;
                w.px_dim(lx + a.cos() * 9.0, ly + a.sin() * 7.0, FLAME);
            }
        }

        // Booting: wisps gathering into him.
        if f.state == State::Booting && f.st < 1.3 {
            let r = (1.0 - f.st / 1.3) * 24.0;
            for k in 0..8 {
                let a = k as f64 * PI / 4.0 + t * 1.5;
                w.px(CX + a.cos() * r, 22.0 + a.sin() * r * 0.8, ECTO);
            }
        }

        // Offline: a thin curl of smoke off the cold lantern.
        if f.state == State::Offline {
            for k in 0..3 {
                let ph = (t * 0.5 + k as f64 / 3.0) % 1.0;
                w.px_dim(lx + (ph * 9.0 + k as f64).sin(), ly - 6.0 - ph * 9.0, PALE);
            }
        }

        // Little context left: a sweat drop.
        if f.context > 0.85 && !f.asleep() {
            let y = p.hy - 8.0 + ((t * 0.8) % 1.0) * 5.0;
            w.px(CX + 11.0, y, DROP);
            w.px(CX + 11.0, y + 1.0, DROP);
            w.px(CX + 10.0, y + 1.0, DROP);
        }
    }
}
