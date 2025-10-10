use once_cell::sync::Lazy;
use std::collections::HashMap;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

// Use Lazy to ensure the HashMap is created only once.
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

/// Removes Vietnamese accents using a pre-initialized mapping table.
fn remove_vietnamese_accents(s: &str) -> String {
    s.nfd() // Normalize to Unicode NFD
        .filter(|c| !is_combining_mark(*c)) // Remove combining marks
        .map(|c| *VIETNAMESE_CHAR_MAP.get(&c).unwrap_or(&c)) // Replace characters
        .collect()
}

/// Normalizes a string: removes accents, converts to lowercase, and removes extra whitespace.
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