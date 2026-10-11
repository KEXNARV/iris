//! SCENE of the original buddy: what is computed once per frame from its mood and shared
//! among the pieces when drawing. Where the body is (the anchor), how much it breathes, beats
//! and stretches, whether it blinks, and the inner layers already placed (pupil, magnifier,
//! reading, liquid) for the center to use if it wants.

use super::mood::Mood;
use super::script::Params;
use super::{Center, Original};
use crate::core::color::{INK_ACCENT, INK_MAIN, INK_RED};
use crate::core::context::{Anchor, Context};
use crate::core::pieces::reading::ReadingLine;
use crate::core::pieces::liquid::Level;
use crate::core::pieces::magnifier::{Spot, Glass};
use crate::core::pieces::gaze::blink;
use crate::core::pieces::eye::{self, Pupil};
use crate::core::pieces::{sprout, ease_out_back};
use crate::core::grid::Grid;

pub(crate) struct Scene<'a> {
    pub p: &'a Params,
    pub mood: &'a Mood,
    /// Small panel (or kid): fine details are turned off.
    pub low: bool,
    pub anchor: Anchor,
    /// How far into the boot it is (0..1).
    pub boot: f64,
    pub yawn: f64,
    pub breath: f64,
    /// While responding it beats with the real volume of its voice; without voice, at its own pace.
    pub beat: f64,
    pub voice: f64,
    /// Stretched by the spring.
    pub sx: f64,
    pub sy: f64,
    pub blink: bool,
    /// Where it's looking and how far it can look.
    pub gaze: [f64; 2],
    pub lim: f64,
    pub magnifier: Spot,
    pub line: ReadingLine,
    pub tip: Option<(f64, f64, f64)>,
    pub pupil: Pupil,
    pub glass: Glass,
    pub level: Level,
}

impl<'a> Scene<'a> {
    pub fn new<C: Center>(o: &'a Original<C>, ctx: &Context, g: &Grid) -> Self {
        let a = &o.mood;
        let p = &a.p;
        let (t, st) = (ctx.t, ctx.st());
        let low = g.dh as f64 * g.k < 40.0;
        let err = if a.red > 0.35 { INK_RED } else { INK_MAIN };
        // The surroundings use the accent; on an error, red like everything else.
        let ring = if a.red > 0.35 { INK_RED } else { INK_ACCENT };

        // Body center: gentle drift + bobbing (asking) + shake (error) − droop.
        let cx = 0.025 * (t * 0.37).sin() + 0.015 * (t * 0.91 + 1.0).sin() + a.shake * 0.05 * (t * 48.0).sin();
        let cy = 0.02 * (t * 0.29 + 2.0).sin() + p.bob * 0.05 * (t * 2.2).sin() - p.droop * 0.08;
        let boot = if p.boot > 0.01 { (st / 2.4).min(1.0) } else { 1.0 };
        let grow = if p.boot > 0.01 { ease_out_back((st / 2.0).min(1.0)) } else { 1.0 };

        let yawn = o.gestures.yawn(t);
        let breath = p.zzz * 0.03 * (t * 1.25).sin() + (1.0 - p.zzz) * 0.012 * (t * 1.2).sin() + yawn * 0.05;
        let beat = p.ripple * 0.045 * if a.level > 0.01 { a.level * 1.6 } else { (t * 5.0).sin().abs() };
        let groove = a.groove * a.music;
        let voice = p.vu * a.level * 0.22 + groove * 0.10;
        // With music it grows from the center with the level, like the key glow.
        let radius = p.blob_r * grow * (1.0 - 0.35 * a.deflate) * (1.0 + groove * 0.18);
        let (sx, sy) = (1.0 + a.spring.sq, 1.0 - a.spring.sq);
        let blink = blink(ctx, a.copy);
        let anchor = Anchor { cx, cy, radius: radius, base: p.blob_r, ink: err, border: ring };

        let gaze = o.gaze.gaze;
        let magnifier = o.magnifier.spot(ctx, &anchor);
        let pupil = o.eye.pupil(eye::Controls {
            x: cx + gaze[0],
            y: cy + gaze[1],
            pupil: p.pupil,
            dilate: p.dilate,
            red: a.red,
            yawn,
            lens: p.lens,
            blink: blink,
            ink: err,
            du: g.du,
        });
        Scene {
            p,
            mood: a,
            low,
            boot,
            yawn,
            breath,
            beat,
            voice,
            sx,
            sy,
            blink,
            gaze,
            lim: o.gaze.lim(p.blob_r),
            magnifier,
            line: o.reading.line(ctx, &anchor, p.scan),
            tip: o.sprout.tip(ctx, &sprout::Paint { anchor: &anchor, bud: p.bud, phase: a.phase }),
            pupil,
            glass: Glass { spot: magnifier, active: p.lens > 0.5 },
            level: o.liquid.level(ctx, &anchor),
            anchor,
        }
    }
}
