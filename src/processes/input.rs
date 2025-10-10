use crate::db::DbManager;
use crate::display;
use crate::pop::context::Context;
use anyhow::anyhow;
use std::io;
use std::path::{Path, PathBuf};

/// Process: Gets the target directory path from the user and saves it to the Context.
pub fn get_target_directory(mut context: Context) -> anyhow::Result<Context> {
    loop {
        display::prompt("\n⌨️ Enter the folder path to index:");

        let mut path_str = String::new();
        io::stdin().read_line(&mut path_str)?;
        let path_str = path_str.trim().to_string();

        if Path::new(&path_str).exists() {
            context.target_path = Some(path_str.into());
            break;
        } else {
            display::show_error("❌ Error: Invalid path. Please enter a valid, existing path.");
        }
    }
    Ok(context)
}

/// Process: Shows a menu of previously indexed paths for the user to select for a rescan.
pub fn select_rescan_target(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let db_manager = DbManager::new(db_path)?;
    let all_locations = db_manager.get_all_locations()?;

    if all_locations.is_empty() {
        display::show_error("\n⚠️ No indexed locations found. Please run an initial scan first.");
        return Err(anyhow!("No indexed locations to rescan."));
    }

    display::show_path_selection_menu(&all_locations);
    display::prompt("⌨️ Select a number to rescan:");

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    match choice.trim().parse::<usize>() {
        Ok(n) if n > 0 && n <= all_locations.len() => {
            if let Some(selected) = all_locations.get(n - 1) {
                context.target_path = Some(PathBuf::from(&selected.0));
                Ok(context)
            } else {
                unreachable!(); // Should be caught by the range check
            }
        }
        _ => {
            display::show_error("Invalid selection.");
            Err(anyhow!("Invalid selection for rescan path."))
        }
    }
}