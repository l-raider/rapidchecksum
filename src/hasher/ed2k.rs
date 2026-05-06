use ::ed2k::digest::Digest;

use super::HashAlgorithm;

pub struct Ed2kHasher {
    inner: ::ed2k::Ed2k,
}

impl Ed2kHasher {
    pub fn new() -> Self {
        Self {
            inner: ::ed2k::Ed2k::new(),
        }
    }
}

impl HashAlgorithm for Ed2kHasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        format!("{result:x}")
    }
}