use eframe::egui;
use serde_json::Value;

use crate::config::keybindings::KeyBindings;
use crate::ui::render::render_app;
use crate::ui::input::handle_input;
use crate::ui::dialogs::{show_open_dialog, show_key_bindings_dialog};
use crate::assets::CelesteAssets;

pub struct CelesteMapEditor {
    pub map_data: Option<Value>,
    pub current_level_index: usize,
    pub camera_pos: egui::Vec2,
    pub dragging: bool,
    pub drag_start: Option<egui::Pos2>,
    pub mouse_pos: egui::Pos2,
    pub bin_path: Option<String>,
    pub temp_json_path: Option<String>,
    pub show_open_dialog: bool,
    pub error_message: Option<String>,
    pub level_names: Vec<String>,
    pub zoom_level: f32,
    pub show_all_rooms: bool,
    pub show_grid: bool,
    pub show_labels: bool,
    pub key_bindings: KeyBindings,
    pub show_key_bindings_dialog: bool,
    pub celeste_assets: CelesteAssets,
    pub show_celeste_path_dialog: bool,
    pub use_textures: bool,
}

impl Default for CelesteMapEditor {
    fn default() -> Self {
        Self {
            map_data: None,
            current_level_index: 0,
            camera_pos: egui::Vec2::new(0.0, 0.0),
            dragging: false,
            drag_start: None,
            mouse_pos: egui::Pos2::new(0.0, 0.0),
            bin_path: None,
            temp_json_path: None,
            show_open_dialog: false,
            error_message: None,
            level_names: Vec::new(),
            zoom_level: 1.0,
            show_all_rooms: true,
            show_grid: true,
            show_labels: true,
            key_bindings: KeyBindings::default(),
            show_key_bindings_dialog: false,
            celeste_assets: CelesteAssets::new(),
            show_celeste_path_dialog: false,
            use_textures: true,
        }
    }
}

impl CelesteMapEditor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut editor = Self::default();
        editor.key_bindings.load();

        // Check if Celeste assets are available, show dialog if not
        if editor.celeste_assets.celeste_dir.is_none() {
            editor.show_celeste_path_dialog = true;
        } else {
            // Initialize atlas manager if Celeste directory is found
            if editor.celeste_assets.init_atlas(&cc.egui_ctx) {
                println!("Successfully initialized atlas manager");
            } else {
                println!("Failed to initialize atlas manager, falling back to PNG loading");
            }
        }

        editor
    }

    pub fn debug_map_structure(&self) {
        println!("\n--- MAP STRUCTURE DEBUG ---");

        if let Some(map) = &self.map_data {
            println!("Map root name: {}", map["__name"].as_str().unwrap_or("unknown"));
            println!("Map package: {}", map["package"].as_str().unwrap_or("unknown"));

            if let Some(map_children) = map["__children"].as_array() {
                println!("Map has {} top-level children", map_children.len());

                // Find the levels element
                for (i, child) in map_children.iter().enumerate() {
                    let child_name = child["__name"].as_str().unwrap_or("unnamed");
                    println!("Child {}: {}", i, child_name);

                    if child_name == "levels" {
                        if let Some(levels) = child["__children"].as_array() {
                            println!("Found {} levels", levels.len());

                            // Print details for the first few levels
                            let max_levels_to_print = 3.min(levels.len());
                            for i in 0..max_levels_to_print {
                                let level = &levels[i];
                                if level["__name"] == "level" {
                                    println!("  Level {}: name={}", i, level["name"].as_str().unwrap_or("unnamed"));
                                    println!("    x={}, y={}, width={}, height={}",
                                             level["x"].as_f64().unwrap_or(0.0),
                                             level["y"].as_f64().unwrap_or(0.0),
                                             level["width"].as_f64().unwrap_or(0.0),
                                             level["height"].as_f64().unwrap_or(0.0));

                                    if let Some(level_children) = level["__children"].as_array() {
                                        println!("    Has {} children elements", level_children.len());

                                        // Look for solids
                                        for (j, level_child) in level_children.iter().enumerate() {
                                            let element_name = level_child["__name"].as_str().unwrap_or("unnamed");
                                            println!("      Child {}: {}", j, element_name);

                                            if element_name == "solids" {
                                                if let Some(solids_text) = level_child["innerText"].as_str() {
                                                    let line_count = solids_text.lines().count();
                                                    let first_line = solids_text.lines().next().unwrap_or("");
                                                    println!("        Found solids with {} lines", line_count);
                                                    println!("        First line: {}", first_line);
                                                    println!("        Line length: {}", first_line.len());
                                                    println!("        offsetX: {}, offsetY: {}",
                                                             level_child["offsetX"].as_i64().unwrap_or(0),
                                                             level_child["offsetY"].as_i64().unwrap_or(0));
                                                } else {
                                                    println!("        solids element has no innerText!");
                                                }
                                            }
                                        }
                                    } else {
                                        println!("    Level has no children array!");
                                    }
                                }
                            }
                        } else {
                            println!("'levels' element has no children array!");
                        }
                    }
                }
            } else {
                println!("Map has no children array!");
            }
        } else {
            println!("No map data available!");
        }

        println!("--- END MAP STRUCTURE DEBUG ---\n");
    }

    pub fn get_sprite_for_tile(&self, tile_char: char) -> Option<&crate::celeste_atlas::Sprite> {
        self.celeste_assets.get_sprite_for_tile(tile_char)
    }

    // New method to draw a sprite for a tile
    pub fn draw_sprite_for_tile(&self, painter: &egui::Painter, rect: egui::Rect, tile_char: char) -> bool {
        self.celeste_assets.draw_sprite_for_tile(painter, rect, tile_char)
    }

    pub fn extract_level_names(&mut self) {
        self.level_names.clear();
        if let Some(map) = &self.map_data {
            println!("Map structure: {}", map["__name"].as_str().unwrap_or("unknown"));

            // Check if the map has a "levels" child
            let mut found_levels = false;

            if let Some(children) = map["__children"].as_array() {
                println!("Map has {} top-level children", children.len());

                for child in children {
                    if let Some(name) = child["__name"].as_str() {
                        println!("Child: {}", name);

                        if name == "levels" {
                            found_levels = true;
                            if let Some(levels) = child["__children"].as_array() {
                                println!("Found 'levels' with {} sub-elements", levels.len());

                                for level in levels {
                                    if level["__name"] == "level" {
                                        if let Some(level_name) = level["name"].as_str() {
                                            println!("Adding level: {}", level_name);
                                            self.level_names.push(level_name.to_string());
                                        } else {
                                            println!("Level has no name attribute!");
                                        }
                                    } else {
                                        println!("Non-level element in levels: {}", level["__name"].as_str().unwrap_or("unknown"));
                                    }
                                }
                            } else {
                                println!("'levels' element has no children array!");
                            }
                        }
                    }
                }

                if !found_levels {
                    println!("WARNING: No 'levels' element found in map!");
                }
            } else {
                println!("Map has no children array!");
            }
        } else {
            println!("No map data available!");
        }

        println!("Extracted {} level names", self.level_names.len());
    }

    pub fn get_current_level(&self) -> Option<&Value> {
        if let Some(map) = &self.map_data {
            // First, find the "levels" element
            if let Some(children) = map["__children"].as_array() {
                for child in children {
                    if child["__name"] == "levels" {
                        // Now, access the level at the current index
                        if let Some(levels) = child["__children"].as_array() {
                            if self.current_level_index < levels.len() {
                                return Some(&levels[self.current_level_index]);
                            }
                        }
                        break; // Exit once we've checked the levels element
                    }
                }
            }
        }
        None
    }

    pub fn get_solids_data(&self) -> Option<String> {
        if let Some(level) = self.get_current_level() {
            for child in level["__children"].as_array()? {
                if child["__name"] == "solids" {
                    return child["innerText"].as_str().map(|s| s.to_string());
                }
            }
        }
        None
    }

    pub fn update_solids_data(&mut self, new_solids: &str) {
        if let Some(map) = &mut self.map_data {
            if let Some(levels) = map["__children"][0]["__children"].as_array_mut() {
                if self.current_level_index < levels.len() {
                    if let Some(level) = levels.get_mut(self.current_level_index) {
                        if let Some(children) = level["__children"].as_array_mut() {
                            for child in children {
                                if child["__name"] == "solids" {
                                    child["innerText"] = serde_json::json!(new_solids);
                                    // We're preserving any existing offsetX and offsetY
                                    return;
                                }
                            }

                            // If we didn't find solids, we might want to add it
                            println!("No 'solids' element found to update!");
                        }
                    }
                }
            }
        }
    }

    pub fn screen_to_map(&self, pos: egui::Pos2) -> (i32, i32) {
        let scaled_tile_size = crate::ui::render::TILE_SIZE * self.zoom_level;
        let x = ((pos.x + self.camera_pos.x) / scaled_tile_size).floor() as i32;
        let y = ((pos.y + self.camera_pos.y) / scaled_tile_size).floor() as i32;
        (x, y)
    }
}

impl eframe::App for CelesteMapEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle user input
        handle_input(self, ctx);

        // Render the application
        render_app(self, ctx);

        // Show dialogs
        if self.show_open_dialog {
            show_open_dialog(self, ctx);
        }

        if self.show_key_bindings_dialog {
            show_key_bindings_dialog(self, ctx);
        }

        if self.show_celeste_path_dialog {
            // We need to implement or import this function
            // show_celeste_path_dialog(self, ctx);
        }
    }
}