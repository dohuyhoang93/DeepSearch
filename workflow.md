# Sơ Đồ Workflow - DeepSearch (v2)

Tài liệu này mô tả chi tiết các luồng xử lý (workflow) chính của ứng dụng DeepSearch, được suy ra từ mã nguồn. Phiên bản này được cập nhật để phản ánh logic refactor và chi tiết hóa luồng gọi hàm.

---

## Workflow 1: Quét và Lập Chỉ Mục Ban Đầu (Initial Scan)

*   **Tên workflow trong code:** `gui_initial_scan`
*   **Mục đích:** Quét một thư mục mới, thu thập thông tin của tất cả các file và tạo chỉ mục tìm kiếm trong cơ sở dữ liệu.

**Sơ đồ xử lý:**
`scan_directory_streaming (scan.rs)` -> `write_index_from_stream_batched (index.rs)`

**Diễn giải chi tiết:**
1.  **Process: `processes::scan::scan_directory_streaming`**
    *   Tạo một channel `mpsc` để stream dữ liệu file.
    *   Tạo một luồng (thread) mới để thực hiện việc quét.
    *   Bên trong luồng quét:
        *   Gọi `utils::discover_fs_structure` để lấy danh sách các file/thư mục ở cấp đầu tiên.
        *   Gọi `utils::scan_subdirs` để quét song song (`rayon::par_iter`) các thư mục con.
        *   Trong quá trình quét, mỗi file tìm thấy sẽ được xử lý bởi `utils::build_file_data` để tạo `FileMetadata`.
        *   Gửi `(path, metadata)` qua `mpsc::Sender`.
    *   Process này trả về `Context` có chứa `mpsc::Receiver` (đầu nhận của channel).

2.  **Process: `processes::index::write_index_from_stream_batched`**
    *   Nhận `Context` chứa `mpsc::Receiver` từ process trước.
    *   Lặp và nhận dữ liệu từ channel.
    *   Gom dữ liệu thành từng lô (batch) có kích thước `BATCH_SIZE`.
    *   Khi một lô đầy, gọi `db::DbManager::write_index_for_path` để ghi toàn bộ lô vào CSDL `redb`.
    *   Hàm `write_index_for_path` sẽ gọi `get_or_create_table_name` để tạo bảng nếu chưa có, sau đó mở transaction và ghi dữ liệu.

---

## Workflow 2: Quét Lại (Rescan) với Atomic Swap

*   **Tên workflow trong code:** `gui_rescan`
*   **Mục đích:** Cập nhật chỉ mục của một địa điểm đã có để phản ánh những thay đổi trong hệ thống file một cách an toàn và hiệu quả.

**Sơ đồ xử lý:**
`rescan_atomic_swap (scan.rs)`

**Diễn giải chi tiết:**
1.  **Process: `processes::scan::rescan_atomic_swap`**
    *   **Pha 1: Xây dựng chỉ mục mới**
        *   Tạo một tên bảng mới, duy nhất (ví dụ: `index_{hash}_{timestamp}`).
        *   Thực hiện quy trình quét file song song y hệt như `scan_directory_streaming` và stream dữ liệu vào một channel `mpsc`.
        *   Nhận dữ liệu từ channel và ghi trực tiếp vào **bảng mới** theo từng lô (`batch`). Toàn bộ quá trình này là một luồng ghi tuần tự hiệu suất cao.
    *   **Pha 2: Hoán đổi và Dọn dẹp**
        *   Sau khi bảng mới đã được ghi xong, gọi `db::DbManager::swap_location_table`.
        *   Hàm `swap_location_table` thực hiện một transaction trong CSDL:
            1.  Đọc tên của bảng chỉ mục **cũ** từ bảng `LOCATIONS`.
            2.  Cập nhật con trỏ của địa điểm (`root_path`) để trỏ tới tên bảng **mới**.
            3.  Trả về tên của bảng **cũ**.
        *   Nhận lại tên bảng cũ, process `rescan_atomic_swap` ngay lập tức gửi yêu cầu xóa toàn bộ bảng cũ đó khỏi CSDL (`delete_table`).

---

## Workflow 3: Tìm Kiếm (Search) - Streaming

*   **Tên workflow trong code:** `gui_search`
*   **Mục đích:** Tìm kiếm và hiển thị kết quả trong thời gian thực khi chúng được tìm thấy.

**Sơ đồ xử lý:**
`search_index (search.rs)`

**Diễn giải chi tiết:**
1.  **Process: `processes::search::search_index`**
    *   Chuẩn hóa từ khóa tìm kiếm.
    *   Lặp qua từng địa điểm (`location`) cần tìm kiếm.
    *   Với mỗi địa điểm, gọi `db::DbManager::search_in_table` để lấy về một danh sách các đường dẫn tương đối khớp với từ khóa.
    *   **Bắt đầu streaming:** Lặp qua danh sách đường dẫn vừa tìm được.
        *   Chuyển đường dẫn tương đối thành tuyệt đối.
        *   Xử lý trước thông tin hiển thị: lấy icon (`utils::get_icon_for_path`) và đóng gói vào struct `DisplayResult`.
        *   Thêm `DisplayResult` vào một lô (batch) tạm thời.
        *   Khi lô đầy (ví dụ: 200 mục), gửi ngay lô này về cho luồng GUI qua thông điệp `GuiUpdate::SearchResultsBatch`.
    *   Sau khi duyệt qua tất cả các địa điểm, gửi nốt lô cuối cùng (nếu còn) và kết thúc bằng một thông điệp `GuiUpdate::SearchFinished`.
2.  **Giao diện (UI):**
    *   Khi nhận được các lô `DisplayResult`, giao diện sẽ nối chúng vào danh sách kết quả.
    *   Khi vẽ từng dòng, nó sẽ dùng logic cắt chuỗi (`truncate_path`) dựa trên một hằng số ước tính chiều rộng ký tự để đảm bảo mỗi kết quả chỉ chiếm một dòng, giúp việc cuộn danh sách luôn mượt mà.