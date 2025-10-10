use crate::db::DbManager;
use crate::pop::context::Context;
use crate::display;

/// Process: Writes the list of scanned files to the redb database (for the initial scan).
pub fn write_index_to_db(context: Context) -> anyhow::Result<Context> {
    display::show_info(&format!("⚙️ Writing {} files to the index...", context.files_found_count));
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();
    
    let db_manager = DbManager::new(db_path)?;
    db_manager.write_index_for_path(target_path, &context.files_to_index)?;

    display::show_info("✅ Indexing complete.");
    Ok(context)
}

/// Process: Loads an existing index from the database into the context.
pub fn load_existing_index(mut context: Context) -> anyhow::Result<Context> {
    display::show_info("⚙️ Loading existing index...");
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();

    let db_manager = DbManager::new(db_path)?;
    let index = db_manager.read_index_for_path(target_path)?;
    
    display::show_info(&format!("Found {} existing entries in the index.", index.len()));
    context.loaded_index = index;
    Ok(context)
}

/// Process: Updates the database with changes (additions, updates, deletions).
pub fn update_index_in_db(context: Context) -> anyhow::Result<Context> {
    display::show_info(&format!(
        "⚙️ Updating index with {} updates and {} deletions...",
        context.files_to_update.len(),
        context.files_to_delete.len()
    ));
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();

    let db_manager = DbManager::new(db_path)?;
    db_manager.update_index_for_path(target_path, &context.files_to_update, &context.files_to_delete)?;

    display::show_info("✅ Index update complete.");
    Ok(context)
}
