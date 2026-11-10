//! RAW disk imaging.
//!
//! [`image_raw`](application::image_raw) reads a [`BlockDevice`] and writes a
//! different destination file. The source trait has no write method, and the
//! destination path is rejected when it is the source file.
//!
//! Layout:
//! - `domain`: the report and errors. No file I/O.
//! - `application`: the imaging use case.
//! - `infrastructure`: the destination file and the same-path guard.

mod application;
mod domain;
mod infrastructure;

pub use application::image_raw::{DEFAULT_CHUNK_SIZE, ImageOptions, image_raw};
pub use core_device::{BlockDevice, DeviceError};
pub use domain::error::ImageError;
pub use domain::report::{BadRange, ImageReport};
