use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings, persisted as JSON.
///
/// To add a new setting: add a field here with `#[serde(default = "...")]`
/// and a corresponding default function. That's it — existing config files
/// will pick up the default automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_true")]
    pub hash_crc32: bool,
    #[serde(default = "default_true")]
    pub hash_sha1: bool,
    #[serde(default = "default_true")]
    pub hash_sha256: bool,
    #[serde(default = "default_true")]
    pub hash_sha512: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hash_crc32: true,
            hash_sha1: true,
            hash_sha256: true,
            hash_sha512: true,
        }
    }
}

impl AppConfig {
    /// Path to the config file: ~/.config/rapidchecksum/settings.json
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("rapidchecksum").join("settings.json"))
    }

    /// Load from disk, falling back to defaults on any error.
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist to disk. Errors are silently ignored.
    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, json);
            }
        }
    }

    /// Return the list of HashKinds that are currently enabled.
    pub fn enabled_hash_kinds(&self) -> Vec<crate::hasher::HashKind> {
        use crate::hasher::HashKind;
        let mut kinds = Vec::new();
        if self.hash_crc32 {
            kinds.push(HashKind::CRC32);
        }
        if self.hash_sha1 {
            kinds.push(HashKind::SHA1);
        }
        if self.hash_sha256 {
            kinds.push(HashKind::SHA256);
        }
        if self.hash_sha512 {
            kinds.push(HashKind::SHA512);
        }
        kinds
    }
}
