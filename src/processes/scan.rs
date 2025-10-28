use crate::db::FileMetadata;
use crate::pop::context::Context;
use crate::utils::normalize_string;
use crate::gui::events::GuiUpdate; // Import the GuiUpdate enum
use rayon::prelude::*;
use std::sync::mpsc::Sender;
use std::time::SystemTime;
use walkdir::WalkDir;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A helper function to send progress updates if a reporter is available.
fn report_progress(reporter: &Option<Sender<GuiUpdate>>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

/// Process: Scans the entire directory for the first time.
pub fn scan_directory_initial(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context
        .target_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Target path not set in context"))?;

    let reporter = context.progress_reporter.clone();
    report_progress(&reporter, 0.0, &format!("🔍 Starting initial scan for '{}'...", root_path.display()));

    let (files, count) = perform_scan(root_path, &reporter)?;

    report_progress(&reporter, 1.0, &format!("✅ Scan complete. Found {} files.", count));

    context.files_found_count = count;
    context.files_to_index = files;

    Ok(context)
}

/// Process: Performs an incremental scan, comparing against the existing index.
pub fn scan_directory_incremental(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context
        .target_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Target path not set in context"))?;

    let reporter = context.progress_reporter.clone();
    report_progress(&reporter, 0.0, &format!("🔄 Starting incremental scan for '{}'...", root_path.display()));

    let (mut found_files, _count) = perform_scan(root_path, &reporter)?;

    report_progress(&reporter, 0.9, "Comparing file lists...");

    let mut files_to_update = vec![];
    let mut loaded_index = context.loaded_index.clone(); // Clone to be able to remove items

    for (path, metadata) in found_files.drain(..) {
        if let Some(existing_meta) = loaded_index.get(&path) {
            if existing_meta.modified_time != metadata.modified_time {
                files_to_update.push((path.clone(), metadata));
            }
            loaded_index.remove(&path);
        } else {
            files_to_update.push((path, metadata));
        }
    }

    let files_to_delete = loaded_index.keys().cloned().collect::<Vec<_>>();

    report_progress(&reporter, 1.0, &format!(
        "✅ Rescan complete. Found {} updates/additions and {} deletions.",
        files_to_update.len(),
        files_to_delete.len()
    ));

    context.files_to_update = files_to_update;
    context.files_to_delete = files_to_delete;

    Ok(context)
}

/// The core scanning logic, performs a 2-phase scan.
fn perform_scan(
    root_path: &std::path::Path,
    reporter: &Option<Sender<GuiUpdate>>,
) -> anyhow::Result<(Vec<(String, FileMetadata)>, usize)> {
    // Phase 1: Discover directories
    report_progress(reporter, 0.05, "Phase 1/2: Discovering directories...");
    let top_level_entries: Vec<_> = WalkDir::new(root_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .collect();

    let mut files: Vec<(String, FileMetadata)> = top_level_entries
        .par_iter()
        .filter(|entry| entry.path().is_file())
        .map(|entry| build_file_data(entry, root_path))
        .collect();

    // Phase 2: Scan subdirectories
    let subdirs: Vec<_> = top_level_entries
        .par_iter()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect();

    let num_subdirs = subdirs.len();
    let processed_subdirs = AtomicUsize::new(0);

    report_progress(reporter, 0.1, "Phase 2/2: Scanning files...");

    let nested_files: Vec<Vec<(String, FileMetadata)>> = subdirs
        .par_iter()
        .map(|subdir| {
            let collected: Vec<_> = WalkDir::new(subdir)
                .into_iter()
                .par_bridge()
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_file())
                .map(|entry| build_file_data(&entry, root_path))
                .collect();

            let processed_count = processed_subdirs.fetch_add(1, Ordering::SeqCst);
            if num_subdirs > 0 {
                // Report progress based on the number of directories processed.
                // We scale this phase from 10% to 90% of the total progress.
                let progress = 0.1 + (processed_count as f32 / num_subdirs as f32) * 0.8;
                report_progress(reporter, progress, &format!("Scanning in {}...", subdir.display()));
            }
            collected
        })
        .collect();

    // Combine results
    for mut vec in nested_files {
        files.append(&mut vec);
    }

    let count = files.len();
    Ok((files, count))
}

/// Helper: Builds FileMetadata from a DirEntry.
fn build_file_data(entry: &walkdir::DirEntry, root_path: &std::path::Path) -> (String, FileMetadata) {
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
