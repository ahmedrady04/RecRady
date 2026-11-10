//! FAT32 deleted-file recovery.
//!
//! Field offsets follow the Microsoft FAT specification (August 30, 2005):
//! the BPB in section 3, FAT32 entries in section 4, and directory entries in
//! section 6. The filesystem type comes from the cluster count in section 3.5,
//! not from the informational `BS_FilSysType` string.
//!
//! Deleting a file stores `0xE5` in `DIR_Name[0]` and frees the cluster chain.
//! The chain itself is gone, so a recovered file is the free clusters that
//! follow the first cluster stored in the directory entry.

mod error;
pub mod fixture;
mod name;
mod recover;
mod volume;

pub use error::FatError;
pub use recover::{RecoveredFile, Recovery, SkipReason, SkippedFile};
pub use volume::Fat32Volume;
