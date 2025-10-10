use crate::pop::context::Context;

/// Process: Hiển thị tóm tắt kết quả sau khi hoàn thành.
pub fn display_summary(context: Context) -> anyhow::Result<Context> {
    println!("\n🎉 Workflow finished!");
    println!("   Total files indexed: {}", context.files_found_count);
    Ok(context)
}
