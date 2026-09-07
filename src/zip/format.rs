use std::fs::File;
use std::io::{Read, Seek};

const END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
pub(crate) const CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0201_4b50;
pub(crate) const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;

pub(crate) struct EndOfCentralDirectory {
    pub(crate) entry_count: usize,
    pub(crate) directory_size: usize,
    pub(crate) directory_offset: usize,
}

pub(crate) fn find_eocd(file: &mut File) -> Result<Option<EndOfCentralDirectory>, String> {
    let file_size = file
        .metadata()
        .map_err(|error| format!("failed to read jar metadata: {}", error))?
        .len();

    let tail_size = file_size.min(65_557);
    let mut tail = vec![0u8; tail_size as usize];
    file.seek(std::io::SeekFrom::End(-(tail_size as i64)))
        .map_err(io)?;
    file.read_exact(&mut tail).map_err(io)?;

    for index in (0..tail.len().saturating_sub(4)).rev() {
        if tail.u32_at(index) == END_OF_CENTRAL_DIRECTORY_SIGNATURE {
            return Ok(Some(EndOfCentralDirectory {
                entry_count: tail.u16_at(index + 10) as usize,
                directory_size: tail.u32_at(index + 12) as usize,
                directory_offset: tail.u32_at(index + 16) as usize,
            }));
        }
    }

    Ok(None)
}

pub(crate) fn io(error: std::io::Error) -> String {
    format!("I/O error: {}", error)
}

// Little-endian field readers.

pub(crate) trait LeField {
    fn u16_at(&self, offset: usize) -> u16;
    fn u32_at(&self, offset: usize) -> u32;
}

impl LeField for [u8] {
    fn u16_at(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self[offset], self[offset + 1]])
    }

    fn u32_at(&self, offset: usize) -> u32 {
        u32::from_le_bytes([
            self[offset],
            self[offset + 1],
            self[offset + 2],
            self[offset + 3],
        ])
    }
}