use cairn::{bin_to_json, json_to_bin};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::io::Write;
use eframe::egui::Vec2;
use rfd;

use crate::app::CelesteMapEditor;

/// Get a temporary JSON path for a given binary map file
pub fn get_temp_json_path(bin_path: &str) -> String {
    let path = Path::new(bin_path);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let temp_dir = std::env::temp_dir();
    temp_dir.join(format!("{}_temp.json", stem)).to_string_lossy().to_string()
}

pub fn load_map(editor: &mut CelesteMapEditor, bin_path: &str) {
    let temp_json_path = get_temp_json_path(bin_path);
    println!("Loading map: {}", bin_path);
    println!("Temp JSON path: {}", temp_json_path);

    // Convert BIN to JSON using Cairn library
    match bin_to_json(bin_path, &temp_json_path) {
        Ok(_) => {
            println!("Successfully converted bin to json");
            if let Ok(file) = File::open(&temp_json_path) {
                let reader = BufReader::new(file);
                match serde_json::from_reader(reader) {
                    Ok(data) => {
                        println!("Successfully parsed JSON data");
                        editor.map_data = Some(data);
                        editor.static_dirty = true;
                        editor.bin_path = Some(bin_path.to_string());
                        editor.temp_json_path = Some(temp_json_path);

                        // Debug the map structure
                        editor.debug_map_structure();

                        // Extract level names
                        editor.extract_level_names();

                        // Reset current level to the first one
                        editor.current_level_index = 0;

                        // Reset camera position
                        editor.camera_pos = Vec2::new(0.0, 0.0);

                        println!("Map loaded successfully with {} levels", editor.level_names.len());
                        editor.error_message = None;
                    }
                    Err(e) => {
                        println!("Failed to parse JSON: {}", e);
                        editor.error_message = Some(format!("Failed to parse JSON: {}", e));
                    }
                }
            } else {
                println!("Failed to open converted JSON file");
                editor.error_message = Some("Failed to open converted JSON file.".to_string());
            }
        }
        Err(e) => {
            println!("Cairn conversion failed: {}", e);
            editor.error_message = Some(format!("Cairn failed: {}", e));
        }
    }
}

pub fn save_map(editor: &CelesteMapEditor) {
    if let (Some(map_data), Some(bin_path), Some(temp_json_path)) = (&editor.map_data, &editor.bin_path, &editor.temp_json_path) {
        // Save the JSON to a temporary file
        match serde_json::to_string_pretty(map_data) {
            Ok(json_str) => {
                if let Err(e) = File::create(&temp_json_path).and_then(|mut file| file.write_all(json_str.as_bytes())) {
                    eprintln!("Failed to write temporary JSON file: {}", e);
                    return;
                }

                // Convert JSON to BIN using Cairn Rust library
                match json_to_bin(&temp_json_path, &bin_path) {
                    Ok(_) => println!("Map saved successfully to {}", bin_path),
                    Err(e) => eprintln!("Failed to convert JSON to BIN: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to serialize map data: {}", e),
        }
    }
}

// Restore save_map_as for Save As functionality
pub fn save_map_as(editor: &mut CelesteMapEditor) {
    if let Some(map_data) = &editor.map_data {
        if let Some(new_bin_path) = rfd::FileDialog::new()
            .add_filter("Celeste Map", &["bin"])
            .save_file()
        {
            let new_bin_path_str = new_bin_path.display().to_string();
            // For minimal version, just save JSON for now
            match serde_json::to_string_pretty(map_data) {
                Ok(json_str) => {
                    if let Err(e) = File::create(&new_bin_path_str).and_then(|mut file| file.write_all(json_str.as_bytes())) {
                        eprintln!("Failed to write file: {}", e);
                        return;
                    }
                    println!("Map saved successfully to {}", new_bin_path_str);
                    editor.bin_path = Some(new_bin_path_str);
                }
                Err(e) => eprintln!("Failed to serialize map data: {}", e),
            }
        }
    }
}
