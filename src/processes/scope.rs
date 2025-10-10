use crate::db::DbManager;
use crate::pop::context::Context;
use std::io::{self, Write};

/// Process: Lấy danh sách các location đã index, cho người dùng chọn scope và lưu vào context.
pub fn select_search_scope(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let db_manager = DbManager::new(db_path)?;
    let all_locations = db_manager.get_all_locations()?;

    if all_locations.is_empty() {
        println!("
⚠️ No indexed locations found. Please run an initial scan first.");
        // Trả về lỗi để dừng workflow
        return Err(anyhow::anyhow!("No indexed locations to search."));
    }

    println!("
--- Select Search Scope ---");
    for (i, (path, _)) in all_locations.iter().enumerate() {
        println!("{}. {}", i + 1, path);
    }
    println!("a. All");
    print!("⌨️ Select a location to search in (1, 2, ..., a): ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    match choice.trim() {
        "a" => {
            context.search_locations = all_locations;
        }
        c => {
            if let Ok(n) = c.parse::<usize>() {
                if n > 0 && n <= all_locations.len() {
                    if let Some(selected) = all_locations.get(n - 1) {
                        context.search_locations = vec![selected.clone()];
                    } else {
                        return Err(anyhow::anyhow!("Invalid selection."));
                    }
                } else {
                    return Err(anyhow::anyhow!("Selection out of range."));
                }
            } else {
                return Err(anyhow::anyhow!("Invalid input."));
            }
        }
    }

    Ok(context)
}
