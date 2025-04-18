#![allow(dead_code, unused_imports, unused_variables)]

use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use crate::app::CelesteMapEditor;
use crate::map::loader::{save_map, save_map_as};

// Constants
pub const TILE_SIZE: f32 = 20.0;
pub const GRID_COLOR: Color32 = Color32::from_rgb(70, 70, 70);
pub const SOLID_TILE_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
pub const BG_COLOR: Color32 = Color32::from_rgb(30, 30, 30);
pub const INFILL_COLOR: Color32 = Color32::from_rgb(40, 36, 60); // distinct from BG_COLOR
pub const EXTERNAL_BORDER_COLOR: Color32 = Color32::from_rgb(220, 220, 220);
pub const ROOM_CONTOUR_SELECTED: Color32 = Color32::from_rgb(110, 130, 170); // gray-blueish
pub const ROOM_CONTOUR_UNSELECTED: Color32 = Color32::from_rgb(60, 120, 220); // blue

// Culling threshold based on zoom level
const CULLING_THRESHOLD_BASE: f32 = 50.0;

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
        // Brown dirt
        '1' => Color32::from_rgb(156, 102, 31),   // brown
        // Blue rocks
        '2' => Color32::from_rgb(70, 120, 200),   // blue
        // Gray metal beams
        '3' => Color32::from_rgb(130, 130, 130),  // gray
        // Green-gray stones
        '4' => Color32::from_rgb(100, 130, 100),  // green-gray
        // Handle any other characters
        _ => SOLID_TILE_COLOR,
    }
}

// Helper to check if a tile is solid (part of any tileset)
fn is_solid_tile(c: char) -> bool {
    matches!(c, '1' | '2' | '3' | '4')
}

// Defensive: don't render if x is out of bounds for this row
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
    _ctx: &egui::Context,
) -> bool {
    if !is_visible || tile_char == '0' || tile_char == ' ' {
        return false;
    }

    let level_data = editor.get_current_level().and_then(|level| extract_level_data(level));
    if let Some(level_data) = &level_data {
        if y >= level_data.solids.len() || x >= level_data.solids[y].len() {
            return false;
        }
    }

    // Infill if all 8 neighbors are either solid or out-of-bounds (room-local)
    let is_internal = {
        if let Some(level_data) = &level_data {
            let solids = &level_data.solids;
            let max_y = solids.len();
            let mut internal = true;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    // If neighbor is out-of-bounds (relative to this room's solids), treat as solid
                    if ny < 0 || nx < 0 || ny as usize >= max_y {
                        continue;
                    }
                    let row = &solids[ny as usize];
                    if nx as usize >= row.len() {
                        continue; // out of this room's row = solid
                    }
                    if !is_solid_tile(row[nx as usize]) {
                        internal = false;
                        break;
                    }
                }
                if !internal { break; }
            }
            internal
        } else {
            false
        }
    };

    let rect = Rect::from_min_size(
        Pos2::new(
            (room_x_tiles + x as f32 + offset_x as f32) * scaled_tile_size - editor.camera_pos.x,
            (room_y_tiles + y as f32 + offset_y as f32) * scaled_tile_size - editor.camera_pos.y,
        ),
        Vec2::new(scaled_tile_size, scaled_tile_size),
    );

    // Tiles: always render as colored blocks (no textures for solid tiles)
    let color = if is_internal {
        INFILL_COLOR
    } else {
        get_tile_color(tile_char)
    };
    painter.rect_filled(rect, 0.0, color);

    // Draw external borders (light gray) just outside the tile if the neighbor is not solid
    if let Some(level_data) = &level_data {
        let solids = &level_data.solids;
        let max_y = solids.len();
        // Up
        let up_external = !(y > 0 && x < solids[y-1].len() && is_solid_tile(solids[y-1][x]));
        if up_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.left(), rect.top() - 1.0),
                Vec2::new(scaled_tile_size, 1.0)
            );
            painter.rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR);
        }
        // Down
        let down_external = !(y+1 < max_y && x < solids[y+1].len() && is_solid_tile(solids[y+1][x]));
        if down_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.left(), rect.bottom()),
                Vec2::new(scaled_tile_size, 1.0)
            );
            painter.rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR);
        }
        // Left
        let left_external = !(x > 0 && x-1 < solids[y].len() && is_solid_tile(solids[y][x-1]));
        if left_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.left() - 1.0, rect.top()),
                Vec2::new(1.0, scaled_tile_size)
            );
            painter.rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR);
        }
        // Right
        let right_external = !(x+1 < solids[y].len() && is_solid_tile(solids[y][x+1]));
        if right_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.right(), rect.top()),
                Vec2::new(1.0, scaled_tile_size)
            );
            painter.rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR);
        }
    }

    true
}

// Render all visible tiles in the level (grayboxing only)
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

    let culling_margin = CULLING_THRESHOLD_BASE * (2.0 / editor.zoom_level.max(0.1));
    let culling_rect = view_rect.expand(culling_margin);

    let start_x = ((culling_rect.min.x + editor.camera_pos.x) / scaled_tile_size - room_x_tiles - offset_x as f32).max(0.0) as usize;
    let start_y = ((culling_rect.min.y + editor.camera_pos.y) / scaled_tile_size - room_y_tiles - offset_y as f32).max(0.0) as usize;
    let end_x = ((culling_rect.max.x + editor.camera_pos.x) / scaled_tile_size - room_x_tiles - offset_x as f32 + 1.0) as usize;
    let end_y = ((culling_rect.max.y + editor.camera_pos.y) / scaled_tile_size - room_y_tiles - offset_y as f32 + 1.0) as usize;

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
            render_tile(
                editor,
                painter,
                x,
                y,
                offset_x,
                offset_y,
                room_x_tiles,
                room_y_tiles,
                c,
                scaled_tile_size,
                true,
                ctx,
            );
        }
    }
}

// Render all visible tiles in the level (grayboxing or fgtiles mode) to shapes
fn batch_render_tiles_to_shapes(
    editor: &mut CelesteMapEditor,
    shapes: &mut Vec<egui::Shape>,
    level_data: &LevelRenderData,
    scaled_tile_size: f32,
    view_rect: Rect,
) {
    let room_x_tiles = level_data.x / 8.0;
    let room_y_tiles = level_data.y / 8.0;
    let offset_x = level_data.offset_x;
    let offset_y = level_data.offset_y;

    let culling_margin = CULLING_THRESHOLD_BASE * (2.0 / editor.zoom_level.max(0.1));
    let culling_rect = view_rect.expand(culling_margin);

    let start_x = ((culling_rect.min.x + editor.camera_pos.x) / scaled_tile_size - room_x_tiles - offset_x as f32).max(0.0) as usize;
    let start_y = ((culling_rect.min.y + editor.camera_pos.y) / scaled_tile_size - room_y_tiles - offset_y as f32).max(0.0) as usize;
    let end_x = ((culling_rect.max.x + editor.camera_pos.x) / scaled_tile_size - room_x_tiles - offset_x as f32 + 1.0) as usize;
    let end_y = ((culling_rect.max.y + editor.camera_pos.y) / scaled_tile_size - room_y_tiles - offset_y as f32 + 1.0) as usize;

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
            render_tile_to_shapes(
                editor,
                shapes,
                x,
                y,
                offset_x,
                offset_y,
                room_x_tiles,
                room_y_tiles,
                c,
                scaled_tile_size,
            );
        }
    }
}

// Render a tile to shapes
fn render_tile_to_shapes(
    editor: &mut CelesteMapEditor,
    shapes: &mut Vec<egui::Shape>,
    x: usize,
    y: usize,
    offset_x: i32,
    offset_y: i32,
    room_x_tiles: f32,
    room_y_tiles: f32,
    tile_char: char,
    scaled_tile_size: f32,
) {
    let level_data = editor.get_current_level().and_then(|level| extract_level_data(level));
    if let Some(level_data) = &level_data {
        if y >= level_data.solids.len() || x >= level_data.solids[y].len() {
            return;
        }
    }

    // Infill if all 8 neighbors are either solid or out-of-bounds (room-local)
    let is_internal = {
        if let Some(level_data) = &level_data {
            let solids = &level_data.solids;
            let max_y = solids.len();
            let mut internal = true;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    // If neighbor is out-of-bounds (relative to this room's solids), treat as solid
                    if ny < 0 || nx < 0 || ny as usize >= max_y {
                        continue;
                    }
                    let row = &solids[ny as usize];
                    if nx as usize >= row.len() {
                        continue; // out of this room's row = solid
                    }
                    if !is_solid_tile(row[nx as usize]) {
                        internal = false;
                        break;
                    }
                }
                if !internal { break; }
            }
            internal
        } else {
            false
        }
    };

    let rect = Rect::from_min_size(
        Pos2::new(
            (room_x_tiles + x as f32 + offset_x as f32) * scaled_tile_size - editor.camera_pos.x,
            (room_y_tiles + y as f32 + offset_y as f32) * scaled_tile_size - editor.camera_pos.y,
        ),
        Vec2::new(scaled_tile_size, scaled_tile_size),
    );

    // Tiles: always render as colored blocks (no textures for solid tiles)
    let color = if is_internal {
        INFILL_COLOR
    } else {
        get_tile_color(tile_char)
    };
    shapes.push(egui::Shape::rect_filled(rect, 0.0, color));

    // Draw external borders (light gray) just outside the tile if the neighbor is not solid
    if let Some(level_data) = &level_data {
        let solids = &level_data.solids;
        let max_y = solids.len();
        // Up
        let up_external = !(y > 0 && x < solids[y-1].len() && is_solid_tile(solids[y-1][x]));
        if up_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.left(), rect.top() - 1.0),
                Vec2::new(scaled_tile_size, 1.0)
            );
            shapes.push(egui::Shape::rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR));
        }
        // Down
        let down_external = !(y+1 < max_y && x < solids[y+1].len() && is_solid_tile(solids[y+1][x]));
        if down_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.left(), rect.bottom()),
                Vec2::new(scaled_tile_size, 1.0)
            );
            shapes.push(egui::Shape::rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR));
        }
        // Left
        let left_external = !(x > 0 && x-1 < solids[y].len() && is_solid_tile(solids[y][x-1]));
        if left_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.left() - 1.0, rect.top()),
                Vec2::new(1.0, scaled_tile_size)
            );
            shapes.push(egui::Shape::rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR));
        }
        // Right
        let right_external = !(x+1 < solids[y].len() && is_solid_tile(solids[y][x+1]));
        if right_external {
            let border_rect = Rect::from_min_size(
                Pos2::new(rect.right(), rect.top()),
                Vec2::new(1.0, scaled_tile_size)
            );
            shapes.push(egui::Shape::rect_filled(border_rect, 0.0, EXTERNAL_BORDER_COLOR));
        }
    }
}

// Extract level data from JSON value
fn extract_level_data(level: &serde_json::Value) -> Option<LevelRenderData> {
    let room_x = level["x"].as_f64()?;
    let room_y = level["y"].as_f64()?;

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

// Loenn-style decal normalization: "scenery/foo.png" -> "decals/scenery/foo"
fn normalize_decal_path(texture: &str) -> String {
    let mut key = texture.replace("\\", "/");
    if key.ends_with(".png") {
        key.truncate(key.len() - 4);
    }
    if !key.starts_with("decals/") {
        key = format!("decals/{}", key);
    }
    key
}

// Render all fgdecals for the current room (Loenn-style lookup)
fn render_fgdecals(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    level: &serde_json::Value,
    scale: f32,
    ctx: &egui::Context,
    room_x: f32,
    room_y: f32,
) {
    if let Some(children) = level["__children"].as_array() {
        for child in children {
            if child["__name"] == "fgdecals" {
                if let Some(decals) = child["__children"].as_array() {
                    for decal in decals {
                        if decal["__name"] == "decal" {
                            let texture = decal["texture"].as_str().unwrap_or("");
                            let path = normalize_decal_path(texture);
                            let x = decal["x"].as_f64().unwrap_or(0.0) as f32;
                            let y = decal["y"].as_f64().unwrap_or(0.0) as f32;
                            let scale_x = decal["scaleX"].as_f64().unwrap_or(1.0) as f32;
                            let scale_y = decal["scaleY"].as_f64().unwrap_or(1.0) as f32;
                            println!("[DECAL] Looking for sprite '{}': pos=({}, {}), scale=({}, {})", path, x, y, scale_x, scale_y);
                            if let Some((_atlas, sprite)) = crate::celeste_atlas::AtlasManager::get_sprite_global(&path) {
                                println!("[DECAL] Found sprite for '{}', drawing...", path);
                                let pos = egui::Pos2::new(
                                    (room_x + x) * scale * editor.zoom_level - editor.camera_pos.x,
                                    (room_y + y) * scale * editor.zoom_level - editor.camera_pos.y,
                                );
                                let size = egui::Vec2::new(
                                    (sprite.metadata.width as f32) * scale_x * scale * editor.zoom_level,
                                    (sprite.metadata.height as f32) * scale_y * scale * editor.zoom_level,
                                );
                                crate::celeste_atlas::AtlasManager::draw_sprite(&editor.atlas_manager.as_ref().unwrap(), &sprite, painter, egui::Rect::from_min_size(pos, size), egui::Color32::WHITE);
                            } else {
                                println!("[DECAL] Sprite '{}' not found in global mapping!", path);
                            }
                        }
                    }
                }
            }
        }
    }
}

// Render all fgdecals for the current room (Loenn-style lookup) to shapes
fn render_fgdecals_to_shapes(
    editor: &mut CelesteMapEditor,
    shapes: &mut Vec<egui::Shape>,
    level: &serde_json::Value,
    scale: f32,
    room_x: f32,
    room_y: f32,
) {
    if let Some(children) = level["__children"].as_array() {
        for child in children {
            if child["__name"] == "fgdecals" {
                if let Some(decals) = child["__children"].as_array() {
                    for decal in decals {
                        if decal["__name"] == "decal" {
                            let texture = decal["texture"].as_str().unwrap_or("");
                            let path = normalize_decal_path(texture);
                            let x = decal["x"].as_f64().unwrap_or(0.0) as f32;
                            let y = decal["y"].as_f64().unwrap_or(0.0) as f32;
                            let scale_x = decal["scaleX"].as_f64().unwrap_or(1.0) as f32;
                            let scale_y = decal["scaleY"].as_f64().unwrap_or(1.0) as f32;
                            println!("[DECAL] Looking for sprite '{}': pos=({}, {}), scale=({}, {})", path, x, y, scale_x, scale_y);
                            if let Some((_atlas, sprite)) = crate::celeste_atlas::AtlasManager::get_sprite_global(&path) {
                                println!("[DECAL] Found sprite for '{}', drawing...", path);
                                let pos = egui::Pos2::new(
                                    (room_x + x) * scale * editor.zoom_level - editor.camera_pos.x,
                                    (room_y + y) * scale * editor.zoom_level - editor.camera_pos.y,
                                );
                                let size = egui::Vec2::new(
                                    (sprite.metadata.width as f32) * scale_x * scale * editor.zoom_level,
                                    (sprite.metadata.height as f32) * scale_y * scale * editor.zoom_level,
                                );
                                let rect = egui::Rect::from_min_size(pos, size);
                            } else {
                                println!("[DECAL] Sprite '{}' not found in global mapping!", path);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_room_content(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    level_data: &LevelRenderData,
    level_json: &serde_json::Value,
    scaled_tile_size: f32,
    view_rect: Rect,
    ctx: &egui::Context,
) {
    batch_render_tiles(editor, painter, level_data, scaled_tile_size, view_rect, ctx);
    let scale = TILE_SIZE / 8.0;
    render_fgdecals(editor, painter, level_json, scale, ctx, level_data.x, level_data.y);
}

fn render_current_room(editor: &mut CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, view_rect: Rect, ctx: &egui::Context) {
    if let Some(level) = editor.get_current_level().cloned() {
        if let Some(level_data) = extract_level_data(&level) {
            render_room_content(editor, painter, &level_data, &level, scaled_tile_size, view_rect, ctx);
        }
    }
}

fn render_all_rooms(editor: &mut CelesteMapEditor, painter: &egui::Painter, scaled_tile_size: f32, response: &egui::Response, ctx: &egui::Context) {
    let view_rect = response.rect;
    let current_level_index = editor.current_level_index;
    let mut levels_to_render = Vec::new();
    let scale = TILE_SIZE / 8.0;

    // Recursively collect all level nodes, robust to JSON structure
    fn collect_levels<'a>(node: &'a serde_json::Value, levels: &mut Vec<(&'a serde_json::Value, usize)>, index: &mut usize) {
        if let Some(name) = node["__name"].as_str() {
            if name == "level" {
                levels.push((node, *index));
                *index += 1;
            }
        }
        if let Some(children) = node["__children"].as_array() {
            for child in children {
                collect_levels(child, levels, index);
            }
        }
    }

    if let Some(map) = &editor.map_data {
        let mut found_levels = Vec::new();
        let mut idx = 0;
        collect_levels(map, &mut found_levels, &mut idx);
        for (level, i) in found_levels {
            if let Some(level_data) = extract_level_data(level) {
                levels_to_render.push((i, level_data));
            }
        }
    }

    // Only recompute shapes if static_dirty is set, unless tile rendering is disabled
    if !editor.show_tiles {
        // Tile rendering disabled: skip all tile and sprite drawing
    } else if editor.static_dirty {
        let mut shapes = Vec::new();
        let mut sprite_cmds: Vec<crate::app::SpriteDrawCommand> = Vec::new();
        let render_decals = editor.show_fgdecals;
        let level_clones: Vec<_> = if let Some(map) = &editor.map_data {
            levels_to_render.iter()
                .filter_map(|(_i, level_data)| find_level_json_by_name(map, &level_data.name).cloned())
                .collect()
        } else {
            Vec::new()
        };
        for (idx, (_i, level_data)) in levels_to_render.iter().enumerate() {
            // Only add solid shapes if toggle is enabled
            if editor.show_solid_tiles {
                batch_render_tiles_to_shapes(editor, &mut shapes, level_data, scaled_tile_size, view_rect);
            }
            if render_decals {
                if let Some(level) = level_clones.get(idx) {
                    let scale = TILE_SIZE / 8.0;
                    render_fgdecals(editor, painter, level, scale, ctx, level_data.x, level_data.y);
                }
            }
        }
        editor.static_shapes = Some(shapes);
        editor.static_sprites = Some(sprite_cmds);
        editor.static_dirty = false;
    }
    // Always extend painter with cached shapes if enabled
    if editor.show_tiles {
        if editor.show_solid_tiles {
            if let Some(shapes) = &editor.static_shapes {
                painter.extend(shapes.clone());
            }
        }
        // Always draw cached sprites
        if let Some(sprite_cmds) = &editor.static_sprites {
            for cmd in sprite_cmds {
                if let Some((_atlas, sprite)) = crate::celeste_atlas::AtlasManager::get_sprite_global(&cmd.sprite_path) {
                    crate::celeste_atlas::AtlasManager::draw_sprite(
                        &editor.atlas_manager.as_ref().unwrap(),
                        &sprite,
                        painter,
                        egui::Rect::from_min_size(cmd.pos, cmd.size),
                        cmd.tint,
                    );
                }
            }
        }
    }

    // Draw the grid above tiles, but below contours/text
    if editor.show_grid {
        draw_grid(painter, response, editor.camera_pos, scaled_tile_size, editor.zoom_level);
    }

    // Draw room contours (strokes) above grid
    for (i, level_data) in &levels_to_render {
        let room_rect = Rect::from_min_size(
            Pos2::new(
                (level_data.x + level_data.offset_x as f32) * scale * editor.zoom_level - editor.camera_pos.x,
                (level_data.y + level_data.offset_y as f32) * scale * editor.zoom_level - editor.camera_pos.y,
            ),
            Vec2::new(level_data.width * scale * editor.zoom_level, level_data.height * scale * editor.zoom_level),
        );
        let is_selected = *i == current_level_index;
        let color = if is_selected { ROOM_CONTOUR_SELECTED } else { ROOM_CONTOUR_UNSELECTED };
        let thickness = if is_selected { 3.0 } else { 2.0 };
        painter.rect_stroke(room_rect, 0.0, Stroke::new(thickness, color));
    }

    // Draw room labels last, on top of grid and contours
    for (_i, level_data) in &levels_to_render {
        if editor.show_labels {
            let room_rect = Rect::from_min_size(
                Pos2::new(
                    (level_data.x + level_data.offset_x as f32) * scale * editor.zoom_level - editor.camera_pos.x,
                    (level_data.y + level_data.offset_y as f32) * scale * editor.zoom_level - editor.camera_pos.y,
                ),
                Vec2::new(level_data.width * scale * editor.zoom_level, level_data.height * scale * editor.zoom_level),
            );
            painter.text(
                Pos2::new(
                    room_rect.min.x + 5.0,
                    room_rect.min.y + 5.0,
                ),
                egui::Align2::LEFT_TOP,
                &level_data.name,
                egui::FontId::proportional(16.0),
                Color32::WHITE,
            );
        }
    }
}

// Draw the grid efficiently
fn draw_grid(painter: &egui::Painter, response: &egui::Response, camera_pos: Vec2, scaled_tile_size: f32, zoom_level: f32) {
    if zoom_level < 0.2 {
        return;
    }

    let grid_start_x = camera_pos.x % scaled_tile_size;
    let grid_start_y = camera_pos.y % scaled_tile_size;

    let grid_step = if zoom_level < 0.5 { 2 } else { 1 };

    let stroke_thickness = if zoom_level < 0.5 { 0.5 } else { 1.0 };

    for i in (0..((response.rect.width() / scaled_tile_size) as i32 + 2)).step_by(grid_step) {
        let x = i as f32 * scaled_tile_size - grid_start_x;
        painter.line_segment(
            [Pos2::new(x, 0.0), Pos2::new(x, response.rect.height())],
            Stroke::new(stroke_thickness, GRID_COLOR),
        );
    }

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
                let prev_fgdecals = editor.show_fgdecals;
                ui.checkbox(&mut editor.show_fgdecals, "Show Foreground Decals");
                if prev_fgdecals != editor.show_fgdecals {
                    editor.static_dirty = true;
                }
                if ui.checkbox(&mut editor.show_tiles, "Show Tiles").changed() {
                    editor.static_dirty = true;
                }
                ui.checkbox(&mut editor.show_all_rooms, "Show All Rooms");
                ui.checkbox(&mut editor.show_grid, "Show Grid");
                ui.checkbox(&mut editor.show_labels, "Show Room Labels");

                ui.separator();

                if ui.button("Zoom In").clicked() {
                    editor.zoom_level *= 1.2;
                    editor.static_dirty = true;
                    ui.close_menu();
                }
                if ui.button("Zoom Out").clicked() {
                    editor.zoom_level /= 1.2;
                    if editor.zoom_level < 0.1 {
                        editor.zoom_level = 0.1;
                    }
                    editor.static_dirty = true;
                    ui.close_menu();
                }
                if ui.button("Reset Zoom").clicked() {
                    editor.zoom_level = 1.0;
                    editor.static_dirty = true;
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
            egui::Sense::hover(),
        );

        editor.mouse_pos = response.hover_pos().unwrap_or_default();

        painter.rect_filled(
            response.rect,
            0.0,
            BG_COLOR,
        );

        let scaled_tile_size = TILE_SIZE * editor.zoom_level;

        if editor.show_all_rooms {
            render_all_rooms(editor, &painter, scaled_tile_size, &response, ctx);
        } else {
            render_current_room(editor, &painter, scaled_tile_size, response.rect, ctx);
            // Draw grid on top if not in all rooms mode
            if editor.show_grid {
                draw_grid(&painter, &response, editor.camera_pos, scaled_tile_size, editor.zoom_level);
            }
        }
    });
}

// Helper: Find a level JSON node by name
fn find_level_json_by_name<'a>(map: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    if let Some(children) = map["__children"].as_array() {
        for child in children {
            if child["__name"].as_str() == Some("levels") {
                if let Some(levels) = child["__children"].as_array() {
                    for level in levels {
                        if level["name"].as_str() == Some(name) {
                            return Some(level);
                        }
                    }
                }
            }
        }
    }
    None
}