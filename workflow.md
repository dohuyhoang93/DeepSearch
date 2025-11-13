# Sơ Đồ Workflow - DeepSearch

Tài liệu này mô tả chi tiết các luồng xử lý (workflow) chính của ứng dụng DeepSearch, được cập nhật để phản ánh chính xác mã nguồn hiện tại.

---

### Ghi chú chung về Công nghệ Quét File

**Điểm quan trọng:** Tất cả các quy trình liên quan đến việc quét hệ thống file trong ứng dụng (Initial Scan, Rescan, và Live Search) đều sử dụng **cùng một hàm tiện ích lõi**: `utils::controlled_two_phase_scan`.

*   **Công nghệ được chọn:** `walkdir` kết hợp với `rayon`.
*   **Chiến lược:** "Quét 2 pha" (2-phase scan). Pha 1 khám phá các thư mục con ở cấp cao nhất, và Pha 2 xử lý song song các thư mục đó.
*   **Lý do lựa chọn:** Đây là một quyết định kiến trúc có chủ đích. Trong các thử nghiệm và benchmark thực tế, chiến lược này cho thấy **hiệu năng vượt trội** so với các phương pháp duyệt song song từ đầu (parallel-first) như `jwalk` + `par_bridge` cho khối lượng công việc của ứng dụng này.
*   **Cơ chế điều khiển:** Tất cả các tác vụ quét đều có thể được Tạm dừng/Tiếp tục/Hủy bỏ từ giao diện người dùng thông qua `TaskController`.

---

## Workflow 1: Quét và Lập Chỉ Mục Ban Đầu (Initial Scan)

*   **Tên workflow trong code:** `gui_initial_scan`
*   **Mục đích:** Quét một thư mục mới, thu thập thông tin của tất cả các file và tạo chỉ mục tìm kiếm trong cơ sở dữ liệu.

**Sơ đồ xử lý:**
`scan_directory_streaming` -> `write_index_from_stream_batched`

**Diễn giải chi tiết:**
1.  **Process: `processes::scan::scan_directory_streaming`**
    *   Tạo một channel `mpsc` để stream dữ liệu file.
    *   Tạo một luồng (thread) mới và gọi `utils::controlled_two_phase_scan` để thực hiện việc quét.
    *   Trong quá trình quét, mỗi file tìm thấy sẽ được xử lý để tạo `FileMetadata`.
    *   Gửi `(path, metadata)` qua `mpsc::Sender`.
    *   Process này trả về `Context` có chứa `mpsc::Receiver` (đầu nhận của channel).

2.  **Process: `processes::index::write_index_from_stream_batched`**
    *   Nhận `Context` chứa `mpsc::Receiver`.
    *   Lặp và nhận dữ liệu từ channel, gom chúng thành từng lô (batch).
    *   Khi một lô đầy, gọi `db::DbManager::write_to_table` để ghi toàn bộ lô vào CSDL `redb`.

---

## Workflow 2: Quét Lại (Rescan) với Atomic Swap

*   **Tên workflow trong code:** `gui_rescan`
*   **Mục đích:** Cập nhật chỉ mục của một địa điểm đã có một cách an toàn và hiệu quả.

**Sơ đồ xử lý:**
`rescan_scan_streaming` -> `rescan_write_index_from_stream_batched` -> `rescan_atomic_swap_final`

**Diễn giải chi tiết:**
1.  **Process: `processes::scan::rescan_scan_streaming`**
    *   Tạo một tên bảng mới, duy nhất trong CSDL.
    *   Lấy tên bảng cũ.
    *   Thực hiện việc quét file tương tự như `scan_directory_streaming` và stream dữ liệu qua channel.

2.  **Process: `processes::index::rescan_write_index_from_stream_batched`**
    *   Nhận dữ liệu từ channel và ghi vào **bảng mới** trong CSDL theo từng lô.

3.  **Process: `processes::scan::rescan_atomic_swap_final`**
    *   Sau khi bảng mới đã được ghi xong, gọi `db::DbManager::swap_location_table` để cập nhật con trỏ trong bảng `locations` trỏ tới bảng mới.
    *   Ngay sau đó, gửi yêu cầu xóa toàn bộ bảng cũ khỏi CSDL.

---

## Workflow 3: Tìm Kiếm trong Chỉ mục (Indexed Search)

*   **Tên workflow trong code:** `gui_search`
*   **Mục đích:** Tìm kiếm trong các chỉ mục đã được tạo và hiển thị kết quả.

**Sơ đồ xử lý:**
`search_index`

**Diễn giải chi tiết:**
1.  **Process: `processes::search::search_index`**
    *   Chuẩn hóa từ khóa tìm kiếm.
    *   Lặp qua từng địa điểm (`location`) cần tìm kiếm.
    *   Với mỗi địa điểm, gọi `db::DbManager::search_in_table` để lấy về danh sách các đường dẫn tương đối khớp với từ khóa.
    *   Gửi kết quả về cho luồng GUI theo từng lô nhỏ (batch) qua thông điệp `GuiUpdate::SearchResultsBatch`.
    *   Khi hoàn tất, gửi `GuiUpdate::SearchFinished`.

---

## Workflow 4: Tìm Kiếm Trực Tiếp (Live Search)

*   **Tên workflow trong code:** `gui_live_search`
*   **Mục đích:** Tìm kiếm trực tiếp trên hệ thống file mà không cần chỉ mục.

**Sơ đồ xử lý:**
`live_search_2_phase`

**Diễn giải chi tiết:**
1.  **Process: `processes::live_search::live_search_2_phase`**
    *   Tạo một luồng (thread) mới để thực hiện toàn bộ tác vụ.
    *   Bên trong luồng, gọi hàm tiện ích `utils::controlled_two_phase_scan` để quét file.
    *   Định nghĩa một hành động (`action`) được thực thi cho mỗi file tìm thấy:
        *   **Nếu tìm theo tên file:** Chuẩn hóa tên file và so khớp với các token của từ khóa bằng `utils::contains_all_tokens`.
        *   **Nếu tìm trong nội dung:**
            *   Kiểm tra đuôi file (`.pdf`, `.docx`, `.xlsx`, `.txt`...).
            *   Sử dụng các thư viện tương ứng để trích xuất nội dung: `pdf_extract`, `docx_rs`, `calamine`.
            *   Tìm kiếm từ khóa trong nội dung đã trích xuất.
    *   Các kết quả tìm thấy (cả tên file và nội dung) được gửi về luồng GUI theo từng lô nhỏ qua `GuiUpdate::LiveSearchResultsBatch` hoặc `GuiUpdate::SearchResultsBatch`.
    *   Khi quét xong, gửi `GuiUpdate::SearchFinished`.