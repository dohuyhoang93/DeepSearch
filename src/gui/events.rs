use std::path::PathBuf;

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
    },
}

/// Updates sent from the Worker thread back to the GUI thread.
#[derive(Debug)]
pub enum GuiUpdate {
    LocationsFetched(Vec<(String, String)>),
    ScanProgress(f32, String), // Progress percentage and a message
    ScanCompleted(usize),      // Number of files indexed
    SearchCompleted(Vec<String>),
    Error(String),
}
