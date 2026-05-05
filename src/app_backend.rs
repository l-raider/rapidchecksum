#![allow(unused_attributes)]
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};

use crate::config::AppConfig;
use crate::fileio::write_hash_file;
use crate::hasher::HashKind;
use crate::model::FileEntry;
use crate::worker::{spawn_hash_worker, FileTask, WorkerMessage};

extern "C" {
    fn qt_set_clipboard(text: *const std::ffi::c_char);
}

fn set_clipboard(text: &str) {
    let cstr = std::ffi::CString::new(text).unwrap_or_default();
    unsafe { qt_set_clipboard(cstr.as_ptr()); }
}

// Role IDs for QAbstractTableModel
const ROLE_DISPLAY: i32 = 0;    // Qt::DisplayRole
const ROLE_IS_ERROR: i32 = 256;  // Qt::UserRole
const ROLE_IS_SELECTED: i32 = 257; // Qt::UserRole + 1
const ROLE_VERIFY_STATUS: i32 = 258; // Qt::UserRole + 2

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++Qt" {
        include!("QtCore/QAbstractTableModel");
        #[qobject]
        type QAbstractTableModel;
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;

        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qvector.h");
        type QVector_i32 = cxx_qt_lib::QVector<i32>;

        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;

        include!("cxx-qt-lib/qt.h");
        #[namespace = "Qt"]
        type Orientation = cxx_qt_lib::Orientation;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[base = QAbstractTableModel]
        #[qproperty(bool, is_hashing)]
        #[qproperty(f32, file_progress)]
        #[qproperty(f32, global_progress)]
        #[qproperty(QString, status_text)]
        #[qproperty(i32, selected_row)]
        #[qproperty(bool, setting_crc32)]
        #[qproperty(bool, setting_md5)]
        #[qproperty(bool, setting_sha1)]
        #[qproperty(bool, setting_sha256)]
        #[qproperty(bool, setting_sha512)]
        #[qproperty(i32, file_count)]
        #[qproperty(QString, setting_rename_pattern)]
        #[qproperty(QString, app_version)]
        type AppBackend = super::AppBackendRust;

        // QAbstractTableModel overrides
        #[cxx_override]
        fn data(self: &AppBackend, index: &QModelIndex, role: i32) -> QVariant;
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &AppBackend, parent: &QModelIndex) -> i32;
        #[cxx_override]
        #[cxx_name = "columnCount"]
        fn column_count(self: &AppBackend, parent: &QModelIndex) -> i32;
        #[cxx_override]
        #[cxx_name = "headerData"]
        fn header_data(
            self: &AppBackend,
            section: i32,
            orientation: Orientation,
            role: i32,
        ) -> QVariant;
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &AppBackend) -> QHash_i32_QByteArray;

        // Inherited model manipulation methods
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut AppBackend>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut AppBackend>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(
            self: Pin<&mut AppBackend>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut AppBackend>);
        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut AppBackend>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut AppBackend>);

        // Inherited signal
        #[inherit]
        #[qsignal]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut AppBackend>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QVector_i32,
        );

        // Needed to create QModelIndex for dataChanged / inherited methods
        #[inherit]
        #[cxx_name = "index"]
        fn index(self: &AppBackend, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        // Invokables
        #[qinvokable]
        fn add_files(self: Pin<&mut AppBackend>, paths: &QStringList);
        #[qinvokable]
        fn add_folder(self: Pin<&mut AppBackend>, folder_path: &QString);
        #[qinvokable]
        fn start_hashing(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn cancel_hashing(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn clear_list(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn remove_selected(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn select_row(self: Pin<&mut AppBackend>, row: i32);
        #[qinvokable]
        fn sort_by(self: Pin<&mut AppBackend>, column: i32, ascending: bool);
        #[qinvokable]
        fn copy_filepath(self: &AppBackend);
        #[qinvokable]
        fn copy_hash(self: &AppBackend, algo: i32);
        #[qinvokable]
        fn open_folder(self: &AppBackend);
        #[qinvokable]
        fn save_hash_file(self: &AppBackend, algo: i32, path: &QString);
        #[qinvokable]
        fn apply_settings(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn visible_columns(self: &AppBackend) -> QStringList;
        #[qinvokable]
        fn apply_rename_settings(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn rename_files(self: Pin<&mut AppBackend>);
        #[qinvokable]
        fn get_rename_preview(self: &AppBackend) -> QString;
    }

    impl cxx_qt::Threading for AppBackend {}
}

use cxx_qt_lib::{
    QByteArray, Orientation, QHash, QHashPair_i32_QByteArray, QModelIndex, QString,
    QStringList, QVariant, QVector,
};

pub struct AppBackendRust {
    entries: Vec<FileEntry>,
    visible_kinds: Vec<HashKind>,
    config: AppConfig,
    cancel_flag: Option<Arc<AtomicBool>>,
    files_completed: usize,
    total_files: usize,
    start_time: Option<Instant>,
    hash_generation: u64,

    // Property backing fields
    is_hashing: bool,
    file_progress: f32,
    global_progress: f32,
    status_text: QString,
    selected_row: i32,
    setting_crc32: bool,
    setting_md5: bool,
    setting_sha1: bool,
    setting_sha256: bool,
    setting_sha512: bool,
    file_count: i32,
    setting_rename_pattern: QString,
    app_version: QString,
}

impl Default for AppBackendRust {
    fn default() -> Self {
        let config = AppConfig::load();
        let rename_pattern = config.rename_pattern.clone();
        Self {
            entries: Vec::new(),
            visible_kinds: config.enabled_hash_kinds(),
            cancel_flag: None,
            files_completed: 0,
            total_files: 0,
            start_time: None,
            setting_crc32: config.hash_crc32,
            setting_md5: config.hash_md5,
            setting_sha1: config.hash_sha1,
            setting_sha256: config.hash_sha256,
            setting_sha512: config.hash_sha512,
            config,
            hash_generation: 0,
            file_count: 0,
            is_hashing: false,
            file_progress: 0.0,
            global_progress: 0.0,
            status_text: QString::from("Ready"),
            selected_row: -1,
            setting_rename_pattern: QString::from(&rename_pattern),
            app_version: QString::from(env!("CARGO_PKG_VERSION")),
        }
    }
}

impl qobject::AppBackend {
    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            return 0;
        }
        self.rust().entries.len() as i32
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        if !index.is_valid() {
            return QVariant::default();
        }
        let row = index.row() as usize;
        let col = index.column() as usize;
        let rust = self.rust();
        let entries = &rust.entries;
        if row >= entries.len() {
            return QVariant::default();
        }
        let entry = &entries[row];
        let visible_kinds = &rust.visible_kinds;
        let num_hash_cols = visible_kinds.len();
        match role {
            ROLE_DISPLAY => {
                if col == 0 {
                    QVariant::from(&QString::from(&entry.filename))
                } else if col >= 1 && col <= num_hash_cols {
                    let kind = visible_kinds[col - 1];
                    QVariant::from(&QString::from(entry.hash_value(kind)))
                } else if col == num_hash_cols + 1 {
                    let text = match entry.verify_status() {
                        1 => "\u{2713} Match",
                        2 => "\u{2717} Mismatch",
                        _ => "",
                    };
                    QVariant::from(&QString::from(text))
                } else if col == num_hash_cols + 2 {
                    let text = if let Some(ref err) = entry.error {
                        format!("Error: {}", err)
                    } else {
                        entry.info.clone()
                    };
                    QVariant::from(&QString::from(&text))
                } else {
                    QVariant::default()
                }
            }
            ROLE_IS_ERROR => QVariant::from(&entry.error.is_some()),
            ROLE_IS_SELECTED => QVariant::from(&(rust.selected_row == row as i32)),
            ROLE_VERIFY_STATUS => QVariant::from(&entry.verify_status()),
            _ => QVariant::default(),
        }
    }

    fn header_data(&self, section: i32, orientation: Orientation, role: i32) -> QVariant {
        if role != ROLE_DISPLAY || section < 0 {
            return QVariant::default();
        }

        if orientation == Orientation::Vertical {
            return QVariant::from(&(section + 1));
        }

        let visible_kinds = &self.rust().visible_kinds;
        let hash_cols = visible_kinds.len() as i32;

        let label = if section == 0 {
            Some(QString::from("Filename"))
        } else if section >= 1 && section <= hash_cols {
            Some(QString::from(visible_kinds[(section - 1) as usize].name()))
        } else if section == hash_cols + 1 {
            Some(QString::from("Verify"))
        } else if section == hash_cols + 2 {
            Some(QString::from("Info"))
        } else {
            None
        };

        label
            .map(|text| QVariant::from(&text))
            .unwrap_or_default()
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut map = QHash::<QHashPair_i32_QByteArray>::default();
        map.insert(ROLE_DISPLAY, QByteArray::from("display"));
        map.insert(ROLE_IS_ERROR, QByteArray::from("isError"));
        map.insert(ROLE_IS_SELECTED, QByteArray::from("isSelected"));
        map.insert(ROLE_VERIFY_STATUS, QByteArray::from("verifyStatus"));
        map
    }

    fn column_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            return 0;
        }
        (3 + self.rust().visible_kinds.len()) as i32
    }

    fn add_files(mut self: Pin<&mut Self>, paths: &QStringList) {
        let mut new_paths: Vec<PathBuf> = Vec::new();
        for i in 0..paths.len() {
            if let Some(s) = paths.get(i) {
                let path = PathBuf::from(s.to_string());
                if path.is_file() {
                    new_paths.push(path);
                }
            }
        }
        if new_paths.is_empty() {
            return;
        }

        let start_row = self.rust().entries.len() as i32;
        let end_row = start_row + new_paths.len() as i32 - 1;

        let invalid = QModelIndex::default();
        unsafe {
            self.as_mut().begin_insert_rows(&invalid, start_row, end_row);
        }
        for path in new_paths {
            let entry = FileEntry::new(path);
            self.as_mut().rust_mut().entries.push(entry);
        }
        unsafe {
            self.as_mut().end_insert_rows();
        }

        let count = self.rust().entries.len();
        self.as_mut().set_file_count(count as i32);
        let text = format!("{} file(s) loaded", count);
        self.as_mut().set_status_text(QString::from(&text));
    }

    fn add_folder(mut self: Pin<&mut Self>, folder_path: &QString) {
        if self.rust().is_hashing {
            return;
        }
        let path = PathBuf::from(folder_path.to_string());
        let mut new_paths: Vec<PathBuf> = Vec::new();
        collect_files_recursive(&path, &mut new_paths);
        if new_paths.is_empty() {
            return;
        }

        let start_row = self.rust().entries.len() as i32;
        let end_row = start_row + new_paths.len() as i32 - 1;

        let invalid = QModelIndex::default();
        unsafe {
            self.as_mut().begin_insert_rows(&invalid, start_row, end_row);
        }
        for path in new_paths {
            let entry = FileEntry::new(path);
            self.as_mut().rust_mut().entries.push(entry);
        }
        unsafe {
            self.as_mut().end_insert_rows();
        }

        let count = self.rust().entries.len();
        self.as_mut().set_file_count(count as i32);
        let text = format!("{} file(s) loaded", count);
        self.as_mut().set_status_text(QString::from(&text));
    }

    fn start_hashing(mut self: Pin<&mut Self>) {
        if self.rust().is_hashing {
            return;
        }
        let kinds = self.rust().visible_kinds.clone();
        if kinds.is_empty() {
            self.as_mut()
                .set_status_text(QString::from("No hash algorithms selected"));
            return;
        }

        let tasks: Vec<FileTask> = self
            .rust()
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| FileTask {
                index: i,
                path: e.path.clone(),
            })
            .collect();

        if tasks.is_empty() {
            self.as_mut()
                .set_status_text(QString::from("No files to hash"));
            return;
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel::<WorkerMessage>();

        let total = tasks.len();
        self.as_mut().rust_mut().hash_generation += 1;
        let generation = self.rust().hash_generation;
        self.as_mut().rust_mut().cancel_flag = Some(cancel.clone());
        self.as_mut().rust_mut().files_completed = 0;
        self.as_mut().rust_mut().total_files = total;
        self.as_mut().rust_mut().start_time = Some(Instant::now());
        self.as_mut().set_is_hashing(true);
        self.as_mut().set_file_progress(0.0);
        self.as_mut().set_global_progress(0.0);
        self.as_mut()
            .set_status_text(QString::from("Hashing..."));

        let qt_thread = self.as_ref().get_ref().qt_thread();
        spawn_hash_worker(tasks, kinds, tx, cancel);

        std::thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let msg_clone = msg.clone();
                qt_thread
                    .queue(move |mut backend: std::pin::Pin<&mut qobject::AppBackend>| {
                        backend.as_mut().handle_worker_message(generation, msg_clone);
                    })
                    .ok();
            }
        });
    }

    fn handle_worker_message(mut self: Pin<&mut Self>, generation: u64, msg: WorkerMessage) {
        // Ignore stale messages from a cancelled/completed run
        if !self.rust().is_hashing || generation != self.rust().hash_generation {
            return;
        }
        match msg {
            WorkerMessage::FileProgress {
                file_index,
                bytes_read,
                total_bytes,
            } => {
                let progress = if total_bytes > 0 {
                    bytes_read as f32 / total_bytes as f32
                } else {
                    0.0
                };
                self.as_mut().set_file_progress(progress);

                let completed = self.rust().files_completed;
                let total = self.rust().total_files;
                if total > 0 {
                    let gp = (completed as f32 + progress) / total as f32;
                    self.as_mut().set_global_progress(gp);
                }

                let text = format!(
                    "Hashing file {}/{}...",
                    file_index + 1,
                    self.rust().total_files
                );
                self.as_mut().set_status_text(QString::from(&text));
            }
            WorkerMessage::FileComplete {
                file_index,
                hashes,
                info,
            } => {
                if let Some(entry) = self.as_mut().rust_mut().entries.get_mut(file_index) {
                    entry.hashes = hashes;
                    entry.info = info;
                    entry.error = None;
                }
                self.as_mut().rust_mut().files_completed += 1;

                let roles = QVector::<i32>::default();
                let col_count = (3 + self.rust().visible_kinds.len()) as i32;
                let row_tl = self.index(file_index as i32, 0, &QModelIndex::default());
                let row_br = self.index(file_index as i32, col_count - 1, &QModelIndex::default());
                self.as_mut().data_changed(&row_tl, &row_br, &roles);
            }
            WorkerMessage::FileError { file_index, error } => {
                if let Some(entry) = self.as_mut().rust_mut().entries.get_mut(file_index) {
                    entry.error = Some(error);
                    entry.hashes.clear();
                    entry.info = String::new();
                }
                self.as_mut().rust_mut().files_completed += 1;

                let roles = QVector::<i32>::default();
                let col_count = (3 + self.rust().visible_kinds.len()) as i32;
                let row_tl = self.index(file_index as i32, 0, &QModelIndex::default());
                let row_br = self.index(file_index as i32, col_count - 1, &QModelIndex::default());
                self.as_mut().data_changed(&row_tl, &row_br, &roles);
            }
            WorkerMessage::AllComplete => {
                let elapsed = self
                    .rust()
                    .start_time
                    .map(|t| t.elapsed().as_secs_f32())
                    .unwrap_or(0.0);
                let total = self.rust().total_files;
                let text = format!("Done! {} file(s) hashed in {:.1}s", total, elapsed);
                self.as_mut().set_status_text(QString::from(&text));
                self.as_mut().set_is_hashing(false);
                self.as_mut().set_file_progress(1.0);
                self.as_mut().set_global_progress(1.0);
                self.as_mut().rust_mut().cancel_flag = None;
            }
        }
    }

    fn cancel_hashing(mut self: Pin<&mut Self>) {
        if let Some(ref flag) = self.rust().cancel_flag {
            flag.store(true, Ordering::SeqCst);
        }
        self.as_mut().rust_mut().cancel_flag = None;
        self.as_mut().set_is_hashing(false);
        self.as_mut()
            .set_status_text(QString::from("Cancelled"));
    }

    fn clear_list(mut self: Pin<&mut Self>) {
        if self.rust().is_hashing {
            return;
        }
        unsafe {
            self.as_mut().begin_reset_model();
        }
        self.as_mut().rust_mut().entries.clear();
        unsafe {
            self.as_mut().end_reset_model();
        }
        self.as_mut().set_file_count(0);
        self.as_mut().set_selected_row(-1);
        self.as_mut()
            .set_status_text(QString::from("Ready"));
    }

    fn remove_selected(mut self: Pin<&mut Self>) {
        let row = self.rust().selected_row;
        if row < 0 {
            return;
        }
        let idx = row as usize;
        if idx >= self.rust().entries.len() {
            return;
        }
        let invalid = QModelIndex::default();
        unsafe {
            self.as_mut().begin_remove_rows(&invalid, row, row);
        }
        self.as_mut().rust_mut().entries.remove(idx);
        unsafe {
            self.as_mut().end_remove_rows();
        }
        let new_count = self.rust().entries.len() as i32;
        self.as_mut().set_file_count(new_count);
        self.as_mut().set_selected_row(-1);
    }

    fn select_row(mut self: Pin<&mut Self>, row: i32) {
        self.as_mut().set_selected_row(row);

        // Notify model that isSelected role changed for all rows
        let count = self.rust().entries.len();
        if count > 0 {
            let col_count = (3 + self.rust().visible_kinds.len()) as i32;
            let top = self.index(0, 0, &QModelIndex::default());
            let bottom = self.index(count as i32 - 1, col_count - 1, &QModelIndex::default());
            let mut roles = QVector::<i32>::default();
            roles.append(ROLE_IS_SELECTED);
            self.as_mut().data_changed(&top, &bottom, &roles);
        }
    }

    fn sort_by(mut self: Pin<&mut Self>, column: i32, ascending: bool) {
        if self.rust().is_hashing {
            return;
        }
        let visible_kinds = self.rust().visible_kinds.clone();
        let col = column as usize;
        unsafe { self.as_mut().begin_reset_model(); }
        self.as_mut().rust_mut().entries.sort_by(|a, b| {
            let a_val = sort_key(a, col, &visible_kinds);
            let b_val = sort_key(b, col, &visible_kinds);
            if ascending {
                a_val.cmp(&b_val)
            } else {
                b_val.cmp(&a_val)
            }
        });
        unsafe { self.as_mut().end_reset_model(); }
        self.as_mut().set_selected_row(-1);
    }

    fn copy_filepath(&self) {
        let row = self.rust().selected_row;
        if row < 0 {
            return;
        }
        let entries = &self.rust().entries;
        if let Some(entry) = entries.get(row as usize) {
            let path_str = entry.path.to_string_lossy().to_string();
            set_clipboard(&path_str);
        }
    }

    fn copy_hash(&self, algo: i32) {
        let row = self.rust().selected_row;
        if row < 0 {
            return;
        }
        let entries = &self.rust().entries;
        if let Some(entry) = entries.get(row as usize) {
            let hash = match algo {
                0 => entry.hash_value(HashKind::CRC32),
                1 => entry.hash_value(HashKind::MD5),
                2 => entry.hash_value(HashKind::SHA1),
                3 => entry.hash_value(HashKind::SHA256),
                4 => entry.hash_value(HashKind::SHA512),
                _ => return,
            };
            if !hash.is_empty() {
                    set_clipboard(hash);
                }
        }
    }

    fn open_folder(&self) {
        let row = self.rust().selected_row;
        if row < 0 {
            return;
        }
        let entries = &self.rust().entries;
        if let Some(entry) = entries.get(row as usize) {
            if let Some(parent) = entry.path.parent() {
                let _ = open::that(parent);
            }
        }
    }

    fn save_hash_file(&self, algo: i32, path: &QString) {
        let kind = match algo {
            0 => HashKind::CRC32,
            1 => HashKind::MD5,
            2 => HashKind::SHA1,
            3 => HashKind::SHA256,
            4 => HashKind::SHA512,
            _ => return,
        };
        let output_path = PathBuf::from(path.to_string());
        let _ = write_hash_file(&self.rust().entries, &output_path, kind);
    }

    fn apply_settings(mut self: Pin<&mut Self>) {
        unsafe { self.as_mut().begin_reset_model(); }
        {
            let mut r = self.as_mut().rust_mut();
            r.config.hash_crc32 = r.setting_crc32;
            r.config.hash_md5 = r.setting_md5;
            r.config.hash_sha1 = r.setting_sha1;
            r.config.hash_sha256 = r.setting_sha256;
            r.config.hash_sha512 = r.setting_sha512;
            let _ = r.config.save();
            r.visible_kinds = r.config.enabled_hash_kinds();
        }
        unsafe { self.as_mut().end_reset_model(); }
    }

    fn visible_columns(&self) -> QStringList {
        let mut list = QStringList::default();
        for kind in &self.rust().visible_kinds {
            list.append(QString::from(kind.name()));
        }
        list
    }

    fn apply_rename_settings(mut self: Pin<&mut Self>) {
        let pattern = self.rust().setting_rename_pattern.to_string();
        self.as_mut().rust_mut().config.rename_pattern = pattern;
        let _ = self.rust().config.save();
    }

    fn rename_files(mut self: Pin<&mut Self>) {
        if self.rust().is_hashing {
            return;
        }

        struct RenameOp {
            index: usize,
            old_path: PathBuf,
            new_path: PathBuf,
            new_filename: String,
        }

        let pattern = self.rust().config.rename_pattern.clone();

        let ops: Vec<RenameOp> = {
            let r = self.rust();
            (0..r.entries.len()).filter_map(|i| {
                let entry = &r.entries[i];
                // Only rename entries that have been hashed and have no error
                if entry.hashes.is_empty() || entry.error.is_some() {
                    return None;
                }
                let path = entry.path.clone();
                let raw_stem = path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                // Strip any existing [XXXXXXXX] CRC32 tags so rename is idempotent
                let stem = strip_crc32_tags(&raw_stem);
                let ext = path.extension()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                let new_name = pattern
                    .replace("%FILENAME%", &stem)
                    .replace("%FILEEXT%", &ext)
                    .replace("%CRC%", entry.hash_value(HashKind::CRC32))
                    .replace("%MD5%", entry.hash_value(HashKind::MD5))
                    .replace("%SHA1%", entry.hash_value(HashKind::SHA1))
                    .replace("%SHA256%", entry.hash_value(HashKind::SHA256))
                    .replace("%SHA512%", entry.hash_value(HashKind::SHA512));

                let parent = path.parent()?;
                let new_path = parent.join(&new_name);
                // Ensure the new path stays within the original directory.
                // Canonicalize to resolve ".." components that bypass starts_with.
                let canon_parent = parent.canonicalize().ok()?;
                let canon_new = new_path.canonicalize()
                    .or_else(|_| {
                        // File doesn't exist yet; canonicalize the parent and re-join the filename
                        new_path.parent()
                            .and_then(|p| p.canonicalize().ok())
                            .map(|p| p.join(new_path.file_name().unwrap_or_default()))
                            .ok_or(std::io::ErrorKind::NotFound)
                    })
                    .ok()?;
                if !canon_new.starts_with(&canon_parent) {
                    return None;
                }
                if new_path == path {
                    return None;
                }
                Some(RenameOp { index: i, old_path: path, new_path, new_filename: new_name })
            }).collect()
        };

        let mut renamed = 0usize;
        let mut error_count = 0usize;
        for op in ops {
            if op.new_path.exists() {
                error_count += 1;
                continue;
            }
            match std::fs::rename(&op.old_path, &op.new_path) {
                Ok(()) => {
                    if let Some(e) = self.as_mut().rust_mut().entries.get_mut(op.index) {
                        e.path = op.new_path;
                        e.filename = op.new_filename;
                        e.refresh_expected_crc32();
                    }
                    renamed += 1;
                }
                Err(_) => error_count += 1,
            }
        }

        let msg = if error_count > 0 {
            format!("{} file(s) renamed, {} failed", renamed, error_count)
        } else {
            format!("{} file(s) renamed", renamed)
        };
        self.as_mut().set_status_text(QString::from(&msg));

        let count = self.rust().entries.len();
        if count > 0 {
            let col_count = (3 + self.rust().visible_kinds.len()) as i32;
            let top = self.index(0, 0, &QModelIndex::default());
            let bottom = self.index(count as i32 - 1, col_count - 1, &QModelIndex::default());
            let roles = QVector::<i32>::default();
            self.as_mut().data_changed(&top, &bottom, &roles);
        }
    }

    fn get_rename_preview(&self) -> QString {
        let r = self.rust();
        let pattern = &r.config.rename_pattern;

        for entry in &r.entries {
            if entry.hashes.is_empty() || entry.error.is_some() {
                continue;
            }
            let path = &entry.path;
            let raw_stem = path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let stem = strip_crc32_tags(&raw_stem);
            let ext = path.extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let new_name = pattern
                .replace("%FILENAME%", &stem)
                .replace("%FILEEXT%", &ext)
                .replace("%CRC%", entry.hash_value(HashKind::CRC32))
                .replace("%MD5%", entry.hash_value(HashKind::MD5))
                .replace("%SHA1%", entry.hash_value(HashKind::SHA1))
                .replace("%SHA256%", entry.hash_value(HashKind::SHA256))
                .replace("%SHA512%", entry.hash_value(HashKind::SHA512));

            let old_name = path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            if new_name == old_name {
                continue;
            }

            return QString::from(&format!("{} → {}", old_name, new_name));
        }

        QString::from("")
    }
}

fn sort_key(entry: &FileEntry, column: usize, visible_kinds: &[HashKind]) -> String {
    if column == 0 {
        return entry.filename.to_lowercase();
    }
    let verify_col = 1 + visible_kinds.len();
    let info_col = 2 + visible_kinds.len();
    if column == verify_col {
        return match entry.verify_status() {
            1 => "1_match".to_string(),
            2 => "2_mismatch".to_string(),
            _ => "0_none".to_string(),
        };
    }
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

fn collect_files_recursive(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Remove all `[XXXXXXXX]` (8 hex digit) CRC32 tags from a filename stem.
fn strip_crc32_tags(stem: &str) -> String {
    let mut result = String::with_capacity(stem.len());
    let bytes = stem.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'['
            && i + 10 <= bytes.len()
            && bytes[i + 9] == b']'
            && bytes[i + 1..i + 9].iter().all(|b| b.is_ascii_hexdigit())
        {
            i += 10; // skip `[XXXXXXXX]`
        } else {
            // Advance past the entire UTF-8 character to avoid corrupting
            // multi-byte sequences (e.g. accented or CJK characters).
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] & 0xC0 == 0x80 {
                i += 1;
            }
            result.push_str(&stem[start..i]);
        }
    }
    result
}
