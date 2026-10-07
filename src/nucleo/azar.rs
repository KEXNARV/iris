//! El azar de un buddy: xorshift con semilla fija, para que todo sea repetible. Es uno solo
//! por buddy y lo comparten sus piezas: el orden en que lo piden es parte de cómo se ve.

pub(crate) struct Azar(u64);

impl Azar {
    pub fn new() -> Self {
        Azar(0x9E37_79B9_7F4A_7C15)
    }

    /// Lo mezcla con `seed`, para que cada hijo tenga el suyo.
    pub fn sembrar(&mut self, seed: u64) {
        self.0 ^= seed.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1;
    }

    /// Un número en 0..1.
    pub fn rand(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}
