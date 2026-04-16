use sha1_crate::Digest;

use super::HashAlgorithm;

pub struct Sha1Hasher {
    inner: sha1_crate::Sha1,
}

impl Sha1Hasher {
    pub fn new() -> Self {
        Self {
            inner: sha1_crate::Sha1::new(),
        }
    }
}

impl HashAlgorithm for Sha1Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        hex_encode(&result)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
