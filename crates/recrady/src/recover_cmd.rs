use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};

use core_device::ImageFileDevice;
use fs_fat::{Fat32Volume, RecoveredFile, Recovery};

use crate::guard::{self, CliError};

pub(crate) fn recover_files(source: &Path, destination: &Path) -> Result<Recovery, CliError> {
    guard::require_regular_file(source)?;
    guard::require_output_dir(source, destination)?;
    if !destination.exists() {
        fs::create_dir_all(destination).map_err(|error| CliError::Io {
            path: destination.to_path_buf(),
            source: error,
        })?;
    }
    let device = ImageFileDevice::open(source)?;
    let volume = Fat32Volume::open(&device)?;
    let recovery = volume.recover_deleted(&device)?;
    for file in &recovery.files {
        write_file(destination, file)?;
    }
    Ok(recovery)
}

fn write_file(root: &Path, file: &RecoveredFile) -> Result<(), CliError> {
    let relative = Path::new(&file.relative_path);
    let safe = relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !safe || relative.as_os_str().is_empty() {
        return Err(CliError::UnsafeOutput(relative.to_path_buf()));
    }
    let path = root.join(relative);
    if let Some(parent) = path.parent()
        && parent != root
    {
        fs::create_dir_all(parent).map_err(|error| CliError::Io {
            path: parent.to_path_buf(),
            source: error,
        })?;
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| CliError::Io {
            path: path.clone(),
            source: error,
        })?;
    output
        .write_all(&file.bytes)
        .map_err(|error| CliError::Io {
            path,
            source: error,
        })?;
    Ok(())
}
