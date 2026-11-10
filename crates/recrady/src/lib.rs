//! File-only imaging command.
//!
//! This binary reads and writes regular files. Disk paths are rejected before
//! any file is opened, so it cannot be pointed at a live drive.

mod guard;

use std::path::Path;

use core_device::ImageFileDevice;
use core_imaging::{ImageOptions, ImageReport, image_raw};

pub use guard::CliError;

/// Copies a RAW image file to another file and returns the imaging report.
pub fn image_files(source: &Path, destination: &Path) -> Result<ImageReport, CliError> {
    guard::require_regular_file(source)?;
    guard::require_safe_destination(destination)?;
    let device = ImageFileDevice::open(source)?;
    Ok(image_raw(
        &device,
        destination,
        ImageOptions::new(Some(source)),
    )?)
}
