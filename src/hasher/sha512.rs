use sha2::Digest;

use super::{hex_encode, HashAlgorithm};

pub struct Sha512Hasher {
    inner: sha2::Sha512,
}

impl Sha512Hasher {
    pub fn new() -> Self {
        Self {
            inner: sha2::Sha512::new(),
        }
    }
}

impl HashAlgorithm for Sha512Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        hex_encode(&result)
    }
}
