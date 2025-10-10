use crate::db::FileMetadata;
use crate::pop::context::Context;
use crate::utils::normalize_string;
use crate::display;
use indicatif::{ParallelProgressIterator, ProgressBar};
use rayon::prelude::*;
use std::time::SystemTime;
use walkdir::WalkDir;

/// Process: Scans the entire directory for the first time, retaining the 2-phase scan strategy.
pub fn scan_directory_initial(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context
        .target_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Target path not set in context"))?;

    let pb = display::new_spinner(&format!("🔍 Starting initial scan for '{}'...", root_path.display()));
    let (files, count) = perform_scan(&root_path, &pb)?;
    pb.finish_with_message(format!("✅ Scan complete. Found {} files.", count));

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

    let pb = display::new_spinner(&format!("🔄 Starting incremental scan for '{}'...", root_path.display()));
    let (mut found_files, _count) = perform_scan(&root_path, &pb)?;
    pb.set_message("Comparing file lists...");

    let mut files_to_update = vec![];
    let mut loaded_index = context.loaded_index.clone(); // Clone to be able to remove items

    for (path, metadata) in found_files.drain(..) {
        if let Some(existing_meta) = loaded_index.get(&path) {
            // File exists in the index, check modified_time
            if existing_meta.modified_time != metadata.modified_time {
                files_to_update.push((path.clone(), metadata));
            }
            // Remove from loaded_index to track deleted files
            loaded_index.remove(&path);
        } else {
            // File is not in the index -> it's a new file
            files_to_update.push((path, metadata));
        }
    }

    // Any files remaining in loaded_index have been deleted
    let files_to_delete = loaded_index.keys().cloned().collect::<Vec<_>>();

    pb.finish_with_message(format!(
        "✅ Rescan complete. Found {} updates/additions and {} deletions.",
        files_to_update.len(),
        files_to_delete.len()
    ));

    context.files_to_update = files_to_update;
    context.files_to_delete = files_to_delete;

    Ok(context)
}

/// The core scanning logic, performs a 2-phase scan, returns a list of files and metadata.
fn perform_scan(
    root_path: &std::path::Path,
    pb: &ProgressBar,
) -> anyhow::Result<(Vec<(String, FileMetadata)>, usize)> {
    // Phase 1: Scan files/directories in the top-level directory
    pb.set_message("Phase 1/2: Discovering directories...");
    let top_level_entries: Vec<_> = WalkDir::new(&root_path)
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

    // Phase 2: Scan subdirectories from level 2 onwards
    let subdirs: Vec<_> = top_level_entries
        .par_iter()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect();

    let num_subdirs = subdirs.len() as u64;
    pb.set_style(display::get_common_progress_style());
    pb.set_length(num_subdirs);
    pb.set_position(0);
    pb.set_message("Phase 2/2: Scanning files...");

    let nested_files: Vec<Vec<(String, FileMetadata)>> = subdirs
        .par_iter()
        .progress_with(pb.clone())
        .map(|subdir| {
            WalkDir::new(subdir)
                .into_iter()
                .par_bridge()
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_file())
                .map(|entry| build_file_data(&entry, root_path))
                .collect::<Vec<_>>()
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
