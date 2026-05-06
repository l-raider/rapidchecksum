mod blake2b_256;
mod blake2b_512;
mod blake2sp;
mod blake3;
mod crc32;
mod ed2k;
mod md5;
mod sha1;
mod sha3_224;
mod sha3_256;
mod sha3_384;
mod sha3_512;
mod sha256;
mod sha512;

use std::fmt;

use serde::{Deserialize, Serialize};

/// Identifies which hash algorithm to use.
/// Adding a new algorithm: add a variant here, implement HashAlgorithm,
/// and register it in `create_hasher()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HashKind {
    #[serde(rename = "crc32")]
    CRC32,
    #[serde(rename = "md5")]
    MD5,
    #[serde(rename = "sha1")]
    SHA1,
    #[serde(rename = "sha256")]
    SHA256,
    #[serde(rename = "sha512")]
    SHA512,
    #[serde(rename = "ed2k")]
    ED2K,
    #[serde(rename = "blake3")]
    BLAKE3,
    #[serde(rename = "blake2sp")]
    BLAKE2sp,
    #[serde(rename = "blake2b_256")]
    BLAKE2b_256,
    #[serde(rename = "blake2b_512")]
    BLAKE2b_512,
    #[serde(rename = "sha3_224")]
    SHA3_224,
    #[serde(rename = "sha3_256")]
    SHA3_256,
    #[serde(rename = "sha3_384")]
    SHA3_384,
    #[serde(rename = "sha3_512")]
    SHA3_512,
}

impl HashKind {
    pub const fn all() -> &'static [Self] {
        &[
            Self::CRC32,
            Self::MD5,
            Self::SHA1,
            Self::SHA256,
            Self::SHA512,
            Self::ED2K,
            Self::BLAKE3,
            Self::BLAKE2sp,
            Self::BLAKE2b_256,
            Self::BLAKE2b_512,
            Self::SHA3_224,
            Self::SHA3_256,
            Self::SHA3_384,
            Self::SHA3_512,
        ]
    }

    pub const fn id(self) -> &'static str {
        match self {
            HashKind::CRC32 => "crc32",
            HashKind::MD5 => "md5",
            HashKind::SHA1 => "sha1",
            HashKind::SHA256 => "sha256",
            HashKind::SHA512 => "sha512",
            HashKind::ED2K => "ed2k",
            HashKind::BLAKE3 => "blake3",
            HashKind::BLAKE2sp => "blake2sp",
            HashKind::BLAKE2b_256 => "blake2b_256",
            HashKind::BLAKE2b_512 => "blake2b_512",
            HashKind::SHA3_224 => "sha3_224",
            HashKind::SHA3_256 => "sha3_256",
            HashKind::SHA3_384 => "sha3_384",
            HashKind::SHA3_512 => "sha3_512",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::all().iter().copied().find(|kind| kind.id() == id)
    }

    pub fn name(&self) -> &'static str {
        match self {
            HashKind::CRC32 => "CRC32",
            HashKind::MD5 => "MD5",
            HashKind::SHA1 => "SHA1",
            HashKind::SHA256 => "SHA256",
            HashKind::SHA512 => "SHA512",
            HashKind::ED2K => "ED2K",
            HashKind::BLAKE3 => "BLAKE3",
            HashKind::BLAKE2sp => "BLAKE2sp",
            HashKind::BLAKE2b_256 => "BLAKE2b-256",
            HashKind::BLAKE2b_512 => "BLAKE2b-512",
            HashKind::SHA3_224 => "SHA3-224",
            HashKind::SHA3_256 => "SHA3-256",
            HashKind::SHA3_384 => "SHA3-384",
            HashKind::SHA3_512 => "SHA3-512",
        }
    }

    pub const fn rename_placeholder(self) -> &'static str {
        match self {
            HashKind::CRC32 => "%CRC%",
            HashKind::MD5 => "%MD5%",
            HashKind::SHA1 => "%SHA1%",
            HashKind::SHA256 => "%SHA256%",
            HashKind::SHA512 => "%SHA512%",
            HashKind::ED2K => "%ED2K%",
            HashKind::BLAKE3 => "%BLAKE3%",
            HashKind::BLAKE2sp => "%BLAKE2SP%",
            HashKind::BLAKE2b_256 => "%BLAKE2B256%",
            HashKind::BLAKE2b_512 => "%BLAKE2B512%",
            HashKind::SHA3_224 => "%SHA3_224%",
            HashKind::SHA3_256 => "%SHA3_256%",
            HashKind::SHA3_384 => "%SHA3_384%",
            HashKind::SHA3_512 => "%SHA3_512%",
        }
    }

    pub const fn output_hex_len(self) -> usize {
        match self {
            HashKind::CRC32 => 8,
            HashKind::MD5 => 32,
            HashKind::SHA1 => 40,
            HashKind::SHA256 => 64,
            HashKind::SHA512 => 128,
            HashKind::ED2K => 32,
            HashKind::BLAKE3 => 64,
            HashKind::BLAKE2sp => 64,
            HashKind::BLAKE2b_256 => 64,
            HashKind::BLAKE2b_512 => 128,
            HashKind::SHA3_224 => 56,
            HashKind::SHA3_256 => 64,
            HashKind::SHA3_384 => 96,
            HashKind::SHA3_512 => 128,
        }
    }

    pub const fn save_dialog_label(self) -> &'static str {
        match self {
            HashKind::CRC32 => "CRC32 / SFV",
            HashKind::MD5 => "MD5",
            HashKind::SHA1 => "SHA1",
            HashKind::SHA256 => "SHA256",
            HashKind::SHA512 => "SHA512",
            HashKind::ED2K => "ED2K",
            HashKind::BLAKE3 => "BLAKE3",
            HashKind::BLAKE2sp => "BLAKE2sp",
            HashKind::BLAKE2b_256 => "BLAKE2b-256",
            HashKind::BLAKE2b_512 => "BLAKE2b-512",
            HashKind::SHA3_224 => "SHA3-224",
            HashKind::SHA3_256 => "SHA3-256",
            HashKind::SHA3_384 => "SHA3-384",
            HashKind::SHA3_512 => "SHA3-512",
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
        HashKind::MD5 => Box::new(md5::Md5Hasher::new()),
        HashKind::SHA1 => Box::new(sha1::Sha1Hasher::new()),
        HashKind::SHA256 => Box::new(sha256::Sha256Hasher::new()),
        HashKind::SHA512 => Box::new(sha512::Sha512Hasher::new()),
        HashKind::ED2K => Box::new(ed2k::Ed2kHasher::new()),
        HashKind::BLAKE3 => Box::new(blake3::Blake3Hasher::new()),
        HashKind::BLAKE2sp => Box::new(blake2sp::Blake2spHasher::new()),
        HashKind::BLAKE2b_256 => Box::new(blake2b_256::Blake2b256Hasher::new()),
        HashKind::BLAKE2b_512 => Box::new(blake2b_512::Blake2b512Hasher::new()),
        HashKind::SHA3_224 => Box::new(sha3_224::Sha3_224Hasher::new()),
        HashKind::SHA3_256 => Box::new(sha3_256::Sha3_256Hasher::new()),
        HashKind::SHA3_384 => Box::new(sha3_384::Sha3_384Hasher::new()),
        HashKind::SHA3_512 => Box::new(sha3_512::Sha3_512Hasher::new()),
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_kind_ids_round_trip_in_declared_order() {
        let all_ids: Vec<_> = HashKind::all().iter().map(|kind| kind.id()).collect();

        assert_eq!(
            all_ids,
            vec![
                "crc32",
                "md5",
                "sha1",
                "sha256",
                "sha512",
                "ed2k",
                "blake3",
                "blake2sp",
                "blake2b_256",
                "blake2b_512",
                "sha3_224",
                "sha3_256",
                "sha3_384",
                "sha3_512",
            ]
        );
        assert_eq!(HashKind::from_id("ed2k"), Some(HashKind::ED2K));
        assert_eq!(HashKind::from_id("sha3_512"), Some(HashKind::SHA3_512));
        assert_eq!(HashKind::from_id("unknown"), None);
    }
}
