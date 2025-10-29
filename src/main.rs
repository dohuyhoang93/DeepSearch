#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use num_cpus;
use eframe::egui;
use image;

mod db;
mod pop;
mod processes;
mod utils;
mod gui;

fn load_icon_from_memory(bytes: &[u8]) -> Result<egui::IconData, anyhow::Error> {
    let image = image::load_from_memory(bytes)?.to_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    Ok(egui::IconData {
        rgba,
        width,
        height,
    })
}

fn main() -> anyhow::Result<()> {
    // --- Configure Rayon Thread Pool ---
    let num_threads = num_cpus::get() * 2;
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    // --- Load Icon from memory ---
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon = load_icon_from_memory(icon_bytes)?;

    // --- Configure and Run the GUI ---
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(icon),
        ..Default::default()
    };
    
    eframe::run_native(
        "DeepSearch",
        native_options,
        Box::new(|cc| {
            // --- Font Setup ---
            let mut fonts = egui::FontDefinitions::default();

            // Install main font
            fonts.font_data.insert(
                "noto_sans".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSans.ttf")).into(),
            );

            // Install a broad symbol font as a fallback
            fonts.font_data.insert(
                "arial".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/arial.ttf")).into(),
            );

            // Install emoji font for modern emojis
            fonts.font_data.insert(
                "noto_emoji".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/NotoColorEmoji-Regular.ttf")).into(),
            );

            // Install fallback for Japanese
            fonts.font_data.insert(
                "noto_sans_jp".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSansJP-Regular.ttf")).into(),
            );

            // Set up fallback chain. Order is important.
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .extend(vec!["noto_sans".to_owned(), "arial".to_owned(), "noto_emoji".to_owned(), "noto_sans_jp".to_owned()]);

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .extend(vec!["noto_sans".to_owned(), "arial".to_owned(), "noto_emoji".to_owned(), "noto_sans_jp".to_owned()]);
            
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(gui::app::DeepSearchApp::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Eframe error: {}", e))?;

    Ok(())
}