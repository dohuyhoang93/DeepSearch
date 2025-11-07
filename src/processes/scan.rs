use crate::db::{DbManager, FileMetadata};
use crate::pop::context::Context;
use crate::utils;
use std::sync::mpsc;
use std::time::SystemTime;
use std::thread;
use redb::TableDefinition;
use rayon::prelude::*;
use jwalk::WalkDir;

const BATCH_SIZE: usize = 50_000;

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
    thread::spawn(move || {
        // Use jwalk + par_bridge for optimal throughput during indexing.
        WalkDir::new(&action_root_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .par_bridge()
            .for_each(|entry| {
                // Controller checks are performed for each file.
                if controller.is_cancelled() { return; }
                controller.check_and_wait_if_paused();
                if controller.is_cancelled() { return; }

                // NOTE: Logic from `utils::build_file_data` is replicated here to work with `jwalk::DirEntry`
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

                tx.send((relative_path, metadata)).ok();
            });
        // The channel will be closed automatically when the thread finishes and `tx` is dropped.
    });

    context.file_data_stream = Some(rx);
    Ok(context)
}

/// Process: Performs a memory-safe, atomic-swap rescan.
/// NOTE: Rescan is a critical, non-pausable operation for data integrity.
pub fn rescan_atomic_swap(context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let db_path = context.db_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();
    let root_path_str = root_path.to_str().unwrap();

    // --- Phase 1: Create a new index table from scratch ---
    utils::report_progress(&reporter, 0.0, "🔄 Rescan Phase 1/2: Building new index...");

    let db_manager = DbManager::new(&db_path)?;
    let new_table_name = format!("index_{:x}_{}", 
        md5::compute(root_path_str.as_bytes()),
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
    );

    // Scan filesystem and stream results using the high-throughput jwalk + par_bridge method.
    let (tx, rx) = mpsc::channel();
    let scanner_root_path = root_path.clone();
    thread::spawn(move || {
        WalkDir::new(&scanner_root_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .par_bridge()
            .for_each(|entry| {
                // NOTE: Logic from `utils::build_file_data` is replicated here to work with `jwalk::DirEntry`
                let relative_path = entry
                    .path()
                    .strip_prefix(&scanner_root_path)
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
                tx.send((relative_path, metadata)).ok();
            });
    });

    // Write stream to the new table
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut total_indexed_count = 0;
    let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&new_table_name);
    let write_txn = db_manager.db.begin_write()?;
    {
        let mut table = write_txn.open_table(table_def)?;
        for (path, metadata) in rx {
            batch.push((path, metadata));
            total_indexed_count += 1;
            if batch.len() >= BATCH_SIZE {
                for (p, m) in batch.drain(..) {
                    let value = bincode::encode_to_vec(m, bincode::config::standard())?;
                    table.insert(p.as_str(), &value[..])?;
                }
                utils::report_progress(&reporter, 0.5, &format!("Indexed {} files...", total_indexed_count));
            }
        }
        // Write final batch
        if !batch.is_empty() {
            for (p, m) in batch.drain(..) {
                let value = bincode::encode_to_vec(m, bincode::config::standard())?;
                table.insert(p.as_str(), &value[..])?;
            }
        }
    }
    write_txn.commit()?;
    utils::report_progress(&reporter, 0.9, &format!("Finalized new index with {} files.", total_indexed_count));

    // --- Phase 2: Atomically swap tables and delete the old one ---
    utils::report_progress(&reporter, 0.95, "🔄 Rescan Phase 2/2: Swapping index and cleaning up...");

    let old_table_name = db_manager.swap_location_table(root_path_str, &new_table_name)?;
    
    let delete_txn = db_manager.db.begin_write()?;
    let old_table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&old_table_name);
    delete_txn.delete_table(old_table_def)?;
    delete_txn.commit()?;

    utils::report_progress(&reporter, 1.0, "✅ Rescan complete.");

    Ok(context)
}