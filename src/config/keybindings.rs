use eframe::egui;
use std::fmt;
use serde::{Serialize, Deserialize};
use log::debug;

#[derive(Clone, Debug, PartialEq)]
pub enum InputBinding {
    Key(egui::Key),
    MouseButton(egui::PointerButton),
}

#[derive(Clone, Debug)]
pub struct KeyBindings {
    pub pan: InputBinding,
    pub place_block: InputBinding,
    pub remove_block: InputBinding,
    pub zoom_in: InputBinding,
    pub zoom_out: InputBinding,
    pub save: InputBinding,
    pub open: InputBinding,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputMode {
    Keyboard,
    Mouse,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingType {
    Pan,
    PlaceBlock,
    RemoveBlock,
    ZoomIn,
    ZoomOut,
    Save,
    Open,
}

#[derive(Serialize, Deserialize)]
struct SerializableKeyBindings {
    pan: String,
    place_block: String,
    remove_block: String,
    zoom_in: String,
    zoom_out: String,
    save: String,
    open: String,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            pan: InputBinding::MouseButton(egui::PointerButton::Middle),
            place_block: InputBinding::MouseButton(egui::PointerButton::Primary),
            remove_block: InputBinding::MouseButton(egui::PointerButton::Secondary),
            zoom_in: InputBinding::Key(egui::Key::E),
            zoom_out: InputBinding::Key(egui::Key::Q),
            save: InputBinding::Key(egui::Key::S),
            open: InputBinding::Key(egui::Key::O),
        }
    }
}

impl fmt::Display for InputBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputBinding::Key(key) => write!(f, "Key: {:?}", key),
            InputBinding::MouseButton(button) => write!(f, "Mouse: {:?}", button),
        }
    }
}

impl KeyBindings {
    // Convert to serializable format
    fn to_serializable(&self) -> SerializableKeyBindings {
        SerializableKeyBindings {
            pan: self.binding_to_string(&self.pan),
            place_block: self.binding_to_string(&self.place_block),
            remove_block: self.binding_to_string(&self.remove_block),
            zoom_in: self.binding_to_string(&self.zoom_in),
            zoom_out: self.binding_to_string(&self.zoom_out),
            save: self.binding_to_string(&self.save),
            open: self.binding_to_string(&self.open),
        }
    }

    fn binding_to_string(&self, binding: &InputBinding) -> String {
        match binding {
            InputBinding::Key(key) => format!("Key:{:?}", key),
            InputBinding::MouseButton(button) => format!("Mouse:{:?}", button),
        }
    }

    // Convert from serializable format
    fn from_serializable(serial: &SerializableKeyBindings) -> Self {
        // Default fallback values
        let mut bindings = Self::default();
        
        // Parse serialized bindings
        bindings.pan = Self::parse_binding(&serial.pan, bindings.pan);
        bindings.place_block = Self::parse_binding(&serial.place_block, bindings.place_block);
        bindings.remove_block = Self::parse_binding(&serial.remove_block, bindings.remove_block);
        bindings.zoom_in = Self::parse_binding(&serial.zoom_in, bindings.zoom_in);
        bindings.zoom_out = Self::parse_binding(&serial.zoom_out, bindings.zoom_out);
        bindings.save = Self::parse_binding(&serial.save, bindings.save);
        bindings.open = Self::parse_binding(&serial.open, bindings.open);
        
        bindings
    }
    
    fn parse_binding(binding_str: &str, default: InputBinding) -> InputBinding {
        if binding_str.starts_with("Key:") {
            let key_str = binding_str.trim_start_matches("Key:");
            match key_str {
                "Space" => InputBinding::Key(egui::Key::Space),
                "E" => InputBinding::Key(egui::Key::E),
                "Q" => InputBinding::Key(egui::Key::Q),
                "Z" => InputBinding::Key(egui::Key::Z),
                "X" => InputBinding::Key(egui::Key::X),
                "S" => InputBinding::Key(egui::Key::S),
                "O" => InputBinding::Key(egui::Key::O),
                "A" => InputBinding::Key(egui::Key::A),
                "W" => InputBinding::Key(egui::Key::W),
                "D" => InputBinding::Key(egui::Key::D),
                // Add more keys as needed
                _ => default,
            }
        } else if binding_str.starts_with("Mouse:") {
            let button_str = binding_str.trim_start_matches("Mouse:");
            match button_str {
                "Primary" => InputBinding::MouseButton(egui::PointerButton::Primary),
                "Secondary" => InputBinding::MouseButton(egui::PointerButton::Secondary),
                "Middle" => InputBinding::MouseButton(egui::PointerButton::Middle),
                _ => default,
            }
        } else {
            default
        }
    }
    
    pub fn get_all_available_keys() -> Vec<egui::Key> {
        vec![
            egui::Key::Space,
            egui::Key::A, egui::Key::B, egui::Key::C, egui::Key::D, egui::Key::E,
            egui::Key::F, egui::Key::G, egui::Key::H, egui::Key::I, egui::Key::J,
            egui::Key::K, egui::Key::L, egui::Key::M, egui::Key::N, egui::Key::O,
            egui::Key::P, egui::Key::Q, egui::Key::R, egui::Key::S, egui::Key::T,
            egui::Key::U, egui::Key::V, egui::Key::W, egui::Key::X, egui::Key::Y,
            egui::Key::Z,
        ]
    }
    
    pub fn get_all_available_mouse_buttons() -> Vec<egui::PointerButton> {
        vec![
            egui::PointerButton::Primary,
            egui::PointerButton::Secondary,
            egui::PointerButton::Middle,
        ]
    }
    
    pub fn save(&self) {
        let serializable = self.to_serializable();
        if let Ok(bindings_json) = serde_json::to_string_pretty(&serializable) {
            let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let config_path = config_dir.join("summit_editor_keys.json");
            if let Err(e) = std::fs::write(&config_path, bindings_json) {
                #[cfg(debug_assertions)]
                debug!("Failed to save key bindings: {}", e);
            }
        }
    }
    
    pub fn load(&mut self) {
        let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let config_path = config_dir.join("summit_editor_keys.json");
        
        if let Ok(file) = std::fs::File::open(config_path) {
            let reader = std::io::BufReader::new(file);
            if let Ok(serializable) = serde_json::from_reader::<_, SerializableKeyBindings>(reader) {
                *self = Self::from_serializable(&serializable);
            }
        }
    }
    
    pub fn get_input_mode(&self, binding_type: BindingType) -> InputMode {
        let binding = match binding_type {
            BindingType::Pan => &self.pan,
            BindingType::PlaceBlock => &self.place_block,
            BindingType::RemoveBlock => &self.remove_block,
            BindingType::ZoomIn => &self.zoom_in,
            BindingType::ZoomOut => &self.zoom_out,
            BindingType::Save => &self.save,
            BindingType::Open => &self.open,
        };
        
        match binding {
            InputBinding::Key(_) => InputMode::Keyboard,
            InputBinding::MouseButton(_) => InputMode::Mouse,
        }
    }
    
    pub fn get_current_key(&self, binding_type: BindingType) -> Option<egui::Key> {
        let binding = match binding_type {
            BindingType::Pan => &self.pan,
            BindingType::PlaceBlock => &self.place_block,
            BindingType::RemoveBlock => &self.remove_block,
            BindingType::ZoomIn => &self.zoom_in,
            BindingType::ZoomOut => &self.zoom_out,
            BindingType::Save => &self.save,
            BindingType::Open => &self.open,
        };
        
        match binding {
            InputBinding::Key(key) => Some(*key),
            _ => None,
        }
    }
    
    pub fn get_current_button(&self, binding_type: BindingType) -> Option<egui::PointerButton> {
        let binding = match binding_type {
            BindingType::Pan => &self.pan,
            BindingType::PlaceBlock => &self.place_block,
            BindingType::RemoveBlock => &self.remove_block,
            BindingType::ZoomIn => &self.zoom_in,
            BindingType::ZoomOut => &self.zoom_out,
            BindingType::Save => &self.save,
            BindingType::Open => &self.open,
        };
        
        match binding {
            InputBinding::MouseButton(button) => Some(*button),
            _ => None,
        }
    }
    
    pub fn update_binding(&mut self, binding_type: BindingType, new_binding: InputBinding) {
        match binding_type {
            BindingType::Pan => self.pan = new_binding,
            BindingType::PlaceBlock => self.place_block = new_binding,
            BindingType::RemoveBlock => self.remove_block = new_binding,
            BindingType::ZoomIn => self.zoom_in = new_binding,
            BindingType::ZoomOut => self.zoom_out = new_binding,
            BindingType::Save => self.save = new_binding,
            BindingType::Open => self.open = new_binding,
        }
    }
}