use std::fs::File;
use std::io::{Read, Seek};
use std::path::PathBuf;

use miniz_oxide::inflate;

use format::{
    find_eocd, io, LeField, LOCAL_FILE_HEADER_SIGNATURE, CENTRAL_DIRECTORY_SIGNATURE,
};

mod format;

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

        let mut directory = vec![0u8; end.directory_size];
        file.seek(std::io::SeekFrom::Start(end.directory_offset as u64))
            .map_err(io)?;
        file.read_exact(&mut directory).map_err(io)?;

        let mut cursor = 0;
        for _ in 0..end.entry_count {
            if cursor + 46 > directory.len()
                || directory.u32_at(cursor) != CENTRAL_DIRECTORY_SIGNATURE
            {
                return Err(format!(
                    "invalid jar at {}: corrupt central directory",
                    self.path.display()
                ));
            }

            let name_length = directory.u16_at(cursor + 28) as usize;
            let extra_length = directory.u16_at(cursor + 30) as usize;
            let comment_length = directory.u16_at(cursor + 32) as usize;
            let name_end = cursor + 46 + name_length;
            let entry = ArchiveEntry {
                method: directory.u16_at(cursor + 10),
                compressed_size: directory.u32_at(cursor + 20),
                local_header_offset: directory.u32_at(cursor + 42),
            };

            if name_end <= directory.len() && &directory[cursor + 46..name_end] == name.as_bytes()
            {
                return Ok(Some(entry));
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
        file.seek(std::io::SeekFrom::Start(entry.local_header_offset as u64))
            .map_err(io)?;
        file.read_exact(&mut header).map_err(io)?;

        if header.u32_at(0) != LOCAL_FILE_HEADER_SIGNATURE {
            return Err(format!(
                "invalid jar at {}: corrupt local file header",
                self.path.display()
            ));
        }

        let data_offset = entry.local_header_offset as u64
            + 30
            + header.u16_at(26) as u64
            + header.u16_at(28) as u64;

        let mut compressed = vec![0u8; entry.compressed_size as usize];
        file.seek(std::io::SeekFrom::Start(data_offset)).map_err(io)?;
        file.read_exact(&mut compressed).map_err(io)?;

        match entry.method {
            0 => Ok(compressed),
            8 => inflate::decompress_to_vec(&compressed)
                .map_err(|error| format!("failed to decompress zip entry: {}", error)),
            other => Err(format!("unsupported zip compression method: {}", other)),
        }
    }
}