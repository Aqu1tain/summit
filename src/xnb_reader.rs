// src/xnb_reader.rs
use std::io::{self, Read, Seek};
use std::fs::File;
use std::path::Path;
use image::RgbaImage;

use crate::binary_reader::BinaryReader;

/// Reads XNB file format used by XNA/MonoGame (and Celeste)
pub struct XnbReader<R: Read + Seek> {
    reader: BinaryReader<R>,
}

impl<R: Read + Seek> XnbReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BinaryReader::new(reader),
        }
    }

    /// Read an XNB texture file and extract the image data
    pub fn read_texture(&mut self) -> io::Result<RgbaImage> {
        // XNB header
        let mut magic = [0u8; 3];
        self.reader.reader.read_exact(&mut magic)?;
        if &magic != b"XNB" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a valid XNB file (wrong magic number)",
            ));
        }

        // Platform and flags
        let platform = self.reader.read_byte()?;
        let version = self.reader.read_byte()?;
        let flag = self.reader.read_byte()?;

        // File size (including header)
        let file_size = self.reader.read_ulong()?;

        // If the content is compressed, we would need to decompress it
        // Fix: Cast flag to u8 before applying the bit mask
        let compressed = ((flag as u8) & 0x80) != 0;
        if compressed {
            // Decompression is complex and depends on the specific compression
            // algorithm used. For a full implementation, consider using a
            // library like lz4 or inflater.
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Compressed XNB files are not supported yet",
            ));
        }

        // Type readers count
        let type_readers_count = self.reader.read_byte()? as u32;
        
        // Skip type readers
        for _ in 0..type_readers_count {
            let _type_reader = self.reader.read_string()?;
            let _type_reader_version = self.reader.read_long()?;
        }

        // Shared resources count
        let shared_resources_count = self.reader.read_byte()? as u32;
        
        // Skip shared resources
        for _ in 0..shared_resources_count {
            // No shared resources in texture files
        }

        // Object data
        let texture_type = self.reader.read_byte()?;
        if texture_type != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a texture2D data type",
            ));
        }

        // Read texture format
        let format = self.reader.read_long()?;
        
        // Read width and height
        let width = self.reader.read_ulong()?;
        let height = self.reader.read_ulong()?;
        
        // Read mipmap count
        let _mipmap_count = self.reader.read_ulong()?;
        
        // Read texture data size
        let data_size = self.reader.read_ulong()?;
        
        // Read actual texture data
        let mut data = vec![0u8; data_size as usize];
        self.reader.reader.read_exact(&mut data)?;
        
        // Now we need to convert the texture data to an RGBA image
        // The format value tells us how the pixel data is stored
        match format {
            0 => self.decode_format_color(data, width as u32, height as u32),
            // Add more format handlers as needed
            _ => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("Texture format {} not supported", format),
            )),
        }
    }

    /// Decode the Color format (32-bit RGBA)
    fn decode_format_color(&self, data: Vec<u8>, width: u32, height: u32) -> io::Result<RgbaImage> {
        if data.len() != (width * height * 4) as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid texture data size",
            ));
        }

        let mut image = RgbaImage::new(width, height);
        
        // XNA stores color as ABGR, we need RGBA
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let a = data[offset];
                let b = data[offset + 1];
                let g = data[offset + 2];
                let r = data[offset + 3];
                
                image.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }
        
        Ok(image)
    }
}

/// Read an XNB file and extract the texture
pub fn extract_xnb_texture(path: &Path) -> io::Result<RgbaImage> {
    let file = File::open(path)?;
    let mut reader = XnbReader::new(file);
    reader.read_texture()
}