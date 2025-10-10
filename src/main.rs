use crate::pop::context::Context;
use crate::pop::engine::Engine;
use crate::pop::registry::Registry;
use std::io;
use std::path::PathBuf;

mod db;
mod pop;
mod processes;
mod utils;

fn main() -> anyhow::Result<()> {
    let db_path = PathBuf::from("deepsearch_index.redb");
    let mut registry = Registry::new();

    // Register Processes
    registry.register_process("get_target_directory", processes::input::get_target_directory);
    registry.register_process("display_summary", processes::output::display_summary);
    registry.register_process("scan_directory_initial", processes::scan::scan_directory_initial);
    registry.register_process("write_index_to_db", processes::index::write_index_to_db);
    registry.register_process("load_existing_index", processes::index::load_existing_index);
    registry.register_process("scan_directory_incremental", processes::scan::scan_directory_incremental);
    registry.register_process("update_index_in_db", processes::index::update_index_in_db);
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
            "get_target_directory".to_string(),
            "load_existing_index".to_string(),
            "scan_directory_incremental".to_string(),
            "update_index_in_db".to_string(),
        ],
    );
    registry.register_workflow(
        "search",
        vec![
            "get_search_keyword".to_string(),
            "search_index".to_string(),
            "display_results".to_string(),
        ],
    );

    let engine = Engine::new(registry);

    loop {
        println!("\n--- DeepSearch Main Menu ---");
        println!("1. Initial Scan (Quét mới)");
        println!("2. Rescan (Quét lại)");
        println!("3. Search (Tìm kiếm)");
        println!("q. Quit (Thoát)");
        print!("Select an option: ");
        io::Write::flush(&mut io::stdout())?;

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
            _ => println!("Lựa chọn không hợp lệ!"),
        }
    }

    Ok(())
}