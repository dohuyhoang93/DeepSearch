use crate::db::FileMetadata;
use std::collections::HashMap;
use std::path::PathBuf;

/// The central struct containing all data and state for the application.
#[derive(Debug, Default)]
pub struct Context {
    // --- Common data for all workflows ---
    pub db_path: Option<PathBuf>,
    pub target_path: Option<PathBuf>,
    pub files_found_count: usize,

    // --- Data for Initial Scan ---
    pub files_to_index: Vec<(String, FileMetadata)>,

    // --- Data for Rescan ---
    pub loaded_index: HashMap<String, FileMetadata>,
    pub files_to_update: Vec<(String, FileMetadata)>,
    pub files_to_delete: Vec<String>,

    // --- Data for Search ---
    pub search_keyword: Option<String>,
    pub search_results: Vec<String>,
    pub search_locations: Vec<(String, String)>, // (location_path, table_name)
}
