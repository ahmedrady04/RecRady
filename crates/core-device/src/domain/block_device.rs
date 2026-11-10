use super::error::DeviceError;

/// Sector size assumed when the source does not report one.
pub const DEFAULT_SECTOR_SIZE: u32 = 512;

/// A read-only, random-access view of a storage device or disk image.
///
/// Implementations must be safe to share between threads: `read_at` takes
/// `&self` and an explicit offset, so concurrent readers never race on a
/// shared file cursor.
pub trait BlockDevice: Send + Sync {
    /// Total size of the device in bytes.
    fn size(&self) -> u64;

    /// Logical sector size in bytes. Always a non-zero power of two.
    fn sector_size(&self) -> u32;

    /// Fills `buf` with the bytes starting at `offset`.
    ///
    /// Either the whole buffer is filled or an error is returned; there are no
    /// short reads. A range that extends past [`size`](Self::size) fails with
    /// [`DeviceError::OutOfBounds`]. A zero-length read succeeds for any
    /// `offset <= size`.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError>;
}

/// Validates that `[offset, offset + len)` lies within a device of `size` bytes.
pub(crate) fn check_bounds(offset: u64, len: usize, size: u64) -> Result<(), DeviceError> {
    let out_of_bounds = DeviceError::OutOfBounds { offset, len, size };
    let len_u64 = u64::try_from(len).map_err(|_| out_of_bounds.clone())?;
    match offset.checked_add(len_u64) {
        Some(end) if end <= size => Ok(()),
        _ => Err(out_of_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_accept_exact_fit_and_empty_read_at_end() {
        assert!(check_bounds(0, 10, 10).is_ok());
        assert!(check_bounds(10, 0, 10).is_ok());
    }

    #[test]
    fn bounds_reject_overflow_and_past_end() {
        assert!(check_bounds(u64::MAX, 1, 10).is_err());
        assert!(check_bounds(5, 6, 10).is_err());
        assert!(check_bounds(11, 0, 10).is_err());
    }
}
