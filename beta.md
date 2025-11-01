# Ghi chú phiên bản v1.2.1-beta

Đây là phiên bản beta đầu tiên kể từ phiên bản ổn định 1.2.0. Phiên bản này tập trung vào việc bổ sung một tính năng hoàn toàn mới là **Live Search** và cải tiến sâu rộng về kiến trúc để đảm bảo hiệu năng và sự nhất quán.

## ✨ Tính năng mới: Live Search (Tìm kiếm trực tiếp)

Bên cạnh tính năng "Indexing" truyền thống, giờ đây người dùng có thể thực hiện tìm kiếm trực tiếp trên một thư mục được chỉ định mà không cần lập chỉ mục trước.

- **Kích hoạt:** Tại tab "Search", chọn checkbox "Live Search in Folder".
- **Chọn thư mục:** Một giao diện chọn thư mục sẽ hiện ra để người dùng chỉ định nơi cần tìm kiếm.
- **Hai chế độ tìm kiếm:**
    1.  **Tìm theo tên file (Mặc định):** Tìm kiếm siêu nhanh, chỉ dựa trên tên file.
    2.  **Tìm trong nội dung (Tùy chọn):** Chọn checkbox "Search in file content" để kích hoạt tìm kiếm bên trong nội dung file.

## 🚀 Cải tiến & Tái cấu trúc

### 1. Hiệu năng Live Search
- **Kiến trúc duyệt file song song:** Đã loại bỏ hoàn toàn kiến trúc 2-phase (khám phá rồi mới quét) và `walkdir` tuần tự. Thay vào đó, Live Search hiện sử dụng thư viện `jwalk` để duyệt cây thư mục một cách song song ngay từ đầu.
- **Cơ chế "Work-Stealing":** Tận dụng tối đa các lõi CPU với cơ chế "tranh việc" của Rayon, giúp cân bằng tải hiệu quả và tăng tốc độ quét trên các thư mục lớn và ổ đĩa mạng (SMB).
- **Phản hồi tức thì:** Kiến trúc mới đảm bảo kết quả đầu tiên được trả về giao diện gần như ngay lập tức, không còn bị chặn ở giai đoạn "discovery".

### 2. Logic Tìm kiếm
- **Thống nhất logic tìm kiếm tên file:**
    - **Sửa lỗi nghiêm trọng:** Logic tìm kiếm tên file của Live Search đã được sửa lại để hoạt động theo cơ chế **token-based** (tách từ khóa thành các token và so khớp) giống hệt như Indexed Search.
    - **Tái sử dụng code:** Logic so khớp token (`contains_all_tokens`) đã được trừu tượng hóa thành một hàm tiện ích trong `utils.rs` và được cả hai chế độ tìm kiếm sử dụng lại, đảm bảo tính nhất quán và dễ bảo trì, đúng theo triết lý POP.

### 3. Xử lý Nội dung File
- **Hỗ trợ tìm kiếm trong file PDF:**
    - Thay thế thư viện `pdf-extract` bằng `lopdf` mạnh mẽ hơn.
    - Live Search giờ đây có thể đọc nội dung văn bản từ file `.pdf` và tìm kiếm bên trong đó.
    - **Hiển thị số trang:** Kết quả tìm thấy trong file PDF sẽ hiển thị rõ ràng số trang (`[Page X]`) thay vì số dòng, giúp người dùng định vị dễ dàng.
- **Bỏ qua file nhị phân:** Khi tìm kiếm nội dung, chương trình sẽ chủ động bỏ qua các file nhị phân không thể đọc được (như `.jpg`, `.exe`, `.zip`...) để tăng tốc và tránh trả về kết quả rác.

## 🐞 Sửa lỗi (Bug Fixes)

- **Sửa lỗi hiển thị của Live Search:**
    - Live Search không còn cộng dồn kết quả của các phiên tìm kiếm khác nhau. Màn hình kết quả sẽ được làm mới sau mỗi lần nhấn "Search".
    - Sửa lỗi kết quả tìm kiếm theo tên file không được hiển thị trên giao diện dù status bar có báo tìm thấy.
- **Sửa lỗi hiển thị kết quả PDF:** Định dạng hiển thị kết quả từ file PDF đã được làm lại cho rõ ràng, dễ hiểu hơn (`path [Page X] - content`).

## 📝 Ghi chú khác

- **Cấu hình Thread Pool:** Đã xác nhận lại rằng dự án đang cấu hình một cách tường minh cho `rayon` sử dụng một thread pool toàn cục với số luồng bằng `số lõi CPU logic * 2` để tối ưu hiệu năng.
