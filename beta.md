# Ghi chú phiên bản v1.2.1-beta

Đây là phiên bản beta đầu tiên kể từ phiên bản ổn định 1.2.0. Phiên bản này tập trung vào việc bổ sung tính năng **Live Search**, đồng thời tái cấu trúc và tối ưu hóa sâu rộng các kiến trúc cốt lõi.

## ✨ Tính năng mới: Live Search (Tìm kiếm trực tiếp)

Bên cạnh tính năng "Indexing" truyền thống, giờ đây người dùng có thể thực hiện tìm kiếm trực tiếp trên một thư mục được chỉ định mà không cần lập chỉ mục trước.

- **Kích hoạt:** Tại tab "Search", chọn checkbox "Live Search in Folder".
- **Hai chế độ tìm kiếm:**
    1.  **Tìm theo tên file (Mặc định):** Tìm kiếm siêu nhanh, chỉ dựa trên tên file.
    2.  **Tìm trong nội dung (Tùy chọn):** Chọn checkbox "Search in file content" để kích hoạt tìm kiếm bên trong nội dung file.

## 🚀 Cải tiến & Tái cấu trúc

### 1. Tối ưu hóa Kiến trúc Quét File
- **Giữ lại chiến lược "2-phase scan":** Sau quá trình thử nghiệm và benchmark, chiến lược "quét 2 pha" (dùng `walkdir` để khám phá thư mục và `rayon` để xử lý song song) được giữ lại làm công nghệ quét file thống nhất cho **tất cả các tác vụ** (Initial Scan, Rescan, và Live Search).
- **Hiệu năng vượt trội:** Trong thực tế, kiến trúc này cho thấy **hiệu năng cao hơn** so với các phương pháp duyệt song song từ đầu (ví dụ: `jwalk`). Điều này khẳng định lựa chọn kiến trúc hiện tại là tối ưu cho workload của ứng dụng.

### 2. Tái cấu trúc luồng "Rescan"
- **Quy trình 3 bước an toàn:** Luồng "Rescan" đã được tái cấu trúc hoàn toàn thành một workflow 3 process riêng biệt (`rescan_scan_streaming`, `rescan_write_index_from_stream_batched`, `rescan_atomic_swap_final`).
- **Tính toàn vẹn dữ liệu:** Mô hình mới đảm bảo việc quét lại diễn ra trên một bảng CSDL tạm. Chỉ sau khi hoàn tất 100%, bảng mới sẽ được hoán đổi (atomic swap) với bảng cũ. Điều này giúp loại bỏ hoàn toàn rủi ro làm hỏng chỉ mục hiện có nếu quá trình quét lại bị gián đoạn.

### 3. Thống nhất Logic Tìm kiếm
- **Tìm kiếm dựa trên token:** Logic tìm kiếm tên file của Live Search đã được sửa lại để hoạt động theo cơ chế **token-based** (tách từ khóa thành các token và so khớp) giống hệt như Indexed Search, đảm bảo kết quả tìm kiếm nhất quán.
- **Tái sử dụng code:** Logic so khớp `contains_all_tokens` đã được trừu tượng hóa và sử dụng chung, đúng theo triết lý POP.

### 4. Mở rộng Xử lý Nội dung File
- **Hỗ trợ đa định dạng:** Khả năng tìm kiếm nội dung đã được mở rộng để hỗ trợ các định dạng phổ biến:
    - **PDF:** Sử dụng thư viện `pdf-extract`. Kết quả sẽ hiển thị rõ ràng số trang (`[Page X]`).
    - **Microsoft Word (.docx):** Sử dụng thư viện `docx_rs`.
    - **Microsoft Excel (.xlsx):** Sử dụng thư viện `calamine`.
- **Bỏ qua file nhị phân:** Chương trình chủ động bỏ qua các file không thể đọc được (như `.jpg`, `.exe`, `.zip`...) để tăng tốc và tránh trả về kết quả rác.

## 🐞 Sửa lỗi (Bug Fixes)

- **Sửa lỗi hiển thị của Live Search:**
    - Live Search không còn cộng dồn kết quả của các phiên tìm kiếm khác nhau.
    - Sửa lỗi kết quả tìm kiếm theo tên file không được hiển thị trên giao diện.
- **Sửa lỗi hiển thị kết quả PDF:** Định dạng hiển thị kết quả từ file PDF đã được làm lại cho rõ ràng hơn.

## 📝 Ghi chú khác

- **Cấu hình Thread Pool:** Đã xác nhận lại rằng dự án đang cấu hình một cách tường minh cho `rayon` sử dụng một thread pool toàn cục với số luồng bằng `số lõi CPU logic * 2` để tối ưu hiệu năng.
