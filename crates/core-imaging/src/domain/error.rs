use std::io;
use std::sync::Arc;

use core_device::DeviceError;

/// Failures of an imaging run.
///
/// A bad sector is not an error. It is recorded on [`ImageReport`](crate::ImageReport)
/// and the destination gets zeros there. These variants abort the run.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ImageError {
    /// The destination path is the source file, including a hard link.
    #[error("destination is the source device; imaging would overwrite the evidence")]
    DestinationIsSource,

    /// The destination path is a directory.
    #[error("destination is a directory")]
    DestinationIsDirectory,

    /// `chunk_size` is zero or not a multiple of the device sector size.
    #[error("chunk size {chunk_size} is not a positive multiple of sector size {sector_size}")]
    InvalidChunkSize {
        /// Requested read size in bytes.
        chunk_size: u64,
        /// Device sector size in bytes.
        sector_size: u32,
    },

    /// The source reported a sector size of zero or not a power of two.
    #[error("invalid sector size {0}: must be a non-zero power of two")]
    InvalidSectorSize(u32),

    /// An offset calculation overflowed. This is a bug in the caller or the device size.
    #[error("offset overflow at {offset}")]
    RangeOverflow {
        /// Offset where the addition overflowed.
        offset: u64,
    },

    /// The source failed in a way that is not a bad sector (for example a range past its size).
    #[error(transparent)]
    Device(DeviceError),

    /// Writing the destination failed. A partial file may remain.
    #[error("destination I/O error at offset {offset}: {source}")]
    DestinationIo {
        /// Offset in the destination that failed, or 0 while opening it.
        offset: u64,
        /// Underlying OS error.
        #[source]
        source: Arc<io::Error>,
    },
}

impl ImageError {
    pub(crate) fn destination_io(offset: u64, source: io::Error) -> Self {
        Self::DestinationIo {
            offset,
            source: Arc::new(source),
        }
    }
}
