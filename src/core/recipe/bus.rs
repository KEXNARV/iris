//! BUS: the channels. Every number the pieces of a buddy share lives here, by name: the
//! script's values, what the mood and the spring publish, where the body is this frame. Names
//! are resolved to indices once, when the recipe is built; after that, reading and writing is
//! indexing a vector.
//!
//! "No value" is NaN (a kid that hasn't sent anything, a sprout with no tip).

use std::collections::HashMap;

#[derive(Clone, Default)]
pub(crate) struct Bus {
    names: HashMap<String, usize>,
    values: Vec<f64>,
}

/// A channel, already resolved.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct Ch(usize);

impl Bus {
    /// The channel with that name, created at 0 if it didn't exist.
    pub fn ch(&mut self, name: &str) -> Ch {
        if let Some(&i) = self.names.get(name) {
            return Ch(i);
        }
        self.values.push(0.0);
        self.names.insert(name.to_string(), self.values.len() - 1);
        Ch(self.values.len() - 1)
    }

    /// The channel with that name, if some piece declared it.
    #[cfg(test)]
    pub fn find(&self, name: &str) -> Option<Ch> {
        self.names.get(name).map(|&i| Ch(i))
    }

    pub fn get(&self, c: Ch) -> f64 {
        self.values[c.0]
    }

    pub fn set(&mut self, c: Ch, v: f64) {
        self.values[c.0] = v;
    }

    /// Adds to what's there. In call order, so the sum comes out the same every time.
    pub fn add(&mut self, c: Ch, v: f64) {
        self.values[c.0] += v;
    }

    pub fn on(&self, c: Ch) -> bool {
        self.values[c.0] > 0.5
    }

    pub fn flag(&mut self, c: Ch, v: bool) {
        self.values[c.0] = if v { 1.0 } else { 0.0 };
    }

    /// An optional value: `None` if it's NaN.
    pub fn opt(&self, c: Ch) -> Option<f64> {
        let v = self.values[c.0];
        (!v.is_nan()).then_some(v)
    }
}
