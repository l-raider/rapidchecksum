mod config;
mod fileio;
mod hasher;
mod model;
mod worker;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use slint::{ComponentHandle, ModelRc, TableColumn, VecModel};

use hasher::HashKind;
use config::AppConfig;
use model::FileListModel;
use worker::{FileTask, WorkerMessage};

// Pull in the generated Slint code
slint::include_modules!();

// Rename the sha1 crate to avoid conflict with our hasher::sha1 module
extern crate sha1 as sha1_crate;

fn main() {
    let app = MainWindow::new().unwrap();
    let settings_dlg = SettingsDialog::new().unwrap();

    let file_list = Rc::new(RefCell::new(FileListModel::new()));
    let cancel_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let config = Rc::new(RefCell::new(AppConfig::load()));

    // Sync loaded settings to the UI and set initial visible columns
    {
        let cfg = config.borrow();
        app.set_setting_crc32(cfg.hash_crc32);
        app.set_setting_md5(cfg.hash_md5);
        app.set_setting_sha1(cfg.hash_sha1);
        app.set_setting_sha256(cfg.hash_sha256);
        app.set_setting_sha512(cfg.hash_sha512);
        settings_dlg.set_setting_crc32(cfg.hash_crc32);
        settings_dlg.set_setting_md5(cfg.hash_md5);
        settings_dlg.set_setting_sha1(cfg.hash_sha1);
        settings_dlg.set_setting_sha256(cfg.hash_sha256);
        settings_dlg.set_setting_sha512(cfg.hash_sha512);
        let kinds = cfg.enabled_hash_kinds();
        app.set_table_columns(build_table_columns(&kinds));
        file_list.borrow_mut().set_visible_kinds(kinds);
    }

    // Keeps the polling timer alive for the duration of a hashing run.
    // Slint timers stop when dropped, so we must hold this somewhere that
    // outlives the callback closure that creates the timer.
    let timer_holder: Rc<RefCell<Option<slint::Timer>>> = Rc::new(RefCell::new(None));

    // Bind the table model to the UI
    app.set_file_rows(file_list.borrow().model_rc());

    setup_open_files(&app, &file_list);
    setup_start_hashing(&app, &file_list, &cancel_flag, &timer_holder, &config);
    setup_row_selection(&app, &file_list);
    setup_cancel(&app, &cancel_flag, &timer_holder);
    setup_clear_list(&app, &file_list);
    setup_remove_selected(&app, &file_list);
    setup_create_hash_files(&app, &file_list);
    setup_exit(&app);
    setup_sort(&app, &file_list);
    let clipboard = Rc::new(RefCell::new(arboard::Clipboard::new().ok()));
    setup_context_menu(&app, &file_list, &clipboard);
    setup_settings(&app, &settings_dlg, &config, &file_list);

    app.run().unwrap();
}

/// "Open Files" button: open a file dialog and add selected files to the list.
/// Does NOT start hashing — use the "Start Hashing" button for that.
fn setup_open_files(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>) {
    let weak = app.as_weak();
    let file_list = file_list.clone();

    app.on_open_files(move || {
        let dialog = rfd::FileDialog::new()
            .set_title("Select files to hash")
            .pick_files();

        let paths = match dialog {
            Some(p) if !p.is_empty() => p,
            _ => return,
        };

        {
            let mut list = file_list.borrow_mut();
            for path in &paths {
                list.add_file(path.clone());
            }
        }

        if let Some(app) = weak.upgrade() {
            app.set_file_rows(file_list.borrow().model_rc());
            app.set_file_count(file_list.borrow().len() as i32);
            app.set_status_text(slint::format!(
                "{} file(s) ready — press Start Hashing",
                file_list.borrow().len()
            ));
        }
    });
}

/// "Start Hashing" button: hash all files currently in the list.
fn setup_start_hashing(
    app: &MainWindow,
    file_list: &Rc<RefCell<FileListModel>>,
    cancel_flag: &Arc<AtomicBool>,
    timer_holder: &Rc<RefCell<Option<slint::Timer>>>,
    config: &Rc<RefCell<AppConfig>>,
) {
    let weak = app.as_weak();
    let file_list = file_list.clone();
    let cancel_flag = cancel_flag.clone();
    let timer_holder = timer_holder.clone();
    let config = config.clone();

    app.on_start_hashing(move || {
        let list = file_list.borrow();
        if list.entries.is_empty() {
            return;
        }

        let tasks: Vec<FileTask> = list
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| FileTask {
                index: i,
                path: entry.path.clone(),
            })
            .collect();
        let total_files = tasks.len();
        drop(list);

        cancel_flag.store(false, Ordering::Relaxed);

        if let Some(app) = weak.upgrade() {
            app.set_is_hashing(true);
            app.set_global_progress(0.0);
            app.set_file_progress(0.0);
            app.set_status_text(slint::format!("Hashing {} file(s)...", total_files));
        }

        let (tx, rx) = mpsc::channel::<WorkerMessage>();
        let kinds = config.borrow().enabled_hash_kinds();
        worker::spawn_hash_worker(tasks, kinds, tx, cancel_flag.clone());

        let start_time = std::time::Instant::now();

        // Store the timer in the holder so it outlives this closure.
        // Without this the timer drops immediately and polling never fires.
        let weak_timer = weak.clone();
        let file_list_timer = file_list.clone();
        let timer_holder_inner = timer_holder.clone();
        let mut files_completed: usize = 0;

        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(16),
            move || {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        WorkerMessage::FileProgress { bytes_read, total_bytes, .. } => {
                            if let Some(app) = weak_timer.upgrade() {
                                let pct = if total_bytes > 0 {
                                    bytes_read as f32 / total_bytes as f32
                                } else {
                                    0.0
                                };
                                app.set_file_progress(pct);
                            }
                        }
                        WorkerMessage::FileComplete { file_index, hashes, info } => {
                            file_list_timer.borrow_mut().update_hashes(file_index, hashes, info);
                            files_completed += 1;
                            if let Some(app) = weak_timer.upgrade() {
                                app.set_global_progress(
                                    files_completed as f32 / total_files as f32,
                                );
                                app.set_status_text(slint::format!(
                                    "{}/{} files completed",
                                    files_completed,
                                    total_files
                                ));
                            }
                        }
                        WorkerMessage::FileError { file_index, error } => {
                            file_list_timer.borrow_mut().set_error(file_index, error);
                            files_completed += 1;
                            if let Some(app) = weak_timer.upgrade() {
                                app.set_global_progress(
                                    files_completed as f32 / total_files as f32,
                                );
                            }
                        }
                        WorkerMessage::AllComplete => {
                            if let Some(app) = weak_timer.upgrade() {
                                let elapsed = start_time.elapsed();
                                let time_str = format_duration(elapsed);
                                app.set_is_hashing(false);
                                app.set_file_progress(0.0);
                                app.set_status_text(slint::format!(
                                    "Done \u{2014} {} file(s) processed in {}",
                                    files_completed,
                                    time_str.as_str()
                                ));
                            }
                            // Drop the timer — polling stops
                            *timer_holder_inner.borrow_mut() = None;
                        }
                    }
                }
            },
        );
        *timer_holder.borrow_mut() = Some(timer);
    });
}

/// When a row is selected in the table, show its details in the Results group.
fn setup_row_selection(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>) {
    let weak = app.as_weak();
    let file_list = file_list.clone();

    app.on_row_selected(move |row_index| {
        let list = file_list.borrow();
        let idx = row_index as usize;

        if let Some(entry) = list.entries.get(idx) {
            if let Some(app) = weak.upgrade() {
                app.set_result_filename(slint::SharedString::from(&entry.filename));
                app.set_result_crc32(slint::SharedString::from(
                    entry.hash_value(HashKind::CRC32),
                ));
                app.set_result_sha1(slint::SharedString::from(
                    entry.hash_value(HashKind::SHA1),
                ));
                app.set_result_sha256(slint::SharedString::from(
                    entry.hash_value(HashKind::SHA256),
                ));
                app.set_result_sha512(slint::SharedString::from(
                    entry.hash_value(HashKind::SHA512),
                ));
                app.set_result_info(slint::SharedString::from(&entry.info));
            }
        }
    });
}

fn setup_cancel(
    app: &MainWindow,
    cancel_flag: &Arc<AtomicBool>,
    timer_holder: &Rc<RefCell<Option<slint::Timer>>>,
) {
    let cancel = cancel_flag.clone();
    let weak = app.as_weak();
    let timer_holder = timer_holder.clone();
    app.on_cancel_hashing(move || {
        cancel.store(true, Ordering::Relaxed);
        // Stop polling
        *timer_holder.borrow_mut() = None;
        if let Some(app) = weak.upgrade() {
            app.set_is_hashing(false);
            app.set_status_text(slint::SharedString::from("Cancelled"));
        }
    });
}

fn setup_clear_list(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>) {
    let file_list = file_list.clone();
    let weak = app.as_weak();
    app.on_clear_list(move || {
        file_list.borrow_mut().clear();
        if let Some(app) = weak.upgrade() {
            clear_results(&app);
            app.set_file_count(0);
            app.set_status_text(slint::SharedString::from("Ready"));
        }
    });
}

fn setup_remove_selected(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>) {
    let file_list = file_list.clone();
    let weak = app.as_weak();
    app.on_remove_selected(move || {
        if let Some(app) = weak.upgrade() {
            let idx = app.get_selected_row() as usize;
            let mut list = file_list.borrow_mut();
            if idx < list.len() {
                list.remove(idx);
                clear_results(&app);
                app.set_file_count(list.len() as i32);
            }
        }
    });
}

fn setup_create_hash_files(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>) {
    // One closure per hash kind for the four "Create" buttons
    for &kind in &[HashKind::CRC32, HashKind::SHA1, HashKind::SHA256, HashKind::SHA512] {
        let file_list = file_list.clone();
        let weak = app.as_weak();

        let callback = move || {
            let list = file_list.borrow();
            if list.entries.is_empty() {
                return;
            }

            let ext = kind.file_extension();
            let dialog = rfd::FileDialog::new()
                .set_title(&format!("Save {} file", kind.name()))
                .set_file_name(&format!("checksums.{}", ext))
                .save_file();

            if let Some(path) = dialog {
                match fileio::write_hash_file(&list.entries, &path, kind) {
                    Ok(()) => {
                        if let Some(app) = weak.upgrade() {
                            app.set_status_text(slint::format!(
                                "Saved {} file: {}",
                                kind.name(),
                                path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        if let Some(app) = weak.upgrade() {
                            app.set_status_text(slint::format!(
                                "Error saving file: {}",
                                e
                            ));
                        }
                    }
                }
            }
        };

        match kind {
            HashKind::CRC32 => app.on_create_sfv_file(callback),
            HashKind::MD5 => app.on_create_md5_file(callback),
            HashKind::SHA1 => app.on_create_sha1_file(callback),
            HashKind::SHA256 => app.on_create_sha256_file(callback),
            HashKind::SHA512 => app.on_create_sha512_file(callback),
        }
    }
}

fn setup_exit(app: &MainWindow) {
    let weak = app.as_weak();
    app.on_exit_app(move || {
        if let Some(app) = weak.upgrade() {
            let _ = app.hide();
        }
    });
}

fn setup_sort(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>) {
    {
        let file_list = file_list.clone();
        app.on_sort_ascending(move |col| {
            file_list.borrow_mut().sort(col as usize, true);
        });
    }
    {
        let file_list = file_list.clone();
        app.on_sort_descending(move |col| {
            file_list.borrow_mut().sort(col as usize, false);
        });
    }
}

fn setup_context_menu(app: &MainWindow, file_list: &Rc<RefCell<FileListModel>>, clipboard: &Rc<RefCell<Option<arboard::Clipboard>>>) {
    // Copy Filepath
    {
        let weak = app.as_weak();
        let file_list = file_list.clone();
        let clipboard = clipboard.clone();
        app.on_copy_filepath(move || {
            if let Some(app) = weak.upgrade() {
                let idx = app.get_selected_row() as usize;
                let list = file_list.borrow();
                if let Some(entry) = list.entries.get(idx) {
                    let path_str = entry.path.display().to_string();
                    if let Some(cb) = clipboard.borrow_mut().as_mut() {
                        let _ = cb.set_text(&path_str);
                    }
                }
            }
        });
    }
    // Copy Hash
    {
        let weak = app.as_weak();
        let file_list = file_list.clone();
        let clipboard = clipboard.clone();
        app.on_copy_hash(move || {
            if let Some(app) = weak.upgrade() {
                let idx = app.get_selected_row() as usize;
                let list = file_list.borrow();
                if let Some(entry) = list.entries.get(idx) {
                    let mut lines = Vec::new();
                    for &kind in &[HashKind::CRC32, HashKind::SHA1, HashKind::SHA256, HashKind::SHA512] {
                        let val = entry.hash_value(kind);
                        if !val.is_empty() {
                            lines.push(format!("{}: {}", kind.name(), val));
                        }
                    }
                    let text = lines.join("\n");
                    if !text.is_empty() {
                        if let Some(cb) = clipboard.borrow_mut().as_mut() {
                            let _ = cb.set_text(&text);
                        }
                    }
                }
            }
        });
    }
    // Open Containing Folder
    {
        let weak = app.as_weak();
        let file_list = file_list.clone();
        app.on_open_containing_folder(move || {
            if let Some(app) = weak.upgrade() {
                let idx = app.get_selected_row() as usize;
                let list = file_list.borrow();
                if let Some(entry) = list.entries.get(idx) {
                    if let Some(parent) = entry.path.parent() {
                        let _ = open::that_detached(parent);
                    }
                }
            }
        });
    }
}

fn setup_settings(
    app: &MainWindow,
    dlg: &SettingsDialog,
    config: &Rc<RefCell<AppConfig>>,
    file_list: &Rc<RefCell<FileListModel>>,
) {
    // Settings button: sync current values into dialog then show it
    {
        let dlg_weak = dlg.as_weak();
        let config = config.clone();
        app.on_open_settings(move || {
            if let Some(dlg) = dlg_weak.upgrade() {
                let cfg = config.borrow();
                dlg.set_setting_crc32(cfg.hash_crc32);
                dlg.set_setting_md5(cfg.hash_md5);
                dlg.set_setting_sha1(cfg.hash_sha1);
                dlg.set_setting_sha256(cfg.hash_sha256);
                dlg.set_setting_sha512(cfg.hash_sha512);
                dlg.window().set_size(slint::LogicalSize::new(220.0, 210.0));
                dlg.show().ok();
            }
        });
    }

    // OK: save config, update columns, close dialog
    {
        let dlg_weak = dlg.as_weak();
        let app_weak = app.as_weak();
        let config = config.clone();
        let file_list = file_list.clone();
        dlg.on_accepted(move |crc32, md5, sha1, sha256, sha512| {
            let mut cfg = config.borrow_mut();
            cfg.hash_crc32 = crc32;
            cfg.hash_md5 = md5;
            cfg.hash_sha1 = sha1;
            cfg.hash_sha256 = sha256;
            cfg.hash_sha512 = sha512;
            cfg.save();

            let kinds = cfg.enabled_hash_kinds();
            if let Some(app) = app_weak.upgrade() {
                app.set_setting_crc32(crc32);
                app.set_setting_md5(md5);
                app.set_setting_sha1(sha1);
                app.set_setting_sha256(sha256);
                app.set_setting_sha512(sha512);
                app.set_table_columns(build_table_columns(&kinds));
            }
            file_list.borrow_mut().set_visible_kinds(kinds);

            if let Some(dlg) = dlg_weak.upgrade() {
                dlg.hide().ok();
            }
        });
    }

    // Cancel: just close
    {
        let dlg_weak = dlg.as_weak();
        dlg.on_cancelled(move || {
            if let Some(dlg) = dlg_weak.upgrade() {
                dlg.hide().ok();
            }
        });
    }
}

/// Build the Slint TableColumn array for the current set of enabled hash kinds.
fn make_table_column(title: &str, min_width: f32, width: f32) -> TableColumn {
    let mut col = TableColumn::default();
    col.title = title.into();
    col.min_width = min_width;
    col.width = width;
    col
}

fn build_table_columns(kinds: &[HashKind]) -> ModelRc<TableColumn> {
    let mut cols = Vec::with_capacity(2 + kinds.len());
    cols.push(make_table_column("File", 150.0, 200.0));
    for &kind in kinds {
        let (min_w, w) = match kind {
            HashKind::CRC32 => (80.0, 90.0),
            HashKind::MD5 => (80.0, 260.0),
            HashKind::SHA1 => (100.0, 340.0),
            HashKind::SHA256 => (100.0, 520.0),
            HashKind::SHA512 => (100.0, 520.0),
        };
        cols.push(make_table_column(kind.name(), min_w, w));
    }
    cols.push(make_table_column("Info", 100.0, 180.0));
    ModelRc::new(VecModel::from(cols))
}

fn clear_results(app: &MainWindow) {
    app.set_result_filename(slint::SharedString::default());
    app.set_result_crc32(slint::SharedString::default());
    app.set_result_sha1(slint::SharedString::default());
    app.set_result_sha256(slint::SharedString::default());
    app.set_result_sha512(slint::SharedString::default());
    app.set_result_info(slint::SharedString::default());
}

fn format_duration(d: std::time::Duration) -> String {
    let total_secs = d.as_secs();
    let millis = d.subsec_millis();
    if total_secs >= 3600 {
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        format!("{}h {:02}m {:02}s", h, m, s)
    } else if total_secs >= 60 {
        let m = total_secs / 60;
        let s = total_secs % 60;
        format!("{}m {:02}s", m, s)
    } else if total_secs >= 10 {
        format!("{}.{:01}s", total_secs, millis / 100)
    } else {
        format!("{}.{:03}s", total_secs, millis)
    }
}
