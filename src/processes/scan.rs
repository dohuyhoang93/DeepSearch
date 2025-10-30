use crate::db::{DbManager, FileMetadata};
use crate::pop::context::Context;
use crate::utils::normalize_string;
use crate::gui::events::GuiUpdate;
use rayon::prelude::*;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::SystemTime;
use walkdir::WalkDir;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BATCH_SIZE: usize = 50_000;

/// A helper function to send progress updates if a reporter is available.
fn report_progress(reporter: &Option<Sender<GuiUpdate>>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

// --- PROCESSES ---

/// Process: Scans the directory and streams file data through a channel for the initial scan.
pub fn scan_directory_streaming(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();
    let (tx, rx) = mpsc::channel();

    report_progress(&reporter, 0.0, &format!("🔍 Starting initial scan for '{}'...", root_path.display()));

    thread::spawn(move || {
        perform_initial_scan_and_send(root_path, reporter, tx);
    });

    context.file_data_stream = Some(rx);
    Ok(context)
}

/// Process: Scans filesystem, compares with DB, and applies updates/additions in batches.
pub fn find_and_apply_updates_streaming(context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let db_path = context.db_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();

    report_progress(&reporter, 0.0, "🔄 Rescan: Finding new and updated files...");

    let db_manager = Arc::new(DbManager::new(&db_path)?);
    let (tx, rx) = mpsc::channel::<(String, FileMetadata)>();

    // --- Writer Thread ---
    let writer_thread = {
        let writer_db_manager = Arc::clone(&db_manager);
        let writer_root_path = root_path.clone();
        let writer_reporter = reporter.clone();
        thread::spawn(move || -> anyhow::Result<usize> {
            let mut updates_batch = Vec::with_capacity(BATCH_SIZE);
            let mut processed_count = 0;
            for (path, metadata) in rx {
                updates_batch.push((path, metadata));
                processed_count += 1;
                if updates_batch.len() >= BATCH_SIZE {
                    writer_db_manager.update_index_for_path(writer_root_path.to_str().unwrap(), &updates_batch, &[])?;
                    report_progress(&writer_reporter, 0.25, &format!("Applied {} updates...", processed_count));
                    updates_batch.clear();
                }
            }
            if !updates_batch.is_empty() {
                writer_db_manager.update_index_for_path(writer_root_path.to_str().unwrap(), &updates_batch, &[])?;
            }
            Ok(processed_count)
        })
    };

    // --- Scanner Thread ---
    let scanner_thread = {
        let scanner_reporter = reporter.clone();
        let scanner_db_manager = Arc::clone(&db_manager);
        thread::spawn(move || {
            perform_rescan_and_send_updates(root_path, scanner_reporter, tx, scanner_db_manager);
        })
    };

    scanner_thread.join().unwrap();
    let count = writer_thread.join().unwrap()?;
    report_progress(&reporter, 0.5, &format!("Found and applied {} new/updated files.", count));

    Ok(context)
}

// --- HELPER FUNCTIONS ---

/// Core scanning logic for the INITIAL scan. Does not access the DB.
fn perform_initial_scan_and_send(
    root_path: PathBuf,
    reporter: Option<Sender<GuiUpdate>>,
    tx: Sender<(String, FileMetadata)>,
) {
    let (top_level_entries, subdirs) = discover_fs_structure(&root_path, &reporter);

    // Handle files at the top level
    top_level_entries
        .par_iter()
        .filter(|entry| entry.path().is_file())
        .for_each(|entry| {
            tx.send(build_file_data(entry, &root_path)).ok();
        });

    // Scan subdirectories in parallel
    scan_subdirs(subdirs, &reporter, |entry| {
        tx.send(build_file_data(entry, &root_path)).ok();
    });
}

/// Core scanning logic for the RESCAN. Accesses the DB safely.
fn perform_rescan_and_send_updates(
    root_path: PathBuf,
    reporter: Option<Sender<GuiUpdate>>,
    tx: Sender<(String, FileMetadata)>,
    db_manager: Arc<DbManager>,
) {
    let table_name = db_manager.get_table_name(root_path.to_str().unwrap()).unwrap().unwrap();
    let (top_level_entries, subdirs) = discover_fs_structure(&root_path, &reporter);

    let comparison_logic = |entry: &walkdir::DirEntry| {
        let (path_str, new_metadata) = build_file_data(entry, &root_path);
        if let Ok(Some(old_metadata)) = db_manager.get_file_metadata(&table_name, &path_str) {
            if old_metadata.modified_time != new_metadata.modified_time {
                tx.send((path_str, new_metadata)).ok(); // Updated file
            }
        } else {
            tx.send((path_str, new_metadata)).ok(); // New file
        }
    };

    // Handle files at the top level
    top_level_entries
        .par_iter()
        .filter(|entry| entry.path().is_file())
        .for_each(comparison_logic);

    // Scan subdirectories in parallel
    scan_subdirs(subdirs, &reporter, comparison_logic);
}

/// Helper for directory discovery (Phase 1 of the 2-phase scan).
fn discover_fs_structure(root_path: &Path, reporter: &Option<Sender<GuiUpdate>>) -> (Vec<walkdir::DirEntry>, Vec<PathBuf>) {
    report_progress(reporter, 0.01, "Phase 1/2: Discovering directories...");
    let top_level_entries: Vec<_> = WalkDir::new(root_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .collect();

    let subdirs: Vec<_> = top_level_entries
        .par_iter()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect();
    
    (top_level_entries, subdirs)
}

/// Helper for scanning subdirectories (Phase 2 of the 2-phase scan).
fn scan_subdirs<F: Fn(&walkdir::DirEntry) + Send + Sync>(
    subdirs: Vec<PathBuf>,
    reporter: &Option<Sender<GuiUpdate>>,
    action: F,
) {
    let num_subdirs = subdirs.len();
    let processed_subdirs = AtomicUsize::new(0);
    report_progress(reporter, 0.05, "Phase 2/2: Scanning files...");

    subdirs
        .par_iter()
        .for_each(|subdir| {
            WalkDir::new(subdir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .for_each(|entry| action(&entry));

            let processed_count = processed_subdirs.fetch_add(1, Ordering::SeqCst);
            if num_subdirs > 0 {
                let progress = 0.05 + (processed_count as f32 / num_subdirs as f32) * 0.40;
                report_progress(reporter, progress, &format!("Scanning in {}...", subdir.display()));
            }
        });
}

/// Helper: Builds FileMetadata from a DirEntry.
fn build_file_data(entry: &walkdir::DirEntry, root_path: &Path) -> (String, FileMetadata) {
    let relative_path = entry
        .path()
        .strip_prefix(root_path)
        .unwrap()
        .to_string_lossy()
        .to_string();

    let metadata = FileMetadata {
        normalized_name: normalize_string(&entry.file_name().to_string_lossy()),
        modified_time: entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    (relative_path, metadata)
}
