use crate::db::DbManager;
use crate::pop::context::Context;
use crate::utils;
use rayon::prelude::*;
use std::sync::mpsc;
use std::time::{SystemTime};
use std::thread;
use redb::{TableDefinition};

const BATCH_SIZE: usize = 50_000;

// --- PROCESSES ---

/// Process: Scans the directory and streams file data through a channel for the initial scan.
pub fn scan_directory_streaming(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context.live_search_root_path.as_ref().unwrap_or_else(|| context.target_path.as_ref().unwrap()).clone();
    let reporter = context.progress_reporter.clone();
    let (tx, rx) = mpsc::channel();

    utils::report_progress(&reporter, 0.0, &format!("🔍 Starting initial scan for '{}'...", root_path.display()));

    thread::spawn(move || {
        let (top_level_entries, subdirs) = utils::discover_fs_structure(&root_path, &reporter);
        // Handle files at the top level
        top_level_entries
            .par_iter()
            .filter(|entry| entry.path().is_file())
            .for_each(|entry| {
                tx.send(utils::build_file_data(entry, &root_path)).ok();
            });
        // Scan subdirectories in parallel
        utils::scan_subdirs(subdirs, &reporter, |entry| {
            tx.send(utils::build_file_data(entry, &root_path)).ok();
        });
    });

    context.file_data_stream = Some(rx);
    Ok(context)
}

/// Process: Performs a memory-safe, atomic-swap rescan.
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

    // Scan filesystem and stream results
    let (tx, rx) = mpsc::channel();
    let scanner_root_path = root_path.clone();
    let scanner_reporter = reporter.clone();
    thread::spawn(move || {
        let (top_level_entries, subdirs) = utils::discover_fs_structure(&scanner_root_path, &scanner_reporter);
        top_level_entries
            .par_iter()
            .filter(|entry| entry.path().is_file())
            .for_each(|entry| {
                tx.send(utils::build_file_data(entry, &scanner_root_path)).ok();
            });
        utils::scan_subdirs(subdirs, &scanner_reporter, |entry| {
            tx.send(utils::build_file_data(entry, &scanner_root_path)).ok();
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



