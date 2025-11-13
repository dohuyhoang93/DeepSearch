use std::path::PathBuf;
use std::sync::Arc;
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