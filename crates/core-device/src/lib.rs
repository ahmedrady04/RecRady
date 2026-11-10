//! Read-only access to block devices and disk images.
//!
//! The [`BlockDevice`] trait is the only way the rest of RecRady touches
//! storage. It deliberately has no write methods, so code built on top of it
//! cannot modify a source device.
//!
//! Layout follows Clean Architecture:
//! - `domain`: the port ([`BlockDevice`]) and its error type. No OS or I/O details.
//! - `infrastructure`: adapters that implement the port ([`ImageFileDevice`]).

mod domain;
mod infrastructure;

pub use domain::block_device::{BlockDevice, DEFAULT_SECTOR_SIZE};
pub use domain::error::DeviceError;
pub use infrastructure::image_file::ImageFileDevice;
