use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

use crate::app::CelesteMapEditor;

/// Get a temporary JSON path for a given binary map file
pub fn get_temp_json_path(bin_path: &str) -> String {
    let path = Path::new(bin_path);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let temp_dir = std::env::temp_dir();
    temp_dir.join(format!("{}_temp.json", stem)).to_string_lossy().to_string()
}

/// Load a map from a binary file into the editor
pub fn load_map(editor: &mut CelesteMapEditor, bin_path: &str) {
    // Create a temporary JSON file path
    let temp_json_path = get_temp_json_path(bin_path);
    
    // Convert BIN to JSON using Cairn
    match Command::new("cairn")
        .args(&["bin2json", bin_path, &temp_json_path])
        .status() 
    {
        Ok(status) => {
            if !status.success() {
                editor.error_message = Some(format!("Failed to convert BIN to JSON: Cairn exited with status {}", status));
                return;
            }
        },
        Err(e) => {
            editor.error_message = Some(format!("Failed to execute Cairn command: {}", e));
            return;
        }
    }

    // Now load the JSON file
    match File::open(&temp_json_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(data) => {
                    editor.map_data = Some(data);
                    editor.bin_path = Some(bin_path.to_string());
                    editor.temp_json_path = Some(temp_json_path);
                    editor.extract_level_names();
                    editor.error_message = None;
                }
                Err(e) => {
                    editor.error_message = Some(format!("Failed to parse JSON: {}", e));
                }
            }
        }
        Err(e) => {
            editor.error_message = Some(format!("Failed to open temporary JSON file: {}", e));
        }
    }
}

/// Save the map to its current binary file
pub fn save_map(editor: &CelesteMapEditor) {
    if let (Some(map_data), Some(bin_path), Some(temp_json_path)) = (&editor.map_data, &editor.bin_path, &editor.temp_json_path) {
        // First save the JSON to the temporary file
        match serde_json::to_string_pretty(map_data) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&temp_json_path, json_str) {
                    eprintln!("Failed to write temporary JSON file: {}", e);
                    return;
                }
                
                // Now convert the JSON to BIN using Cairn
                match Command::new("cairn")
                    .args(&["json2bin", &temp_json_path, &bin_path])
                    .status() 
                {
                    Ok(status) => {
                        if !status.success() {
                            eprintln!("Failed to convert JSON to BIN: Cairn exited with status {}", status);
                        } else {
                            println!("Map saved successfully to {}", bin_path);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to execute Cairn command: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize map data: {}", e);
            }
        }
    }
}

/// Save the map to a new binary file
pub fn save_map_as(editor: &mut CelesteMapEditor) {
    if let Some(map_data) = &editor.map_data {
        if let Some(new_bin_path) = rfd::FileDialog::new()
            .add_filter("Celeste Map", &["bin"])
            .save_file()
        {
            let new_bin_path_str = new_bin_path.display().to_string();
            let new_temp_json_path = get_temp_json_path(&new_bin_path_str);
            
            // Save the JSON to the temporary file
            match serde_json::to_string_pretty(map_data) {
                Ok(json_str) => {
                    if let Err(e) = std::fs::write(&new_temp_json_path, json_str) {
                        eprintln!("Failed to write temporary JSON file: {}", e);
                        return;
                    }
                    
                    // Now convert the JSON to BIN using Cairn
                    match Command::new("cairn")
                        .args(&["json2bin", &new_temp_json_path, &new_bin_path_str])
                        .status() 
                    {
                        Ok(status) => {
                            if !status.success() {
                                eprintln!("Failed to convert JSON to BIN: Cairn exited with status {}", status);
                            } else {
                                println!("Map saved successfully to {}", new_bin_path_str);
                                // Update paths
                                editor.bin_path = Some(new_bin_path_str);
                                editor.temp_json_path = Some(new_temp_json_path);
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to execute Cairn command: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to serialize map data: {}", e);
                }
            }
        }
    }
}