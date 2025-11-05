use eframe::egui;
use crate::gui::app::AppState;

#[derive(Default)]
pub struct StatusBar;

impl StatusBar {
    pub fn ui(&self, ui: &mut egui::Ui, state: &AppState) {
        ui.add_space(5.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(&state.current_status);
            if state.is_running_task && state.scan_progress > 0.0 && state.scan_progress < 1.0 {
                ui.add(egui::ProgressBar::new(state.scan_progress).show_percentage());
            }
        });
    }
}
