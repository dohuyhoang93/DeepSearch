use crate::db::DbManager;
use crate::pop::context::Context;
use crate::utils::normalize_string;
use crate::gui::events::GuiUpdate;
use std::sync::mpsc::Sender;

/// A helper function to send progress updates if a reporter is available.
fn report_progress(reporter: &Option<Sender<GuiUpdate>>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

/// Process: Performs the search in the database.
pub fn search_index(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let raw_keyword = context.search_keyword.as_ref().unwrap();
    let reporter = &context.progress_reporter;

    // Normalize the keyword before searching
    let normalized_keyword = normalize_string(raw_keyword);

    report_progress(reporter, 0.0, &format!("🔍 Searching for '{}'...", raw_keyword));

    let db_manager = DbManager::new(db_path)?;
    let locations_to_search = &context.search_locations;
    let num_locations = locations_to_search.len();

    let mut results = vec![];
    for (i, (location_path, table_name)) in locations_to_search.iter().enumerate() {
        report_progress(reporter, i as f32 / num_locations as f32, &format!("Searching in {}...", location_path));
        
        // Use the normalized keyword for the search
        let mut found_paths = db_manager.search_in_table(table_name, &normalized_keyword)?;
        // Convert the relative path to an absolute path
        for path in found_paths.iter_mut() {
            let combined_path = std::path::Path::new(location_path).join(&*path);
            *path = combined_path.to_string_lossy().to_string();
        }
        results.append(&mut found_paths);
    }

    report_progress(reporter, 1.0, &format!("Found {} results.", results.len()));
    context.search_results = results;
    Ok(context)
}
