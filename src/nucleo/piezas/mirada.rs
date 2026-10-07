//! MIRADA: hacia dónde mira el buddy. Salta rápido hacia su objetivo, como una sacada: en
//! espera mira al azar; en cada estado, hacia donde pasan las cosas (la entrada, el reloj,
//! las tareas, el hijo que le habla). Con una alucinación de whisper, niega.

use std::f64::consts::{PI, TAU};

use super::gestos::Gesture;
use super::Pieza;
use crate::nucleo::azar::Azar;
use crate::nucleo::contexto::Contexto;
use crate::nucleo::estado::State;

pub(crate) struct Mirada {
    /// Hacia dónde mira ahora, en unidades del mundo desde el centro del cuerpo.
    pub gaze: [f64; 2],
    /// El objetivo de la última sacada en espera, y cuándo toca la próxima.
    gaze_t: [f64; 2],
    next_saccade: f64,
    /// Cuánto puede apartarse, en radios del cuerpo.
    alcance: f64,
}

pub(crate) struct Paso<'a> {
    pub azar: &'a mut Azar,
    /// Radio del cuerpo: la mirada no sale de él.
    pub base: f64,
    pub gesto: Option<Gesture>,
    /// Un hijo le mandó algo: (cuándo, ángulo).
    pub oido: Option<(f64, f64)>,
    /// Fase del cuerpo (delegando mira donde va el brote).
    pub fase: f64,
    /// Negando (0..1).
    pub nope: f64,
}

impl Mirada {
    pub fn new() -> Self {
        Mirada { gaze: [0.0; 2], gaze_t: [0.0; 2], next_saccade: 0.0, alcance: 0.42 }
    }

    /// Hasta dónde llega desde el centro.
    pub fn lim(&self, base: f64) -> f64 {
        base * self.alcance
    }

    fn objetivo(&mut self, ctx: &Contexto, m: &mut Paso) -> [f64; 2] {
        use State::*;
        let (t, st) = (ctx.t, ctx.st());
        let lim = self.lim(m.base);
        if let Some([x, y]) = ctx.mira {
            return [lim * x, lim * y];
        }
        if let Some(g) = m.gesto {
            match g {
                // El reloj está arriba a la derecha de la pantalla; el chat, a la izquierda.
                Gesture::LookClock => return [lim * 0.8, lim * 0.85],
                Gesture::LookChat => return [-lim * 0.9, 0.0],
                _ => {}
            }
        }
        // Un hijo le mandó algo: lo mira un momento, salvo que esté ocupado con vos.
        if let Some((t0, a)) = m.oido {
            if t - t0 < 0.7 && matches!(ctx.estado, Idle | Thinking | Delegating | Speaking | Reading | Searching) {
                return [lim * 0.9 * a.cos(), lim * 0.9 * a.sin()];
            }
        }
        match ctx.estado {
            Idle => {
                if t > self.next_saccade {
                    let a = m.azar.rand() * TAU;
                    let r = m.azar.rand() * lim;
                    self.gaze_t = [r * a.cos(), r * a.sin() * 0.8];
                    self.next_saccade = t + 1.2 + m.azar.rand() * 2.5;
                }
                self.gaze_t
            }
            Typing => [-lim * 0.6, -lim * 0.8],
            Listening => [0.0, -lim * 0.7],
            NoVoice => [lim * 0.9 * (t * 1.4).sin().signum(), -lim * 0.2],
            Transcribing => [0.02 * (t * 30.0).sin(), 0.02 * (t * 27.0).cos()],
            Thinking => [lim * 0.5 * (t * 0.7).sin(), lim * 0.75],
            Reading => {
                let line = st * 0.9;
                [-lim * 0.8 + line.fract() * lim * 1.6, lim * 0.6 - (line.floor() % 4.0) * lim * 0.4]
            }
            Searching => [m.base * 0.55 * (st * 1.3).sin(), m.base * 0.45 * (st * 2.1 + 1.0).sin()],
            Planning => {
                let (n, d) = ctx.sig.todos;
                let a = if n > 0 { PI / 2.0 - d.min(n - 1) as f64 * TAU / n as f64 } else { PI / 2.0 };
                [lim * 0.9 * a.cos(), lim * 0.9 * a.sin()]
            }
            Editing => [lim * 0.6 * (t * 2.6).cos(), lim * 0.6 * (t * 2.6).sin()],
            Running => [0.0, -lim * 0.75],
            Testing => {
                let a = PI / 2.0 - (st * 9.0 % 24.0) * TAU / 24.0;
                [lim * 0.8 * a.cos(), lim * 0.8 * a.sin()]
            }
            Git => [lim * 0.3, lim * 0.9],
            Web => [lim * 0.7 * (t * 0.8).cos(), lim * 0.7 * (t * 0.8).sin()],
            Delegating => {
                let a = m.fase * 0.9;
                [lim * 0.8 * a.cos(), lim * 0.8 * a.sin()]
            }
            Speaking => [-lim * 0.8, 0.0],
            Asking => [-lim * 0.4, -lim * 0.8],
            _ => [0.0, 0.0],
        }
    }
}

impl Pieza for Mirada {
    type Paso<'a> = Paso<'a>;
    type Pinta<'a> = ();

    fn step(&mut self, ctx: &Contexto, mut m: Paso, dt: f64) {
        let t = ctx.t;
        let gt = self.objetivo(ctx, &mut m);
        let gt = if m.nope > 0.01 { [m.base * 0.4 * (t * 22.0).sin() * (m.nope * 2.0).min(1.0), gt[1]] } else { gt };
        let kg = (dt * 14.0).min(1.0);
        for i in 0..2 {
            self.gaze[i] += (gt[i] - self.gaze[i]) * kg;
        }
    }
}

/// Parpadea cada tanto en espera y escribiendo, y con un destello (copia, despertar).
pub(crate) fn parpadeo(ctx: &Contexto, copy: f64) -> bool {
    (matches!(ctx.estado, State::Idle | State::Typing) && (ctx.t % 4.3) < 0.12) || copy > 0.3
}
