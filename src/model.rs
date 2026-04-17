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
}

impl FileEntry {
    pub fn new(path: PathBuf) -> Self {
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Self {
            path,
            filename,
            hashes: HashMap::new(),
            info: String::new(),
            error: None,
        }
    }

    pub fn hash_value(&self, kind: HashKind) -> &str {
        self.hashes.get(&kind).map(|s| s.as_str()).unwrap_or("")
    }
}
