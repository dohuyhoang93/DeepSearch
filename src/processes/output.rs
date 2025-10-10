use crate::pop::context::Context;
use crate::display;

/// Process: Displays a summary after the workflow is complete.
pub fn display_summary(context: Context) -> anyhow::Result<Context> {
    display::show_summary(context.files_found_count);
    Ok(context)
}
