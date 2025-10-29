#[cfg(windows)]
fn main() {
    embed_resource::compile("app.rc", &[] as &[&str]);
}

#[cfg(not(windows))]
fn main() {
    // No-op on non-windows platforms
}