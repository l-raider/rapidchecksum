use sha3::{Digest, Sha3_256};

use super::{hex_encode, HashAlgorithm};

pub struct Sha3_256Hasher {
    inner: Sha3_256,
}

impl Sha3_256Hasher {
    pub fn new() -> Self {
        Self {
            inner: Sha3_256::new(),
        }
    }
}

impl HashAlgorithm for Sha3_256Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        hex_encode(&result)
    }
}