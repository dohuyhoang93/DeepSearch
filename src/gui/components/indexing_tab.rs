use std::path::PathBuf;
use std::sync::mpsc::Sender;
use eframe::egui;
use crate::gui::app::AppState;
use crate::gui::events::Command;

pub struct IndexingTab {
    pub target_path_input: String,
    pub confirming_delete: Option<String>,
}

impl Default for IndexingTab {
    fn default() -> Self {
        Self {
            target_path_input: "".to_owned(),
            confirming_delete: None,
        }
    }
}

impl IndexingTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState, command_sender: &Sender<Command>) {
        // --- Top section for adding a new path ---
        ui.add_enabled_ui(!state.is_running_task, |ui| {
            ui.horizontal(|ui| {
                ui.label("Path to Index:");
                let text_edit = egui::TextEdit::singleline(&mut self.target_path_input)
                    .hint_text(r"C:\Users\YourUser\Documents");
                ui.add(text_edit);

                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.target_path_input = path.display().to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Scan & Index").clicked() {
                    if !self.target_path_input.is_empty() {
                        state.is_running_task = true;
                        state.scan_progress = 0.0;
                        state.current_status = "Starting scan...".to_string();
                        let path = PathBuf::from(&self.target_path_input);
                        command_sender.send(Command::StartInitialScan(path)).unwrap();
                    } else {
                        state.current_status = "Please select a path to index.".to_string();
                    }
                }
            });
        });

        // --- Section for listing indexed locations ---
        ui.label(egui::RichText::new("Indexed Locations").strong());

        let mut dialog_result: Option<bool> = None;
        if let Some(path) = &self.confirming_delete {
            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Are you sure you want to delete the index for '{}'?", path));
                    ui.horizontal(|ui| {
                        if ui.button("Yes, Delete").clicked() {
                            dialog_result = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            dialog_result = Some(false);
                        }
                    });
                });
        }

        match dialog_result {
            Some(true) => {
                if let Some(path) = self.confirming_delete.take() {
                    command_sender.send(Command::DeleteLocation(path)).unwrap();
                }
            }
            Some(false) => {
                self.confirming_delete = None;
            }
            None => {}
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            if state.locations.is_empty() {
                ui.label("No locations have been indexed yet.");
            } else {
                for (path, _table_name, count) in &state.locations {
                    let item_frame = egui::Frame::default()
                        .inner_margin(5.0)
                        .stroke(egui::Stroke::NONE)
                        .fill(ui.style().visuals.widgets.inactive.bg_fill);

                    item_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("📁 {}", path));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗑").on_hover_text("Delete Index").clicked() {
                                    self.confirming_delete = Some(path.clone());
                                }
                                if ui.button("🔄").on_hover_text("Rescan").clicked() {
                                    state.is_running_task = true;
                                    state.scan_progress = 0.0;
                                    state.current_status = format!("Rescanning {}...", path);
                                    command_sender.send(Command::StartRescan(PathBuf::from(path))).unwrap();
                                }
                                ui.label(format!("({} files)", count));
                            });
                        });
                    });
                }
            }
        });
    }
}
