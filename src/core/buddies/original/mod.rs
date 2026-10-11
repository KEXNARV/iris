//! ORIGINAL: the classic buddy. Its script (how much of each piece per state), its mood and
//! all its pieces, in the order they are drawn. Whatever is put in the center goes there: the
//! blob, or Baymax's face (see `buddies::baymax`), and the rest stays the same.
//!
//! Order matters: the grid keeps the brightest dot, so almost everything comes out the same,
//! but the distant motes are only drawn where there's nothing else, which is why they go last.

use super::Buddy;
use crate::core::rand::Rand;
use crate::core::color::{tones, GREEN, RED, WARM};
use crate::core::context::{Notice, Context};
use crate::core::state::State;
use crate::core::pieces::arcs::{self, Arcs};
use crate::core::pieces::sprout::{self, Sprout};
use crate::core::pieces::body::{self, Blob, Shape};
use crate::core::pieces::scale::{self, Scale};
use crate::core::pieces::gestures::{self, Gestures};
use crate::core::pieces::git::{self, Git};
use crate::core::pieces::kids::{self, Kids};
use crate::core::pieces::ideas::{self, Ideas};
use crate::core::pieces::reading::ReadingBar;
use crate::core::pieces::liquid::Liquid;
use crate::core::pieces::magnifier::{self, Magnifier};
use crate::core::pieces::marks::{Tail, Tasks};
use crate::core::pieces::gaze::{self, Gaze};
use crate::core::pieces::eye::Eye;
use crate::core::pieces::waves::Waves;
use crate::core::pieces::packages::{self, Packages};
use crate::core::pieces::particles::{self, Motes};
use crate::core::pieces::presses::{self, Presses};
use crate::core::pieces::question::{self, QuestionMark};
use crate::core::pieces::stitches::{self, Stitches};
use crate::core::pieces::radar::{self, Radar};
use crate::core::pieces::voice::{self, Voice};
use crate::core::pieces::zees::{self, Zees};
use crate::core::pieces::{Piece, Fill};
use crate::core::grid::Grid;

pub(crate) mod mood;
pub(crate) mod scene;

/// Which pieces get drawn, one bit per piece (`LAYER_*`); all by default. The terminal doesn't
/// touch it; the Iris web page uses it to show the core taken apart, piece by piece.
pub(crate) static LAYERS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
pub(crate) const LAYER_SCALE: u32 = 0;
pub(crate) const LAYER_TASKS: u32 = 1;
pub(crate) const LAYER_ARCS: u32 = 2;
pub(crate) const LAYER_TAIL: u32 = 3;
pub(crate) const LAYER_PRESSES: u32 = 4;
pub(crate) const LAYER_VOICE: u32 = 5;
pub(crate) const LAYER_WAVES: u32 = 6;
pub(crate) const LAYER_RADAR: u32 = 7;
pub(crate) const LAYER_IDEAS: u32 = 8;
pub(crate) const LAYER_PACKAGES: u32 = 9;
pub(crate) const LAYER_STITCHES: u32 = 10;
pub(crate) const LAYER_GIT: u32 = 11;
pub(crate) const LAYER_QUESTION: u32 = 12;
pub(crate) const LAYER_ZEES: u32 = 13;
pub(crate) const LAYER_SPROUT: u32 = 14;
pub(crate) const LAYER_MAGNIFIER: u32 = 15;
pub(crate) const LAYER_CENTER: u32 = 16;
pub(crate) const LAYER_KIDS: u32 = 17;
pub(crate) const LAYER_MOTES: u32 = 18;

fn layer_on(layer: u32) -> bool {
    LAYERS.load(std::sync::atomic::Ordering::Relaxed) & (1 << layer) != 0
}
pub(crate) mod script;

use mood::Mood;
use scene::Scene;
use script::BASE;

/// What goes in the center of the original.
pub(crate) trait Center {
    fn draw(&self, g: &mut Grid, ctx: &Context, e: &Scene);

    /// The tones of its own ink (`INK_FACE`), derived from the state color.
    fn tones(&self, col: [f64; 3]) -> [[f64; 3]; 4];
}

pub(crate) struct Original<C: Center> {
    /// It's a kid: no scale, arcs or motes, which only clutter things at miniature size.
    mini: bool,
    rand: Rand,
    pub mood: Mood,
    scale: Scale,
    tasks: Tasks,
    /// Three long ones outside and six short ones inside, spinning in opposite directions.
    arcs: [Arcs; 2],
    tail: Tail,
    presses: Presses,
    voice: Voice,
    pub waves: Waves,
    radar: Radar,
    ideas: Ideas,
    packages: Packages,
    stitches: Stitches,
    git: Git,
    question: QuestionMark,
    zees: Zees,
    sprout: Sprout,
    magnifier: Magnifier,
    reading: ReadingBar,
    liquid: Liquid,
    eye: Eye,
    center: C,
    kids: Kids,
    motes: Motes,
    /// The motes from a store piece, if one is installed (see `load_store_motes`). If it
    /// breaks, it's dropped and the built-in ones come back.
    #[cfg(feature = "pieces")]
    store_motes: Option<std::cell::RefCell<crate::core::host::Host>>,
    gaze: Gaze,
    gestures: Gestures,
}

impl<C: Center> Original<C> {
    pub fn new(center: C) -> Self {
        let mut rand = Rand::new();
        // The store one starts with a copy of the RNG: it consumes the same as the built-in ones,
        // so the rest of the buddy looks the same.
        #[cfg(feature = "pieces")]
        let store_motes = load_store_motes(&mut rand.clone()).map(std::cell::RefCell::new);
        let motes = Motes::seed(&mut rand);
        Original {
            mini: false,
            rand,
            mood: Mood::new(State::Booting),
            scale: Scale::new(),
            tasks: Tasks::new(),
            arcs: [Arcs::outer(), Arcs::inner()],
            tail: Tail::new(),
            presses: Presses::new(),
            voice: Voice::new(),
            waves: Waves::new(),
            radar: Radar::new(),
            ideas: Ideas::new(),
            packages: Packages::new(),
            stitches: Stitches::new(),
            git: Git::new(),
            question: QuestionMark::new(),
            zees: Zees::new(),
            sprout: Sprout::new(),
            magnifier: Magnifier::new(),
            reading: ReadingBar::new(),
            liquid: Liquid::new(),
            eye: Eye::new(),
            center,
            kids: Kids::new(BASE.blob_r),
            motes,
            #[cfg(feature = "pieces")]
            store_motes,
            gaze: Gaze::new(),
            gestures: Gestures::new(),
        }
    }
}

impl Original<Blob> {
    /// A kid: already awake in `state`, with no boot or motes, and its own RNG.
    pub fn mini(state: State, seed: u64) -> Self {
        let mut o = Original::new(Blob::new());
        o.mini = true;
        o.motes.clear();
        #[cfg(feature = "pieces")]
        {
            o.store_motes = None;
        }
        o.mood.p = state.params();
        o.mood.col = state.rgb();
        o.rand.seed(seed);
        o
    }
}

/// The blob in the center, with its inner layers from highest to lowest: pupil >
/// magnifier > reading line > liquid (and underneath, the body's hatching).
impl Center for Blob {
    fn draw(&self, g: &mut Grid, ctx: &Context, e: &Scene) {
        let (p, a) = (e.p, e.mood);
        let fills: [&dyn Fill; 4] = [&e.pupil, &e.glass, &e.line, &e.level];
        let shape = Shape {
            radius: e.anchor.radius,
            breath: e.breath,
            beat: e.beat,
            voice: e.voice,
            amp: p.amp,
            phase: a.phase,
            chaos: p.chaos + a.red * 2.0,
            red: a.red,
            t: ctx.t,
        };
        let m = body::Paint {
            cx: e.anchor.cx,
            cy: e.anchor.cy,
            sx: e.sx,
            sy: e.sy,
            shape,
            dashed: p.dashed,
            stitch: p.stitch,
            tip: e.tip,
            ink: e.anchor.ink,
            fills: &fills,
        };
        self.paint(g, ctx, m);
    }

    /// The blob has no ink of its own: it's painted with the state's.
    fn tones(&self, col: [f64; 3]) -> [[f64; 3]; 4] {
        tones(col)
    }
}

impl<C: Center + 'static> Buddy for Original<C> {
    fn step(&mut self, ctx: &Context, dt: f64) {
        let with_kids = self.kids.alive();
        self.mood.approach(ctx, dt, with_kids);
        self.mood.fade_out(ctx, dt);
        self.waves.step(ctx, &mut self.mood.spring, dt);
        self.kids.step(ctx, kids::Step { spring: &mut self.mood.spring, waves: &mut self.waves }, dt);
        self.mood.advance(ctx, dt);
        // Occasional gestures when it's been idle for a while.
        self.gestures.step(ctx, gestures::Step { rand: &mut self.rand, spring: &mut self.mood.spring, groove: self.mood.groove }, dt);
        let a = &self.mood;
        self.gaze.step(
            ctx,
            gaze::Step {
                rand: &mut self.rand,
                base: a.p.blob_r,
                gesture: self.gestures.current(),
                heard: self.kids.heard,
                phase: a.phase,
                nope: a.nope,
            },
            dt,
        );
        let step = particles::Step { base: a.p.blob_r, level: a.level, swallow: a.swallow, dance: a.groove * a.music };
        #[cfg(feature = "pieces")]
        if let Some(h) = self.store_motes.as_mut().map(std::cell::RefCell::get_mut) {
            h.step(ctx, &mote_params(&step, None), None, dt);
            if h.broken.is_none() {
                return;
            }
            self.store_motes = None;
        }
        self.motes.step(ctx, step, dt);
    }

    fn change(&mut self, from: State, a: State, ctx: &Context) {
        self.mood.change(from, a, ctx);
        self.gestures.cut();
    }

    fn notice(&mut self, notice: Notice, ctx: &Context) {
        match notice {
            Notice::Event(ev) => {
                self.mood.event(ev, ctx);
                #[cfg(feature = "pieces")]
                if let Some(h) = self.store_motes.as_mut().map(std::cell::RefCell::get_mut) {
                    h.event(ev, ctx, &[0.0; 16]);
                }
                self.waves.event(ev, ctx, &mut self.rand);
            }
            Notice::Born(id) => {
                self.kids.born(ctx, id);
                self.mood.kick(1.6, ctx);
            }
            Notice::Pulse(id, error) => self.kids.pulse(ctx, id, error),
            Notice::Finish(id, ok) => self.kids.finish(ctx, id, ok),
            Notice::Gone => self.kids.gone(),
        }
    }

    fn paint(&self, g: &mut Grid, ctx: &Context) {
        let e = Scene::new(self, ctx, g);
        let (p, a, anchor) = (&self.mood.p, &self.mood, &e.anchor);
        if !self.mini && layer_on(LAYER_SCALE) {
            self.scale.paint(
                g,
                ctx,
                scale::Paint {
                    anchor,
                    boot: e.boot,
                    level: a.level,
                    spin: a.arc_phase,
                    music: a.music,
                    groove: a.groove,
                    dones: &self.waves.dones,
                    pass: a.pass,
                    copy: a.copy,
                    red: a.red,
                    low: e.low,
                    vu: p.vu,
                    fill: p.fill,
                    sweep: p.sweep,
                    cardinal: p.cardinal,
                    tests: p.tests,
                },
            );
        }
        if layer_on(LAYER_TASKS) {
            self.tasks.paint(g, ctx, p.plan);
        }
        if !self.mini && layer_on(LAYER_ARCS) {
            for arcs in &self.arcs {
                arcs.paint(g, ctx, arcs::Paint { anchor, spin: a.arc_phase, arcs: p.arcs, arc_len: p.arc_len, boot: e.boot });
            }
        }
        if layer_on(LAYER_TAIL) {
            self.tail.paint(g, ctx, anchor);
        }
        if layer_on(LAYER_PRESSES) {
            self.presses.paint(g, ctx, presses::Paint { anchor, press: p.press });
        }
        if layer_on(LAYER_VOICE) {
            self.voice.paint(g, ctx, voice::Paint { anchor, ripple: p.ripple });
        }
        if layer_on(LAYER_WAVES) {
            self.waves.paint(g, ctx, anchor);
        }
        if layer_on(LAYER_RADAR) {
            self.radar.paint(g, ctx, radar::Paint { anchor, sweep: p.sweep, spin: a.arc_phase });
        }
        if layer_on(LAYER_IDEAS) {
            self.ideas.paint(g, ctx, ideas::Paint { anchor, orbit: p.orbit, amp: p.amp, phase: a.phase, low: e.low });
        }
        if layer_on(LAYER_PACKAGES) {
            self.packages.paint(g, ctx, packages::Paint { anchor, packets: p.packets, low: e.low });
        }
        if layer_on(LAYER_STITCHES) {
            self.stitches.paint(g, ctx, stitches::Paint { anchor, stitch: p.stitch });
        }
        if layer_on(LAYER_GIT) {
            self.git.paint(g, ctx, git::Paint { anchor, git: p.git });
        }
        if layer_on(LAYER_QUESTION) {
            self.question.paint(g, ctx, question::Paint { anchor, droop: p.droop });
        }
        if layer_on(LAYER_ZEES) {
            self.zees.paint(g, ctx, zees::Paint { anchor, zzz: p.zzz, low: e.low });
        }
        if layer_on(LAYER_SPROUT) {
            self.sprout.paint(g, ctx, sprout::Paint { anchor, bud: p.bud, phase: a.phase });
        }
        if layer_on(LAYER_MAGNIFIER) {
            self.magnifier.paint(g, ctx, magnifier::Paint { anchor, lens: p.lens, low: e.low });
        }
        if layer_on(LAYER_CENTER) {
            self.center.draw(g, ctx, &e);
        }
        if layer_on(LAYER_KIDS) {
            self.kids.paint(g, ctx, anchor);
        }
        if !self.mini && layer_on(LAYER_MOTES) {
            let m = particles::Paint { motes: p.motes, boot: e.boot, press: p.press, gaze: self.gaze.gaze, low: e.low };
            #[cfg(feature = "pieces")]
            if let Some(cell) = self.store_motes.as_ref() {
                // `paint` takes &self, but calling the piece mutates its memory: hence the
                // RefCell. If it breaks here, the next `step` drops it.
                let mut h = cell.borrow_mut();
                if h.broken.is_none() {
                    let step = particles::Step { base: p.blob_r, level: a.level, swallow: a.swallow, dance: a.groove * a.music };
                    h.draw(g, ctx, &mote_params(&step, Some(&m)), Some(anchor));
                    return;
                }
            }
            self.motes.paint(g, ctx, m);
        }
    }

    /// State, green, red, amber and accent; the center's ink; one per kid.
    fn palette(&self) -> Vec<[[f64; 3]; 4]> {
        let col = self.mood.col;
        let mut pal: Vec<_> = [col, GREEN, RED, WARM, crate::theme::accent_rgb()].into_iter().map(tones).collect();
        pal.push(self.center.tones(col));
        pal.extend(self.kids.inks());
        pal
    }

    fn color(&self) -> [f64; 3] {
        self.mood.col
    }

    fn alarm(&self) -> f64 {
        self.mood.red
    }

    #[cfg(test)]
    fn kids(&self) -> usize {
        self.kids.len()
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        self
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
fn load_store_motes(rand: &mut Rand) -> Option<crate::core::host::Host> {
    let path = std::fs::read_to_string(crate::paths::config()?.join("particulas")).ok()?;
    let wasm = std::fs::read(path.trim()).ok()?;
    let h = crate::core::host::Host::load(&wasm, rand).ok()?;
    (h.space == iris_piece::abi::Space::Particles).then_some(h)
}
