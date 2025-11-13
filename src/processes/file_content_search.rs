use crate::pop::context::Context;
use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::sync::mpsc;
use crate::gui::events::{GuiUpdate, LiveSearchResult};
pub fn file_content_search(mut context: Context) -> Result<Context> {
    let search_keyword = context.search_keyword.clone().ok_or_else(|| anyhow::anyhow!("Search keyword not provided"))?;
    let file_data_stream = context.file_data_stream.take().ok_or_else(|| anyhow::anyhow!("File data stream not available"))?;
    let reporter = context.progress_reporter.clone();

    let (tx, rx) = mpsc::channel();

    // Spawn a thread to process the file data stream and perform search
    std::thread::spawn(move || {
        let mut files_processed = 0;
        for (file_path_str, _metadata) in file_data_stream {
            files_processed += 1;
            if let Ok(file) = File::open(&file_path_str) {
                let reader = BufReader::new(file);
                for (line_number, line) in reader.lines().enumerate() {
                    if let Ok(line_content) = line {
                        if line_content.contains(&search_keyword) {
                            tx.send(LiveSearchResult {
                                file_path: file_path_str.clone(),
                                line_number: line_number + 1, // 1-based line number
                                line_content: line_content.trim().to_string(),
                            }).ok();
                        }
                    }
                }
            }
            // Report progress (e.g., number of files processed)
            if let Some(reporter_tx) = &reporter {
                reporter_tx.send(GuiUpdate::ScanProgress(0.0, format!("Searching in {} files...", files_processed))).ok();
            }
        }
        if let Some(reporter_tx) = &reporter {
            reporter_tx.send(GuiUpdate::ScanProgress(1.0, format!("Search complete. Processed {} files.", files_processed))).ok();
        }
    });

    context.live_search_results_stream = Some(rx);

    Ok(context)
}
