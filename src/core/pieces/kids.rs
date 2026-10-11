//! KIDS: the subagents. Each one is a whole miniature core orbiting the main one, tied to it
//! by a line that particles travel up. It's born from the body, does its thing and melts back
//! in (or, if it failed, fades out in red where it is). When something arrives from a kid,
//! the main one looks at it.

use std::f64::consts::{PI, TAU};

use super::ease_out_back;
use super::waves::Waves;
use super::spring::Spring;
use super::Piece;
use crate::core::color::{tones, INK_ACCENT, INK_RED, MAX_KID_INKS};
use crate::core::grid::MAX_INK;
use crate::core::recipe::Recipe;
use std::sync::Arc;
use crate::core::context::{Anchor, Context};
use crate::core::state::{Event, Signals, State};
use crate::core::grid::Grid;
use crate::core::Core;

struct Kid {
    id: u64,
    /// Angle in the orbit; shared out with the other live kids.
    a: f64,
    born: f64,
    /// When it finished and whether it went well: good returns and melts in, bad fades in red where it is.
    end: Option<(f64, bool)>,
    merged: bool,
    /// The kid is a whole miniature core: the same states, colors and details as the main one,
    /// without the surroundings (scale, arcs, specks).
    core: Box<Core>,
    /// Particles traveling up the line: when they left and whether they carry an error.
    pulses: Vec<(f64, bool)>,
}

pub(crate) struct Kids {
    kids: Vec<Kid>,
    /// Where the main one looks when a kid sends it something: (when, angle).
    pub heard: Option<(f64, f64)>,
    /// Orbit radius: between the body and the scale.
    orbit: f64,
    /// How long a kid takes to leave the body and to return to it.
    travel: f64,
    /// How long a particle takes to travel from the kid to the main one.
    pulse: f64,
    /// The kid's body radius in its own world: used to scale it to its size.
    scale: f64,
    recipe: Option<Arc<Recipe>>,
}

pub(crate) struct Step<'a> {
    /// The one coming back nudges the main one and makes it flash.
    pub spring: &'a mut Spring,
    pub waves: &'a mut Waves,
}

impl Kids {
    /// `recipe`: what the kids are made of (its kid parts); none, the original's.
    pub fn new(scale: f64, recipe: Option<Arc<Recipe>>) -> Self {
        Kids { kids: vec![], heard: None, orbit: 0.70, travel: 0.8, pulse: 0.65, scale, recipe }
    }

    /// How many there are (alive or saying goodbye).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.kids.len()
    }

    /// Is any of them working?
    pub fn alive(&self) -> bool {
        self.kids.iter().any(|k| k.end.is_none())
    }

    /// A subagent is born: it sprouts from the body toward its spot in the orbit.
    pub fn born(&mut self, ctx: &Context, id: u64) {
        // It comes out where it will end up: the gap left by spreading out with one more.
        let alive = self.kids.iter().filter(|k| k.end.is_none()).count();
        let a = kid_slot(ctx.t, alive, alive + 1, ctx.calm());
        self.kids.push(Kid { id, a, born: ctx.t, end: None, merged: false, core: Box::new(Core::mini(State::Thinking, id, self.recipe.as_deref())), pulses: vec![] });
    }

    /// The kid did something (requested a tool or got its result): a particle rises.
    pub fn pulse(&mut self, ctx: &Context, id: u64, error: bool) {
        if let Some(k) = self.kids.iter_mut().find(|k| k.id == id) {
            k.pulses.push((ctx.t, error));
            k.core.fire(if error { Event::Error } else { Event::Key });
        }
    }

    /// The subagent finished: if it went well it returns to the body; if not, it fades out in red.
    pub fn finish(&mut self, ctx: &Context, id: u64, ok: bool) {
        if let Some(k) = self.kids.iter_mut().find(|k| k.id == id && k.end.is_none()) {
            k.end = Some((ctx.t, ok));
            if !ok {
                k.core.fire(Event::Error);
            }
        }
    }

    /// The engine is gone: the kids go with it, without ceremony.
    pub fn gone(&mut self) {
        self.kids.clear();
        self.heard = None;
    }

    /// The colors of the kids' own parts (the same recipe for all: the first one's), after the
    /// five base inks.
    fn own(&self) -> Vec<[[f64; 3]; 4]> {
        self.kids.first().map(|k| k.core.palette().split_off(5)).unwrap_or_default()
    }

    /// How many kids get an ink of their own, starting at `base`, leaving room for their own
    /// colors after them.
    fn slots(&self, base: u8, own: usize) -> usize {
        let room = (MAX_INK as usize + 1).saturating_sub(base as usize + own);
        self.kids.len().min(MAX_KID_INKS).min(room).max(1)
    }

    /// The kids' inks, each with the color of what it's doing, and then their own colors.
    pub fn inks(&self, base: u8) -> Vec<[[f64; 3]; 4]> {
        if self.kids.is_empty() {
            return vec![];
        }
        let own = self.own();
        let mut out: Vec<_> = self.kids.iter().take(self.slots(base, own.len())).map(|k| tones(k.core.ink())).collect();
        out.extend(own);
        out
    }
}

impl Piece for Kids {
    type Step<'a> = Step<'a>;
    /// The main one's body (the line leaves from its edge), and where the kids' inks start in
    /// the palette (after the inks of whatever is drawn before them).
    type Paint<'a> = (&'a Anchor, u8);

    /// Their state, their spot in the orbit, their gaze and their goodbye.
    fn step(&mut self, ctx: &Context, m: Step, dt: f64) {
        let t = ctx.t;
        let calm = ctx.calm();
        let alive: Vec<usize> = (0..self.kids.len()).filter(|&i| self.kids[i].end.is_none()).collect();
        let n = alive.len();
        for (slot, &i) in alive.iter().enumerate() {
            let want = kid_slot(t, slot, n, calm);
            let k = &mut self.kids[i];
            let d = (want - k.a + PI).rem_euclid(TAU) - PI;
            k.a += d * (dt * 2.5).min(1.0);
        }
        let child = Signals { calm: ctx.sig.calm, ..Default::default() };
        let mut merged = vec![];
        for k in &mut self.kids {
            let want = match ctx.sig.kids.iter().find(|(id, _)| *id == k.id) {
                Some(&(_, st)) if k.end.is_none() => st,
                _ => k.core.state(),
            };
            k.pulses.retain(|p| t - p.0 < self.pulse);
            // Right when it sends something, or on the way back, it looks at the main one (toward the center).
            let fresh = k.pulses.last().is_some_and(|p| t - p.0 < 0.3);
            k.core.look_at((fresh || k.end.is_some()).then(|| [-0.8 * k.a.cos(), -0.8 * k.a.sin()]));
            k.core.step(dt, want, &child);
            k.core.take_fired();
            if let Some((t0, true)) = k.end {
                if !k.merged && t - t0 >= self.travel {
                    k.merged = true;
                    merged.push(k.a);
                }
            }
        }
        // Each particle reaching the main one makes it look toward that kid.
        for k in &self.kids {
            if k.pulses.iter().any(|p| (t - p.0 - self.pulse).abs() < dt.max(0.02)) {
                self.heard = Some((t, k.a));
            }
        }
        for a in merged {
            m.spring.push(2.4, calm);
            m.waves.done(t);
            self.heard = Some((t, a));
        }
        let travel = self.travel;
        self.kids.retain(|k| match k.end {
            Some((t0, true)) => t - t0 < travel + 0.05,
            Some((t0, false)) => t - t0 < 1.4,
            None => true,
        });
    }

    /// The kids, with their line to the main one and the particles traveling up it.
    fn paint(&self, g: &mut Grid, ctx: &Context, (anchor, ink0): (&Anchor, u8)) {
        let t = ctx.t;
        let (cx, cy, radius) = (anchor.cx, anchor.cy, anchor.radius);
        let alive = self.kids.iter().filter(|k| k.end.is_none()).count();
        // A kid's body radius: with many they shrink; never below what still reads as an
        // eye.
        let kr = (if alive > 4 { 0.10_f64 } else { 0.13 }).max(2.6 * g.du);
        let slots = self.slots(ink0, self.own().len());
        let own0 = ink0.saturating_add(slots as u8);
        for (n, k) in self.kids.iter().enumerate() {
            // How far it has left the body (0 inside, 1 in the orbit) and how big it is.
            let (out, size) = match k.end {
                None => {
                    let u = ((t - k.born) / self.travel).min(1.0);
                    (ease_out_back(u).min(1.08), 0.35 + 0.65 * ease_out_back(u))
                }
                Some((t0, true)) => {
                    let u = ((t - t0) / self.travel).min(1.0);
                    let back = 1.0 - u * u;
                    (back, 0.4 + 0.6 * back)
                }
                Some((t0, false)) => (1.0, 1.0 - 0.8 * ((t - t0) / 1.4).min(1.0)),
            };
            let red = k.core.alarm() > 0.35;
            let ink = ink0 + n.min(slots - 1) as u8;
            let bob = 0.012 * (t * 1.7 + k.id as f64).sin();
            let orbit = radius + (self.orbit - radius) * out;
            let (kx, ky) = (orbit * k.a.cos(), orbit * k.a.sin() + bob);
            let r = kr * size;

            // The line: from the main one's edge to the kid's, in the accent. Dots flowing
            // inward.
            let (ax, ay) = (cx + (radius + 0.05) * k.a.cos(), cy + (radius + 0.05) * k.a.sin());
            let (bx, by) = (kx - (r + 0.05) * k.a.cos(), ky - (r + 0.05) * k.a.sin());
            let len = (bx - ax).hypot(by - ay);
            if len > 0.02 {
                let line = if red { INK_RED } else { INK_ACCENT };
                let step = 2.2 * g.du;
                let steps = (len / step).floor() as usize;
                let flow = if k.end.is_none() { t * 1.6 } else { 0.0 };
                for q in 0..=steps {
                    let u = q as f64 / steps.max(1) as f64;
                    // A crest traveling from the kid to the main one: the line "breathes" toward the center.
                    let tone = if (u * 3.0 + flow).fract() < 0.18 { 3 } else { 2 };
                    g.plot(bx + (ax - bx) * u, by + (ay - by) * u, tone, line);
                }
                // Particles: one per thing the kid did, in its color, from the kid to the main one.
                let d = g.du;
                for &(p0, err) in &k.pulses {
                    let u = ((t - p0) / self.pulse).clamp(0.0, 1.0);
                    let e = u * u * (3.0 - 2.0 * u);
                    let (px, py) = (bx + (ax - bx) * e, by + (ay - by) * e);
                    let pink = if err { INK_RED } else { ink };
                    for oy in [-d, 0.0, d] {
                        for ox in [-d, 0.0, d] {
                            g.plot(px + ox, py + oy, 3, pink);
                        }
                    }
                    // Trail toward the kid.
                    for (q, back) in [(3, 0.07), (2, 0.13)] {
                        let e2 = (e - back).max(0.0);
                        let (qx, qy) = (bx + (ax - bx) * e2, by + (ay - by) * e2);
                        g.plot(qx, qy, q, pink);
                        g.plot(qx + d, qy, q, pink);
                    }
                }
            }

            // The kid: its own core, scaled and in place, painted with its ink.
            g.view(kx, ky, r / self.scale, ink, own0);
            k.core.paint(g);
            g.unview();
        }
    }
}

/// Where kid number `slot` of `n` goes right now: evenly spread, turning slowly, the first
/// one top-left (the clock takes the right corner).
fn kid_slot(t: f64, slot: usize, n: usize, calm: f64) -> f64 {
    PI * 0.75 + t * 0.06 * calm + slot as f64 * TAU / n.max(1) as f64
}
