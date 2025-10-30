use once_cell::sync::Lazy;
use std::collections::HashMap;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;
use crate::gui::events::GuiUpdate;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;
use walkdir::WalkDir;
use crate::db::FileMetadata;
use std::time::SystemTime;

// --- String Normalization Helpers ---

static VIETNAMESE_CHAR_MAP: Lazy<HashMap<char, char>> = Lazy::new(|| {
    let mut map = HashMap::new();
    let pairs = [
        ('à', 'a'), ('á', 'a'), ('ạ', 'a'), ('ả', 'a'), ('ã', 'a'),
        ('â', 'a'), ('ầ', 'a'), ('ấ', 'a'), ('ậ', 'a'), ('ẩ', 'a'), ('ẫ', 'a'),
        ('ă', 'a'), ('ằ', 'a'), ('ắ', 'a'), ('ặ', 'a'), ('ẳ', 'a'), ('ẵ', 'a'),
        ('è', 'e'), ('é', 'e'), ('ẹ', 'e'), ('ẻ', 'e'), ('ẽ', 'e'),
        ('ê', 'e'), ('ề', 'e'), ('ế', 'e'), ('ệ', 'e'), ('ể', 'e'), ('ễ', 'e'),
        ('ì', 'i'), ('í', 'i'), ('ị', 'i'), ('ỉ', 'i'), ('ĩ', 'i'),
        ('ò', 'o'), ('ó', 'o'), ('ọ', 'o'), ('ỏ', 'o'), ('õ', 'o'),
        ('ô', 'o'), ('ồ', 'o'), ('ố', 'o'), ('ộ', 'o'), ('ổ', 'o'), ('ỗ', 'o'),
        ('ơ', 'o'), ('ờ', 'o'), ('ớ', 'o'), ('ợ', 'o'), ('ở', 'o'), ('ỡ', 'o'),
        ('ù', 'u'), ('ú', 'u'), ('ụ', 'u'), ('ủ', 'u'), ('ũ', 'u'),
        ('ư', 'u'), ('ừ', 'u'), ('ứ', 'u'), ('ự', 'u'), ('ử', 'u'), ('ữ', 'u'),
        ('ỳ', 'y'), ('ý', 'y'), ('ỵ', 'y'), ('ỷ', 'y'), ('ỹ', 'y'),
        ('đ', 'd'), ('Đ', 'D'),
    ];
    for (from, to) in pairs {
        map.insert(from, to);
    }
    map
});

fn remove_vietnamese_accents(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| *VIETNAMESE_CHAR_MAP.get(&c).unwrap_or(&c))
        .collect()
}

pub fn normalize_string(s: &str) -> String {
    remove_vietnamese_accents(s)
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// --- Filesystem Scan Helpers ---

pub fn report_progress(reporter: &Option<Sender<GuiUpdate>>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

pub fn discover_fs_structure(root_path: &Path, reporter: &Option<Sender<GuiUpdate>>) -> (Vec<walkdir::DirEntry>, Vec<PathBuf>) {
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

pub fn scan_subdirs<F: Fn(&walkdir::DirEntry) + Send + Sync>(
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

pub fn build_file_data(entry: &walkdir::DirEntry, root_path: &Path) -> (String, FileMetadata) {
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
