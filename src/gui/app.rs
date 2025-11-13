use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use eframe::egui;
use crate::db::DbManager;
use crate::gui::components::indexing_tab::IndexingTab;
use crate::gui::components::menu_bar::MenuBar;
use crate::gui::components::search_tab::SearchTab;
use crate::gui::components::status_bar::StatusBar;
use crate::pop::context::Context;
use crate::pop::control::TaskController;
use crate::pop::engine::Engine;
use crate::pop::registry::Registry;
use crate::processes;
use super::events::{Command, GuiUpdate};

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    Indexing,
    Search,
}

// Holds the shared state of the application that multiple components might need to access or modify.
pub struct AppState {
    pub locations: Vec<(String, String, u64)>,
    pub current_status: String,
    pub scan_progress: f32,
    pub is_running_task: bool,
    pub is_paused: bool,
    pub active_task_control: Option<Arc<TaskController>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            locations: vec![],
            current_status: "Ready. Fetching locations...".to_string(),
            scan_progress: 0.0,
            is_running_task: false,
            is_paused: false,
            active_task_control: None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DeepSearchApp {
    dark_mode: bool,
    background_alpha: u8,
    active_tab: Tab,

    #[serde(skip)]
    state: AppState,
    #[serde(skip)]
    menu_bar: MenuBar,
    #[serde(skip)]
    indexing_tab: IndexingTab,
    #[serde(skip)]
    search_tab: SearchTab,
    #[serde(skip)]
    status_bar: StatusBar,

    #[serde(skip)]
    command_sender: Sender<Command>,
    #[serde(skip)]
    update_receiver: Receiver<GuiUpdate>,

    #[serde(skip)]
    pub background_texture: Option<egui::TextureHandle>,
}

impl Default for DeepSearchApp {
    fn default() -> Self {
        let (command_sender, command_receiver) = mpsc::channel();
        let (update_sender, update_receiver) = mpsc::channel();

        thread::spawn(move || {
            let mut registry = Registry::new();
            registry.register_process("scan_directory_streaming", processes::scan::scan_directory_streaming);
            registry.register_process("write_index_from_stream_batched", processes::index::write_index_from_stream_batched);
            registry.register_process("rescan_scan_streaming", processes::scan::rescan_scan_streaming);
            registry.register_process("rescan_write_index_from_stream_batched", processes::index::rescan_write_index_from_stream_batched);
            registry.register_process("rescan_atomic_swap_final", processes::scan::rescan_atomic_swap_final);
            registry.register_process("search_index", processes::search::search_index);
            registry.register_process("live_search_2_phase", processes::live_search::live_search_2_phase);
            registry.register_workflow("gui_initial_scan", vec!["scan_directory_streaming".to_string(), "write_index_from_stream_batched".to_string()]);
            registry.register_workflow("gui_rescan", vec!["rescan_scan_streaming".to_string(), "rescan_write_index_from_stream_batched".to_string(), "rescan_atomic_swap_final".to_string()]);
            registry.register_workflow("gui_search", vec!["search_index".to_string()]);
            registry.register_workflow("gui_live_search", vec!["live_search_2_phase".to_string()]);

            let engine = Engine::new(registry);
            let db_path = PathBuf::from("deepsearch_index.redb");

            for command in command_receiver {
                let mut context = Context {
                    search_keyword: None,

                    progress_reporter: Some(update_sender.clone()),
                    live_search_root_path: None,
                    search_in_content: false,
                    search_in_pdf: false,
                    search_in_office: false,
                    search_in_plain_text: false,
                    task_controller: None,
                    db_path: Some(db_path.clone()),
                    target_path: None,
                    file_data_stream: None,
                    files_found_count: 0,
                    search_locations: None,
                    new_table_name: None,
                    old_table_name: None,
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
                                update_sender.send(GuiUpdate::LocationsUpdated(locations_with_counts)).unwrap();
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
                    Command::StartInitialScan { path, task_controller } => {
                        context.target_path = Some(path);
                        context.task_controller = Some(task_controller);
                        match engine.run_workflow("gui_initial_scan", context) {
                            Ok(final_context) => {
                                update_sender.send(GuiUpdate::ScanCompleted(final_context.files_found_count)).unwrap();
                            }
                            Err(e) => update_sender.send(GuiUpdate::Error(e.to_string())).unwrap(),
                        }
                    }
                    Command::StartRescan { path, task_controller } => {
                        context.target_path = Some(path);
                        context.task_controller = Some(task_controller);
                        match engine.run_workflow("gui_rescan", context) {
                            Ok(final_context) => {
                                update_sender.send(GuiUpdate::ScanCompleted(final_context.files_found_count)).unwrap();
                            }
                            Err(e) => update_sender.send(GuiUpdate::Error(e.to_string())).unwrap(),
                        }
                    }
                    Command::StartSearch { locations, keyword, is_live_search_active, live_search_path, search_in_content, search_in_pdf, search_in_office, search_in_plain_text, task_controller } => {
                        context.search_keyword = Some(keyword);
                        context.search_in_content = search_in_content;
                        context.search_in_pdf = search_in_pdf;
                        context.search_in_office = search_in_office;
                        context.search_in_plain_text = search_in_plain_text;
                        context.task_controller = Some(task_controller);
                        if is_live_search_active {
                            context.live_search_root_path = live_search_path;
                            if let Err(e) = engine.run_workflow("gui_live_search", context) {
                                update_sender.send(GuiUpdate::Error(e.to_string())).unwrap();
                            }
                        } else {
                            context.search_locations = Some(locations);
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
            background_alpha: 210,
            state: AppState::default(),
            menu_bar: MenuBar::default(),
            indexing_tab: IndexingTab::default(),
            search_tab: SearchTab::default(),
            status_bar: StatusBar,
            background_texture: None,
        }
    }
}

impl eframe::App for DeepSearchApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Handle Updates from Backend Thread ---
        while let Ok(update) = self.update_receiver.try_recv() {
            match update {
                GuiUpdate::LocationsUpdated(locations) => {
                    self.state.locations = locations;
                    self.search_tab.search_scope.clear();
                    for (path, _, _) in &self.state.locations {
                        self.search_tab.search_scope.insert(path.clone(), true);
                    }
                    self.state.current_status = format!("{} locations loaded.", self.state.locations.len());
                }
                GuiUpdate::ScanProgress(progress, status) => {
                    self.state.scan_progress = progress;
                    self.state.current_status = status;
                }
                GuiUpdate::ScanCompleted(count) => {
                    self.state.is_running_task = false;
                    self.state.scan_progress = 1.0;
                    self.state.current_status = format!("✅ Scan completed. Indexed {} files.", count);
                    self.command_sender.send(Command::FetchLocations).unwrap();
                }
                GuiUpdate::SearchResultsBatch(results) => {
                    self.search_tab.search_results.extend(results);
                }
                GuiUpdate::LiveSearchResultsBatch(results) => {
                    self.search_tab.live_search_results.extend(results);
                }
                GuiUpdate::SearchFinished => {
                    self.state.is_running_task = false;
                    self.state.is_paused = false;
                    self.state.current_status = format!("Found {} results.", self.search_tab.search_results.len() + self.search_tab.live_search_results.len());
                }
                GuiUpdate::Error(e) => {
                    self.state.is_running_task = false;
                    self.state.current_status = format!("Error: {}", e);
                }
            }
        }

        // --- Set Style (Cyberpunk inspired) ---
        let is_background_present = self.background_texture.is_some();
        let new_visuals = if self.dark_mode {
            let mut visuals = egui::Visuals::dark();
            if is_background_present { // Cyberpunk theme
                visuals.override_text_color = Some(egui::Color32::from_rgb(0, 255, 170));
                visuals.window_fill = egui::Color32::TRANSPARENT;
                visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(10, 25, 35, 220);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(20, 40, 55, 180);
                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(30, 55, 70, 220);
                visuals.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(15, 30, 45, 240);
                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 255, 170, 100));
                visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(0, 255, 170, 180));
                visuals.widgets.active.bg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(0, 255, 170, 255));
                visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(0, 100, 70, 150);
            } else { // Standard dark
                visuals.override_text_color = Some(egui::Color32::from_gray(220));
                visuals.panel_fill = egui::Color32::from_gray(38);
            }
            visuals
        } else {
            let mut visuals = egui::Visuals::light();
            if is_background_present { // Light theme with background
                visuals.override_text_color = Some(egui::Color32::from_gray(20));
                visuals.window_fill = egui::Color32::TRANSPARENT;
                visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(245, 248, 255, 220);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(230, 235, 245, 180);
                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(210, 220, 235, 220);
                visuals.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(190, 200, 215, 240);
                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 139, 100));
                visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 139, 150));
                visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 139, 200));
                visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(170, 210, 255, 150);
            }
            visuals
        };
        ctx.set_visuals(new_visuals);

        // --- Draw Background ---
        if let Some(texture) = &self.background_texture {
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("background_painter")));
            let screen_rect = ctx.viewport_rect();
            let texture_size = texture.size_vec2();
            let screen_aspect = screen_rect.width() / screen_rect.height();
            let texture_aspect = texture_size.x / texture_size.y;
            let mut uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            if screen_aspect > texture_aspect {
                let new_height = texture_size.x / screen_aspect;
                let y_offset = (texture_size.y - new_height) / 2.0;
                uv.set_top(y_offset / texture_size.y);
                uv.set_bottom(1.0 - (y_offset / texture_size.y));
            } else {
                let new_width = texture_size.y * screen_aspect;
                let x_offset = (texture_size.x - new_width) / 2.0;
                uv.set_left(x_offset / texture_size.x);
                uv.set_right(1.0 - (x_offset / texture_size.x));
            }
            painter.image(texture.id(), screen_rect, uv, egui::Color32::from_rgba_unmultiplied(255, 255, 255, self.background_alpha));
        }

        // --- Define Frames ---
        let main_panel_frame = egui::Frame::default()
            .inner_margin(egui::Margin { left: 16, right: 16, top: 16, bottom: 16 })
            .corner_radius(egui::CornerRadius::from(8))
            .fill(ctx.style().visuals.panel_fill)
            .stroke(ctx.style().visuals.widgets.inactive.bg_stroke);

        // --- Top Bar ---
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui.add_space(5.0);
                egui::Frame::default()
                    .inner_margin(egui::Margin { left: 20, right: 20, top: 10, bottom: 10 })
                    .corner_radius(egui::CornerRadius::from(8))
                    .fill(ctx.style().visuals.panel_fill)
                    .stroke(ctx.style().visuals.widgets.inactive.bg_stroke)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Left side
                            self.menu_bar.ui(ctx, ui);

                            // Right side
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if self.dark_mode {
                                    if ui.button("🌞").on_hover_text("Switch to Light Mode").clicked() { self.dark_mode = false; }
                                } else if ui.button("🌙").on_hover_text("Switch to Dark Mode").clicked() { self.dark_mode = true; }

                                ui.add_space(10.0);
                                ui.heading("DeepSearch");
                            });
                        });
                    });
                    ui.add_space(5.0);
            });

        // --- Status Bar ---
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                ui.add_space(5.0);
                egui::Frame::default()
                    .inner_margin(egui::Margin { left: 16, right: 16, top: 10, bottom: 10 })
                    .corner_radius(egui::CornerRadius::from(8))
                    .fill(ctx.style().visuals.panel_fill)
                    .stroke(ctx.style().visuals.widgets.inactive.bg_stroke)
                    .show(ui, |ui| {
                        self.status_bar.ui(ui, &self.state);
                    });
                ui.add_space(5.0);
            });

        // --- Main Content ---
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::TRANSPARENT))
            .show(ctx, |ui| {
                main_panel_frame.show(ui, |ui| {
                    // --- Main Content Area (Tabs) ---
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::Indexing, "Indexing");
                        ui.selectable_value(&mut self.active_tab, Tab::Search, "Search");
                    });
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(10.0);

                    match self.active_tab {
                        Tab::Indexing => self.indexing_tab.ui(ui, &mut self.state, &self.command_sender),
                        Tab::Search => self.search_tab.ui(ui, &mut self.state, &self.command_sender),
                    }
                });
            });
    }
}
