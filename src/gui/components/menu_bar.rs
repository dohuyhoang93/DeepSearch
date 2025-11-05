use eframe::egui;

#[derive(Default)]
pub struct MenuBar {
    pub show_about_window: bool,
}


impl MenuBar {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.show_about_window = true;
                    ui.close();
                }
            });
        });

        if self.show_about_window {
            egui::Window::new("About DeepSearch")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("DeepSearch");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(10.0);
                        ui.label("Developed by Do Huy Hoang");
                        ui.hyperlink("https://github.com/dohuyhoang93/DeepSearch");
                        ui.add_space(10.0);
                        if ui.button("Close").clicked() {
                            self.show_about_window = false;
                        }
                    });
                });
        }
    }
}
