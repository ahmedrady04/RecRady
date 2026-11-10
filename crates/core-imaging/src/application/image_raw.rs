use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::domain::error::ImageError;
use crate::domain::report::{self, BadRange, ImageReport};
use crate::infrastructure::destination;
use core_device::{BlockDevice, DeviceError};

/// Default read size. A multiple of both 512 and 4096.
pub const DEFAULT_CHUNK_SIZE: u64 = 1024 * 1024;

/// Inputs for one imaging run, other than the destination path.
pub struct ImageOptions<'a> {
    /// Path of a file-backed source. Required to refuse imaging a file onto itself.
    pub source_path: Option<&'a Path>,
    /// Maximum bytes per read. Must be a positive multiple of the sector size.
    pub chunk_size: u64,
}

impl<'a> ImageOptions<'a> {
    /// Options with [`DEFAULT_CHUNK_SIZE`].
    pub fn new(source_path: Option<&'a Path>) -> Self {
        Self {
            source_path,
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }

    /// Overrides the read size. Validated when imaging starts.
    pub fn with_chunk_size(mut self, chunk_size: u64) -> Self {
        self.chunk_size = chunk_size;
        self
    }
}

/// Copies `source` to a new RAW file at `destination`.
///
/// Unreadable sectors are zeros in the destination. The source is only read.
/// On a hard error before any byte is written, the destination file is removed.
/// A hard error after that leaves the partial file in place.
pub fn image_raw(
    source: &dyn BlockDevice,
    destination: &Path,
    options: ImageOptions<'_>,
) -> Result<ImageReport, ImageError> {
    let sector_size = source.sector_size();
    if !sector_size.is_power_of_two() {
        return Err(ImageError::InvalidSectorSize(sector_size));
    }
    let sector = u64::from(sector_size);
    if options.chunk_size == 0 || options.chunk_size % sector != 0 {
        return Err(ImageError::InvalidChunkSize {
            chunk_size: options.chunk_size,
            sector_size,
        });
    }
    if let Some(source_path) = options.source_path
        && destination::paths_conflict(source_path, destination)
            .map_err(|error| ImageError::destination_io(0, error))?
    {
        return Err(ImageError::DestinationIsSource);
    }
    if destination.is_dir() {
        return Err(ImageError::DestinationIsDirectory);
    }

    let chunk_usize =
        usize::try_from(options.chunk_size).map_err(|_| ImageError::InvalidChunkSize {
            chunk_size: options.chunk_size,
            sector_size,
        })?;
    let mut file =
        File::create(destination).map_err(|error| ImageError::destination_io(0, error))?;
    let mut buf = vec![0u8; chunk_usize];
    let mut hasher = Sha256::new();
    let mut bad_ranges = Vec::new();
    let mut offset = 0u64;
    let size = source.size();

    let copy_result = (|| {
        while offset < size {
            let len = options.chunk_size.min(size - offset);
            let len_usize =
                usize::try_from(len).map_err(|_| ImageError::RangeOverflow { offset })?;
            let Some(window) = buf.get_mut(..len_usize) else {
                return Err(ImageError::RangeOverflow { offset });
            };
            write_chunk(
                source,
                &mut file,
                &mut hasher,
                &mut bad_ranges,
                offset,
                window,
                sector,
            )?;
            offset = offset
                .checked_add(len)
                .ok_or(ImageError::RangeOverflow { offset })?;
        }
        file.flush()
            .map_err(|error| ImageError::destination_io(offset, error))?;
        file.sync_all()
            .map_err(|error| ImageError::destination_io(offset, error))?;
        Ok(())
    })();

    if copy_result.is_err() && offset == 0 {
        drop(file);
        let _ = fs::remove_file(destination);
    }
    copy_result?;

    let sha256 = hasher.finalize().into();
    Ok(ImageReport {
        bytes_written: size,
        bad_ranges,
        sha256,
    })
}

/// Reads one window. A failed read is retried one sector at a time so a single
/// bad sector does not zero the rest of the window.
fn write_chunk(
    source: &dyn BlockDevice,
    file: &mut File,
    hasher: &mut Sha256,
    bad_ranges: &mut Vec<BadRange>,
    offset: u64,
    window: &mut [u8],
    sector: u64,
) -> Result<(), ImageError> {
    match source.read_at(offset, window) {
        Ok(()) => write_bytes(file, hasher, offset, window),
        Err(DeviceError::Io { .. }) => {
            write_by_sector(source, file, hasher, bad_ranges, offset, window, sector)
        }
        Err(error) => Err(ImageError::Device(error)),
    }
}

fn write_by_sector(
    source: &dyn BlockDevice,
    file: &mut File,
    hasher: &mut Sha256,
    bad_ranges: &mut Vec<BadRange>,
    offset: u64,
    window: &mut [u8],
    sector: u64,
) -> Result<(), ImageError> {
    let mut pos = 0usize;
    while pos < window.len() {
        let piece = sector.min(u64::try_from(window.len() - pos).unwrap_or(sector));
        let piece_usize =
            usize::try_from(piece).map_err(|_| ImageError::RangeOverflow { offset })?;
        let Some(next) = pos.checked_add(piece_usize) else {
            return Err(ImageError::RangeOverflow { offset });
        };
        let Some(slice) = window.get_mut(pos..next) else {
            return Err(ImageError::RangeOverflow { offset });
        };
        let pos_u64 = u64::try_from(pos).map_err(|_| ImageError::RangeOverflow { offset })?;
        let at = offset
            .checked_add(pos_u64)
            .ok_or(ImageError::RangeOverflow { offset })?;
        match source.read_at(at, slice) {
            Ok(()) => write_bytes(file, hasher, at, slice)?,
            Err(DeviceError::Io { .. }) => {
                slice.fill(0);
                report::note_bad(bad_ranges, at, piece);
                write_bytes(file, hasher, at, slice)?;
            }
            Err(error) => return Err(ImageError::Device(error)),
        }
        pos = next;
    }
    Ok(())
}

fn write_bytes(
    file: &mut File,
    hasher: &mut Sha256,
    offset: u64,
    bytes: &[u8],
) -> Result<(), ImageError> {
    file.write_all(bytes)
        .map_err(|error| ImageError::destination_io(offset, error))?;
    hasher.update(bytes);
    Ok(())
}
