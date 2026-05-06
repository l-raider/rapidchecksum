use sha3::{Digest, Sha3_512};

use super::{hex_encode, HashAlgorithm};

pub struct Sha3_512Hasher {
    inner: Sha3_512,
}

impl Sha3_512Hasher {
    pub fn new() -> Self {
        Self {
            inner: Sha3_512::new(),
        }
    }
}

impl HashAlgorithm for Sha3_512Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        hex_encode(&result)
    }
}