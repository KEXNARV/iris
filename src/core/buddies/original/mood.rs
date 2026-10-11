//! MOOD of the original buddy: what moves inside it and its pieces share. The script's
//! parameters approaching the state, the color, the phases of the wobble and the spin, the
//! spring, the flashes events leave behind and what comes in from the voice and the music.

use super::script::Params;
use crate::core::color::{mix, RED};
use crate::core::context::Context;
use crate::core::state::{Event, State};
use crate::core::pieces::spring::Spring;
use crate::core::pieces::Piece;

pub(crate) struct Mood {
    pub p: Params,
    pub col: [f64; 3],
    /// Phase of the body's wobble and the spin of what surrounds it.
    pub phase: f64,
    pub arc_phase: f64,
    /// Elastic squash.
    pub spring: Spring,
    pub shake: f64,
    pub red: f64,
    pub deflate: f64,
    pub nope: f64,
    pub copy: f64,
    pub pass: f64,
    /// When going from listening to transcribing, the motes get swallowed toward the center.
    pub swallow: f64,
    pub level: f64,
    /// Music level and how much it is dancing (0..1: comes in over half a second, leaves at once).
    pub music: f64,
    pub groove: f64,
}

impl Mood {
    pub fn new(s: State) -> Self {
        Mood {
            p: s.params(),
            col: s.rgb(),
            phase: 0.0,
            arc_phase: 0.0,
            spring: Spring::new(),
            shake: 0.0,
            red: 0.0,
            deflate: 0.0,
            nope: 0.0,
            copy: 0.0,
            pass: 0.0,
            swallow: 0.0,
            level: 0.0,
            music: 0.0,
            groove: 0.0,
        }
    }

    pub fn kick(&mut self, v: f64, ctx: &Context) {
        self.spring.push(v, ctx.calm());
    }

    /// Transitions with intent: every change has its gesture.
    pub fn change(&mut self, from: State, to: State, ctx: &Context) {
        use State::*;
        match (from, to) {
            (_, Offline) => self.kick(-1.5, ctx),
            (Sleeping, _) => {
                self.kick(3.0, ctx);
                self.copy = 0.5; // blinks on waking up
            }
            (Listening | NoVoice, Transcribing) => self.swallow = 1.0,
            (f, Thinking) if f.is_tool() => self.kick(-0.8, ctx),
            (_, t) if t.is_tool() => self.kick(1.2, ctx),
            _ => self.kick(1.0, ctx),
        }
    }

    /// What each event does to the body: flashes and pushes.
    pub fn event(&mut self, ev: Event, ctx: &Context) {
        match ev {
            Event::Key => self.kick(1.2, ctx),
            Event::Error => {
                self.red = 1.0;
                self.shake = if ctx.sig.calm { 0.0 } else { 1.0 };
                self.kick(-2.5, ctx);
            }
            Event::Done => self.kick(3.2, ctx),
            Event::Cancel => {
                self.deflate = 1.0;
                self.kick(-3.0, ctx);
            }
            Event::Nope => self.nope = 1.0,
            Event::Merge => {}
            Event::Copy => {
                self.copy = 1.0;
                self.kick(0.8, ctx);
            }
            Event::Pass => {
                self.pass = 1.0;
                self.kick(1.5, ctx);
            }
        }
    }

    /// The parameters and the color move toward the state's; with an error, toward red.
    pub fn approach(&mut self, ctx: &Context, dt: f64, with_kids: bool) {
        let k = 1.0 - (-dt / 0.35).exp();
        let mut target = ctx.state.params();
        // With kids, the main one steps aside a bit to leave them the orbit,
        // and the arcs dim a bit so the network reads.
        if with_kids {
            target.blob_r *= 0.72;
            target.arcs *= 0.45;
            target.motes *= 0.5;
        }
        self.p.approach(&target, k);
        let base = ctx.state.rgb();
        let want_col = if self.red > 0.05 { mix(base, RED, (self.red * 1.4).min(1.0)) } else { base };
        let kc = if self.red > 0.05 { 0.5 } else { k };
        for i in 0..3 {
            self.col[i] += (want_col[i] - self.col[i]) * kc;
        }
    }

    /// The spring bounces and the flashes fade.
    pub fn fade_out(&mut self, ctx: &Context, dt: f64) {
        self.spring.step(ctx, (), dt);
        for v in [&mut self.red, &mut self.nope] {
            *v = (*v - dt * 1.1).max(0.0);
        }
        self.shake = (self.shake - dt * 1.6).max(0.0);
        self.deflate = (self.deflate - dt * 1.4).max(0.0);
        self.copy = (self.copy - dt * 3.0).max(0.0);
        self.pass = (self.pass - dt * 1.2).max(0.0);
        self.swallow = (self.swallow - dt * 1.5).max(0.0);
    }

    /// The phases advance, and the outside comes in: the voice and the music.
    pub fn advance(&mut self, ctx: &Context, dt: f64) {
        let calm = ctx.calm();
        self.phase += dt * self.p.wob * calm;
        self.arc_phase += dt * self.p.arc_speed * calm;

        let lvl = if matches!(ctx.state, State::Listening | State::Speaking) { (ctx.sig.level as f64 * 6.0).min(1.0) } else { 0.0 };
        self.level += (lvl - self.level) * if lvl > self.level { 0.5 } else { 0.12 };
        // Music: cava already comes smoothed, so it is followed almost as is. Like the keyboard,
        // it shows up in half a second and a key (or any other state) turns it off instantly.
        let m = ctx.sig.music as f64;
        // A beat (the level jumps suddenly) squashes it a little: that's how you see it dance.
        if m - self.music > 0.15 {
            self.kick(1.5 * self.groove, ctx);
        }
        self.music += (m - self.music) * (dt * 30.0).min(1.0);
        self.groove = if ctx.state != State::Idle {
            (self.groove - dt * 8.0).max(0.0)
        } else if m > 0.02 {
            (self.groove + dt / 0.5).min(1.0)
        } else {
            (self.groove - dt).max(0.0) // between songs it fades out slowly
        };
    }
}
