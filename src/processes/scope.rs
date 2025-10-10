use crate::db::DbManager;
use crate::pop::context::Context;
use crate::display;
use std::io::{self};

/// Process: Gets the list of indexed locations, lets the user select a scope, and saves it to the context.
pub fn select_search_scope(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let db_manager = DbManager::new(db_path)?;
    let all_locations = db_manager.get_all_locations()?;

    if all_locations.is_empty() {
        display::show_error("\n⚠️ No indexed locations found. Please run an initial scan first.");
        // Return an error to stop the workflow
        return Err(anyhow::anyhow!("No indexed locations to search."));
    }

    display::show_scope_selection(&all_locations);
    display::prompt("⌨️ Select a location to search in (1, 2, ..., a):");

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    match choice.trim() {
        "a" => {
            context.search_locations = all_locations;
        }
        c => {
            if let Ok(n) = c.parse::<usize>() {
                if n > 0 && n <= all_locations.len() {
                    if let Some(selected) = all_locations.get(n - 1) {
                        context.search_locations = vec![selected.clone()];
                    } else {
                        // This case should theoretically not be reached due to the length check
                        display::show_error("Invalid selection.");
                        return Err(anyhow::anyhow!("Invalid selection."));
                    }
                } else {
                    display::show_error("Selection out of range.");
                    return Err(anyhow::anyhow!("Selection out of range."));
                }
            } else {
                display::show_error("Invalid input.");
                return Err(anyhow::anyhow!("Invalid input."));
            }
        }
    }

    Ok(context)
}
