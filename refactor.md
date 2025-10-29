# Tài liệu kiến trúc dự án DeepSearch

Tài liệu này mô tả chi tiết kiến trúc và luồng hoạt động của ứng dụng DeepSearch. Mục đích là cung cấp một cái nhìn tổng quan, rõ ràng cho việc bảo trì và phát triển các tính năng trong tương lai.

### **Mục tiêu đã đạt được**

*   **Kiến trúc Module hóa:** Tái cấu trúc thành công mã nguồn thành các module riêng biệt, linh hoạt theo mô hình Hướng quy trình (POP), giúp dễ quản lý và mở rộng.
*   **Hệ thống chỉ mục hiệu năng cao:** Tích hợp thành công cơ sở dữ liệu `redb` để xây dựng và quản lý chỉ mục, cho phép tìm kiếm gần như tức thì trên hàng triệu tệp.
*   **Tối ưu hóa Quét lại (Rescan):** Triển khai logic quét lại thông minh, chỉ xử lý các tệp đã thay đổi, thêm mới hoặc bị xóa, giúp tiết kiệm thời gian và tài nguyên hệ thống.
*   **Ứng dụng giao diện đồ họa (GUI):** Phát triển vượt kế hoạch ban đầu (một ứng dụng CLI) để xây dựng một ứng dụng GUI hoàn chỉnh, thân thiện với người dùng bằng `eframe` (egui).
*   **Hiệu năng cao:** Duy trì và tối ưu hóa việc xử lý song song bằng `rayon` trong các tác vụ nặng (quét file, tìm kiếm), đảm bảo giao diện người dùng luôn mượt mà.

### **Công nghệ sử dụng**

*   **`eframe` (egui):** Framework để xây dựng giao diện người dùng đồ họa (GUI).
*   **`rayon`**: Thư viện xử lý song song, được sử dụng để tăng tốc các tác vụ quét file và tìm kiếm.
*   **`walkdir`**: Duyệt cây thư mục một cách hiệu quả.
*   **`redb`**: Cơ sở dữ liệu nhúng dạng key-value hiệu năng cao, an toàn cho đa luồng, dùng để lưu trữ chỉ mục.
*   **`serde` & `bincode`**: Để serialize và deserialize dữ liệu (cụ thể là `FileMetadata`) trước khi lưu vào `redb`.
*   **`once_cell`**: Để khởi tạo các tài nguyên tốn kém một lần duy nhất (ví dụ: bảng chuyển đổi ký tự).
*   **`open`**: Mở file và thư mục theo mặc định của hệ điều hành.

---

### **Tổng quan kiến trúc**

Kiến trúc của DeepSearch được chia thành ba lớp chính: Lớp giao diện (GUI), Lớp xử lý lõi (POP), và Lớp dữ liệu (Database).

#### **1. Kiến trúc tổng thể: Giao diện (GUI) và Luồng xử lý (Worker)**

Ứng dụng hoạt động trên mô hình hai luồng để đảm bảo giao diện người dùng không bao giờ bị "đóng băng" khi thực hiện các tác vụ nặng.

*   **Luồng GUI (Main Thread):**
    *   Được quản lý bởi `eframe`.
    *   Chịu trách nhiệm vẽ toàn bộ giao diện người dùng, xử lý các sự kiện (click chuột, nhập phím), và quản lý trạng thái của UI (trong `DeepSearchApp`).
    *   Khi người dùng thực hiện một hành động (ví dụ: "Bắt đầu quét"), luồng GUI sẽ không tự thực hiện mà sẽ tạo một `Command` và gửi nó qua một kênh (`mpsc::channel`) cho Luồng xử lý.

*   **Luồng xử lý (Worker Thread):**
    *   Chạy ở chế độ nền, được khởi tạo một lần duy nhất khi ứng dụng bắt đầu.
    *   Luôn lắng nghe các `Command` được gửi đến từ luồng GUI.
    *   Chịu trách nhiệm thực thi tất cả các tác vụ nặng: quét thư mục, đọc/ghi cơ sở dữ liệu, tìm kiếm.
    *   Trong quá trình thực thi, nó sẽ gửi các cập nhật về trạng thái (`GuiUpdate`) trở lại luồng GUI để hiển thị tiến trình (ví dụ: thanh progress bar, thông báo trạng thái).

*   **Kênh giao tiếp (Communication Channel):**
    *   **`Command` (`src/gui/events.rs`):** Enum định nghĩa các lệnh mà luồng GUI có thể gửi cho luồng Worker (ví dụ: `StartInitialScan`, `StartSearch`, `DeleteLocation`).
    *   **`GuiUpdate` (`src/gui/events.rs`):** Enum định nghĩa các cập nhật mà luồng Worker có thể gửi về cho luồng GUI (ví dụ: `LocationsFetched`, `ScanProgress`, `SearchCompleted`, `Error`).

#### **2. Kiến trúc xử lý lõi: Hướng quy trình (Process-Oriented Programming - POP)**

Lớp này được triển khai trong module `src/pop` và là "bộ não" của các tác vụ xử lý.

1.  **`Context` (`pop/context.rs`):** Một `struct` trung tâm chứa toàn bộ dữ liệu và trạng thái cần thiết cho một chuỗi công việc. Nó được truyền qua và chỉnh sửa bởi mỗi bước trong một workflow.
2.  **`Process` (`pop/registry.rs`):** Một `type alias` cho một hàm độc lập, nhận vào một `Context` và trả về một `Result<Context>`. Mỗi `Process` chỉ thực hiện một nhiệm vụ duy nhất (ví dụ: `scan_directory_initial`, `write_index_to_db`).
3.  **`Registry` (`pop/registry.rs`):** Một `struct` chứa `HashMap` để đăng ký và lưu trữ tất cả các `Process` và `Workflow` có sẵn trong ứng dụng.
4.  **`Workflow`:** Một `Vec<String>` định nghĩa một chuỗi các tên của các `Process` sẽ được thực thi tuần tự.
5.  **`Engine` (`pop/engine.rs`):** Chứa hàm `run_workflow` nhận vào tên của một `Workflow` và một `Context`, sau đó tuần tự gọi các `Process` tương ứng đã đăng ký trong `Registry` để thực thi.

#### **3. Kiến trúc cơ sở dữ liệu với `redb` (`src/db.rs`)**

*   **Một file CSDL duy nhất:** Toàn bộ chỉ mục được lưu trong file `deepsearch_index.redb`.
*   **Bảng Quản lý Vị trí (`locations`):**
    *   **Mục đích:** Theo dõi tất cả các thư mục gốc đã được index.
    *   **Key:** Đường dẫn tuyệt đối đến thư mục gốc (ví dụ: `"C:\Users\YourUser\Documents"`).
    *   **Value:** Tên của bảng dữ liệu tương ứng (ví dụ: `"index_a1b2c3d4"`), được tạo ra bằng cách băm (MD5) đường dẫn gốc.
*   **Các Bảng Dữ liệu Index (ví dụ: `index_a1b2c3d4`):**
    *   Mỗi thư mục gốc có một bảng dữ liệu riêng.
    *   **Key:** Đường dẫn **tương đối** của file so với thư mục gốc (ví dụ: `"project-a\report.docx"`).
    *   **Value:** `struct FileMetadata` được serialize bằng `bincode`.
        ```rust
        #[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
        pub struct FileMetadata {
            pub normalized_name: String, // Tên file đã được chuẩn hóa (viết thường, bỏ dấu)
            pub modified_time: u64,      // Thời gian sửa đổi file (dưới dạng timestamp)
        }
        ```
*   **Đảm bảo toàn vẹn dữ liệu với Giao dịch nguyên tử (Atomic Transactions):**
    *   Một trong những tính năng quan trọng nhất của `redb` là hỗ trợ các giao dịch ghi có tính nguyên tử (atomic).
    *   Trong dự án, tất cả các chuỗi thao tác ghi (thêm, sửa, xóa) cho một tác vụ logic đều được gói gọn trong một giao dịch duy nhất, bắt đầu bằng `db.begin_write()?` và kết thúc bằng `txn.commit()?`.
    *   Điều này đảm bảo nguyên tắc "tất cả hoặc không có gì": Nếu có bất kỳ lỗi nào xảy ra trước khi `commit()` được gọi, toàn bộ các thay đổi trong giao dịch đó sẽ được hủy bỏ (rollback). Cơ sở dữ liệu sẽ không bao giờ rơi vào trạng thái hỏng hoặc không nhất quán, giúp bảo vệ toàn vẹn dữ liệu chỉ mục.

### **Cấu trúc thư mục `src`**

```
src/
│   db.rs           # Quản lý mọi tương tác với CSDL redb.
│   main.rs         # Điểm khởi đầu của ứng dụng, thiết lập eframe và các tài nguyên.
│   utils.rs        # Các hàm tiện ích, ví dụ: normalize_string.
│
├── gui/
│   │   app.rs      # Chứa struct DeepSearchApp, định nghĩa toàn bộ UI và logic trạng thái của GUI.
│   │   events.rs   # Định nghĩa các enum Command và GuiUpdate.
│   │   mod.rs
│
├── pop/
│   │   context.rs  # Định nghĩa struct Context.
│   │   engine.rs   # Định nghĩa Engine và hàm run_workflow.
│   │   mod.rs
│   │   registry.rs # Định nghĩa Process, Registry.
│
└── processes/
        index.rs    # Các process liên quan đến việc đọc/ghi chỉ mục vào CSDL.
        scan.rs     # Các process quét thư mục (initial và incremental).
        search.rs   # Process thực hiện tìm kiếm.
        └── ...         # Các file khác có thể trống (di sản từ phiên bản CLI).
```

### **Luồng hoạt động chi tiết (Workflows)**

Các workflow được định nghĩa và đăng ký trong `gui/app.rs`. Chúng được khởi chạy bởi luồng Worker khi nhận được `Command` tương ứng.

1.  **Quét lần đầu (Initial Scan)**
    *   **Kích hoạt:** Người dùng nhập đường dẫn mới và nhấn nút "Start Initial Scan".
    *   **Luồng:**
        1.  GUI gửi `Command::StartInitialScan(path)`.
        2.  Worker nhận lệnh, tạo `Context` và chạy workflow `gui_initial_scan`: `["scan_directory_initial", "write_index_to_db"]`.
        3.  `scan_directory_initial`: Quét toàn bộ thư mục (sử dụng chiến lược 2 giai đoạn song song) và lưu danh sách file vào `Context`. Gửi `GuiUpdate::ScanProgress` liên tục.
        4.  `write_index_to_db`: Tạo bảng mới trong `redb` và ghi toàn bộ danh sách file từ `Context` vào đó.
        5.  Worker gửi `GuiUpdate::ScanCompleted` khi hoàn tất.

2.  **Quét lại (Rescan)**
    *   **Kích hoạt:** Người dùng nhấn nút "Rescan" trên một vị trí đã được index.
    *   **Luồng:**
        1.  GUI gửi `Command::StartRescan(path)`.
        2.  Worker chạy workflow `gui_rescan`: `["load_existing_index", "scan_directory_incremental", "update_index_in_db"]`.
        3.  `load_existing_index`: Đọc toàn bộ chỉ mục cũ từ `redb` vào một `HashMap` trong `Context`.
        4.  `scan_directory_incremental`: Quét lại thư mục. So sánh từng file với `HashMap` đã tải để xác định file mới, file bị thay đổi, và file bị xóa.
        5.  `update_index_in_db`: Ghi lại các thay đổi (thêm, sửa, xóa) vào bảng tương ứng trong `redb`.
        6.  Worker gửi `GuiUpdate::ScanCompleted`.

3.  **Tìm kiếm (Search)**
    *   **Kích hoạt:** Người dùng nhập từ khóa, chọn phạm vi tìm kiếm và nhấn "Search" (hoặc Enter).
    *   **Luồng:**
        1.  GUI gửi `Command::StartSearch { locations, keyword }`.
        2.  Worker chạy workflow `gui_search`: `["search_index"]`.
        3.  `search_index`: Chuẩn hóa từ khóa. Duyệt qua các bảng dữ liệu được chọn trong `redb`, tìm kiếm song song các file có `normalized_name` chứa từ khóa.
        4.  Worker gửi `GuiUpdate::SearchCompleted(results)` với danh sách kết quả.

### **Hướng dẫn bảo trì và mở rộng**

*   **Để thêm một Process mới:**
    1.  Viết một hàm public mới trong một module phù hợp trong `src/processes/` (ví dụ: `my_new_process(Context) -> anyhow::Result<Context>`).
    2.  Vào `gui/app.rs`, trong khối `thread::spawn`, đăng ký process mới với `registry.register_process("my_new_process", processes::path::to::my_new_process);`.

*   **Để thêm một Workflow mới:**
    1.  Trong `gui/app.rs`, đăng ký workflow mới: `registry.register_workflow("my_new_workflow", vec!["process1".to_string(), "process2".to_string()]);`.
    2.  Tạo một `Command` mới (nếu cần) để kích hoạt workflow này từ GUI.
    3.  Trong `match command` của luồng Worker, gọi `engine.run_workflow("my_new_workflow", context)`.

*   **Để sửa đổi giao diện:**
    *   Toàn bộ logic vẽ UI nằm trong `impl eframe::App for DeepSearchApp` trong `src/gui/app.rs`.
    *   Các hàm `draw_indexing_tab` và `draw_search_tab` chịu trách nhiệm cho hai tab chính của ứng dụng.

*   **Lưu ý về xử lý lỗi:**
    *   Hạn chế tối đa việc sử dụng `.unwrap()` hoặc `.expect()`.
    *   Sử dụng toán tử `?` và `anyhow::Result` để cho phép lỗi được trả về một cách tường minh qua các lớp xử lý.
    *   Các lỗi xảy ra trong luồng Worker nên được bắt lại và gửi về GUI qua `GuiUpdate::Error(String)` để người dùng được thông báo.
