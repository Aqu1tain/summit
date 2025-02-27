use eframe::{egui, epi};
use egui::{Pos2, Vec2};
use serde_json::Value;

use crate::config::keybindings::KeyBindings;
use crate::ui::render::render_app;
use crate::ui::input::handle_input;
use crate::ui::dialogs::{show_open_dialog, show_key_bindings_dialog};

pub struct CelesteMapEditor {
    pub map_data: Option<Value>,
    pub current_level_index: usize,
    pub camera_pos: Vec2,
    pub dragging: bool,
    pub drag_start: Option<Pos2>,
    pub mouse_pos: Pos2,
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
}

impl Default for CelesteMapEditor {
    fn default() -> Self {
        Self {
            map_data: None,
            current_level_index: 0,
            camera_pos: Vec2::new(0.0, 0.0),
            dragging: false,
            drag_start: None,
            mouse_pos: Pos2::new(0.0, 0.0),
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
        }
    }
}

impl CelesteMapEditor {
    pub fn new() -> Self {
        let mut editor = Self::default();
        editor.key_bindings.load();
        editor
    }

    pub fn extract_level_names(&mut self) {
        self.level_names.clear();
        if let Some(map) = &self.map_data {
            if let Some(levels) = map["__children"][0]["__children"].as_array() {
                for level in levels {
                    if level["__name"] == "level" {
                        if let Some(name) = level["name"].as_str() {
                            self.level_names.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    pub fn get_current_level(&self) -> Option<&Value> {
        if let Some(map) = &self.map_data {
            if let Some(levels) = map["__children"][0]["__children"].as_array() {
                if self.current_level_index < levels.len() {
                    return Some(&levels[self.current_level_index]);
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
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn screen_to_map(&self, pos: Pos2) -> (i32, i32) {
        let scaled_tile_size = crate::ui::render::TILE_SIZE * self.zoom_level;
        let x = ((pos.x + self.camera_pos.x) / scaled_tile_size).floor() as i32;
        let y = ((pos.y + self.camera_pos.y) / scaled_tile_size).floor() as i32;
        (x, y)
    }
}

impl epi::App for CelesteMapEditor {
    fn name(&self) -> &str {
        "Summit - Celeste Map Editor"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
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
    }
}