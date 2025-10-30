use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use eframe::egui;
use crate::db::DbManager;
use crate::pop::context::Context;
use crate::pop::engine::Engine;
use crate::pop::registry::Registry;
use crate::processes;
use super::events::{Command, GuiUpdate};

// --- App State ---

#[derive(PartialEq)]
enum Tab {
    Indexing,
    Search,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DeepSearchApp {
    // --- Persisted State ---
    dark_mode: bool,
    background_alpha: u8,

    // --- Runtime State (won't be saved) ---
    #[serde(skip)]
    command_sender: Sender<Command>,
    #[serde(skip)]
    update_receiver: Receiver<GuiUpdate>,
    #[serde(skip)]
    active_tab: Tab,
    #[serde(skip)]
    target_path_input: String,
    #[serde(skip)]
    current_status: String,
    #[serde(skip)]
    scan_progress: f32,
    #[serde(skip)]
    is_running_task: bool,
    #[serde(skip)]
    confirming_delete: Option<String>,
    #[serde(skip)]
    show_about_window: bool,
    #[serde(skip)]
    pub background_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    locations: Vec<(String, String)>,
    #[serde(skip)]
    search_keyword: String,
    #[serde(skip)]
    search_scope: HashMap<String, bool>,
    #[serde(skip)]
    search_results: Vec<String>,
}

impl Default for DeepSearchApp {
    fn default() -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (update_sender, update_receiver) = mpsc::channel();

        // --- Spawn the Worker Thread ---
        thread::spawn(move || {
            let mut registry = Registry::new();

            // Register all processes
            registry.register_process("scan_directory_streaming", processes::scan::scan_directory_streaming);
            registry.register_process("write_index_from_stream_batched", processes::index::write_index_from_stream_batched);
            registry.register_process("find_and_apply_updates_streaming", processes::scan::find_and_apply_updates_streaming);
            registry.register_process("find_and_apply_deletions", processes::index::find_and_apply_deletions);
            registry.register_process("search_index", processes::search::search_index);

            // Register GUI-specific workflows
            registry.register_workflow("gui_initial_scan", vec!["scan_directory_streaming".to_string(), "write_index_from_stream_batched".to_string()]);
            registry.register_workflow("gui_rescan", vec!["find_and_apply_updates_streaming".to_string(), "find_and_apply_deletions".to_string()]);
            registry.register_workflow("gui_search", vec!["search_index".to_string()]);

            let engine = Engine::new(registry);
            let db_path = PathBuf::from("deepsearch_index.redb");

            // The worker loop
            for command in command_receiver {
                let mut context = Context {
                    progress_reporter: Some(update_sender.clone()),
                    db_path: Some(db_path.clone()),
                    ..Default::default()
                };

                match command {
                    Command::FetchLocations => {
                        if let Ok(db_manager) = DbManager::new(&db_path) {
                            if let Ok(locations) = db_manager.get_all_locations() {
                                update_sender.send(GuiUpdate::LocationsFetched(locations)).unwrap();
                            }
                        }
                    }
                    Command::OpenFile(path) => {
                        if let Err(e) = open::that(&path) {
                            update_sender.send(GuiUpdate::Error(format!("Failed to open file {}: {}", path, e))).unwrap();
                        }
                    }
                    Command::OpenLocation(path) => {
                        let parent_dir = Path::new(&path).parent().unwrap_or_else(|| Path::new("."));
                        if let Err(e) = open::that(parent_dir) {
                            update_sender.send(GuiUpdate::Error(format!("Failed to open location for {}: {}", path, e))).unwrap();
                        }
                    }
                    Command::DeleteLocation(path) => {
                        if let Ok(db_manager) = DbManager::new(&db_path) {
                            if let Err(e) = db_manager.delete_location(&path) {
                                update_sender.send(GuiUpdate::Error(format!("Failed to delete {}: {}", path, e))).unwrap();
                            }
                        }
                        update_sender.send(GuiUpdate::ScanCompleted(0)).unwrap();
                    }
                    Command::StartInitialScan(path) => {
                        context.target_path = Some(path);
                        match engine.run_workflow("gui_initial_scan", context) {
                            Ok(final_context) => {
                                update_sender.send(GuiUpdate::ScanCompleted(final_context.files_found_count)).unwrap();
                            }
                            Err(e) => update_sender.send(GuiUpdate::Error(e.to_string())).unwrap(),
                        }
                    }
                    Command::StartRescan(path) => {
                        context.target_path = Some(path.into());
                         match engine.run_workflow("gui_rescan", context) {
                            Ok(_) => {
                                update_sender.send(GuiUpdate::ScanCompleted(0)).unwrap();
                            }
                            Err(e) => update_sender.send(GuiUpdate::Error(e.to_string())).unwrap(),
                        }
                    }
                    Command::StartSearch { locations, keyword } => {
                        context.search_locations = locations;
                        context.search_keyword = Some(keyword);
                        match engine.run_workflow("gui_search", context) {
                            Ok(final_context) => {
                                update_sender.send(GuiUpdate::SearchCompleted(final_context.search_results)).unwrap();
                            }
                            Err(e) => update_sender.send(GuiUpdate::Error(e.to_string())).unwrap(),
                        }
                    }
                }
            }
        });

        command_sender.send(Command::FetchLocations).unwrap();

        Self {
            command_sender,
            update_receiver,
            active_tab: Tab::Indexing,
            dark_mode: true,
            background_alpha: 210, // Default alpha
            target_path_input: "".to_string(),
            current_status: "Ready. Fetching locations...".to_string(),
            scan_progress: 0.0,
            is_running_task: false,
            confirming_delete: None,
            show_about_window: false,
            background_texture: None,
            locations: vec![],
            search_keyword: "".to_string(),
            search_scope: HashMap::new(),
            search_results: vec![],
        }
    }
}

impl eframe::App for DeepSearchApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Custom visuals and background drawing ---
        let mut visuals = if self.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };
        if self.background_texture.is_some() {
            let panel_alpha = self.background_alpha;
            let window_alpha = (panel_alpha as u16 + 20).min(255) as u8;

            if self.dark_mode {
                // Dark mode: semi-transparent black panels, white text
                visuals.panel_fill = egui::Color32::from_rgba_premultiplied(20, 20, 20, panel_alpha);
                visuals.window_fill = egui::Color32::from_rgba_premultiplied(30, 30, 30, window_alpha);
                visuals.override_text_color = Some(egui::Color32::WHITE);
                visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(120));
            } else {
                // Light mode with background -> "Cyberpunk" theme
                // Start with dark visuals as a base for high contrast widgets
                visuals = egui::Visuals::dark();
                let neon_blue = egui::Color32::from_rgb(0, 255, 255);

                // Override text for the neon effect
                visuals.override_text_color = Some(neon_blue);

                // Make panels and windows transparent
                visuals.panel_fill = egui::Color32::from_rgba_premultiplied(0, 0, 0, panel_alpha / 2);
                visuals.window_fill = egui::Color32::from_rgba_premultiplied(20, 20, 20, (panel_alpha as u16 / 2 + 20).min(255) as u8);

                // Style strokes for the theme
                visuals.window_stroke = egui::Stroke::new(1.0, neon_blue.linear_multiply(0.5));
                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, neon_blue.linear_multiply(0.4)); // Button outline
            }
        }
        ctx.set_visuals(visuals);

        if let Some(texture) = &self.background_texture {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let screen_rect = ctx.viewport_rect(); // FIX: Use viewport_rect() instead of deprecated screen_rect()
            let texture_size = texture.size_vec2();
            let screen_aspect = screen_rect.width() / screen_rect.height();
            let texture_aspect = texture_size.x / texture_size.y;

            // CORRECT "Cover" logic
            let image_rect = if screen_aspect > texture_aspect {
                // Screen is WIDER than texture -> scale to screen WIDTH
                let width = screen_rect.width();
                let height = width / texture_aspect;
                let y_offset = (screen_rect.height() - height) / 2.0;
                egui::Rect::from_min_size(screen_rect.min + egui::vec2(0.0, y_offset), egui::vec2(width, height))
            } else {
                // Screen is TALLER than texture -> scale to screen HEIGHT
                let height = screen_rect.height();
                let width = height * texture_aspect;
                let x_offset = (screen_rect.width() - width) / 2.0;
                egui::Rect::from_min_size(screen_rect.min + egui::vec2(x_offset, 0.0), egui::vec2(width, height))
            };
            painter.image(texture.id(), image_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }

        if let Ok(update) = self.update_receiver.try_recv() {
            match update {
                GuiUpdate::LocationsFetched(locations) => {
                    self.locations = locations;
                    self.search_scope = self.locations.iter().map(|(path, _)| (path.clone(), true)).collect();
                    self.current_status = "Ready.".to_string();
                }
                GuiUpdate::ScanProgress(progress, message) => {
                    self.scan_progress = progress;
                    self.current_status = message;
                }
                GuiUpdate::ScanCompleted(count) => {
                    self.is_running_task = false;
                    self.scan_progress = 0.0;
                    self.current_status = if count > 0 {
                        format!("✅ Scan complete! Indexed {} files.", count)
                    } else {
                        "✅ Operation complete!".to_string()
                    };
                    self.command_sender.send(Command::FetchLocations).unwrap();
                }
                GuiUpdate::SearchCompleted(results) => {
                    self.is_running_task = false;
                    self.scan_progress = 0.0;
                    self.current_status = format!("Found {} results.", results.len());
                    self.search_results = results;
                }
                GuiUpdate::Error(msg) => {
                    self.is_running_task = false;
                    self.scan_progress = 0.0;
                    self.current_status = format!("❌ Error: {}", msg);
                }
            }
        }

        if let Some(path_to_delete) = &self.confirming_delete.clone() {
            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Are you sure you want to delete the index for '{}'?", path_to_delete));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirming_delete = None;
                        }
                        if ui.button("Delete").clicked() {
                            self.is_running_task = true;
                            self.current_status = format!("Deleting index for {}...", path_to_delete);
                            self.command_sender.send(Command::DeleteLocation(path_to_delete.clone())).unwrap();
                            self.confirming_delete = None;
                        }
                    });
                });
        }

        if self.show_about_window {
            let mut is_open = true;
            egui::Window::new("About DeepSearch")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("DeepSearch");
                        ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(10.0);
                    });

                    ui.label("A high-performance, cross-platform desktop application for fast file indexing and searching.");
                    ui.label("Built with Rust and the egui framework.");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Author:");
                        ui.label("Do Huy Hoang");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Source Code:");
                        ui.hyperlink("https://github.com/dohuyhoang93/DeepSearch");
                    });
                    ui.add_space(15.0);
                    
                    ui.separator();
                    ui.add_space(10.0);

                    // --- Opacity Slider ---
                    ui.horizontal(|ui| {
                        ui.label("UI Opacity:");
                        ui.add(egui::Slider::new(&mut self.background_alpha, 100..=255));
                    });
                    ui.add_space(10.0);

                    ui.separator();
                    ui.add_space(10.0);

                    ui.vertical_centered(|ui| {
                        ui.label("If you find this project useful, please consider supporting its development.");
                        ui.add_space(10.0);
                        
                        ui.strong("Donate via Bank Transfer:");
                        ui.label("Bank: BIDV (Bank for Investment and Development of Vietnam)");
                        ui.label("Account Holder: DO HUY HOANG");
                        ui.label("Account Number: 25610004007052");
                    });
                });
            if !is_open {
                self.show_about_window = false;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("DeepSearch");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.dark_mode { "🌙" } else { "☼" }).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }
                    if ui.button("About").clicked() {
                        self.show_about_window = true;
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.current_status);
                if self.is_running_task {
                    ui.add(egui::ProgressBar::new(self.scan_progress).show_percentage());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Indexing, "Indexing");
                ui.selectable_value(&mut self.active_tab, Tab::Search, "Search");
            });
            ui.separator();

            match self.active_tab {
                Tab::Indexing => self.draw_indexing_tab(ui),
                Tab::Search => self.draw_search_tab(ui),
            }
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

impl DeepSearchApp {
    fn draw_indexing_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(!self.is_running_task, |ui| {
            egui::Grid::new("indexing_grid").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui|{
                ui.label(egui::RichText::new("Initial Scan").strong());
                ui.end_row();

                ui.label("New Folder Path:");
                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::singleline(&mut self.target_path_input).hint_text("C:\\Users\\YourUser\\Documents");
                    ui.add(text_edit);

                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.target_path_input = path.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label("");
                if ui.button("Start Initial Scan").clicked() {
                    if !self.target_path_input.is_empty() {
                        self.is_running_task = true;
                        self.search_results.clear();
                        self.current_status = "Requesting scan...".to_string();
                        self.command_sender.send(Command::StartInitialScan(self.target_path_input.clone().into())).unwrap();
                    } else {
                        self.current_status = "Please enter a path to scan.".to_string();
                    }
                }
                ui.end_row();
            });
        });
        
        ui.separator();

        ui.label(egui::RichText::new("Manage Indexed Locations").strong());
        if ui.button("Refresh").clicked() && !self.is_running_task {
            self.current_status = "Fetching locations...".to_string();
            self.command_sender.send(Command::FetchLocations).unwrap();
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (path, _) in self.locations.clone() {
                ui.horizontal(|ui| {
                    ui.label(&path);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑 Delete").clicked() {
                            self.confirming_delete = Some(path.clone());
                        }
                        if ui.button("🔄 Rescan").clicked() {
                            self.is_running_task = true;
                            self.search_results.clear();
                            self.current_status = "Requesting rescan...".to_string();
                            self.command_sender.send(Command::StartRescan(path.clone())).unwrap();
                        }
                    });
                });
                ui.separator();
            }
        });
    }

    fn draw_search_tab(&mut self, ui: &mut egui::Ui) {
        // Search controls
        ui.add_enabled_ui(!self.is_running_task, |ui| {
            ui.horizontal(|ui| {
                ui.label("Keyword:");
                let response = ui.text_edit_singleline(&mut self.search_keyword);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.trigger_search();
                }
            });

            if ui.button("Search").clicked() {
                self.trigger_search();
            }
        });

        ui.separator();

        // Search scope and results are in a side-by-side layout
        egui::SidePanel::left("search_scope_panel")
            .resizable(true)
            .default_width(250.0)
            .max_width(400.0)
            .show_inside(ui, |ui| {
                ui.add_enabled_ui(!self.is_running_task, |ui| {
                    ui.label(egui::RichText::new("Search In:").strong());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (path, _) in &self.locations {
                            if let Some(is_selected) = self.search_scope.get_mut(path) {
                                ui.checkbox(is_selected, path);
                            }
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label(egui::RichText::new("Results:").strong());
            let text_height = ui.text_style_height(&egui::TextStyle::Body);
            egui::ScrollArea::vertical().show_rows(ui, text_height, self.search_results.len(), |ui, row_range| {
                for i in row_range {
                    if let Some(result) = self.search_results.get(i) {
                        let response = ui.label(result);
                        response.context_menu(|ui| {
                            if ui.button("Open File").clicked() {
                                self.command_sender.send(Command::OpenFile(result.clone())).unwrap();
                                ui.close();
                            }
                            if ui.button("Open File Location").clicked() {
                                self.command_sender.send(Command::OpenLocation(result.clone())).unwrap();
                                ui.close();
                            }
                        });
                    }
                }
            });
        });
    }

    fn trigger_search(&mut self) {
        if !self.search_keyword.is_empty() {
            let selected_locations: Vec<_> = self.locations.iter()
                .filter(|(path, _)| *self.search_scope.get(path).unwrap_or(&false))
                .cloned()
                .collect();
            
            if !selected_locations.is_empty() {
                self.is_running_task = true;
                self.search_results.clear();
                self.current_status = "Requesting search...".to_string();
                self.command_sender.send(Command::StartSearch { locations: selected_locations, keyword: self.search_keyword.clone() }).unwrap();
            } else {
                self.current_status = "Please select at least one location to search in.".to_string();
            }
        }
    }
}

