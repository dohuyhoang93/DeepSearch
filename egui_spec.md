
## 🧠 2. Nguyên tắc tách module trong egui

Mục tiêu:

> “Mỗi thành phần GUI là một module có **state riêng**, **hàm render riêng**, và chỉ expose API cần thiết.”

---

### Cấu trúc thư mục gợi ý

```
src/
 ├─ main.rs
 ├─ app/
 │   ├─ mod.rs
 │   ├─ dashboard.rs
 │   ├─ settings.rs
 │   ├─ logs.rs
 │   └─ chart.rs
 └─ ui/
     ├─ mod.rs
     ├─ sidebar.rs
     ├─ toolbar.rs
     └─ statusbar.rs
```

---

## 🧩 3. Mỗi module GUI nên gồm 2 phần

### ① State riêng

Ví dụ `dashboard.rs`:

```rust
use egui::Ui;

pub struct Dashboard {
    counter: u32,
}

impl Dashboard {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.heading("Dashboard");
        if ui.button("Increase").clicked() {
            self.counter += 1;
        }
        ui.label(format!("Counter: {}", self.counter));
    }
}
```

---

### ② Sử dụng trong `app.rs` (gọn gàng)

```rust
use crate::app::{Dashboard, Settings};

pub struct MyApp {
    dashboard: Dashboard,
    settings: Settings,
}

impl MyApp {
    pub fn new() -> Self {
        Self {
            dashboard: Dashboard::new(),
            settings: Settings::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("My Modular App");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.dashboard.ui(ui);
        });
    }
}
```

---

## 🧩 4. Khi cần chia layout phức tạp

Nếu ứng dụng có nhiều layout (ví dụ tab, popup, hoặc dynamic window):

* Dùng `egui::Window::new()` cho từng module:

  ```rust
  self.settings.ui(ctx);
  self.logs.ui(ctx);
  ```
* Mỗi module tự quyết định có hiển thị hay không (`visible` flag).

---

## 🧩 5. Khi có dữ liệu chung giữa các module

Tránh truyền tham chiếu chéo lẫn nhau.
Hãy tạo một **struct AppState** trung gian:

```rust
pub struct AppState {
    pub connection_status: bool,
    pub logs: Vec<String>,
}
```

→ Mỗi module nhận `&mut AppState` khi cần:

```rust
pub fn ui(&mut self, ui: &mut Ui, state: &mut AppState) {
    if ui.button("Reconnect").clicked() {
        state.connection_status = true;
    }
}
```

---

## 🧱 6. Lợi ích

| Ưu điểm        | Mô tả                                         |
| -------------- | --------------------------------------------- |
| Dễ refactor    | Chỉ sửa 1 module khi thêm/chỉnh tính năng.    |
| Dễ test        | Có thể test từng module UI riêng.             |
| Dễ mở rộng     | Thêm tab mới chỉ cần thêm file + gọi `.ui()`. |
| Dễ tái sử dụng | Module có thể tách sang project khác.         |

---

## 🔧 7. Khi project rất lớn

Bạn có thể nâng cấp lên **pattern kiểu ECS (Entity Component System)** hoặc **event bus**, ví dụ:

* Dùng `crossbeam-channel` để gửi event GUI ↔ logic.
* Mỗi module UI đăng ký nhận event mà không phụ thuộc lẫn nhau.

---
