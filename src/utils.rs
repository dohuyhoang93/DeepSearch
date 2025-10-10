use once_cell::sync::Lazy;
use std::collections::HashMap;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

// Sử dụng Lazy để đảm bảo HashMap chỉ được tạo một lần.
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

/// Loại bỏ dấu tiếng Việt bằng bảng ánh xạ đã được khởi tạo một lần.
fn remove_vietnamese_accents(s: &str) -> String {
    s.nfd() // Chuẩn hóa Unicode NFD
        .filter(|c| !is_combining_mark(*c)) // Xóa dấu kết hợp
        .map(|c| *VIETNAMESE_CHAR_MAP.get(&c).unwrap_or(&c)) // Thay thế ký tự
        .collect()
}

/// Chuẩn hóa chuỗi: Loại bỏ dấu, chuyển thành chữ thường, xóa khoảng trắng dư thừa.
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