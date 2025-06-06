use eframe::egui::Pos2;
use crate::app::CelesteMapEditor;
use log::{warn};

pub fn place_block(editor: &mut CelesteMapEditor, pos: Pos2) {
    // If in "all rooms" mode, determine which room was clicked
    if editor.show_all_rooms {
        let scaled_tile_size = crate::ui::render::TILE_SIZE * editor.zoom_level;
        
        if let Some(map) = &editor.map_data {
            if let Some(levels) = map["__children"][0]["__children"].as_array() {
                for (i, level) in levels.iter().enumerate() {
                    if level["__name"] == "level" {
                        if let (Some(room_x), Some(room_y)) = (level["x"].as_f64(), level["y"].as_f64()) {
                            let room_width = level.get("width").and_then(|w| w.as_f64()).unwrap_or(320.0);
                            let room_height = level.get("height").and_then(|h| h.as_f64()).unwrap_or(184.0);
                            
                            // Calculate room bounds in screen space
                            let room_screen_x = room_x as f32 * scaled_tile_size - editor.camera_pos.x;
                            let room_screen_y = room_y as f32 * scaled_tile_size - editor.camera_pos.y;
                            let room_screen_width = room_width as f32 * scaled_tile_size;
                            let room_screen_height = room_height as f32 * scaled_tile_size;
                            
                            // Check if click is within this room
                            if pos.x >= room_screen_x && pos.x < room_screen_x + room_screen_width && 
                               pos.y >= room_screen_y && pos.y < room_screen_y + room_screen_height {
                                // Convert room coordinates from pixels to tiles
                                let room_x_tiles = room_x / 8.0;
                                let room_y_tiles = room_y / 8.0;
                                
                                // Adjust position to be relative to room
                                let adjusted_x = pos.x + editor.camera_pos.x - (room_x_tiles as f32 * scaled_tile_size);
                                let adjusted_y = pos.y + editor.camera_pos.y - (room_y_tiles as f32 * scaled_tile_size);
                                
                                // Switch to this room and place the block
                                editor.current_level_index = i;
                                let adjusted_pos = Pos2::new(adjusted_x, adjusted_y);
                                place_block_in_current_room(editor, adjusted_pos);
                                return;
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Normal mode - place block in current room
        place_block_in_current_room(editor, pos);
    }
}

pub fn place_block_in_current_room(editor: &mut CelesteMapEditor, pos: Pos2) {
    let (tile_x, tile_y) = editor.screen_to_map(pos);

    if let Some(level) = editor.get_current_level() {
        if let (Some(room_x), Some(room_y), Some(room_width), Some(room_height)) = (
            level["x"].as_f64(),
            level["y"].as_f64(),
            level["width"].as_f64(),
            level["height"].as_f64(),
        ) {
            // Convert room coordinates from pixels to tile units (1 tile = 8 pixels)
            let room_x_tiles = (room_x / 8.0) as i32;
            let room_y_tiles = (room_y / 8.0) as i32;
            let room_width_tiles = (room_width / 8.0) as i32;
            let room_height_tiles = (room_height / 8.0) as i32;

            // Ensure the tile is inside the room's boundaries
            if tile_x < room_x_tiles || tile_y < room_y_tiles
                || tile_x >= room_x_tiles + room_width_tiles
                || tile_y >= room_y_tiles + room_height_tiles
            {
                warn!("Attempted to place block outside of room boundaries!");
                return;
            }
        }
    }

    // Get solids data
    if let Some(solids) = editor.get_solids_data() {
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

        // Adjust tile position for offsets
        let adjusted_x = tile_x - offset_x;
        let adjusted_y = tile_y - offset_y;

        // Skip if position would be negative after adjustment
        if adjusted_x < 0 || adjusted_y < 0 {
            warn!("Cannot place block at negative position after offset adjustment");
            return;
        }

        let mut rows: Vec<String> = solids.split('\n').map(|s| s.to_string()).collect();

        // Ensure we have enough rows
        while rows.len() <= adjusted_y as usize {
            rows.push(String::new());
        }

        // Ensure the row is long enough
        let row = &mut rows[adjusted_y as usize];
        while row.len() <= adjusted_x as usize {
            row.push('0');
        }

        // Place a solid tile (use '9' or appropriate character based on map type)
        if let Some(_c) = row.chars().nth(adjusted_x as usize) {
            let mut new_row = row.clone();
            new_row.replace_range(adjusted_x as usize..adjusted_x as usize + 1, "9");
            rows[adjusted_y as usize] = new_row;

            // Update the map data
            let new_solids = rows.join("\n");
            editor.update_solids_data(&new_solids);
        }
    }
}


pub fn remove_block(editor: &mut CelesteMapEditor, pos: Pos2) {
    // If in "all rooms" mode, determine which room was clicked
    if editor.show_all_rooms {
        let scaled_tile_size = crate::ui::render::TILE_SIZE * editor.zoom_level;
        
        if let Some(map) = &editor.map_data {
            if let Some(levels) = map["__children"][0]["__children"].as_array() {
                for (i, level) in levels.iter().enumerate() {
                    if level["__name"] == "level" {
                        if let (Some(room_x), Some(room_y)) = (level["x"].as_f64(), level["y"].as_f64()) {
                            let room_width = level.get("width").and_then(|w| w.as_f64()).unwrap_or(320.0);
                            let room_height = level.get("height").and_then(|h| h.as_f64()).unwrap_or(184.0);
                            
                            // Calculate room bounds in screen space
                            let room_screen_x = room_x as f32 * scaled_tile_size - editor.camera_pos.x;
                            let room_screen_y = room_y as f32 * scaled_tile_size - editor.camera_pos.y;
                            let room_screen_width = room_width as f32 * scaled_tile_size;
                            let room_screen_height = room_height as f32 * scaled_tile_size;
                            
                            // Check if click is within this room
                            if pos.x >= room_screen_x && pos.x < room_screen_x + room_screen_width && 
                               pos.y >= room_screen_y && pos.y < room_screen_y + room_screen_height {
                                // Adjust position to be relative to room
                                let adjusted_x = pos.x + editor.camera_pos.x - (room_x as f32 * scaled_tile_size);
                                let adjusted_y = pos.y + editor.camera_pos.y - (room_y as f32 * scaled_tile_size);
                                
                                // Switch to this room and remove the block
                                editor.current_level_index = i;
                                let adjusted_pos = Pos2::new(adjusted_x, adjusted_y);
                                remove_block_in_current_room(editor, adjusted_pos);
                                return;
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Normal mode - remove block in current room
        remove_block_in_current_room(editor, pos);
    }
}

pub fn remove_block_in_current_room(editor: &mut CelesteMapEditor, pos: Pos2) {
    let (tile_x, tile_y) = editor.screen_to_map(pos);

    // Get solids data
    if let Some(solids) = editor.get_solids_data() {
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

        // Adjust tile position for offsets
        let adjusted_x = tile_x - offset_x;
        let adjusted_y = tile_y - offset_y;

        // Skip if position would be negative after adjustment
        if adjusted_x < 0 || adjusted_y < 0 {
            return;
        }

        let mut rows: Vec<String> = solids.split('\n').map(|s| s.to_string()).collect();

        // Check if tile coordinates are valid
        if adjusted_y >= 0 && adjusted_y < rows.len() as i32 {
            let row = &mut rows[adjusted_y as usize];
            if adjusted_x >= 0 && adjusted_x < row.len() as i32 {
                // Replace with an empty tile ('0')
                let mut new_row = row.clone();
                new_row.replace_range(adjusted_x as usize..adjusted_x as usize + 1, "0");
                rows[adjusted_y as usize] = new_row;

                // Update the map data
                let new_solids = rows.join("\n");
                editor.update_solids_data(&new_solids);
            }
        }
    }
}