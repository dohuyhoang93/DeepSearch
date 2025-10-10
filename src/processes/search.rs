use crate::db::DbManager;
use crate::pop::context::Context;
use crate::utils::normalize_string;
use std::io::{self, Write};

/// Process: Lấy từ khóa tìm kiếm từ người dùng.
pub fn get_search_keyword(mut context: Context) -> anyhow::Result<Context> {
    print!("\n⌨️ Enter search keyword: ");
    io::stdout().flush()?;

    let mut keyword = String::new();
    io::stdin().read_line(&mut keyword)?;
    context.search_keyword = Some(normalize_string(keyword.trim()));
    Ok(context)
}

/// Process: Thực hiện tìm kiếm trong CSDL.
pub fn search_index(mut context: Context) -> anyhow::Result<Context> {
    let db_path = context.db_path.as_ref().unwrap();
    let keyword = context.search_keyword.as_ref().unwrap();
    // TODO: Mở rộng để cho phép chọn scope (tất cả hoặc một location cụ thể)

    println!("🔍 Searching for '{}'...", keyword);

    let db_manager = DbManager::new(db_path)?;
    let all_locations = db_manager.get_all_locations()?;

    let mut results = vec![];
    for (location_path, table_name) in all_locations {
        let mut found_paths = db_manager.search_in_table(&table_name, keyword)?;
        // Chuyển từ đường dẫn tương đối sang tuyệt đối
        for path in found_paths.iter_mut() {
            *path = format!("{}/{}", location_path, path);
        }
        results.append(&mut found_paths);
    }

    context.search_results = results;
    Ok(context)
}

/// Process: Hiển thị kết quả tìm kiếm.
pub fn display_results(context: Context) -> anyhow::Result<Context> {
    println!("\n--- Search Results ({} found) ---", context.search_results.len());
    if context.search_results.is_empty() {
        println!("No files found.");
    } else {
        for path in &context.search_results {
            println!("{}", path);
        }
    }
    Ok(context)
}
