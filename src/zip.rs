use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use miniz_oxide::inflate;

const END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;

pub struct ZipArchive {
    path: PathBuf,
}

pub struct ArchiveEntry {
    pub(crate) method: u16,
    pub(crate) compressed_size: u32,
    pub(crate) local_header_offset: u32,
}

impl ZipArchive {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Locates an entry by its exact central-directory name, e.g.
    /// `META-INF/MANIFEST.MF`.
    pub fn entry(&self, name: &str) -> Result<Option<ArchiveEntry>, String> {
        let mut file = File::open(&self.path)
            .map_err(|error| format!("failed to open jar at {}: {}", self.path.display(), error))?;

        let end = find_eocd(&mut file)?.ok_or_else(|| {
            format!(
                "invalid jar at {}: end of central directory record not found",
                self.path.display()
            )
        })?;

        let count = entry_count(&end);
        let size = directory_size(&end);
        let offset = directory_offset(&end);

        let mut directory = vec![0u8; size];
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(io_err)?;
        file.read_exact(&mut directory).map_err(io_err)?;

        let mut cursor = 0;
        for _ in 0..count {
            if cursor + 46 > directory.len()
                || read_u32(&directory, cursor) != CENTRAL_DIRECTORY_SIGNATURE
            {
                return Err(format!(
                    "invalid jar at {}: corrupt central directory",
                    self.path.display()
                ));
            }

            let method = read_u16(&directory, cursor + 10);
            let compressed_size = read_u32(&directory, cursor + 20);
            let name_length = read_u16(&directory, cursor + 28) as usize;
            let extra_length = read_u16(&directory, cursor + 30) as usize;
            let comment_length = read_u16(&directory, cursor + 32) as usize;
            let local_header_offset = read_u32(&directory, cursor + 42);

            let name_end = cursor + 46 + name_length;
            if name_end <= directory.len()
                && &directory[cursor + 46..name_end] == name.as_bytes()
            {
                return Ok(Some(ArchiveEntry {
                    method,
                    compressed_size,
                    local_header_offset,
                }));
            }

            cursor = name_end + extra_length + comment_length;
        }

        Ok(None)
    }

    /// Reads and decompresses the raw contents of an entry.
    pub fn entry_contents(&self, entry: &ArchiveEntry) -> Result<Vec<u8>, String> {
        let mut file = File::open(&self.path)
            .map_err(|error| format!("failed to open jar at {}: {}", self.path.display(), error))?;

        let mut header = [0u8; 30];
        file.seek(SeekFrom::Start(entry.local_header_offset as u64))
            .map_err(io_err)?;
        file.read_exact(&mut header).map_err(io_err)?;

        if read_u32(&header, 0) != LOCAL_FILE_HEADER_SIGNATURE {
            return Err(format!(
                "invalid jar at {}: corrupt local file header",
                self.path.display()
            ));
        }

        let name_length = read_u16(&header, 26) as u64;
        let extra_length = read_u16(&header, 28) as u64;
        let data_offset = entry.local_header_offset as u64 + 30 + name_length + extra_length;

        let mut compressed = vec![0u8; entry.compressed_size as usize];
        file.seek(SeekFrom::Start(data_offset)).map_err(io_err)?;
        file.read_exact(&mut compressed).map_err(io_err)?;

        match entry.method {
            0 => Ok(compressed),
            8 => inflate::decompress_to_vec(&compressed)
                .map_err(|error| format!("failed to decompress zip entry: {}", error)),
            other => Err(format!("unsupported zip compression method: {}", other)),
        }
    }
}

struct EndOfCentralDirectory {
    record: Vec<u8>,
}

fn find_eocd(file: &mut File) -> Result<Option<EndOfCentralDirectory>, String> {
    let file_size = file
        .metadata()
        .map_err(|error| format!("failed to read jar metadata: {}", error))?
        .len();

    let tail_size = file_size.min(65_557);
    let mut tail = vec![0u8; tail_size as usize];
    file.seek(SeekFrom::End(-(tail_size as i64)))
        .map_err(io_err)?;
    file.read_exact(&mut tail).map_err(io_err)?;

    for index in (0..tail.len().saturating_sub(4)).rev() {
        if read_u32(&tail, index) == END_OF_CENTRAL_DIRECTORY_SIGNATURE {
            return Ok(Some(EndOfCentralDirectory {
                record: tail[index..index + 22].to_vec(),
            }));
        }
    }

    Ok(None)
}

fn entry_count(end: &EndOfCentralDirectory) -> usize {
    read_u16(&end.record, 10) as usize
}

fn directory_size(end: &EndOfCentralDirectory) -> usize {
    read_u32(&end.record, 12) as usize
}

fn directory_offset(end: &EndOfCentralDirectory) -> usize {
    read_u32(&end.record, 16) as usize
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn io_err(error: std::io::Error) -> String {
    format!("I/O error: {}", error)
}