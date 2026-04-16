use super::HashAlgorithm;

pub struct Crc32Hasher {
    inner: crc32fast::Hasher,
}

impl Crc32Hasher {
    pub fn new() -> Self {
        Self {
            inner: crc32fast::Hasher::new(),
        }
    }
}

impl HashAlgorithm for Crc32Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        format!("{:08X}", self.inner.finalize())
    }
}
