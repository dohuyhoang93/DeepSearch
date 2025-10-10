use crate::db::FileMetadata;
use std::collections::HashMap;
use std::path::PathBuf;

/// Context là một struct trung tâm chứa toàn bộ dữ liệu và trạng thái của ứng dụng.
#[derive(Debug, Default)]
pub struct Context {
    // --- Dữ liệu chung cho các workflow ---
    pub db_path: Option<PathBuf>,
    pub target_path: Option<PathBuf>,
    pub files_found_count: usize,

    // --- Dữ liệu cho Initial Scan ---
    pub files_to_index: Vec<(String, FileMetadata)>,

    // --- Dữ liệu cho Rescan ---
    pub loaded_index: HashMap<String, FileMetadata>,
    pub files_to_update: Vec<(String, FileMetadata)>,
    pub files_to_delete: Vec<String>,

    // --- Dữ liệu cho Search ---
    pub search_keyword: Option<String>,
    pub search_results: Vec<String>,
    pub search_locations: Vec<(String, String)>, // (location_path, table_name)
}
