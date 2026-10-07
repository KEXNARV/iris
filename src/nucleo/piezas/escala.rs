//! ESCALA: las marcas ancladas al marco, alrededor de todo. Hacen de medidor: la voz al
//! escuchar, el avance al transcribir y al probar, el barrido del radar, la música en reposo,
//! y se encienden en verde con un fin de turno o pruebas que pasan.

use std::f64::consts::{PI, TAU};

use super::Pieza;
use crate::nucleo::color::{INK_ACCENT, INK_GREEN, INK_RED};
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Escala {
    marcas: usize,
    /// Cada cuántas marcas va una mayor (más larga, y la que parpadea preguntando).
    mayor: usize,
    /// Radios de las marcas; el tercero solo en las mayores.
    radios: [f64; 3],
    /// Hasta qué fracción del medidor de música va en el acento; de ahí en adelante, rojo
    /// (como la barra del teclado).
    medidor: f64,
}

/// Lo que la escala necesita saber del buddy en este cuadro.
pub(crate) struct Pinta<'a> {
    pub ancla: &'a Ancla,
    /// Cuánto lleva del arranque (0..1): las marcas aparecen en orden.
    pub boot: f64,
    /// Nivel de la voz, 0..1.
    pub nivel: f64,
    /// Giro de lo de alrededor, el mismo que el de los arcos.
    pub giro: f64,
    /// Música: nivel y cuánto está bailando.
    pub music: f64,
    pub groove: f64,
    /// Cuándo hubo un fin de turno: un barrido verde.
    pub listos: &'a [f64],
    /// Destellos que la encienden entera: pruebas que pasan, copia, error.
    pub pass: f64,
    pub copy: f64,
    pub red: f64,
    pub low: bool,
    /// Cuánto de cada medidor: voz, avance, radar, preguntando y pruebas.
    pub vu: f64,
    pub fill: f64,
    pub sweep: f64,
    pub cardinal: f64,
    pub tests: f64,
}

impl Escala {
    pub fn new() -> Self {
        Escala { marcas: 24, mayor: 6, radios: [0.94, 0.90, 0.86], medidor: 0.5 }
    }
}

impl Pieza for Escala {
    type Paso<'a> = ();
    type Pinta<'a> = Pinta<'a>;

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let (t, st) = (ctx.t, ctx.st());
        let n = self.marcas as f64;
        for k in 0..self.marcas {
            if k as f64 / n > m.boot * 1.05 {
                continue;
            }
            let a = PI / 2.0 - k as f64 * TAU / n;
            let (mut tone, mut ink) = (1, m.ancla.borde);
            if m.vu > 0.01 && (k as f64 / n) < m.nivel * 1.15 * m.vu {
                tone = 3;
            }
            if m.fill > 0.01 && k as f64 / n <= (st * 0.35).fract() * m.fill {
                tone = 3;
            }
            if m.sweep > 0.01 {
                let d = (a + m.giro * 1.3).rem_euclid(TAU);
                if d < 0.7 * m.sweep {
                    tone = if d < 0.25 { 3 } else { 2 };
                }
            }
            if m.cardinal > 0.01 && k % self.mayor == 0 && (t * 4.0).sin() > 0.0 {
                tone = 3;
            }
            // Música: el medidor de la barra del teclado, llenándose desde abajo hacia los dos
            // lados, en el acento y rojo cuando el golpe pasa de la mitad.
            if m.groove > 0.01 {
                let mitad = n / 2.0;
                let from = (k as f64 - mitad).abs() / mitad;
                if from <= m.music * m.groove {
                    tone = 3;
                    ink = if from < self.medidor * 0.6 { INK_ACCENT } else { INK_RED };
                }
            }
            if m.tests > 0.5 && (k as f64) < (st * 9.0) % (n + 1.0) {
                tone = 3; // avance de la corrida; el resultado lo dicen Pass o Error
            }
            for b in m.listos {
                let u = (t - b) / 1.1;
                if u > 0.45 && u < 1.0 && ((k as f64 / n - (u - 0.45) * 1.8).rem_euclid(1.0)) < 0.08 {
                    tone = 3;
                    ink = INK_GREEN;
                }
            }
            if m.pass > 0.3 {
                tone = 3;
                ink = INK_GREEN;
            }
            if m.copy > 0.5 {
                tone = 3;
            }
            if m.red > 0.35 {
                tone = tone.max(2);
            }
            g.polar(self.radios[0], a, tone, ink);
            g.polar(self.radios[1], a, tone, ink);
            if k % self.mayor == 0 && !m.low {
                g.polar(self.radios[2], a, tone, ink);
            }
        }
    }
}
