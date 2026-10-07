//! ORIGINAL: el buddy de siempre. Su guion (cuánto de cada pieza por estado), su ánimo y
//! todas sus piezas, en el orden en que se dibujan. En el centro va lo que se le ponga: el
//! blob, o la cara de Baymax (ver `buddies::baymax`), y el resto sigue igual.
//!
//! El orden importa: la rejilla se queda con el punto más brillante, así que casi todo da lo
//! mismo, pero las motas lejanas se dibujan solo donde no hay nada y por eso van al final.

use super::Buddy;
use crate::nucleo::azar::Azar;
use crate::nucleo::color::{tonos, GREEN, RED, WARM};
use crate::nucleo::contexto::{Aviso, Contexto};
use crate::nucleo::estado::State;
use crate::nucleo::piezas::arcos::{self, Arcos};
use crate::nucleo::piezas::brote::{self, Brote};
use crate::nucleo::piezas::cuerpo::{self, Blob, Forma};
use crate::nucleo::piezas::escala::{self, Escala};
use crate::nucleo::piezas::gestos::{self, Gestos};
use crate::nucleo::piezas::git::{self, Git};
use crate::nucleo::piezas::hijos::{self, Hijos};
use crate::nucleo::piezas::ideas::{self, Ideas};
use crate::nucleo::piezas::lectura::Lectura;
use crate::nucleo::piezas::liquido::Liquido;
use crate::nucleo::piezas::lupa::{self, Lupa};
use crate::nucleo::piezas::marcas::{Cola, Tareas};
use crate::nucleo::piezas::mirada::{self, Mirada};
use crate::nucleo::piezas::ojo::Ojo;
use crate::nucleo::piezas::ondas::Ondas;
use crate::nucleo::piezas::paquetes::{self, Paquetes};
use crate::nucleo::piezas::particulas::{self, Motas};
use crate::nucleo::piezas::prensas::{self, Prensas};
use crate::nucleo::piezas::pregunta::{self, Pregunta};
use crate::nucleo::piezas::puntadas::{self, Puntadas};
use crate::nucleo::piezas::radar::{self, Radar};
use crate::nucleo::piezas::voz::{self, Voz};
use crate::nucleo::piezas::zetas::{self, Zetas};
use crate::nucleo::piezas::{Pieza, Relleno};
use crate::nucleo::rejilla::Grid;

pub(crate) mod animo;
pub(crate) mod escena;
pub(crate) mod guion;

use animo::Animo;
use escena::Escena;
use guion::BASE;

/// Lo que va en el centro del original.
pub(crate) trait Centro {
    fn pintar(&self, g: &mut Grid, ctx: &Contexto, e: &Escena);

    /// Los tonos de su tinta propia (`INK_CARA`), a partir del color del estado.
    fn tonos(&self, col: [f64; 3]) -> [[f64; 3]; 4];
}

pub(crate) struct Original<C: Centro> {
    /// Es un hijo: sin escala, arcos ni motas, que en miniatura solo ensucian.
    mini: bool,
    azar: Azar,
    pub animo: Animo,
    escala: Escala,
    tareas: Tareas,
    /// Tres largos por fuera y seis cortos por dentro, en sentidos opuestos.
    arcos: [Arcos; 2],
    cola: Cola,
    prensas: Prensas,
    voz: Voz,
    pub ondas: Ondas,
    radar: Radar,
    ideas: Ideas,
    paquetes: Paquetes,
    puntadas: Puntadas,
    git: Git,
    pregunta: Pregunta,
    zetas: Zetas,
    brote: Brote,
    lupa: Lupa,
    lectura: Lectura,
    liquido: Liquido,
    ojo: Ojo,
    centro: C,
    hijos: Hijos,
    motas: Motas,
    mirada: Mirada,
    gestos: Gestos,
}

impl<C: Centro> Original<C> {
    pub fn nuevo(centro: C) -> Self {
        let mut azar = Azar::new();
        let motas = Motas::sembrar(&mut azar);
        Original {
            mini: false,
            azar,
            animo: Animo::new(State::Booting),
            escala: Escala::new(),
            tareas: Tareas::new(),
            arcos: [Arcos::afuera(), Arcos::adentro()],
            cola: Cola::new(),
            prensas: Prensas::new(),
            voz: Voz::new(),
            ondas: Ondas::new(),
            radar: Radar::new(),
            ideas: Ideas::new(),
            paquetes: Paquetes::new(),
            puntadas: Puntadas::new(),
            git: Git::new(),
            pregunta: Pregunta::new(),
            zetas: Zetas::new(),
            brote: Brote::new(),
            lupa: Lupa::new(),
            lectura: Lectura::new(),
            liquido: Liquido::new(),
            ojo: Ojo::new(),
            centro,
            hijos: Hijos::new(BASE.blob_r),
            motas,
            mirada: Mirada::new(),
            gestos: Gestos::new(),
        }
    }
}

impl Original<Blob> {
    /// Un hijo: ya despierto en `state`, sin arranque ni motas, con su propio azar.
    pub fn mini(state: State, seed: u64) -> Self {
        let mut o = Original::nuevo(Blob::new());
        o.mini = true;
        o.motas.vaciar();
        o.animo.p = state.params();
        o.animo.col = state.rgb();
        o.azar.sembrar(seed);
        o
    }
}

/// El blob en el centro, con sus capas de adentro de la más alta a la más baja: pupila >
/// lupa > línea de lectura > líquido (y debajo, la trama del cuerpo).
impl Centro for Blob {
    fn pintar(&self, g: &mut Grid, ctx: &Contexto, e: &Escena) {
        let (p, a) = (e.p, e.animo);
        let rellenos: [&dyn Relleno; 4] = [&e.pupila, &e.vidrio, &e.linea, &e.nivel];
        let forma = Forma {
            radius: e.ancla.radio,
            breath: e.breath,
            beat: e.beat,
            voice: e.voice,
            amp: p.amp,
            phase: a.phase,
            chaos: p.chaos + a.red * 2.0,
            red: a.red,
            t: ctx.t,
        };
        let m = cuerpo::Pinta {
            cx: e.ancla.cx,
            cy: e.ancla.cy,
            sx: e.sx,
            sy: e.sy,
            forma,
            dashed: p.dashed,
            stitch: p.stitch,
            yema: e.yema,
            tinta: e.ancla.tinta,
            rellenos: &rellenos,
        };
        self.paint(g, ctx, m);
    }

    /// El blob no tiene tinta propia: se pinta con la del estado.
    fn tonos(&self, col: [f64; 3]) -> [[f64; 3]; 4] {
        tonos(col)
    }
}

impl<C: Centro + 'static> Buddy for Original<C> {
    fn step(&mut self, ctx: &Contexto, dt: f64) {
        let con_hijos = self.hijos.vivos();
        self.animo.acercar(ctx, dt, con_hijos);
        self.animo.apagar(ctx, dt);
        self.ondas.step(ctx, &mut self.animo.resorte, dt);
        self.hijos.step(ctx, hijos::Paso { resorte: &mut self.animo.resorte, ondas: &mut self.ondas }, dt);
        self.animo.ritmo(ctx, dt);
        // Gestos ocasionales cuando lleva rato quieto.
        self.gestos.step(ctx, gestos::Paso { azar: &mut self.azar, resorte: &mut self.animo.resorte, groove: self.animo.groove }, dt);
        let a = &self.animo;
        self.mirada.step(
            ctx,
            mirada::Paso {
                azar: &mut self.azar,
                base: a.p.blob_r,
                gesto: self.gestos.actual(),
                oido: self.hijos.oido,
                fase: a.phase,
                nope: a.nope,
            },
            dt,
        );
        self.motas.step(ctx, particulas::Paso { base: a.p.blob_r, nivel: a.level, traga: a.swallow, baile: a.groove * a.music }, dt);
    }

    fn cambio(&mut self, de: State, a: State, ctx: &Contexto) {
        self.animo.cambio(de, a, ctx);
        self.gestos.cortar();
    }

    fn aviso(&mut self, aviso: Aviso, ctx: &Contexto) {
        match aviso {
            Aviso::Evento(ev) => {
                self.animo.evento(ev, ctx);
                self.ondas.evento(ev, ctx, &mut self.azar);
            }
            Aviso::Nace(id) => {
                self.hijos.nace(ctx, id);
                self.animo.kick(1.6, ctx);
            }
            Aviso::Pulso(id, error) => self.hijos.pulso(ctx, id, error),
            Aviso::Fin(id, ok) => self.hijos.fin(ctx, id, ok),
            Aviso::Fuera => self.hijos.fuera(),
        }
    }

    fn paint(&self, g: &mut Grid, ctx: &Contexto) {
        let e = Escena::new(self, ctx, g);
        let (p, a, ancla) = (&self.animo.p, &self.animo, &e.ancla);
        if !self.mini {
            self.escala.paint(
                g,
                ctx,
                escala::Pinta {
                    ancla,
                    boot: e.boot,
                    nivel: a.level,
                    giro: a.arc_phase,
                    music: a.music,
                    groove: a.groove,
                    listos: &self.ondas.listos,
                    pass: a.pass,
                    copy: a.copy,
                    red: a.red,
                    low: e.low,
                    vu: p.vu,
                    fill: p.fill,
                    sweep: p.sweep,
                    cardinal: p.cardinal,
                    tests: p.tests,
                },
            );
        }
        self.tareas.paint(g, ctx, p.plan);
        if !self.mini {
            for arcos in &self.arcos {
                arcos.paint(g, ctx, arcos::Pinta { ancla, giro: a.arc_phase, arcs: p.arcs, arc_len: p.arc_len, boot: e.boot });
            }
        }
        self.cola.paint(g, ctx, ancla);
        self.prensas.paint(g, ctx, prensas::Pinta { ancla, press: p.press });
        self.voz.paint(g, ctx, voz::Pinta { ancla, ripple: p.ripple });
        self.ondas.paint(g, ctx, ancla);
        self.radar.paint(g, ctx, radar::Pinta { ancla, sweep: p.sweep, giro: a.arc_phase });
        self.ideas.paint(g, ctx, ideas::Pinta { ancla, orbit: p.orbit, amp: p.amp, fase: a.phase, low: e.low });
        self.paquetes.paint(g, ctx, paquetes::Pinta { ancla, packets: p.packets, low: e.low });
        self.puntadas.paint(g, ctx, puntadas::Pinta { ancla, stitch: p.stitch });
        self.git.paint(g, ctx, git::Pinta { ancla, git: p.git });
        self.pregunta.paint(g, ctx, pregunta::Pinta { ancla, droop: p.droop });
        self.zetas.paint(g, ctx, zetas::Pinta { ancla, zzz: p.zzz, low: e.low });
        self.brote.paint(g, ctx, brote::Pinta { ancla, bud: p.bud, fase: a.phase });
        self.lupa.paint(g, ctx, lupa::Pinta { ancla, lens: p.lens, low: e.low });
        self.centro.pintar(g, ctx, &e);
        self.hijos.paint(g, ctx, ancla);
        if !self.mini {
            let m = particulas::Pinta { motes: p.motes, boot: e.boot, press: p.press, mirada: self.mirada.gaze, low: e.low };
            self.motas.paint(g, ctx, m);
        }
    }

    /// Estado, verde, rojo, ámbar y acento; la tinta del centro; una por hijo.
    fn paleta(&self) -> Vec<[[f64; 3]; 4]> {
        let col = self.animo.col;
        let mut pal: Vec<_> = [col, GREEN, RED, WARM, crate::theme::accent_rgb()].into_iter().map(tonos).collect();
        pal.push(self.centro.tonos(col));
        pal.extend(self.hijos.tintas());
        pal
    }

    fn color(&self) -> [f64; 3] {
        self.animo.col
    }

    fn alarma(&self) -> f64 {
        self.animo.red
    }

    #[cfg(test)]
    fn hijos(&self) -> usize {
        self.hijos.len()
    }

    #[cfg(test)]
    fn como_any(&self) -> &dyn std::any::Any {
        self
    }
}
