use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::time::Duration;

pub fn show_menu() {
    println!("\n--- {} ---", "DeepSearch Main Menu".bold().yellow());
    println!("1. {}", "Initial Scan".green());
    println!("2. {}", "Rescan".green());
    println!("3. {}", "Search".green());
    println!("q. {}", "Quit".red());
    print!("{} ", "Select an option:".yellow().bold());
    // We need to flush stdout to ensure the prompt appears before input is expected.
    io::stdout().flush().unwrap();
}

pub fn show_invalid_option() {
    println!("{}", "Invalid option!".red().bold());
}

pub fn show_scope_selection(locations: &[(String, String)]) {
    println!("\n--- {} ---", "Select Search Scope".bold().yellow());
    for (i, (path, _)) in locations.iter().enumerate() {
        println!("{}. {}", i + 1, path.cyan());
    }
    println!("a. {}", "All".green());
}

pub fn show_path_selection_menu(paths: &[(String, String)]) {
    println!("\n--- {} ---", "Select a path to Rescan".bold().yellow());
    for (i, (path, _)) in paths.iter().enumerate() {
        println!("{}. {}", i + 1, path.cyan());
    }
}

pub fn prompt(message: &str) {
    print!("{} ", message.yellow().bold());
    io::stdout().flush().unwrap();
}

pub fn show_error(message: &str) {
    eprintln!("{}", message.red().bold());
}

pub fn show_info(message: &str) {
    println!("{}", message.blue());
}

pub fn show_search_results(results: &[String]) {
    println!("\n--- {} ({}) ---", "Search Results".bold().yellow(), format!("{} found", results.len()).green());
    if results.is_empty() {
        println!("No files found.");
    } else {
        for path in results {
            println!("{}", path);
        }
    }
}

pub fn show_summary(count: usize) {
    println!("\n🎉 {}", "Workflow finished!".bold().green());
    println!("   Total files indexed: {}", count);
}

pub fn get_common_progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
        .unwrap()
        .progress_chars("#+-")
}

pub fn new_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb
}

pub fn new_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(get_common_progress_style());
    pb
}
