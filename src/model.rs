use std::collections::HashMap;
use std::path::PathBuf;

use crate::hasher::HashKind;

#[derive(Clone, Default)]
pub struct FileEntry {
    pub path: PathBuf,
    pub filename: String,
    pub hashes: HashMap<HashKind, String>,
    pub info: String,
    pub error: Option<String>,
    pub expected_crc32: Option<String>,
}

impl FileEntry {
    pub fn new(path: PathBuf) -> Self {
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let expected_crc32 = parse_crc32_from_filename(&filename);
        Self {
            path,
            filename,
            hashes: HashMap::new(),
            info: String::new(),
            error: None,
            expected_crc32,
        }
    }

    pub fn hash_value(&self, kind: HashKind) -> &str {
        self.hashes.get(&kind).map(|s| s.as_str()).unwrap_or("")
    }

    /// Verify status: 0 = no expected hash / not yet computed, 1 = match, 2 = mismatch.
    pub fn verify_status(&self) -> i32 {
        match &self.expected_crc32 {
            None => 0,
            Some(expected) => {
                let computed = self.hash_value(HashKind::CRC32);
                if computed.is_empty() {
                    0
                } else if computed.eq_ignore_ascii_case(expected) {
                    1
                } else {
                    2
                }
            }
        }
    }
}

/// Extract a CRC32 hash embedded in a filename as `[XXXXXXXX]` (exactly 8 hex digits).
fn parse_crc32_from_filename(filename: &str) -> Option<String> {
    let bytes = filename.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'[' && i + 10 <= bytes.len() && bytes[i + 9] == b']' {
            let candidate = &filename[i + 1..i + 9];
            if candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Some(candidate.to_uppercase());
            }
        }
    }
    None
}
