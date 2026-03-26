use eframe::egui::Pos2;
use crate::app::CelesteMapEditor;

const CELESTE_TILE_PX: f32 = 8.0;

pub fn place_block(editor: &mut CelesteMapEditor, pos: Pos2) {
    if editor.show_all_rooms {
        match find_room_at(editor, pos) {
            Some(i) => editor.current_level_index = i,
            None => return,
        }
    }
    modify_tile(editor, pos, '9');
}

pub fn remove_block(editor: &mut CelesteMapEditor, pos: Pos2) {
    if editor.show_all_rooms {
        match find_room_at(editor, pos) {
            Some(i) => editor.current_level_index = i,
            None => return,
        }
    }
    modify_tile(editor, pos, '0');
}

fn find_room_at(editor: &CelesteMapEditor, pos: Pos2) -> Option<usize> {
    let scale = crate::ui::render::TILE_SIZE / CELESTE_TILE_PX * editor.zoom_level;
    let map = editor.map_data.as_ref()?;
    let levels = find_levels(map)?;

    for (i, level) in levels.iter().enumerate() {
        if level["__name"] != "level" { continue; }

        let rx = level["x"].as_f64()? as f32;
        let ry = level["y"].as_f64()? as f32;
        let rw = level["width"].as_f64().unwrap_or(320.0) as f32;
        let rh = level["height"].as_f64().unwrap_or(184.0) as f32;

        let screen_x = rx * scale - editor.camera_pos.x;
        let screen_y = ry * scale - editor.camera_pos.y;

        if pos.x >= screen_x && pos.x < screen_x + rw * scale
            && pos.y >= screen_y && pos.y < screen_y + rh * scale
        {
            return Some(i);
        }
    }
    None
}

fn find_levels(map: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    map["__children"].as_array()?
        .iter()
        .find(|c| c["__name"] == "levels")?
        ["__children"].as_array()
}

fn get_solids_offset(level: &serde_json::Value) -> (i32, i32) {
    level["__children"].as_array()
        .and_then(|children| children.iter().find(|c| c["__name"] == "solids"))
        .map(|s| (
            s["offsetX"].as_i64().unwrap_or(0) as i32,
            s["offsetY"].as_i64().unwrap_or(0) as i32,
        ))
        .unwrap_or((0, 0))
}

fn modify_tile(editor: &mut CelesteMapEditor, pos: Pos2, tile_char: char) {
    let (abs_x, abs_y) = editor.screen_to_map(pos);

    let Some(level) = editor.get_current_level() else { return };
    let room_x = level["x"].as_f64().unwrap_or(0.0) as f32;
    let room_y = level["y"].as_f64().unwrap_or(0.0) as f32;
    let room_w = (level["width"].as_f64().unwrap_or(0.0) / CELESTE_TILE_PX as f64) as i32;
    let room_h = (level["height"].as_f64().unwrap_or(0.0) / CELESTE_TILE_PX as f64) as i32;
    let (offset_x, offset_y) = get_solids_offset(level);

    let origin_x = ((room_x + offset_x as f32) / CELESTE_TILE_PX).floor() as i32;
    let origin_y = ((room_y + offset_y as f32) / CELESTE_TILE_PX).floor() as i32;
    let local_x = abs_x - origin_x;
    let local_y = abs_y - origin_y;

    if local_x < 0 || local_y < 0 || local_x >= room_w || local_y >= room_h { return; }

    let Some(solids) = editor.get_solids_data() else { return };
    let mut rows: Vec<String> = solids.split('\n').map(|s| s.to_string()).collect();

    if tile_char == '0' {
        if local_y as usize >= rows.len() { return; }
        let row = &rows[local_y as usize];
        if local_x as usize >= row.len() { return; }
        let mut new_row = row.clone();
        new_row.replace_range(local_x as usize..local_x as usize + 1, "0");
        rows[local_y as usize] = new_row;
    } else {
        while rows.len() <= local_y as usize {
            rows.push(String::new());
        }
        let row = &mut rows[local_y as usize];
        while row.len() <= local_x as usize {
            row.push('0');
        }
        let mut new_row = row.clone();
        new_row.replace_range(local_x as usize..local_x as usize + 1, &tile_char.to_string());
        rows[local_y as usize] = new_row;
    }

    editor.update_solids_data(&rows.join("\n"));
}
