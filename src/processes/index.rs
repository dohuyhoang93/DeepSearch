use crate::db::{DbManager, FileMetadata};
use crate::pop::context::Context;
use crate::gui::events::GuiUpdate;
use std::sync::mpsc::Sender;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;
use walkdir::WalkDir;
use rayon::prelude::*;
use redb::ReadableTable;
use std::sync::Arc;

const BATCH_SIZE: usize = 50_000;

/// A helper function to send progress updates if a reporter is available.
fn report_progress(reporter: &Option<Sender<GuiUpdate>>, progress: f32, message: &str) {
    if let Some(sender) = reporter {
        sender.send(GuiUpdate::ScanProgress(progress, message.to_string())).ok();
    }
}

/// Process: Reads file data from the stream in the context and writes it to the DB in batches.
pub fn write_index_from_stream_batched(mut context: Context) -> anyhow::Result<Context> {
    let reporter = &context.progress_reporter;
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();
    let rx = context.file_data_stream.take().unwrap(); // Take ownership of the receiver

    let db_manager = DbManager::new(db_path)?;
    let mut batch: Vec<(String, FileMetadata)> = Vec::with_capacity(BATCH_SIZE);
    let mut total_indexed_count = 0;

    report_progress(reporter, 0.90, "⚙️ Indexing files...");

    for file_data in rx {
        batch.push(file_data);
        total_indexed_count += 1;
        if batch.len() >= BATCH_SIZE {
            db_manager.write_index_for_path(target_path, &batch)?;
            report_progress(reporter, 0.90, &format!("⚙️ Indexed {} files...", total_indexed_count));
            batch.clear();
        }
    }

    // Write any remaining files in the last batch
    if !batch.is_empty() {
        db_manager.write_index_for_path(target_path, &batch)?;
        report_progress(reporter, 0.99, &format!("⚙️ Indexed {} files, finalizing...", total_indexed_count));
    }

    context.files_found_count = total_indexed_count;
    Ok(context)
}


/// Process: Finds and removes deleted file entries from the index.
pub fn find_and_apply_deletions(context: Context) -> anyhow::Result<Context> {
    let root_path = context.target_path.as_ref().unwrap().clone();
    let db_path = context.db_path.as_ref().unwrap().clone();
    let reporter = context.progress_reporter.clone();

    report_progress(&reporter, 0.5, "Phase 1/2: Snapshotting current files...");

    let db_manager = Arc::new(DbManager::new(&db_path)?);
    let main_table_name = db_manager.get_table_name(root_path.to_str().unwrap())?.unwrap();
    let temp_table_name = format!("_temp_deletions_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs());

    // --- Step 1: Stream all current FS paths into a temporary table ---
    {
        let (tx, rx) = mpsc::channel::<String>();
        let temp_db_writer = Arc::clone(&db_manager);
        let temp_table_name_clone = temp_table_name.clone();

        let writer = thread::spawn(move || -> anyhow::Result<()> {
            let temp_table_def: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(&temp_table_name_clone);
            let txn = temp_db_writer.begin_write()?;
            {
                let mut temp_table = txn.open_table(temp_table_def)?;
                for path in rx {
                    temp_table.insert(path.as_str(), &[] as &[u8])?;
                }
            }
            txn.commit()?;
            Ok(())
        });

        WalkDir::new(&root_path)
            .into_iter()
            .par_bridge()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .for_each_with(tx, |tx, entry| {
                let path_str = entry.path().strip_prefix(&root_path).unwrap().to_string_lossy().to_string();
                tx.send(path_str).ok();
            });
        
        writer.join().unwrap()?;
    }

    report_progress(&reporter, 0.7, "Phase 2/2: Comparing and deleting stale entries...");

    // --- Step 2: Iterate main index, check against temp table, and delete in batches ---
    let main_table_def: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(&main_table_name);
    let temp_table_def: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new(&temp_table_name);
    let read_txn = db_manager.begin_read()?;
    let main_table = read_txn.open_table(main_table_def)?;
    let temp_table = read_txn.open_table(temp_table_def)?;

    let mut deletions_batch = Vec::with_capacity(BATCH_SIZE);
    let mut total_deleted_count = 0;

    for item in main_table.iter()? {
        let (path, _value) = item?;
        if temp_table.get(path.value())?.is_none() {
            deletions_batch.push(path.value().to_string());
            if deletions_batch.len() >= BATCH_SIZE {
                total_deleted_count += deletions_batch.len();
                db_manager.update_index_for_path(root_path.to_str().unwrap(), &[], &deletions_batch)?;
                report_progress(&reporter, 0.8, &format!("Removed {} stale files...", total_deleted_count));
                deletions_batch.clear();
            }
        }
    }

    if !deletions_batch.is_empty() {
        total_deleted_count += deletions_batch.len();
        db_manager.update_index_for_path(root_path.to_str().unwrap(), &[], &deletions_batch)?;
    }
    
    drop(main_table);
    drop(temp_table);
    drop(read_txn);

    // --- Step 3: Clean up temporary table ---
    let write_txn = db_manager.begin_write()?;
    write_txn.delete_table(redb::TableDefinition::<&str, &[u8]>::new(&temp_table_name))?;
    write_txn.commit()?;

    report_progress(&reporter, 0.9, &format!("Finished removing {} stale files.", total_deleted_count));

    Ok(context)
}
