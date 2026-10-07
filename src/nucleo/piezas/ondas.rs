//! ONDAS: el rastro que dejan los eventos alrededor del cuerpo. Cada tecla, una onda corta;
//! cada fin de turno, un anillo verde que se expande hasta la escala; cada subagente que
//! vuelve, una gota que viaja desde la escala y se funde (y al llegar empuja al cuerpo).

use std::f64::consts::TAU;

use super::resorte::Resorte;
use super::Pieza;
use crate::nucleo::azar::Azar;
use crate::nucleo::color::{INK_GREEN, INK_MAIN};
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::estado::Event;
use crate::nucleo::rejilla::Grid;

pub(crate) struct Ondas {
    /// Cuándo fue cada tecla.
    teclas: Vec<f64>,
    /// Cuándo fue cada fin de turno (o cada fundido): la escala también los lee.
    pub listos: Vec<f64>,
    /// Gotas en viaje: cuándo salieron, desde qué ángulo y si ya llegaron.
    gotas: Vec<(f64, f64, bool)>,
    /// Lo que tarda una gota en llegar, y el empujón que da.
    viaje: f64,
    golpe: f64,
}

impl Ondas {
    pub fn new() -> Self {
        Ondas { teclas: vec![], listos: vec![], gotas: vec![], viaje: 0.7, golpe: 2.4 }
    }

    /// Un anillo verde, ahora.
    pub fn listo(&mut self, t: f64) {
        self.listos.push(t);
    }
}

impl Pieza for Ondas {
    /// Las gotas que llegan empujan al cuerpo.
    type Paso<'a> = &'a mut Resorte;
    type Pinta<'a> = &'a Ancla;

    fn evento(&mut self, ev: Event, ctx: &Contexto, azar: &mut Azar) {
        let t = ctx.t;
        match ev {
            Event::Key => self.teclas.push(t),
            Event::Done => self.listos.push(t),
            Event::Merge => {
                let a = azar.rand() * TAU;
                self.gotas.push((t, a, false));
            }
            _ => {}
        }
    }

    fn step(&mut self, ctx: &Contexto, resorte: &mut Resorte, _dt: f64) {
        let t = ctx.t;
        self.listos.retain(|b| t - b < 1.2);
        self.teclas.retain(|k0| t - k0 < 0.5);
        let mut pops = 0;
        for m in &mut self.gotas {
            if t - m.0 > self.viaje && !m.2 {
                m.2 = true;
                pops += 1;
            }
        }
        for _ in 0..pops {
            resorte.empujar(self.golpe, ctx.calma());
            self.listos.push(t);
        }
        self.gotas.retain(|m| t - m.0 < 1.0);
    }

    fn paint(&self, g: &mut Grid, ctx: &Contexto, ancla: &Ancla) {
        let (t, base) = (ctx.t, ancla.base);
        // Ondas de tecla.
        for k0 in &self.teclas {
            let u = (t - k0) / 0.45;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let r = base + 0.05 + u * 0.16;
            for q in 0..36 {
                let (x, y) = ancla.punto(r, q as f64 * TAU / 36.0);
                g.plot(x, y, if u < 0.5 { 2 } else { 1 }, INK_MAIN);
            }
        }
        // Listo: anillo verde que se expande hasta la escala.
        for b in &self.listos {
            let u = (t - b) / 1.1;
            if !(0.0..=0.75).contains(&u) {
                continue;
            }
            let r = base + 0.04 + u * 0.62;
            let n = 50 + (u * 40.0) as usize;
            for q in 0..n {
                let (x, y) = ancla.punto(r, q as f64 * TAU / n as f64);
                g.plot(x, y, if u < 0.45 { 3 } else { 2 }, INK_GREEN);
            }
        }
        // Vuelve el subagente: una gota viaja desde la escala y se funde.
        for &(t0, a, _) in &self.gotas {
            let u = (t - t0) / self.viaje;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let r = 0.86 - u * (0.86 - base);
            let rr = 0.07 * (1.0 - u * 0.5);
            let (bx, by) = ancla.punto(r, a);
            for q in 0..14 {
                let b = q as f64 * TAU / 14.0;
                g.plot(bx + rr * b.cos(), by + rr * b.sin(), 3, INK_MAIN);
            }
        }
    }
}
