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

pub struct DeepSearchApp {
    // --- Channel Communication ---
    command_sender: Sender<Command>,
    update_receiver: Receiver<GuiUpdate>,

    // --- UI State ---
    active_tab: Tab,
    dark_mode: bool,
    target_path_input: String,
    current_status: String,
    scan_progress: f32,
    is_running_task: bool,
    confirming_delete: Option<String>,
    show_about_window: bool,

    // --- State for Rescan & Search ---
    locations: Vec<(String, String)>,
    search_keyword: String,
    search_scope: HashMap<String, bool>,
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
            registry.register_process("scan_directory_initial", processes::scan::scan_directory_initial);
            registry.register_process("write_index_to_db", processes::index::write_index_to_db);
            registry.register_process("load_existing_index", processes::index::load_existing_index);
            registry.register_process("scan_directory_incremental", processes::scan::scan_directory_incremental);
            registry.register_process("update_index_in_db", processes::index::update_index_in_db);
            registry.register_process("search_index", processes::search::search_index);

            // Register GUI-specific workflows
            registry.register_workflow("gui_initial_scan", vec!["scan_directory_initial".to_string(), "write_index_to_db".to_string()]);
            registry.register_workflow("gui_rescan", vec!["load_existing_index".to_string(), "scan_directory_incremental".to_string(), "update_index_in_db".to_string()]);
            registry.register_workflow("gui_search", vec!["search_index".to_string()]);

            let engine = Engine::new(registry);
            let db_path = PathBuf::from("deepsearch_index.redb");

            // The worker loop
            for command in command_receiver {
                let mut context = Context::default();
                context.progress_reporter = Some(update_sender.clone());
                context.db_path = Some(db_path.clone());

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
            target_path_input: "".to_string(),
            current_status: "Ready. Fetching locations...".to_string(),
            scan_progress: 0.0,
            is_running_task: false,
            confirming_delete: None,
            show_about_window: false,
            locations: vec![],
            search_keyword: "".to_string(),
            search_scope: HashMap::new(),
            search_results: vec![],
        }
    }
}

// --- App UI and Logic ---

impl eframe::App for DeepSearchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let visuals = if self.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };
        ctx.set_visuals(visuals);

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

