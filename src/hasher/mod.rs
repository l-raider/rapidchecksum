mod crc32;
mod sha1;
mod sha256;
mod sha512;

use std::fmt;

/// Identifies which hash algorithm to use.
/// Adding a new algorithm: add a variant here, implement HashAlgorithm,
/// and register it in `HashKind::all()` and `create_hasher()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HashKind {
    CRC32,
    SHA1,
    SHA256,
    SHA512,
}

impl HashKind {
    pub fn all() -> &'static [HashKind] {
        &[
            HashKind::CRC32,
            HashKind::SHA1,
            HashKind::SHA256,
            HashKind::SHA512,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            HashKind::CRC32 => "CRC32",
            HashKind::SHA1 => "SHA1",
            HashKind::SHA256 => "SHA256",
            HashKind::SHA512 => "SHA512",
        }
    }

    /// File extension used for hash files (e.g. "sfv" for CRC32)
    pub fn file_extension(&self) -> &'static str {
        match self {
            HashKind::CRC32 => "sfv",
            HashKind::SHA1 => "sha1",
            HashKind::SHA256 => "sha256",
            HashKind::SHA512 => "sha512",
        }
    }
}

impl fmt::Display for HashKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Trait for incremental hash computation. Each algorithm wraps its
/// underlying implementation behind this uniform interface.
pub trait HashAlgorithm: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize(self: Box<Self>) -> String;
}

/// Factory: create a fresh hasher for the given algorithm.
pub fn create_hasher(kind: HashKind) -> Box<dyn HashAlgorithm> {
    match kind {
        HashKind::CRC32 => Box::new(crc32::Crc32Hasher::new()),
        HashKind::SHA1 => Box::new(sha1::Sha1Hasher::new()),
        HashKind::SHA256 => Box::new(sha256::Sha256Hasher::new()),
        HashKind::SHA512 => Box::new(sha512::Sha512Hasher::new()),
    }
}
