
use crate::db::{DbManager, FileMetadata};
use crate::pop::context::Context;
use crate::utils;
use std::sync::mpsc;
use std::time::SystemTime;
use std::thread;
use redb::TableDefinition;




// --- PROCESSES ---

/// Process: Scans the directory using a throughput-optimized parallel method (jwalk + par_bridge)
/// and streams file data. This process is controllable.
pub fn scan_directory_streaming(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();
    let controller = context.task_controller.take().ok_or_else(|| anyhow::anyhow!("Task controller not available for scan"))?;
    let (tx, rx) = mpsc::channel();

    utils::report_progress(&reporter, 0.0, &format!("🔍 Starting initial scan for '{}'...", root_path.display()));

    let action_root_path = root_path.clone();
    let tx_clone = tx.clone(); // Clone tx for the closure

    thread::spawn(move || {
        let action = |entry: walkdir::DirEntry| {
            let relative_path = entry
                .path()
                .strip_prefix(&action_root_path)
                .unwrap()
                .to_string_lossy()
                .to_string();

            let metadata = match entry.metadata() {
                Ok(meta) => FileMetadata {
                    normalized_name: utils::normalize_string(&entry.file_name().to_string_lossy()),
                    modified_time: meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                },
                Err(_) => FileMetadata {
                    normalized_name: utils::normalize_string(&entry.file_name().to_string_lossy()),
                    modified_time: 0,
                },
            };

            tx_clone.send((relative_path, metadata)).ok();
        };

        utils::controlled_two_phase_scan(
            &action_root_path,
            &reporter,
            &controller,
            action,
        );
    });

    context.file_data_stream = Some(rx);
    Ok(context)
}

/// Process: Scans the directory for a rescan operation, generates new table name,
/// retrieves old table name, and streams file data.
pub fn rescan_scan_streaming(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let db_path = context.db_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();
    let controller = context.task_controller.take().ok_or_else(|| anyhow::anyhow!("Task controller not available for rescan scan"))?;
    let (tx, rx) = mpsc::channel();

    utils::report_progress(&reporter, 0.0, &format!("🔄 Rescan Phase 1/3: Scanning for '{}'...", root_path.display()));

    let db_manager = DbManager::new(&db_path)?;
    let root_path_str = root_path.to_str().unwrap();

    // Get old table name
    let old_table_name = db_manager.get_table_name(root_path_str)?
        .ok_or_else(|| anyhow::anyhow!("Could not find old table name for location '{}'", root_path_str))?;

    // Generate new table name
    let new_table_name = format!("index_{:x}_{}",
        md5::compute(root_path_str.as_bytes()),
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
    );

    // Store table names in context for subsequent processes
    context.old_table_name = Some(old_table_name);
    context.new_table_name = Some(new_table_name.clone());

    let action_root_path = root_path.clone();
    let tx_clone = tx.clone(); // Clone tx for the closure

    thread::spawn(move || {
        let action = |entry: walkdir::DirEntry| {
            let relative_path = entry
                .path()
                .strip_prefix(&action_root_path)
                .unwrap()
                .to_string_lossy()
                .to_string();

            let metadata = match entry.metadata() {
                Ok(meta) => FileMetadata {
                    normalized_name: utils::normalize_string(&entry.file_name().to_string_lossy()),
                    modified_time: meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                },
                Err(_) => FileMetadata {
                    normalized_name: utils::normalize_string(&entry.file_name().to_string_lossy()),
                    modified_time: 0,
                },
            };

            tx_clone.send((relative_path, metadata)).ok();
        };

        utils::controlled_two_phase_scan(
            &action_root_path,
            &reporter,
            &controller,
            action,
        );
    });

    context.file_data_stream = Some(rx);
    Ok(context)
}



/// Process: Performs the final atomic swap of the new index table with the old one, and cleans up.
pub fn rescan_atomic_swap_final(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let db_path = context.db_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();
    let root_path_str = root_path.to_str().unwrap();

    let new_table_name = context.new_table_name.take()
        .ok_or_else(|| anyhow::anyhow!("New table name not found in context for atomic swap"))?;
    let old_table_name = context.old_table_name.take()
        .ok_or_else(|| anyhow::anyhow!("Old table name not found in context for atomic swap"))?;

    utils::report_progress(&reporter, 0.66, "🔄 Rescan Phase 3/3: Swapping index and cleaning up...");

    let db_manager = DbManager::new(&db_path)?;

    // Atomically swap tables
    db_manager.swap_location_table(root_path_str, &new_table_name)?;
    
    // Delete the old table
    let delete_txn = db_manager.db.begin_write()?;
    let old_table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&old_table_name);
    delete_txn.delete_table(old_table_def)?;
    delete_txn.commit()?;

    utils::report_progress(&reporter, 1.0, "✅ Rescan complete.");

    Ok(context)
}
