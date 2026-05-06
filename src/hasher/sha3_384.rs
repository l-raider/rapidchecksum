use sha3::{Digest, Sha3_384};

use super::{hex_encode, HashAlgorithm};

pub struct Sha3_384Hasher {
    inner: Sha3_384,
}

impl Sha3_384Hasher {
    pub fn new() -> Self {
        Self {
            inner: Sha3_384::new(),
        }
    }
}

impl HashAlgorithm for Sha3_384Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        hex_encode(&result)
    }
}