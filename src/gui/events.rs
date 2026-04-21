use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use eframe::egui;
use crate::pop::control::TaskController;

pub enum Command {
    FetchLocations,
    StartSearch {
        locations: Vec<(String, String)>,
        keyword: String,
        is_live_search_active: bool,
        live_search_path: Option<PathBuf>,
        search_in_content: bool,
        search_in_pdf: bool,
        search_in_office: bool,
        search_in_plain_text: bool,
        task_controller: Arc<TaskController>,
    },
    OpenFile(String),
    OpenLocation(String),
    DeleteLocation(String),
    StartInitialScan { path: PathBuf, task_controller: Arc<TaskController> },
    StartRescan { path: PathBuf, task_controller: Arc<TaskController> },
}

#[derive(Debug)]
pub enum GuiUpdate {
    SearchResultsBatch(Vec<DisplayResult>),
    LiveSearchResultsBatch(Vec<LiveSearchResult>),
    ScanProgress(f32, String),
    ScanCompleted(usize),
    SearchFinished,
    LocationsUpdated(Vec<(String, String, u64)>),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveSearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayResult {
    pub full_path: Arc<str>,
    pub icon: String,
}

/// Wraps `Sender<GuiUpdate>` to automatically call `ctx.request_repaint()` after each send,
/// enabling push-based UI updates from background threads without polling.
#[derive(Clone)]
pub struct GuiSender {
    sender: Sender<GuiUpdate>,
    repaint_ctx: Arc<OnceLock<egui::Context>>,
}

impl GuiSender {
    pub fn new(sender: Sender<GuiUpdate>, repaint_ctx: Arc<OnceLock<egui::Context>>) -> Self {
        Self { sender, repaint_ctx }
    }

    pub fn send(&self, msg: GuiUpdate) -> Result<(), std::sync::mpsc::SendError<GuiUpdate>> {
        let result = self.sender.send(msg);
        if let Some(ctx) = self.repaint_ctx.get() {
            ctx.request_repaint();
        }
        result
    }
}