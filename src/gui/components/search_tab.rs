use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eframe::egui;
use crate::gui::app::AppState;
use crate::gui::events::{Command, DisplayResult, LiveSearchResult};
use crate::pop::control::TaskController;

pub struct SearchTab {
    pub search_keyword: String,
    pub search_scope: HashMap<String, bool>,
    pub search_results: Vec<DisplayResult>,
    pub live_search_path_input: String,
    pub live_search_results: Vec<LiveSearchResult>,
    pub is_live_search_active: bool,
    pub live_search_in_content: bool,
    pub search_in_pdf: bool,
    pub search_in_office: bool,
    pub search_in_plain_text: bool,
}

impl Default for SearchTab {
    fn default() -> Self {
        Self {
            search_keyword: "".to_owned(),
            search_scope: HashMap::new(),
            search_results: vec![],
            live_search_path_input: "".to_owned(),
            live_search_results: vec![],
            is_live_search_active: false,
            live_search_in_content: false,
            search_in_pdf: true,
            search_in_office: true,
            search_in_plain_text: true,
        }
    }
}

impl SearchTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState, command_sender: &Sender<Command>) {
        // --- Top controls (Live Search, Path, etc.) ---
        ui.add_enabled_ui(!state.is_running_task, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.is_live_search_active, "Live Search in Folder");
                });

                if self.is_live_search_active {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.live_search_in_content, "Search in file content");
                    });

                    ui.add_enabled_ui(self.live_search_in_content, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Include:");
                            ui.checkbox(&mut self.search_in_pdf, "PDFs");
                            ui.checkbox(&mut self.search_in_office, "Office Files");
                            ui.checkbox(&mut self.search_in_plain_text, "Plain Text");
                        });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Path:");
                        let text_edit = egui::TextEdit::singleline(&mut self.live_search_path_input).hint_text("C:\\Users\\YourUser\\Documents");
                        ui.add(text_edit);

                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.live_search_path_input = path.display().to_string();
                            }
                        }
                    });
                }
            });
        });

        ui.add_space(10.0);

        // --- Search Bar and Control Buttons ---
        ui.horizontal(|ui| {
            ui.label("Keyword:");
            ui.label("🔍");
            let response = ui.add_enabled(!state.is_running_task, egui::TextEdit::singleline(&mut self.search_keyword));
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.trigger_search(state, command_sender);
            }
            if ui.add_enabled(!state.is_running_task, egui::Button::new("Search")).clicked() {
                self.trigger_search(state, command_sender);
            }

            if state.is_running_task && self.is_live_search_active {
                if ui.button("Stop").clicked() {
                    if let Some(controller) = &state.active_task_control {
                        controller.cancel();
                    }
                    state.is_running_task = false;
                    state.is_paused = false;
                    state.current_status = "Search stopped.".to_string();
                }

                if state.is_paused {
                    if ui.button("Resume").clicked() {
                        if let Some(controller) = &state.active_task_control {
                            controller.resume();
                        }
                        state.is_paused = false;
                        state.current_status = "Resuming search...".to_string();
                    }
                } else if ui.button("Pause").clicked() {
                    if let Some(controller) = &state.active_task_control {
                        controller.pause();
                    }
                    state.is_paused = true;
                    state.current_status = "Search paused.".to_string();
                }
            }
        });

        ui.add_space(10.0);

        // --- Results Panels ---
        egui::SidePanel::left("search_scope_panel")
            .resizable(true)
            .default_width(250.0)
            .max_width(400.0)
            .show_inside(ui, |ui| {
                ui.add_enabled_ui(!state.is_running_task, |ui| {
                    ui.label(egui::RichText::new("Search In:").strong());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (path, _, _) in &state.locations {
                            if let Some(is_selected) = self.search_scope.get_mut(path) {
                                ui.checkbox(is_selected, path);
                            }
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label(egui::RichText::new("Results:").strong());

            if self.is_live_search_active && self.live_search_in_content {
                self.draw_live_search_results(ui, state, command_sender);
            } else {
                self.draw_indexed_search_results(ui, state, command_sender);
            }
        });
    }

    fn trigger_search(&mut self, state: &mut AppState, command_sender: &Sender<Command>) {
        if !self.search_keyword.is_empty() {
            let selected_locations: Vec<_> = state.locations.iter()
                .filter(|(path, _, _)| *self.search_scope.get(path).unwrap_or(&false))
                .map(|(path, table_name, _)| (path.clone(), table_name.clone()))
                .collect();

            if !selected_locations.is_empty() || self.is_live_search_active {
                self.search_results.clear();
                self.live_search_results.clear();
                state.is_running_task = true;
                state.is_paused = false;

                let controller = TaskController::new();
                state.active_task_control = Some(controller.clone());

                command_sender.send(Command::StartSearch {
                    locations: selected_locations,
                    keyword: self.search_keyword.clone(),
                    is_live_search_active: self.is_live_search_active,
                    live_search_path: if self.is_live_search_active && !self.live_search_path_input.is_empty() {
                        Some(PathBuf::from(&self.live_search_path_input))
                    } else {
                        None
                    },
                    search_in_content: self.live_search_in_content,
                    search_in_pdf: self.search_in_pdf,
                    search_in_office: self.search_in_office,
                    search_in_plain_text: self.search_in_plain_text,
                    task_controller: controller,
                }).unwrap();
            } else {
                state.current_status = "Please select at least one location to search in.".to_string();
            }
        }
    }

    fn draw_live_search_results(&self, ui: &mut egui::Ui, state: &AppState, command_sender: &Sender<Command>) {
        if self.live_search_results.is_empty() && !state.is_running_task {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if self.search_keyword.is_empty() {
                    ui.label("Enter a keyword to begin live searching.");
                } else {
                    ui.label(format!("No live results found for '{}'", self.search_keyword));
                }
            });
        } else {
            let text_height = ui.text_style_height(&egui::TextStyle::Body);
            egui::ScrollArea::vertical().show_rows(ui, text_height, self.live_search_results.len(), |ui, row_range| {
                for i in row_range {
                    if let Some(result) = self.live_search_results.get(i) {
                        let display_text = if result.file_path.ends_with(".pdf") {
                            format!("{} [Page {}] - {}", result.file_path, result.line_number, result.line_content)
                        } else {
                            format!("{} [Line {}] - {}", result.file_path, result.line_number, result.line_content)
                        };
                        egui::Frame::NONE.stroke(egui::Stroke::NONE).show(ui, |ui| {
                            let response = ui.selectable_label(false, display_text)
                                .on_hover_text(&result.file_path);

                            response.context_menu(|ui| {
                                if ui.button("Open File").clicked() {
                                    command_sender.send(Command::OpenFile(result.file_path.clone())).unwrap();
                                    ui.close();
                                }
                                if ui.button("Open File Location").clicked() {
                                    command_sender.send(Command::OpenLocation(result.file_path.clone())).unwrap();
                                    ui.close();
                                }
                            });
                        });
                    }
                }
            });
        }
    }

    fn draw_indexed_search_results(&self, ui: &mut egui::Ui, _state: &AppState, command_sender: &Sender<Command>) {
        if self.search_results.is_empty() && !_state.is_running_task {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if self.search_keyword.is_empty() {
                    ui.label("Enter a keyword to begin searching.");
                } else {
                    ui.label(format!("No results found for '{}'", self.search_keyword));
                }
            });
        } else {
            let text_height = ui.text_style_height(&egui::TextStyle::Body);
            const WIDE_CHAR_APPROX_WIDTH: f32 = 10.0;

            egui::ScrollArea::vertical().show_rows(ui, text_height, self.search_results.len(), |ui, row_range| {
                let available_width = ui.available_width();
                let num_chars_to_keep = (available_width / WIDE_CHAR_APPROX_WIDTH).floor() as usize;

                for i in row_range {
                    if let Some(result) = self.search_results.get(i) {
                        let truncated_path = self.truncate_path(&result.full_path, num_chars_to_keep);
                        let display_text = format!("{} {}", result.icon, truncated_path);
                        egui::Frame::NONE.stroke(egui::Stroke::NONE).show(ui, |ui| {
                            let response = ui.selectable_label(false, display_text)
                                .on_hover_text(&*result.full_path);

                            response.context_menu(|ui| {
                                if ui.button("Open File").clicked() {
                                    command_sender.send(Command::OpenFile(result.full_path.to_string())).unwrap();
                                    ui.close();
                                }
                                if ui.button("Open File Location").clicked() {
                                    command_sender.send(Command::OpenLocation(result.full_path.to_string())).unwrap();
                                    ui.close();
                                }
                            });
                        });
                    }
                }
            });
        }
    }

    fn truncate_path(&self, path: &str, max_chars: usize) -> String {
        if path.chars().count() <= max_chars {
            return path.to_string();
        }
        if max_chars <= 5 {
            return "...".to_string();
        }

        let truncated_chars: String = path.chars().rev().take(max_chars.saturating_sub(4)).collect();
        let truncated_path = truncated_chars.chars().rev().collect::<String>();

        format!("...{}", truncated_path)
    }
}
