//! WAVES: the trail events leave around the body. Each key, a short ripple; each end of
//! turn, a green ring that expands up to the scale; each subagent that comes back, a drop
//! that travels from the scale and merges (and pushes the body when it arrives).

use std::f64::consts::TAU;

use super::spring::Spring;
use super::Piece;
use crate::core::rand::Rand;
use crate::core::color::{INK_GREEN, INK_MAIN};
use crate::core::context::{Anchor, Context};
use crate::core::state::Event;
use crate::core::grid::Grid;

pub(crate) struct Waves {
    /// When each key happened.
    keys: Vec<f64>,
    /// When each end of turn (or each merge) happened: the scale reads them too.
    pub dones: Vec<f64>,
    /// Drops in flight: when they left, from what angle and whether they've arrived.
    drops: Vec<(f64, f64, bool)>,
    /// How long a drop takes to arrive, and the push it gives.
    travel: f64,
    impact: f64,
}

impl Waves {
    pub fn new() -> Self {
        Waves { keys: vec![], dones: vec![], drops: vec![], travel: 0.7, impact: 2.4 }
    }

    /// A green ring, now.
    pub fn done(&mut self, t: f64) {
        self.dones.push(t);
    }
}

impl Piece for Waves {
    /// Arriving drops push the body.
    type Step<'a> = &'a mut Spring;
    type Paint<'a> = &'a Anchor;

    fn event(&mut self, ev: Event, ctx: &Context, rand: &mut Rand) {
        let t = ctx.t;
        match ev {
            Event::Key => self.keys.push(t),
            Event::Done => self.dones.push(t),
            Event::Merge => {
                let a = rand.rand() * TAU;
                self.drops.push((t, a, false));
            }
            _ => {}
        }
    }

    fn step(&mut self, ctx: &Context, spring: &mut Spring, _dt: f64) {
        let t = ctx.t;
        self.dones.retain(|b| t - b < 1.2);
        self.keys.retain(|k0| t - k0 < 0.5);
        let mut pops = 0;
        for m in &mut self.drops {
            if t - m.0 > self.travel && !m.2 {
                m.2 = true;
                pops += 1;
            }
        }
        for _ in 0..pops {
            spring.push(self.impact, ctx.calm());
            self.dones.push(t);
        }
        self.drops.retain(|m| t - m.0 < 1.0);
    }

    fn paint(&self, g: &mut Grid, ctx: &Context, anchor: &Anchor) {
        let (t, base) = (ctx.t, anchor.base);
        // Key ripples.
        for k0 in &self.keys {
            let u = (t - k0) / 0.45;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let r = base + 0.05 + u * 0.16;
            for q in 0..36 {
                let (x, y) = anchor.dot(r, q as f64 * TAU / 36.0);
                g.plot(x, y, if u < 0.5 { 2 } else { 1 }, INK_MAIN);
            }
        }
        // Done: a green ring that expands up to the scale.
        for b in &self.dones {
            let u = (t - b) / 1.1;
            if !(0.0..=0.75).contains(&u) {
                continue;
            }
            let r = base + 0.04 + u * 0.62;
            let n = 50 + (u * 40.0) as usize;
            for q in 0..n {
                let (x, y) = anchor.dot(r, q as f64 * TAU / n as f64);
                g.plot(x, y, if u < 0.45 { 3 } else { 2 }, INK_GREEN);
            }
        }
        // The subagent comes back: a drop travels from the scale and merges.
        for &(t0, a, _) in &self.drops {
            let u = (t - t0) / self.travel;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let r = 0.86 - u * (0.86 - base);
            let rr = 0.07 * (1.0 - u * 0.5);
            let (bx, by) = anchor.dot(r, a);
            for q in 0..14 {
                let b = q as f64 * TAU / 14.0;
                g.plot(bx + rr * b.cos(), by + rr * b.sin(), 3, INK_MAIN);
            }
        }
    }
}
