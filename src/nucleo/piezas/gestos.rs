//! GESTOS: lo que hace cuando lleva rato quieto, para que no repita siempre lo mismo. Se
//! estira, mira el reloj, mira el chat o, si ya pasó mucho, bosteza.

use std::f64::consts::PI;

use super::resorte::Resorte;
use super::Pieza;
use crate::nucleo::azar::Azar;
use crate::nucleo::contexto::Contexto;
use crate::nucleo::estado::State;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Gesture {
    Stretch,
    LookClock,
    LookChat,
    Yawn,
}

impl Gesture {
    /// Lo que dura.
    fn len(self) -> f64 {
        match self {
            Gesture::Stretch => 0.8,
            Gesture::LookClock => 1.6,
            Gesture::LookChat => 1.2,
            Gesture::Yawn => 2.2,
        }
    }
}

pub(crate) struct Gestos {
    /// El que está haciendo y cuándo empezó.
    actual: Option<(Gesture, f64)>,
    proximo: f64,
    /// Segundos sin actividad para el primero, y para bostezar.
    quieto: f64,
    sueño: f64,
}

pub(crate) struct Paso<'a> {
    pub azar: &'a mut Azar,
    /// Estirarse es un empujón.
    pub resorte: &'a mut Resorte,
    /// Bailando no hace gestos.
    pub groove: f64,
}

impl Gestos {
    pub fn new() -> Self {
        Gestos { actual: None, proximo: 20.0, quieto: 15.0, sueño: 95.0 }
    }

    pub fn actual(&self) -> Option<Gesture> {
        self.actual.map(|(g, _)| g)
    }

    /// Cualquier cambio de estado lo corta.
    pub fn cortar(&mut self) {
        self.actual = None;
    }

    /// Cuánto está bostezando (0..1).
    pub fn bostezo(&self, t: f64) -> f64 {
        match self.actual {
            Some((Gesture::Yawn, t0)) => (PI * ((t - t0) / 2.2).min(1.0)).sin(),
            _ => 0.0,
        }
    }
}

impl Pieza for Gestos {
    type Paso<'a> = Paso<'a>;
    type Pinta<'a> = ();

    fn step(&mut self, ctx: &Contexto, m: Paso, _dt: f64) {
        let (t, idle) = (ctx.t, ctx.sig.idle);
        if ctx.estado == State::Idle && m.groove < 0.05 && idle > self.quieto && t > self.proximo && self.actual.is_none() {
            let g = if idle > self.sueño {
                Gesture::Yawn
            } else {
                [Gesture::Stretch, Gesture::LookClock, Gesture::LookChat][(m.azar.rand() * 3.0) as usize % 3]
            };
            if g == Gesture::Stretch {
                m.resorte.empujar(3.5, ctx.calma());
            }
            self.actual = Some((g, t));
            self.proximo = t + 8.0 + m.azar.rand() * 8.0;
        }
        if let Some((g, t0)) = self.actual {
            if t - t0 > g.len() || ctx.estado != State::Idle {
                self.actual = None;
            }
        }
    }
}
