use super::HashAlgorithm;

pub struct Blake3Hasher {
    inner: ::blake3::Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self {
            inner: ::blake3::Hasher::new(),
        }
    }
}

impl HashAlgorithm for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        self.inner.finalize().to_hex().to_string()
    }
}