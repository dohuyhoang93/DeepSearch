Chắc chắn rồi. Dựa trên yêu cầu của bạn, tôi đã xây dựng một kế hoạch chi tiết để tái cấu trúc dự án **DeepSearch** theo kiến trúc Hướng quy trình (POP), tích hợp `redb` làm cơ sở dữ liệu chỉ mục (index), và tối ưu hóa quá trình quét lại (rescan).

### **Mục tiêu**

*   **Tái cấu trúc mã nguồn** từ một file `main.rs` nguyên khối thành một kiến trúc module hóa, linh hoạt, và dễ bảo trì theo mô hình POP.
*   **Xây dựng hệ thống chỉ mục** bằng `redb` để tăng tốc độ tìm kiếm một cách đáng kể, hỗ trợ nhiều vị trí (ổ đĩa/thư mục) khác nhau.
*   **Tối ưu hóa việc quét lại** bằng cách chỉ quét các tệp đã thay đổi, thêm mới hoặc xóa bỏ, thay vì quét toàn bộ thư mục mỗi lần.
*   **Đảm bảo hiệu năng cao** bằng cách duy trì và tối ưu hóa việc xử lý song song trong các tác vụ nặng (quét file, đọc/ghi index).

### **Tổng quan kiến trúc POP**

Chúng ta sẽ mô phỏng lại kiến trúc từ file `pop.py` trong Rust:

1.  **`Context` (Ngữ cảnh):** Một `struct` trung tâm chứa toàn bộ dữ liệu và trạng thái của ứng dụng được truyền qua các bước xử lý.
2.  **`Process` (Quy trình):** Các hàm độc lập, mỗi hàm thực hiện một nhiệm vụ duy nhất (ví dụ: `lấy_đường_dẫn`, `quét_thư_mục`, `ghi_vào_index`).
3.  **`Workflow` (Luồng công việc):** Một danh sách các chuỗi tên của các `Process`.
4.  **`Engine` (Bộ điều khiển):** Một hàm `run_workflow` sẽ nhận vào một `Workflow` và `Context`, sau đó tuần tự gọi các `Process` tương ứng để thực thi.

### **Công nghệ sử dụng**

*   **`rayon`**: Tiếp tục sử dụng để song song hóa việc quét hệ thống tệp.
*   **`walkdir`**: Duyệt cây thư mục một cách hiệu quả.
*   **`redb`**: Cơ sở dữ liệu nhúng hiệu năng cao, an toàn cho đa luồng, dùng để lưu trữ index.
*   **`serde`**: Để serialize và deserialize dữ liệu trước khi lưu vào `redb`.
*   **`once_cell`**: Để khởi tạo các tài nguyên tốn kém một lần duy nhất.
*   **`colored`**: Giữ lại để làm đẹp giao diện CLI.

---

### **Kế hoạch chi tiết theo từng giai đoạn**

#### **Giai đoạn 1: Nền tảng - Cấu trúc lại dự án và thiết lập POP**

1.  **Tạo cấu trúc module mới:**
    *   `src/main.rs`: Chỉ chứa vòng lặp chính và gọi `Engine`.
    *   `src/pop/`: Module chứa các thành phần cốt lõi của kiến trúc POP (`context.rs`, `engine.rs`, `registry.rs`).
    *   `src/db.rs`: Module quản lý mọi tương tác với `redb`.
    *   `src/processes/`: Thư mục chứa các file định nghĩa `Process` (`input.rs`, `scan.rs`, `index.rs`, `search.rs`).
    *   `src/utils.rs`: Chứa các hàm tiện ích như `normalize_string`.

2.  **Chiến lược quản lý Index và Schema (Nâng cao):**
    Chúng ta sẽ sử dụng một **file CSDL `redb` trung tâm duy nhất** (ví dụ: `%APPDATA%\DeepSearch\index.redb`) để quản lý tất cả các vị trí được index. Bên trong file này sẽ có nhiều **bảng (tables)**:

    *   **Bảng Quản lý Vị trí (`locations_table`):**
        *   **Mục đích:** Theo dõi tất cả các thư mục gốc đã được index.
        *   **Key:** Đường dẫn tuyệt đối đến thư mục gốc (ví dụ: `"D:\Work"`).
        *   **Value:** Một ID định danh duy nhất cho thư mục đó (ví dụ: `"idx_a1b2c3"`).

    *   **Các Bảng Dữ liệu Index (ví dụ: `idx_a1b2c3`):**
        *   Mỗi thư mục gốc sẽ có một bảng dữ liệu riêng.
        *   **Key:** Đường dẫn **tương đối** của file so với thư mục gốc (ví dụ: `"project-a\docs\report.docx"`).
        *   **Value:** `struct FileMetadata` được định nghĩa như sau:
            ```rust
            use serde::{Serialize, Deserialize};
            
            #[derive(Serialize, Deserialize, Debug, Clone)]
            pub struct FileMetadata {
                pub normalized_name: String,
                pub modified_time: u64, // Thời gian sửa đổi file (dưới dạng timestamp)
            }
            ```

3.  **Tối ưu hóa tìm kiếm và đặc tính của `redb`:**
    *   `redb` sử dụng cấu trúc **B-Tree**, có nghĩa là nó **tự động lưu trữ các key theo thứ tự alphabet**.
    *   Chúng ta **không cần** làm gì thêm để có được sự tối ưu này.
    *   **Lợi ích:** Việc tìm kiếm theo tiền tố (ví dụ: tìm tất cả file bắt đầu bằng "report") và quét theo khoảng sẽ cực kỳ nhanh chóng và hiệu quả.

4.  **Lưu ý quan trọng về Tối ưu hóa Quét:**
    *   Kiến trúc refactor này sẽ **bảo tồn và tích hợp** chiến lược quét song song hai giai đoạn tinh vi từ `main.rs` gốc. Thay vì dùng một `WalkDir` duy nhất, chiến lược này chủ động 'chia để trị' bằng cách khởi chạy các luồng quét song song riêng biệt cho các thư mục con lớn. Điều này đặc biệt hiệu quả trên ổ đĩa mạng (SMB) và sẽ được triển khai bên trong các process `scan_directory_*`.

#### **Giai đoạn 2: Xây dựng Workflow cho Lần quét đầu tiên (Initial Scan)**

*   **Workflow:** `["get_target_directory", "scan_directory_initial", "write_index_to_db", "display_summary"]`

1.  **Process `get_target_directory`:** Lấy đường dẫn thư mục gốc từ người dùng và lưu vào `Context`.

2.  **Process `scan_directory_initial`:**
    *   Triển khai lại **chiến lược quét song song hai giai đoạn** hiệu quả từ `main.rs` gốc:
        1.  **Giai đoạn 1:** Quét song song các file/thư mục ở cấp 1.
        2.  **Giai đoạn 2:** Thu thập các thư mục con và khởi chạy một luồng quét song song riêng biệt cho mỗi thư mục con đó (`par_iter`).
    *   Process sẽ thu thập danh sách các cặp `(đường_dẫn_tương_đối, FileMetadata)` vào `Context`.

3.  **Process `write_index_to_db`:**
    *   Mở một **write transaction**.
    *   Tra cứu trong `locations_table` để lấy ID bảng dữ liệu, hoặc tạo mới nếu chưa có.
    *   Mở bảng dữ liệu tương ứng và ghi toàn bộ danh sách đã thu thập ở bước trên.

4.  **Process `display_summary`:** In ra thông báo hoàn thành.

#### **Giai đoạn 3: Xây dựng Workflow cho Quét lại (Rescan) và Tìm kiếm**

*   **Workflow Rescan:** `["get_target_directory", "load_existing_index", "scan_directory_incremental", "update_index_in_db", "display_summary"]`

1.  **Process `load_existing_index`:**
    *   Dựa vào đường dẫn gốc, tra cứu `locations_table` để tìm ID bảng dữ liệu.
    *   Mở **read transaction** và đọc toàn bộ nội dung bảng dữ liệu đó vào một `HashMap` trong `Context`.

2.  **Process `scan_directory_incremental`:**
    *   Sử dụng lại **chiến lược quét song song hai giai đoạn** để duyệt cây thư mục.
    *   Với mỗi file tìm thấy, process sẽ so sánh `modified_time` với dữ liệu trong `HashMap` đã tải.
    *   Phân loại các file thành 3 danh sách trong `Context`: `files_to_add`, `files_to_update`, và `files_to_delete` (những file còn sót lại trong `HashMap` sau khi quét xong).

3.  **Process `update_index_in_db`:**
    *   Mở một **write transaction** và mở đúng bảng dữ liệu.
    *   Thực hiện 3 thao tác: thêm, cập nhật, và xóa các file tương ứng.

*   **Workflow Search:** `["get_search_keyword", "select_search_scope", "search_index", "display_results"]`

1.  **Process `get_search_keyword`:** Lấy từ khóa tìm kiếm.
2.  **Process `select_search_scope`:** Hỏi người dùng muốn tìm ở một vị trí cụ thể hay trên tất cả các vị trí đã index.
3.  **Process `search_index`:**
    *   Mở **read transaction**.
    *   Dựa vào lựa chọn của người dùng, duyệt qua một bảng dữ liệu cụ thể hoặc tất cả các bảng dữ liệu.
    *   Sử dụng `par_bridge()` để song song hóa việc tìm kiếm.
    *   Thu thập kết quả vào `Context`.
4.  **Process `display_results`:** In kết quả.

#### **Giai đoạn 4: Hoàn thiện và Tối ưu**

1.  **Tích hợp `main.rs`:** Viết lại `main.rs` để trở nên gọn nhẹ, chỉ đóng vai trò điều phối, chọn `Workflow` và khởi chạy `Engine`.
2.  **Xử lý lỗi:** Thay thế toàn bộ `.unwrap()`, `.expect()` bằng cách xử lý `Result` tường minh (ví dụ: dùng `anyhow` hoặc `thiserror`).
3.  **Tối ưu hóa:** Đảm bảo các tài nguyên dùng chung được khởi tạo một lần bằng `once_cell::sync::Lazy`.
4.  **Giao diện người dùng:** Cải tiến các tính năng tương tác như tạm dừng/tiếp tục/dừng.

Kế hoạch này sẽ biến đổi dự án của bạn thành một ứng dụng mạnh mẽ, có cấu trúc tốt, hiệu năng cao và sẵn sàng để mở rộng trong tương lai. Bạn có muốn tôi bắt đầu thực hiện giai đoạn đầu tiên không?