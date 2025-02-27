use eframe;

mod app;
mod map;
mod ui;
mod config;

fn main() {
    let app = app::CelesteMapEditor::new();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}