use sha2::Digest;

use super::HashAlgorithm;

pub struct Sha256Hasher {
    inner: sha2::Sha256,
}

impl Sha256Hasher {
    pub fn new() -> Self {
        Self {
            inner: sha2::Sha256::new(),
        }
    }
}

impl HashAlgorithm for Sha256Hasher {
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
