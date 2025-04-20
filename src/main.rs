mod app;
mod map;
mod ui;
mod config;
mod data;

use eframe;

fn main() {
    #[cfg(debug_assertions)]
    {
        use std::env;
        if env::var("RUST_LOG").is_err() {
            env::set_var("RUST_LOG", "info");
        }
        env_logger::init();
    }
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Summit - Celeste Map Editor",
        options,
        Box::new(|cc| Box::new(crate::app::CelesteMapEditor::new(cc))),
    );
}