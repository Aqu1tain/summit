use eframe::egui;

/// Shows a clean, simple loading screen.
pub fn show_loading_screen(ctx: &egui::Context) {
    // Use egui's input().time for animation (seconds since start)
    let secs = ctx.input().time as f32;
    let pulse = (secs * 2.0).sin() * 0.5 + 0.5;
    
    // Create a dark background
    let bg_color = egui::Color32::from_rgb(16, 24, 36);
    
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(bg_color))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Center content vertically
                ui.add_space(ui.available_height() * 0.3);
                
                // Simple title
                ui.heading(
                    egui::RichText::new("SUMMIT")
                        .color(egui::Color32::from_rgb(135, 206, 250))
                        .size(38.0)
                        .strong()
                );
                
                ui.add_space(8.0);
                
                // Subtitle
                ui.label(
                    egui::RichText::new("Celeste Map Editor")
                        .color(egui::Color32::from_rgb(200, 220, 255))
                        .size(20.0)
                );
                
                ui.add_space(30.0);
                
                // Loading message with subtle pulse
                let alpha = 180 + (pulse * 75.0) as u8;
                ui.label(
                    egui::RichText::new("Loading...")
                        .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha))
                        .size(16.0)
                );
                
                ui.add_space(20.0);
                
                // Simple spinner
                let spinner = egui::Spinner::new().size(24.0);
                ui.add(spinner);
                
                // Small tip at bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("Celeste is property of Maddy Makes Games / EXOK. This is a fan tool.")
                            .color(egui::Color32::from_rgb(100, 110, 130))
                            .size(12.0)
                    );
                });
            });
            
            // Request continuous repaints for animation
            ctx.request_repaint();
        });
}
