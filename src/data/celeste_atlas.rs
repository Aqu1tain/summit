#![allow(dead_code, unused_imports, unused_variables)]

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;
use std::sync::{Arc, Mutex};
use byteorder::{LittleEndian, ReadBytesExt};
use eframe::egui;
use image::RgbaImage;
use lazy_static::lazy_static;
use log::{debug, info, warn, error};

/// Metadata for a sprite in a Celeste atlas
#[derive(Debug, Clone)]
pub struct SpriteMetadata {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    pub offset_x: i16,
    pub offset_y: i16,
    pub real_width: i16,
    pub real_height: i16,
}

/// Represents a sprite from a Celeste atlas
#[derive(Debug, Clone)]
pub struct Sprite {
    pub metadata: SpriteMetadata,
    pub texture_id: egui::TextureId,
    pub data_file: String,
    // Added pre-computed UV coordinates for faster rendering
    pub uv_rect: Option<egui::Rect>,
}

/// A Celeste texture atlas that contains multiple sprites
pub struct Atlas {
    pub name: String,
    pub sprites: HashMap<String, Sprite>,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub data_files: Vec<String>,
    // Added to store raw image data for sprite extraction
    pub images: HashMap<String, RgbaImage>,
}

lazy_static! {
    pub static ref GLOBAL_SPRITE_MAP: Mutex<HashMap<String, (String, Sprite)>> = Mutex::new(HashMap::new());
}

impl Atlas {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sprites: HashMap::new(),
            textures: HashMap::new(),
            data_files: Vec::new(),
            images: HashMap::new(),
        }
    }

    pub fn get_sprite(&self, path: &str) -> Option<&Sprite> {
        self.sprites.get(path)
    }
}

/// Manages multiple Celeste texture atlases
pub struct AtlasManager {
    pub atlases: HashMap<String, Atlas>,
    // Cache for faster atlas lookup by texture ID
    texture_id_to_atlas: HashMap<egui::TextureId, String>,
}

impl AtlasManager {
    pub fn new() -> Self {
        Self {
            atlases: HashMap::new(),
            texture_id_to_atlas: HashMap::new(),
        }
    }

    /// Load a Celeste atlas from a .meta file
    pub fn load_atlas(&mut self, name: &str, celeste_dir: &Path, ctx: &egui::Context) -> io::Result<()> {
        debug!("Loading atlas '{}'", name);
        // On MacOS, Celeste's assets are inside Celeste.app/Contents/Resources/Content/Graphics/Atlases
        // If the provided celeste_dir contains 'Celeste.app', use as-is. Otherwise, append 'Celeste.app'.
        let mut atlas_base = celeste_dir.to_path_buf();
        #[cfg(target_os = "macos")]
        {
            use std::ffi::OsStr;
            if !celeste_dir.ends_with("Celeste.app") {
                atlas_base = atlas_base.join("Celeste.app");
            }
            // Always append Contents/Resources
            atlas_base = atlas_base.join("Contents").join("Resources");
        }
        let atlas_path = atlas_base
            .join("Content")
            .join("Graphics")
            .join("Atlases");

        let meta_path = atlas_path.join(format!("{}.meta", name));

        if !meta_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Meta file not found: {}", meta_path.display())
            ));
        }

        let mut atlas = Atlas::new(name);
        self.load_meta_file(&meta_path, &mut atlas, &atlas_path, ctx)?;

        debug!("Loaded {} sprites in atlas '{}'", atlas.sprites.len(), name);
        debug!("Loaded {} textures in atlas '{}'", atlas.textures.len(), name);
        debug!("Loaded {} images in atlas '{}'", atlas.images.len(), name);

        // Update texture ID to atlas mapping
        for texture in atlas.textures.values() {
            self.texture_id_to_atlas.insert(texture.id(), name.to_string());
        }

        // Register all sprites in the global mapping
        for (path, sprite) in &atlas.sprites {
            // Ensure keys are stored as-is (should already be normalized with "decals/" prefix)
            Self::register_sprite_global(name, path, sprite);
        }

        self.atlases.insert(name.to_string(), atlas);

        Ok(())
    }

    /// Load a .meta file and parse its contents
    fn load_meta_file(&self, meta_path: &Path, atlas: &mut Atlas, atlas_dir: &Path, ctx: &egui::Context) -> io::Result<()> {
        let mut file = File::open(meta_path)?;

        // Split into smaller functions for clarity
        self.read_meta_header(&mut file)?;
        self.read_atlas_data(&mut file, atlas, atlas_dir, ctx)
    }

    /// Read the meta file header
    fn read_meta_header(&self, file: &mut File) -> io::Result<()> {
        // Skip header (4 bytes signature + variable-length string + 4 bytes value)
        let _ = file.read_i32::<LittleEndian>()?;
        self.read_string(file)?;
        let _ = file.read_i32::<LittleEndian>()?;
        Ok(())
    }

    /// Read the actual atlas data
    fn read_atlas_data(&self, file: &mut File, atlas: &mut Atlas, atlas_dir: &Path, ctx: &egui::Context) -> io::Result<()> {
        // Read count of data files
        let count = file.read_i16::<LittleEndian>()?;

        // Read each data file
        for _ in 0..count {
            let data_file = self.read_string(file)?;
            atlas.data_files.push(data_file.clone());

            let sprites_count = file.read_i16::<LittleEndian>()?;

            let data_path = atlas_dir.join(format!("{}.data", data_file));
            let image = self.load_data_file(&data_path)?;

            // Store the raw image for later sprite extraction
            atlas.images.insert(data_file.clone(), image.clone());

            // Create texture and add to atlas
            let texture_name = format!("{}_{}", atlas.name, data_file);
            let texture_handle = self.add_image_to_egui(ctx, &image, &texture_name);
            let texture_id = texture_handle.id();
            atlas.textures.insert(data_file.clone(), texture_handle);

            // Size needed for UV calculations
            let atlas_width = image.width() as f32;
            let atlas_height = image.height() as f32;

            // Read each sprite in the data file
            for _ in 0..sprites_count {
                let path = self.read_string(file)?;
                let path = path.replace("\\", "/");

                let metadata = SpriteMetadata {
                    x: file.read_i16::<LittleEndian>()?,
                    y: file.read_i16::<LittleEndian>()?,
                    width: file.read_i16::<LittleEndian>()?,
                    height: file.read_i16::<LittleEndian>()?,
                    offset_x: file.read_i16::<LittleEndian>()?,
                    offset_y: file.read_i16::<LittleEndian>()?,
                    real_width: file.read_i16::<LittleEndian>()?,
                    real_height: file.read_i16::<LittleEndian>()?,
                };

                // Pre-compute UV coordinates
                let uv_min = egui::pos2(
                    metadata.x as f32 / atlas_width,
                    metadata.y as f32 / atlas_height,
                );
                let uv_max = egui::pos2(
                    (metadata.x as f32 + metadata.width as f32) / atlas_width,
                    (metadata.y as f32 + metadata.height as f32) / atlas_height,
                );
                let uv_rect = egui::Rect::from_min_max(uv_min, uv_max);

                let sprite = Sprite {
                    metadata,
                    texture_id,
                    data_file: data_file.clone(),
                    uv_rect: Some(uv_rect),
                };

                atlas.sprites.insert(path, sprite);
            }
        }

        Ok(())
    }

    /// Read a variable-length string from a binary file
    fn read_string<R: Read>(&self, reader: &mut R) -> io::Result<String> {
        let length = reader.read_u8()? as usize;
        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer)?;

        String::from_utf8(buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Load a Celeste .data file which contains a run-length encoded image
    pub fn load_data_file(&self, data_path: &Path) -> io::Result<RgbaImage> {
        use std::io::Read;
        debug!("Attempting to open .data file: {}", data_path.display());
        let mut file = File::open(data_path)?;

        // Read header: width (i32), height (i32), has_alpha (u8)
        let width = file.read_i32::<LittleEndian>()? as u32;
        let height = file.read_i32::<LittleEndian>()? as u32;
        let has_alpha = file.read_u8()? != 0;
        debug!("width: {width}, height: {height}, has_alpha: {has_alpha}");

        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        let mut total_pixels = 0u32;

        let mut repeats_left = 0u8;
        let mut r = 0u8;
        let mut g = 0u8;
        let mut b = 0u8;
        let mut a = 255u8;

        while total_pixels < width * height {
            if repeats_left == 0 {
                let rep = file.read_u8()?;
                repeats_left = rep;
                if has_alpha {
                    let alpha = file.read_u8()?;
                    if alpha > 0 {
                        b = file.read_u8()?;
                        g = file.read_u8()?;
                        r = file.read_u8()?;
                        a = alpha;
                    } else {
                        r = 0;
                        g = 0;
                        b = 0;
                        a = 0;
                    }
                } else {
                    b = file.read_u8()?;
                    g = file.read_u8()?;
                    r = file.read_u8()?;
                    a = 255;
                }
            }
            // Write pixel
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
            repeats_left -= 1;
            total_pixels += 1;
        }

        debug!("Finished decoding. Total pixels: {}", pixels.len() / 4);
        let image = RgbaImage::from_vec(width, height, pixels)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "failed to create image from buffer"))?;
        Ok(image)
    }

    /// Convert RgbaImage to egui texture
    fn add_image_to_egui(&self, ctx: &egui::Context, image: &RgbaImage, name: &str) -> egui::TextureHandle {
        let size = [image.width() as usize, image.height() as usize];
        let pixels = image.as_flat_samples();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            pixels.as_slice()
        );

        ctx.load_texture(name, color_image, egui::TextureFilter::Nearest)
    }

    /// Get a sprite by path from a specific atlas
    pub fn get_sprite(&self, atlas_name: &str, sprite_path: &str) -> Option<&Sprite> {
        if let Some(atlas) = self.atlases.get(atlas_name) {
            if !atlas.sprites.contains_key(sprite_path) {
                debug!("Sprite not found: '{}'. Available keys (first 10): {:?}", sprite_path, atlas.sprites.keys().take(10).collect::<Vec<_>>());
            } else {
                debug!("Sprite found: '{}'", sprite_path);
            }
            atlas.get_sprite(sprite_path)
        } else {
            debug!("Atlas '{}' not found!", atlas_name);
            None
        }
    }

    /// Get the raw image data from an atlas
    pub fn get_atlas_image(&self, atlas_name: &str, data_file: &str) -> Option<&RgbaImage> {
        debug!("get_atlas_image('{}', '{}')", atlas_name, data_file);
        self.atlases.get(atlas_name)?.images.get(data_file)
    }

    /// Draw a sprite to the screen
    pub fn draw_sprite(&self, sprite: &Sprite, painter: &egui::Painter, rect: egui::Rect, tint: egui::Color32) {
        // Use the pre-computed UV coordinates if available
        if let Some(uv_rect) = &sprite.uv_rect {
            // Create mesh for the sprite
            let mut mesh = egui::epaint::Mesh::with_texture(sprite.texture_id);
            mesh.add_rect_with_uv(rect, *uv_rect, tint);
            painter.add(egui::epaint::Shape::mesh(mesh));
            return;
        }

        // Fall back to computing UV coordinates on the fly if needed
        // This should rarely happen since we pre-compute UVs when loading
        let atlas_name = match self.texture_id_to_atlas.get(&sprite.texture_id) {
            Some(name) => name,
            None => return, // Can't find the atlas, can't draw the sprite
        };

        let atlas = match self.atlases.get(atlas_name) {
            Some(atlas) => atlas,
            None => return, // Can't find the atlas, can't draw the sprite
        };

        let texture = atlas.textures.values().find(|t| t.id() == sprite.texture_id).unwrap();
        let atlas_width = texture.size_vec2().x;
        let atlas_height = texture.size_vec2().y;

        // Sprite metadata gives the position of the full tileset in the atlas
        let sprite_x = sprite.metadata.x as f32;
        let sprite_y = sprite.metadata.y as f32;
        // Compute UV coordinates for the sprite within the atlas
        let uv_min = egui::pos2(
            sprite_x / atlas_width,
            sprite_y / atlas_height,
        );
        let uv_max = egui::pos2(
            (sprite_x + sprite.metadata.width as f32) / atlas_width,
            (sprite_y + sprite.metadata.height as f32) / atlas_height,
        );

        let uv_rect = egui::Rect::from_min_max(uv_min, uv_max);

        // Create mesh for the sprite
        let mut mesh = egui::epaint::Mesh::with_texture(sprite.texture_id);
        mesh.add_rect_with_uv(rect, uv_rect, tint);
        painter.add(egui::epaint::Shape::mesh(mesh));
    }

    /// Draw a sprite subregion to the screen (e.g., an 8x8 tile from a tileset)
    pub fn draw_sprite_region(
        &self,
        sprite: &Sprite,
        painter: &egui::Painter,
        rect: egui::Rect,
        tint: egui::Color32,
        region: egui::Rect, // in sprite-local pixel coordinates
    ) {
        let atlas_name = match self.texture_id_to_atlas.get(&sprite.texture_id) {
            Some(name) => name,
            None => return,
        };
        let atlas = match self.atlases.get(atlas_name) {
            Some(atlas) => atlas,
            None => return,
        };
        let texture = atlas.textures.values().find(|t| t.id() == sprite.texture_id).unwrap();
        let atlas_width = texture.size_vec2().x;
        let atlas_height = texture.size_vec2().y;
        // Sprite metadata gives the position of the full tileset in the atlas
        let sprite_x = sprite.metadata.x as f32;
        let sprite_y = sprite.metadata.y as f32;
        // Compute UVs for the subregion
        let uv_min = egui::pos2(
            (sprite_x + region.min.x) / atlas_width,
            (sprite_y + region.min.y) / atlas_height,
        );
        let uv_max = egui::pos2(
            (sprite_x + region.max.x) / atlas_width,
            (sprite_y + region.max.y) / atlas_height,
        );
        let uv_rect = egui::Rect::from_min_max(uv_min, uv_max);
        // Create mesh for the subregion
        let mut mesh = egui::epaint::Mesh::with_texture(sprite.texture_id);
        mesh.add_rect_with_uv(rect, uv_rect, tint);
        painter.add(egui::epaint::Shape::mesh(mesh));
    }

    /// Register a sprite globally
    pub fn register_sprite_global(atlas_name: &str, path: &str, sprite: &Sprite) {
        GLOBAL_SPRITE_MAP.lock().unwrap().insert(path.to_string(), (atlas_name.to_string(), sprite.clone()));
    }

    /// Get a sprite globally by path
    pub fn get_sprite_global(path: &str) -> Option<(String, Sprite)> {
        GLOBAL_SPRITE_MAP.lock().unwrap().get(path).cloned()
    }
}