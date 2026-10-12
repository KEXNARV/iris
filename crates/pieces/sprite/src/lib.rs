//! A pixel-art character as a recipe part. The engine (this file and `kit.rs`) is the same for
//! all of them; each character is its own module, `src/chars/<id>.rs`, picked with
//! `IRIS_CHARACTER=<id>` when it's built (see `build.rs`). A character draws itself on a canvas
//! of pixels every frame (any pose, any frame of animation, its own face) and may add things
//! around it in the world (sparks, rain, petals…); the engine moves the whole drawing (breath,
//! hops, shakes, sways) and turns each pixel into a block of dots.
//!
//! Its cost per frame is bounded: it works on the canvas's pixels (at most 48×48), whatever the
//! size of the window; what goes around it is a handful of particles.

use iris_piece::export_part;
use iris_piece::part::*;

pub mod kit;
use kit::*;

#[allow(dead_code)]
mod chars {
    include!(concat!(env!("OUT_DIR"), "/current.rs"));
}
use chars::current::Char;

// The inputs, in the order of `INPUTS`.
const BX: usize = 0;
const BY: usize = 1;
const BR: usize = 2;
const SX: usize = 3;
const SY: usize = 4;
const GX: usize = 5;
const GY: usize = 6;
const BLINK: usize = 7;
const RED: usize = 8;
const YAWN: usize = 9;
const VOICE: usize = 10;
const GROOVE: usize = 11;
const BEAT: usize = 12;

struct Piece {
    who: Char,
    times: Times,
    seed: u64,
}

impl Part for Piece {
    const INPUTS: &'static [&'static str] =
        &["body.x", "body.y", "body.radius", "body.sx", "body.sy", "gaze.x", "gaze.y", "blink", "red", "yawn", "body.voice", "groove", "beat"];
    const SETTINGS: &'static [Setting] = &[Setting::new("size", 1.0, 0.3, 2.0)];
    const COLORS: &'static [&'static str] = <Char as Character>::COLORS;

    fn new(_io: &Io, rand: &mut Rand) -> Self {
        let seed = (rand.rand() * 1e15) as u64 | 1;
        Piece { who: Char::new(seed), times: Times::default(), seed }
    }

    fn step(&mut self, io: &mut Io) {
        let f = frame(io, &self.times, self.seed);
        self.who.step(&f);
    }

    fn event(&mut self, h: Happening, io: &mut Io) {
        let t = io.t();
        let ev = match h {
            Happening::Event(e) => Some(e),
            _ => None,
        };
        match ev {
            Some(Event::Error) => self.times.error = t,
            Some(Event::Done | Event::Pass) => self.times.done = t,
            Some(Event::Key) => self.times.key = t,
            Some(Event::Copy) => self.times.copy = t,
            Some(Event::Merge) => self.times.merge = t,
            Some(Event::Nope | Event::Cancel) => self.times.nope = t,
            _ => {}
        }
        if let Happening::Born(_) = h {
            self.times.born = t;
        }
        let f = frame(io, &self.times, self.seed);
        self.who.event(h, &f);
    }

    fn paint(&self, g: &mut Grid, io: &Io) {
        let r = io.input(BR) * io.setting(0);
        if !(r > io.du()) || !io.input(BX).is_finite() || !io.input(BY).is_finite() {
            return;
        }
        let f = frame(io, &self.times, self.seed);
        let mut canvas = Canvas::new(Char::W, Char::H);
        self.who.draw(&f, &mut canvas);
        let mut m = Motion::standard(&f);
        self.who.motion(&f, &mut m);
        let ok = |x: f64| if x.is_finite() && x > 0.1 { x } else { 1.0 };
        m.sx *= ok(io.input(SX));
        m.sy *= ok(io.input(SY));
        let place = Place::new(io, io.input(BX), io.input(BY), r, &m, Char::W, Char::H);
        if let Some(place) = place {
            place.blit(g, &canvas);
            let mut world = World { g, place: &place };
            self.who.around(&f, &mut world);
        }
    }
}

/// What a character sees each frame.
fn frame(io: &Io, times: &Times, seed: u64) -> Frame {
    let v = |i: usize| io.input(i);
    let r = v(BR).max(1e-6);
    Frame {
        t: io.t(),
        dt: io.dt().clamp(0.0, 0.1),
        state: io.state(),
        st: io.st(),
        voice: v(VOICE).max(0.0),
        music: io.music(),
        groove: v(GROOVE).clamp(0.0, 1.0),
        beat: v(BEAT).clamp(0.0, 1.0),
        gaze: (v(GX) / r, v(GY) / r),
        blink: v(BLINK) > 0.5,
        yawn: v(YAWN).clamp(0.0, 1.0),
        red: v(RED).clamp(0.0, 1.0),
        context: io.context(),
        times: *times,
        seed,
    }
}

export_part!(Piece);
