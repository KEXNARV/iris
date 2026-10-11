//! A buddy's randomness: xorshift with a fixed seed, so everything is repeatable. There is one
//! per buddy and its pieces share it: the order in which they ask for it is part of how it looks.

#[derive(Clone)]
pub(crate) struct Rand(u64);

impl Rand {
    pub fn new() -> Self {
        Rand(0x9E37_79B9_7F4A_7C15)
    }

    /// Mixes it with `seed`, so each kid gets its own.
    pub fn seed(&mut self, seed: u64) {
        self.0 ^= seed.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1;
    }

    /// A number in 0..1.
    pub fn rand(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}
