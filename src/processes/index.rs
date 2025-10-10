use crate::db::DbManager;
use crate::pop::context::Context;

/// Process: Ghi danh sách file đã quét vào CSDL redb (cho lần quét đầu).
pub fn write_index_to_db(context: Context) -> anyhow::Result<Context> {
    println!("⚙️ Writing {} files to index...", context.files_found_count);
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();
    
    let db_manager = DbManager::new(db_path)?;
    db_manager.write_index_for_path(target_path, &context.files_to_index)?;

    println!("✅ Indexing complete.");
    Ok(context)
}

/// Process: Tải index đã có từ CSDL vào context.
pub fn load_existing_index(mut context: Context) -> anyhow::Result<Context> {
    println!("⚙️ Loading existing index...");
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();

    let db_manager = DbManager::new(db_path)?;
    let index = db_manager.read_index_for_path(target_path)?;
    
    println!("Found {} existing entries in index.", index.len());
    context.loaded_index = index;
    Ok(context)
}

/// Process: Cập nhật CSDL với các thay đổi (thêm, sửa, xóa).
pub fn update_index_in_db(context: Context) -> anyhow::Result<Context> {
    println!(
        "⚙️ Updating index with {} updates and {} deletions...",
        context.files_to_update.len(),
        context.files_to_delete.len()
    );
    let db_path = context.db_path.as_ref().unwrap();
    let target_path = context.target_path.as_ref().unwrap().to_str().unwrap();

    let db_manager = DbManager::new(db_path)?;
    db_manager.update_index_for_path(target_path, &context.files_to_update, &context.files_to_delete)?;

    println!("✅ Index update complete.");
    Ok(context)
}
