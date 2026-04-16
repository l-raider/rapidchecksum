use md5::Digest;

use super::HashAlgorithm;

pub struct Md5Hasher {
    inner: md5::Md5,
}

impl Md5Hasher {
    pub fn new() -> Self {
        Self {
            inner: md5::Md5::new(),
        }
    }
}

impl HashAlgorithm for Md5Hasher {
    fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.inner.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
