use crate::pop::context::Context;
use std::io::{self, Write};
use std::path::Path;

/// Process: Gets the target directory path from the user and saves it to the Context.
pub fn get_target_directory(mut context: Context) -> anyhow::Result<Context> {
    loop {
        print!("\n⌨️ Enter the folder path to index: ");
        io::stdout().flush()?;

        let mut path_str = String::new();
        io::stdin().read_line(&mut path_str)?;
        let path_str = path_str.trim().to_string();

        if Path::new(&path_str).exists() {
            context.target_path = Some(path_str.into());
            break;
        } else {
            println!("❌ Error: Invalid path. Please enter a valid, existing path.");
        }
    }
    Ok(context)
}
