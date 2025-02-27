// src/celeste_atlas.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};
use eframe::egui;
use image::RgbaImage;

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
}

/// A Celeste texture atlas that contains multiple sprites
pub struct Atlas {
    pub name: String,
    pub sprites: HashMap<String, Sprite>,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub data_files: Vec<String>,
}

impl Atlas {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            sprites: HashMap::new(),
            textures: HashMap::new(),
            data_files: Vec::new(),
        }
    }

    pub fn get_sprite(&self, path: &str) -> Option<&Sprite> {
        self.sprites.get(path)
    }
}

/// Manages multiple Celeste texture atlases
pub struct AtlasManager {
    pub atlases: HashMap<String, Atlas>,
}

impl AtlasManager {
    pub fn new() -> Self {
        Self {
            atlases: HashMap::new(),
        }
    }

    /// Load a Celeste atlas from a .meta file
    pub fn load_atlas(&mut self, name: &str, celeste_dir: &Path, ctx: &egui::Context) -> io::Result<()> {
        let atlas_path = celeste_dir
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
        self.atlases.insert(name.to_string(), atlas);
        
        Ok(())
    }

    /// Load a .meta file and parse its contents
    fn load_meta_file(&self, meta_path: &Path, atlas: &mut Atlas, atlas_dir: &Path, ctx: &egui::Context) -> io::Result<()> {
        let mut file = File::open(meta_path)?;
        
        // Skip header (4 bytes signature + variable-length string + 4 bytes value)
        let _ = file.read_i32::<LittleEndian>()?;
        self.read_string(&mut file)?;
        let _ = file.read_i32::<LittleEndian>()?;
        
        // Read count of data files
        let count = file.read_i16::<LittleEndian>()?;
        
        // Read each data file
        for _ in 0..count {
            let data_file = self.read_string(&mut file)?;
            atlas.data_files.push(data_file.clone());
            
            let sprites_count = file.read_i16::<LittleEndian>()?;
            
            let data_path = atlas_dir.join(format!("{}.data", data_file));
            let image = self.load_data_file(&data_path)?;
            
            // Create texture and add to atlas
            let texture_name = format!("{}_{}", atlas.name, data_file);
            let texture_handle = self.add_image_to_egui(ctx, &image, &texture_name);
            atlas.textures.insert(data_file.clone(), texture_handle.clone());
            
            // Read each sprite in the data file
            for _ in 0..sprites_count {
                let path = self.read_string(&mut file)?;
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
                
                let sprite = Sprite {
                    metadata,
                    texture_id: texture_handle.id(),
                    data_file: data_file.clone(),
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
    fn load_data_file(&self, data_path: &Path) -> io::Result<RgbaImage> {
        let mut file = File::open(data_path)?;
        
        // Read image dimensions and alpha flag
        let width = file.read_i32::<LittleEndian>()? as u32;
        let height = file.read_i32::<LittleEndian>()? as u32;
        let has_alpha = file.read_u8()? != 0;
        
        // Create image buffer
        let mut image = RgbaImage::new(width, height);
        
        // Track RLE state
        let mut repeats_left = 0;
        let mut r = 0u8;
        let mut g = 0u8;
        let mut b = 0u8;
        let mut a = 255u8;
        
        for y in 0..height {
            for x in 0..width {
                if repeats_left == 0 {
                    // Read new pixel and repeat count
                    let rep = file.read_u8()?;
                    repeats_left = rep - 1;
                    
                    if has_alpha {
                        let alpha = file.read_u8()?;
                        
                        if alpha > 0 {
                            a = alpha;
                            // Celeste stores BGR, we need RGB
                            b = file.read_u8()?;
                            g = file.read_u8()?;
                            r = file.read_u8()?;
                            
                            // Un-premultiply alpha
                            if alpha < 255 {
                                let alpha_f = alpha as f32 / 255.0;
                                r = (r as f32 / alpha_f).min(255.0) as u8;
                                g = (g as f32 / alpha_f).min(255.0) as u8;
                                b = (b as f32 / alpha_f).min(255.0) as u8;
                            }
                        } else {
                            r = 0;
                            g = 0;
                            b = 0;
                            a = 0;
                        }
                    } else {
                        // No alpha channel
                        b = file.read_u8()?;
                        g = file.read_u8()?;
                        r = file.read_u8()?;
                        a = 255;
                    }
                    
                    image.put_pixel(x, y, image::Rgba([r, g, b, a]));
                } else {
                    // Repeat the previous pixel
                    image.put_pixel(x, y, image::Rgba([r, g, b, a]));
                    repeats_left -= 1;
                }
            }
        }
        
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
        
        ctx.load_texture(name, color_image, Default::default())
    }

    /// Get a sprite by path from a specific atlas
    pub fn get_sprite(&self, atlas_name: &str, sprite_path: &str) -> Option<&Sprite> {
        self.atlases.get(atlas_name).and_then(|atlas| atlas.get_sprite(sprite_path))
    }

    /// Draw a sprite to the screen
    pub fn draw_sprite(&self, sprite: &Sprite, painter: &egui::Painter, rect: egui::Rect, tint: egui::Color32) {
        let atlas = self.atlases.values().find(|a| {
            a.textures.values().any(|t| t.id() == sprite.texture_id)
        });
        
        if let Some(atlas) = atlas {
            let texture = atlas.textures.values().find(|t| t.id() == sprite.texture_id).unwrap();
            let atlas_width = texture.size_vec2().x;
            let atlas_height = texture.size_vec2().y;
            
            // Calculate UV coordinates for the sprite within the atlas
            let uv_min = egui::pos2(
                sprite.metadata.x as f32 / atlas_width,
                sprite.metadata.y as f32 / atlas_height,
            );
            let uv_max = egui::pos2(
                (sprite.metadata.x as f32 + sprite.metadata.width as f32) / atlas_width,
                (sprite.metadata.y as f32 + sprite.metadata.height as f32) / atlas_height,
            );
            
            let uv_rect = egui::Rect::from_min_max(uv_min, uv_max);
            
            // Create mesh for the sprite
            let mut mesh = egui::epaint::Mesh::with_texture(sprite.texture_id);
            mesh.add_rect_with_uv(rect, uv_rect, tint);
            painter.add(egui::epaint::Shape::mesh(mesh));
        }
    }

    /// Get texture path for common Celeste tile characters
    pub fn get_texture_path_for_tile(&self, tile_char: char) -> Option<&'static str> {
        match tile_char {
            '9' => Some("tilesSolid"),        // Main solid tiles
            'm' => Some("mountainTiles"),     // Mountain tiles
            'n' => Some("templeTiles"),       // Temple tiles
            'a' => Some("coreTiles"),         // Core (alt) tiles
            'b' => Some("reflectionTiles"),   // Reflection tiles
            'c' => Some("moonTiles"),         // Moon tiles
            'd' => Some("dreamTiles"),        // Dream tiles
            // Add more mappings as needed
            _ => None
        }
    }
}