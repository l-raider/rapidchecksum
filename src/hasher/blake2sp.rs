use ::blake2s_simd::blake2sp;

use super::HashAlgorithm;

pub struct Blake2spHasher {
    inner: blake2sp::State,
}

impl Blake2spHasher {
    pub fn new() -> Self {
        Self {
            inner: blake2sp::Params::new().to_state(),
        }
    }
}

impl HashAlgorithm for Blake2spHasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        self.inner.finalize().to_hex().to_string()
    }
}