use std::io;
use std::sync::Arc;

/// Errors returned by [`BlockDevice`](crate::BlockDevice) implementations.
///
/// `Clone` is cheap (the I/O source is reference-counted) so errors can be
/// recorded in scan reports without losing the original cause.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum DeviceError {
    /// The requested range is not fully inside the device.
    #[error("read of {len} bytes at offset {offset} is out of bounds (device size {size})")]
    OutOfBounds {
        /// Requested start offset.
        offset: u64,
        /// Requested length.
        len: usize,
        /// Device size in bytes.
        size: u64,
    },

    /// The underlying OS read failed.
    #[error("I/O error at offset {offset}: {source}")]
    Io {
        /// Offset of the failed read, or 0 when opening the device.
        offset: u64,
        /// Underlying OS error.
        #[source]
        source: Arc<io::Error>,
    },

    /// The sector size is zero or not a power of two.
    #[error("invalid sector size {0}: must be a non-zero power of two")]
    InvalidSectorSize(u32),
}

impl DeviceError {
    /// Wraps an OS error with the offset of the failed operation.
    pub(crate) fn io(offset: u64, source: io::Error) -> Self {
        Self::Io {
            offset,
            source: Arc::new(source),
        }
    }
}
