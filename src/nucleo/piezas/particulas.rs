//! PARTÍCULAS: las motas de la corona, en tres capas de profundidad. Caen al centro
//! escuchando, salen respondiendo y flotan el resto. Una «cámara» que se mece y sigue un poco
//! a la mirada las corre según lo cerca que estén, y eso da el paralaje. Las lejanas van
//! detrás de todo (solo donde no hay nada); las cercanas, delante de todo.

use std::f64::consts::TAU;

use super::Pieza;
use crate::nucleo::azar::Azar;
use crate::nucleo::color::INK_ACCENT;
use crate::nucleo::contexto::Contexto;
use crate::nucleo::estado::State;
use crate::nucleo::rejilla::Grid;

#[derive(Clone, Copy)]
struct Mote {
    a: f64,
    r: f64,
    s: f64,
    /// Profundidad: 0 al fondo, 1 al frente. Lo cercano brilla más, es más grande, va más
    /// rápido y se desplaza más con la mirada: eso es lo que da el paralaje.
    z: f64,
}

pub(crate) struct Motas {
    motas: Vec<Mote>,
    /// Cuántas hay con todo encendido.
    cuantas: usize,
}

pub(crate) struct Paso {
    /// Radio del cuerpo: las lejanas viven por fuera de él.
    pub base: f64,
    /// Nivel de la voz (escuchando caen más rápido).
    pub nivel: f64,
    /// Tragando (de escuchar a transcribir): se van al centro.
    pub traga: f64,
    /// Bailando giran más rápido.
    pub baile: f64,
}

pub(crate) struct Pinta {
    /// Cuánto se ven (0..1) y cuánto lleva del arranque.
    pub motes: f64,
    pub boot: f64,
    /// Compactando: se aprietan hacia adentro.
    pub press: f64,
    /// La mirada, que arrastra a la cámara.
    pub mirada: [f64; 2],
    pub low: bool,
}

impl Motas {
    /// Las del arranque, cada una en su capa y en su banda. Gasta azar aunque después se
    /// vacíen (los hijos): así cada uno sigue con el mismo azar de siempre.
    pub fn sembrar(azar: &mut Azar) -> Self {
        let cuantas = 24;
        let motas = (0..cuantas)
            .map(|k| {
                let z = ((k % 3) as f64 / 2.0 + (azar.rand() - 0.5) * 0.2).clamp(0.0, 1.0);
                let (lo, hi) = mote_band(z, 0.40);
                Mote { a: k as f64 * TAU / cuantas as f64 + azar.rand(), r: lo + azar.rand() * (hi - lo), s: azar.rand() - 0.5, z }
            })
            .collect();
        Motas { motas, cuantas }
    }

    pub fn vaciar(&mut self) {
        self.motas.clear();
    }

    /// La «cámara» se mece despacio y sigue un poco a la mirada.
    fn camera(ctx: &Contexto, mirada: [f64; 2]) -> [f64; 2] {
        let c = ctx.calma();
        [0.12 * (ctx.t * 0.23).sin() * c - mirada[0] * 0.8, 0.07 * (ctx.t * 0.17 + 1.0).sin() * c - mirada[1] * 0.8]
    }
}

/// Dónde cae una mota: lo cercano se corre mucho con la cámara, lo lejano casi nada.
fn mote_pos(m: &Mote, cam: [f64; 2], press: f64, t: f64) -> (f64, f64) {
    let r = m.r * (1.0 - press * 0.25 * (1.0 + (t * 5.0).sin()));
    let shift = 0.1 + m.z * m.z * 1.9;
    (r * m.a.cos() + cam[0] * shift, r * m.a.sin() + cam[1] * shift)
}

impl Pieza for Motas {
    type Paso<'a> = Paso;
    type Pinta<'a> = Pinta;

    /// Caen al centro escuchando, salen respondiendo, flotan el resto.
    fn step(&mut self, ctx: &Contexto, m: Paso, dt: f64) {
        let (t, calm) = (ctx.t, ctx.calma());
        let (r0, shown, level, swallow, groove) = (m.base, ctx.estado, m.nivel, m.traga, m.baile);
        for m in &mut self.motas {
            if swallow > 0.01 {
                m.r += (r0 - m.r) * (dt * 6.0).min(1.0);
                if m.r < r0 + 0.03 {
                    m.r = 0.86;
                }
            } else if shown == State::Listening {
                m.r -= dt * (0.18 + level * 0.5) * mote_speed(m.z);
                if m.r < r0 + 0.08 {
                    m.r = mote_band(m.z, r0).1;
                }
            } else if shown == State::Speaking {
                m.r += dt * 0.22 * mote_speed(m.z);
                if m.r > mote_band(m.z, r0).1 {
                    m.r = r0 + 0.1;
                }
            } else {
                let (lo, hi) = mote_band(m.z, r0);
                m.a += dt * 0.12 * (1.0 + m.s * 0.5) * mote_speed(m.z) * calm * (1.0 + groove * 4.0);
                m.r += (t * 0.7 + m.a * 3.0).sin() * dt * 0.02;
                m.r = m.r.clamp(lo, hi);
            }
        }
    }

    fn paint(&self, g: &mut Grid, ctx: &Contexto, m: Pinta) {
        let t = ctx.t;
        let cam = Motas::camera(ctx, m.mirada);
        // Lejanas y medias: al final y solo en celdas vacías, así el anillo, los arcos y el
        // cuerpo las tapan. Las cercanas van después, por encima de todo.
        if m.motes > 0.05 && !m.low {
            let n = (self.cuantas as f64 * m.motes * m.boot).round() as usize;
            for mo in self.motas.iter().take(n).filter(|mo| mo.z < 0.66) {
                let (x, y) = mote_pos(mo, cam, m.press, t);
                g.plot_under(x, y, 1);
                if mo.z >= 0.33 {
                    g.plot_under(x + g.du, y, 1);
                }
            }
        }
        // Cercanas: al frente de todo, al color pleno, más grandes y con estela.
        if m.motes > 0.05 {
            let n = (self.cuantas as f64 * m.motes * m.boot).round() as usize;
            let d = g.du;
            for mo in self.motas.iter().take(n).filter(|mo| mo.z >= 0.66) {
                let (x, y) = mote_pos(mo, cam, m.press, t);
                for (ox, oy) in [(0.0, 0.0), (d, 0.0), (0.0, d), (d, d)] {
                    g.plot(x + ox, y + oy, 3, INK_ACCENT);
                }
                // Estela hacia atrás en su giro.
                for k in 1..3 {
                    let back = Mote { a: mo.a - k as f64 * 0.05, ..*mo };
                    let (x, y) = mote_pos(&back, cam, m.press, t);
                    g.plot(x, y, if k == 1 { 2 } else { 1 }, INK_ACCENT);
                }
            }
        }
    }
}

/// Entre qué radios vive una mota según su profundidad: las lejanas entre el cuerpo y el
/// anillo, las cercanas por fuera, encima de la escala.
fn mote_band(z: f64, blob_r: f64) -> (f64, f64) {
    if z < 0.33 {
        (blob_r + 0.12, 0.72)
    } else if z < 0.66 {
        (0.55, 0.88)
    } else {
        (0.70, 1.05)
    }
}

/// Velocidad relativa de una mota: las cercanas van ~10× más rápido.
fn mote_speed(z: f64) -> f64 {
    0.25 + 2.4 * z * z
}
