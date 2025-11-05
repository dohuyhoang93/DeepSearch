use crate::db::DbManager;
use crate::pop::context::Context;
use crate::utils;
use crate::gui::events::{GuiUpdate, DisplayResult};

const BATCH_SIZE: usize = 200; // Send results in small batches for a responsive UI

/// Process: Performs the search and streams results back to the UI thread in batches.
pub fn search_index(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let raw_keyword = context.search_keyword.as_ref().unwrap();
    let reporter = context.progress_reporter.as_ref().unwrap();

    let normalized_keyword = utils::normalize_string(raw_keyword);
    reporter.send(GuiUpdate::ScanProgress(0.0, format!("🔍 Searching for '{}'...", raw_keyword)))?;

    let db_manager = DbManager::new(db_path)?;
    let locations_to_search = std::mem::take(&mut context.search_locations);

    let mut total_found = 0;
    let mut batch = Vec::with_capacity(BATCH_SIZE);

    if let Some(locations_to_search) = locations_to_search {
        let num_locations = locations_to_search.len();
        if num_locations == 0 {
            reporter.send(GuiUpdate::SearchFinished)?;
            return Ok(context); // No locations to search
        }

        for (i, (location_path, table_name)) in locations_to_search.iter().enumerate() {
            reporter.send(GuiUpdate::ScanProgress(i as f32 / num_locations as f32, format!("Searching in {}...", location_path)))?;
            
            let found_paths = db_manager.search_in_table(table_name, &normalized_keyword)?;

            for path in found_paths {
                let full_path = std::path::Path::new(location_path).join(&path).to_string_lossy().to_string();
                
                let display_result = DisplayResult {
                    icon: utils::get_icon_for_path(&full_path).to_string(),
                    full_path: full_path.into(),
                };

                batch.push(display_result);
                total_found += 1;

                if batch.len() >= BATCH_SIZE {
                    reporter.send(GuiUpdate::SearchResultsBatch(batch.clone()))?;
                    batch.clear();
                }
            }
        }
    }

    // Send the final batch if any results are left
    if !batch.is_empty() {
        reporter.send(GuiUpdate::SearchResultsBatch(batch))?;
    }

    reporter.send(GuiUpdate::SearchFinished)?;
    context.files_found_count = total_found; // Reuse this field to pass the final count
    Ok(context)
}