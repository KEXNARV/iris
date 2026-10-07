//! ESCENA del buddy original: lo que se calcula una vez por cuadro a partir de su ánimo y se
//! reparte entre las piezas al dibujar. Dónde está el cuerpo (el ancla), cuánto respira, late
//! y se estira, si parpadea, y las capas de adentro ya ubicadas (pupila, lupa, lectura,
//! líquido) para que el centro las use si quiere.

use super::animo::Animo;
use super::guion::Params;
use super::{Centro, Original};
use crate::nucleo::color::{INK_ACCENT, INK_MAIN, INK_RED};
use crate::nucleo::contexto::{Ancla, Contexto};
use crate::nucleo::piezas::lectura::Linea;
use crate::nucleo::piezas::liquido::Nivel;
use crate::nucleo::piezas::lupa::{Donde, Vidrio};
use crate::nucleo::piezas::mirada::parpadeo;
use crate::nucleo::piezas::ojo::{self, Pupila};
use crate::nucleo::piezas::{brote, ease_out_back};
use crate::nucleo::rejilla::Grid;

pub(crate) struct Escena<'a> {
    pub p: &'a Params,
    pub animo: &'a Animo,
    /// Panel (o hijo) chico: se apagan los detalles finos.
    pub low: bool,
    pub ancla: Ancla,
    /// Cuánto lleva del arranque (0..1).
    pub boot: f64,
    pub yawn: f64,
    pub breath: f64,
    /// Respondiendo late con el volumen real de su voz; sin voz, a su propio ritmo.
    pub beat: f64,
    pub voice: f64,
    /// Estirado por el resorte.
    pub sx: f64,
    pub sy: f64,
    pub blink: bool,
    /// Hacia dónde mira y hasta dónde puede mirar.
    pub mirada: [f64; 2],
    pub lim: f64,
    pub lupa: Donde,
    pub linea: Linea,
    pub yema: Option<(f64, f64, f64)>,
    pub pupila: Pupila,
    pub vidrio: Vidrio,
    pub nivel: Nivel,
}

impl<'a> Escena<'a> {
    pub fn new<C: Centro>(o: &'a Original<C>, ctx: &Contexto, g: &Grid) -> Self {
        let a = &o.animo;
        let p = &a.p;
        let (t, st) = (ctx.t, ctx.st());
        let low = g.dh as f64 * g.k < 40.0;
        let err = if a.red > 0.35 { INK_RED } else { INK_MAIN };
        // Lo de alrededor va en el acento; con un error, en rojo como todo.
        let ring = if a.red > 0.35 { INK_RED } else { INK_ACCENT };

        // Centro del cuerpo: deriva suave + flotar (preguntando) + sacudida (error) − encorvado.
        let cx = 0.025 * (t * 0.37).sin() + 0.015 * (t * 0.91 + 1.0).sin() + a.shake * 0.05 * (t * 48.0).sin();
        let cy = 0.02 * (t * 0.29 + 2.0).sin() + p.bob * 0.05 * (t * 2.2).sin() - p.droop * 0.08;
        let boot = if p.boot > 0.01 { (st / 2.4).min(1.0) } else { 1.0 };
        let grow = if p.boot > 0.01 { ease_out_back((st / 2.0).min(1.0)) } else { 1.0 };

        let yawn = o.gestos.bostezo(t);
        let breath = p.zzz * 0.03 * (t * 1.25).sin() + (1.0 - p.zzz) * 0.012 * (t * 1.2).sin() + yawn * 0.05;
        let beat = p.ripple * 0.045 * if a.level > 0.01 { a.level * 1.6 } else { (t * 5.0).sin().abs() };
        let groove = a.groove * a.music;
        let voice = p.vu * a.level * 0.22 + groove * 0.10;
        // Con música crece desde el centro con el nivel, como el brillo de las teclas.
        let radius = p.blob_r * grow * (1.0 - 0.35 * a.deflate) * (1.0 + groove * 0.18);
        let (sx, sy) = (1.0 + a.resorte.sq, 1.0 - a.resorte.sq);
        let blink = parpadeo(ctx, a.copy);
        let ancla = Ancla { cx, cy, radio: radius, base: p.blob_r, tinta: err, borde: ring };

        let mirada = o.mirada.gaze;
        let lupa = o.lupa.donde(ctx, &ancla);
        let pupila = o.ojo.pupila(ojo::Mando {
            x: cx + mirada[0],
            y: cy + mirada[1],
            pupil: p.pupil,
            dilate: p.dilate,
            red: a.red,
            yawn,
            lens: p.lens,
            parpadeo: blink,
            tinta: err,
            du: g.du,
        });
        Escena {
            p,
            animo: a,
            low,
            boot,
            yawn,
            breath,
            beat,
            voice,
            sx,
            sy,
            blink,
            mirada,
            lim: o.mirada.lim(p.blob_r),
            lupa,
            linea: o.lectura.linea(ctx, &ancla, p.scan),
            yema: o.brote.yema(ctx, &brote::Pinta { ancla: &ancla, bud: p.bud, fase: a.phase }),
            pupila,
            vidrio: Vidrio { donde: lupa, activa: p.lens > 0.5 },
            nivel: o.liquido.nivel(ctx, &ancla),
            ancla,
        }
    }
}
