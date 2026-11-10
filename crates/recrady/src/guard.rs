use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use core_device::DeviceError;
use core_imaging::ImageError;
use fs_fat::FatError;

/// Why the file-only command refused to run.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// The arguments were not `image <source> <destination>`.
    #[error(
        "usage: recrady image <source.img> <destination.img>\n       recrady recover <source.img> <destination-directory>"
    )]
    Usage,

    /// The path is a disk or device namespace, not a file.
    #[error("refusing device path {}", .0.display())]
    DevicePath(PathBuf),

    /// The path exists but is not a regular file.
    #[error("not a regular file: {}", .0.display())]
    NotAFile(PathBuf),

    /// Recovery output has to be a directory.
    #[error("destination is not a directory: {}", .0.display())]
    NotADirectory(PathBuf),

    /// The destination path is the source image.
    #[error("destination is the source image")]
    DestinationIsSource,

    /// A recovered relative path was not a plain file name.
    #[error("refusing output path {}", .0.display())]
    UnsafeOutput(PathBuf),

    /// Reading file metadata failed.
    #[error("cannot inspect {}: {source}", path.display())]
    Io {
        /// Path that could not be inspected.
        path: PathBuf,
        /// Underlying OS error.
        #[source]
        source: io::Error,
    },

    /// Opening the source image failed.
    #[error(transparent)]
    Device(#[from] DeviceError),

    /// Imaging failed after the path checks.
    #[error(transparent)]
    Image(#[from] ImageError),

    /// Reading the FAT32 image failed.
    #[error(transparent)]
    Fat(#[from] FatError),
}

pub(crate) fn require_regular_file(path: &Path) -> Result<(), CliError> {
    reject_device_path(path)?;
    let meta = fs::metadata(path).map_err(|source| CliError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !meta.is_file() {
        return Err(CliError::NotAFile(path.to_path_buf()));
    }
    let canonical = path.canonicalize().map_err(|source| CliError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    reject_device_path(&canonical)
}

pub(crate) fn require_output_dir(source: &Path, destination: &Path) -> Result<(), CliError> {
    reject_device_path(destination)?;
    if same_path(source, destination).map_err(|source_error| CliError::Io {
        path: destination.to_path_buf(),
        source: source_error,
    })? {
        return Err(CliError::DestinationIsSource);
    }
    if destination.exists() {
        if !destination.is_dir() {
            return Err(CliError::NotADirectory(destination.to_path_buf()));
        }
        let canonical = destination.canonicalize().map_err(|source| CliError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
        return reject_device_path(&canonical);
    }
    if let Some(parent) = destination.parent()
        && parent.components().next().is_some()
        && parent.exists()
    {
        let canonical = parent.canonicalize().map_err(|source| CliError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        reject_device_path(&canonical)?;
    }
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> io::Result<bool> {
    if left.exists() && right.exists() {
        return Ok(left.canonicalize()? == right.canonicalize()?);
    }
    Ok(std::path::absolute(left)? == std::path::absolute(right)?)
}

pub(crate) fn require_safe_destination(path: &Path) -> Result<(), CliError> {
    reject_device_path(path)?;
    if path.exists() {
        return require_regular_file(path);
    }
    if let Some(parent) = path.parent()
        && parent.components().next().is_some()
        && parent.exists()
    {
        let canonical = parent.canonicalize().map_err(|source| CliError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        reject_device_path(&canonical)?;
    }
    Ok(())
}

fn reject_device_path(path: &Path) -> Result<(), CliError> {
    if is_device_path(path) {
        Err(CliError::DevicePath(path.to_path_buf()))
    } else {
        Ok(())
    }
}

pub(crate) fn is_device_path(path: &Path) -> bool {
    is_windows_device(path) || is_unix_dev(path)
}

fn is_windows_device(path: &Path) -> bool {
    let text = path.as_os_str().to_string_lossy();
    let slash: String = text
        .chars()
        .map(|ch| if ch == '/' { '\\' } else { ch })
        .collect();
    let lower = slash.to_ascii_lowercase();
    if lower.starts_with(r"\\.\") {
        return true;
    }
    if let Some(rest) = lower.strip_prefix(r"\\?\") {
        return rest.starts_with(r"unc\")
            || rest.starts_with("volume")
            || rest.starts_with("physicaldrive");
    }
    false
}

/// `/dev/...` only. `C:\dev\photo.img` starts with a drive prefix and is a file.
fn is_unix_dev(path: &Path) -> bool {
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return false;
    }
    matches!(components.next(), Some(Component::Normal(name)) if name == "dev")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_paths_are_refused() {
        assert!(is_device_path(Path::new("/dev/disk2")));
        assert!(is_device_path(Path::new("/dev/rdisk2")));
        assert!(is_device_path(Path::new(r"\\.\PhysicalDrive0")));
        assert!(is_device_path(Path::new(r"\\.\C:")));
        assert!(is_device_path(Path::new(r"\\?\Volume{abc}")));
    }

    #[test]
    fn ordinary_files_are_not_devices() {
        assert!(!is_device_path(Path::new("/tmp/flash.img")));
        assert!(!is_device_path(Path::new("flash.img")));
        assert!(!is_device_path(Path::new(r"C:\dev\photo.img")));
        assert!(!is_device_path(Path::new(r"\\?\C:\images\flash.img")));
        assert!(!is_device_path(Path::new("PhysicalDrive0.img")));
    }
}
