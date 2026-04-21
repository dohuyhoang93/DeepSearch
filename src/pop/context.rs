use crate::gui::events::GuiSender;
use crate::pop::control::TaskController;
use std::sync::mpsc::Receiver;
use std::path::PathBuf;
use std::sync::Arc;

#[allow(clippy::struct_excessive_bools)]
pub struct Context {
    pub search_keyword: Option<String>,

    pub progress_reporter: Option<GuiSender>,
    pub live_search_root_path: Option<PathBuf>,
    pub search_in_content: bool,
    pub search_in_pdf: bool,
    pub search_in_office: bool,
    pub search_in_plain_text: bool,
    pub task_controller: Option<Arc<TaskController>>,

    pub db_path: Option<PathBuf>,
    pub target_path: Option<PathBuf>,
    pub file_data_stream: Option<Receiver<(String, crate::db::FileMetadata)>>,
    pub files_found_count: usize,
    pub search_locations: Option<Vec<(String, String)>>,
    pub new_table_name: Option<String>,
    pub old_table_name: Option<String>,
}
