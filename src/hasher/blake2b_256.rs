use ::blake2b_simd::Params;
use super::HashAlgorithm;

pub struct Blake2b256Hasher {
    inner: blake2b_simd::State,
}

impl Blake2b256Hasher {
    pub fn new() -> Self {
        Self {
            inner: Params::new().hash_length(32).to_state(),
        }
    }
}

impl HashAlgorithm for Blake2b256Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        self.inner.finalize().to_hex().to_string()
    }
}
