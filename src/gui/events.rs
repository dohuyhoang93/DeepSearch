use std::path::PathBuf;
use std::sync::Arc;

/// A struct to hold pre-processed info for display, to make the UI loop faster.
#[derive(Debug, Clone)]
pub struct DisplayResult {
    pub full_path: Arc<str>,
    pub icon: &'static str,
}

/// Detailed search result for live search.
#[derive(Debug, Clone)]
pub struct LiveSearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
}

/// Commands sent from the GUI thread to the Worker thread.
#[derive(Debug)]
#[allow(dead_code)] // The compiler doesn't see the usage in the context_menu closure
pub enum Command {
    FetchLocations,
    StartInitialScan(PathBuf),
    StartRescan(String), // The path of the location to rescan
    DeleteLocation(String), // The path of the location to delete
    OpenFile(String),
    OpenLocation(String),
    StartSearch {
        locations: Vec<(String, String)>,
        keyword: String,
        is_live_search_active: bool,
        live_search_path: Option<PathBuf>,
        search_in_content: bool,
    },
}

/// Updates sent from the Worker thread back to the GUI thread.
#[derive(Debug)]
pub enum GuiUpdate {
    LocationsFetched(Vec<(String, String, u64)>), // (path, table_name, count)
    ScanProgress(f32, String), // Progress percentage and a message
    ScanCompleted(usize),      // Number of files indexed
    SearchResultsBatch(Vec<DisplayResult>),
    LiveSearchResultsBatch(Vec<LiveSearchResult>),
    SearchFinished,
    Error(String),
}
