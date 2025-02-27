// src/binary_reader.rs
use std::io::{self, Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};

/// A helper for reading Celeste's binary formats
pub struct BinaryReader<R: Read + Seek> {
    pub reader: R,  // Making reader public to fix access errors
}

impl<R: Read + Seek> BinaryReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Read a signed byte
    pub fn read_byte(&mut self) -> io::Result<i8> {
        self.reader.read_i8()
    }

    /// Read an unsigned byte
    pub fn read_ubyte(&mut self) -> io::Result<u8> {
        self.reader.read_u8()
    }

    /// Read a signed short (16-bit integer)
    pub fn read_short(&mut self) -> io::Result<i16> {
        self.reader.read_i16::<LittleEndian>()
    }

    /// Read an unsigned short (16-bit integer)
    pub fn read_ushort(&mut self) -> io::Result<u16> {
        self.reader.read_u16::<LittleEndian>()
    }

    /// Read a signed long (32-bit integer)
    pub fn read_long(&mut self) -> io::Result<i32> {
        self.reader.read_i32::<LittleEndian>()
    }

    /// Read an unsigned long (32-bit integer)
    pub fn read_ulong(&mut self) -> io::Result<u32> {
        self.reader.read_u32::<LittleEndian>()
    }

    /// Read a boolean (1 byte)
    pub fn read_bool(&mut self) -> io::Result<bool> {
        Ok(self.reader.read_u8()? != 0)
    }

    /// Read a string prefixed with a length byte
    pub fn read_string(&mut self) -> io::Result<String> {
        let length = self.reader.read_u8()? as usize;
        let mut buffer = vec![0u8; length];
        self.reader.read_exact(&mut buffer)?;
        
        String::from_utf8(buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Skip a number of bytes
    pub fn skip(&mut self, bytes: u64) -> io::Result<()> {
        self.reader.seek(SeekFrom::Current(bytes as i64))?;
        Ok(())
    }

    /// Get the current position in the file
    pub fn position(&mut self) -> io::Result<u64> {
        self.reader.seek(SeekFrom::Current(0))
    }

    /// Set the current position in the file
    pub fn set_position(&mut self, pos: u64) -> io::Result<()> {
        self.reader.seek(SeekFrom::Start(pos))?;
        Ok(())
    }
}