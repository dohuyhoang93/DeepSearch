use crate::pop::context::Context;

/// Process: Displays a summary after the workflow is complete.
pub fn display_summary(context: Context) -> anyhow::Result<Context> {
    println!("\n🎉 Workflow finished!");
    println!("   Total files indexed: {}", context.files_found_count);
    Ok(context)
}
