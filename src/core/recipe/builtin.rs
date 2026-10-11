//! The built-in parts: each piece of `core::pieces` behind the `Part` contract, plus the ones
//! that used to be the original buddy's insides (the script follower, the spring, the flashes,
//! the rhythm, the scene). Each one reads its inputs from the bus by name (`Build::input`, which
//! honors the recipe's wiring) and publishes what it knows under its own names.
//!
//! The arithmetic is exactly the one the original buddy did, in the same order: the golden test
//! (`core::tests::golden`) checks that not a single dot changed.

use std::collections::HashMap;

use super::bus::{Bus, Ch};
use super::part::{Frame, Msg, Part, World};
use super::Build;
use crate::core::color::{mix, tones, INK_ACCENT, INK_MAIN, INK_RED, RED};
use crate::core::context::{Anchor, Context, Notice};
use crate::core::grid::Grid;
use crate::core::pieces::arcs::{self, Arcs};
use crate::core::pieces::body::{self, Blob, Shape};
use crate::core::pieces::eye::{self, Eye, Pupil};
use crate::core::pieces::gaze::{self, blink, Gaze};
use crate::core::pieces::gestures::{self, Gesture, Gestures};
use crate::core::pieces::git::{self, Git};
use crate::core::pieces::ideas::{self, Ideas};
use crate::core::pieces::kids::{self, Kids};
use crate::core::pieces::liquid::{Level, Liquid};
use crate::core::pieces::magnifier::{self, Glass, Magnifier, Spot};
use crate::core::pieces::marks::{Tail, Tasks};
use crate::core::pieces::packages::{self, Packages};
use crate::core::pieces::particles::{self, Motes};
use crate::core::pieces::presses::{self, Presses};
use crate::core::pieces::question::{self, QuestionMark};
use crate::core::pieces::radar::{self, Radar};
use crate::core::pieces::reading::{ReadingBar, ReadingLine};
use crate::core::pieces::scale::{self, Scale};
use crate::core::pieces::spring::Spring;
use crate::core::pieces::sprout::{self, Sprout};
use crate::core::pieces::stitches::{self, Stitches};
use crate::core::pieces::voice::{self, Voice};
use crate::core::pieces::waves::Waves;
use crate::core::pieces::zees::{self, Zees};
use crate::core::pieces::{ease_out_back, Fill, Piece};
use crate::core::state::{Event, State, ALL};

/// Builds the part a recipe asks for.
pub(crate) fn make(b: &mut Build) -> Result<Box<dyn Part>, String> {
    let uses = b.spec.uses.clone();
    if let Some(path) = uses.strip_prefix("file:") {
        let wasm = read(b, path)?;
        return wasm_part(b, &wasm).map_err(|e| format!("«{path}»: {e}"));
    }
    if let Some(path) = uses.strip_prefix("shape:") {
        let src = String::from_utf8(read(b, path)?).map_err(|_| format!("«{path}» no es texto"))?;
        return Ok(Box::new(super::shape::ShapePart::new(&src, b).map_err(|e| format!("«{path}»: {e}"))?));
    }
    if let Some((name, major)) = uses.split_once('@') {
        let major: u64 = major.parse().map_err(|_| format!("«{uses}»: después de @ va la versión mayor (kirby@1)"))?;
        let files = b.recipe.files.clone().ok_or(format!("«{uses}»: esta receta no puede usar piezas de la tienda"))?;
        let (format, code) = files.store(name, major)?;
        return match format.as_str() {
            "wasm" => wasm_part(b, &code).map_err(|e| format!("«{uses}»: {e}")),
            _ => {
                let src = String::from_utf8(code).map_err(|_| format!("«{uses}» no es texto"))?;
                Ok(Box::new(super::shape::ShapePart::new(&src, b).map_err(|e| format!("«{uses}»: {e}"))?))
            }
        };
    }
    let name = b.spec.uses.strip_prefix("builtin:").ok_or_else(|| format!("«{}»: no sé de dónde sacar esa pieza", b.spec.uses))?;
    Ok(match name {
        "script" => Box::new(ScriptPart::new(b)?),
        "spring" => Box::new(SpringPart::new(b)),
        "flashes" => Box::new(Flashes::new(b)),
        "waves" => Box::new(WavesPart::new(b)),
        "scale" => Box::new(ScalePart::new(b)),
        "kids" => Box::new(KidsPart::new(b)),
        "rhythm" => Box::new(Rhythm::new(b)),
        "gestures" => Box::new(GesturesPart::new(b)),
        "gaze" => Box::new(GazePart::new(b)),
        "motes" => Box::new(MotesPart::new(b)),
        "presence" => Box::new(Presence::new(b)),
        "magnifier" => Box::new(MagnifierPart::new(b)),
        "eye" => Box::new(EyePart::new(b)),
        "reading" => Box::new(ReadingPart::new(b)),
        "sprout" => Box::new(SproutPart::new(b)),
        "liquid" => Box::new(LiquidPart::new(b)),
        "blob" => Box::new(BlobPart::new(b)),
        "baymax-face" => Box::new(FacePart::new(b)),
        "tasks" => Box::new(TasksPart { tasks: Tasks::new(), plan: b.input("plan") }),
        "arcs" => Box::new(ArcsPart::new(b)?),
        "tail" => Box::new(TailPart { tail: Tail::new(), anchor: AnchorIn::new(b) }),
        "presses" => Box::new(Simple::new(b, Presses::new(), &["press"], |p, g, ctx, a, v| p.paint(g, ctx, presses::Paint { anchor: a, press: v[0] }))),
        "voice" => Box::new(Simple::new(b, Voice::new(), &["ripple"], |p, g, ctx, a, v| p.paint(g, ctx, voice::Paint { anchor: a, ripple: v[0] }))),
        "radar" => Box::new(Simple::new(b, Radar::new(), &["sweep", "arc_phase"], |p, g, ctx, a, v| {
            p.paint(g, ctx, radar::Paint { anchor: a, sweep: v[0], spin: v[1] })
        })),
        "ideas" => Box::new(Simple::new(b, Ideas::new(), &["orbit", "amp", "phase", "low"], |p, g, ctx, a, v| {
            p.paint(g, ctx, ideas::Paint { anchor: a, orbit: v[0], amp: v[1], phase: v[2], low: v[3] > 0.5 })
        })),
        "packages" => Box::new(Simple::new(b, Packages::new(), &["packets", "low"], |p, g, ctx, a, v| {
            p.paint(g, ctx, packages::Paint { anchor: a, packets: v[0], low: v[1] > 0.5 })
        })),
        "stitches" => Box::new(Simple::new(b, Stitches::new(), &["stitch"], |p, g, ctx, a, v| p.paint(g, ctx, stitches::Paint { anchor: a, stitch: v[0] }))),
        "git" => Box::new(Simple::new(b, Git::new(), &["git"], |p, g, ctx, a, v| p.paint(g, ctx, git::Paint { anchor: a, git: v[0] }))),
        "question" => Box::new(Simple::new(b, QuestionMark::new(), &["droop"], |p, g, ctx, a, v| p.paint(g, ctx, question::Paint { anchor: a, droop: v[0] }))),
        "zees" => Box::new(Simple::new(b, Zees::new(), &["zzz", "low"], |p, g, ctx, a, v| p.paint(g, ctx, zees::Paint { anchor: a, zzz: v[0], low: v[1] > 0.5 }))),
        other => return Err(format!("no hay ninguna pieza de fábrica «{other}»")),
    })
}

/// A file the recipe names: next to it, or anywhere by its absolute path.
fn read(b: &Build, path: &str) -> Result<Vec<u8>, String> {
    match &b.recipe.files {
        Some(f) => f.get(path),
        None => std::fs::read(path).map_err(|e| format!("«{path}»: {e}")),
    }
}

#[cfg(feature = "pieces")]
fn wasm_part(b: &mut Build, wasm: &[u8]) -> Result<Box<dyn Part>, String> {
    Ok(Box::new(super::wasm::WasmPart::load(wasm, b)?))
}

#[cfg(not(feature = "pieces"))]
fn wasm_part(_b: &mut Build, _wasm: &[u8]) -> Result<Box<dyn Part>, String> {
    Err("este Iris no carga piezas en WebAssembly".into())
}

/// Where the body is, read from the bus (what `presence` publishes).
struct AnchorIn {
    x: Ch,
    y: Ch,
    radius: Ch,
    base: Ch,
    ink: Ch,
    border: Ch,
}

impl AnchorIn {
    fn new(b: &mut Build) -> Self {
        AnchorIn {
            x: b.input("body.x"),
            y: b.input("body.y"),
            radius: b.input("body.radius"),
            base: b.input("body.base"),
            ink: b.input("ink.body"),
            border: b.input("ink.ring"),
        }
    }

    fn get(&self, bus: &Bus) -> Anchor {
        Anchor {
            cx: bus.get(self.x),
            cy: bus.get(self.y),
            radius: bus.get(self.radius),
            base: bus.get(self.base),
            ink: bus.get(self.ink) as u8,
            border: bus.get(self.border) as u8,
        }
    }
}

/// The spring as the pieces know it, loaded from the bus and saved back after they push it.
struct SpringIn {
    sq: Ch,
    v: Ch,
}

impl SpringIn {
    fn new(b: &mut Build) -> Self {
        SpringIn { sq: b.input("spring.sq"), v: b.input("spring.v") }
    }

    fn with<T>(&self, bus: &mut Bus, f: impl FnOnce(&mut Spring) -> T) -> T {
        let mut s = Spring::at(bus.get(self.sq), bus.get(self.v));
        let out = f(&mut s);
        bus.set(self.sq, s.sq);
        bus.set(self.v, s.v());
        out
    }

    /// A push; with `/calm`, gentler.
    fn kick(&self, w: &mut World, v: f64) {
        w.bus.add(self.v, v * w.ctx.calm());
    }
}

// ─── The script, the spring, the flashes, the rhythm ─────────────────────────────────────────

/// Follows the script: each channel eases toward the value the state gives it, and the color
/// toward the state's (toward red after an error).
struct ScriptPart {
    names: Vec<Ch>,
    table: HashMap<State, Vec<f64>>,
    /// With kids, the main one steps aside: these channels are multiplied by these factors.
    with_kids: Vec<(usize, f64)>,
    kids: Ch,
    red: Ch,
    col: [Ch; 3],
}

impl ScriptPart {
    fn new(b: &mut Build) -> Result<Self, String> {
        let names_s = b.recipe.script_channels();
        let names: Vec<Ch> = names_s.iter().map(|n| b.bus.ch(n)).collect();
        let mut table: HashMap<State, Vec<f64>> = HashMap::new();
        for s in ALL {
            let vals = b.recipe.script_for(s);
            table.insert(s, names_s.iter().map(|n| vals.iter().find(|(k, _)| k == n).map_or(0.0, |v| v.1)).collect());
        }
        let mut with_kids = vec![];
        if let Some(t) = b.setting("with_kids").and_then(|t| t.as_table()) {
            for (k, v) in t {
                let i = names_s.iter().position(|n| n == k).ok_or(format!("with_kids: «{k}» no está en el guion"))?;
                with_kids.push((i, v.as_float().ok_or(format!("with_kids: «{k}» no es un número"))?));
            }
        }
        let col = [b.bus.ch("col.r"), b.bus.ch("col.g"), b.bus.ch("col.b")];
        for (c, v) in names.iter().zip(&table[&b.state]) {
            b.bus.set(*c, *v);
        }
        for (c, v) in col.iter().zip(b.state.rgb()) {
            b.bus.set(*c, v);
        }
        Ok(ScriptPart { names, table, with_kids, kids: b.input("kids.alive"), red: b.input("red"), col })
    }
}

impl Part for ScriptPart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let ctx = w.ctx;
        let k = 1.0 - (-dt / 0.35).exp();
        let mut target = self.table[&ctx.state].clone();
        if w.bus.on(self.kids) {
            for &(i, m) in &self.with_kids {
                target[i] *= m;
            }
        }
        for (c, to) in self.names.iter().zip(&target) {
            let p = w.bus.get(*c);
            w.bus.set(*c, p + (to - p) * k);
        }
        let red = w.bus.get(self.red);
        let base = ctx.state.rgb();
        let want_col = if red > 0.05 { mix(base, RED, (red * 1.4).min(1.0)) } else { base };
        let kc = if red > 0.05 { 0.5 } else { k };
        for i in 0..3 {
            let c = w.bus.get(self.col[i]);
            w.bus.set(self.col[i], c + (want_col[i] - c) * kc);
        }
    }
}

/// The elastic squash: everything that happens pushes it (adding to `spring.v`).
struct SpringPart {
    spring: SpringIn,
}

impl SpringPart {
    fn new(b: &mut Build) -> Self {
        SpringPart { spring: SpringIn::new(b) }
    }
}

impl Part for SpringPart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let ctx = w.ctx;
        self.spring.with(w.bus, |s| s.step(ctx, (), dt));
    }
}

/// What events and changes leave on the body: flashes that fade, and pushes.
struct Flashes {
    spring: SpringIn,
    red: Ch,
    nope: Ch,
    shake: Ch,
    deflate: Ch,
    copy: Ch,
    pass: Ch,
    swallow: Ch,
}

impl Flashes {
    fn new(b: &mut Build) -> Self {
        Flashes {
            spring: SpringIn::new(b),
            red: b.bus.ch("red"),
            nope: b.bus.ch("nope"),
            shake: b.bus.ch("shake"),
            deflate: b.bus.ch("deflate"),
            copy: b.bus.ch("copy"),
            pass: b.bus.ch("pass"),
            swallow: b.bus.ch("swallow"),
        }
    }
}

impl Part for Flashes {
    fn step(&mut self, w: &mut World, dt: f64) {
        let b = &mut *w.bus;
        for (c, rate) in [(self.red, 1.1), (self.nope, 1.1), (self.shake, 1.6), (self.deflate, 1.4), (self.copy, 3.0), (self.pass, 1.2), (self.swallow, 1.5)] {
            b.set(c, (b.get(c) - dt * rate).max(0.0));
        }
    }

    /// Transitions with intent: every change has its gesture.
    fn change(&mut self, w: &mut World, from: State, to: State) {
        use State::*;
        match (from, to) {
            (_, Offline) => self.spring.kick(w, -1.5),
            (Sleeping, _) => {
                self.spring.kick(w, 3.0);
                w.bus.set(self.copy, 0.5); // blinks on waking up
            }
            (Listening | NoVoice, Transcribing) => w.bus.set(self.swallow, 1.0),
            (f, Thinking) if f.is_tool() => self.spring.kick(w, -0.8),
            (_, t) if t.is_tool() => self.spring.kick(w, 1.2),
            _ => self.spring.kick(w, 1.0),
        }
    }

    fn notice(&mut self, w: &mut World, n: Notice) {
        match n {
            Notice::Event(ev) => match ev {
                Event::Key => self.spring.kick(w, 1.2),
                Event::Error => {
                    w.bus.set(self.red, 1.0);
                    w.bus.set(self.shake, if w.ctx.sig.calm { 0.0 } else { 1.0 });
                    self.spring.kick(w, -2.5);
                }
                Event::Done => self.spring.kick(w, 3.2),
                Event::Cancel => {
                    w.bus.set(self.deflate, 1.0);
                    self.spring.kick(w, -3.0);
                }
                Event::Nope => w.bus.set(self.nope, 1.0),
                Event::Merge => {}
                Event::Copy => {
                    w.bus.set(self.copy, 1.0);
                    self.spring.kick(w, 0.8);
                }
                Event::Pass => {
                    w.bus.set(self.pass, 1.0);
                    self.spring.kick(w, 1.5);
                }
            },
            // A subagent is born: a push.
            Notice::Born(_) => self.spring.kick(w, 1.6),
            _ => {}
        }
    }
}

/// The phases advance, and the outside comes in: the voice and the music.
struct Rhythm {
    spring: SpringIn,
    phase: Ch,
    arc_phase: Ch,
    wob: Ch,
    arc_speed: Ch,
    level: Ch,
    music: Ch,
    groove: Ch,
}

impl Rhythm {
    fn new(b: &mut Build) -> Self {
        Rhythm {
            spring: SpringIn::new(b),
            phase: b.bus.ch("phase"),
            arc_phase: b.bus.ch("arc_phase"),
            wob: b.input("wob"),
            arc_speed: b.input("arc_speed"),
            level: b.bus.ch("level"),
            music: b.bus.ch("music"),
            groove: b.bus.ch("groove"),
        }
    }
}

impl Part for Rhythm {
    fn step(&mut self, w: &mut World, dt: f64) {
        let ctx = w.ctx;
        let calm = ctx.calm();
        let b = &mut *w.bus;
        b.set(self.phase, b.get(self.phase) + dt * b.get(self.wob) * calm);
        b.set(self.arc_phase, b.get(self.arc_phase) + dt * b.get(self.arc_speed) * calm);

        let lvl = if matches!(ctx.state, State::Listening | State::Speaking) { (ctx.sig.level as f64 * 6.0).min(1.0) } else { 0.0 };
        let level = b.get(self.level);
        b.set(self.level, level + (lvl - level) * if lvl > level { 0.5 } else { 0.12 });
        // Music: cava already comes smoothed, so it is followed almost as is. Like the keyboard,
        // it shows up in half a second and a key (or any other state) turns it off instantly.
        let m = ctx.sig.music as f64;
        let (music, groove) = (b.get(self.music), b.get(self.groove));
        // A beat (the level jumps suddenly) squashes it a little: that's how you see it dance.
        if m - music > 0.15 {
            self.spring.kick(w, 1.5 * groove);
        }
        let b = &mut *w.bus;
        b.set(self.music, music + (m - music) * (dt * 30.0).min(1.0));
        b.set(
            self.groove,
            if ctx.state != State::Idle {
                (groove - dt * 8.0).max(0.0)
            } else if m > 0.02 {
                (groove + dt / 0.5).min(1.0)
            } else {
                (groove - dt).max(0.0) // between songs it fades out slowly
            },
        );
    }
}

// ─── Pieces that move ────────────────────────────────────────────────────────────────────────

struct WavesPart {
    waves: Waves,
    spring: SpringIn,
    anchor: AnchorIn,
}

impl WavesPart {
    fn new(b: &mut Build) -> Self {
        WavesPart { waves: Waves::new(), spring: SpringIn::new(b), anchor: AnchorIn::new(b) }
    }
}

impl Part for WavesPart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let (ctx, waves) = (w.ctx, &mut self.waves);
        self.spring.with(w.bus, |s| waves.step(ctx, s, dt));
    }

    fn notice(&mut self, w: &mut World, n: Notice) {
        if let Notice::Event(ev) = n {
            self.waves.event(ev, w.ctx, w.rand);
        }
    }

    fn message(&mut self, _w: &mut World, m: Msg) {
        let Msg::Ring(t) = m;
        self.waves.done(t);
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        self.waves.paint(g, f.ctx, &self.anchor.get(&f.bus));
    }

    #[cfg(test)]
    fn rings(&self) -> usize {
        self.waves.dones.len()
    }
}

/// The scale, with its own list of green rings (end of turn, a subagent coming back).
struct ScalePart {
    scale: Scale,
    dones: Vec<f64>,
    anchor: AnchorIn,
    v: [Ch; 14],
}

impl ScalePart {
    fn new(b: &mut Build) -> Self {
        let names = ["boot.t", "level", "arc_phase", "music", "groove", "pass", "copy", "red", "low", "vu", "fill", "sweep", "cardinal", "tests"];
        let mut v = [Ch::default(); 14];
        for (i, n) in names.iter().enumerate() {
            v[i] = b.input(n);
        }
        ScalePart { scale: Scale::new(), dones: vec![], anchor: AnchorIn::new(b), v }
    }
}

impl Part for ScalePart {
    fn step(&mut self, w: &mut World, _dt: f64) {
        let t = w.ctx.t;
        self.dones.retain(|b| t - b < 1.2);
    }

    fn notice(&mut self, w: &mut World, n: Notice) {
        if let Notice::Event(Event::Done) = n {
            self.dones.push(w.ctx.t);
        }
    }

    fn message(&mut self, _w: &mut World, m: Msg) {
        let Msg::Ring(t) = m;
        self.dones.push(t);
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        let b = &f.bus;
        let v = |i: usize| b.get(self.v[i]);
        let anchor = self.anchor.get(b);
        self.scale.paint(
            g,
            f.ctx,
            scale::Paint {
                anchor: &anchor,
                boot: v(0),
                level: v(1),
                spin: v(2),
                music: v(3),
                groove: v(4),
                dones: &self.dones,
                pass: v(5),
                copy: v(6),
                red: v(7),
                low: v(8) > 0.5,
                vu: v(9),
                fill: v(10),
                sweep: v(11),
                cardinal: v(12),
                tests: v(13),
            },
        );
    }
}

/// The subagents: each one a small core of its own, in orbit.
struct KidsPart {
    kids: Kids,
    spring: SpringIn,
    anchor: AnchorIn,
    alive: Ch,
    heard_t: Ch,
    heard_a: Ch,
}

impl KidsPart {
    fn new(b: &mut Build) -> Self {
        let scale = b.number("scale", 0.40);
        let p = KidsPart {
            kids: Kids::new(scale),
            spring: SpringIn::new(b),
            anchor: AnchorIn::new(b),
            alive: b.bus.ch("kids.alive"),
            heard_t: b.bus.ch("heard.t"),
            heard_a: b.bus.ch("heard.a"),
        };
        p.publish(b.bus);
        p
    }

    fn publish(&self, bus: &mut Bus) {
        bus.flag(self.alive, self.kids.alive());
        let (t, a) = self.kids.heard.unwrap_or((f64::NAN, f64::NAN));
        bus.set(self.heard_t, t);
        bus.set(self.heard_a, a);
    }
}

impl Part for KidsPart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let (ctx, kids) = (w.ctx, &mut self.kids);
        // The ones coming back light a ring: it reaches the other parts as a message.
        let mut waves = Waves::new();
        self.spring.with(w.bus, |s| kids.step(ctx, kids::Step { spring: s, waves: &mut waves }, dt));
        w.out.extend(waves.dones.iter().map(|&t| Msg::Ring(t)));
        self.publish(w.bus);
    }

    fn notice(&mut self, w: &mut World, n: Notice) {
        let ctx = w.ctx;
        match n {
            Notice::Born(id) => self.kids.born(ctx, id),
            Notice::Pulse(id, error) => self.kids.pulse(ctx, id, error),
            Notice::Finish(id, ok) => self.kids.finish(ctx, id, ok),
            Notice::Gone => self.kids.gone(),
            Notice::Event(_) => {}
        }
        self.publish(w.bus);
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        self.kids.paint(g, f.ctx, &self.anchor.get(&f.bus));
    }

    fn inks(&self, _col: [f64; 3]) -> Vec<[[f64; 3]; 4]> {
        self.kids.inks().collect()
    }

    #[cfg(test)]
    fn kids(&self) -> usize {
        self.kids.len()
    }
}

/// Occasional gestures after a while standing still.
struct GesturesPart {
    gestures: Gestures,
    spring: SpringIn,
    groove: Ch,
    gesture: Ch,
    yawn: Ch,
}

impl GesturesPart {
    fn new(b: &mut Build) -> Self {
        GesturesPart { gestures: Gestures::new(), spring: SpringIn::new(b), groove: b.input("groove"), gesture: b.bus.ch("gesture"), yawn: b.bus.ch("yawn") }
    }

    fn publish(&self, bus: &mut Bus) {
        let id = match self.gestures.current() {
            None => 0.0,
            Some(Gesture::Stretch) => 1.0,
            Some(Gesture::LookClock) => 2.0,
            Some(Gesture::LookChat) => 3.0,
            Some(Gesture::Yawn) => 4.0,
        };
        bus.set(self.gesture, id);
    }
}

/// The gesture a channel holds (what `GesturesPart::publish` wrote).
fn gesture_of(v: f64) -> Option<Gesture> {
    match v as u8 {
        1 => Some(Gesture::Stretch),
        2 => Some(Gesture::LookClock),
        3 => Some(Gesture::LookChat),
        4 => Some(Gesture::Yawn),
        _ => None,
    }
}

impl Part for GesturesPart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let (ctx, gestures, groove) = (w.ctx, &mut self.gestures, w.bus.get(self.groove));
        let rand = &mut *w.rand;
        self.spring.with(w.bus, |s| gestures.step(ctx, gestures::Step { rand, spring: s, groove }, dt));
        self.publish(w.bus);
    }

    fn change(&mut self, w: &mut World, _from: State, _to: State) {
        self.gestures.cut();
        self.publish(w.bus);
    }

    fn layout(&self, f: &mut Frame, _g: &Grid) {
        f.bus.set(self.yawn, self.gestures.yawn(f.ctx.t));
    }
}

/// Where it looks: saccades, gestures, the kid that just sent something.
struct GazePart {
    gaze: Gaze,
    base: Ch,
    gesture: Ch,
    heard_t: Ch,
    heard_a: Ch,
    phase: Ch,
    nope: Ch,
    x: Ch,
    y: Ch,
    lim: Ch,
}

impl GazePart {
    fn new(b: &mut Build) -> Self {
        GazePart {
            gaze: Gaze::new(),
            base: b.input("blob_r"),
            gesture: b.input("gesture"),
            heard_t: b.input("heard.t"),
            heard_a: b.input("heard.a"),
            phase: b.input("phase"),
            nope: b.input("nope"),
            x: b.bus.ch("gaze.x"),
            y: b.bus.ch("gaze.y"),
            lim: b.bus.ch("gaze.lim"),
        }
    }
}

impl Part for GazePart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let b = &*w.bus;
        let heard = b.opt(self.heard_t).map(|t| (t, b.get(self.heard_a)));
        let m = gaze::Step { rand: &mut *w.rand, base: b.get(self.base), gesture: gesture_of(b.get(self.gesture)), heard, phase: b.get(self.phase), nope: b.get(self.nope) };
        self.gaze.step(w.ctx, m, dt);
        w.bus.set(self.x, self.gaze.gaze[0]);
        w.bus.set(self.y, self.gaze.gaze[1]);
    }

    fn layout(&self, f: &mut Frame, _g: &Grid) {
        let lim = self.gaze.lim(f.bus.get(self.base));
        f.bus.set(self.lim, lim);
    }
}

/// The corona's motes. If `~/.config/iris/particulas` points to a store piece, that one
/// draws them instead, until it breaks.
struct MotesPart {
    motes: Motes,
    #[cfg(feature = "pieces")]
    store: Option<std::cell::RefCell<crate::core::host::Host>>,
    v: [Ch; 10],
    low: Ch,
    anchor: AnchorIn,
}

impl MotesPart {
    fn new(b: &mut Build) -> Self {
        // The store one starts with a copy of the RNG: it consumes the same as the built-in ones,
        // so the rest of the buddy looks the same.
        #[cfg(feature = "pieces")]
        let store = load_store_motes(&mut b.rand.clone()).map(std::cell::RefCell::new);
        let motes = Motes::seed(b.rand);
        let names = ["blob_r", "level", "swallow", "groove", "music", "motes", "boot.t", "press", "gaze.x", "gaze.y"];
        let mut v = [Ch::default(); 10];
        for (i, n) in names.iter().enumerate() {
            v[i] = b.input(n);
        }
        MotesPart {
            low: b.input("low"),
            motes,
            #[cfg(feature = "pieces")]
            store,
            v,
            anchor: AnchorIn::new(b),
        }
    }

    fn step_in(&self, b: &Bus) -> particles::Step {
        let v = |i: usize| b.get(self.v[i]);
        particles::Step { base: v(0), level: v(1), swallow: v(2), dance: v(3) * v(4) }
    }
}

impl Part for MotesPart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let step = self.step_in(w.bus);
        #[cfg(feature = "pieces")]
        if let Some(h) = self.store.as_mut().map(std::cell::RefCell::get_mut) {
            h.step(w.ctx, &mote_params(&step, None), None, dt);
            if h.broken.is_none() {
                return;
            }
            self.store = None;
        }
        self.motes.step(w.ctx, step, dt);
    }

    fn notice(&mut self, _w: &mut World, _n: Notice) {
        #[cfg(feature = "pieces")]
        if let (Notice::Event(ev), Some(h)) = (_n, self.store.as_mut().map(std::cell::RefCell::get_mut)) {
            h.event(ev, _w.ctx, &[0.0; 16]);
        }
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        let b = &f.bus;
        let v = |i: usize| b.get(self.v[i]);
        let m = particles::Paint { motes: v(5), boot: v(6), press: v(7), gaze: [v(8), v(9)], low: b.on(self.low) };
        #[cfg(feature = "pieces")]
        if let Some(cell) = self.store.as_ref() {
            // `paint` takes &self, but calling the piece mutates its memory: hence the
            // RefCell. If it breaks here, the next `step` drops it.
            let mut h = cell.borrow_mut();
            if h.broken.is_none() {
                let anchor = self.anchor.get(b);
                h.draw(g, f.ctx, &mote_params(&self.step_in(b), Some(&m)), Some(&anchor));
                return;
            }
        }
        self.motes.paint(g, f.ctx, m);
    }
}

/// The parameters of the particle space (`iris_piece::abi::Space::Particles`).
#[cfg(feature = "pieces")]
fn mote_params(step: &particles::Step, paint: Option<&particles::Paint>) -> [f64; 16] {
    let mut p = [0.0; 16];
    p[..4].copy_from_slice(&[step.base, step.level, step.swallow, step.dance]);
    if let Some(m) = paint {
        p[4..10].copy_from_slice(&[m.motes, m.boot, m.press, m.gaze[0], m.gaze[1], if m.low { 1.0 } else { 0.0 }]);
    }
    p
}

/// If `~/.config/iris/particulas` holds a path to a particle piece (.wasm), loads it.
/// Any problem (missing, wrong space, fails to load) keeps the built-in ones.
#[cfg(feature = "pieces")]
fn load_store_motes(rand: &mut crate::core::rand::Rand) -> Option<crate::core::host::Host> {
    let path = std::fs::read_to_string(crate::paths::config()?.join("particulas")).ok()?;
    let wasm = std::fs::read(path.trim()).ok()?;
    let h = crate::core::host::Host::load(&wasm, rand).ok()?;
    (h.space == iris_piece::abi::Space::Particles).then_some(h)
}

// ─── The scene: where the body is this frame ─────────────────────────────────────────────────

/// Computes, once per drawing, where the body is and how it moves (drift, breathing, beat,
/// stretch, blink) and publishes it for everything drawn around and inside it.
struct Presence {
    i: [Ch; 16],
    o: [Ch; 14],
}

impl Presence {
    fn new(b: &mut Build) -> Self {
        let ins = [
            "blob_r", "boot", "zzz", "ripple", "level", "groove", "music", "vu", "deflate", "spring.sq", "copy", "shake", "bob",
            "droop", "yawn", "red",
        ];
        let outs = [
            "body.x", "body.y", "body.radius", "body.base", "ink.body", "ink.ring", "boot.t", "breath", "beat", "body.voice", "body.sx",
            "body.sy", "blink", "low",
        ];
        let mut i = [Ch::default(); 16];
        for (k, n) in ins.iter().enumerate() {
            i[k] = b.input(n);
        }
        let mut o = [Ch::default(); 14];
        for (k, n) in outs.iter().enumerate() {
            o[k] = b.bus.ch(n);
        }
        Presence { i, o }
    }
}

impl Part for Presence {
    fn layout(&self, f: &mut Frame, g: &Grid) {
        let ctx = f.ctx;
        let v = |k: usize| f.bus.get(self.i[k]);
        let (blob_r, p_boot, zzz, ripple, level, groove_a, music, vu, deflate, sq, copy, shake, bob, droop, yawn, red) =
            (v(0), v(1), v(2), v(3), v(4), v(5), v(6), v(7), v(8), v(9), v(10), v(11), v(12), v(13), v(14), v(15));
        let (t, st) = (ctx.t, ctx.st());
        let low = g.dh as f64 * g.k < 40.0;
        let err = if red > 0.35 { INK_RED } else { INK_MAIN };
        // The surroundings use the accent; on an error, red like everything else.
        let ring = if red > 0.35 { INK_RED } else { INK_ACCENT };

        // Body center: gentle drift + bobbing (asking) + shake (error) − droop.
        let cx = 0.025 * (t * 0.37).sin() + 0.015 * (t * 0.91 + 1.0).sin() + shake * 0.05 * (t * 48.0).sin();
        let cy = 0.02 * (t * 0.29 + 2.0).sin() + bob * 0.05 * (t * 2.2).sin() - droop * 0.08;
        let boot = if p_boot > 0.01 { (st / 2.4).min(1.0) } else { 1.0 };
        let grow = if p_boot > 0.01 { ease_out_back((st / 2.0).min(1.0)) } else { 1.0 };

        let breath = zzz * 0.03 * (t * 1.25).sin() + (1.0 - zzz) * 0.012 * (t * 1.2).sin() + yawn * 0.05;
        let beat = ripple * 0.045 * if level > 0.01 { level * 1.6 } else { (t * 5.0).sin().abs() };
        let groove = groove_a * music;
        let voice = vu * level * 0.22 + groove * 0.10;
        // With music it grows from the center with the level, like the key glow.
        let radius = blob_r * grow * (1.0 - 0.35 * deflate) * (1.0 + groove * 0.18);
        let (sx, sy) = (1.0 + sq, 1.0 - sq);
        let blink = blink(ctx, copy);
        let out = [cx, cy, radius, blob_r, err as f64, ring as f64, boot, breath, beat, voice, sx, sy, if blink { 1.0 } else { 0.0 }, if low { 1.0 } else { 0.0 }];
        for (c, x) in self.o.iter().zip(out) {
            f.bus.set(*c, x);
        }
    }
}

// ─── The body and its inner layers ───────────────────────────────────────────────────────────

/// The magnifier: a lens wandering over the body while searching.
struct MagnifierPart {
    magnifier: Magnifier,
    anchor: AnchorIn,
    lens: Ch,
    low: Ch,
    o: [Ch; 3],
}

impl MagnifierPart {
    fn new(b: &mut Build) -> Self {
        let o = [b.bus.ch("magnifier.x"), b.bus.ch("magnifier.y"), b.bus.ch("magnifier.r")];
        MagnifierPart { magnifier: Magnifier::new(), anchor: AnchorIn::new(b), lens: b.input("lens"), low: b.input("low"), o }
    }
}

impl Part for MagnifierPart {
    fn layout(&self, f: &mut Frame, _g: &Grid) {
        let s = self.magnifier.spot(f.ctx, &self.anchor.get(&f.bus));
        for (c, v) in self.o.iter().zip([s.x, s.y, s.r]) {
            f.bus.set(*c, v);
        }
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        let anchor = self.anchor.get(&f.bus);
        self.magnifier.paint(g, f.ctx, magnifier::Paint { anchor: &anchor, lens: f.bus.get(self.lens), low: f.bus.on(self.low) });
    }
}

/// The eye: where the pupil goes and how big it is. The body draws it.
struct EyePart {
    eye: Eye,
    i: [Ch; 10],
    o: [Ch; 5],
    ink: Ch,
}

impl EyePart {
    fn new(b: &mut Build) -> Self {
        let ins = ["body.x", "body.y", "gaze.x", "gaze.y", "pupil", "dilate", "red", "yawn", "lens", "blink"];
        let mut i = [Ch::default(); 10];
        for (k, n) in ins.iter().enumerate() {
            i[k] = b.input(n);
        }
        let o = [b.bus.ch("pupil.x"), b.bus.ch("pupil.y"), b.bus.ch("pupil.r"), b.bus.ch("pupil.blink"), b.bus.ch("pupil.ink")];
        EyePart { eye: Eye::new(), i, o, ink: b.input("ink.body") }
    }
}

impl Part for EyePart {
    fn layout(&self, f: &mut Frame, g: &Grid) {
        let v = |k: usize| f.bus.get(self.i[k]);
        let ink = f.bus.get(self.ink) as u8;
        let p = self.eye.pupil(eye::Controls {
            x: v(0) + v(2),
            y: v(1) + v(3),
            pupil: v(4),
            dilate: v(5),
            red: v(6),
            yawn: v(7),
            lens: v(8),
            blink: v(9) > 0.5,
            ink,
            du: g.du,
        });
        for (c, x) in self.o.iter().zip([p.x, p.y, p.r, if p.blink { 1.0 } else { 0.0 }, p.ink as f64]) {
            f.bus.set(*c, x);
        }
    }
}

/// The reading line sweeping the body.
struct ReadingPart {
    bar: ReadingBar,
    anchor: AnchorIn,
    scan: Ch,
    o: [Ch; 3],
}

impl ReadingPart {
    fn new(b: &mut Build) -> Self {
        let o = [b.bus.ch("reading.y"), b.bus.ch("reading.thickness"), b.bus.ch("reading.active")];
        ReadingPart { bar: ReadingBar::new(), anchor: AnchorIn::new(b), scan: b.input("scan"), o }
    }
}

impl Part for ReadingPart {
    fn layout(&self, f: &mut Frame, _g: &Grid) {
        let l = self.bar.line(f.ctx, &self.anchor.get(&f.bus), f.bus.get(self.scan));
        for (c, v) in self.o.iter().zip([l.y, l.thickness, if l.active { 1.0 } else { 0.0 }]) {
            f.bus.set(*c, v);
        }
    }
}

/// The context used, as a liquid filling the body.
struct LiquidPart {
    liquid: Liquid,
    anchor: AnchorIn,
    o: [Ch; 5],
}

impl LiquidPart {
    fn new(b: &mut Build) -> Self {
        let o = [b.bus.ch("liquid.ctx"), b.bus.ch("liquid.cap"), b.bus.ch("liquid.swell"), b.bus.ch("liquid.t"), b.bus.ch("liquid.ink")];
        LiquidPart { liquid: Liquid::new(), anchor: AnchorIn::new(b), o }
    }
}

impl Part for LiquidPart {
    fn layout(&self, f: &mut Frame, _g: &Grid) {
        let l = self.liquid.level(f.ctx, &self.anchor.get(&f.bus));
        for (c, v) in self.o.iter().zip([l.ctx, l.cap, l.swell, l.t, l.ink as f64]) {
            f.bus.set(*c, v);
        }
    }
}

/// The sprout: a bud growing from the body toward a subagent.
struct SproutPart {
    sprout: Sprout,
    anchor: AnchorIn,
    bud: Ch,
    phase: Ch,
    o: [Ch; 3],
}

impl SproutPart {
    fn new(b: &mut Build) -> Self {
        let o = [b.bus.ch("sprout.x"), b.bus.ch("sprout.y"), b.bus.ch("sprout.r")];
        SproutPart { sprout: Sprout::new(), anchor: AnchorIn::new(b), bud: b.input("bud"), phase: b.input("phase"), o }
    }
}

impl Part for SproutPart {
    fn layout(&self, f: &mut Frame, _g: &Grid) {
        let anchor = self.anchor.get(&f.bus);
        let tip = self.sprout.tip(f.ctx, &sprout::Paint { anchor: &anchor, bud: f.bus.get(self.bud), phase: f.bus.get(self.phase) });
        let (x, y, r) = tip.unwrap_or((f64::NAN, f64::NAN, f64::NAN));
        for (c, v) in self.o.iter().zip([x, y, r]) {
            f.bus.set(*c, v);
        }
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        let anchor = self.anchor.get(&f.bus);
        self.sprout.paint(g, f.ctx, sprout::Paint { anchor: &anchor, bud: f.bus.get(self.bud), phase: f.bus.get(self.phase) });
    }
}

/// What the inner layers published, read back as the structures the bodies know.
struct Inner {
    pupil: [Ch; 5],
    magnifier: [Ch; 3],
    reading: [Ch; 3],
    liquid: [Ch; 5],
    sprout: [Ch; 3],
}

impl Inner {
    fn new(b: &mut Build) -> Self {
        let mut grab = |names: &[&str]| -> Vec<Ch> { names.iter().map(|n| b.input(n)).collect() };
        Inner {
            pupil: grab(&["pupil.x", "pupil.y", "pupil.r", "pupil.blink", "pupil.ink"]).try_into().unwrap(),
            magnifier: grab(&["magnifier.x", "magnifier.y", "magnifier.r"]).try_into().unwrap(),
            reading: grab(&["reading.y", "reading.thickness", "reading.active"]).try_into().unwrap(),
            liquid: grab(&["liquid.ctx", "liquid.cap", "liquid.swell", "liquid.t", "liquid.ink"]).try_into().unwrap(),
            sprout: grab(&["sprout.x", "sprout.y", "sprout.r"]).try_into().unwrap(),
        }
    }

    fn pupil(&self, b: &Bus) -> Pupil {
        let v = |k: usize| b.get(self.pupil[k]);
        Pupil { x: v(0), y: v(1), r: v(2), blink: v(3) > 0.5, ink: v(4) as u8 }
    }

    fn spot(&self, b: &Bus) -> Spot {
        Spot { x: b.get(self.magnifier[0]), y: b.get(self.magnifier[1]), r: b.get(self.magnifier[2]) }
    }

    fn line(&self, b: &Bus) -> ReadingLine {
        ReadingLine { y: b.get(self.reading[0]), thickness: b.get(self.reading[1]), active: b.on(self.reading[2]) }
    }

    fn level(&self, b: &Bus) -> Level {
        let v = |k: usize| b.get(self.liquid[k]);
        Level { ctx: v(0), cap: v(1), swell: v(2), t: v(3), ink: v(4) as u8 }
    }

    fn tip(&self, b: &Bus) -> Option<(f64, f64, f64)> {
        b.opt(self.sprout[0]).map(|x| (x, b.get(self.sprout[1]), b.get(self.sprout[2])))
    }
}

/// The values every body reads: where it is and how it moves.
struct BodyIn {
    anchor: AnchorIn,
    inner: Inner,
    v: [Ch; 17],
}

impl BodyIn {
    fn new(b: &mut Build) -> Self {
        let names = [
            "breath", "beat", "body.voice", "amp", "phase", "chaos", "red", "dashed", "stitch", "body.sx", "body.sy", "lens", "ripple",
            "level", "yawn", "gaze.lim", "blink",
        ];
        let mut v = [Ch::default(); 17];
        for (i, n) in names.iter().enumerate() {
            v[i] = b.input(n);
        }
        BodyIn { anchor: AnchorIn::new(b), inner: Inner::new(b), v }
    }
}

/// The blob: the original's body, with its inner layers from highest to lowest: pupil >
/// magnifier > reading line > liquid (and underneath, the body's hatching).
struct BlobPart {
    blob: Blob,
    i: BodyIn,
}

impl BlobPart {
    fn new(b: &mut Build) -> Self {
        BlobPart { blob: Blob::new(), i: BodyIn::new(b) }
    }
}

impl Part for BlobPart {
    fn paint(&self, g: &mut Grid, f: &Frame) {
        let b = &f.bus;
        let v = |k: usize| b.get(self.i.v[k]);
        let anchor = self.i.anchor.get(b);
        let inner = &self.i.inner;
        let (pupil, line, level) = (inner.pupil(b), inner.line(b), inner.level(b));
        let glass = Glass { spot: inner.spot(b), active: v(11) > 0.5 };
        let fills: [&dyn Fill; 4] = [&pupil, &glass, &line, &level];
        let red = v(6);
        let shape = Shape { radius: anchor.radius, breath: v(0), beat: v(1), voice: v(2), amp: v(3), phase: v(4), chaos: v(5) + red * 2.0, red, t: f.ctx.t };
        let m = body::Paint {
            cx: anchor.cx,
            cy: anchor.cy,
            sx: v(9),
            sy: v(10),
            shape,
            dashed: v(7),
            stitch: v(8),
            tip: inner.tip(b),
            ink: anchor.ink,
            fills: &fills,
        };
        self.blob.paint(g, f.ctx, m);
    }

    /// The blob has no ink of its own: it's painted with the state's.
    fn inks(&self, col: [f64; 3]) -> Vec<[[f64; 3]; 4]> {
        vec![tones(col)]
    }
}

/// Baymax's face, in place of the blob.
struct FacePart {
    face: crate::core::buddies::baymax::Face,
    i: BodyIn,
    pupil: Ch,
    gaze: [Ch; 2],
}

impl FacePart {
    fn new(b: &mut Build) -> Self {
        FacePart { face: crate::core::buddies::baymax::Face::new(), i: BodyIn::new(b), pupil: b.input("pupil"), gaze: [b.input("gaze.x"), b.input("gaze.y")] }
    }
}

impl Part for FacePart {
    fn paint(&self, g: &mut Grid, f: &Frame) {
        let b = &f.bus;
        let v = |k: usize| b.get(self.i.v[k]);
        let anchor = self.i.anchor.get(b);
        let look = crate::core::buddies::baymax::Look {
            ripple: v(12),
            level: v(13),
            yawn: v(14),
            lens: v(11),
            magnifier: self.i.inner.spot(b),
            line: self.i.inner.line(b),
            anchor,
            breath: v(0),
            beat: v(1),
            voice: v(2),
            sx: v(9),
            sy: v(10),
            lim: v(15),
            gaze: [b.get(self.gaze[0]), b.get(self.gaze[1])],
            blink: v(16) > 0.5,
            pupil: b.get(self.pupil),
            red: v(6),
            dashed: v(7),
        };
        self.face.draw(g, f.ctx, &look);
    }

    fn inks(&self, col: [f64; 3]) -> Vec<[[f64; 3]; 4]> {
        vec![self.face.tones(col)]
    }
}

// ─── Pieces that only draw ───────────────────────────────────────────────────────────────────

struct TasksPart {
    tasks: Tasks,
    plan: Ch,
}

impl Part for TasksPart {
    fn paint(&self, g: &mut Grid, f: &Frame) {
        self.tasks.paint(g, f.ctx, f.bus.get(self.plan));
    }
}

struct TailPart {
    tail: Tail,
    anchor: AnchorIn,
}

impl Part for TailPart {
    fn paint(&self, g: &mut Grid, f: &Frame) {
        self.tail.paint(g, f.ctx, &self.anchor.get(&f.bus));
    }
}

/// The arcs: three long ones outside (`kind = "outer"`) or six short ones inside.
struct ArcsPart {
    arcs: Arcs,
    anchor: AnchorIn,
    v: [Ch; 4],
}

impl ArcsPart {
    fn new(b: &mut Build) -> Result<Self, String> {
        let arcs = match b.setting("kind").and_then(|k| k.as_str()).unwrap_or("outer") {
            "outer" => Arcs::outer(),
            "inner" => Arcs::inner(),
            k => return Err(format!("arcs: no hay arcos «{k}» (outer o inner)")),
        };
        let v = [b.input("arc_phase"), b.input("arcs"), b.input("arc_len"), b.input("boot.t")];
        Ok(ArcsPart { arcs, anchor: AnchorIn::new(b), v })
    }
}

impl Part for ArcsPart {
    fn paint(&self, g: &mut Grid, f: &Frame) {
        let b = &f.bus;
        let anchor = self.anchor.get(b);
        self.arcs.paint(g, f.ctx, arcs::Paint { anchor: &anchor, spin: b.get(self.v[0]), arcs: b.get(self.v[1]), arc_len: b.get(self.v[2]), boot: b.get(self.v[3]) });
    }
}

/// A piece that only draws around the body, with a few numbers from the bus.
struct Simple<P> {
    piece: P,
    anchor: AnchorIn,
    v: Vec<Ch>,
    paint: fn(&P, &mut Grid, &Context, &Anchor, &[f64]),
}

impl<P> Simple<P> {
    fn new(b: &mut Build, piece: P, inputs: &[&str], paint: fn(&P, &mut Grid, &Context, &Anchor, &[f64])) -> Self {
        let v = inputs.iter().map(|n| b.input(n)).collect();
        Simple { piece, anchor: AnchorIn::new(b), v, paint }
    }
}

impl<P> Part for Simple<P> {
    fn paint(&self, g: &mut Grid, f: &Frame) {
        let anchor = self.anchor.get(&f.bus);
        let vals: Vec<f64> = self.v.iter().map(|c| f.bus.get(*c)).collect();
        (self.paint)(&self.piece, g, f.ctx, &anchor, &vals);
    }
}
