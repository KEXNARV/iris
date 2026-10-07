//! ÁNIMO del buddy original: lo que se mueve por dentro y comparten sus piezas. Los parámetros
//! del guion acercándose al estado, el color, las fases de la ondulación y del giro, el
//! resorte, los destellos que dejan los eventos y lo que llega de la voz y la música.

use super::guion::Params;
use crate::nucleo::color::{mix, RED};
use crate::nucleo::contexto::Contexto;
use crate::nucleo::estado::{Event, State};
use crate::nucleo::piezas::resorte::Resorte;
use crate::nucleo::piezas::Pieza;

pub(crate) struct Animo {
    pub p: Params,
    pub col: [f64; 3],
    /// Fase de la ondulación del cuerpo y giro de lo de alrededor.
    pub phase: f64,
    pub arc_phase: f64,
    /// Aplastamiento elástico.
    pub resorte: Resorte,
    pub shake: f64,
    pub red: f64,
    pub deflate: f64,
    pub nope: f64,
    pub copy: f64,
    pub pass: f64,
    /// Al pasar de escuchar a transcribir, las motas se tragan hacia el centro.
    pub swallow: f64,
    pub level: f64,
    /// Nivel de la música y cuánto está bailando (0..1: entra en medio segundo, se va de golpe).
    pub music: f64,
    pub groove: f64,
}

impl Animo {
    pub fn new(s: State) -> Self {
        Animo {
            p: s.params(),
            col: s.rgb(),
            phase: 0.0,
            arc_phase: 0.0,
            resorte: Resorte::new(),
            shake: 0.0,
            red: 0.0,
            deflate: 0.0,
            nope: 0.0,
            copy: 0.0,
            pass: 0.0,
            swallow: 0.0,
            level: 0.0,
            music: 0.0,
            groove: 0.0,
        }
    }

    pub fn kick(&mut self, v: f64, ctx: &Contexto) {
        self.resorte.empujar(v, ctx.calma());
    }

    /// Transiciones con intención: cada cambio tiene su gesto.
    pub fn cambio(&mut self, from: State, to: State, ctx: &Contexto) {
        use State::*;
        match (from, to) {
            (_, Offline) => self.kick(-1.5, ctx),
            (Sleeping, _) => {
                self.kick(3.0, ctx);
                self.copy = 0.5; // parpadea al despertar
            }
            (Listening | NoVoice, Transcribing) => self.swallow = 1.0,
            (f, Thinking) if f.is_tool() => self.kick(-0.8, ctx),
            (_, t) if t.is_tool() => self.kick(1.2, ctx),
            _ => self.kick(1.0, ctx),
        }
    }

    /// Lo que cada evento le hace al cuerpo: destellos y empujones.
    pub fn evento(&mut self, ev: Event, ctx: &Contexto) {
        match ev {
            Event::Key => self.kick(1.2, ctx),
            Event::Error => {
                self.red = 1.0;
                self.shake = if ctx.sig.calm { 0.0 } else { 1.0 };
                self.kick(-2.5, ctx);
            }
            Event::Done => self.kick(3.2, ctx),
            Event::Cancel => {
                self.deflate = 1.0;
                self.kick(-3.0, ctx);
            }
            Event::Nope => self.nope = 1.0,
            Event::Merge => {}
            Event::Copy => {
                self.copy = 1.0;
                self.kick(0.8, ctx);
            }
            Event::Pass => {
                self.pass = 1.0;
                self.kick(1.5, ctx);
            }
        }
    }

    /// Los parámetros y el color van hacia los del estado; con un error, hacia el rojo.
    pub fn acercar(&mut self, ctx: &Contexto, dt: f64, con_hijos: bool) {
        let k = 1.0 - (-dt / 0.35).exp();
        let mut target = ctx.estado.params();
        // Con hijos, el principal se hace un poco a un lado para dejarles la órbita.
        // y los arcos se apagan un poco para que la red se lea.
        if con_hijos {
            target.blob_r *= 0.72;
            target.arcs *= 0.45;
            target.motes *= 0.5;
        }
        self.p.approach(&target, k);
        let base = ctx.estado.rgb();
        let want_col = if self.red > 0.05 { mix(base, RED, (self.red * 1.4).min(1.0)) } else { base };
        let kc = if self.red > 0.05 { 0.5 } else { k };
        for i in 0..3 {
            self.col[i] += (want_col[i] - self.col[i]) * kc;
        }
    }

    /// El resorte rebota y los destellos se apagan.
    pub fn apagar(&mut self, ctx: &Contexto, dt: f64) {
        self.resorte.step(ctx, (), dt);
        for v in [&mut self.red, &mut self.nope] {
            *v = (*v - dt * 1.1).max(0.0);
        }
        self.shake = (self.shake - dt * 1.6).max(0.0);
        self.deflate = (self.deflate - dt * 1.4).max(0.0);
        self.copy = (self.copy - dt * 3.0).max(0.0);
        self.pass = (self.pass - dt * 1.2).max(0.0);
        self.swallow = (self.swallow - dt * 1.5).max(0.0);
    }

    /// Las fases avanzan, y llega lo de afuera: la voz y la música.
    pub fn ritmo(&mut self, ctx: &Contexto, dt: f64) {
        let calm = ctx.calma();
        self.phase += dt * self.p.wob * calm;
        self.arc_phase += dt * self.p.arc_speed * calm;

        let lvl = if matches!(ctx.estado, State::Listening | State::Speaking) { (ctx.sig.level as f64 * 6.0).min(1.0) } else { 0.0 };
        self.level += (lvl - self.level) * if lvl > self.level { 0.5 } else { 0.12 };
        // Música: cava ya viene suavizado, así que se sigue casi tal cual. Como en el teclado,
        // aparece en medio segundo y una tecla (o cualquier otro estado) lo apaga al instante.
        let m = ctx.sig.music as f64;
        // Un golpe (el nivel sube de golpe) lo aplasta un poco: así se ve que baila.
        if m - self.music > 0.15 {
            self.kick(1.5 * self.groove, ctx);
        }
        self.music += (m - self.music) * (dt * 30.0).min(1.0);
        self.groove = if ctx.estado != State::Idle {
            (self.groove - dt * 8.0).max(0.0)
        } else if m > 0.02 {
            (self.groove + dt / 0.5).min(1.0)
        } else {
            (self.groove - dt).max(0.0) // entre canción y canción se apaga despacio
        };
    }
}
