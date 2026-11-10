//! Offset-based reads that never touch a shared file cursor.
//!
//! This is the only module in the crate with OS-specific code.

use std::fs::File;
use std::io;

/// Reads exactly `buf.len()` bytes from `file` starting at `offset`.
#[cfg(unix)]
pub fn read_exact_at(file: &File, offset: u64, buf: &mut [u8]) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buf, offset)
}

/// Reads exactly `buf.len()` bytes from `file` starting at `offset`.
///
/// `seek_read` may return fewer bytes than requested, so it is looped.
#[cfg(windows)]
pub fn read_exact_at(file: &File, offset: u64, buf: &mut [u8]) -> io::Result<()> {
    use std::os::windows::fs::FileExt;

    let mut done = 0usize;
    while let Some(rest) = buf.get_mut(done..) {
        if rest.is_empty() {
            break;
        }
        let Some(pos) = offset.checked_add(done as u64) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "read offset overflow",
            ));
        };
        match file.seek_read(rest, pos) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => done += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
