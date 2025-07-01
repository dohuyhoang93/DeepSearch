use colored::*;
use num_cpus;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;
use walkdir::WalkDir;

// Kiểm tra xem có thư mục cấp 2 trong cấp 1 hay không
fn has_second_level(path: &Path) -> bool {
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .any(|e| {
            e.path().is_dir()
                && WalkDir::new(e.path())
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .any(|f| f.is_ok())
        })
}

// Thiết lập số thread Rayon gấp đôi số nhân logic CPU
fn setup_thread_pool() {
    let num_cpus = num_cpus::get();
    let num_threads = num_cpus * 2;
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();
    println!(
        "🚀 Logical CPUs: {} | Threads used: {}",
        num_cpus, num_threads
    );
}

/// Trả về bảng ánh xạ từ ký tự có dấu sang không dấu
fn vietnamese_char_map() -> HashMap<char, char> {
    let mut map = HashMap::new();
    let pairs = [
        ('à', 'a'),
        ('á', 'a'),
        ('ạ', 'a'),
        ('ả', 'a'),
        ('ã', 'a'),
        ('â', 'a'),
        ('ầ', 'a'),
        ('ấ', 'a'),
        ('ậ', 'a'),
        ('ẩ', 'a'),
        ('ẫ', 'a'),
        ('ă', 'a'),
        ('ằ', 'a'),
        ('ắ', 'a'),
        ('ặ', 'a'),
        ('ẳ', 'a'),
        ('ẵ', 'a'),
        ('è', 'e'),
        ('é', 'e'),
        ('ẹ', 'e'),
        ('ẻ', 'e'),
        ('ẽ', 'e'),
        ('ê', 'e'),
        ('ề', 'e'),
        ('ế', 'e'),
        ('ệ', 'e'),
        ('ể', 'e'),
        ('ễ', 'e'),
        ('ì', 'i'),
        ('í', 'i'),
        ('ị', 'i'),
        ('ỉ', 'i'),
        ('ĩ', 'i'),
        ('ò', 'o'),
        ('ó', 'o'),
        ('ọ', 'o'),
        ('ỏ', 'o'),
        ('õ', 'o'),
        ('ô', 'o'),
        ('ồ', 'o'),
        ('ố', 'o'),
        ('ộ', 'o'),
        ('ổ', 'o'),
        ('ỗ', 'o'),
        ('ơ', 'o'),
        ('ờ', 'o'),
        ('ớ', 'o'),
        ('ợ', 'o'),
        ('ở', 'o'),
        ('ỡ', 'o'),
        ('ù', 'u'),
        ('ú', 'u'),
        ('ụ', 'u'),
        ('ủ', 'u'),
        ('ũ', 'u'),
        ('ư', 'u'),
        ('ừ', 'u'),
        ('ứ', 'u'),
        ('ự', 'u'),
        ('ử', 'u'),
        ('ữ', 'u'),
        ('ỳ', 'y'),
        ('ý', 'y'),
        ('ỵ', 'y'),
        ('ỷ', 'y'),
        ('ỹ', 'y'),
        ('đ', 'd'),
        ('Đ', 'D'),
    ];
    for (from, to) in pairs {
        map.insert(from, to);
    }
    map
}

/// Loại bỏ dấu tiếng Việt bằng bảng ánh xạ
fn remove_vietnamese_accents(s: &str) -> String {
    let char_map = vietnamese_char_map();
    s.nfd() // Chuẩn hóa Unicode NFD
        .filter(|c| !is_combining_mark(*c)) // Xóa dấu kết hợp
        .map(|c| *char_map.get(&c).unwrap_or(&c)) // Thay thế ký tự có dấu thành không dấu
        .collect()
}

/// Chuẩn hóa chuỗi: Loại bỏ dấu, chuyển thành chữ thường, xóa khoảng trắng dư thừa
fn normalize_string(s: &str) -> String {
    remove_vietnamese_accents(s) // Bỏ dấu tiếng Việt
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace()) // Giữ chữ cái, số và khoảng trắng
        .collect::<String>()
        .to_lowercase() // Chuyển thành chữ thường
        .split_whitespace()
        .collect::<Vec<_>>() // Xóa khoảng trắng dư thừa giữa các từ
        .join(" ") // Nối lại thành chuỗi với dấu cách chuẩn
}

fn input_path() -> String {
    loop {
        print!("{}", "\n⌨️ Enter folder path: ".bold().green());
        io::stdout().flush().unwrap();
        let mut path = String::new();
        io::stdin()
            .read_line(&mut path)
            .expect("Invalid path. Re-enter");
        let path = path.trim().to_string();
        if Path::new(&path).exists() {
            break path;
        } else {
            println!("❌ Error: Invalid path! Re enter path.");
        }
    }
}

fn input_keyword() -> String {
    print!("{}", "\n⌨️ Enter keyword: ".bold().green());
    io::stdout().flush().unwrap();
    let mut keyword = String::new();
    io::stdin()
        .read_line(&mut keyword)
        .expect("Error keyword input");
    keyword.trim().to_string()
}

fn main() {
    let logo = r#"
██████╗░███████╗███████╗██████╗░  ░██████╗███████╗░█████╗░██████╗░░█████╗░██╗░░██╗
██╔══██╗██╔════╝██╔════╝██╔══██╗  ██╔════╝██╔════╝██╔══██╗██╔══██╗██╔══██╗██║░░██║
██║░░██║█████╗░░█████╗░░██████╔╝  ╚█████╗░█████╗░░███████║██████╔╝██║░░╚═╝███████║
██║░░██║██╔══╝░░██╔══╝░░██╔═══╝░  ░╚═══██╗██╔══╝░░██╔══██║██╔══██╗██║░░██╗██╔══██║
██████╔╝███████╗███████╗██║░░░░░  ██████╔╝███████╗██║░░██║██║░░██║╚█████╔╝██║░░██║
╚═════╝░╚══════╝╚══════╝╚═╝░░░░░  ╚═════╝░╚══════╝╚═╝░░╚═╝╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝"#;
    println!("{}", logo.bold().blue());
    println!("{}", "Developed by THEUS".bold().blue());

    setup_thread_pool();
    let is_running = Arc::new(AtomicBool::new(true));
    let is_paused = Arc::new(AtomicBool::new(false)); // Kiểm soát tạm dừng
    let is_stopped = Arc::new(AtomicBool::new(false)); // Kiểm soát dừng ngay lập tức

    loop {
        let path = input_path();
        let depth_limit = if has_second_level(Path::new(&path)) {
            2
        } else {
            1
        };

        println!("📁 Folder depth: {}", depth_limit);

        let keyword = input_keyword();
        let normalized_keyword = normalize_string(&keyword);

        // Reset trạng thái trước mỗi lần tìm kiếm
        is_running.store(true, Ordering::Relaxed);
        is_paused.store(false, Ordering::Relaxed);
        is_stopped.store(false, Ordering::Relaxed);

        let is_running_clone = Arc::clone(&is_running);
        let is_paused_clone = Arc::clone(&is_paused);
        let is_stopped_clone = Arc::clone(&is_stopped);

        // 🏆 Thread lắng nghe input nhưng sẽ tự động dừng khi `is_running == false`
        let input_handle = thread::spawn(move || {
            while is_running_clone.load(Ordering::SeqCst) {
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err()
                    || !is_running_clone.load(Ordering::Relaxed)
                {
                    break; // ⏹ Thoát thread nếu `is_running = false`
                }
                let input = input.trim();

                match input {
                    "p" => {
                        is_paused_clone.store(true, Ordering::SeqCst);
                        println!(
                            "{}",
                            "⏸  Paused 💤! Enter 'r' to resume".bold().bright_purple()
                        );
                    }
                    "r" => {
                        is_paused_clone.store(false, Ordering::SeqCst);
                        println!("{}", "▶  Scanning...".bold().bright_purple());
                    }
                    "s" => {
                        is_stopped_clone.store(true, Ordering::SeqCst);
                        println!("{}", ">> Stop...".bold().bright_purple());
                        break;
                    }
                    _ => {}
                }
            }
        });

        let start = Instant::now();
        println!(
            "\n🔍 Scanning '{}' in '{}'...",
            normalized_keyword.bold().yellow(),
            path.bold().yellow()
        );
        println!(
            "{}",
            "Enter 'p' to Pause, 'r' to Resume, 's' to Stop".bright_purple()
        );

        let count = Arc::new(AtomicUsize::new(0)); // Tạo biến đếm an toàn
        let count_clone = Arc::clone(&count); // Clone để truyền vào closure
        let is_stopped_clone = Arc::clone(&is_stopped);
        let is_paused_clone = Arc::clone(&is_paused);

        // Giai đoạn 1: Quét file trong thư mục cấp 1
        WalkDir::new(&path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .par_bridge()
            .filter_map(Result::ok)
            .for_each(|entry| {
                while is_paused_clone.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(100));
                }
                if is_stopped_clone.load(Ordering::SeqCst) {
                    return;
                }

                let file_name = entry.file_name().to_string_lossy();
                let normalized_file_name = normalize_string(&file_name);
                if normalized_file_name.contains(&normalized_keyword) {
                    println!("{}", entry.path().display().to_string().cyan());
                    count_clone.fetch_add(1, Ordering::Relaxed);
                }
            });

        // Giai đoạn 2: Quét các thư mục cấp 2
        let subdirs: Vec<_> = WalkDir::new(&path)
            .min_depth(depth_limit)
            .max_depth(depth_limit)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .map(|e| e.path().to_path_buf())
            .collect();

        let is_stopped_clone = Arc::clone(&is_stopped);
        let is_paused_clone = Arc::clone(&is_paused);
        let count_clone = Arc::clone(&count);

        let _ = subdirs.par_iter().try_for_each(|subdir| {
            WalkDir::new(subdir)
                .into_iter()
                .par_bridge()
                .filter_map(Result::ok)
                .try_for_each(|entry| {
                    if is_stopped_clone.load(Ordering::SeqCst) {
                        return Err(());
                    } // Dừng toàn bộ thread
                    while is_paused_clone.load(Ordering::SeqCst) {
                        if is_stopped_clone.load(Ordering::SeqCst) {
                            return Err(());
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                    if is_stopped_clone.load(Ordering::SeqCst) {
                        return Err(());
                    }

                    let file_name = entry.file_name().to_string_lossy();
                    let normalized_file_name = normalize_string(&file_name);
                    if normalized_file_name.contains(&normalized_keyword) {
                        println!("{}", entry.path().display().to_string().cyan());
                        count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(())
                })
        });

        let duration = start.elapsed();
        let final_count = count.load(Ordering::Relaxed);
        println!(
            "\n🔎 Total file: {} | Time: {} min {} s",
            final_count,
            duration.as_secs() / 60,
            duration.as_secs() % 60
        );
        println!(
            "{}",
            ">> Scan completed. Press Enter to start a new session!"
                .bold()
                .bright_green()
        );

        is_running.store(false, Ordering::SeqCst);
        input_handle.join().unwrap();
    }
}
