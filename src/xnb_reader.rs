#![allow(dead_code, unused_imports, unused_variables)]

// src/xnb_reader.rs
use std::io::{self, Read, Seek};
use std::fs::File;
use std::path::Path;
use image::RgbaImage;

use crate::binary_reader::BinaryReader;

// Custom error type for XNB-related errors
#[derive(Debug)]
pub enum XnbError {
    IoError(io::Error),
    InvalidFormat(String),
    UnsupportedFeature(String),
}

impl From<io::Error> for XnbError {
    fn from(err: io::Error) -> Self {
        XnbError::IoError(err)
    }
}

impl From<XnbError> for io::Error {
    fn from(err: XnbError) -> Self {
        match err {
            XnbError::IoError(e) => e,
            XnbError::InvalidFormat(msg) => io::Error::new(io::ErrorKind::InvalidData, msg),
            XnbError::UnsupportedFeature(msg) => io::Error::new(io::ErrorKind::Unsupported, msg),
        }
    }
}

type Result<T> = std::result::Result<T, XnbError>;

/// Supported XNB texture formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFormat {
    Color = 0,
    Bgr565 = 1,
    Bgra5551 = 2,
    Bgra4444 = 3,
    Dxt1 = 4,
    Dxt3 = 5,
    Dxt5 = 6,
    NormalizedByte2 = 7,
    NormalizedByte4 = 8,
    Rgba1010102 = 9,
    Rg32 = 10,
    Rgba64 = 11,
    Alpha8 = 12,
    Single = 13,
    Vector2 = 14,
    Vector4 = 15,
    HalfSingle = 16,
    HalfVector2 = 17,
    HalfVector4 = 18,
    HdrBlendable = 19,
    Unknown,
}

impl From<i32> for TextureFormat {
    fn from(value: i32) -> Self {
        match value {
            0 => TextureFormat::Color,
            1 => TextureFormat::Bgr565,
            2 => TextureFormat::Bgra5551,
            3 => TextureFormat::Bgra4444,
            4 => TextureFormat::Dxt1,
            5 => TextureFormat::Dxt3,
            6 => TextureFormat::Dxt5,
            7 => TextureFormat::NormalizedByte2,
            8 => TextureFormat::NormalizedByte4,
            9 => TextureFormat::Rgba1010102,
            10 => TextureFormat::Rg32,
            11 => TextureFormat::Rgba64,
            12 => TextureFormat::Alpha8,
            13 => TextureFormat::Single,
            14 => TextureFormat::Vector2,
            15 => TextureFormat::Vector4,
            16 => TextureFormat::HalfSingle,
            17 => TextureFormat::HalfVector2,
            18 => TextureFormat::HalfVector4,
            19 => TextureFormat::HdrBlendable,
            _ => TextureFormat::Unknown,
        }
    }
}

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
    pub fn read_texture(&mut self) -> Result<RgbaImage> {
        // Parse the XNB header
        self.parse_header()?;

        // Skip type readers and shared resources
        self.skip_readers_and_resources()?;

        // Read texture data
        self.read_texture_data()
    }

    /// Parse the XNB file header
    fn parse_header(&mut self) -> Result<()> {
        // XNB magic number
        let mut magic = [0u8; 3];
        self.reader.get_reader_mut().read_exact(&mut magic)?;

        if &magic != b"XNB" {
            return Err(XnbError::InvalidFormat("Not a valid XNB file (wrong magic number)".to_string()));
        }

        // Platform, version and flags
        let _platform = self.reader.read_byte()?;
        let _version = self.reader.read_byte()?;
        let flag = self.reader.read_byte()? as u8;

        // File size (including header)
        let _file_size = self.reader.read_ulong()?;

        // Check if the content is compressed
        let compressed = (flag & 0x80) != 0;
        if compressed {
            return Err(XnbError::UnsupportedFeature("Compressed XNB files are not supported yet".to_string()));
        }

        Ok(())
    }

    /// Skip type readers and shared resources sections
    fn skip_readers_and_resources(&mut self) -> Result<()> {
        // Type readers count
        let type_readers_count = self.reader.read_byte()? as u32;

        // Skip type readers
        for _ in 0..type_readers_count {
            let _type_reader = self.reader.read_string()?;
            let _type_reader_version = self.reader.read_long()?;
        }

        // Shared resources count
        let shared_resources_count = self.reader.read_byte()? as u32;

        // Skip shared resources (typically none for textures)
        for _ in 0..shared_resources_count {
            // No resources to skip in texture files
        }

        Ok(())
    }

    /// Read the actual texture data
    fn read_texture_data(&mut self) -> Result<RgbaImage> {
        // Object data - should be 1 for Texture2D
        let texture_type = self.reader.read_byte()?;
        if texture_type != 1 {
            return Err(XnbError::InvalidFormat(format!("Not a texture2D data type (got {})", texture_type)));
        }

        // Read texture format
        let format_value = self.reader.read_long()?;
        let format = TextureFormat::from(format_value);

        // Read width and height
        let width = self.reader.read_ulong()? as u32;
        let height = self.reader.read_ulong()? as u32;

        // Validate dimensions
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(XnbError::InvalidFormat(format!("Invalid texture dimensions: {}x{}", width, height)));
        }

        // Read mipmap count
        let mipmap_count = self.reader.read_ulong()?;

        // Read texture data size
        let data_size = self.reader.read_ulong()? as usize;

        // Validate data size
        let expected_min_size = match format {
            TextureFormat::Color => width * height * 4, // 4 bytes per pixel for RGBA
            _ => width * height,  // At least 1 byte per pixel for compressed formats
        };

        if data_size < expected_min_size as usize {
            return Err(XnbError::InvalidFormat(format!(
                "Data size too small: got {} bytes, expected at least {}",
                data_size, expected_min_size
            )));
        }

        // Read actual texture data
        let mut data = vec![0u8; data_size];
        self.reader.get_reader_mut().read_exact(&mut data)?;

        // Process the texture data based on format
        match format {
            TextureFormat::Color => self.decode_format_color(data, width, height),
            _ => Err(XnbError::UnsupportedFeature(format!("Texture format {:?} not supported yet", format))),
        }
    }

    /// Decode the Color format (32-bit RGBA)
    fn decode_format_color(&self, data: Vec<u8>, width: u32, height: u32) -> Result<RgbaImage> {
        if data.len() != (width * height * 4) as usize {
            return Err(XnbError::InvalidFormat(format!(
                "Invalid texture data size: got {} bytes, expected {}",
                data.len(), width * height * 4
            )));
        }

        let mut image = RgbaImage::new(width, height);

        // XNA stores color as ABGR (or sometimes BGRA), we need RGBA
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;

                // Safety check for buffer access
                if offset + 3 >= data.len() {
                    return Err(XnbError::InvalidFormat("Buffer overflow when reading pixel data".to_string()));
                }

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
    reader.read_texture().map_err(|e| e.into())
}