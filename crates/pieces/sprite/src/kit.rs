//! What a character works with: what it sees each frame (`Frame`), the canvas it draws on
//! (`Canvas`, with sprites, eyes, mouths and shapes), how its drawing moves (`Motion`) and the
//! world around it (`World`, for particles and props in the same pixels).

use core::f64::consts::PI;
use iris_piece::part::*;

/// When each thing last happened (seconds of `t`; far in the past if never).
#[derive(Clone, Copy)]
pub struct Times {
    pub error: f64,
    pub done: f64,
    pub key: f64,
    pub copy: f64,
    pub merge: f64,
    pub nope: f64,
    pub born: f64,
}

impl Default for Times {
    fn default() -> Self {
        Times { error: -99.0, done: -99.0, key: -99.0, copy: -99.0, merge: -99.0, nope: -99.0, born: -99.0 }
    }
}

/// What a character sees each frame.
pub struct Frame {
    pub t: f64,
    pub dt: f64,
    pub state: State,
    /// Seconds in this state.
    pub st: f64,
    /// The voice (0..1, around 0.1 when talking), the music playing, the groove and the beat.
    pub voice: f64,
    pub music: f64,
    pub groove: f64,
    pub beat: f64,
    /// Where it looks, in body radii (small: ±0.1 is a lot).
    pub gaze: (f64, f64),
    pub blink: bool,
    pub yawn: f64,
    /// The red of an error (0..1, fades).
    pub red: f64,
    /// How much of the context is used (0..1).
    pub context: f64,
    pub times: Times,
    pub seed: u64,
}

impl Frame {
    /// Seconds since `at`.
    pub fn since(&self, at: f64) -> f64 {
        self.t - at
    }

    /// Whether `at` happened less than `secs` ago.
    pub fn within(&self, at: f64, secs: f64) -> bool {
        let d = self.t - at;
        (0.0..secs).contains(&d)
    }

    /// 0..1 through the `secs` after `at` (none outside them).
    pub fn progress(&self, at: f64, secs: f64) -> Option<f64> {
        let d = (self.t - at) / secs;
        (0.0..1.0).contains(&d).then_some(d)
    }

    pub fn hurt(&self) -> bool {
        self.within(self.times.error, 1.3) || self.red > 0.5
    }

    pub fn happy(&self) -> bool {
        self.within(self.times.done, 1.4)
    }

    pub fn is(&self, states: &[State]) -> bool {
        states.contains(&self.state)
    }

    /// Reading, searching, on the web: taking things in.
    pub fn reading(&self) -> bool {
        self.is(&[State::Reading, State::Searching, State::Web])
    }

    pub fn asleep(&self) -> bool {
        self.is(&[State::Sleeping, State::Offline])
    }

    pub fn thinking(&self) -> bool {
        self.is(&[State::Thinking, State::Planning])
    }

    pub fn working(&self) -> bool {
        self.is(&[State::Running, State::Testing, State::Editing, State::Git, State::Compacting])
    }

    /// The usual expression for this moment; a character may use it or decide its own.
    pub fn eyes(&self) -> Eyes {
        if self.hurt() {
            Eyes::Cross
        } else if self.happy() {
            Eyes::Happy
        } else if self.reading() {
            Eyes::Squint
        } else if self.asleep() {
            Eyes::Asleep
        } else if self.blink || self.yawn > 0.6 {
            Eyes::Shut
        } else if self.is(&[State::Listening, State::Asking]) {
            Eyes::Wide
        } else {
            Eyes::Open
        }
    }

    /// Where the pupils go, in whole pixels (-1, 0 or 1 each way): the gaze, and up while
    /// thinking.
    pub fn look(&self) -> (i32, i32) {
        let step = |v: f64| if v > 0.04 { 1 } else if v < -0.04 { -1 } else { 0 };
        let (x, y) = (step(self.gaze.0), -step(self.gaze.1));
        if self.thinking() { (x, -1) } else { (x, y) }
    }

    /// How open the mouth wants to be (0..1): talking, listening, taking things in.
    pub fn mouth(&self) -> f64 {
        match self.state {
            State::Speaking => (self.voice * 3.0 + 0.3 * (0.5 + 0.5 * (self.t * 11.0).sin())).clamp(0.2, 1.0),
            State::Listening | State::Transcribing => 0.15 + (self.voice * 2.0).min(0.3),
            _ if self.reading() => 0.6 + 0.2 * (self.t * 6.0).sin(),
            _ if self.hurt() => 0.35,
            _ => 0.0,
        }
    }

    /// A number in 0..1 that's always the same for this `k` (and this buddy).
    pub fn rand(&self, k: u64) -> f64 {
        hash(self.seed ^ k.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    /// A wave between -1 and 1.
    pub fn wave(&self, speed: f64, phase: f64) -> f64 {
        (self.t * speed + phase).sin()
    }
}

/// A number in 0..1 out of any `u64`.
pub fn hash(mut x: u64) -> f64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Eyes {
    Open,
    /// A blink: a line.
    Shut,
    /// Asleep: a curve.
    Asleep,
    /// ^ ^
    Happy,
    /// > <
    Squint,
    /// x x
    Cross,
    /// Open, a pixel taller.
    Wide,
}

/// The character: its colors, its canvas size, and what it does.
pub trait Character: Sized {
    /// Its own colors ("#rrggbb", at most 16): pixel value `n` is `COLORS[n - 1]`.
    const COLORS: &'static [&'static str];
    /// The canvas, at most 48×48. The drawing stands on its bottom row, centered.
    const W: usize;
    const H: usize;

    fn new(seed: u64) -> Self;
    fn step(&mut self, _f: &Frame) {}
    fn event(&mut self, _h: Happening, _f: &Frame) {}
    /// Its pixels this frame.
    fn draw(&self, f: &Frame, c: &mut Canvas);
    /// How the whole drawing moves (it starts as `Motion::standard`).
    fn motion(&self, _f: &Frame, _m: &mut Motion) {}
    /// What goes around it, in the same pixels (sparks, rain, props).
    fn around(&self, _f: &Frame, _w: &mut World) {}
}

/// The pixels of a frame: 0 is empty, `n` is the character's color `n`.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: [u8; 48 * 48],
}

/// A drawing in text: one row per line, one char per pixel; `key` maps chars to colors (its
/// n-th char is color n + 1; '.' and ' ' are empty).
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub px: [u8; 48 * 48],
}

impl Sprite {
    pub fn parse(rows: &str, key: &str) -> Sprite {
        let mut s = Sprite { w: 0, h: 0, px: [0; 48 * 48] };
        for (y, row) in rows.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).enumerate().take(48) {
            for (x, ch) in row.chars().enumerate().take(48) {
                let c = key.chars().position(|k| k == ch).map_or(0, |i| i as u8 + 1);
                s.px[y * 48 + x] = c;
                s.w = s.w.max(x + 1);
            }
            s.h = y + 1;
        }
        s
    }

    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h { 0 } else { self.px[y as usize * 48 + x as usize] }
    }
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Canvas {
        Canvas { w: w.min(48), h: h.min(48), px: [0; 48 * 48] }
    }

    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h { 0 } else { self.px[y as usize * 48 + x as usize] }
    }

    /// One pixel (outside the canvas, nothing; color 0, nothing).
    pub fn set(&mut self, x: i32, y: i32, c: u8) {
        if c > 0 && x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * 48 + x as usize] = c;
        }
    }

    /// Erases a pixel.
    pub fn clear(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * 48 + x as usize] = 0;
        }
    }

    /// A sprite with its top-left at (x, y); `flip` mirrors it.
    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32, flip: bool) {
        for sy in 0..s.h as i32 {
            for sx in 0..s.w as i32 {
                let c = s.get(sx, sy);
                let dx = if flip { s.w as i32 - 1 - sx } else { sx };
                self.set(x + dx, y + sy, c);
            }
        }
    }

    /// Only the pixels of `s` whose color is `only`, recolored to `to`.
    pub fn blit_recolor(&mut self, s: &Sprite, x: i32, y: i32, map: &[(u8, u8)]) {
        for sy in 0..s.h as i32 {
            for sx in 0..s.w as i32 {
                let c = s.get(sx, sy);
                let c = map.iter().find(|m| m.0 == c).map_or(c, |m| m.1);
                self.set(x + sx, y + sy, c);
            }
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// Replaces one color with another inside a box.
    pub fn recolor(&mut self, x: i32, y: i32, w: i32, h: i32, from: u8, to: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                if self.get(xx, yy) == from {
                    self.set(xx, yy, to);
                }
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.set(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// A filled ellipse centered at (cx, cy) with radii (rx, ry), in pixels (halves allowed).
    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, c: u8) {
        let (x0, x1) = ((cx - rx).floor() as i32, (cx + rx).ceil() as i32);
        let (y0, y1) = ((cy - ry).floor() as i32, (cy + ry).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = ((x as f64 + 0.5 - cx) / rx.max(0.5), (y as f64 + 0.5 - cy) / ry.max(0.5));
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y, c);
                }
            }
        }
    }

    /// A filled triangle.
    pub fn triangle(&mut self, a: (f64, f64), b: (f64, f64), c: (f64, f64), col: u8) {
        let (x0, x1) = (a.0.min(b.0).min(c.0).floor() as i32, a.0.max(b.0).max(c.0).ceil() as i32);
        let (y0, y1) = (a.1.min(b.1).min(c.1).floor() as i32, a.1.max(b.1).max(c.1).ceil() as i32);
        let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = (x as f64 + 0.5, y as f64 + 0.5);
                let (d1, d2, d3) = (side(a, b, p), side(b, c, p), side(c, a, p));
                let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
                let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
                if !(neg && pos) {
                    self.set(x, y, col);
                }
            }
        }
    }

    /// A filled quadrilateral (two triangles).
    pub fn quad(&mut self, a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64), col: u8) {
        self.triangle(a, b, c, col);
        self.triangle(a, c, d, col);
    }

    /// A line `width` pixels thick (a run of discs along it).
    pub fn thick(&mut self, a: (f64, f64), b: (f64, f64), width: f64, col: u8) {
        let steps = ((b.0 - a.0).hypot(b.1 - a.1) * 2.0).ceil().max(1.0) as i32;
        for k in 0..=steps {
            let u = k as f64 / steps as f64;
            self.ellipse(a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u, width / 2.0, width / 2.0, col);
        }
    }

    /// The outline: every empty pixel next to a drawn one (up, down, left, right) takes `col`.
    /// Pixels of a color in `skip` don't count as drawn (so the outline doesn't hug them).
    pub fn outline(&mut self, col: u8, skip: &[u8]) {
        let old = self.px;
        let (w, h) = (self.w, self.h);
        let drawn = |x: i32, y: i32| {
            if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                return false;
            }
            let c = old[y as usize * 48 + x as usize];
            c > 0 && c != col && !skip.contains(&c)
        };
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if old[y as usize * 48 + x as usize] == 0 && (drawn(x - 1, y) || drawn(x + 1, y) || drawn(x, y - 1) || drawn(x, y + 1)) {
                    self.set(x, y, col);
                }
            }
        }
    }

    /// Shading: pixels of `base` with the outline right of them or below them take `shade`
    /// (light from the top left).
    pub fn shade(&mut self, base: u8, shade: u8, line: u8) {
        let old = self.px;
        let (w, h) = (self.w, self.h);
        let at = |x: i32, y: i32| if x < 0 || y < 0 || x as usize >= w || y as usize >= h { 0 } else { old[y as usize * 48 + x as usize] };
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if at(x, y) == base && (at(x + 1, y) == line || at(x, y + 1) == line || at(x + 1, y + 1) == line) {
                    self.set(x, y, shade);
                }
            }
        }
    }

    /// A highlight: pixels of `base` with the outline left of them or above them take `light`.
    pub fn light(&mut self, base: u8, light: u8, line: u8) {
        let old = self.px;
        let (w, h) = (self.w, self.h);
        let at = |x: i32, y: i32| if x < 0 || y < 0 || x as usize >= w || y as usize >= h { 0 } else { old[y as usize * 48 + x as usize] };
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if at(x, y) == base && (at(x - 1, y) == line || at(x, y - 1) == line) && at(x + 1, y) != line {
                    self.set(x, y, light);
                }
            }
        }
    }

    /// Every pixel of `from` becomes `to`, the whole canvas.
    pub fn swap(&mut self, from: u8, to: u8) {
        for p in self.px.iter_mut() {
            if *p == from {
                *p = to;
            }
        }
    }

    /// An eye in a `w`×`h` box at (x, y): the expression, the pupils looking `look`, `ink` for
    /// the eye and `shine` for its glint (0: none). `skin` paints the box first (0: leave it).
    #[allow(clippy::too_many_arguments)]
    pub fn eye(&mut self, x: i32, y: i32, w: i32, h: i32, e: Eyes, look: (i32, i32), ink: u8, shine: u8, skin: u8, right: bool) {
        if skin > 0 {
            self.rect(x, y, w, h, skin);
        }
        let bottom = y + h - 1;
        match e {
            Eyes::Open | Eyes::Wide => {
                let (lx, ly) = if w >= 2 { look } else { (0, look.1) };
                let top = if e == Eyes::Wide { y - 1 } else { y };
                for yy in top..y + h {
                    for xx in x..x + w {
                        self.set(xx + lx, yy + ly, ink);
                    }
                }
                if shine > 0 && w >= 2 && h >= 2 {
                    self.set(x + lx + if right { w - 1 } else { 0 }, top + ly, shine);
                }
            }
            Eyes::Shut => {
                for xx in x..x + w {
                    self.set(xx, bottom, ink);
                }
            }
            Eyes::Asleep => {
                for xx in x..x + w {
                    self.set(xx, bottom, ink);
                }
                if w >= 3 {
                    self.set(x - 1, bottom - 1, ink);
                    self.set(x + w, bottom - 1, ink);
                } else {
                    self.set(x - 1, bottom - 1, ink);
                }
            }
            Eyes::Happy => {
                let peak = y + (h - 1) / 2;
                let wid = w.max(2);
                for k in 0..=wid / 2 + 1 {
                    self.set(x + (wid - 1) / 2 - k, peak + k, ink);
                    self.set(x + wid / 2 + k, peak + k, ink);
                }
            }
            Eyes::Squint => {
                let mid = y + h / 2;
                let half = (h.max(3) / 2).max(1);
                for k in 0..=half {
                    let xx = if right { x + w.max(2) - 1 - k } else { x + k };
                    self.set(xx, mid - half + k, ink);
                    self.set(xx, mid + half - k, ink);
                }
            }
            Eyes::Cross => {
                let s = w.max(h).max(3);
                for k in 0..s {
                    self.set(x + k * (w.max(3) - 1) / (s - 1), y + k * (h.max(3) - 1) / (s - 1), ink);
                    self.set(x + w.max(3) - 1 - k * (w.max(3) - 1) / (s - 1), y + k * (h.max(3) - 1) / (s - 1), ink);
                }
            }
        }
    }

    /// A mouth `open` (0..1) at (cx, cy), up to `max_w` × `max_h` pixels: `inside` its dark,
    /// `tongue` its bottom (0: none).
    pub fn mouth(&mut self, cx: i32, cy: i32, open: f64, max_w: i32, max_h: i32, inside: u8, tongue: u8) {
        if open < 0.08 {
            return;
        }
        let rx = 0.6 + open * (max_w as f64 / 2.0 - 0.6).max(0.0);
        let ry = 0.5 + open * (max_h as f64 / 2.0 - 0.5).max(0.0);
        let cyf = cy as f64 + ry - 0.5;
        self.ellipse(cx as f64 + 0.5, cyf, rx, ry, inside);
        if tongue > 0 && ry >= 1.5 {
            self.ellipse(cx as f64 + 0.5, cyf + ry * 0.55, rx * 0.6, ry * 0.4, tongue);
        }
    }

    /// A smile ‿ `w` pixels wide, its middle at (cx, cy).
    pub fn smile(&mut self, cx: i32, cy: i32, w: i32, c: u8) {
        let half = w / 2;
        for k in -half..=half {
            let dy = if k.abs() == half { 0 } else { 1 };
            self.set(cx + k, cy + dy, c);
        }
    }

    /// Every pixel of one color, moved by (dx, dy): an arm raised, an ear down.
    pub fn shift_color(&mut self, c: u8, dx: i32, dy: i32) {
        let old = self.px;
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if old[y as usize * 48 + x as usize] == c {
                    self.clear(x, y);
                }
            }
        }
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if old[y as usize * 48 + x as usize] == c {
                    self.set(x + dx, y + dy, c);
                }
            }
        }
    }
}

/// How the whole drawing moves this frame, in body radii and scales.
#[derive(Clone, Copy)]
pub struct Motion {
    pub dx: f64,
    pub dy: f64,
    pub sx: f64,
    pub sy: f64,
    /// How far its top leans sideways (its bottom stays), in radii.
    pub lean: f64,
}

fn ease_out(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}

impl Motion {
    /// What everyone does: breathing, a hop when done, a shake on errors and on "no", a tap per
    /// key, running in place, a bounce when asking, swaying to the music, arriving from above
    /// when Iris boots, squashed while compacting.
    pub fn standard(f: &Frame) -> Motion {
        let t = f.t;
        let breath = if f.asleep() { 0.04 * (t * 0.9).sin() } else { 0.022 * (t * 1.7).sin() };
        let hop = f.progress(f.times.done, 0.6).map_or(0.0, |u| (PI * u).sin() * 0.35);
        let land = f.progress(f.times.done + 0.6, 0.2).map_or(0.0, |u| (PI * u).sin() * 0.1);
        let shake = f.progress(f.times.error, 0.5).map_or(0.0, |u| 0.05 * (t * 60.0).sin() * (1.0 - u))
            + f.progress(f.times.nope, 0.45).map_or(0.0, |u| 0.06 * (t * 28.0).sin() * (1.0 - u));
        let tap = if f.within(f.times.key, 0.15) { 0.03 } else { 0.0 };
        let run = if f.is(&[State::Running, State::Testing]) { (t * 12.0).sin().abs() * 0.07 } else { 0.0 };
        let ask = if f.state == State::Asking { (t * 5.0).sin().abs() * 0.05 } else { 0.0 };
        let press = if f.state == State::Compacting { 0.5 + 0.5 * (t * 3.0).sin() } else { 0.0 };
        let drop = if f.state == State::Booting { (1.0 - ease_out((f.st / 1.2).min(1.0))) * 3.0 } else { 0.0 };
        let lean = f.groove * 0.12 * (t * PI * 2.0 * 0.55).sin() + if f.state == State::Asking { 0.06 } else { 0.0 };
        Motion {
            dx: shake,
            dy: hop + run + ask + tap + drop,
            sx: 1.0 + land + press * 0.12,
            sy: 1.0 + breath - land - press * 0.22,
            lean,
        }
    }
}

/// Where the canvas goes on the grid this frame.
pub struct Place {
    /// The world point of the canvas's (0, 0) corner's... see `rect`.
    cx: f64,
    foot: f64,
    pw: f64,
    ph: f64,
    lean: f64,
    w: usize,
    h: usize,
    x0: f64,
    y0: f64,
    di: f64,
    dj: f64,
    dw: usize,
    dh: usize,
    /// More dots than one frame can send: blocks go checkered.
    checker: bool,
    pub du: f64,
}

/// Past this many dots in a frame, a pixel's block is drawn checkered.
const MAX_DOTS: usize = 15_000;

impl Place {
    pub fn new(io: &Io, bx: f64, by: f64, r: f64, m: &Motion, w: usize, h: usize) -> Option<Place> {
        let (dw, dh) = io.grid_size();
        if dw < 2 || dh < 2 {
            return None;
        }
        let (x0, y0) = io.center(0, 0);
        let (x1, y1) = io.center(1, 1);
        let (di, dj) = (x1 - x0, y1 - y0);
        if di == 0.0 || dj == 0.0 {
            return None;
        }
        // The drawing is 2.2 radii tall (or wide, whichever is bigger), standing 1.1 under.
        let ps = 2.2 * r / w.max(h) as f64;
        let (pw, ph) = (ps * m.sx, ps * m.sy);
        let per = ((pw / di.abs()).max(1.0) * (ph / dj.abs()).max(1.0)) as usize;
        Some(Place {
            cx: bx + m.dx * r,
            foot: by - 1.1 * r + m.dy * r,
            pw,
            ph,
            lean: m.lean * r,
            w,
            h,
            x0,
            y0,
            di,
            dj,
            dw,
            dh,
            checker: w * h * per / 2 > MAX_DOTS,
            du: io.du(),
        })
    }

    /// The world box of canvas pixel (x, y) (it may be outside the canvas): left, top, right,
    /// bottom.
    fn rect(&self, x: f64, y: f64) -> (f64, f64, f64, f64) {
        let height = self.h as f64 - y;
        let top = self.foot + height * self.ph;
        let shift = self.lean * height / self.h as f64;
        let left = self.cx + (x - self.w as f64 / 2.0) * self.pw + shift;
        (left, top, left + self.pw, top - self.ph)
    }

    /// One pixel's block of dots, in a color (`ink` is already a grid ink).
    fn fill(&self, g: &mut Grid, x: f64, y: f64, tone: u8, ink: u8) {
        let (left, top, right, bottom) = self.rect(x, y);
        let (ia, ib) = (((left - self.x0) / self.di).ceil(), ((right - self.x0) / self.di).floor());
        let (ja, jb) = (((top - self.y0) / self.dj).ceil(), ((bottom - self.y0) / self.dj).floor());
        if ib < ia || jb < ja {
            g.plot((left + right) / 2.0, (top + bottom) / 2.0, tone, ink);
            return;
        }
        if ib < 0.0 || jb < 0.0 || ia >= self.dw as f64 || ja >= self.dh as f64 {
            return;
        }
        let (ia, ib) = (ia.max(0.0) as usize, ib.min(self.dw as f64 - 1.0) as usize);
        let (ja, jb) = (ja.max(0.0) as usize, jb.min(self.dh as f64 - 1.0) as usize);
        for j in ja..=jb {
            for i in ia..=ib {
                if !self.checker || (i + j) % 2 == 0 {
                    g.dot(i, j, tone, ink);
                }
            }
        }
    }

    pub fn blit(&self, g: &mut Grid, c: &Canvas) {
        for y in 0..c.h {
            for x in 0..c.w {
                let p = c.px[y * 48 + x];
                if p > 0 {
                    self.fill(g, x as f64, y as f64, 3, own(p - 1));
                }
            }
        }
    }
}

/// The world around the character, in the canvas's pixels (outside the canvas too: negative,
/// or past its size), for what isn't part of its body.
pub struct World<'a> {
    pub g: &'a mut Grid,
    pub place: &'a Place,
}

impl World<'_> {
    /// A pixel of the character's color `c` at canvas coordinates (x, y), whole or halves.
    pub fn px(&mut self, x: f64, y: f64, c: u8) {
        if c > 0 {
            self.place.fill(self.g, x, y, 3, own(c - 1));
        }
    }

    /// The same, dimmer (tone 2: it reads as further away, or fading).
    pub fn px_dim(&mut self, x: f64, y: f64, c: u8) {
        if c > 0 {
            self.place.fill(self.g, x, y, 2, own(c - 1));
        }
    }

    /// A pixel in the state's color (the one that changes with what Iris is doing).
    pub fn px_state(&mut self, x: f64, y: f64) {
        self.place.fill(self.g, x, y, 3, INK_MAIN);
    }

    /// A sprite in the world, its top-left at (x, y) (outside the canvas is fine).
    pub fn sprite(&mut self, s: &Sprite, x: f64, y: f64) {
        for sy in 0..s.h as i32 {
            for sx in 0..s.w as i32 {
                let c = s.get(sx, sy);
                if c > 0 {
                    self.px(x + sx as f64, y + sy as f64, c);
                }
            }
        }
    }

    /// A small dot, finer than a pixel (one grid dot), at canvas coordinates.
    pub fn speck(&mut self, x: f64, y: f64, c: u8) {
        let (l, t, r, b) = self.place.rect(x, y);
        if c > 0 {
            self.g.plot((l + r) / 2.0, (t + b) / 2.0, 3, own(c - 1));
        }
    }
}
