use std::fs::File;
use std::path::{Path, PathBuf};

use super::positional_read;
use crate::domain::block_device::{BlockDevice, DEFAULT_SECTOR_SIZE, check_bounds};
use crate::domain::error::DeviceError;

/// A [`BlockDevice`] backed by a RAW (`dd`-style) disk image file.
///
/// The file is opened read-only and its size is captured once at open time,
/// so the device presents a stable view even if the file changes later.
#[derive(Debug)]
pub struct ImageFileDevice {
    file: File,
    path: PathBuf,
    size: u64,
    sector_size: u32,
}

impl ImageFileDevice {
    /// Opens a RAW image read-only with the default 512-byte sector size.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DeviceError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| DeviceError::io(0, e))?;
        let size = file.metadata().map_err(|e| DeviceError::io(0, e))?.len();
        Ok(Self {
            file,
            path: path.to_path_buf(),
            size,
            sector_size: DEFAULT_SECTOR_SIZE,
        })
    }

    /// Overrides the logical sector size (e.g. 4096 for 4Kn images).
    pub fn with_sector_size(mut self, sector_size: u32) -> Result<Self, DeviceError> {
        if !sector_size.is_power_of_two() {
            return Err(DeviceError::InvalidSectorSize(sector_size));
        }
        self.sector_size = sector_size;
        Ok(self)
    }

    /// Path the image was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl BlockDevice for ImageFileDevice {
    fn size(&self) -> u64 {
        self.size
    }

    fn sector_size(&self) -> u32 {
        self.sector_size
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        check_bounds(offset, buf.len(), self.size)?;
        if buf.is_empty() {
            return Ok(());
        }
        positional_read::read_exact_at(&self.file, offset, buf)
            .map_err(|e| DeviceError::io(offset, e))
    }
}
