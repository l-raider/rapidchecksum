use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::hasher::HashKind;

/// Application settings, persisted as JSON.
///
/// To add a new setting: add a field here with `#[serde(default = "...")]`
/// and a corresponding default function. That's it — existing config files
/// will pick up the default automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_enabled_algorithms")]
    pub enabled_algorithms: Vec<HashKind>,
    #[serde(default = "default_true")]
    pub hash_uppercase: bool,
    #[serde(default = "default_rename_pattern")]
    pub rename_pattern: String,
    #[serde(default = "default_hidden_columns")]
    pub hidden_columns: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LegacyAppConfig {
    #[serde(default)]
    enabled_algorithms: Option<Vec<HashKind>>,
    #[serde(default = "default_true")]
    hash_uppercase: bool,
    #[serde(default = "default_rename_pattern")]
    rename_pattern: String,
    #[serde(default = "default_hidden_columns")]
    hidden_columns: Vec<String>,
    #[serde(default = "default_true")]
    hash_crc32: bool,
    #[serde(default = "default_true")]
    hash_md5: bool,
    #[serde(default = "default_true")]
    hash_sha1: bool,
    #[serde(default = "default_true")]
    hash_sha256: bool,
    #[serde(default = "default_true")]
    hash_sha512: bool,
}

fn default_true() -> bool {
    true
}

fn default_rename_pattern() -> String {
    "%FILENAME%.%FILEEXT%".to_string()
}

fn default_enabled_algorithms() -> Vec<HashKind> {
    HashKind::all().to_vec()
}

fn default_hidden_columns() -> Vec<String> {
    Vec::new()
}

fn normalize_enabled_algorithms(enabled_algorithms: &[HashKind]) -> Vec<HashKind> {
    HashKind::all()
        .iter()
        .copied()
        .filter(|kind| enabled_algorithms.contains(kind))
        .collect()
}

fn normalize_hidden_columns(hidden_columns: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    hidden_columns
        .iter()
        .filter(|column| !column.is_empty())
        .filter(|column| seen.insert((*column).clone()))
        .cloned()
        .collect()
}

impl LegacyAppConfig {
    fn into_current(self) -> AppConfig {
        let enabled_algorithms = if let Some(enabled_algorithms) = self.enabled_algorithms {
            normalize_enabled_algorithms(&enabled_algorithms)
        } else {
            let mut enabled_algorithms = default_enabled_algorithms();
            if !self.hash_crc32 {
                enabled_algorithms.retain(|kind| *kind != HashKind::CRC32);
            }
            if !self.hash_md5 {
                enabled_algorithms.retain(|kind| *kind != HashKind::MD5);
            }
            if !self.hash_sha1 {
                enabled_algorithms.retain(|kind| *kind != HashKind::SHA1);
            }
            if !self.hash_sha256 {
                enabled_algorithms.retain(|kind| *kind != HashKind::SHA256);
            }
            if !self.hash_sha512 {
                enabled_algorithms.retain(|kind| *kind != HashKind::SHA512);
            }
            enabled_algorithms
        };

        AppConfig {
            enabled_algorithms,
            hash_uppercase: self.hash_uppercase,
            rename_pattern: self.rename_pattern,
            hidden_columns: normalize_hidden_columns(&self.hidden_columns),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enabled_algorithms: default_enabled_algorithms(),
            hash_uppercase: true,
            rename_pattern: default_rename_pattern(),
            hidden_columns: default_hidden_columns(),
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
            .and_then(|s| serde_json::from_str::<LegacyAppConfig>(&s).ok())
            .map(LegacyAppConfig::into_current)
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
    pub fn enabled_hash_kinds(&self) -> Vec<HashKind> {
        normalize_enabled_algorithms(&self.enabled_algorithms)
    }

    pub fn is_hash_enabled(&self, kind: HashKind) -> bool {
        self.enabled_algorithms.contains(&kind)
    }

    pub fn set_hash_enabled(&mut self, kind: HashKind, enabled: bool) {
        if enabled {
            if !self.enabled_algorithms.contains(&kind) {
                self.enabled_algorithms.push(kind);
            }
        } else {
            self.enabled_algorithms.retain(|candidate| *candidate != kind);
        }

        self.enabled_algorithms = normalize_enabled_algorithms(&self.enabled_algorithms);
    }

    pub fn set_hidden_columns(&mut self, hidden_columns: &[String]) {
        self.hidden_columns = normalize_hidden_columns(hidden_columns);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_keeps_old_disabled_algorithms_and_enables_new_ones() {
        let legacy = r#"{
            "hash_crc32": true,
            "hash_md5": false,
            "hash_sha1": true,
            "hash_sha256": false,
            "hash_sha512": true,
            "hash_uppercase": false,
            "rename_pattern": "%FILENAME% [%CRC%].%FILEEXT%"
        }"#;

        let config = serde_json::from_str::<LegacyAppConfig>(legacy)
            .unwrap()
            .into_current();

        assert!(config.is_hash_enabled(HashKind::CRC32));
        assert!(!config.is_hash_enabled(HashKind::MD5));
        assert!(config.is_hash_enabled(HashKind::SHA1));
        assert!(!config.is_hash_enabled(HashKind::SHA256));
        assert!(config.is_hash_enabled(HashKind::SHA512));
        assert!(config.is_hash_enabled(HashKind::ED2K));
        assert!(config.is_hash_enabled(HashKind::BLAKE3));
        assert_eq!(config.hash_uppercase, false);
        assert_eq!(config.rename_pattern, "%FILENAME% [%CRC%].%FILEEXT%");
    }

    #[test]
    fn enabled_algorithms_are_normalized_to_known_order() {
        let mut config = AppConfig {
            enabled_algorithms: vec![HashKind::SHA3_512, HashKind::CRC32, HashKind::SHA3_256],
            ..AppConfig::default()
        };

        config.set_hash_enabled(HashKind::SHA3_256, true);

        assert_eq!(
            config.enabled_hash_kinds(),
            vec![HashKind::CRC32, HashKind::SHA3_256, HashKind::SHA3_512]
        );
    }

    #[test]
    fn hidden_columns_are_normalized_and_preserved() {
        let legacy = r#"{
            "hidden_columns": ["path", "path", "hash:sha256", ""]
        }"#;

        let config = serde_json::from_str::<LegacyAppConfig>(legacy)
            .unwrap()
            .into_current();

        assert_eq!(config.hidden_columns, vec!["path", "hash:sha256"]);
    }
}
