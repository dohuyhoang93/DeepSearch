use crate::db::FileMetadata;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use crate::gui::events::GuiUpdate;

/// The central struct containing all data and state for the application.
#[derive(Debug, Default)]
pub struct Context {
    // --- Common data for all workflows ---
    pub db_path: Option<PathBuf>,
    pub target_path: Option<PathBuf>,
    pub live_search_root_path: Option<PathBuf>,
    pub files_found_count: usize,
    pub progress_reporter: Option<Sender<GuiUpdate>>,

    // --- Data for Streaming Scan ---
    pub file_data_stream: Option<Receiver<(String, FileMetadata)>>,

    // --- Data for Search ---
    pub search_keyword: Option<String>,
    pub search_locations: Vec<(String, String)>,
    pub search_in_content: bool,
    pub search_in_pdf: bool,
    pub search_in_office: bool,
    pub search_in_plain_text: bool,
}
