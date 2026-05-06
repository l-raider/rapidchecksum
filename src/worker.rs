use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Instant;

use crate::hasher::{self, HashKind};

const READ_BUFFER_SIZE: usize = 64 * 1024; // 64 KB chunks

/// Message sent from worker thread back to the UI thread.
#[derive(Clone)]
pub enum WorkerMessage {
    /// Progress update for a single file: (file_index, bytes_read_so_far, total_bytes)
    FileProgress {
        file_index: usize,
        bytes_read: u64,
        total_bytes: u64,
    },
    /// A single file has been fully hashed
    FileComplete {
        file_index: usize,
        hashes: HashMap<HashKind, String>,
        info: String,
    },
    /// An error occurred processing a file
    FileError {
        file_index: usize,
        error: String,
    },
    /// All files have been processed
    AllComplete,
}

/// Describes one file to be hashed.
pub struct FileTask {
    pub index: usize,
    pub path: PathBuf,
}

/// Spawn a worker thread that hashes all given files with all enabled algorithms.
/// Sends progress and results back via the channel. Respects the cancel flag.
pub fn spawn_hash_worker(
    files: Vec<FileTask>,
    kinds: Vec<HashKind>,
    tx: Sender<WorkerMessage>,
    cancel: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for file_task in &files {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            hash_single_file(file_task, &kinds, &tx, &cancel);
        }
        let _ = tx.send(WorkerMessage::AllComplete);
    })
}

fn hash_single_file(
    task: &FileTask,
    kinds: &[HashKind],
    tx: &Sender<WorkerMessage>,
    cancel: &AtomicBool,
) {
    let file = match File::open(&task.path) {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(WorkerMessage::FileError {
                file_index: task.index,
                error: e.to_string(),
            });
            return;
        }
    };

    let total_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);

    // Create one hasher per algorithm so we can feed the same data to all
    let mut hashers: Vec<(HashKind, Box<dyn hasher::HashAlgorithm>)> = kinds
        .iter()
        .map(|&k| (k, hasher::create_hasher(k)))
        .collect();

    let mut reader = file;
    let mut buf = vec![0u8; READ_BUFFER_SIZE];
    let mut bytes_read_total: u64 = 0;
    let start = Instant::now();

    loop {
        if cancel.load(Ordering::Relaxed) {
            return;
        }

        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = tx.send(WorkerMessage::FileError {
                    file_index: task.index,
                    error: e.to_string(),
                });
                return;
            }
        };

        let chunk = &buf[..n];
        for (_, hasher) in &mut hashers {
            hasher.update(chunk);
        }

        bytes_read_total += n as u64;

        let _ = tx.send(WorkerMessage::FileProgress {
            file_index: task.index,
            bytes_read: bytes_read_total,
            total_bytes,
        });
    }

    let elapsed = start.elapsed();
    let speed = if elapsed.as_secs_f64() > 0.0 {
        bytes_read_total as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0)
    } else {
        0.0
    };

    let info = format!(
        "{} read in {:.2}s => {:.1} MB/s",
        format_size(bytes_read_total),
        elapsed.as_secs_f64(),
        speed,
    );

    let hashes: HashMap<HashKind, String> = hashers
        .into_iter()
        .map(|(kind, hasher)| (kind, hasher.finalize()))
        .collect();

    let _ = tx.send(WorkerMessage::FileComplete {
        file_index: task.index,
        hashes,
        info,
    });
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
