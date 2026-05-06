use sha3::{Digest, Sha3_224};

use super::{hex_encode, HashAlgorithm};

pub struct Sha3_224Hasher {
    inner: Sha3_224,
}

impl Sha3_224Hasher {
    pub fn new() -> Self {
        Self {
            inner: Sha3_224::new(),
        }
    }
}

impl HashAlgorithm for Sha3_224Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        hex_encode(&result)
    }
}