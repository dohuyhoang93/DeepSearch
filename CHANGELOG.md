# Changelog

All notable changes to DeepSearch will be documented in this file.

---

## [1.2.1] - 2026-04-21

### ✨ Tính năng mới

#### Live Search (Tìm kiếm trực tiếp)
Bên cạnh "Indexed Search" truyền thống, người dùng có thể tìm kiếm trực tiếp trên một thư mục mà không cần lập chỉ mục trước.

- **Kích hoạt:** Tại tab "Search", chọn checkbox "Live Search in Folder".
- **Hai chế độ:**
  1. **Tìm theo tên file (Mặc định):** Tìm kiếm siêu nhanh, chỉ dựa trên tên file.
  2. **Tìm trong nội dung:** Chọn checkbox "Search in file content" để tìm kiếm bên trong nội dung file.

#### Hỗ trợ đa định dạng nội dung
- **PDF** (`pdf-extract`): Hiển thị kết quả kèm số trang `[Page X]`.
- **Microsoft Word** (`.docx`): Hỗ trợ qua `docx-rs`.
- **Microsoft Excel** (`.xlsx`): Hỗ trợ qua `calamine`.
- Tự động bỏ qua file nhị phân (`.jpg`, `.exe`, `.zip`, ...) để tránh kết quả rác.

---

### 🚀 Cải tiến & Tái cấu trúc

#### Nâng cấp redb 2.6.3 → 4.1.0
Nâng cấp thư viện cơ sở dữ liệu nhúng lên phiên bản mới nhất mang lại:
- **~15% nhanh hơn** khi đọc concurrent từ nhiều thread.
- **~1.5x nhanh hơn** tốc độ ghi tổng quát.
- File DB tối thiểu giảm từ ~2.5 MB xuống còn ~50 KB.
- Nhiều bug data corruption và memory leak nghiêm trọng được vá.

#### Kiến trúc quét file
- **Giữ lại "2-phase scan":** Dùng `walkdir` để khám phá thư mục và `rayon` để xử lý song song — thống nhất cho tất cả tác vụ (Initial Scan, Rescan, Live Search).
- **Rescan an toàn hơn:** Quy trình 3 bước (`scan → write temp table → atomic swap`), đảm bảo chỉ mục hiện có không bị hỏng nếu quá trình bị gián đoạn.

#### Tìm kiếm nhất quán
- Logic tìm kiếm tên file của Live Search hoạt động theo cơ chế **token-based**, giống Indexed Search — kết quả nhất quán giữa hai chế độ.
- `contains_all_tokens` được trừu tượng hóa và dùng chung.

#### Code quality — Clippy pedantic
Toàn bộ codebase được làm sạch với `cargo clippy -D clippy::all -D clippy::pedantic` (63 lỗi đã fix):
- `ref_option`: Đổi `&Option<T>` → `Option<&T>` trong các hàm utility để tối ưu borrow.
- `non_std_lazy_statics`: Loại bỏ dependency `once_cell`, dùng `std::sync::LazyLock` (stable từ Rust 1.80).
- `unnested_or_patterns`, `uninlined_format_args`, `map_unwrap_or`, `implicit_clone`, `redundant_closure`, `if_not_else`, `derivable_impls`, `manual_string_new`, `default_trait_access`, `doc_markdown`, `case_sensitive_file_extension_comparisons`.
- Xóa dependency `once_cell` khỏi `Cargo.toml`.

---

### 🐞 Sửa lỗi

- Live Search không còn cộng dồn kết quả giữa các phiên tìm kiếm khác nhau.
- Sửa lỗi kết quả tìm kiếm theo tên file không hiển thị trên giao diện.
- Sửa lỗi hiển thị kết quả PDF (định dạng rõ ràng hơn).
- Sửa lỗi so sánh phần mở rộng file không phân biệt hoa/thường (`.pdf` → `.PDF` nay được nhận diện đúng).

---

## [1.2.0] - 2025-xx-xx

- Phiên bản ổn định trước đó.

---

## [1.1.0]

- Xem git log để biết chi tiết.

## [1.0.0]

- Phát hành lần đầu.
