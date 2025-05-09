#![allow(dead_code, unused_imports, unused_variables)]

use eframe::egui;

use crate::app::CelesteMapEditor;
use crate::config::keybindings::{BindingType, InputBinding, InputMode, KeyBindings};
use crate::map::loader::load_map;

pub fn show_open_dialog(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::Window::new("Open Map File")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut path = editor.bin_path.clone().unwrap_or_default();
                ui.label("File path:");
                if ui.text_edit_singleline(&mut path).changed() {
                    editor.bin_path = Some(path);
                }

                if ui.button("Browse...").clicked() {
                    let mut dialog = rfd::FileDialog::new();
                    dialog = dialog.add_filter("Celeste Map", &["bin"]);
                    let mut fallback_to_home = true;
                    if let Some(celeste_dir) = &editor.celeste_assets.celeste_dir {
                        #[cfg(target_os = "macos")]
                        {
                            let maps_path = celeste_dir.join("Contents").join("Resources").join("Content").join("Maps");
                            if maps_path.exists() {
                                dialog = dialog.set_directory(&maps_path);
                                fallback_to_home = false;
                            }
                        }
                        #[cfg(any(target_os = "windows", target_os = "linux"))]
                        {
                            let maps_path = celeste_dir.join("Content").join("Maps");
                            if maps_path.exists() {
                                dialog = dialog.set_directory(&maps_path);
                                fallback_to_home = false;
                            }
                        }
                    }
                    if fallback_to_home {
                        if let Some(home_dir) = dirs::home_dir() {
                            dialog = dialog.set_directory(home_dir);
                        }
                    }
                    if let Some(path) = dialog.pick_file() {
                        editor.bin_path = Some(path.display().to_string());
                    }
                }
            });

            #[cfg(target_os = "macos")]
            if let Some(celeste_dir) = &editor.celeste_assets.celeste_dir {
                let maps_path = celeste_dir.join("Contents").join("Resources").join("Content").join("Maps");
                if !maps_path.exists() || !maps_path.starts_with(dirs::home_dir().unwrap_or_default()) {
                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Can't access Maps folder directly?").strong());
                        ui.label("Use ⌘+Shift+G in the dialog and paste the path below:");
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("Copy Maps Path").color(egui::Color32::from_rgb(0, 120, 255))).clicked() {
                                ui.output().copied_text = maps_path.display().to_string();
                            }
                            ui.add(egui::TextEdit::singleline(&mut maps_path.display().to_string()).desired_width(300.0).font(egui::TextStyle::Monospace));
                        });
                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("⚠️  On some macOS versions, using ⌘+Shift+G may crash the app due to a bug in the file dialog library. If this happens, please navigate manually or update your app to the latest version.").color(egui::Color32::from_rgb(200, 50, 50)).size(13.0));
                    });
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    editor.show_open_dialog = false;
                }

                if ui.button("Open").clicked() {
                    let path_clone = editor.bin_path.clone();
                    if let Some(path) = path_clone {
                        load_map(editor, &path);
                    }
                    editor.show_open_dialog = false;
                }
            });
        });
}

pub fn show_key_bindings_dialog(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::Window::new("Key Bindings")
        .collapsible(false)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Key Bindings");
            ui.add_space(10.0);
            
            ui.label("Note: Changes take effect immediately.");
            ui.add_space(10.0);
            
            render_binding_selector(editor, ui, "Pan Camera:", BindingType::Pan);
            render_binding_selector(editor, ui, "Place Block:", BindingType::PlaceBlock);
            render_binding_selector(editor, ui, "Remove Block:", BindingType::RemoveBlock);
            render_binding_selector(editor, ui, "Zoom In:", BindingType::ZoomIn);
            render_binding_selector(editor, ui, "Zoom Out:", BindingType::ZoomOut);
            render_binding_selector(editor, ui, "Save (Ctrl+):", BindingType::Save);
            render_binding_selector(editor, ui, "Open (Ctrl+):", BindingType::Open);
            
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                if ui.button("Reset to Default").clicked() {
                    editor.key_bindings = KeyBindings::default();
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save & Close").clicked() {
                        editor.key_bindings.save();
                        editor.show_key_bindings_dialog = false;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        // Reload bindings to discard changes
                        editor.key_bindings.load();
                        editor.show_key_bindings_dialog = false;
                    }
                });
            });
        });
}

fn render_binding_selector(editor: &mut CelesteMapEditor, ui: &mut egui::Ui, label: &str, binding_type: BindingType) {
    ui.horizontal(|ui| {
        ui.label(label);
        
        // First, show a combo box to select between Key and Mouse
        let current_mode = editor.key_bindings.get_input_mode(binding_type.clone());
        let mode_text = match current_mode {
            InputMode::Keyboard => "Keyboard Key",
            InputMode::Mouse => "Mouse Button",
        };
        
        let mut mode_changed = false;
        let mut new_mode = current_mode.clone();
        
        egui::ComboBox::from_id_source(format!("{}_type", label))
            .selected_text(mode_text)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current_mode == InputMode::Keyboard, "Keyboard Key").clicked() {
                    new_mode = InputMode::Keyboard;
                    mode_changed = true;
                }
                if ui.selectable_label(current_mode == InputMode::Mouse, "Mouse Button").clicked() {
                    new_mode = InputMode::Mouse;
                    mode_changed = true;
                }
            });
        
        // Handle mode change
        if mode_changed {
            match new_mode {
                InputMode::Keyboard => {
                    editor.key_bindings.update_binding(binding_type.clone(), InputBinding::Key(egui::Key::Space));
                },
                InputMode::Mouse => {
                    editor.key_bindings.update_binding(binding_type.clone(), InputBinding::MouseButton(egui::PointerButton::Middle));
                },
            }
        }
        
        // Then show specific options based on the current mode
        match editor.key_bindings.get_input_mode(binding_type.clone()) {
            InputMode::Keyboard => {
                if let Some(current_key) = editor.key_bindings.get_current_key(binding_type.clone()) {
                    egui::ComboBox::from_id_source(format!("{}_key", label))
                        .selected_text(format!("{:?}", current_key))
                        .show_ui(ui, |ui| {
                            for key in KeyBindings::get_all_available_keys() {
                                if ui.selectable_label(current_key == key, format!("{:?}", key)).clicked() {
                                    editor.key_bindings.update_binding(binding_type.clone(), InputBinding::Key(key));
                                }
                            }
                        });
                }
            },
            InputMode::Mouse => {
                if let Some(current_button) = editor.key_bindings.get_current_button(binding_type.clone()) {
                    egui::ComboBox::from_id_source(format!("{}_button", label))
                        .selected_text(format!("{:?}", current_button))
                        .show_ui(ui, |ui| {
                            for button in KeyBindings::get_all_available_mouse_buttons() {
                                if ui.selectable_label(current_button == button, format!("{:?}", button)).clicked() {
                                    editor.key_bindings.update_binding(binding_type.clone(), InputBinding::MouseButton(button));
                                }
                            }
                        });
                }
            },
        }
    });
}

pub fn show_celeste_path_dialog(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    egui::Window::new("Celeste Installation Path")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Celeste Installation Path");
            ui.add_space(10.0);
            
            if editor.celeste_assets.celeste_dir.is_none() {
                ui.label("Celeste installation not found!");
                ui.label("The app will try to auto-detect your Celeste installation in the default location for your OS.");
                ui.label("If it is not found, please specify the path to your Celeste installation folder manually.");
                ui.label("This is needed to load textures for the map editor.");
            } else {
                ui.label("Current Celeste installation path:");
                ui.label(editor.celeste_assets.celeste_dir.as_ref().unwrap().display().to_string());
                ui.label("You can change the path if needed.");
            }
            
            ui.add_space(10.0);
                
            ui.horizontal(|ui| {
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Celeste Installation Folder")
                        .pick_folder() {
                        if !editor.celeste_assets.set_celeste_dir(&path) {
                            editor.error_message = Some("Invalid Celeste installation directory.".to_string());
                        }
                    }
                }
                
                ui.checkbox(&mut editor.use_textures, "Use textures when available");
            });
            
            ui.add_space(10.0);
            
            let is_valid = editor.celeste_assets.celeste_dir.is_some();
            
            ui.horizontal(|ui| {
                if ui.button("Continue Without Textures").clicked() {
                    editor.use_textures = false;
                    editor.show_celeste_path_dialog = false;
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(is_valid, egui::Button::new("OK")).clicked() {
                        editor.show_celeste_path_dialog = false;
                    }
                });
            });
        });
}