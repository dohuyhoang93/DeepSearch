use crate::db::DbManager;
use crate::pop::context::Context;
use crate::gui::events::GuiUpdate;
use std::sync::mpsc::Sender;

/// A helper function to send progress updates if a reporter is available.
fn report_progress(reporter: &Option<Sender<GuiUpdate>>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        // Use a high progress value to indicate we are in the final stages.
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

/// Process: Writes the list of scanned files to the redb database (for the initial scan).
pub fn write_index_to_db(context: Context) -> anyhow::Result<Context> {
    let reporter = &context.progress_reporter;
    report_progress(reporter, 0.95, &format!("⚙️ Writing {} files to the index...", context.files_found_count));

    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();
    
    let db_manager = DbManager::new(db_path)?;
    db_manager.write_index_for_path(target_path, &context.files_to_index)?;

    // The final "ScanCompleted" message will be sent by the worker thread after the workflow finishes.
    // report_progress(reporter, 1.0, "✅ Indexing complete.");
    Ok(context)
}

/// Process: Loads an existing index from the database into the context.
pub fn load_existing_index(mut context: Context) -> anyhow::Result<Context> {
    let reporter = &context.progress_reporter;
    report_progress(reporter, 0.01, "⚙️ Loading existing index...");

    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();

    let db_manager = DbManager::new(db_path)?;
    let index = db_manager.read_index_for_path(target_path)?;
    
    report_progress(reporter, 0.05, &format!("Found {} existing entries in the index.", index.len()));
    context.loaded_index = index;
    Ok(context)
}

/// Process: Updates the database with changes (additions, updates, deletions).
pub fn update_index_in_db(context: Context) -> anyhow::Result<Context> {
    let reporter = &context.progress_reporter;
    let message = format!(
        "⚙️ Updating index with {} updates and {} deletions...",
        context.files_to_update.len(),
        context.files_to_delete.len()
    );
    report_progress(reporter, 0.95, &message);

    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();

    let db_manager = DbManager::new(db_path)?;
    db_manager.update_index_for_path(target_path, &context.files_to_update, &context.files_to_delete)?;

    // report_progress(reporter, 1.0, "✅ Index update complete.");
    Ok(context)
}
