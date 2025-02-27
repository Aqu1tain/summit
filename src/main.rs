use eframe;

mod app;
mod map;
mod ui;
mod config;
mod assets;
mod celeste_atlas;
mod binary_reader;
mod xnb_reader;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Summit - Celeste Map Editor",
        options,
        Box::new(|cc| Box::new(app::CelesteMapEditor::new(cc))),
    );
}