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

    /// Convert this entry to a row for the given set of visible hash kinds.
    /// Column order: File, [enabled hash columns...], Info
    pub fn to_row(&self, visible_kinds: &[HashKind]) -> ModelRc<StandardListViewItem> {
        let info_text = if let Some(ref err) = self.error {
            format!("Error: {}", err)
        } else {
            self.info.clone()
        };

        let mut items = Vec::with_capacity(2 + visible_kinds.len());
        items.push(StandardListViewItem::from(SharedString::from(&self.filename)));
        for &kind in visible_kinds {
            items.push(StandardListViewItem::from(SharedString::from(self.hash_value(kind))));
        }
        items.push(StandardListViewItem::from(SharedString::from(&info_text)));
        ModelRc::new(VecModel::from(items))
    }
}

/// Manages the list of files and their Slint table model.
pub struct FileListModel {
    pub entries: Vec<FileEntry>,
    pub table_model: Rc<VecModel<ModelRc<StandardListViewItem>>>,
    /// Which hash columns are currently visible (drives row generation).
    pub visible_kinds: Vec<HashKind>,
}

impl FileListModel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            table_model: Rc::new(VecModel::default()),
            visible_kinds: vec![HashKind::CRC32, HashKind::SHA1, HashKind::SHA256, HashKind::SHA512],
        }
    }

    pub fn add_file(&mut self, path: PathBuf) {
        let entry = FileEntry::new(path);
        let row = entry.to_row(&self.visible_kinds);
        self.entries.push(entry);
        self.table_model.push(row);
    }

    /// Update the hash results for a specific file and refresh its table row.
    pub fn update_hashes(&mut self, index: usize, hashes: HashMap<HashKind, String>, info: String) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.hashes = hashes;
            entry.info = info;
            self.table_model.set_row_data(index, entry.to_row(&self.visible_kinds));
        }
    }

    /// Mark a file as having an error and refresh its table row.
    pub fn set_error(&mut self, index: usize, error: String) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.error = Some(error);
            self.table_model.set_row_data(index, entry.to_row(&self.visible_kinds));
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

    /// Update which hash columns are visible and refresh all rows.
    pub fn set_visible_kinds(&mut self, kinds: Vec<HashKind>) {
        self.visible_kinds = kinds;
        self.refresh_all_rows();
    }

    /// Refresh all table rows (e.g. after column change).
    pub fn refresh_all_rows(&self) {
        for (i, entry) in self.entries.iter().enumerate() {
            self.table_model.set_row_data(i, entry.to_row(&self.visible_kinds));
        }
    }

    /// Sort entries by the given dynamic column index.
    pub fn sort(&mut self, column: usize, ascending: bool) {
        let visible_kinds = self.visible_kinds.clone();
        self.entries.sort_by(|a, b| {
            let a_val = sort_key_for_kinds(a, column, &visible_kinds);
            let b_val = sort_key_for_kinds(b, column, &visible_kinds);
            if ascending {
                a_val.cmp(&b_val)
            } else {
                b_val.cmp(&a_val)
            }
        });
        self.refresh_all_rows();
    }

    pub fn model_rc(&self) -> ModelRc<ModelRc<StandardListViewItem>> {
        ModelRc::from(self.table_model.clone())
    }
}

/// Standalone sort-key helper (avoids borrow issues in sort closure).
fn sort_key_for_kinds(entry: &FileEntry, column: usize, visible_kinds: &[HashKind]) -> String {
    if column == 0 {
        return entry.filename.to_lowercase();
    }
    let info_col = 1 + visible_kinds.len();
    if column == info_col {
        return if let Some(ref err) = entry.error {
            format!("error: {}", err.to_lowercase())
        } else {
            entry.info.to_lowercase()
        };
    }
    let kind_idx = column - 1;
    if let Some(&kind) = visible_kinds.get(kind_idx) {
        entry.hash_value(kind).to_ascii_lowercase()
    } else {
        String::new()
    }
}
