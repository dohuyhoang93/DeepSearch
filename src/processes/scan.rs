use crate::db::{DbManager, FileMetadata};
use crate::pop::context::Context;
use crate::utils;
use rayon::prelude::*;
use std::sync::mpsc;
use std::time::{SystemTime};
use std::thread;
use std::sync::Arc;
use redb::{ReadableTable, TableDefinition};

const BATCH_SIZE: usize = 50_000;

// --- PROCESSES ---

/// Process: Scans the directory and streams file data through a channel for the initial scan.
pub fn scan_directory_streaming(mut context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
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

/// Process: Performs a unified, memory-safe rescan.
pub fn rescan_unified_streaming(context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let db_path = context.db_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();

    let db_manager = Arc::new(DbManager::new(&db_path)?);
    let main_table_name = db_manager.get_table_name(root_path.to_str().unwrap())?.unwrap();

    // --- Phase 1: Scan FS for updates and snapshot existing paths ---
    utils::report_progress(&reporter, 0.0, "🔄 Rescan Phase 1/2: Finding changes and snapshotting...");

    let (updates_tx, updates_rx) = mpsc::channel::<(String, FileMetadata)>();
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<String>();
    let temp_table_name = format!("_temp_snapshot_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs());

    // --- Writer Threads ---
    let updates_writer = {
        let db = Arc::clone(&db_manager);
        let root = root_path.clone();
        let rep = reporter.clone();
        thread::spawn(move || -> anyhow::Result<usize> {
            let mut batch = Vec::with_capacity(BATCH_SIZE);
            let mut count = 0;
            for item in updates_rx {
                batch.push(item);
                count += 1;
                if batch.len() >= BATCH_SIZE {
                    db.update_index_for_path(root.to_str().unwrap(), &batch, &[])?;
                    utils::report_progress(&rep, 0.1 + (count as f32 * 0.00001), &format!("Applied {} updates...", count));
                    batch.clear();
                }
            }
            if !batch.is_empty() {
                db.update_index_for_path(root.to_str().unwrap(), &batch, &[])?;
            }
            Ok(count)
        })
    };

    let snapshot_writer = {
        let db = Arc::clone(&db_manager);
        let temp_name = temp_table_name.clone();
        thread::spawn(move || -> anyhow::Result<()> {
            let mut batch = Vec::with_capacity(BATCH_SIZE);
            for path in snapshot_rx {
                batch.push(path);
                if batch.len() >= BATCH_SIZE {
                    db.write_paths_to_table(&temp_name, &batch)?;
                    batch.clear();
                }
            }
            if !batch.is_empty() {
                db.write_paths_to_table(&temp_name, &batch)?;
            }
            Ok(())
        })
    };

    // --- Scanner Thread ---
    let scanner = {
        let scanner_reporter = reporter.clone();
        let db = Arc::clone(&db_manager);
        let main_table = main_table_name.clone();
        thread::spawn(move || {
            let (top_level_entries, subdirs) = utils::discover_fs_structure(&root_path, &scanner_reporter);
            let scan_action = |entry: &walkdir::DirEntry| {
                let (path_str, new_metadata) = utils::build_file_data(entry, &root_path);
                // Send path to snapshot
                snapshot_tx.send(path_str.clone()).ok();
                // Compare and send to updates if needed
                if let Ok(Some(old_metadata)) = db.get_file_metadata(&main_table, &path_str) {
                    if old_metadata.modified_time != new_metadata.modified_time {
                        updates_tx.send((path_str, new_metadata)).ok();
                    }
                } else {
                    updates_tx.send((path_str, new_metadata)).ok();
                }
            };
            top_level_entries.par_iter().filter(|e| e.path().is_file()).for_each(scan_action.clone());
            utils::scan_subdirs(subdirs, &scanner_reporter, scan_action);
        })
    };

    scanner.join().unwrap();
    // After scanner is done, the senders go out of scope, and the channels are closed.
    // The writer threads will then finish processing any remaining items.
    let updates_count = updates_writer.join().unwrap()?;
    snapshot_writer.join().unwrap()?;
    utils::report_progress(&reporter, 0.5, &format!("Found {} new/updated files. Now checking for deletions...", updates_count));

    // --- Phase 2: Find and apply deletions ---
    let mut deletions_batch = Vec::with_capacity(BATCH_SIZE);
    let mut total_deleted_count = 0;
    {
        let read_txn = db_manager.begin_read()?;
        let main_table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&main_table_name);
        let temp_table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&temp_table_name);
        let main_table = read_txn.open_table(main_table_def)?;
        let temp_table = read_txn.open_table(temp_table_def)?;

        for item in main_table.iter()? {
            let (path, _value) = item?;
            if temp_table.get(path.value())?.is_none() {
                deletions_batch.push(path.value().to_string());
                if deletions_batch.len() >= BATCH_SIZE {
                    db_manager.update_index_for_path(context.target_path.as_ref().unwrap().to_str().unwrap(), &[], &deletions_batch)?;
                    total_deleted_count += deletions_batch.len();
                    utils::report_progress(&reporter, 0.75, &format!("Removed {} stale files...", total_deleted_count));
                    deletions_batch.clear();
                }
            }
        }
    }

    if !deletions_batch.is_empty() {
        total_deleted_count += deletions_batch.len();
        db_manager.update_index_for_path(context.target_path.as_ref().unwrap().to_str().unwrap(), &[], &deletions_batch)?;
    }

    // --- Step 3: Clean up temporary table ---
    let write_txn = db_manager.begin_write()?;
    write_txn.delete_table(TableDefinition::<&str, &[u8]>::new(&temp_table_name))?;
    write_txn.commit()?;

    utils::report_progress(&reporter, 1.0, &format!("Rescan complete. {} updates, {} deletions.", updates_count, total_deleted_count));

    Ok(context)
}



