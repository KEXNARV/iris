//! RECIPES: a buddy as data. A recipe says which parts it has (built-in ones today; from the
//! store later), in what order they move, in what order they're drawn, how each one is wired to
//! the channels and how much of each thing shows in each state (the script). The original buddy
//! and Baymax are two recipes (`original.toml`, `baymax.toml`) and nothing else.
//!
//! The order of `[[piece]]` is the order in which the parts move, react and lay out; `draw`
//! is the order in which they're drawn (the first one ends up underneath).

mod builtin;
pub(crate) mod bus;
pub(crate) mod files;
mod shape;
pub(crate) mod part;
#[cfg(feature = "pieces")]
pub(crate) mod wasm;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use super::buddies::Buddy;
use super::color::{tones, GREEN, RED, WARM};
use super::context::{Context, Notice};
use super::grid::Grid;
use super::rand::Rand;
use super::state::{State, ALL};
use bus::{Bus, Ch};
use part::{Frame, Msg, Part, World};

/// Which parts get drawn, one bit per part (its `layer`); all by default. The terminal doesn't
/// touch it; the Iris web page uses it to show the core taken apart, piece by piece.
pub(crate) static LAYERS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

fn layer_on(layer: u32) -> bool {
    LAYERS.load(std::sync::atomic::Ordering::Relaxed) & (1 << layer) != 0
}

/// One `[[piece]]` of a recipe.
pub(crate) struct PieceSpec {
    /// How the recipe refers to it in `draw`; by default, what it uses.
    pub id: String,
    /// Which part: `builtin:arcs`.
    pub uses: String,
    pub settings: toml::Table,
    /// Input → channel, for the inputs that don't read the channel of their own name.
    pub inputs: HashMap<String, String>,
    /// Whether the kids (the subagents' small cores) carry it too.
    pub kid: bool,
    /// Its bit in `LAYERS`.
    pub layer: Option<u32>,
}

pub(crate) struct Recipe {
    /// Where its `use` lines find their files; none for the built-in recipes.
    pub files: Option<Arc<dyn files::Files>>,
    /// The script: the value of each channel in each state. `default` holds every channel;
    /// each state, only what changes.
    script: Vec<(String, f64)>,
    states: HashMap<State, Vec<(String, f64)>>,
    pub pieces: Vec<PieceSpec>,
    pub draw: Vec<String>,
}

/// The lowercase name of a state, as recipes write it.
pub(crate) fn state_name(s: State) -> String {
    format!("{s:?}").to_lowercase()
}

impl Recipe {
    pub fn parse(src: &str) -> Result<Recipe, String> {
        let doc: toml::Table = src.parse().map_err(|e: toml::de::Error| e.to_string())?;
        doc.get("recipe").and_then(|r| r.get("name")).and_then(|n| n.as_str()).ok_or("falta [recipe] name")?;
        let numbers = |t: &toml::Value| -> Result<Vec<(String, f64)>, String> {
            let t = t.as_table().ok_or("el guion va en tablas")?;
            t.iter()
                .map(|(k, v)| {
                    let n = v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).ok_or(format!("«{k}» no es un número"))?;
                    Ok((k.clone(), n))
                })
                .collect()
        };
        let script_t = doc.get("script").and_then(|s| s.as_table()).ok_or("falta [script]")?;
        let script = numbers(script_t.get("default").ok_or("falta [script.default]")?)?;
        let mut states = HashMap::new();
        for (k, v) in script_t {
            if k == "default" {
                continue;
            }
            let s = ALL.iter().copied().find(|s| state_name(*s) == *k).ok_or(format!("no hay ningún estado «{k}»"))?;
            states.insert(s, numbers(v)?);
        }
        let mut pieces = vec![];
        for p in doc.get("piece").and_then(|p| p.as_array()).ok_or("faltan las piezas ([[piece]])")? {
            let uses = p.get("use").and_then(|u| u.as_str()).ok_or("una pieza sin «use»")?.to_string();
            let id = p.get("id").and_then(|i| i.as_str()).map_or_else(|| uses.trim_start_matches("builtin:").to_string(), str::to_string);
            let settings = p.get("settings").and_then(|s| s.as_table()).cloned().unwrap_or_default();
            let inputs = p
                .get("inputs")
                .and_then(|s| s.as_table())
                .map(|t| t.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect())
                .unwrap_or_default();
            let kid = p.get("kid").and_then(|k| k.as_bool()).unwrap_or(true);
            let layer = p.get("layer").and_then(|l| l.as_integer()).map(|l| l as u32);
            pieces.push(PieceSpec { id, uses, settings, inputs, kid, layer });
        }
        let draw: Vec<String> = doc
            .get("draw")
            .and_then(|d| d.get("order"))
            .and_then(|o| o.as_array())
            .ok_or("falta [draw] order")?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        if let Some(missing) = draw.iter().find(|d| !pieces.iter().any(|p| &p.id == *d)) {
            return Err(format!("[draw] nombra «{missing}», que no está entre las piezas"));
        }
        Ok(Recipe { files: None, script, states, pieces, draw })
    }

    /// A recipe from a folder: its `recipe.toml`, and its files next to it.
    pub fn load_dir(dir: &std::path::Path) -> Result<Recipe, String> {
        let src = std::fs::read_to_string(dir.join("recipe.toml")).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut r = Recipe::parse(&src)?;
        r.files = Some(Arc::new(files::Disk { dir: dir.to_path_buf() }));
        Ok(r)
    }

    /// The script's values for a state: the default ones with that state's on top.
    pub fn script_for(&self, s: State) -> Vec<(String, f64)> {
        let mut out = self.script.clone();
        for (k, v) in self.states.get(&s).into_iter().flatten() {
            match out.iter_mut().find(|(n, _)| n == k) {
                Some(slot) => slot.1 = *v,
                None => out.push((k.clone(), *v)),
            }
        }
        out
    }

    /// Every channel the script names, in any state.
    pub fn script_channels(&self) -> Vec<String> {
        let mut names: Vec<String> = self.script.iter().map(|(n, _)| n.clone()).collect();
        for (k, _) in self.states.values().flatten() {
            if !names.contains(k) {
                names.push(k.clone());
            }
        }
        names
    }
}

/// The built-in recipes, parsed once.
pub(crate) fn original() -> Arc<Recipe> {
    static R: OnceLock<Arc<Recipe>> = OnceLock::new();
    R.get_or_init(|| Arc::new(Recipe::parse(include_str!("original.toml")).expect("original.toml"))).clone()
}

pub(crate) fn baymax() -> Arc<Recipe> {
    static R: OnceLock<Arc<Recipe>> = OnceLock::new();
    R.get_or_init(|| Arc::new(Recipe::parse(include_str!("baymax.toml")).expect("baymax.toml"))).clone()
}

struct Slot {
    id: String,
    part: Box<dyn Part>,
    layer: Option<u32>,
    /// It broke and was already reported: it's no longer called.
    off: bool,
}

/// A recipe running: the buddy the core sees.
pub(crate) struct Assembled {
    slots: Vec<Slot>,
    /// Indices into `slots`, in drawing order.
    draw: Vec<usize>,
    bus: Bus,
    rand: Rand,
    col: [Ch; 3],
    red: Ch,
    out: Vec<Msg>,
}

/// What a part gets when it's built.
pub(crate) struct Build<'a> {
    pub recipe: &'a Recipe,
    pub spec: &'a PieceSpec,
    pub bus: &'a mut Bus,
    pub rand: &'a mut Rand,
    /// The state it's born in.
    pub state: State,
}

impl Build<'_> {
    /// The channel an input reads: the one the recipe wires it to, or the one with its name.
    pub fn input(&mut self, name: &str) -> Ch {
        let target = self.spec.inputs.get(name).map_or(name, String::as_str).to_string();
        self.bus.ch(&target)
    }

    pub fn setting(&self, name: &str) -> Option<&toml::Value> {
        self.spec.settings.get(name)
    }

    pub fn number(&self, name: &str, default: f64) -> f64 {
        self.setting(name).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(default)
    }
}

impl Assembled {
    /// A buddy born booting.
    pub fn new(recipe: &Recipe) -> Result<Self, String> {
        Self::build(recipe, State::Booting, false)
    }

    /// A kid: already awake in `state`, without the parts kids don't carry, and its own RNG.
    pub fn kid(recipe: &Recipe, state: State, seed: u64) -> Result<Self, String> {
        let mut a = Self::build(recipe, state, true)?;
        a.rand.seed(seed);
        Ok(a)
    }

    fn build(recipe: &Recipe, state: State, kid: bool) -> Result<Self, String> {
        let mut bus = Bus::default();
        let mut rand = Rand::new();
        let mut slots = vec![];
        for spec in recipe.pieces.iter().filter(|p| !kid || p.kid) {
            let mut b = Build { recipe, spec, bus: &mut bus, rand: &mut rand, state };
            let part = builtin::make(&mut b)?;
            slots.push(Slot { id: spec.id.clone(), part, layer: spec.layer, off: false });
        }
        let draw = recipe.draw.iter().filter_map(|d| slots.iter().position(|s| &s.id == d)).collect();
        let col = [bus.ch("col.r"), bus.ch("col.g"), bus.ch("col.b")];
        let red = bus.ch("red");
        Ok(Assembled { slots, draw, bus, rand, col, red, out: vec![] })
    }

    /// Runs `f` on each part in order, delivering the messages each one sends before the next.
    fn each(&mut self, ctx: &Context, mut f: impl FnMut(&mut dyn Part, &mut World)) {
        for i in 0..self.slots.len() {
            if self.slots[i].off {
                continue;
            }
            let mut w = World { ctx, bus: &mut self.bus, rand: &mut self.rand, out: &mut self.out };
            f(&mut *self.slots[i].part, &mut w);
            while !self.out.is_empty() {
                let msgs = std::mem::take(&mut self.out);
                for m in msgs {
                    for (j, s) in self.slots.iter_mut().enumerate() {
                        if j != i && !s.off {
                            let mut w = World { ctx, bus: &mut self.bus, rand: &mut self.rand, out: &mut self.out };
                            s.part.message(&mut w, m);
                        }
                    }
                }
            }
        }
    }

    /// How many green rings the parts are showing, for tests.
    #[cfg(test)]
    pub fn rings(&self) -> usize {
        self.slots.iter().map(|s| s.part.rings()).sum()
    }

    /// A channel's value, for tests.
    #[cfg(test)]
    pub fn channel(&self, name: &str) -> f64 {
        self.bus.find(name).map_or(f64::NAN, |c| self.bus.get(c))
    }
}

impl Buddy for Assembled {
    fn step(&mut self, ctx: &Context, dt: f64) {
        self.each(ctx, |p, w| p.step(w, dt));
    }

    fn change(&mut self, from: State, to: State, ctx: &Context) {
        self.each(ctx, |p, w| p.change(w, from, to));
    }

    fn notice(&mut self, notice: Notice, ctx: &Context) {
        self.each(ctx, |p, w| p.notice(w, notice));
    }

    fn paint(&self, g: &mut Grid, ctx: &Context) {
        let mut f = Frame { ctx, bus: self.bus.clone() };
        for s in self.slots.iter().filter(|s| !s.off) {
            s.part.layout(&mut f, g);
        }
        for &i in &self.draw {
            let s = &self.slots[i];
            if !s.off && s.layer.is_none_or(layer_on) {
                s.part.paint(g, &f);
            }
        }
    }

    /// State, green, red, amber and accent; then what each part adds, in drawing order.
    fn palette(&self) -> Vec<[[f64; 3]; 4]> {
        let col = self.color();
        let mut pal: Vec<_> = [col, GREEN, RED, WARM, crate::theme::accent_rgb()].into_iter().map(tones).collect();
        for &i in &self.draw {
            pal.extend(self.slots[i].part.inks(col));
        }
        pal
    }

    fn color(&self) -> [f64; 3] {
        self.col.map(|c| self.bus.get(c))
    }

    fn channels(&self, ctx: &Context) -> Vec<(String, f64)> {
        let g = Grid::with_dots(120, 120, 2, 4);
        let mut f = Frame { ctx, bus: self.bus.clone() };
        for s in self.slots.iter().filter(|s| !s.off) {
            s.part.layout(&mut f, &g);
        }
        f.bus.all()
    }

    fn take_broken(&mut self) -> Vec<(String, String)> {
        let mut out = vec![];
        for s in &mut self.slots {
            if !s.off {
                if let Some(why) = s.part.broken() {
                    s.off = true;
                    out.push((s.id.clone(), why));
                }
            }
        }
        out
    }

    fn alarm(&self) -> f64 {
        self.bus.get(self.red)
    }

    #[cfg(test)]
    fn kids(&self) -> usize {
        self.slots.iter().map(|s| s.part.kids()).sum()
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
