use crate::db::{DbManager, FileMetadata};
use crate::pop::context::Context;
use crate::gui::events::GuiUpdate;
use std::sync::mpsc::Sender;

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



