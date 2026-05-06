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

    pub fn formatted_hash_value(&self, kind: HashKind, uppercase: bool) -> String {
        let value = self.hash_value(kind);
        if uppercase {
            value.to_ascii_uppercase()
        } else {
            value.to_ascii_lowercase()
        }
    }

    pub fn set_expected_crc32(&mut self, expected_crc32: &str) {
        self.expected_crc32 = Some(expected_crc32.to_ascii_uppercase());
    }

    /// Re-parse the CRC32 tag from the current filename.
    /// Must be called whenever `filename` is updated (e.g. after a rename).
    pub fn refresh_expected_crc32(&mut self) {
        self.expected_crc32 = parse_crc32_from_filename(&self.filename);
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
    let mut parsed = None;
    for i in 0..bytes.len() {
        if bytes[i] == b'[' && i + 10 <= bytes.len() && bytes[i + 9] == b']' {
            let candidate = &filename[i + 1..i + 9];
            if candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
                parsed = Some(candidate.to_uppercase());
            }
        }
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hash_values_in_requested_case() {
        let mut entry = FileEntry::default();
        entry.hashes.insert(HashKind::MD5, "a1b2c3d4".to_string());
        entry.hashes.insert(HashKind::CRC32, "DEADBEEF".to_string());

        assert_eq!(entry.formatted_hash_value(HashKind::MD5, true), "A1B2C3D4");
        assert_eq!(entry.formatted_hash_value(HashKind::CRC32, false), "deadbeef");
    }

    #[test]
    fn parse_crc32_from_filename_prefers_last_tag() {
        assert_eq!(
            parse_crc32_from_filename("movie [DEADBEEF] [CAFEBABE].mkv"),
            Some("CAFEBABE".to_string())
        );
    }

    #[test]
    fn set_expected_crc32_normalizes_case() {
        let mut entry = FileEntry::default();

        entry.set_expected_crc32("deadbeef");

        assert_eq!(entry.expected_crc32.as_deref(), Some("DEADBEEF"));
    }
}
