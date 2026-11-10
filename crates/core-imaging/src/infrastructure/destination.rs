//! Destination file guard.
//!
//! Imaging must refuse to open a destination that is the source, including
//! when the two paths are different names for the same file (hard link).

use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};

/// True when writing `dest` would modify the file at `source`.
pub(crate) fn paths_conflict(source: &Path, dest: &Path) -> io::Result<bool> {
    if source.exists() && dest.exists() {
        let source_meta = source.metadata()?;
        let dest_meta = dest.metadata()?;
        if same_file(&source_meta, &dest_meta) {
            return Ok(true);
        }
    }
    Ok(normalize(source)? == normalize(dest)?)
}

fn normalize(path: &Path) -> io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(canonical) => Ok(canonical),
        Err(error) if error.kind() == io::ErrorKind::NotFound => std::path::absolute(path),
        Err(error) => Err(error),
    }
}

fn same_file(source: &Metadata, dest: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // Catches a hard link whose path differs from the source.
        source.dev() == dest.dev() && source.ino() == dest.ino()
    }
    #[cfg(not(unix))]
    {
        // `MetadataExt::file_index` is still unstable on Windows (rustc 1.96).
        // Canonical paths still stop a destination that names the source directly.
        let _ = (source, dest);
        false
    }
}
