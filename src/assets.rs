use std::path::{Path, PathBuf};
use std::collections::HashMap;

use image::RgbaImage;
use eframe::egui;

use crate::celeste_atlas::{AtlasManager, Sprite};

/// Structure to manage Celeste installation and asset loading
pub struct CelesteAssets {
    pub celeste_dir: Option<PathBuf>,
    pub texture_cache: HashMap<String, egui::TextureHandle>,
    pub textures: HashMap<String, RgbaImage>,
    pub atlas_manager: Option<AtlasManager>,
}

impl CelesteAssets {
    pub fn new() -> Self {
        let mut assets = Self {
            celeste_dir: None,
            texture_cache: HashMap::new(),
            textures: HashMap::new(),
            atlas_manager: Some(AtlasManager::new()),
        };

        // Try to detect Celeste installation
        assets.detect_celeste_installation();

        // Try loading from saved config
        if assets.celeste_dir.is_none() {
            if let Some(path) = load_celeste_path() {
                let path = Path::new(&path);
                if Self::is_valid_celeste_dir(path) {
                    assets.celeste_dir = Some(path.to_path_buf());
                }
            }
        }

        assets
    }

    /// Initialize the atlas manager and load game assets
    pub fn init_atlas(&mut self, ctx: &egui::Context) -> bool {
        if self.celeste_dir.is_none() {
            return false;
        }

        let celeste_dir = self.celeste_dir.as_ref().unwrap();

        // Load the gameplay atlas which contains most tiles
        if let Some(atlas_manager) = &mut self.atlas_manager {
            match atlas_manager.load_atlas("Gameplay", celeste_dir, ctx) {
                Ok(_) => {
                    true
                },
                Err(e) => {
                    eprintln!("Failed to load Gameplay atlas: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn detect_celeste_installation(&mut self) {
        // Try common installation paths
        let common_paths = [
            // Windows paths
            r"C:\Program Files (x86)\Steam\steamapps\common\Celeste",
            r"C:\Program Files\Epic Games\Celeste",
            r"C:\Program Files (x86)\GOG Galaxy\Games\Celeste",

            // macOS Steam paths
            "~/Library/Application Support/Steam/steamapps/common/Celeste",
            "/Library/Application Support/Steam/steamapps/common/Celeste",
            "/Users/Shared/Library/Application Support/Steam/steamapps/common/Celeste",
            // macOS App Store or direct .app installs
            "/Applications/Celeste.app/Contents/Resources",
            "~/Applications/Celeste.app/Contents/Resources",
            // macOS itch.io or other custom installs (user may select .app directly)
            "/Applications/Celeste.app",
            "~/Applications/Celeste.app",

            // Linux paths (with expansion)
            "~/.steam/steam/steamapps/common/Celeste",
            "~/GOG Games/Celeste",
            "~/.local/share/Steam/steamapps/common/Celeste",
        ];

        for path_str in common_paths.iter() {
            let path_str = shellexpand::full(path_str).unwrap_or_else(|_| std::borrow::Cow::Borrowed(*path_str));
            let path = Path::new(path_str.as_ref());

            // On macOS, if user selected a .app bundle, check Contents/Resources inside it
            #[cfg(target_os = "macos")]
            let candidate = if path.extension().map_or(false, |ext| ext == "app") {
                let res_path = path.join("Contents").join("Resources");
                if res_path.exists() && Self::is_valid_celeste_dir(&res_path) {
                    Some(res_path)
                } else {
                    None
                }
            } else if path.exists() && Self::is_valid_celeste_dir(path) {
                Some(path.to_path_buf())
            } else if path.is_dir() {
                // Check for a .app bundle inside this directory
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.extension().map_or(false, |ext| ext == "app") {
                            let res_path = entry_path.join("Contents").join("Resources");
                            if res_path.exists() && Self::is_valid_celeste_dir(&res_path) {
                                return self.celeste_dir = Some(res_path);
                            }
                        }
                    }
                }
                None
            } else {
                None
            };

            #[cfg(not(target_os = "macos"))]
            let candidate = if path.exists() && Self::is_valid_celeste_dir(path) {
                Some(path.to_path_buf())
            } else {
                None
            };

            if let Some(valid_path) = candidate {
                self.celeste_dir = Some(valid_path);
                break;
            }
        }
    }

    fn is_valid_celeste_dir(path: &Path) -> bool {
        // Check for some expected files/directories in a Celeste installation
        let content_dir = path.join("Content");

        // Check if the Content directory exists and contains expected subdirectories
        if content_dir.exists() && content_dir.is_dir() {
            // Check if the Tileset directory exists
            let tileset_dir = content_dir.join("Graphics").join("Atlases").join("Gameplay");
            if tileset_dir.exists() && tileset_dir.is_dir() {
                return true;
            }

            // Alternate check for Celeste.exe (Windows) or Celeste executable (macOS/Linux)
            let exe_path = path.join("Celeste.exe");
            let bin_path = path.join("Celeste");
            if exe_path.exists() || bin_path.exists() {
                return true;
            }
        }

        false
    }

    pub fn load_texture(&mut self, texture_path: &str, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        // Check if we already have the texture in cache
        if self.texture_cache.contains_key(texture_path) {
            return self.texture_cache.get(texture_path);
        }

        // Try to load from atlas first
        if let Some(atlas_manager) = &self.atlas_manager {
            // Extract the first character if it's a tileset identifier
            if let Some(first_char) = texture_path.chars().next() {
                if let Some(sprite_path) = atlas_manager.get_texture_path_for_tile(first_char) {
                    // Try to get the sprite from the atlas (for .data-based atlases)
                    if let Some(sprite) = atlas_manager.get_sprite("Gameplay", sprite_path) {
                        if let Some(image) = self.extract_sprite_image(sprite, atlas_manager) {
                            let texture_handle = add_image_to_egui(ctx, &image, texture_path);
                            self.textures.insert(texture_path.to_string(), image);
                            self.texture_cache.insert(texture_path.to_string(), texture_handle);
                            return self.texture_cache.get(texture_path);
                        }
                    }
                }
            }
        }

        // Try to load from .data file directly if present (macOS/modern Celeste)
        if let Some(celeste_dir) = &self.celeste_dir {
            let data_path = celeste_dir
                .join("Content")
                .join("Graphics")
                .join("Atlases")
                .join("Gameplay")
                .join(format!("{}.data", texture_path));
            if data_path.exists() {
                if let Some(atlas_manager) = &self.atlas_manager {
                    match atlas_manager.load_data_file(&data_path) {
                        Ok(image) => {
                            let texture_handle = add_image_to_egui(ctx, &image, texture_path);
                            self.textures.insert(texture_path.to_string(), image);
                            self.texture_cache.insert(texture_path.to_string(), texture_handle);
                            return self.texture_cache.get(texture_path);
                        },
                        Err(e) => {
                            eprintln!("Failed to load .data texture {}: {}", data_path.display(), e);
                        }
                    }
                }
            }
        }

        // Fall back to loading individual PNGs
        if let Some(celeste_dir) = &self.celeste_dir {
            let full_path = celeste_dir
                .join("Content")
                .join("Graphics")
                .join("Atlases")
                .join("Gameplay")
                .join(texture_path);

            // For PNG files (pre-extracted assets)
            match load_texture_from_path(&full_path) {
                Ok(image) => {
                    let texture_handle = add_image_to_egui(ctx, &image, texture_path);
                    self.textures.insert(texture_path.to_string(), image);
                    self.texture_cache.insert(texture_path.to_string(), texture_handle);
                    return self.texture_cache.get(texture_path);
                },
                Err(e) => {
                    eprintln!("Failed to load texture {}: {}", texture_path, e);
                }
            }

            // Try XNB file as fallback
            let xnb_path = full_path.with_extension("xnb");
            if xnb_path.exists() {
                match crate::xnb_reader::extract_xnb_texture(&xnb_path) {
                    Ok(image) => {
                        let texture_handle = add_image_to_egui(ctx, &image, texture_path);
                        self.textures.insert(texture_path.to_string(), image);
                        self.texture_cache.insert(texture_path.to_string(), texture_handle);
                        return self.texture_cache.get(texture_path);
                    },
                    Err(e) => {
                        eprintln!("Failed to extract XNB texture {}: {}", xnb_path.display(), e);
                    }
                }
            }
        }

        None
    }

    /// Extract a sprite image from an atlas
    fn extract_sprite_image(&self, sprite: &Sprite, atlas_manager: &AtlasManager) -> Option<RgbaImage> {
        // Find the atlas containing this sprite
        let atlas = atlas_manager.atlases.values().find(|a| {
            a.textures.values().any(|t| t.id() == sprite.texture_id)
        })?;

        // Find the texture in the atlas
        let data_file = &sprite.data_file;
        let full_image = atlas_manager.get_atlas_image(atlas.name.as_str(), data_file)?;

        // Extract the portion of the image corresponding to this sprite
        let meta = &sprite.metadata;

        // Ensure coordinates are within bounds
        if meta.x < 0 || meta.y < 0 ||
            meta.x + meta.width > full_image.width() as i16 ||
            meta.y + meta.height > full_image.height() as i16 {
            return None;
        }

        // Create a new image with the sprite dimensions
        let mut sprite_image = RgbaImage::new(meta.width as u32, meta.height as u32);

        // Copy the sprite pixels from the atlas image
        for y in 0..meta.height {
            for x in 0..meta.width {
                let src_x = (meta.x + x) as u32;
                let src_y = (meta.y + y) as u32;
                let pixel = full_image.get_pixel(src_x, src_y);
                sprite_image.put_pixel(x as u32, y as u32, *pixel);
            }
        }

        Some(sprite_image)
    }

    pub fn set_celeste_dir(&mut self, path: &Path) -> bool {
        if Self::is_valid_celeste_dir(path) {
            self.celeste_dir = Some(path.to_path_buf());
            // Clear the texture cache to reload textures from the new path
            self.texture_cache.clear();
            self.textures.clear();

            // Reset atlas manager
            self.atlas_manager = Some(AtlasManager::new());

            // Save the path to config
            save_celeste_path(&path.to_string_lossy());

            true
        } else {
            false
        }
    }

    /// Get the appropriate texture path for a tile character
    pub fn get_texture_path_for_tile(&self, tile_char: char) -> Option<&'static str> {
        // Fixed to match the format in AtlasManager (removed .png extension)
        match tile_char {
            // Modded map tiles
            '9' => Some("tilesSolid"),     // Main solid tiles texture
            'm' => Some("mountainTiles"),  // Mountain tiles
            'n' => Some("templeTiles"),    // Temple tiles
            'a' => Some("coreTiles"),      // Core (alt) tiles

            // Base game tiles
            '1' => Some("tilesSolid"),     // Standard solid tile
            '3' => Some("tilesSolid"),     // Another standard solid tile
            '4' => Some("tilesSolid"),     // Yet another standard solid tile
            '7' => Some("tilesSolid"),     // And another standard solid tile

            // Additional tiles
            'b' => Some("reflectionTiles"), // Reflection tiles
            'c' => Some("moonTiles"),       // Moon tiles
            'd' => Some("dreamTiles"),      // Dream tiles

            _ => None
        }
    }

    /// Get a sprite from the atlas for a specific tile character
    pub fn get_sprite_for_tile(&self, tile_char: char) -> Option<&Sprite> {
        if let Some(atlas_manager) = &self.atlas_manager {
            if let Some(sprite_path) = self.get_texture_path_for_tile(tile_char) {
                return atlas_manager.get_sprite("Gameplay", sprite_path);
            }
        }
        None
    }

    /// Draw a sprite from the atlas
    pub fn draw_sprite_for_tile(&self, painter: &egui::Painter, rect: egui::Rect, tile_char: char) -> bool {
        if let Some(atlas_manager) = &self.atlas_manager {
            if let Some(sprite) = self.get_sprite_for_tile(tile_char) {
                atlas_manager.draw_sprite(sprite, painter, rect, egui::Color32::WHITE);
                return true;
            }
        }
        false
    }

    /// Normalize user-selected Celeste path for best UX
    pub fn normalize_celeste_dir(path: &std::path::Path) -> Option<std::path::PathBuf> {
        #[cfg(target_os = "macos")]
        {
            use std::fs;
            // If user selects a .app bundle, use Contents/Resources inside it
            if path.extension().map_or(false, |ext| ext == "app") {
                let res_path = path.join("Contents").join("Resources");
                if res_path.exists() {
                    return Some(res_path);
                }
            }
            // If user selects a parent directory containing a .app, use that
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.extension().map_or(false, |ext| ext == "app") {
                            let res_path = entry_path.join("Contents").join("Resources");
                            if res_path.exists() {
                                return Some(res_path);
                            }
                        }
                    }
                }
            }
            // If user selects Contents/Resources directly, use as-is if valid
            if path.ends_with("Contents/Resources") && path.exists() {
                return Some(path.to_path_buf());
            }
        }
        // On other OSes, just use as-is
        if path.exists() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }
}

fn load_texture_from_path(path: &Path) -> Result<RgbaImage, String> {
    // Load an image from a file path
    match image::open(path) {
        Ok(img) => Ok(img.to_rgba8()),
        Err(e) => Err(format!("Failed to load image: {}", e))
    }
}

fn add_image_to_egui(ctx: &egui::Context, image: &RgbaImage, name: &str) -> egui::TextureHandle {
    // Convert RgbaImage to egui::ColorImage
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.as_flat_samples();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice()
    );

    // Add the image to egui
    ctx.load_texture(name, color_image, Default::default())
}

pub fn save_celeste_path(path: &str) {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let config_path = config_dir.join("summit_editor_celeste.txt");
    if let Err(e) = std::fs::write(&config_path, path) {
        eprintln!("Failed to save Celeste path: {}", e);
    }
}

pub fn load_celeste_path() -> Option<String> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let config_path = config_dir.join("summit_editor_celeste.txt");

    if let Ok(path) = std::fs::read_to_string(config_path) {
        Some(path)
    } else {
        None
    }
}