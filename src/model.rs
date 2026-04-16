use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel};

use crate::hasher::HashKind;

/// Holds the state for one file in the list.
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

    /// Convert this entry to a row of StandardListViewItems for the table.
    /// Column order: File, CRC32, SHA1, SHA256, SHA512, Info
    pub fn to_row(&self) -> ModelRc<StandardListViewItem> {
        let info_text = if let Some(ref err) = self.error {
            format!("Error: {}", err)
        } else {
            self.info.clone()
        };

        let items = vec![
            StandardListViewItem::from(SharedString::from(&self.filename)),
            StandardListViewItem::from(SharedString::from(self.hash_value(HashKind::CRC32))),
            StandardListViewItem::from(SharedString::from(self.hash_value(HashKind::SHA1))),
            StandardListViewItem::from(SharedString::from(self.hash_value(HashKind::SHA256))),
            StandardListViewItem::from(SharedString::from(self.hash_value(HashKind::SHA512))),
            StandardListViewItem::from(SharedString::from(&info_text)),
        ];
        ModelRc::new(VecModel::from(items))
    }
}

/// Manages the list of files and their Slint table model.
pub struct FileListModel {
    pub entries: Vec<FileEntry>,
    pub table_model: Rc<VecModel<ModelRc<StandardListViewItem>>>,
}

impl FileListModel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            table_model: Rc::new(VecModel::default()),
        }
    }

    pub fn add_file(&mut self, path: PathBuf) {
        let entry = FileEntry::new(path);
        let row = entry.to_row();
        self.entries.push(entry);
        self.table_model.push(row);
    }

    /// Update the hash results for a specific file and refresh its table row.
    pub fn update_hashes(&mut self, index: usize, hashes: HashMap<HashKind, String>, info: String) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.hashes = hashes;
            entry.info = info;
            self.table_model.set_row_data(index, entry.to_row());
        }
    }

    /// Mark a file as having an error and refresh its table row.
    pub fn set_error(&mut self, index: usize, error: String) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.error = Some(error);
            self.table_model.set_row_data(index, entry.to_row());
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        // Replace the model contents: remove all rows
        let count = self.table_model.row_count();
        for _ in (0..count).rev() {
            self.table_model.remove(0);
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
            self.table_model.remove(index);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn model_rc(&self) -> ModelRc<ModelRc<StandardListViewItem>> {
        ModelRc::from(self.table_model.clone())
    }
}
