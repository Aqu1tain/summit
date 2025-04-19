#![allow(dead_code, unused_imports, unused_variables)]

use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use crate::app::CelesteMapEditor;
use crate::map::loader::{save_map, save_map_as};
use crate::tile_xml::{load_tileset_id_path_map, get_tileset_path_for_id, ensure_tileset_id_path_map_loaded_from_celeste, get_first_tile_coords_for_id_or_default, get_tilesets_with_rules};
use crate::celeste_atlas::AtlasManager;

// Constants
pub const TILE_SIZE: f32 = 20.0;
pub const GRID_COLOR: Color32 = Color32::from_rgb(70, 70, 70);
pub const SOLID_TILE_COLOR: Color32 = Color32::from_rgb(200, 200, 200);
pub const BG_COLOR: Color32 = Color32::from_rgb(30, 30, 30);
pub const INFILL_COLOR: Color32 = Color32::from_rgb(40, 36, 60);
pub const EXTERNAL_BORDER_COLOR: Color32 = Color32::from_rgb(220, 220, 220);
pub const ROOM_CONTOUR_SELECTED: Color32 = Color32::from_rgb(110, 130, 170);
pub const ROOM_CONTOUR_UNSELECTED: Color32 = Color32::from_rgb(60, 120, 220);

const DECAL_SCALE: f32 = 1.0;
// Culling threshold based on zoom level
const CULLING_THRESHOLD_BASE: f32 = 50.0;

// Cached representation for rendering
#[derive(Clone, Default)]
pub struct LevelRenderData {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub solids: Vec<Vec<char>>,
    pub offset_x: i32,
    pub offset_y: i32,
    pub autotile_coords: Vec<Vec<Option<(u32, u32)>>>, // cache for autotiling
}

impl LevelRenderData {
    pub fn compute_autotile_coords(&mut self, xml_path: &str) {
        let tilesets = crate::tile_xml::get_tilesets_with_rules(xml_path);
        let is_solid = |c: char| is_solid_tile(c);
        self.autotile_coords = self.solids.iter().enumerate().map(|(y, row)| {
            row.iter().enumerate().map(|(x, &tile)| {
                crate::tile_xml::autotile_tile_coord(tile, &self.solids, x, y, tilesets, &is_solid)
            }).collect()
        }).collect();
    }
}

// Add a static or cached tileset id→path mapping
use std::sync::OnceLock;
static TILESET_ID_PATH_MAP: OnceLock<std::collections::HashMap<char, String>> = OnceLock::new();

/// Call this at startup or before rendering to load the mapping
pub fn ensure_tileset_id_path_map_loaded(xml_path: &str) {
    TILESET_ID_PATH_MAP.get_or_init(|| load_tileset_id_path_map(xml_path));
}

/// Returns the color for a tile character, or None if a texture should be used.
fn get_tile_color(tile_char: char) -> Option<Color32> {
    //match tile_char {
    //    '1' => Some(Color32::from_rgb(156, 102, 31)),
    //    '2' => Some(Color32::from_rgb(70, 120, 200)),
    //    '3' => Some(Color32::from_rgb(130, 130, 130)),
    //    '4' => Some(Color32::from_rgb(100, 130, 100)),
    //    _ => None, // Use texture if available
    //}
    None
}

/// Is this a solid tile?
fn is_solid_tile(c: char) -> bool {
    matches!(c, '1' | '2' | '3' | '4')
}

/// Extract level data from JSON node.
pub(crate) fn extract_level_data(level: &serde_json::Value, editor: &CelesteMapEditor) -> Option<LevelRenderData> {
    let x = level["x"].as_f64()? as f32;
    let y = level["y"].as_f64()? as f32;
    let width = level.get("width").and_then(|v| v.as_f64()).unwrap_or(320.0) as f32;
    let height = level.get("height").and_then(|v| v.as_f64()).unwrap_or(184.0) as f32;

    let mut solids = Vec::new();
    let mut offset_x = 0;
    let mut offset_y = 0;
    if let Some(children) = level["__children"].as_array() {
        for child in children {
            if child["__name"] == "solids" {
                offset_x = child["offsetX"].as_i64().unwrap_or(0) as i32;
                offset_y = child["offsetY"].as_i64().unwrap_or(0) as i32;
                if let Some(text) = child["innerText"].as_str() {
                    for line in text.lines() {
                        solids.push(line.chars().collect());
                    }
                }
                break;
            }
        }
    }
    let name = level["name"].as_str().unwrap_or("").to_string();
    let mut ld = LevelRenderData { name, x, y, width, height, solids, offset_x, offset_y, autotile_coords: Vec::new() };
    // Compute autotile coordinates on load
    let xml_path = get_celeste_fgtiles_xml_path_from_editor(editor);
    ld.compute_autotile_coords(&xml_path);
    Some(ld)
}

/// Normalize decal path to "decals/..."
fn normalize_decal_path(texture: &str) -> String {
    let mut key = texture.replace("\\", "/");
    if key.ends_with(".png") { key.truncate(key.len()-4); }
    if !key.starts_with("decals/") { key = format!("decals/{}", key); }
    key
}

/// Render a single tile (filled + borders) using the passed LevelRenderData
fn render_tile(
    painter: &egui::Painter,
    ld: &LevelRenderData,
    editor: &CelesteMapEditor,
    x: usize,
    y: usize,
    tile: char,
    tile_size: f32,
    visible: bool,
) {
    // Ensure tileset mapping is loaded from Celeste assets (once per session)
    ensure_tileset_id_path_map_loaded_from_celeste(editor);
    // TEMP DEBUG: print mapping status for first tile
    if x == 0 && y == 0 {
        let map = crate::tile_xml::TILESET_ID_PATH_MAP.get();
        eprintln!("[TILE DEBUG] tile char: {}", tile);
        if let Some(map) = map {
            if let Some(path) = get_tileset_path_for_id(map, tile) {
                eprintln!("[TILE DEBUG] tileset path for '{}': {}", tile, path);
                let sprite_path = format!("tilesets/{}", path);
                eprintln!("[TILE DEBUG] sprite_path: {}", sprite_path);
                if let Some(atlas_mgr) = &editor.atlas_manager {
                    let found = atlas_mgr.get_sprite("Gameplay", &sprite_path).is_some();
                    eprintln!("[TILE DEBUG] atlas get_sprite('{}'): {}", sprite_path, found);
                } else {
                    eprintln!("[TILE DEBUG] atlas_manager is None");
                }
            } else {
                eprintln!("[TILE DEBUG] No tileset path for '{}'", tile);
            }
        } else {
            eprintln!("[TILE DEBUG] TILESET_ID_PATH_MAP is None");
        }
    }
    if !visible || tile == '0' || tile == ' ' {
        return;
    }
    let solids = &ld.solids;
    if y >= solids.len() || x >= solids[y].len() {
        return;
    }
    let scale = TILE_SIZE / 8.0;
    let world_x0 = (ld.x + ld.offset_x as f32) * scale * editor.zoom_level;
    let world_y0 = (ld.y + ld.offset_y as f32) * scale * editor.zoom_level;
    let px = world_x0 + x as f32 * tile_size - editor.camera_pos.x;
    let py = world_y0 + y as f32 * tile_size - editor.camera_pos.y;
    let pos = Pos2::new(px, py);
    let rect = Rect::from_min_size(pos, Vec2::splat(tile_size));

    // Infill check
    let mut internal = true;
    let max_y = solids.len();
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 { continue; }
            let ny = y as isize + dy;
            let nx = x as isize + dx;
            if ny < 0 || nx < 0 || ny as usize >= max_y {
                continue;
            }
            let row = &solids[ny as usize];
            if nx as usize >= row.len() || !is_solid_tile(row[nx as usize]) {
                internal = false;
                break;
            }
        }
        if !internal { break; }
    }
    let map = crate::tile_xml::TILESET_ID_PATH_MAP.get();
    let mut drew_texture = false;
    if !ld.autotile_coords.is_empty() {
        if let Some(coord) = ld.autotile_coords.get(y).and_then(|row| row.get(x)).and_then(|v| *v) {
            if let Some(map) = map {
                if let Some(path) = get_tileset_path_for_id(map, tile) {
                    let region = egui::Rect::from_min_size(
                        egui::Pos2::new((coord.0 * 8) as f32, (coord.1 * 8) as f32),
                        egui::Vec2::new(8.0, 8.0),
                    );
                    if let Some(atlas_mgr) = &editor.atlas_manager {
                        let sprite_path = format!("tilesets/{}", path);
                        match atlas_mgr.get_sprite("Gameplay", &sprite_path) {
                            Some(sprite) => {
                                atlas_mgr.draw_sprite_region(sprite, painter, rect, Color32::WHITE, region);
                                drew_texture = true;
                            }
                            None => {}
                        }
                    }
                }
            }
        }
    } else {
        // fallback: recompute on the fly (shouldn't happen)
        if let Some(map) = map {
            if let Some(path) = get_tileset_path_for_id(map, tile) {
                let xml_path = if let Some(ref celeste_dir) = editor.celeste_assets.celeste_dir {
                    #[cfg(target_os = "macos")]
                    {
                        let mut p = celeste_dir.clone();
                        if !p.ends_with("Celeste.app") {
                            p = p.join("Celeste.app");
                        }
                        p.join("Contents/Resources/Content/Graphics/ForegroundTiles.xml").to_string_lossy().to_string()
                    }
                    #[cfg(not(target_os = "macos") )]
                    {
                        celeste_dir.join("Content/Graphics/ForegroundTiles.xml").to_string_lossy().to_string()
                    }
                } else {
                    String::new()
                };
                let tilesets = crate::tile_xml::get_tilesets_with_rules(&xml_path);
                let is_solid = |c: char| is_solid_tile(c);
                if let Some((tile_x, tile_y)) = crate::tile_xml::autotile_tile_coord(tile, solids, x, y, tilesets, &is_solid) {
                    let region = egui::Rect::from_min_size(
                        egui::Pos2::new((tile_x * 8) as f32, (tile_y * 8) as f32),
                        egui::Vec2::new(8.0, 8.0),
                    );
                    if let Some(atlas_mgr) = &editor.atlas_manager {
                        let sprite_path = format!("tilesets/{}", path);
                        match atlas_mgr.get_sprite("Gameplay", &sprite_path) {
                            Some(sprite) => {
                                atlas_mgr.draw_sprite_region(sprite, painter, rect, Color32::WHITE, region);
                                drew_texture = true;
                            }
                            None => {}
                        }
                    }
                }
            }
        }
    }
    if !drew_texture {
        eprintln!("[TILE DEBUG] drew fallback color for '{}'", tile);
        // Fallback: draw colored rect
        let color = get_tile_color(tile).unwrap_or(SOLID_TILE_COLOR);
        painter.rect_filled(rect, 0.0, color);

        // External borders
        // Up
        if !(y > 0 && x < solids[y-1].len() && is_solid_tile(solids[y-1][x])) {
            painter.rect_filled(Rect::from_min_size(Pos2::new(pos.x, pos.y - 1.0), Vec2::new(tile_size, 1.0)), 0.0, EXTERNAL_BORDER_COLOR);
        }
        // Down
        if !(y + 1 < max_y && x < solids[y+1].len() && is_solid_tile(solids[y+1][x])) {
            painter.rect_filled(Rect::from_min_size(Pos2::new(pos.x, pos.y + tile_size), Vec2::new(tile_size, 1.0)), 0.0, EXTERNAL_BORDER_COLOR);
        }
        // Left
        if !(x > 0 && x - 1 < solids[y].len() && is_solid_tile(solids[y][x-1])) {
            painter.rect_filled(Rect::from_min_size(Pos2::new(pos.x - 1.0, pos.y), Vec2::new(1.0, tile_size)), 0.0, EXTERNAL_BORDER_COLOR);
        }
        // Right
        if !(x + 1 < solids[y].len() && is_solid_tile(solids[y][x+1])) {
            painter.rect_filled(Rect::from_min_size(Pos2::new(pos.x + tile_size, pos.y), Vec2::new(1.0, tile_size)), 0.0, EXTERNAL_BORDER_COLOR);
        }
    }
}

/// Batch render tiles
fn batch_render_tiles(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    ld: &LevelRenderData,
    tile_size: f32,
    view: Rect,
    _ctx: &egui::Context,
) {
    // expand the visible area by a zoom‑aware margin
    let margin = CULLING_THRESHOLD_BASE * (2.0 / editor.zoom_level.max(0.1));
    let rect   = view.expand(margin);

    // convert room origin from Celeste pixels (8px units) into tile-space
    let origin_tiles_x = (ld.x + ld.offset_x as f32) / 8.0;
    let origin_tiles_y = (ld.y + ld.offset_y as f32) / 8.0;

    // compute the range of tile indices intersecting our expanded view
    let start_x = ((rect.min.x + editor.camera_pos.x) / tile_size - origin_tiles_x)
        .floor()
        .max(0.0) as usize;
    let start_y = ((rect.min.y + editor.camera_pos.y) / tile_size - origin_tiles_y)
        .floor()
        .max(0.0) as usize;
    let end_x   = ((rect.max.x + editor.camera_pos.x) / tile_size - origin_tiles_x)
        .ceil()
        .max(0.0) as usize;
    let end_y   = ((rect.max.y + editor.camera_pos.y) / tile_size - origin_tiles_y)
        .ceil()
        .max(0.0) as usize;

    // only iterate over those rows/cols
    for yy in start_y..=end_y {
        if yy >= ld.solids.len() { continue; }
        for xx in start_x..=end_x {
            if xx >= ld.solids[yy].len() { continue; }
            let tile = ld.solids[yy][xx];
            render_tile(painter, ld, editor, xx, yy, tile, tile_size, true);
        }
    }
}

fn render_bgdecals(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    level: &serde_json::Value,
    scale: f32,
    _ctx: &egui::Context,
    room_x: f32, // = ld.x
    room_y: f32, // = ld.y
) {
    if let Some(children) = level["__children"].as_array() {
        for c in children.iter().filter(|c| c["__name"] == "bgdecals") {
            if let Some(decs) = c["__children"].as_array() {
                for d in decs.iter().filter(|d| d["__name"] == "decal") {
                    let path = normalize_decal_path(d["texture"].as_str().unwrap_or(""));
                    let x    = d["x"].as_f64().unwrap_or(0.0)    as f32;
                    let y    = d["y"].as_f64().unwrap_or(0.0)    as f32;
                    let sx   = d["scaleX"].as_f64().unwrap_or(1.0) as f32;
                    let sy   = d["scaleY"].as_f64().unwrap_or(1.0) as f32;

                    if let Some(spr) = editor
                        .atlas_manager
                        .as_ref()
                        .and_then(|am| am.get_sprite("Gameplay", &path))
                    {
                        let center_x = (room_x + x) * scale * editor.zoom_level - editor.camera_pos.x;
                        let center_y = (room_y + y) * scale * editor.zoom_level - editor.camera_pos.y;

                        let width_px  = spr.metadata.width  as f32 * sx * scale * editor.zoom_level * DECAL_SCALE;
                        let height_px = spr.metadata.height as f32 * sy * scale * editor.zoom_level * DECAL_SCALE;

                        let pos  = Pos2::new(center_x - width_px  * 0.5, center_y - height_px * 0.5);
                        let size = Vec2::new(width_px, height_px);

                        crate::celeste_atlas::AtlasManager::draw_sprite(
                            &editor.atlas_manager.as_ref().unwrap(),
                            &spr,
                            painter,
                            Rect::from_min_size(pos, size),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
    }
}

fn render_fgdecals(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    level: &serde_json::Value,
    scale: f32,
    _ctx: &egui::Context,
    room_x: f32, // = ld.x
    room_y: f32, // = ld.y
) {
    if let Some(children) = level["__children"].as_array() {
        for c in children.iter().filter(|c| c["__name"] == "fgdecals") {
            if let Some(decs) = c["__children"].as_array() {
                for d in decs.iter().filter(|d| d["__name"] == "decal") {
                    let path = normalize_decal_path(d["texture"].as_str().unwrap_or(""));
                    let x    = d["x"].as_f64().unwrap_or(0.0)    as f32;
                    let y    = d["y"].as_f64().unwrap_or(0.0)    as f32;
                    let sx   = d["scaleX"].as_f64().unwrap_or(1.0) as f32;
                    let sy   = d["scaleY"].as_f64().unwrap_or(1.0) as f32;

                    if let Some(spr) = editor
                        .atlas_manager
                        .as_ref()
                        .and_then(|am| am.get_sprite("Gameplay", &path))
                    {
                        let center_x = (room_x + x) * scale * editor.zoom_level - editor.camera_pos.x;
                        let center_y = (room_y + y) * scale * editor.zoom_level - editor.camera_pos.y;

                        let width_px  = spr.metadata.width  as f32 * sx * scale * editor.zoom_level * DECAL_SCALE;
                        let height_px = spr.metadata.height as f32 * sy * scale * editor.zoom_level * DECAL_SCALE;

                        let pos  = Pos2::new(center_x - width_px  * 0.5, center_y - height_px * 0.5);
                        let size = Vec2::new(width_px, height_px);

                        crate::celeste_atlas::AtlasManager::draw_sprite(
                            &editor.atlas_manager.as_ref().unwrap(),
                            &spr,
                            painter,
                            Rect::from_min_size(pos, size),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
    }
}


/// Draw grid lines
fn draw_grid(painter: &egui::Painter, view: Rect, cam: Vec2, tile_size: f32, zoom: f32) {
    if zoom<0.2 { return; }
    let start_x=cam.x%tile_size; let start_y=cam.y%tile_size;
    let step=if zoom<0.5 {2} else {1};
    let th=if zoom<0.5 {0.5} else {1.0};
    for i in (0..((view.width()/tile_size) as i32+2)).step_by(step) {
        let x=i as f32*tile_size-start_x;
        painter.line_segment([Pos2::new(x,0.0),Pos2::new(x,view.height())],Stroke::new(th,GRID_COLOR));
    }
    for i in (0..((view.height()/tile_size) as i32+2)).step_by(step) {
        let y=i as f32*tile_size-start_y;
        painter.line_segment([Pos2::new(0.0,y),Pos2::new(view.width(),y)],Stroke::new(th,GRID_COLOR));
    }
}

fn render_room_content(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    ld: &LevelRenderData,
    json: &serde_json::Value,
    tile_size: f32,
    view: Rect,
    ctx: &egui::Context,
) {
    // 1) échelle pour passer des pixels Celeste (8px) aux tiles de l'éditeur
    let scale = TILE_SIZE / 8.0;

    // 2) on applique offset_x/offset_y au coin haut‑gauche de la room
    let base_x = ld.x;
    let base_y = ld.y;
    
    // 3) on dessine les bg decals avec ce nouvel origin
    render_bgdecals(editor, painter, json, scale, ctx, base_x, base_y);

    // 4) les tuiles solides
    if editor.show_tiles {
        batch_render_tiles(editor, painter, ld, tile_size, view, ctx);
    }

    // 5) puis les fg decals, même origine
    if editor.show_fgdecals {
        render_fgdecals(editor, painter, json, scale, ctx, base_x, base_y);
    }
}



/// Draw outline and label
fn render_room_outline_and_label(
    editor: &CelesteMapEditor,
    painter: &egui::Painter,
    ld: &LevelRenderData,
    tile_size: f32,
    ctx: &egui::Context,
    selected: bool,
) {
    let scale=TILE_SIZE/8.0;
    let px=(ld.x)*scale*editor.zoom_level-editor.camera_pos.x;
    let py=(ld.y)*scale*editor.zoom_level-editor.camera_pos.y;
    let w=ld.width*scale*editor.zoom_level;
    let h=ld.height*scale*editor.zoom_level;
    let rect=Rect::from_min_size(Pos2::new(px,py),Vec2::new(w,h));
    let col=if selected {ROOM_CONTOUR_SELECTED} else {ROOM_CONTOUR_UNSELECTED};
    let th=if selected {3.0} else {2.0};
    painter.rect_stroke(rect,0.0,Stroke::new(th,col));
    if editor.show_labels {
        painter.text(Pos2::new(px+5.0,py+5.0),egui::Align2::LEFT_TOP,&ld.name,egui::FontId::proportional(16.0),Color32::WHITE);
    }
}

/// Collect levels with data and JSON
fn collect_levels_with_json(
    node: &serde_json::Value,
    out: &mut Vec<(usize, LevelRenderData, serde_json::Value)>,
    mut idx: usize,
    editor: &CelesteMapEditor,
) -> usize {
    if node["__name"].as_str()==Some("level") {
        if let Some(ld)=extract_level_data(node, editor) {
            out.push((idx,ld,node.clone())); idx+=1;
        }
    }
    if let Some(children)=node["__children"].as_array() {
        for c in children { idx=collect_levels_with_json(c,out,idx,editor); }
    }
    idx
}

/// Render all rooms
fn render_all_rooms(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    tile_size: f32,
    response: &egui::Response,
    ctx: &egui::Context,
) {
    let view = response.rect;
    let cached_rooms_len = editor.cached_rooms.len();
    for i in 0..cached_rooms_len {
        // Copy the data out to avoid borrow conflicts
        let (ld, json) = {
            let room = &editor.cached_rooms[i];
            (room.level_data.clone(), room.json.clone())
        };
        // Compute room rectangle in world coordinates
        let scale = TILE_SIZE / 8.0;
        let room_x = (ld.x) * scale * editor.zoom_level - editor.camera_pos.x;
        let room_y = (ld.y) * scale * editor.zoom_level - editor.camera_pos.y;
        let room_w = ld.width * scale * editor.zoom_level;
        let room_h = ld.height * scale * editor.zoom_level;
        let room_rect = egui::Rect::from_min_size(
            egui::Pos2::new(room_x, room_y),
            egui::Vec2::new(room_w, room_h),
        );
        // Expand view for culling margin
        let margin = CULLING_THRESHOLD_BASE * (2.0 / editor.zoom_level.max(0.1));
        let expanded_view = view.expand(margin);
        // Cull rooms not in view
        if room_rect.intersects(expanded_view) {
            let sel = i == editor.current_level_index;
            render_room_content(editor, painter, &ld, &json, tile_size, view, ctx);
            render_room_outline_and_label(editor, painter, &ld, tile_size, ctx, sel);
        }
    }
}

/// Render only current room
fn render_current_room(
    editor: &mut CelesteMapEditor,
    painter: &egui::Painter,
    tile_size: f32,
    view: Rect,
    ctx: &egui::Context,
) {
    let idx = editor.current_level_index;
    if idx < editor.cached_rooms.len() {
        let (ld, json) = {
            let room = &editor.cached_rooms[idx];
            (room.level_data.clone(), room.json.clone())
        };
        render_room_content(editor, painter, &ld, &json, tile_size, view, ctx);
        render_room_outline_and_label(editor, painter, &ld, tile_size, ctx, true);
    }
}

/// Main app rendering
pub fn render_app(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    render_top_panel(editor,ctx);
    render_bottom_panel(editor,ctx);
    render_central_panel(editor,ctx);
}

fn render_top_panel(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx,|ui|{
        ui.horizontal(|ui|{
            ui.menu_button("File",|ui|{
                if ui.button("Open...").clicked(){ editor.show_open_dialog=true;ui.close_menu(); }
                if ui.button("Save").clicked(){ save_map(editor);ui.close_menu(); }
                if ui.button("Save As...").clicked(){ save_map_as(editor);ui.close_menu(); }
                ui.separator();
                if ui.button("Set Celeste Path...").clicked(){ editor.show_celeste_path_dialog=true;ui.close_menu(); }
                ui.separator();
                if ui.button("Quit").clicked(){ std::process::exit(0); }
            });
            ui.menu_button("View",|ui|{
                let prev=editor.show_fgdecals;
                if ui.checkbox(&mut editor.show_fgdecals,"Show Fg Decals").changed(){ editor.static_dirty=true; }
                if ui.checkbox(&mut editor.show_tiles,"Show Tiles").changed(){ editor.static_dirty=true; }
                ui.checkbox(&mut editor.show_all_rooms,"Show All Rooms");
                ui.checkbox(&mut editor.show_grid,"Show Grid");
                ui.checkbox(&mut editor.show_labels,"Show Labels");
                ui.separator();
                if ui.button("Zoom In").clicked(){ editor.zoom_level*=1.2;editor.static_dirty=true;ui.close_menu(); }
                if ui.button("Zoom Out").clicked(){ editor.zoom_level=(editor.zoom_level/1.2).max(0.1);editor.static_dirty=true;ui.close_menu(); }
                if ui.button("Reset Zoom").clicked(){ editor.zoom_level=1.0;editor.static_dirty=true;ui.close_menu(); }
                ui.separator();
                if ui.button("Key Bindings...").clicked(){ editor.show_key_bindings_dialog=true;ui.close_menu(); }
            });
            ui.separator();
            if !editor.show_all_rooms {
                ui.label("Room:");
                egui::ComboBox::from_id_source("level_selector")
                    .selected_text(editor.level_names.get(editor.current_level_index).unwrap_or(&"None".to_string()))
                    .show_ui(ui,|ui|{
                        for (i,name) in editor.level_names.iter().enumerate(){ if ui.selectable_label(editor.current_level_index==i,name).clicked(){ editor.current_level_index=i; }}
                    });
            }
        });
    });
}

fn render_bottom_panel(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx,|ui|{
        ui.horizontal(|ui|{
            if let Some(p)=editor.drag_start { ui.label(format!("Drag: ({:.1},{:.1})",p.x,p.y)); }
            ui.label(format!("Mouse: ({:.1},{:.1})",editor.mouse_pos.x,editor.mouse_pos.y));
            let (tx,ty)=editor.screen_to_map(editor.mouse_pos);
            ui.label(format!("Tile: ({},{})",tx,ty));
            if let Some(path)=&editor.bin_path { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ ui.label(format!("File: {}",path)); }); }
        });
    });
}

fn render_central_panel(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx,|ui|{
        if let Some(err)=&editor.error_message { ui.heading("Error");ui.label(err);return; }
        let (resp,painter)=ui.allocate_painter(ui.available_size(),egui::Sense::hover());
        editor.mouse_pos=resp.hover_pos().unwrap_or_default();
        painter.rect_filled(
                resp.rect,
                0.0,
                BG_COLOR,
            );
            // Draw grid even if no map is loaded
            if editor.show_grid {
                let size = TILE_SIZE * editor.zoom_level;
                draw_grid(&painter, resp.rect, editor.camera_pos, size, editor.zoom_level);
            }
            let size=TILE_SIZE*editor.zoom_level;
        if editor.show_all_rooms { render_all_rooms(editor,&painter,size,&resp,ctx); }
        else { render_current_room(editor,&painter,size,resp.rect,ctx); }
    });
}

// Helper: get the ForegroundTiles.xml path for the current platform/editor
fn get_celeste_fgtiles_xml_path_from_editor(editor: &CelesteMapEditor) -> String {
    if let Some(ref celeste_dir) = editor.celeste_assets.celeste_dir {
        #[cfg(target_os = "macos")]
        {
            let mut p = celeste_dir.clone();
            if !p.ends_with("Celeste.app") {
                p = p.join("Celeste.app");
            }
            p.join("Contents/Resources/Content/Graphics/ForegroundTiles.xml").to_string_lossy().to_string()
        }
        #[cfg(not(target_os = "macos") )]
        {
            celeste_dir.join("Content/Graphics/ForegroundTiles.xml").to_string_lossy().to_string()
        }
    } else {
        String::new()
    }
}
