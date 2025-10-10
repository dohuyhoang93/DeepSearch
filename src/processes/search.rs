use crate::db::DbManager;
use crate::pop::context::Context;
use crate::utils::normalize_string;
use std::io::{self, Write};

/// Process: Gets the search keyword from the user.
pub fn get_search_keyword(mut context: Context) -> anyhow::Result<Context> {
    print!("\n⌨️ Enter search keyword: ");
    io::stdout().flush()?;

    let mut keyword = String::new();
    io::stdin().read_line(&mut keyword)?;
    context.search_keyword = Some(normalize_string(keyword.trim()));
    Ok(context)
}

/// Process: Performs the search in the database.
pub fn search_index(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let keyword = context.search_keyword.as_ref().unwrap();

    println!("🔍 Searching for '{}' in the selected scope...", keyword);

    let db_manager = DbManager::new(db_path)?;
    // NOTE: Instead of getting all locations, we now iterate over the locations
    // selected by the user in the previous step.
    let locations_to_search = &context.search_locations;

    let mut results = vec![];
    for (location_path, table_name) in locations_to_search {
        let mut found_paths = db_manager.search_in_table(&table_name, keyword)?;
        // Convert the relative path to an absolute path
        for path in found_paths.iter_mut() {
            // Handle potential path separator issues between Windows and POSIX
            let combined_path = std::path::Path::new(location_path).join(&*path);
            *path = combined_path.to_string_lossy().to_string();
        }
        results.append(&mut found_paths);
    }

    context.search_results = results;
    Ok(context)
}

/// Process: Displays the search results.
pub fn display_results(context: Context) -> anyhow::Result<Context> {
    println!("\n--- Search Results ({} found) ---", context.search_results.len());
    if context.search_results.is_empty() {
        println!("No files found.");
    } else {
        for path in &context.search_results {
            println!("{}", path);
        }
    }
    Ok(context)
}
