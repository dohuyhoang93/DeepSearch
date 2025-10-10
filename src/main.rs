use crate::pop::context::Context;
use crate::pop::engine::Engine;
use crate::pop::registry::Registry;
use std::io;
use std::path::PathBuf;
use num_cpus;

mod db;
mod pop;
mod processes;
mod utils;
pub mod display;

fn main() -> anyhow::Result<()> {
    // --- Configure Rayon Thread Pool ---
    // For I/O-bound tasks like scanning a network drive, it's beneficial to have more threads
    // than logical cores. This allows other threads to do CPU work while many are waiting for I/O.
    // We'll start with a factor of 2.
    let num_threads = num_cpus::get() * 2;
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    let db_path = PathBuf::from("deepsearch_index.redb");
    let mut registry = Registry::new();

    // Register Processes
    registry.register_process("get_target_directory", processes::input::get_target_directory);
    registry.register_process("select_rescan_target", processes::input::select_rescan_target);
    registry.register_process("display_summary", processes::output::display_summary);
    registry.register_process("scan_directory_initial", processes::scan::scan_directory_initial);
    registry.register_process("write_index_to_db", processes::index::write_index_to_db);
    registry.register_process("load_existing_index", processes::index::load_existing_index);
    registry.register_process(
        "scan_directory_incremental",
        processes::scan::scan_directory_incremental,
    );
    registry.register_process("update_index_in_db", processes::index::update_index_in_db);
    registry.register_process("select_search_scope", processes::scope::select_search_scope);
    registry.register_process("get_search_keyword", processes::search::get_search_keyword);
    registry.register_process("search_index", processes::search::search_index);
    registry.register_process("display_results", processes::search::display_results);

    // Register Workflows
    registry.register_workflow(
        "initial_scan",
        vec![
            "get_target_directory".to_string(),
            "scan_directory_initial".to_string(),
            "write_index_to_db".to_string(),
            "display_summary".to_string(),
        ],
    );
    registry.register_workflow(
        "rescan",
        vec![
            "select_rescan_target".to_string(),
            "load_existing_index".to_string(),
            "scan_directory_incremental".to_string(),
            "update_index_in_db".to_string(),
        ],
    );
    registry.register_workflow(
        "search",
        vec![
            "select_search_scope".to_string(),
            "get_search_keyword".to_string(),
            "search_index".to_string(),
            "display_results".to_string(),
        ],
    );

    let engine = Engine::new(registry);

    loop {
        display::show_menu();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;

        let mut context = Context::default();
        context.db_path = Some(db_path.clone());

        match choice.trim() {
            "1" => {
                engine.run_workflow("initial_scan", context)?;
            }
            "2" => {
                engine.run_workflow("rescan", context)?;
            }
            "3" => {
                engine.run_workflow("search", context)?;
            }
            "q" => break,
            _ => display::show_invalid_option(),
        }
    }

    Ok(())
}