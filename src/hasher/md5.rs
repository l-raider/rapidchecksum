use md5::{Digest, Md5};
use openssl::hash::{Hasher, MessageDigest};

use super::{hex_encode, HashAlgorithm};

pub struct Md5Hasher {
    inner: Md5Backend,
}

enum Md5Backend {
    OpenSsl(OpenSslMd5Hasher),
    Rust(Md5),
}

struct OpenSslMd5Hasher {
    inner: Option<Hasher>,
}

impl Md5Hasher {
    pub fn new() -> Self {
        if let Some(inner) = OpenSslMd5Hasher::new() {
            return Self {
                inner: Md5Backend::OpenSsl(inner),
            };
        }

        Self {
            inner: Md5Backend::Rust(Md5::new()),
        }
    }
}

impl HashAlgorithm for Md5Hasher {
    fn update(&mut self, data: &[u8]) {
        match &mut self.inner {
            Md5Backend::OpenSsl(hasher) => hasher.update(data),
            Md5Backend::Rust(hasher) => hasher.update(data),
        }
    }

    fn finalize(self: Box<Self>) -> String {
        let result = match self.inner {
            Md5Backend::OpenSsl(hasher) => hasher.finalize(),
            Md5Backend::Rust(hasher) => hasher.finalize().to_vec(),
        };

        hex_encode(&result)
    }
}

impl OpenSslMd5Hasher {
    fn new() -> Option<Self> {
        let inner = Hasher::new(MessageDigest::md5()).ok()?;
        Some(Self { inner: Some(inner) })
    }

    fn update(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        if let Some(ref mut inner) = self.inner {
            if inner.update(data).is_err() {
                // OpenSSL update failed; mark as failed so finalize returns empty
                self.inner = None;
            }
        }
    }

    fn finalize(mut self) -> Vec<u8> {
        match self.inner.take() {
            Some(mut inner) => inner
                .finish()
                .map(|b| b.as_ref().to_vec())
                .unwrap_or_default(),
            None => Vec::new(),
        }
    }
}
