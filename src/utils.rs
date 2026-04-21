
use std::sync::LazyLock;
use std::collections::HashMap;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;
use crate::gui::events::{GuiUpdate, GuiSender};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use rayon::prelude::*;
use walkdir::WalkDir;
use std::sync::Arc;
use crate::pop::control::TaskController;


// --- String Normalization Helpers ---

static VIETNAMESE_CHAR_MAP: LazyLock<HashMap<char, char>> = LazyLock::new(|| {
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

pub fn report_progress(reporter: Option<&GuiSender>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

pub fn discover_fs_structure(root_path: &Path, reporter: Option<&GuiSender>) -> (Vec<walkdir::DirEntry>, Vec<PathBuf>) {
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

/// A new, reusable helper that performs a controllable 2-phase scan.
/// It encapsulates the logic of discovering, iterating, and checking the controller state.
pub fn controlled_two_phase_scan<F>(
    root_path: &Path,
    reporter: Option<&GuiSender>,
    controller: &Arc<TaskController>,
    action: F,
)
where
    F: Fn(walkdir::DirEntry) + Send + Sync,
{
    let (top_level_entries, subdirs) = discover_fs_structure(root_path, reporter);

    // --- Phase 1: Process top-level files ---
    let top_level_files: Vec<_> = top_level_entries.into_iter().filter(|e| e.path().is_file()).collect();
    top_level_files.into_par_iter().for_each(|entry| {
        if controller.is_cancelled() { return; }
        controller.check_and_wait_if_paused();
        if controller.is_cancelled() { return; }
        action(entry);
    });

    if controller.is_cancelled() { return; }

    // --- Phase 2: Process subdirectories ---
    let num_subdirs = subdirs.len();
    let processed_subdirs = AtomicUsize::new(0);
    report_progress(reporter, 0.05, "Phase 2/2: Scanning files...");

    subdirs.into_par_iter().for_each(|subdir| {
        if controller.is_cancelled() { return; }

        let walker = WalkDir::new(&subdir).into_iter().filter_map(Result::ok);
        for entry in walker {
            if entry.path().is_file() {
                if controller.is_cancelled() { break; } // Break from inner loop
                controller.check_and_wait_if_paused();
                if controller.is_cancelled() { break; }
                action(entry);
            }
        }

        let processed_count = processed_subdirs.fetch_add(1, Ordering::SeqCst);
        if num_subdirs > 0 {
            #[allow(clippy::cast_precision_loss)]
            let progress = 0.05 + (processed_count as f32 / num_subdirs as f32) * 0.40;
            report_progress(reporter, progress, &format!("Scanning in {}...", subdir.display()));
        }
    });
}



/// Checks if a target string contains all of the provided tokens.
pub fn contains_all_tokens(target: &str, tokens: &[&str]) -> bool {
    if tokens.is_empty() {
        return true; // Or false, depending on desired behavior for empty query
    }
    tokens.iter().all(|token| target.contains(token))
}

// Helper to get an icon based on file extension
pub fn get_icon_for_path(path: &str) -> &'static str {
    let path_buf = PathBuf::from(path);
    if path_buf.is_dir() {
        return "📁"; // Folder icon
    }
    match path_buf.extension().and_then(|s| s.to_str()) {
        Some("txt" | "md" | "log") => "📄", // Text file
        Some("pdf") => "📃", // PDF
        Some("doc" | "docx") => "📝", // Word document
        Some("xls" | "xlsx" | "csv") => "📊", // Spreadsheet
        Some("ppt" | "pptx") => " presentation", // Presentation
        Some("zip" | "rar" | "7z" | "tar" | "gz") => "📦", // Archive
        Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg") => "🖼️", // Image
        Some("mp3" | "wav" | "flac" | "ogg") => "🎵", // Audio
        Some("mp4" | "mkv" | "avi" | "mov") => "🎬", // Video
        Some("exe" | "dll" | "bin") => "⚙️", // Executable/Binary
        Some("rs" | "py" | "js" | "html" | "css" | "json" | "xml") => "💻", // Code
        _ => "🗄️", // Generic file
    }
}
