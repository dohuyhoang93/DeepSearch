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
use super::events::{Command, GuiUpdate, DisplayResult, LiveSearchResult};

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
    locations: Vec<(String, String, u64)>,
    #[serde(skip)]
    search_keyword: String,
    #[serde(skip)]
    search_scope: HashMap<String, bool>,
    #[serde(skip)]
    search_results: Vec<DisplayResult>,
    #[serde(skip)]
    live_search_path_input: String,
    #[serde(skip)]
    live_search_results: Vec<LiveSearchResult>,
    #[serde(skip)]
    is_live_search_active: bool,
    #[serde(skip)]
    live_search_in_content: bool,
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
            registry.register_process("rescan_atomic_swap", processes::scan::rescan_atomic_swap);
            registry.register_process("search_index", processes::search::search_index);
            registry.register_process("live_search_and_stream_results", processes::live_search::live_search_and_stream_results);

            // Register GUI-specific workflows
            registry.register_workflow("gui_initial_scan", vec!["scan_directory_streaming".to_string(), "write_index_from_stream_batched".to_string()]);
            registry.register_workflow("gui_rescan", vec!["rescan_atomic_swap".to_string()]);
            registry.register_workflow("gui_search", vec!["search_index".to_string()]);
            registry.register_workflow("gui_live_search", vec!["live_search_and_stream_results".to_string()]);

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
                                let mut locations_with_counts = Vec::new();
                                for (path, table_name) in locations {
                                    let count = db_manager.get_table_len(&table_name).unwrap_or(0);
                                    locations_with_counts.push((path, table_name, count));
                                }
                                update_sender.send(GuiUpdate::LocationsFetched(locations_with_counts)).unwrap();
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
                    Command::StartSearch { locations, keyword, is_live_search_active, live_search_path, search_in_content } => {
                        context.search_keyword = Some(keyword);
                        context.search_in_content = search_in_content;
                        if is_live_search_active {
                            context.live_search_root_path = live_search_path;
                            if let Err(e) = engine.run_workflow("gui_live_search", context) {
                                update_sender.send(GuiUpdate::Error(e.to_string())).unwrap();
                            }
                        } else {
                            context.search_locations = locations;
                            if let Err(e) = engine.run_workflow("gui_search", context) {
                                update_sender.send(GuiUpdate::Error(e.to_string())).unwrap();
                            }
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
            live_search_path_input: "".to_string(),
            live_search_results: vec![],
            is_live_search_active: false,
            live_search_in_content: false,
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
                    self.search_scope = self.locations.iter().map(|(path, _, _)| (path.clone(), true)).collect();
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
                GuiUpdate::SearchResultsBatch(batch) => {
                    self.search_results.extend(batch);
                    self.current_status = format!("Found {} results...", self.search_results.len());
                }
                GuiUpdate::LiveSearchResultsBatch(batch) => {
                    self.live_search_results.extend(batch);
                    self.current_status = format!("Found {} live results...", self.live_search_results.len());
                }
                GuiUpdate::SearchFinished => {
                    self.is_running_task = false;
                    self.scan_progress = 0.0;
                    self.current_status = if self.is_live_search_active && self.live_search_in_content {
                        format!("Search finished. Found {} live results.", self.live_search_results.len())
                    } else {
                        format!("Search finished. Found {} results.", self.search_results.len())
                    };
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
                    ui.add_space(20.0);

                    // --- Opacity Slider ---
                    ui.horizontal(|ui| {
                        ui.label("UI Opacity:");
                        ui.add(egui::Slider::new(&mut self.background_alpha, 100..=255));
                    });
                    ui.add_space(20.0);

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
                    ui.add(egui::Spinner::new());
                    ui.add(egui::ProgressBar::new(self.scan_progress).show_percentage());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(5.0); // Add some top padding
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Indexing, "Indexing");
                ui.selectable_value(&mut self.active_tab, Tab::Search, "Search");
            });
            ui.add_space(10.0); // Space after the tab bar

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
        ui.add_space(10.0);

        // --- Initial Scan Section ---
        ui.add_enabled_ui(!self.is_running_task, |ui| {
            // Use a standard left-aligned vertical layout
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Initial Scan").strong());
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("New Folder Path:");
                    let text_edit = egui::TextEdit::singleline(&mut self.target_path_input).hint_text("C:\\Users\\YourUser\\Documents");
                    ui.add(text_edit);

                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.target_path_input = path.display().to_string();
                        }
                    }
                });

                ui.add_space(5.0);

                // This button will now have its natural width
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
            });
        });

        ui.add_space(20.0); // Replaces the separator

        // --- Manage Indexed Locations Section ---
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Manage Indexed Locations").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⟲ Refresh").clicked() && !self.is_running_task {
                    self.current_status = "Fetching locations...".to_string();
                    self.command_sender.send(Command::FetchLocations).unwrap();
                }
            });
        });
        ui.add_space(10.0);

        // Define a custom frame for the cards
        let card_frame = egui::Frame {
            inner_margin: egui::Margin { left: 10, right: 10, top: 10, bottom: 10 },
            corner_radius: 5.0.into(),
            fill: {
                let [r, g, b, a] = ui.visuals().panel_fill.to_array();
                let new_a = (a as u16 + 15).min(255) as u8;
                egui::Color32::from_rgba_premultiplied(r, g, b, new_a)
            },
            ..egui::Frame::default()
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (path, _, count) in self.locations.clone() {
                // Use the custom frame for each location card
                card_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&path).strong().size(14.0));
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
                    ui.label(format!("{} files indexed", count));
                });
                ui.add_space(5.0); // Space between cards
            }
        });
    }

    fn draw_search_tab(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(!self.is_running_task, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.is_live_search_active, "Live Search in Folder");
                if self.is_live_search_active {
                    ui.checkbox(&mut self.live_search_in_content, "Search in file content");
                    ui.label("Path:");
                    let text_edit = egui::TextEdit::singleline(&mut self.live_search_path_input).hint_text("C:\\Users\\YourUser\\Documents");
                    ui.add(text_edit);

                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.live_search_path_input = path.display().to_string();
                        }
                    }
                }
            });
            ui.add_space(10.0);

            // Search controls
            ui.horizontal(|ui| {
                ui.label("Keyword:");
                ui.label("🔍"); // Search icon
                let response = ui.text_edit_singleline(&mut self.search_keyword);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.trigger_search();
                }
            });

            if ui.button("Search").clicked() {
                self.trigger_search();
            }
        });

        ui.add_space(10.0);

        // Search scope and results are in a side-by-side layout
        egui::SidePanel::left("search_scope_panel")
            .resizable(true)
            .default_width(250.0)
            .max_width(400.0)
            .show_inside(ui, |ui| {
                ui.add_enabled_ui(!self.is_running_task, |ui| {
                    ui.label(egui::RichText::new("Search In:").strong());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (path, _, _) in &self.locations {
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
                // Display live search results (content search)
                if self.live_search_results.is_empty() && !self.is_running_task {
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
                                let response = ui.selectable_label(false, display_text)
                                    .on_hover_text(&result.file_path);

                                response.context_menu(|ui| {
                                    if ui.button("Open File").clicked() {
                                        self.command_sender.send(Command::OpenFile(result.file_path.clone())).unwrap();
                                        ui.close();
                                    }
                                    if ui.button("Open File Location").clicked() {
                                        self.command_sender.send(Command::OpenLocation(result.file_path.clone())).unwrap();
                                        ui.close();
                                    }
                                });
                            }
                        }
                    });
                }
            } else {
                // Display indexed search results (existing logic)
                if self.search_results.is_empty() && !self.is_running_task {
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

                    const WIDE_CHAR_APPROX_WIDTH: f32 = 10.0; // Use a constant approximation

                    egui::ScrollArea::vertical().show_rows(ui, text_height, self.search_results.len(), |ui, row_range| {
                        let available_width = ui.available_width();
                        let num_chars_to_keep = (available_width / WIDE_CHAR_APPROX_WIDTH).floor() as usize;

                        for i in row_range {
                            if let Some(result) = self.search_results.get(i) {
                                let truncated_path = self.truncate_path(&result.full_path, num_chars_to_keep);

                                let display_text = format!("{} {}", result.icon, truncated_path);
                                let response = ui.selectable_label(false, display_text)
                                    .on_hover_text(&*result.full_path);

                                response.context_menu(|ui| {
                                    if ui.button("Open File").clicked() {
                                        self.command_sender.send(Command::OpenFile(result.full_path.to_string())).unwrap();
                                        ui.close();
                                    }
                                    if ui.button("Open File Location").clicked() {
                                        self.command_sender.send(Command::OpenLocation(result.full_path.to_string())).unwrap();
                                        ui.close();
                                    }
                                });
                            }
                        }
                    });
                }
            }
        });
    }

    fn trigger_search(&mut self) {
        if !self.search_keyword.is_empty() {
            let selected_locations: Vec<_> = self.locations.iter()
                .filter(|(path, _, _)| *self.search_scope.get(path).unwrap_or(&false))
                .cloned()
                .collect();
            
            if !selected_locations.is_empty() {
                if self.is_live_search_active && self.live_search_in_content {
                    self.live_search_results.clear();
                } else {
                    self.search_results.clear();
                }
                self.is_running_task = true;
                self.current_status = "Requesting search...".to_string();
                let locations_for_search: Vec<_> = selected_locations.iter()
                    .map(|(path, table_name, _)| (path.clone(), table_name.clone()))
                    .collect();
                self.command_sender.send(Command::StartSearch {
                    locations: locations_for_search,
                    keyword: self.search_keyword.clone(),
                    is_live_search_active: self.is_live_search_active,
                    live_search_path: if self.is_live_search_active && !self.live_search_path_input.is_empty() {
                        Some(PathBuf::from(&self.live_search_path_input))
                    } else {
                        None
                    },
                    search_in_content: self.live_search_in_content,
                }).unwrap();
            } else {
                self.current_status = "Please select at least one location to search in.".to_string();
            }
        }
    }

    // Helper to truncate path from the start if it's too long
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