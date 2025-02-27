use eframe::egui;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::app::CelesteMapEditor;
use crate::map::loader::{save_map, save_map_as};

fn render_current_room(editor: &CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32) {
    // Draw only current room
    if let Some(solids) = editor.get_solids_data() {
        for (y, line) in solids.lines().enumerate() {
            for (x, c) in line.chars().enumerate() {
                if c != '0' && c != ' ' {
                    let rect = Rect::from_min_size(
                        Pos2::new(
                            x as f32 * scaled_tile_size - editor.camera_pos.x,
                            y as f32 * scaled_tile_size - editor.camera_pos.y,
                        ),
                        Vec2::new(scaled_tile_size, scaled_tile_size),
                    );
                    
                    // Pick a color based on the character
                    let color = match c {
                        '9' => Color32::from_rgb(255, 255, 255),
                        'm' => Color32::from_rgb(150, 100, 50),
                        'n' => Color32::from_rgb(50, 150, 100),
                        'a' => Color32::from_rgb(150, 50, 150),
                        _ => SOLID_TILE_COLOR,
                    };
                    
                    painter.rect_filled(rect, 0.0, color);
                    
                    // Only draw stroke for larger zoom levels
                    if editor.zoom_level > 0.5 {
                        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_rgb(0, 0, 0)));
                    }
                }
            }
        }
    }
}

fn render_all_rooms(editor: &CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, _response: &egui::Response) {
    // Draw all rooms in two phases to ensure selected room is on top
    if let Some(map) = &editor.map_data {
        if let Some(levels) = map["__children"][0]["__children"].as_array() {
            // Phase 1: Draw non-selected rooms first
            for (i, level) in levels.iter().enumerate() {
                // Skip the currently selected room in this phase
                if i == editor.current_level_index {
                    continue;
                }
                
                if level["__name"] == "level" {
                    render_room(editor, painter, level, i, scaled_tile_size, false);
                }
            }
            
            // Phase 2: Draw the selected room on top
            if editor.current_level_index < levels.len() {
                let level = &levels[editor.current_level_index];
                if level["__name"] == "level" {
                    render_room(editor, painter, level, editor.current_level_index, scaled_tile_size, true);
                }
            }
        }
    }
}

fn render_room(editor: &CelesteMapEditor, painter: &egui::Painter, level: &serde_json::Value, 
               _index: usize, scaled_tile_size: f32, is_selected: bool) {
    if let (Some(room_x), Some(room_y)) = (level["x"].as_f64(), level["y"].as_f64()) {
        // Convert room coordinates from pixels to tiles
        let room_x_tiles = room_x / 8.0;
        let room_y_tiles = room_y / 8.0;
        
        // Convert pixel sizes to tile counts (divide by 8)
        let room_width_pixels = level.get("width").and_then(|w| w.as_f64()).unwrap_or(320.0);
        let room_height_pixels = level.get("height").and_then(|h| h.as_f64()).unwrap_or(184.0);
        
        // Convert to tile counts (1 tile = 8 pixels in Celeste)
        let room_width = (room_width_pixels / 8.0).ceil();
        let room_height = (room_height_pixels / 8.0).ceil();
        
        // Calculate actual boundaries based on solids content
        let mut max_width = 0;
        let mut max_height = 0;
        
        // Get solids content to determine actual room size
        if let Some(children) = level["__children"].as_array() {
            for child in children {
                if child["__name"] == "solids" {
                    if let Some(solids_text) = child["innerText"].as_str() {
                        for (y, line) in solids_text.lines().enumerate() {
                            max_height = max_height.max(y + 1);
                            max_width = max_width.max(line.len());
                        }
                    }
                }
            }
        }
        
        // Use the larger of declared size and content size
        let effective_width = (room_width as usize).max(max_width);
        let effective_height = (room_height as usize).max(max_height);
        
        // Draw room boundary
        let room_rect = Rect::from_min_size(
            Pos2::new(
                room_x_tiles as f32 * scaled_tile_size - editor.camera_pos.x,
                room_y_tiles as f32 * scaled_tile_size - editor.camera_pos.y,
            ),
            Vec2::new(
                effective_width as f32 * scaled_tile_size,
                effective_height as f32 * scaled_tile_size,
            ),
        );
        
        // Choose boundary color based on selected status
        let boundary_color = if is_selected {
            Color32::from_rgb(100, 200, 100) // Green for selected
        } else {
            Color32::from_rgb(200, 100, 100) // Red for non-selected
        };
        
        // Get solids for this room
        if let Some(children) = level["__children"].as_array() {
            for child in children {
                if child["__name"] == "solids" {
                    if let Some(solids_text) = child["innerText"].as_str() {
                        for (y, line) in solids_text.lines().enumerate() {
                            for (x, c) in line.chars().enumerate() {
                                if c != '0' && c != ' ' {
                                    let rect = Rect::from_min_size(
                                        Pos2::new(
                                            (room_x_tiles as f32 + x as f32) * scaled_tile_size - editor.camera_pos.x,
                                            (room_y_tiles as f32 + y as f32) * scaled_tile_size - editor.camera_pos.y,
                                        ),
                                        Vec2::new(scaled_tile_size, scaled_tile_size),
                                    );
                                    
                                    // Pick a color based on the character
                                    let color = match c {
                                        '9' => Color32::from_rgb(255, 255, 255),
                                        'm' => Color32::from_rgb(150, 100, 50),
                                        'n' => Color32::from_rgb(50, 150, 100),
                                        'a' => Color32::from_rgb(150, 50, 150),
                                        _ => SOLID_TILE_COLOR,
                                    };
                                    
                                    painter.rect_filled(rect, 0.0, color);
                                    
                                    // Only draw stroke for larger zoom levels
                                    if editor.zoom_level > 0.5 {
                                        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_rgb(0, 0, 0)));
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
        
        // Draw the room boundary
        painter.rect_stroke(room_rect, 0.0, Stroke::new(if is_selected { 3.0 } else { 2.0 }, boundary_color));
        
        // Draw room name
        if editor.show_labels {
            if let Some(name) = level["name"].as_str() {
                painter.text(
                    Pos2::new(
                        room_rect.min.x + 5.0,
                        room_rect.min.y + 5.0,
                    ),
                    egui::Align2::LEFT_TOP,
                    name,
                    FontId::proportional(14.0), // Use FontId instead of TextStyle
                    Color32::WHITE,
                );
            }
        }
    }
}

// Constants
pub const TILE_SIZE: f32 = 20.0;
pub const GRID_COLOR: Color32 = Color32::from_rgb(70, 70, 70);
pub const SOLID_TILE_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
pub const BG_COLOR: Color32 = Color32::from_rgb(30, 30, 30);

pub fn render_app(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    render_top_panel(editor, ctx);
    render_bottom_panel(editor, ctx);
    render_central_panel(editor, ctx);
}

fn render_top_panel(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open...").clicked() {
                    editor.show_open_dialog = true;
                    ui.close_menu();
                }
                if ui.button("Save").clicked() {
                    save_map(editor);
                    ui.close_menu();
                }
                if ui.button("Save As...").clicked() {
                    save_map_as(editor);
                    ui.close_menu();
                }
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
            
            ui.menu_button("View", |ui| {
                if ui.checkbox(&mut editor.show_all_rooms, "Show All Rooms").clicked() {
                    // Reset camera position when switching view modes
                    editor.camera_pos = Vec2::new(0.0, 0.0);
                }
                
                ui.separator();

                ui.checkbox(&mut editor.show_grid, "Show Grid");
                ui.checkbox(&mut editor.show_labels, "Show Labels");

                ui.separator();
                
                if ui.button("Zoom In").clicked() {
                    editor.zoom_level *= 1.2;
                    ui.close_menu();
                }
                if ui.button("Zoom Out").clicked() {
                    editor.zoom_level /= 1.2;
                    // Prevent zooming out too far
                    if editor.zoom_level < 0.1 {
                        editor.zoom_level = 0.1;
                    }
                    ui.close_menu();
                }
                if ui.button("Reset Zoom").clicked() {
                    editor.zoom_level = 1.0;
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Key Bindings...").clicked() {
                    editor.show_key_bindings_dialog = true;
                    ui.close_menu();
                }                    
            });
            
            ui.separator();
            
            if !editor.show_all_rooms {
                ui.label("Room: ");
                egui::ComboBox::from_id_source("level_selector")
                    .selected_text(editor.level_names.get(editor.current_level_index)
                        .unwrap_or(&"None".to_string()))
                    .show_ui(ui, |ui| {
                        for (i, name) in editor.level_names.iter().enumerate() {
                            if ui.selectable_label(editor.current_level_index == i, name).clicked() {
                                editor.current_level_index = i;
                            }
                        }
                    });
            }
        });
    });
}

fn render_bottom_panel(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(pos) = editor.drag_start {
                ui.label(format!("Drag from: ({:.1}, {:.1})", pos.x, pos.y));
            }
            ui.label(format!("Mouse: ({:.1}, {:.1})", editor.mouse_pos.x, editor.mouse_pos.y));
            
            let (tile_x, tile_y) = editor.screen_to_map(editor.mouse_pos);
            ui.label(format!("Tile: ({}, {})", tile_x, tile_y));
            
            if let Some(path) = &editor.bin_path {
                ui.with_layout(egui::Layout::right_to_left(), |ui| {
                    ui.label(format!("File: {}", path));
                });
            }
        });
    });
}

fn render_central_panel(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(error) = &editor.error_message {
            ui.heading("Error");
            ui.label(error);
            return;
        }

        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            Sense::click_and_drag(),
        );
        
        editor.mouse_pos = response.hover_pos().unwrap_or_default();
        
        // Draw background
        painter.rect_filled(
            response.rect,
            0.0,
            BG_COLOR,
        );
        
        // Calculate scaled tile size based on zoom level
        let scaled_tile_size = TILE_SIZE * editor.zoom_level;
        
        // Draw grid
        if editor.show_grid {
            let grid_start_x = editor.camera_pos.x % scaled_tile_size;
            let grid_start_y = editor.camera_pos.y % scaled_tile_size;
            
            for i in 0..((response.rect.width() / scaled_tile_size) as i32 + 2) {
                let x = i as f32 * scaled_tile_size - grid_start_x;
                painter.line_segment(
                    [Pos2::new(x, 0.0), Pos2::new(x, response.rect.height())],
                    Stroke::new(1.0, GRID_COLOR),
                );
            }
            
            for i in 0..((response.rect.height() / scaled_tile_size) as i32 + 2) {
                let y = i as f32 * scaled_tile_size - grid_start_y;
                painter.line_segment(
                    [Pos2::new(0.0, y), Pos2::new(response.rect.width(), y)],
                    Stroke::new(1.0, GRID_COLOR),
                );
            }
        }
        
        if editor.show_all_rooms {
            render_all_rooms(editor, &painter, scaled_tile_size, &response);
        } else {
            render_current_room(editor, &painter, scaled_tile_size);
        }
    });
}