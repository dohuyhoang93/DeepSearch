use crate::pop::context::Context;
use crate::utils;
use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, BufRead, Read};
use std::thread;
use std::sync::{Arc, Mutex};
use std::mem;
use jwalk::WalkDir;
use rayon::prelude::*;
use crate::gui::events::{GuiUpdate, LiveSearchResult, DisplayResult};
use calamine::{open_workbook, Reader, Xlsx};
use docx_rs::{read_docx, DocumentChild, ParagraphChild, RunChild};

const BATCH_SIZE: usize = 100;

/// Process: Scans a directory and searches file contents on the fly, streaming results back.
pub fn live_search_and_stream_results(context: Context) -> Result<Context> {
    let root_path = context.live_search_root_path.clone().ok_or_else(|| anyhow::anyhow!("Live search path not provided"))?;
    let search_keyword = context.search_keyword.clone().ok_or_else(|| anyhow::anyhow!("Search keyword not provided"))?;
    let normalized_keyword = utils::normalize_string(&search_keyword);
    let search_in_content = context.search_in_content;
    let reporter = context.progress_reporter.clone().ok_or_else(|| anyhow::anyhow!("Reporter not available"))?;

    utils::report_progress(&Some(reporter.clone()), 0.0, &format!("🔍 Starting live search for '{}' in '{}'...", search_keyword, root_path.display()));

    thread::spawn(move || {
        let live_results_batch = Arc::new(Mutex::new(Vec::with_capacity(BATCH_SIZE)));
        let indexed_results_batch = Arc::new(Mutex::new(Vec::with_capacity(BATCH_SIZE)));

        WalkDir::new(root_path)
            .into_iter()
            .par_bridge()
            .for_each(|entry_result| {
                if let Ok(entry) = entry_result {
                    if !entry.file_type().is_file() {
                        return;
                    }

                    if search_in_content {
                        let path = entry.path();
                        let extension = path.extension().and_then(|s| s.to_str());
                        match extension {
                            Some("pdf") => {
                                if context.search_in_pdf {
                                    if let Ok(text_content) = pdf_extract::extract_text(entry.path()) {
                                        for (page_num_zero_based, page_text) in
                                            text_content.split('\x0C').enumerate()
                                        {
                                            if page_text.contains(&search_keyword) {
                                                let snippet = page_text
                                                    .lines()
                                                    .find(|l| l.contains(&search_keyword))
                                                    .unwrap_or("")
                                                    .trim()
                                                    .to_string();
                                                let result = LiveSearchResult {
                                                    file_path: entry
                                                        .path()
                                                        .to_string_lossy()
                                                        .to_string(),
                                                    line_number: page_num_zero_based + 1, // Page number is 1-based
                                                    line_content: snippet,
                                                };
                                                let mut batch = live_results_batch.lock().unwrap();
                                                batch.push(result);
                                                if batch.len() >= BATCH_SIZE {
                                                    reporter
                                                        .send(GuiUpdate::LiveSearchResultsBatch(
                                                            mem::take(&mut *batch),
                                                        ))
                                                        .ok();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some("docx") => {
                                if context.search_in_office {
                                    if let Ok(mut file) = File::open(entry.path()) {
                                        let mut buf = Vec::new();
                                        if file.read_to_end(&mut buf).is_ok() {
                                            if let Ok(docx) = read_docx(&buf) {
                                                let mut full_text = String::new();
                                                for child in docx.document.children {
                                                    if let DocumentChild::Paragraph(p) = child {
                                                        for p_child in p.children {
                                                            if let ParagraphChild::Run(r) = p_child {
                                                                for r_child in r.children {
                                                                    if let RunChild::Text(t) = r_child {
                                                                        full_text.push_str(&t.text);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        full_text.push('\n');
                                                    }
                                                }

                                                if full_text.contains(&search_keyword) {
                                                    let snippet = full_text.lines().find(|l| l.contains(&search_keyword)).unwrap_or("").trim().to_string();
                                                    let result = LiveSearchResult {
                                                        file_path: entry.path().to_string_lossy().to_string(),
                                                        line_number: 1, // DOCX doesn't have a clear page/line concept here, so we use 1
                                                        line_content: snippet,
                                                    };
                                                    let mut batch = live_results_batch.lock().unwrap();
                                                    batch.push(result);
                                                    if batch.len() >= BATCH_SIZE {
                                                        reporter.send(GuiUpdate::LiveSearchResultsBatch(mem::take(&mut *batch))).ok();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some("xlsx") => {
                                if context.search_in_office {
                                    if let Ok(mut workbook) = open_workbook::<Xlsx<_>, _>(entry.path()) {
                                        for sheet_name in workbook.sheet_names().to_owned() {
                                            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                                                for (i, row) in range.rows().enumerate() {
                                                    let row_text: String = row.iter().map(|cell| cell.to_string()).collect::<Vec<_>>().join(" | ");
                                                    if row_text.contains(&search_keyword) {
                                                        let result = LiveSearchResult {
                                                            file_path: entry.path().to_string_lossy().to_string(),
                                                            line_number: i + 1, // 1-based row number
                                                            line_content: row_text.trim().to_string(),
                                                        };
                                                        let mut batch = live_results_batch.lock().unwrap();
                                                        batch.push(result);
                                                        if batch.len() >= BATCH_SIZE {
                                                            reporter.send(GuiUpdate::LiveSearchResultsBatch(mem::take(&mut *batch))).ok();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // Plain text file extensions
                            Some("txt") | Some("md") | Some("log") | Some("rs") | Some("py") | Some("js") | Some("html") | Some("css") | Some("json") | Some("xml") | Some("toml") => {
                                if context.search_in_plain_text {
                                    if let Ok(file) = File::open(entry.path()) {
                                        let reader = BufReader::new(file);
                                        for (line_number, line) in reader.lines().enumerate() {
                                            if let Ok(line_content) = line {
                                                if line_content.contains(&search_keyword) {
                                                    let result = LiveSearchResult {
                                                        file_path: entry.path().to_string_lossy().to_string(),
                                                        line_number: line_number + 1, // 1-based
                                                        line_content: line_content.trim().to_string(),
                                                    };
                                                    let mut batch = live_results_batch.lock().unwrap();
                                                    batch.push(result);
                                                    if batch.len() >= BATCH_SIZE {
                                                        reporter.send(GuiUpdate::LiveSearchResultsBatch(mem::take(&mut *batch))).ok();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => { /* Skip other file types */ }
                        }
                    } else {
                                    // Search only in filename (token-based)
                                    let query_tokens: Vec<&str> = normalized_keyword.split_whitespace().collect();
                                    let normalized_filename = utils::normalize_string(&entry.file_name().to_string_lossy());
                                    if utils::contains_all_tokens(&normalized_filename, &query_tokens) {                            let result = DisplayResult {
                                full_path: entry.path().to_string_lossy().to_string().into(),
                                icon: utils::get_icon_for_path(&entry.path().to_string_lossy()),
                            };
                            let mut batch = indexed_results_batch.lock().unwrap();
                            batch.push(result);
                            if batch.len() >= BATCH_SIZE {
                                reporter.send(GuiUpdate::SearchResultsBatch(mem::take(&mut *batch))).ok();
                            }
                        }
                    }
                }
            });

        // Send any remaining results
        let mut live_batch = live_results_batch.lock().unwrap();
        if !live_batch.is_empty() {
            reporter.send(GuiUpdate::LiveSearchResultsBatch(mem::take(&mut *live_batch))).ok();
        }
        let mut indexed_batch = indexed_results_batch.lock().unwrap();
        if !indexed_batch.is_empty() {
            reporter.send(GuiUpdate::SearchResultsBatch(mem::take(&mut *indexed_batch))).ok();
        }

        // Signal completion
        reporter.send(GuiUpdate::SearchFinished).ok();
    });

    Ok(context)
}
