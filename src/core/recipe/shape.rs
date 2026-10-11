//! A shape piece inside a recipe: layers of shapes whose properties are formulas over the
//! channels (`crates/iris-shape`). It reads the channels its formulas name, publishes its
//! `[outputs]`, remembers when things happened (for `ev.<name>`) and draws.

use iris_shape::{Canvas, Env, Instance, Shape};

use super::bus::{Bus, Ch};
use super::part::{Frame, Part, World};
use super::Build;
use crate::core::context::{Context, Notice};
use crate::core::grid::Grid;
use crate::core::state::{State, ALL};

pub(crate) struct ShapePart {
    shape: Shape,
    inst: Instance,
    inputs: Vec<Ch>,
    outputs: Vec<Ch>,
}

impl ShapePart {
    pub fn new(src: &str, b: &mut Build) -> Result<Self, String> {
        let shape = Shape::parse(src)?;
        let mut settings = vec![];
        for (k, v) in &b.spec.settings {
            if !shape.settings.iter().any(|s| &s.name == k) {
                return Err(format!("no tiene ningún ajuste «{k}»"));
            }
            let n = v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).or_else(|| v.as_bool().map(|x| if x { 1.0 } else { 0.0 }));
            settings.push((k.clone(), n.ok_or(format!("el ajuste «{k}» tiene que ser un número"))?));
        }
        let seed = (b.rand.rand() * 1e9) as u64;
        let inst = Instance::new(&shape, &settings, seed);
        let inputs = shape.inputs.iter().map(|n| b.input(n)).collect();
        let outputs = shape.outputs.iter().map(|(n, _)| b.bus.ch(n)).collect();
        Ok(ShapePart { shape, inst, inputs, outputs })
    }

    fn env<'a>(&self, ctx: &Context, bus: &Bus, dt: f64, du: f64, channels: &'a mut Vec<f64>) -> Env<'a> {
        channels.clear();
        channels.extend(self.inputs.iter().map(|c| bus.get(*c)));
        Env {
            t: ctx.t,
            dt,
            st: ctx.st(),
            state: ALL.iter().position(|s| *s == ctx.state).unwrap_or(2),
            calm: ctx.calm(),
            voice: ctx.sig.level as f64,
            music: ctx.sig.music as f64,
            context: ctx.sig.ctx,
            du,
            channels,
        }
    }

    fn publish(&self, env: &Env, bus: &mut Bus) {
        for (c, v) in self.outputs.iter().zip(self.inst.outputs(&self.shape, env)) {
            bus.set(*c, v);
        }
    }
}

impl Part for ShapePart {
    fn step(&mut self, w: &mut World, dt: f64) {
        let mut ch = vec![];
        let env = self.env(w.ctx, w.bus, dt, 0.0, &mut ch);
        self.inst.step(&self.shape, &env);
        self.publish(&env, w.bus);
    }

    fn change(&mut self, w: &mut World, _from: State, _to: State) {
        self.inst.happen("change", w.ctx.t);
    }

    fn notice(&mut self, w: &mut World, n: Notice) {
        let t = w.ctx.t;
        match n {
            Notice::Event(ev) => self.inst.happen(&format!("{ev:?}").to_lowercase(), t),
            Notice::Born(_) => self.inst.happen("born", t),
            Notice::Pulse(..) => self.inst.happen("pulse", t),
            Notice::Finish(..) => self.inst.happen("finish", t),
            Notice::Gone => {}
        }
    }

    fn layout(&self, f: &mut Frame, g: &Grid) {
        let mut ch = vec![];
        let env = self.env(f.ctx, &f.bus, 0.0, g.du, &mut ch);
        let out = self.inst.outputs(&self.shape, &env);
        for (c, v) in self.outputs.iter().zip(out) {
            f.bus.set(*c, v);
        }
    }

    fn paint(&self, g: &mut Grid, f: &Frame) {
        let mut ch = vec![];
        let env = self.env(f.ctx, &f.bus, 0.0, g.du, &mut ch);
        self.inst.paint(&self.shape, &env, &mut GridCanvas(g));
    }
}

/// Iris's grid, as a shape piece sees it.
struct GridCanvas<'a>(&'a mut Grid);

impl Canvas for GridCanvas<'_> {
    fn du(&self) -> f64 {
        self.0.du
    }

    fn plot(&mut self, x: f64, y: f64, tone: u8, ink: u8) {
        self.0.plot(x, y, tone, ink);
    }

    fn each(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, f: &mut dyn FnMut(f64, f64, usize, usize) -> Option<(u8, u8)>) {
        let g = &mut *self.0;
        let ((i0, i1), (j0, j1)) = g.span(x0, y0, x1, y1);
        for j in j0..j1 {
            for i in i0..i1 {
                let (x, y) = g.center(i, j);
                if let Some((tone, ink)) = f(x, y, i, j) {
                    g.dot(i, j, tone, ink);
                }
            }
        }
    }
}
