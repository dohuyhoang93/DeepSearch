use crate::db::FileMetadata;
use crate::pop::context::Context;
use crate::utils::normalize_string;
use rayon::prelude::*;
use std::time::SystemTime;
use walkdir::WalkDir;

/// Process: Quét toàn bộ thư mục lần đầu, giữ lại chiến lược quét 2 giai đoạn.
pub fn scan_directory_initial(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context
        .target_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Target path not set in context"))?;

    println!("🔍 Starting initial scan for '{}'...", root_path.display());

    let (files, count) = perform_scan(&root_path)?;
    context.files_found_count = count;
    context.files_to_index = files;

    println!("✅ Scan complete. Found {} files.", context.files_found_count);

    Ok(context)
}

/// Process: Quét cập nhật, so sánh với index đã có.
pub fn scan_directory_incremental(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context
        .target_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Target path not set in context"))?;

    println!("🔄 Starting incremental scan for '{}'...", root_path.display());

    let (mut found_files, _count) = perform_scan(&root_path)?;

    let mut files_to_update = vec![];
    let mut loaded_index = context.loaded_index.clone(); // Clone để có thể xóa item

    for (path, metadata) in found_files.drain(..) {
        if let Some(existing_meta) = loaded_index.get(&path) {
            // File tồn tại trong index, kiểm tra modified_time
            if existing_meta.modified_time != metadata.modified_time {
                files_to_update.push((path.clone(), metadata));
            }
            // Xóa khỏi loaded_index để theo dõi các file đã bị xóa
            loaded_index.remove(&path);
        } else {
            // File không có trong index -> file mới
            files_to_update.push((path, metadata));
        }
    }

    // Những file còn lại trong loaded_index là những file đã bị xóa
    let files_to_delete = loaded_index.keys().cloned().collect::<Vec<_>>();

    println!(
        "✅ Rescan complete. Found {} updates/additions and {} deletions.",
        files_to_update.len(),
        files_to_delete.len()
    );

    context.files_to_update = files_to_update;
    context.files_to_delete = files_to_delete;

    Ok(context)
}

/// Lõi quét, thực hiện logic quét 2 giai đoạn, trả về danh sách file và metadata.
fn perform_scan(root_path: &std::path::Path) -> anyhow::Result<(Vec<(String, FileMetadata)>, usize)> {
    // Giai đoạn 1: Quét file/thư mục trong thư mục cấp 1
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

    // Giai đoạn 2: Quét các thư mục cấp 2 trở đi
    let subdirs: Vec<_> = top_level_entries
        .par_iter()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect();

    let nested_files: Vec<Vec<(String, FileMetadata)>> = subdirs
        .par_iter()
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

    // Gộp kết quả
    for mut vec in nested_files {
        files.append(&mut vec);
    }

    let count = files.len();
    Ok((files, count))
}

/// Helper: Xây dựng FileMetadata từ một DirEntry.
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
