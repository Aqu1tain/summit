use eframe::egui;
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;

use crate::app::CelesteMapEditor;
use crate::map::loader::{save_map, save_map_as};

// Constants
pub const TILE_SIZE: f32 = 20.0;
pub const GRID_COLOR: Color32 = Color32::from_rgb(70, 70, 70);
pub const SOLID_TILE_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
pub const BG_COLOR: Color32 = Color32::from_rgb(30, 30, 30);

// Culling threshold based on zoom level
const CULLING_THRESHOLD_BASE: f32 = 50.0;

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

// A struct to cache level data for more efficient rendering
#[derive(Clone, Default)]
struct LevelRenderData {
    name: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    solids: Vec<Vec<char>>,
    offset_x: i32,
    offset_y: i32,
}

// Returns the color for a tile character
fn get_tile_color(tile_char: char) -> Color32 {
    match tile_char {
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
    }
}

// Render a single tile at the given position
fn render_tile(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    x: usize,
    y: usize,
    offset_x: i32,
    offset_y: i32,
    room_x_tiles: f32,
    room_y_tiles: f32,
    tile_char: char,
    scaled_tile_size: f32,
    is_visible: bool,
    ctx: &egui::Context,
) -> bool {
    if !is_visible || tile_char == '0' || tile_char == ' ' {
        return false;
    }

    let rect = Rect::from_min_size(
        Pos2::new(
            (room_x_tiles + x as f32 + offset_x as f32) * scaled_tile_size - editor.camera_pos.x,
            (room_y_tiles + y as f32 + offset_y as f32) * scaled_tile_size - editor.camera_pos.y,
        ),
        Vec2::new(scaled_tile_size, scaled_tile_size),
    );

    let mut used_texture = false;

    // Use textures if enabled and available
    if editor.use_textures {
        // Try using the atlas system first
        used_texture = editor.draw_sprite_for_tile(painter, rect, tile_char);

        // Fall back to the old PNG texture loading
        if !used_texture {
            if let Some(texture_path) = editor.celeste_assets.get_texture_path_for_tile(tile_char) {
                if let Some(texture_handle) = editor.celeste_assets.load_texture(texture_path, ctx) {
                    draw_texture(painter, rect, texture_handle.id(), Color32::WHITE);
                    used_texture = true;
                }
            }
        }
    }

    // Fall back to colored rectangle if no texture was used
    if !used_texture {
        let color = get_tile_color(tile_char);
        painter.rect_filled(rect, 0.0, color);

        // Only draw stroke for larger zoom levels
        if editor.zoom_level > 0.5 {
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::BLACK));
        }
    }

    true
}

// Batch render tiles by texture ID
fn batch_render_tiles(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    level_data: &LevelRenderData,
    scaled_tile_size: f32,
    view_rect: Rect,
    ctx: &egui::Context,
) {
    let room_x_tiles = level_data.x / 8.0;
    let room_y_tiles = level_data.y / 8.0;
    let offset_x = level_data.offset_x;
    let offset_y = level_data.offset_y;

    // Calculate the culling bounds based on view_rect with some margin
    let culling_margin = CULLING_THRESHOLD_BASE * (2.0 / editor.zoom_level.max(0.1));
    let culling_rect = view_rect.expand(culling_margin);

    // Calculate the tile range that's potentially visible
    let start_x = ((culling_rect.min.x + editor.camera_pos.x) / scaled_tile_size - room_x_tiles - offset_x as f32).max(0.0) as usize;
    let start_y = ((culling_rect.min.y + editor.camera_pos.y) / scaled_tile_size - room_y_tiles - offset_y as f32).max(0.0) as usize;
    let end_x = ((culling_rect.max.x + editor.camera_pos.x) / scaled_tile_size - room_x_tiles - offset_x as f32 + 1.0) as usize;
    let end_y = ((culling_rect.max.y + editor.camera_pos.y) / scaled_tile_size - room_y_tiles - offset_y as f32 + 1.0) as usize;

    // Group tiles by character for batching
    let mut tile_groups: HashMap<char, Vec<(usize, usize)>> = HashMap::new();

    for y in start_y..end_y {
        if y >= level_data.solids.len() {
            continue;
        }
        let line = &level_data.solids[y];

        for x in start_x..end_x {
            if x >= line.len() {
                continue;
            }

            let c = line[x];
            if c != '0' && c != ' ' {
                tile_groups.entry(c).or_default().push((x, y));
            }
        }
    }

    // Render each group of tiles
    for (tile_char, positions) in tile_groups {
        for &(x, y) in &positions {
            let tile_rect = Rect::from_min_size(
                Pos2::new(
                    (room_x_tiles + x as f32 + offset_x as f32) * scaled_tile_size - editor.camera_pos.x,
                    (room_y_tiles + y as f32 + offset_y as f32) * scaled_tile_size - editor.camera_pos.y,
                ),
                Vec2::new(scaled_tile_size, scaled_tile_size),
            );

            // Skip tiles that aren't visible in the viewport
            if !tile_rect.intersects(view_rect) {
                continue;
            }

            render_tile(
                editor,
                painter,
                x,
                y,
                offset_x,
                offset_y,
                room_x_tiles,
                room_y_tiles,
                tile_char,
                scaled_tile_size,
                true,
                ctx,
            );
        }
    }
}

// Extract level data from JSON value
fn extract_level_data(level: &serde_json::Value) -> Option<LevelRenderData> {
    let room_x = level["x"].as_f64()?;
    let room_y = level["y"].as_f64()?;

    // Convert pixel sizes to tile counts (divide by 8)
    let room_width_pixels = level.get("width").and_then(|w| w.as_f64()).unwrap_or(320.0);
    let room_height_pixels = level.get("height").and_then(|h| h.as_f64()).unwrap_or(184.0);

    let mut solids = Vec::new();
    let mut offset_x = 0;
    let mut offset_y = 0;

    if let Some(children) = level["__children"].as_array() {
        for child in children {
            if child["__name"] == "solids" {
                offset_x = child["offsetX"].as_i64().unwrap_or(0) as i32;
                offset_y = child["offsetY"].as_i64().unwrap_or(0) as i32;

                if let Some(solids_text) = child["innerText"].as_str() {
                    for line in solids_text.lines() {
                        solids.push(line.chars().collect());
                    }
                }
                break;
            }
        }
    }

    let name = level["name"].as_str().unwrap_or("").to_string();

    Some(LevelRenderData {
        name,
        x: room_x as f32,
        y: room_y as f32,
        width: room_width_pixels as f32,
        height: room_height_pixels as f32,
        solids,
        offset_x,
        offset_y,
    })
}

fn render_current_room(editor: &mut CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, view_rect: Rect, ctx: &egui::Context) {
    if let Some(level) = editor.get_current_level() {
        // Debug logging only in debug mode
        #[cfg(debug_assertions)]
        if let Some(name) = level["name"].as_str() {
            println!("Current level: {}", name);
        }

        // Extract level data
        if let Some(level_data) = extract_level_data(&level) {
            #[cfg(debug_assertions)]
            println!("Found solids with {} lines", level_data.solids.len());

            // Batch render tiles
            batch_render_tiles(editor, painter, &level_data, scaled_tile_size, view_rect, ctx);

            #[cfg(debug_assertions)]
            println!("Rendered solid tiles");
        } else {
            #[cfg(debug_assertions)]
            println!("No 'solids' data found!");
        }
    }
}

fn render_all_rooms(editor: &mut CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, response: &egui::Response, ctx: &egui::Context) {
    let view_rect = response.rect; // the current viewport
    let mut levels_to_render = Vec::new();
    let current_level_index = editor.current_level_index;

    // Cache for level render data
    let mut level_data_cache = Vec::new();

    // Collect level data
    if let Some(map) = &editor.map_data {
        if let Some(children) = map["__children"].as_array() {
            for child in children {
                if child["__name"] == "levels" {
                    if let Some(levels) = child["__children"].as_array() {
                        for (i, level) in levels.iter().enumerate() {
                            if level["__name"] == "level" {
                                if let Some(level_data) = extract_level_data(level) {
                                    level_data_cache.push(level_data);
                                    levels_to_render.push(i);
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    if !levels_to_render.is_empty() {
        // Render non-selected rooms first
        for &i in &levels_to_render {
            if i == current_level_index {
                continue;
            }

            if i >= level_data_cache.len() {
                continue;
            }

            let level_data = &level_data_cache[i];

            // Calculate room rectangle
            let room_x_tiles = level_data.x / 8.0;
            let room_y_tiles = level_data.y / 8.0;
            let room_width = level_data.width / 8.0;
            let room_height = level_data.height / 8.0;

            let room_rect = egui::Rect::from_min_size(
                egui::Pos2::new(
                    room_x_tiles * scaled_tile_size - editor.camera_pos.x,
                    room_y_tiles * scaled_tile_size - editor.camera_pos.y,
                ),
                egui::Vec2::new(room_width * scaled_tile_size, room_height * scaled_tile_size)
            );

            // Culling check - only render if room is visible (or near visible)
            let culling_margin = CULLING_THRESHOLD_BASE * (2.0 / editor.zoom_level.max(0.1));
            if !room_rect.intersects(view_rect.expand(culling_margin)) {
                continue;
            }

            // Draw room boundary
            let boundary_color = Color32::from_rgb(200, 100, 100); // Red for non-selected
            painter.rect_stroke(room_rect, 0.0, Stroke::new(2.0, boundary_color));

            // Batch render tiles
            batch_render_tiles(editor, painter, level_data, scaled_tile_size, view_rect, ctx);

            // Draw room name if enabled
            if editor.show_labels {
                painter.text(
                    Pos2::new(
                        room_rect.min.x + 5.0,
                        room_rect.min.y + 5.0,
                    ),
                    egui::Align2::LEFT_TOP,
                    &level_data.name,
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }
        }

        // Render selected room on top
        if let Some(&i) = levels_to_render.iter().find(|&&i| i == current_level_index) {
            if i < level_data_cache.len() {
                let level_data = &level_data_cache[i];

                // Calculate room rectangle
                let room_x_tiles = level_data.x / 8.0;
                let room_y_tiles = level_data.y / 8.0;
                let room_width = level_data.width / 8.0;
                let room_height = level_data.height / 8.0;

                let room_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(
                        room_x_tiles * scaled_tile_size - editor.camera_pos.x,
                        room_y_tiles * scaled_tile_size - editor.camera_pos.y,
                    ),
                    egui::Vec2::new(room_width * scaled_tile_size, room_height * scaled_tile_size)
                );

                // Draw room boundary
                let boundary_color = Color32::from_rgb(100, 200, 100); // Green for selected
                painter.rect_stroke(room_rect, 0.0, Stroke::new(3.0, boundary_color));

                // Batch render tiles
                batch_render_tiles(editor, painter, level_data, scaled_tile_size, view_rect, ctx);

                // Draw room name if enabled
                if editor.show_labels {
                    painter.text(
                        Pos2::new(
                            room_rect.min.x + 5.0,
                            room_rect.min.y + 5.0,
                        ),
                        egui::Align2::LEFT_TOP,
                        &level_data.name,
                        FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                }
            }
        }
    }
}

// Draw the grid efficiently
fn draw_grid(painter: &egui::Painter, response: &egui::Response, camera_pos: Vec2, scaled_tile_size: f32, zoom_level: f32) {
    // Skip grid rendering at very low zoom levels for performance
    if zoom_level < 0.2 {
        return;
    }

    let grid_start_x = camera_pos.x % scaled_tile_size;
    let grid_start_y = camera_pos.y % scaled_tile_size;

    // Calculate grid step based on zoom level
    let grid_step = if zoom_level < 0.5 { 2 } else { 1 };

    // Calculate grid stroke thickness based on zoom level
    let stroke_thickness = if zoom_level < 0.5 { 0.5 } else { 1.0 };

    // Vertical lines
    for i in (0..((response.rect.width() / scaled_tile_size) as i32 + 2)).step_by(grid_step) {
        let x = i as f32 * scaled_tile_size - grid_start_x;
        painter.line_segment(
            [Pos2::new(x, 0.0), Pos2::new(x, response.rect.height())],
            Stroke::new(stroke_thickness, GRID_COLOR),
        );
    }

    // Horizontal lines
    for i in (0..((response.rect.height() / scaled_tile_size) as i32 + 2)).step_by(grid_step) {
        let y = i as f32 * scaled_tile_size - grid_start_y;
        painter.line_segment(
            [Pos2::new(0.0, y), Pos2::new(response.rect.width(), y)],
            Stroke::new(stroke_thickness, GRID_COLOR),
        );
    }
}

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

        // Draw grid if enabled
        if editor.show_grid {
            draw_grid(&painter, &response, editor.camera_pos, scaled_tile_size, editor.zoom_level);
        }

        if editor.show_all_rooms {
            render_all_rooms(editor, &painter, scaled_tile_size, &response, ctx);
        } else {
            render_current_room(editor, &painter, scaled_tile_size, response.rect, ctx);
        }
    });
}