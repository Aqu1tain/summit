use eframe::egui;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::app::CelesteMapEditor;
use crate::map::loader::{save_map, save_map_as};

// Helper function to draw a textured rectangle using a mesh.
fn draw_texture(painter: &egui::Painter, rect: Rect, texture_id: egui::TextureId, tint: Color32) {
    let uv_rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
    let mut mesh = egui::epaint::Mesh::with_texture(texture_id);
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.min,
        uv: uv_rect.min,
        color: tint,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: Pos2::new(rect.max.x, rect.min.y),
        uv: Pos2::new(uv_rect.max.x, uv_rect.min.y),
        color: tint,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.max,
        uv: uv_rect.max,
        color: tint,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: Pos2::new(rect.min.x, rect.max.y),
        uv: Pos2::new(uv_rect.min.x, uv_rect.max.y),
        color: tint,
    });
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::epaint::Shape::mesh(mesh));
}

fn render_current_room(editor: &mut CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, ctx: &egui::Context) {
    // Debug logging to see what solids we're getting
    if let Some(level) = editor.get_current_level() {
        if let Some(name) = level["name"].as_str() {
            println!("Current level: {}", name);
        }

        // Print all children names for debugging
        if let Some(children) = level["__children"].as_array() {
            println!("Children elements:");
            for (i, child) in children.iter().enumerate() {
                println!("  {}. {}", i, child["__name"].as_str().unwrap_or("unknown"));
            }
        }
    }

    // Get solids data
    if let Some(solids_text) = editor.get_solids_data() {
        println!("Found solids with {} lines", solids_text.lines().count());

        // Get offsets from the solids element
        let mut offset_x = 0;
        let mut offset_y = 0;

        if let Some(level) = editor.get_current_level() {
            if let Some(children) = level["__children"].as_array() {
                for child in children {
                    if child["__name"] == "solids" {
                        offset_x = child["offsetX"].as_i64().unwrap_or(0) as i32;
                        offset_y = child["offsetY"].as_i64().unwrap_or(0) as i32;
                        break;
                    }
                }
            }
        }

        // Render all non-empty tiles with colors based on character
        for (y, line) in solids_text.lines().enumerate() {
            for (x, c) in line.chars().enumerate() {
                if c != '0' && c != ' ' {
                    let rect = Rect::from_min_size(
                        Pos2::new(
                            (x as f32 + offset_x as f32) * scaled_tile_size - editor.camera_pos.x,
                            (y as f32 + offset_y as f32) * scaled_tile_size - editor.camera_pos.y,
                        ),
                        Vec2::new(scaled_tile_size, scaled_tile_size),
                    );

                    let mut used_texture = false;

                    // Use textures if enabled and available
                    if editor.use_textures {
                        // First try to use the atlas system
                        used_texture = editor.draw_sprite_for_tile(painter, rect, c);

                        // Fall back to the old PNG texture loading system if atlas didn't work
                        if !used_texture {
                            if let Some(texture_path) = editor.celeste_assets.get_texture_path_for_tile(c) {
                                if let Some(texture_handle) = editor.celeste_assets.load_texture(texture_path, ctx) {
                                    draw_texture(painter, rect, texture_handle.id(), Color32::WHITE);
                                    used_texture = true;
                                }
                            }
                        }
                    }

                    // Fall back to colored rectangle if no texture was used
                    if !used_texture {
                        // Enhanced color matching for both modded and base game tiles
                        let color = match c {
                            // Modded map tiles
                            '9' => Color32::from_rgb(255, 255, 255), // Pure white
                            'm' => Color32::from_rgb(150, 100, 50),  // Brown (mountain)
                            'n' => Color32::from_rgb(50, 150, 100),  // Green (temple)
                            'a' => Color32::from_rgb(150, 50, 150),  // Purple (core)

                            // Base game tiles
                            '1' => Color32::from_rgb(220, 220, 220), // Light gray
                            '3' => Color32::from_rgb(200, 200, 200), // Medium gray
                            '4' => Color32::from_rgb(180, 180, 180), // Darker gray
                            '7' => Color32::from_rgb(160, 160, 160), // Even darker gray

                            // Handle any other characters
                            _ => SOLID_TILE_COLOR,
                        };

                        painter.rect_filled(rect, 0.0, color);

                        // Add a black border for visibility when zoomed in
                        if editor.zoom_level > 0.5 {
                            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::BLACK));
                        }
                    }
                }
            }
        }

        println!("Rendered solid tiles");
    } else {
        println!("No 'solids' data found!");
    }
}

fn render_all_rooms(editor: &mut CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, _response: &egui::Response, ctx: &egui::Context) {
    // First collect all the data we need to avoid borrowing issues
    let mut levels_to_render = Vec::new();
    let current_level_index = editor.current_level_index;

    // Extract levels data to avoid borrow checker issues
    if let Some(map) = &editor.map_data {
        // Find the levels element
        if let Some(children) = map["__children"].as_array() {
            for child in children {
                if child["__name"] == "levels" {
                    if let Some(levels) = child["__children"].as_array() {
                        // Store all the level data we need
                        for (i, level) in levels.iter().enumerate() {
                            if level["__name"] == "level" {
                                levels_to_render.push((i, level.clone()));
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    // Now render the levels without borrowing editor.map_data anymore
    if !levels_to_render.is_empty() {
        println!("Found {} levels to render in all-rooms mode", levels_to_render.len());

        // Phase 1: Draw non-selected rooms first
        for (i, level) in &levels_to_render {
            // Skip the currently selected room in this phase
            if *i == current_level_index {
                continue;
            }

            render_room(editor, painter, level, *i, scaled_tile_size, false, ctx);
        }

        // Phase 2: Draw the selected room on top if it exists
        if let Some((i, level)) = levels_to_render.iter().find(|(i, _)| *i == current_level_index) {
            render_room(editor, painter, level, *i, scaled_tile_size, true, ctx);
        }
    } else {
        println!("No levels found in all-rooms mode!");
    }
}

fn render_room(editor: &mut CelesteMapEditor, painter: &egui::Painter, level: &serde_json::Value,
               index: usize, scaled_tile_size: f32, is_selected: bool, ctx: &egui::Context) {
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
            let mut found_solids = false;
            for child in children {
                if child["__name"] == "solids" {
                    found_solids = true;
                    if let Some(solids_text) = child["innerText"].as_str() {
                        for (y, line) in solids_text.lines().enumerate() {
                            max_height = max_height.max(y + 1);
                            max_width = max_width.max(line.len());
                        }
                    }
                    break;
                }
            }

            if !found_solids {
                println!("No solids found for room at ({}, {})", room_x_tiles, room_y_tiles);
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
                        // Get offsets or default to 0
                        let offset_x = child["offsetX"].as_i64().unwrap_or(0) as i32;
                        let offset_y = child["offsetY"].as_i64().unwrap_or(0) as i32;

                        for (y, line) in solids_text.lines().enumerate() {
                            for (x, c) in line.chars().enumerate() {
                                if c != '0' && c != ' ' {
                                    let rect = Rect::from_min_size(
                                        Pos2::new(
                                            (room_x_tiles as f32 + x as f32 + offset_x as f32) * scaled_tile_size - editor.camera_pos.x,
                                            (room_y_tiles as f32 + y as f32 + offset_y as f32) * scaled_tile_size - editor.camera_pos.y,
                                        ),
                                        Vec2::new(scaled_tile_size, scaled_tile_size),
                                    );

                                    let mut used_texture = false;

                                    // Use textures if enabled and available
                                    if editor.use_textures {
                                        // Try using the atlas system first
                                        used_texture = editor.draw_sprite_for_tile(painter, rect, c);

                                        // Fall back to the old PNG texture loading
                                        if !used_texture {
                                            if let Some(texture_path) = editor.celeste_assets.get_texture_path_for_tile(c) {
                                                if let Some(texture_handle) = editor.celeste_assets.load_texture(texture_path, ctx) {
                                                    draw_texture(painter, rect, texture_handle.id(), Color32::WHITE);
                                                    used_texture = true;
                                                }
                                            }
                                        }
                                    }

                                    // Fall back to colored rectangle if no texture was used
                                    if !used_texture {
                                        // Enhanced color matching for both modded and base game tiles
                                        let color = match c {
                                            // Modded map tiles
                                            '9' => Color32::from_rgb(255, 255, 255), // Pure white
                                            'm' => Color32::from_rgb(150, 100, 50),  // Brown (mountain)
                                            'n' => Color32::from_rgb(50, 150, 100),  // Green (temple)
                                            'a' => Color32::from_rgb(150, 50, 150),  // Purple (core)

                                            // Base game tiles
                                            '1' => Color32::from_rgb(220, 220, 220), // Light gray
                                            '3' => Color32::from_rgb(200, 200, 200), // Medium gray
                                            '4' => Color32::from_rgb(180, 180, 180), // Darker gray
                                            '7' => Color32::from_rgb(160, 160, 160), // Even darker gray

                                            // Handle any other characters
                                            _ => SOLID_TILE_COLOR,
                                        };

                                        painter.rect_filled(rect, 0.0, color);

                                        // Only draw stroke for larger zoom levels
                                        if editor.zoom_level > 0.5 {
                                            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::BLACK));
                                        }
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
                    FontId::proportional(14.0),
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
                ui.separator();
                if ui.button("Set Celeste Path...").clicked() {
                    editor.show_celeste_path_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
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
                ui.checkbox(&mut editor.use_textures, "Use Textures");

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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
            render_all_rooms(editor, &painter, scaled_tile_size, &response, ctx);
        } else {
            render_current_room(editor, &painter, scaled_tile_size, ctx);
        }
    });
}