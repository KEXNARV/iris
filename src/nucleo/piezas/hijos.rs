//! HIJOS: los subagentes. Cada uno es un núcleo entero en miniatura que orbita al principal,
//! unido a él por una línea por la que suben partículas. Nace del cuerpo, hace lo suyo y
//! vuelve a fundirse (o, si falló, se apaga en rojo donde está). Cuando le llega algo de un
//! hijo, el principal lo mira.

use std::f64::consts::{PI, TAU};

use super::ease_out_back;
use super::ondas::Ondas;
use super::resorte::Resorte;
use super::Pieza;
use crate::nucleo::color::{tonos, INK_ACCENT, INK_KID, INK_RED, MAX_KID_INKS};
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::estado::{Event, Signals, State};
use crate::nucleo::rejilla::Grid;
use crate::nucleo::Core;

struct Kid {
    id: u64,
    /// Ángulo en la órbita; se reparte con los demás hijos vivos.
    a: f64,
    born: f64,
    /// Cuándo terminó y si salió bien: bien vuelve y se funde, mal se apaga en rojo donde está.
    end: Option<(f64, bool)>,
    merged: bool,
    /// El hijo es un núcleo entero en miniatura: los mismos estados, colores y detalles que el
    /// principal, sin lo de alrededor (escala, arcos, motas).
    core: Box<Core>,
    /// Partículas que suben por la línea: cuándo salieron y si llevan un error.
    pulses: Vec<(f64, bool)>,
}

pub(crate) struct Hijos {
    kids: Vec<Kid>,
    /// Hacia dónde mira el principal cuando un hijo le manda algo: (cuándo, ángulo).
    pub oido: Option<(f64, f64)>,
    /// Radio de la órbita: entre el cuerpo y la escala.
    orbita: f64,
    /// Lo que tarda un hijo en salir del cuerpo y en volver a él.
    viaje: f64,
    /// Lo que tarda una partícula en subir del hijo al principal.
    pulso: f64,
    /// El radio del cuerpo del hijo en su propio mundo: con eso se lo escala a su tamaño.
    escala: f64,
}

pub(crate) struct Paso<'a> {
    /// El que vuelve empuja al principal y lo hace destellar.
    pub resorte: &'a mut Resorte,
    pub ondas: &'a mut Ondas,
}

impl Hijos {
    pub fn new(escala: f64) -> Self {
        Hijos { kids: vec![], oido: None, orbita: 0.70, viaje: 0.8, pulso: 0.65, escala }
    }

    /// Cuántos hay (vivos o despidiéndose).
    pub fn len(&self) -> usize {
        self.kids.len()
    }

    /// ¿Hay alguno trabajando?
    pub fn vivos(&self) -> bool {
        self.kids.iter().any(|k| k.end.is_none())
    }

    /// Nace un subagente: brota del cuerpo hacia su lugar en la órbita.
    pub fn nace(&mut self, ctx: &Contexto, id: u64) {
        // Sale por donde va a quedar: el hueco que deja el reparto con uno más.
        let alive = self.kids.iter().filter(|k| k.end.is_none()).count();
        let a = kid_slot(ctx.t, alive, alive + 1, ctx.calma());
        self.kids.push(Kid { id, a, born: ctx.t, end: None, merged: false, core: Box::new(Core::mini(State::Thinking, id)), pulses: vec![] });
    }

    /// El hijo hizo algo (pidió una herramienta o le llegó su resultado): una partícula sube.
    pub fn pulso(&mut self, ctx: &Contexto, id: u64, error: bool) {
        if let Some(k) = self.kids.iter_mut().find(|k| k.id == id) {
            k.pulses.push((ctx.t, error));
            k.core.fire(if error { Event::Error } else { Event::Key });
        }
    }

    /// Terminó el subagente: si salió bien vuelve al cuerpo; si no, se apaga en rojo.
    pub fn fin(&mut self, ctx: &Contexto, id: u64, ok: bool) {
        if let Some(k) = self.kids.iter_mut().find(|k| k.id == id && k.end.is_none()) {
            k.end = Some((ctx.t, ok));
            if !ok {
                k.core.fire(Event::Error);
            }
        }
    }

    /// El motor se fue: los hijos se van con él, sin ceremonia.
    pub fn fuera(&mut self) {
        self.kids.clear();
        self.oido = None;
    }

    /// Las tintas de los hijos, cada una con el color de lo que está haciendo.
    pub fn tintas(&self) -> impl Iterator<Item = [[f64; 3]; 4]> + '_ {
        self.kids.iter().take(MAX_KID_INKS).map(|k| tonos(k.core.tinta()))
    }
}

impl Pieza for Hijos {
    type Paso<'a> = Paso<'a>;
    /// El cuerpo del principal: de su borde sale la línea.
    type Pinta<'a> = &'a Ancla;

    /// Su estado, su lugar en la órbita, su mirada y su despedida.
    fn step(&mut self, ctx: &Contexto, m: Paso, dt: f64) {
        let t = ctx.t;
        let calm = ctx.calma();
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
            k.pulses.retain(|p| t - p.0 < self.pulso);
            // Justo al mandar algo, o al volver, mira al principal (que está hacia el centro).
            let fresh = k.pulses.last().is_some_and(|p| t - p.0 < 0.3);
            k.core.mirar_hacia((fresh || k.end.is_some()).then(|| [-0.8 * k.a.cos(), -0.8 * k.a.sin()]));
            k.core.step(dt, want, &child);
            k.core.take_fired();
            if let Some((t0, true)) = k.end {
                if !k.merged && t - t0 >= self.viaje {
                    k.merged = true;
                    merged.push(k.a);
                }
            }
        }
        // Cada partícula que llega al principal le hace mirar hacia ese hijo.
        for k in &self.kids {
            if k.pulses.iter().any(|p| (t - p.0 - self.pulso).abs() < dt.max(0.02)) {
                self.oido = Some((t, k.a));
            }
        }
        for a in merged {
            m.resorte.empujar(2.4, calm);
            m.ondas.listo(t);
            self.oido = Some((t, a));
        }
        let viaje = self.viaje;
        self.kids.retain(|k| match k.end {
            Some((t0, true)) => t - t0 < viaje + 0.05,
            Some((t0, false)) => t - t0 < 1.4,
            None => true,
        });
    }

    /// Los hijos, con su línea al principal y las partículas que suben por ella.
    fn paint(&self, g: &mut Grid, ctx: &Contexto, ancla: &Ancla) {
        let t = ctx.t;
        let (cx, cy, radius) = (ancla.cx, ancla.cy, ancla.radio);
        let alive = self.kids.iter().filter(|k| k.end.is_none()).count();
        // Radio del cuerpo de un hijo: con muchos se achican; nunca por debajo de lo que se
        // lee como un ojo.
        let kr = (if alive > 4 { 0.10_f64 } else { 0.13 }).max(2.6 * g.du);
        for (n, k) in self.kids.iter().enumerate() {
            // Cuánto salió del cuerpo (0 adentro, 1 en la órbita) y qué tamaño tiene.
            let (out, size) = match k.end {
                None => {
                    let u = ((t - k.born) / self.viaje).min(1.0);
                    (ease_out_back(u).min(1.08), 0.35 + 0.65 * ease_out_back(u))
                }
                Some((t0, true)) => {
                    let u = ((t - t0) / self.viaje).min(1.0);
                    let back = 1.0 - u * u;
                    (back, 0.4 + 0.6 * back)
                }
                Some((t0, false)) => (1.0, 1.0 - 0.8 * ((t - t0) / 1.4).min(1.0)),
            };
            let red = k.core.alarma() > 0.35;
            let ink = INK_KID + n.min(MAX_KID_INKS - 1) as u8;
            let bob = 0.012 * (t * 1.7 + k.id as f64).sin();
            let orbit = radius + (self.orbita - radius) * out;
            let (kx, ky) = (orbit * k.a.cos(), orbit * k.a.sin() + bob);
            let r = kr * size;

            // La línea: del borde del principal al del hijo, en el acento. Puntos que fluyen
            // hacia adentro.
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
                    // Una cresta que viaja del hijo al principal: la línea «respira» hacia el centro.
                    let tone = if (u * 3.0 + flow).fract() < 0.18 { 3 } else { 2 };
                    g.plot(bx + (ax - bx) * u, by + (ay - by) * u, tone, line);
                }
                // Partículas: una por cada cosa que hizo el hijo, en su color, del hijo al principal.
                let d = g.du;
                for &(p0, err) in &k.pulses {
                    let u = ((t - p0) / self.pulso).clamp(0.0, 1.0);
                    let e = u * u * (3.0 - 2.0 * u);
                    let (px, py) = (bx + (ax - bx) * e, by + (ay - by) * e);
                    let pink = if err { INK_RED } else { ink };
                    for oy in [-d, 0.0, d] {
                        for ox in [-d, 0.0, d] {
                            g.plot(px + ox, py + oy, 3, pink);
                        }
                    }
                    // Estela hacia el hijo.
                    for (q, back) in [(3, 0.07), (2, 0.13)] {
                        let e2 = (e - back).max(0.0);
                        let (qx, qy) = (bx + (ax - bx) * e2, by + (ay - by) * e2);
                        g.plot(qx, qy, q, pink);
                        g.plot(qx + d, qy, q, pink);
                    }
                }
            }

            // El hijo: su propio núcleo, a escala y en su lugar, pintado con su tinta.
            g.view(kx, ky, r / self.escala, ink);
            k.core.paint(g);
            g.unview();
        }
    }
}

/// Dónde va el hijo número `slot` de `n` en este momento: repartidos parejo, girando
/// despacio, el primero arriba a la izquierda (el reloj ocupa la esquina de la derecha).
fn kid_slot(t: f64, slot: usize, n: usize, calm: f64) -> f64 {
    PI * 0.75 + t * 0.06 * calm + slot as f64 * TAU / n.max(1) as f64
}
