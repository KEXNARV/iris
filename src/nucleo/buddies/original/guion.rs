//! GUION del buddy original: cuánto de cada pieza muestra en cada estado (radio, ondas, arcos,
//! detalles) y cómo pasa suavemente de unos valores a otros. Es suyo, no del sistema: otro
//! buddy puede tener su propio guion o ninguno.

use crate::nucleo::estado::State;

#[derive(Clone, Copy)]
pub(crate) struct Params {
    pub(crate) blob_r: f64,
    pub(crate) amp: f64,
    pub(crate) wob: f64,
    pub(crate) chaos: f64,
    pub(crate) arc_speed: f64,
    pub(crate) arcs: f64,
    pub(crate) arc_len: f64,
    pub(crate) ripple: f64,
    pub(crate) orbit: f64,
    pub(crate) sweep: f64,
    pub(crate) fill: f64,
    pub(crate) vu: f64,
    pub(crate) cardinal: f64,
    pub(crate) dashed: f64,
    pub(crate) scan: f64,
    pub(crate) stitch: f64,
    pub(crate) packets: f64,
    pub(crate) bud: f64,
    pub(crate) zzz: f64,
    pub(crate) pupil: f64,
    pub(crate) bob: f64,
    pub(crate) motes: f64,
    pub(crate) boot: f64,
    pub(crate) lens: f64,
    pub(crate) plan: f64,
    pub(crate) tests: f64,
    pub(crate) git: f64,
    pub(crate) press: f64,
    pub(crate) droop: f64,
    /// Apertura de la pupila: >1 dilatada (escucha, busca), <1 contraída (concentrado).
    pub(crate) dilate: f64,
}

pub(crate) const BASE: Params = Params {
    blob_r: 0.40,
    amp: 0.035,
    wob: 0.35,
    chaos: 0.1,
    arc_speed: 0.25,
    arcs: 1.0,
    arc_len: 1.0,
    ripple: 0.0,
    orbit: 0.0,
    sweep: 0.0,
    fill: 0.0,
    vu: 0.0,
    cardinal: 0.0,
    dashed: 0.0,
    scan: 0.0,
    stitch: 0.0,
    packets: 0.0,
    bud: 0.0,
    zzz: 0.0,
    pupil: 1.0,
    bob: 0.0,
    motes: 1.0,
    boot: 0.0,
    lens: 0.0,
    plan: 0.0,
    tests: 0.0,
    git: 0.0,
    press: 0.0,
    droop: 0.0,
    dilate: 1.0,
};

impl Params {
    pub(crate) fn approach(&mut self, to: &Params, k: f64) {
        macro_rules! go {
            ($($f:ident),*) => { $( self.$f += (to.$f - self.$f) * k; )* };
        }
        go!(blob_r, amp, wob, chaos, arc_speed, arcs, arc_len, ripple, orbit, sweep, fill, vu, cardinal, dashed,
            scan, stitch, packets, bud, zzz, pupil, bob, motes, boot, lens, plan, tests, git, press, droop, dilate);
    }
}

impl State {
    pub(crate) fn params(self) -> Params {
        use State::*;
        let b = BASE;
        match self {
            Booting => Params { boot: 1.0, arc_speed: 0.5, ..b },
            Sleeping => Params { blob_r: 0.34, amp: 0.02, wob: 0.12, arcs: 0.0, pupil: 0.0, zzz: 1.0, motes: 0.3, ..b },
            Idle => b,
            Typing => Params { amp: 0.04, wob: 0.5, arc_speed: 0.4, ..b },
            Listening => Params { amp: 0.05, wob: 0.8, chaos: 0.5, arc_speed: 0.6, arc_len: 0.8, vu: 1.0, dilate: 1.3, ..b },
            NoVoice => Params { amp: 0.025, wob: 0.3, arc_speed: 0.15, arc_len: 0.6, droop: 1.0, dilate: 1.4, ..b },
            Transcribing => Params {
                blob_r: 0.32, amp: 0.05, wob: 3.0, chaos: 0.9, arc_speed: 2.2, arc_len: 0.6, fill: 1.0, ..b
            },
            Thinking => Params { amp: 0.10, wob: 0.7, chaos: 0.3, arc_speed: 1.1, orbit: 1.0, dilate: 0.75, ..b },
            Planning => Params { amp: 0.04, wob: 0.5, arc_speed: 0.5, plan: 1.0, ..b },
            Searching => Params { amp: 0.03, wob: 0.5, arc_speed: 0.8, lens: 1.0, ..b },
            Reading => Params { amp: 0.03, wob: 0.4, arc_speed: 0.7, scan: 1.0, ..b },
            Editing => Params { amp: 0.05, wob: 0.9, arc_speed: 1.2, arc_len: 0.7, stitch: 1.0, ..b },
            Running => Params { blob_r: 0.38, amp: 0.08, wob: 2.0, chaos: 0.4, arc_speed: 2.4, arc_len: 0.7, sweep: 1.0, ..b },
            Testing => Params { amp: 0.04, wob: 1.0, arc_speed: 1.4, arc_len: 0.6, tests: 1.0, ..b },
            Git => Params { blob_r: 0.34, amp: 0.04, wob: 0.6, arc_speed: 0.6, git: 1.0, ..b },
            Web => Params { blob_r: 0.36, amp: 0.05, wob: 1.0, arc_speed: 0.8, packets: 1.0, ..b },
            Delegating => Params { blob_r: 0.36, amp: 0.06, wob: 0.8, arc_speed: 0.9, bud: 1.0, ..b },
            Speaking => Params { amp: 0.05, wob: 1.0, chaos: 0.2, arc_speed: 0.9, ripple: 1.0, dilate: 1.1, ..b },
            Asking => Params { blob_r: 0.38, amp: 0.03, wob: 0.3, arc_speed: 0.12, cardinal: 1.0, bob: 1.0, ..b },
            Compacting => Params { blob_r: 0.36, amp: 0.03, wob: 0.6, arc_speed: 1.6, arc_len: 0.5, press: 1.0, ..b },
            Offline => Params {
                blob_r: 0.30, amp: 0.015, wob: 0.0, chaos: 0.0, arc_speed: 0.0, arcs: 0.0, dashed: 1.0, pupil: 0.0,
                motes: 0.0, ..b
            },
        }
    }
}
