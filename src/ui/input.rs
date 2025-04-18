use eframe::egui;

use crate::app::CelesteMapEditor;
use crate::config::keybindings::InputBinding;
use crate::map::editor::{place_block, remove_block};
use crate::map::loader::save_map;

pub fn handle_input(editor: &mut CelesteMapEditor, ctx: &egui::Context) {
    let input = ctx.input();

    // Handle mouse wheel for zooming
    let scroll_delta = input.scroll_delta.y;
    if scroll_delta != 0.0 {
        // Calculate the zoom center (use mouse position or center of screen)
        let zoom_center = input.pointer.hover_pos().unwrap_or_else(|| {
            let screen_rect = ctx.available_rect();
            egui::Pos2::new(screen_rect.width() / 2.0, screen_rect.height() / 2.0)
        });

        let old_zoom = editor.zoom_level;
        if scroll_delta > 0.0 {
            editor.zoom_level *= 1.1;
            editor.static_dirty = true;
        } else {
            editor.zoom_level /= 1.1;
            editor.static_dirty = true;
        }
        if editor.zoom_level < 0.1 {
            editor.zoom_level = 0.1;
        }
        
        // Adjust camera position to zoom toward mouse cursor
        let zoom_ratio = editor.zoom_level / old_zoom;
        let offset = (zoom_ratio - 1.0) * zoom_center.to_vec2();
        editor.camera_pos = zoom_ratio * editor.camera_pos + offset;
        editor.static_dirty = true;
    }

    // Handle keyboard shortcuts
    let zoom_in_pressed = match &editor.key_bindings.zoom_in {
        InputBinding::Key(key) => input.key_pressed(*key),
        InputBinding::MouseButton(_) => false, // Only support keys for these shortcuts
    };
    
    if zoom_in_pressed {
        editor.zoom_level *= 1.2;
        editor.static_dirty = true;
    }
    
    let zoom_out_pressed = match &editor.key_bindings.zoom_out {
        InputBinding::Key(key) => input.key_pressed(*key),
        InputBinding::MouseButton(_) => false,
    };
    
    if zoom_out_pressed {
        editor.zoom_level /= 1.2;
        if editor.zoom_level < 0.1 {
            editor.zoom_level = 0.1;
        }
        editor.static_dirty = true;
    }
    
    // Use modifiers.ctrl to check for Ctrl key instead of separate KeyCode
    let save_pressed = match &editor.key_bindings.save {
        InputBinding::Key(key) => input.key_pressed(*key) && input.modifiers.ctrl,
        InputBinding::MouseButton(_) => false,
    };
    
    if save_pressed {
        save_map(editor);
    }
    
    let open_pressed = match &editor.key_bindings.open {
        InputBinding::Key(key) => input.key_pressed(*key) && input.modifiers.ctrl,
        InputBinding::MouseButton(_) => false,
    };
    
    if open_pressed {
        editor.show_open_dialog = true;
    }

    // Handle mouse input for interaction with the map
    let pointer = &input.pointer;
    
    // Check if the pan key/button is pressed
    let pan_pressed = match &editor.key_bindings.pan {
        InputBinding::Key(key) => input.key_down(*key),
        InputBinding::MouseButton(button) => pointer.button_down(*button),
    };
    
    // Handle panning with dragging
    if pointer.is_moving() && pan_pressed {
        if !editor.dragging {
            editor.drag_start = pointer.hover_pos();
            editor.dragging = true;
        }
        
        let delta = pointer.delta();
        editor.camera_pos -= delta;
        editor.static_dirty = true;
    } else {
        editor.dragging = false;
        editor.drag_start = None;
    }
    
    // Handle placing/removing blocks
    let place_pressed = match &editor.key_bindings.place_block {
        InputBinding::Key(key) => input.key_pressed(*key),
        InputBinding::MouseButton(button) => input.pointer.any_pressed() && pointer.button_down(*button),
    };
    
    if place_pressed {
        if let Some(pos) = pointer.hover_pos() {
            place_block(editor, pos);
        }
    }

    let remove_pressed = match &editor.key_bindings.remove_block {
        InputBinding::Key(key) => input.key_pressed(*key),
        InputBinding::MouseButton(button) => input.pointer.any_pressed() && pointer.button_down(*button),
    };
    
    if remove_pressed {
        if let Some(pos) = pointer.hover_pos() {
            remove_block(editor, pos);
        }
    }
}