//! Shape pieces: a stack of layers, each a shape whose every property is a live formula over
//! the buddy's channels (see [`expr`]). What Iris Studio builds, and what Iris, the browser and
//! the store's server all read the same way.
//!
//! ```toml
//! [[settings]]
//! name = "size"
//! default = 1.0
//! min = 0.5
//! max = 2.0
//!
//! colors = ["#ff8cc8"]           # its own inks (optional): ink 5 is the first, 6 the second…
//!
//! [outputs]                     # channels this piece publishes (optional)
//! "ears.tip" = "body.y + body.radius * 1.4"
//!
//! [[layer]]
//! shape = "blob"
//! x = "body.x"
//! y = "body.y"
//! radius = "body.radius * size + breath"
//! amp = "0.03 + s.thinking * 0.05"
//! fill = "hatch"
//! ```
//!
//! Every shape has `x`, `y` (where its center goes), `sx`, `sy` (scale), `rotation` (radians),
//! `tone` (1 dim … 3 full), `ink` (0 the state's color, 1 green, 2 red, 3 amber, 4 accent; the
//! channel `ink.body` is the body's, red on errors; 5 onwards, the piece's own `colors`) and `visible` (> 0.5 draws). Outlined shapes
//! take `fill`: `none`, `solid` or `hatch`. The rest depends on the shape (see [`Kind`]).

pub mod expr;

use expr::{Expr, Frame, Names, EVENTS, NEVER};

/// A setting the piece offers: a number between `min` and `max`.
#[derive(Clone, Debug)]
pub struct Setting {
    pub name: String,
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fill {
    None,
    Solid,
    Hatch,
}

/// What a layer draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `radius`.
    Circle,
    /// `radius`, `start` and `length` (fractions of a turn, 0..1), `dashes` (0 solid).
    Ring,
    /// A closed wobbly curve: `radius`, `amp`, `lobes`, `phase`, `chaos`.
    Blob,
    /// Through `points` (pairs of formulas, around its center); `closed` and `smooth` (on/off).
    Curve,
    /// An eye: `radius`, `pupil` (fraction of the radius), `look_x`, `look_y` (−1..1), `blink`
    /// (> 0.5 closed).
    Eye,
    /// Dots that are born, move and die: `rate` (per second), `life` (seconds), `speed`,
    /// `angle` and `spread` (radians), `gravity`, `fade` (on/off).
    Emitter,
}

impl Kind {
    fn by_name(n: &str) -> Option<Kind> {
        Some(match n {
            "circle" => Kind::Circle,
            "ring" => Kind::Ring,
            "blob" => Kind::Blob,
            "curve" => Kind::Curve,
            "eye" => Kind::Eye,
            "emitter" => Kind::Emitter,
            _ => return None,
        })
    }

    /// Its own properties and their defaults.
    fn props(self) -> &'static [(&'static str, f64)] {
        match self {
            Kind::Circle => &[("radius", 0.2)],
            Kind::Ring => &[("radius", 0.5), ("start", 0.0), ("length", 1.0), ("dashes", 0.0)],
            Kind::Blob => &[("radius", 0.35), ("amp", 0.03), ("lobes", 3.0), ("phase", 0.0), ("chaos", 0.0)],
            Kind::Curve => &[],
            Kind::Eye => &[("radius", 0.1), ("pupil", 0.45), ("look_x", 0.0), ("look_y", 0.0), ("blink", 0.0)],
            Kind::Emitter => &[("rate", 8.0), ("life", 1.2), ("speed", 0.3), ("angle", 1.5708), ("spread", 0.6), ("gravity", 0.0), ("fade", 1.0)],
        }
    }
}

/// The properties every layer has, and their defaults.
const COMMON: [(&str, f64); 8] = [("x", 0.0), ("y", 0.0), ("sx", 1.0), ("sy", 1.0), ("rotation", 0.0), ("tone", 3.0), ("ink", 0.0), ("visible", 1.0)];

#[derive(Clone, Debug)]
pub struct Layer {
    pub kind: Kind,
    pub fill: Fill,
    /// `COMMON` first, then the kind's own, in the order of `Kind::props`.
    props: Vec<Expr>,
    /// For curves: the points, around the layer's center.
    points: Vec<(Expr, Expr)>,
    closed: bool,
    smooth: bool,
}

/// A shape piece, parsed and with every formula compiled.
#[derive(Clone, Debug)]
pub struct Shape {
    pub settings: Vec<Setting>,
    /// The channels its formulas read, in the order [`Instance`] expects their values.
    pub inputs: Vec<String>,
    /// The channels it publishes, with their formulas.
    pub outputs: Vec<(String, Expr)>,
    /// Its own inks, as 0–255 RGB: ink 5 is the first.
    pub colors: Vec<[f64; 3]>,
    pub layers: Vec<Layer>,
}

/// Most own colors in a piece.
pub const MAX_COLORS: usize = 16;

/// "#rrggbb" as 0–255 RGB.
pub fn hex(s: &str) -> Option<[f64; 3]> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(f64::from);
    Some([c(0)?, c(2)?, c(4)?])
}

/// Most layers in a piece.
pub const MAX_LAYERS: usize = 64;

fn formula(v: &toml::Value, what: &str, names: &mut Names) -> Result<Expr, String> {
    match v {
        toml::Value::Float(f) => Ok(Expr::constant(*f)),
        toml::Value::Integer(i) => Ok(Expr::constant(*i as f64)),
        toml::Value::Boolean(b) => Ok(Expr::constant(if *b { 1.0 } else { 0.0 })),
        toml::Value::String(s) => Expr::compile(s, names).map_err(|e| format!("{what}: {e}")),
        _ => Err(format!("{what}: tiene que ser un número o una fórmula")),
    }
}

impl Shape {
    /// Parses a `shape.toml`; the error says what's wrong, in Spanish.
    pub fn parse(src: &str) -> Result<Shape, String> {
        let doc: toml::Table = src.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let mut settings = vec![];
        for s in doc.get("settings").and_then(|s| s.as_array()).map(|a| a.as_slice()).unwrap_or_default() {
            let name = s.get("name").and_then(|n| n.as_str()).ok_or("un ajuste sin «name»")?.to_string();
            let n = |k: &str| s.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)));
            let default = n("default").ok_or(format!("el ajuste «{name}» no tiene «default»"))?;
            let (min, max) = (n("min").unwrap_or(f64::MIN), n("max").unwrap_or(f64::MAX));
            if min > max {
                return Err(format!("el ajuste «{name}»: «min» es mayor que «max»"));
            }
            settings.push(Setting { name, default: default.clamp(min, max), min, max });
        }
        let setting_names: Vec<String> = settings.iter().map(|s| s.name.clone()).collect();
        let mut channels = vec![];
        let mut names = Names { settings: &setting_names, channels: &mut channels };

        let mut outputs = vec![];
        if let Some(t) = doc.get("outputs") {
            let t = t.as_table().ok_or("[outputs] va como tabla: nombre = fórmula")?;
            for (k, v) in t {
                outputs.push((k.clone(), formula(v, &format!("salida «{k}»"), &mut names)?));
            }
        }

        let mut colors = vec![];
        if let Some(c) = doc.get("colors") {
            let c = c.as_array().ok_or("«colors» va como lista: [\"#ff8cc8\", …]")?;
            if c.len() > MAX_COLORS {
                return Err(format!("tiene {} colores propios; el máximo es {MAX_COLORS}", c.len()));
            }
            for v in c {
                let t = v.as_str().unwrap_or_default();
                colors.push(hex(t).ok_or(format!("«{t}» no es un color: va como \"#rrggbb\""))?);
            }
        }

        let raw = doc.get("layer").and_then(|l| l.as_array()).map(|a| a.as_slice()).unwrap_or_default();
        if raw.len() > MAX_LAYERS {
            return Err(format!("tiene {} capas y el máximo es {MAX_LAYERS}", raw.len()));
        }
        let mut layers = vec![];
        for (n, l) in raw.iter().enumerate() {
            let at = format!("capa {}", n + 1);
            let l = l.as_table().ok_or(format!("{at}: va como [[layer]]"))?;
            let kind_s = l.get("shape").and_then(|s| s.as_str()).ok_or(format!("{at}: le falta «shape»"))?;
            let kind = Kind::by_name(kind_s).ok_or(format!("{at}: no hay ninguna forma «{kind_s}» (circle, ring, blob, curve, eye, emitter)"))?;
            let fill = match l.get("fill").and_then(|f| f.as_str()).unwrap_or("none") {
                "none" => Fill::None,
                "solid" => Fill::Solid,
                "hatch" => Fill::Hatch,
                f => return Err(format!("{at}: «fill» es none, solid o hatch, no «{f}»")),
            };
            let known: Vec<&str> = COMMON.iter().chain(kind.props()).map(|p| p.0).chain(["shape", "fill", "points", "closed", "smooth"]).collect();
            if let Some(k) = l.keys().find(|k| !known.contains(&k.as_str())) {
                return Err(format!("{at}: «{kind_s}» no tiene ninguna propiedad «{k}»"));
            }
            let mut props = vec![];
            for (p, def) in COMMON.iter().chain(kind.props()) {
                props.push(match l.get(*p) {
                    Some(v) => formula(v, &format!("{at}, {p}"), &mut names)?,
                    None => Expr::constant(*def),
                });
            }
            let mut points = vec![];
            if kind == Kind::Curve {
                let pts = l.get("points").and_then(|p| p.as_array()).ok_or(format!("{at}: una curva necesita «points» = [[x, y], …]"))?;
                for (k, p) in pts.iter().enumerate() {
                    let xy = p.as_array().filter(|a| a.len() == 2).ok_or(format!("{at}: el punto {} va como [x, y]", k + 1))?;
                    points.push((formula(&xy[0], &format!("{at}, punto {}", k + 1), &mut names)?, formula(&xy[1], &format!("{at}, punto {}", k + 1), &mut names)?));
                }
                if points.len() < 2 {
                    return Err(format!("{at}: una curva necesita al menos dos puntos"));
                }
            }
            let flag = |k: &str, d: bool| l.get(k).and_then(|v| v.as_bool()).unwrap_or(d);
            layers.push(Layer { kind, fill, props, points, closed: flag("closed", false), smooth: flag("smooth", true) });
        }
        if layers.is_empty() && outputs.is_empty() {
            return Err("no dibuja nada ni publica nada: le faltan capas ([[layer]])".into());
        }
        Ok(Shape { settings, inputs: channels, outputs, colors, layers })
    }
}

/// Where a shape piece draws: Iris's grid, or the browser's.
pub trait Canvas {
    /// The size of one dot, in world units.
    fn du(&self) -> f64;
    /// A dot at a point of the world (the brightest one wins).
    fn plot(&mut self, x: f64, y: f64, tone: u8, ink: u8);
    /// Every dot whose center falls in the box: `f` gets its world center and its column and
    /// row, and says what to paint there (or nothing).
    fn each(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, f: &mut dyn FnMut(f64, f64, usize, usize) -> Option<(u8, u8)>);
}

/// What changes every frame, for an instance to read.
pub struct Env<'a> {
    pub t: f64,
    pub dt: f64,
    /// Seconds in the visible state, and the state (Iris's index).
    pub st: f64,
    pub state: usize,
    pub calm: f64,
    pub voice: f64,
    pub music: f64,
    pub context: f64,
    pub du: f64,
    /// The values of `Shape::inputs`, in order.
    pub channels: &'a [f64],
}

#[derive(Clone, Copy)]
struct Mote {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
}

/// A shape piece running: its settings, when things happened, and its emitters' dots.
pub struct Instance {
    settings: Vec<f64>,
    /// When each of `expr::EVENTS` last happened (−∞ if never).
    when: [f64; 12],
    motes: Vec<Vec<Mote>>,
    /// Dots owed to each emitter, carried between frames.
    owed: Vec<f64>,
    seed: f64,
    born: u64,
}

/// Most dots alive in one emitter.
const MAX_MOTES: usize = 400;

impl Instance {
    /// `settings`: the values the recipe gives, by name; the rest take their default, and all
    /// are kept within their limits.
    pub fn new(shape: &Shape, settings: &[(String, f64)], seed: u64) -> Self {
        let settings = shape
            .settings
            .iter()
            .map(|s| settings.iter().find(|(n, _)| *n == s.name).map_or(s.default, |v| v.1).clamp(s.min, s.max))
            .collect();
        let n = shape.layers.len();
        Instance { settings, when: [f64::NEG_INFINITY; 12], motes: vec![vec![]; n], owed: vec![0.0; n], seed: seed as f64, born: 0 }
    }

    /// Something happened at `t`: one of `expr::EVENTS` (`"error"`, `"change"`, `"born"`…).
    pub fn happen(&mut self, what: &str, t: f64) {
        if let Some(i) = EVENTS.iter().position(|e| *e == what) {
            self.when[i] = t;
        }
    }

    fn frame<'a>(&'a self, env: &'a Env) -> Frame<'a> {
        let mut since = [NEVER; 12];
        for (s, w) in since.iter_mut().zip(self.when) {
            if w.is_finite() {
                *s = env.t - w;
            }
        }
        Frame {
            vars: [env.t, env.dt, env.st, env.state as f64, env.calm, env.voice, env.music, env.context, env.du],
            since,
            settings: &self.settings,
            channels: env.channels,
        }
    }

    /// Its outputs now, in the order of `Shape::outputs`.
    pub fn outputs(&self, shape: &Shape, env: &Env) -> Vec<f64> {
        let f = self.frame(env);
        shape.outputs.iter().map(|(_, e)| e.eval(&f)).collect()
    }

    /// Moves the emitters `env.dt` seconds.
    pub fn step(&mut self, shape: &Shape, env: &Env) {
        for (n, layer) in shape.layers.iter().enumerate() {
            if layer.kind != Kind::Emitter {
                continue;
            }
            let f = self.frame(env);
            let v = |k: usize| layer.props[k].eval(&f);
            let (visible, rate, life, speed, angle, spread, gravity) = (v(7), v(8), v(9).max(0.01), v(10), v(11), v(12), v(13));
            let dt = env.dt;
            let motes = &mut self.motes[n];
            for m in motes.iter_mut() {
                m.age += dt;
                m.vy -= gravity * dt;
                m.x += m.vx * dt;
                m.y += m.vy * dt;
            }
            motes.retain(|m| m.age < m.life);
            if visible <= 0.5 {
                self.owed[n] = 0.0;
                continue;
            }
            self.owed[n] += rate.max(0.0) * dt;
            while self.owed[n] >= 1.0 && self.motes[n].len() < MAX_MOTES {
                self.owed[n] -= 1.0;
                self.born += 1;
                let r = |k: f64| expr::hash(self.seed * 7.31 + self.born as f64 * 1.618 + k);
                let a = angle + (r(0.1) - 0.5) * 2.0 * spread;
                let s = speed * (0.6 + 0.8 * r(0.2));
                self.motes[n].push(Mote { x: 0.0, y: 0.0, vx: s * a.cos(), vy: s * a.sin(), age: 0.0, life: life * (0.7 + 0.6 * r(0.3)) });
            }
            self.owed[n] = self.owed[n].min(1.0);
        }
    }

    /// Draws every visible layer, in order.
    pub fn paint(&self, shape: &Shape, env: &Env, c: &mut dyn Canvas) {
        let f = self.frame(env);
        for (n, layer) in shape.layers.iter().enumerate() {
            let v = |k: usize| layer.props[k].eval(&f);
            if v(7) <= 0.5 {
                continue;
            }
            let tf = Transform { x: v(0), y: v(1), sx: v(2), sy: v(3), rot: v(4) };
            if tf.sx.abs() < 1e-9 || tf.sy.abs() < 1e-9 {
                continue;
            }
            let tone = v(5).round().clamp(1.0, 3.0) as u8;
            let ink = v(6).round().clamp(0.0, (4 + shape.colors.len()) as f64) as u8;
            let own = |k: usize| v(COMMON.len() + k);
            let du = c.du().max(1e-4);
            match layer.kind {
                Kind::Circle => {
                    let r = own(0).abs();
                    outline(c, &tf, layer.fill, tone, ink, r, |_| r, du);
                }
                Kind::Blob => {
                    let (r, amp, lobes, phase, chaos) = (own(0).abs(), own(1), own(2).round().max(1.0), own(3), own(4));
                    let at = move |th: f64| {
                        r + amp * (0.60 * (lobes * th + phase).sin() + 0.40 * ((lobes + 2.0) * th - phase * 1.6 + 1.0).sin() + 0.30 * chaos * ((lobes + 4.0) * th + phase * 2.7 + 2.0).sin())
                    };
                    outline(c, &tf, layer.fill, tone, ink, r + amp.abs() * 1.3, at, du);
                }
                Kind::Ring => {
                    let (r, start, len, dashes) = (own(0).abs(), own(1), own(2).clamp(0.0, 1.0), own(3).round().max(0.0));
                    let turn = std::f64::consts::TAU;
                    let steps = ((len * turn * r * tf.sx.abs().max(tf.sy.abs())) / (du * 0.6)).ceil().clamp(4.0, 4000.0) as usize;
                    for k in 0..=steps {
                        let u = k as f64 / steps as f64;
                        if dashes > 0.0 && ((u * dashes * 2.0).floor() as i64) % 2 == 1 {
                            continue;
                        }
                        let th = (start + u * len) * turn;
                        let (x, y) = tf.apply(r * th.cos(), r * th.sin());
                        c.plot(x, y, tone, ink);
                    }
                }
                Kind::Curve => {
                    let pts: Vec<(f64, f64)> = layer.points.iter().map(|(x, y)| (x.eval(&f), y.eval(&f))).collect();
                    let path = if layer.smooth { smooth(&pts, layer.closed) } else { pts.clone() };
                    let mut segs: Vec<((f64, f64), (f64, f64))> = path.windows(2).map(|w| (w[0], w[1])).collect();
                    if layer.closed && path.len() > 2 {
                        segs.push((*path.last().unwrap(), path[0]));
                    }
                    for (a, b) in &segs {
                        let len = ((b.0 - a.0) * tf.sx).hypot((b.1 - a.1) * tf.sy);
                        let steps = (len / (du * 0.6)).ceil().clamp(1.0, 4000.0) as usize;
                        for k in 0..=steps {
                            let u = k as f64 / steps as f64;
                            let (x, y) = tf.apply(a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
                            c.plot(x, y, tone, ink);
                        }
                    }
                    if layer.closed && layer.fill != Fill::None && path.len() > 2 {
                        let reach = path.iter().map(|p| p.0.hypot(p.1)).fold(0.0, f64::max);
                        fill_with(c, &tf, layer.fill, tone, ink, reach, |lx, ly| inside(&path, lx, ly));
                    }
                }
                Kind::Eye => {
                    let (r, pupil, lx, ly, blink) = (own(0).abs(), own(1).clamp(0.0, 1.0), own(2).clamp(-1.0, 1.0), own(3).clamp(-1.0, 1.0), own(4));
                    if blink > 0.5 {
                        let steps = ((2.0 * r * tf.sx.abs()) / (du * 0.6)).ceil().clamp(2.0, 2000.0) as usize;
                        for k in 0..=steps {
                            let (x, y) = tf.apply(-r + 2.0 * r * k as f64 / steps as f64, 0.0);
                            c.plot(x, y, tone, ink);
                        }
                    } else {
                        outline(c, &tf, layer.fill, softer(tone), ink, r, |_| r, du);
                        let pr = r * pupil;
                        let (px, py) = (lx * (r - pr), ly * (r - pr));
                        fill_with(c, &tf, Fill::Solid, tone, ink, r, |x, y| (x - px).hypot(y - py) < pr);
                    }
                }
                Kind::Emitter => {
                    let fade = own(6) > 0.5;
                    for m in &self.motes[n] {
                        let tone = if fade { (3.0 * (1.0 - m.age / m.life)).ceil().clamp(1.0, tone as f64) as u8 } else { tone };
                        let (x, y) = tf.apply(m.x, m.y);
                        c.plot(x, y, tone, ink);
                    }
                }
            }
        }
    }
}

/// Where a layer goes: scaled, rotated, then moved.
struct Transform {
    x: f64,
    y: f64,
    sx: f64,
    sy: f64,
    rot: f64,
}

impl Transform {
    fn apply(&self, lx: f64, ly: f64) -> (f64, f64) {
        let (x, y) = (lx * self.sx, ly * self.sy);
        let (s, c) = self.rot.sin_cos();
        (self.x + x * c - y * s, self.y + x * s + y * c)
    }

    fn invert(&self, wx: f64, wy: f64) -> (f64, f64) {
        let (dx, dy) = (wx - self.x, wy - self.y);
        let (s, c) = self.rot.sin_cos();
        ((dx * c + dy * s) / self.sx, (-dx * s + dy * c) / self.sy)
    }
}

/// One tone below, for what goes behind an outline (a hatch, an eye's ring), but never down to
/// tone 1 unless it already was: tone 1 is the theme's dim gray, with no ink, and a filled
/// shape would lose its color.
fn softer(tone: u8) -> u8 {
    tone.saturating_sub(1).max(2).min(tone)
}

/// A closed outline given by its radius at each angle, and its inside if it has a fill.
#[allow(clippy::too_many_arguments)]
fn outline(c: &mut dyn Canvas, tf: &Transform, fill: Fill, tone: u8, ink: u8, reach: f64, r_at: impl Fn(f64) -> f64, du: f64) {
    let turn = std::f64::consts::TAU;
    let steps = ((turn * reach * tf.sx.abs().max(tf.sy.abs())) / (du * 0.6)).ceil().clamp(8.0, 6000.0) as usize;
    for k in 0..steps {
        let th = k as f64 / steps as f64 * turn;
        let r = r_at(th);
        let (x, y) = tf.apply(r * th.cos(), r * th.sin());
        c.plot(x, y, tone, ink);
    }
    if fill != Fill::None {
        let tone_in = if fill == Fill::Solid { tone } else { softer(tone) };
        fill_with(c, tf, fill, tone_in, ink, reach, |lx, ly| lx.hypot(ly) < r_at(ly.atan2(lx).rem_euclid(turn)));
    }
}

/// Paints the dots of the box around the layer that `inside` says are in (in local coordinates).
fn fill_with(c: &mut dyn Canvas, tf: &Transform, fill: Fill, tone: u8, ink: u8, reach: f64, inside: impl Fn(f64, f64) -> bool) {
    let r = reach * tf.sx.abs().max(tf.sy.abs());
    c.each(tf.x - r, tf.y - r, tf.x + r, tf.y + r, &mut |x, y, i, j| {
        if fill == Fill::Hatch && (i + j) % 2 == 1 {
            return None;
        }
        let (lx, ly) = tf.invert(x, y);
        inside(lx, ly).then_some((tone, ink))
    });
}

/// Is the point inside the polygon?
fn inside(poly: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut yes = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let ((xi, yi), (xj, yj)) = (poly[i], poly[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            yes = !yes;
        }
        j = i;
    }
    yes
}

/// A Catmull-Rom curve through the points, as a polyline.
fn smooth(p: &[(f64, f64)], closed: bool) -> Vec<(f64, f64)> {
    let n = p.len();
    let get = |i: isize| -> (f64, f64) {
        if closed {
            p[i.rem_euclid(n as isize) as usize]
        } else {
            p[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let segs = if closed { n } else { n - 1 };
    let mut out = vec![];
    for s in 0..segs as isize {
        let (p0, p1, p2, p3) = (get(s - 1), get(s), get(s + 1), get(s + 2));
        for k in 0..12 {
            let t = k as f64 / 12.0;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f64, b: f64, c: f64, d: f64| 0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
            out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    out.push(if closed { p[0] } else { p[n - 1] });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 60×60 grid over −1..1, like Iris's.
    struct Test {
        dots: Vec<u8>,
    }

    impl Canvas for Test {
        fn du(&self) -> f64 {
            2.0 / 59.0
        }
        fn plot(&mut self, x: f64, y: f64, tone: u8, _ink: u8) {
            let i = ((x + 1.0) / 2.0 * 59.0).round();
            let j = ((1.0 - y) / 2.0 * 59.0).round();
            if (0.0..60.0).contains(&i) && (0.0..60.0).contains(&j) {
                let k = j as usize * 60 + i as usize;
                self.dots[k] = self.dots[k].max(tone);
            }
        }
        fn each(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, f: &mut dyn FnMut(f64, f64, usize, usize) -> Option<(u8, u8)>) {
            for j in 0..60 {
                for i in 0..60 {
                    let (x, y) = (i as f64 / 59.0 * 2.0 - 1.0, 1.0 - j as f64 / 59.0 * 2.0);
                    if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                        if let Some((t, _)) = f(x, y, i, j) {
                            self.dots[j * 60 + i] = self.dots[j * 60 + i].max(t);
                        }
                    }
                }
            }
        }
    }

    fn env(channels: &[f64]) -> Env<'_> {
        Env { t: 1.0, dt: 1.0 / 30.0, st: 1.0, state: 2, calm: 1.0, voice: 0.0, music: 0.0, context: 0.0, du: 2.0 / 59.0, channels }
    }

    const KIRBY: &str = r#"
[[settings]]
name = "size"
default = 1.0
min = 0.5
max = 2.0

[outputs]
"kirby.top" = "body.y + body.radius"

[[layer]]
shape = "blob"
x = "body.x"
y = "body.y"
radius = "body.radius * size"
amp = 0.02
fill = "hatch"

[[layer]]
shape = "eye"
x = "body.x - 0.1"
y = "body.y + 0.05"
look_x = "gaze.x"

[[layer]]
shape = "curve"
points = [[-0.1, -0.1], [0, -0.15], [0.1, -0.1]]

[[layer]]
shape = "emitter"
rate = 30
"#;

    #[test]
    fn parses_and_finds_its_channels() {
        let s = Shape::parse(KIRBY).unwrap();
        assert_eq!(s.inputs, ["body.y", "body.radius", "body.x", "gaze.x"]);
        assert_eq!(s.layers.len(), 4);
        assert_eq!(s.settings[0].name, "size");
        let i = Instance::new(&s, &[("size".into(), 9.0)], 1);
        assert_eq!(i.settings, [2.0], "settings stay within their limits");
        assert_eq!(i.outputs(&s, &env(&[0.1, 0.4, 0.0, 0.0])), [0.5]);
    }

    #[test]
    fn draws_and_emits() {
        let s = Shape::parse(KIRBY).unwrap();
        let mut i = Instance::new(&s, &[], 1);
        let ch = [0.0, 0.4, 0.0, 0.3];
        for _ in 0..30 {
            i.step(&s, &env(&ch));
        }
        assert!(i.motes[3].len() > 10);
        let mut c = Test { dots: vec![0; 3600] };
        i.paint(&s, &env(&ch), &mut c);
        let lit = c.dots.iter().filter(|d| **d > 0).count();
        assert!(lit > 100, "{lit}");
    }

    #[test]
    fn errors_point_at_the_layer() {
        assert!(Shape::parse("[[layer]]\nshape = \"star\"").unwrap_err().contains("capa 1"));
        assert!(Shape::parse("[[layer]]\nshape = \"circle\"\nradius = \"1 +\"").unwrap_err().contains("radius"));
        assert!(Shape::parse("[[layer]]\nshape = \"circle\"\ncolour = 1").unwrap_err().contains("colour"));
        assert!(Shape::parse("").is_err());
    }
}
