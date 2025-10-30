# Tài liệu kiến trúc dự án DeepSearch

Tài liệu này mô tả chi tiết kiến trúc và luồng hoạt động của ứng dụng DeepSearch. Mục đích là cung cấp một cái nhìn tổng quan, rõ ràng cho việc bảo trì và phát triển các tính năng trong tương lai.

### **Mục tiêu đã đạt được**

*   **Kiến trúc Module hóa:** Tái cấu trúc thành công mã nguồn thành các module riêng biệt, linh hoạt theo mô hình Hướng quy trình (POP), giúp dễ quản lý và mở rộng.
*   **Hệ thống chỉ mục hiệu năng cao:** Tích hợp thành công cơ sở dữ liệu `redb` để xây dựng và quản lý chỉ mục, cho phép tìm kiếm gần như tức thì trên hàng triệu tệp.
*   **Tối ưu hóa Quét lại (Rescan):** Triển khai logic quét lại thông minh, chỉ xử lý các tệp đã thay đổi, thêm mới hoặc bị xóa, giúp tiết kiệm thời gian và tài nguyên hệ thống.
*   **Ứng dụng giao diện đồ họa (GUI):** Phát triển vượt kế hoạch ban đầu (một ứng dụng CLI) để xây dựng một ứng dụng GUI hoàn chỉnh, thân thiện với người dùng bằng `eframe` (egui).
*   **Hiệu năng cao:** Duy trì và tối ưu hóa việc xử lý song song bằng `rayon` trong các tác vụ nặng (quét file, tìm kiếm), đảm bảo giao diện người dùng luôn mượt mà.
*   **Xử lý dữ liệu lớn:** Tái cấu trúc thành công các quy trình cốt lõi (quét, quét lại, tìm kiếm) sang mô hình xử lý theo luồng (streaming) và theo lô (batching), giải quyết triệt để vấn đề tràn bộ nhớ khi làm việc với các chỉ mục hàng triệu file.

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
2.  **`Process` (`pop/registry.rs`):** Một `type alias` cho một hàm độc lập, nhận vào một `Context` và trả về một `Result<Context>`. Mỗi `Process` chỉ thực hiện một nhiệm vụ duy nhất (ví dụ: `scan_directory_streaming`, `write_index_from_stream_batched`).
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
*   **Toàn vẹn dữ liệu và Xử lý Dữ liệu Lớn (Sự đánh đổi)**
    *   Để có thể xử lý các chỉ mục cực lớn (hàng triệu file) mà không gây tràn bộ nhớ, ứng dụng sử dụng chiến lược ghi theo lô (batching).
    *   Mỗi lô (ví dụ: 50,000 file) được ghi vào CSDL trong một **giao dịch nguyên tử (atomic transaction)** riêng biệt. `redb` đảm bảo rằng mỗi lô này sẽ được ghi một cách "tất cả hoặc không có gì".
    *   **Sự đánh đổi:** Toàn bộ một tác vụ lớn (như "Initial Scan" hoặc "Rescan") **không phải là một giao dịch nguyên tử duy nhất**. Điều này có nghĩa là nếu ứng dụng bị sập giữa chừng, CSDL sẽ ở trạng thái "dở dang" (ví dụ: đã ghi được một nửa số file).
    *   **Giảm thiểu rủi ro:** Trạng thái "dở dang" này không làm hỏng file CSDL. Người dùng có thể dễ dàng khắc phục bằng cách chạy lại "Initial Scan" hoặc chạy "Rescan" để CSDL tự động tìm và bổ sung các file còn thiếu. Đây là sự đánh đổi cần thiết để có được khả năng xử lý dữ liệu lớn.

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

Các workflow được định nghĩa và đăng ký trong `gui/app.rs`. Chúng được khởi chạy bởi luồng Worker khi nhận được `Command` tương ứng. Kiến trúc này xử lý dữ liệu theo luồng (streaming) để đảm bảo sử dụng bộ nhớ hiệu quả.

1.  **Quét lần đầu (Initial Scan)**
    *   **Kích hoạt:** Người dùng nhập đường dẫn mới và nhấn nút "Start Initial Scan".
    *   **Workflow:** `["scan_directory_streaming", "write_index_from_stream_batched"]`
    *   **Luồng:**
        1.  GUI gửi `Command::StartInitialScan(path)`.
        2.  `scan_directory_streaming`: Chạy trong một luồng nền, quét toàn bộ thư mục và liên tục gửi dữ liệu file tìm thấy qua một `channel`. Process này hoàn thành gần như ngay lập tức, trả về `Context` chứa đầu nhận của `channel`.
        3.  `write_index_from_stream_batched`: Nhận dữ liệu từ `channel`, gom chúng thành từng lô (batch), và ghi mỗi lô vào `redb` trong một transaction riêng.
        4.  Worker gửi `GuiUpdate::ScanCompleted` khi hoàn tất.

2.  **Quét lại (Rescan)**
    *   **Kích hoạt:** Người dùng nhấn nút "Rescan" trên một vị trí đã được index.
    *   **Workflow:** `["find_and_apply_updates_streaming", "find_and_apply_deletions"]`
    *   **Luồng:**
        1.  GUI gửi `Command::StartRescan(path)`.
        2.  `find_and_apply_updates_streaming`: Quét hệ thống file, so sánh từng file với CSDL (dùng các truy vấn nhỏ, không tải toàn bộ CSDL). Các file mới/thay đổi được tìm thấy và ghi vào CSDL theo từng lô.
        3.  `find_and_apply_deletions`: Sử dụng một bảng CSDL tạm để xác định các file đã bị xóa khỏi hệ thống file, sau đó thực hiện xóa chúng khỏi chỉ mục chính theo từng lô.
        4.  Worker gửi `GuiUpdate::ScanCompleted`.

3.  **Tìm kiếm (Search)**
    *   **Kích hoạt:** Người dùng nhập từ khóa, chọn phạm vi tìm kiếm và nhấn "Search" (hoặc Enter).
    *   **Workflow:** `["search_index"]`
    *   **Luồng:**
        1.  GUI gửi `Command::StartSearch { locations, keyword }`.
        2.  `search_index`: Chuẩn hóa từ khóa. Yêu cầu `DbManager` tìm kiếm.
        3.  Bên trong `DbManager`, quá trình tìm kiếm được thực hiện song song theo luồng: nó duyệt qua CSDL trên đĩa, kiểm tra từng file, và chỉ thu thập các kết quả khớp vào bộ nhớ.
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
