# Tầm nhìn và Lộ trình Phát triển Dự án DeepSearch

## Tài liệu định hướng kiến trúc cho phiên bản 2.0 và xa hơn

### 1. Tầm nhìn Dài hạn (Long-Term Vision)

DeepSearch không chỉ là một công cụ tìm kiếm file. Tầm nhìn của dự án là trở thành **ứng dụng tham chiếu (benchmark)** cho việc xây dựng các phần mềm máy tính để bàn hiệu năng cực cao, an toàn và đa nền tảng bằng Rust. Dự án sẽ là một minh chứng cho sức mạnh của các công nghệ xử lý song song, bất đồng bộ và an toàn bộ nhớ mà Rust cung cấp.

**Mục tiêu cuối cùng:** Trở thành một tiện ích tìm kiếm tiêu chuẩn, được tin cậy, đóng gói sẵn trong các kho phần mềm của các bản phân phối Linux lớn và được cộng đồng biết đến như một ví dụ điển hình về chất lượng và hiệu năng của phần mềm Rust.

### 2. Các Trụ cột Công nghệ và Nguyên tắc Kiến trúc

Để đạt được tầm nhìn trên, mọi sự phát triển trong tương lai phải tuân thủ các nguyên tắc sau:

**a. Hiệu năng Cực cao (Extreme Performance):**
*   **Ưu tiên Bất đồng bộ (Async First):** Mọi tác vụ liên quan đến I/O (quét file, đọc/ghi CSDL) phải được chuyển đổi hoàn toàn sang mô hình `async/await` với `tokio`. Điều này giúp giải phóng luồng xử lý, cho phép ứng dụng xử lý hàng ngàn tác vụ đồng thời với mức sử dụng tài nguyên tối thiểu, đặc biệt hiệu quả khi quét các ổ đĩa mạng (network shares).
*   **Tận dụng CPU Triệt để:** Giữ lại và tối ưu hóa `rayon` cho các tác vụ thuần túy CPU-bound (ví dụ: xử lý dữ liệu sau khi đã đọc, so khớp phức tạp).
*   **Zero-Cost Abstractions:** Tiếp tục sử dụng các đặc tính mạnh mẽ của Rust (traits, generics) để xây dựng các thành phần tái sử dụng mà không gây ảnh hưởng đến hiệu năng lúc chạy.

**b. An toàn và Bền bỉ (Safety and Robustness):**
*   **An toàn bộ nhớ tuyệt đối:** Cam kết 100% mã nguồn an toàn (`safe code`). Mọi việc sử dụng `unsafe` (nếu có) phải được chứng minh là tuyệt đối cần thiết và được đóng gói trong một API an toàn.
*   **Zero Panic:** Loại bỏ hoàn toàn các lệnh `.unwrap()` và `.expect()` trong mã nguồn của phiên bản release. Mọi lỗi đều phải được xử lý một cách tường minh thông qua kiểu `Result`.
*   **Toàn vẹn dữ liệu:** Tiếp tục sử dụng và phát huy cơ chế giao dịch nguyên tử (atomic transactions) của `redb` cho mọi thao tác ghi, đảm bảo CSDL không bao giờ bị hỏng.
*   **Logging chuyên sâu:** Tích hợp thư viện `tracing` để cung cấp hệ thống logging có cấu trúc, thay thế cho việc chỉ in lỗi ra màn hình.

**c. Kiến trúc linh hoạt và có thể mở rộng:**
*   **Tách biệt Lõi và Giao diện:** Tái cấu trúc dự án để tách phần lõi xử lý (quét, chỉ mục, tìm kiếm) thành một crate thư viện độc lập (`deepsearch-core`). Crate ứng dụng GUI hiện tại sẽ trở thành một người dùng của thư viện này. Kiến trúc này mở đường cho các giao diện khác trong tương lai (CLI, web service, ...).
*   **Mô hình Actor hoặc Service:** Trong tương lai xa, phần lõi có thể được thiết kế như một `daemon` hoặc `service` chạy nền. Ứng dụng GUI sẽ giao tiếp với service này qua RPC (ví dụ: gRPC hoặc tonic). Điều này cho phép việc quét và cập nhật chỉ mục diễn ra liên tục mà không cần mở cửa sổ GUI.

### 3. Lộ trình Phát triển Tiếp theo (Next Development Roadmap)

**Giai đoạn 1: Chuyển đổi sang `async` và Hiện đại hóa nền tảng**
1.  **Tích hợp Tokio:** Đưa `tokio` vào làm runtime bất đồng bộ chính.
2.  **Refactor Kênh giao tiếp:** Chuyển đổi kênh `mpsc` hiện tại sang `tokio::mpsc`.
3.  **Async hóa Processes:** Viết lại các `Process` trong `src/processes` thành các hàm `async fn`. Sử dụng `tokio::fs` cho các thao tác đọc/ghi file không bị chặn (non-blocking).
4.  **Async hóa CSDL:** Đảm bảo các truy vấn CSDL với `redb` được thực hiện một cách không bị chặn, có thể bằng cách bọc các lệnh gọi trong `tokio::task::spawn_blocking`.
5.  **Loại bỏ Panic:** Rà soát và loại bỏ toàn bộ `.unwrap()`/`.expect()` và thay thế bằng `anyhow::Result` hoặc các kiểu lỗi tùy chỉnh.

**Giai đoạn 2: Tách lõi và Mở rộng Tính năng Tìm kiếm**
1.  **Tạo `deepsearch-core`:** Tách toàn bộ logic backend (modules `db`, `pop`, `processes`, `utils`) ra một crate thư viện riêng.
2.  **Tìm kiếm Nội dung File:** Triển khai tính năng quét nội dung file. Cần nghiên cứu các thư viện lõi của `ripgrep` hoặc các kỹ thuật streaming hiệu quả để đọc file mà không chiếm nhiều bộ nhớ.
3.  **Hỗ trợ Regex/Wildcard:** Tích hợp các thư viện như `regex` hoặc `glob` vào logic tìm kiếm.

**Giai đoạn 3: Tích hợp Hệ thống và Đóng gói**
1.  **Xây dựng Daemon/Service:** Tạo một phiên bản `deepsearchd` chạy nền, liên tục theo dõi và cập nhật chỉ mục.
2.  **Đóng gói cho các Distro:** Viết các kịch bản đóng gói cho các trình quản lý package phổ biến (Homebrew cho macOS, `PKGBUILD` cho Arch Linux, tạo file `.deb` cho Debian/Ubuntu, `winget` cho Windows).
3.  **Tự động cập nhật:** Tích hợp cơ chế tự động kiểm tra và thông báo phiên bản mới.

### 4. Tiêu chuẩn Đóng góp (Contribution Standards)

Để đảm bảo chất lượng và tính nhất quán của dự án trong dài hạn, mọi đóng góp phải tuân thủ:

1.  Mã nguồn phải được định dạng bằng `rustfmt`.
2.  Mã nguồn phải vượt qua `cargo clippy -- -D warnings` (coi tất cả cảnh báo là lỗi).
3.  Mọi tính năng mới phải đi kèm với test đơn vị (unit tests) hoặc test tích hợp (integration tests).
4.  Tất cả các API public phải được viết tài liệu (documentation comments).