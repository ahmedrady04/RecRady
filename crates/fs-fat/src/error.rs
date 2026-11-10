use core_device::DeviceError;

/// A FAT image could not be read or is not a FAT32 volume.
#[derive(Debug, thiserror::Error)]
pub enum FatError {
    /// The source device rejected a read.
    #[error(transparent)]
    Device(#[from] DeviceError),

    /// No FAT32 boot sector was found at the start of the image or in a partition.
    #[error("no FAT32 filesystem found")]
    NotFat32,

    /// A multi-byte field ran past the buffer.
    #[error("truncated FAT structure")]
    Truncated,

    /// A BPB field is outside the values the FAT specification allows.
    #[error("invalid FAT32 boot sector")]
    InvalidBoot,

    /// A directory chain repeated a cluster.
    #[error("directory cluster loop")]
    ClusterLoop,

    /// More directory entries were seen than the scan limit.
    #[error("directory scan limit reached")]
    ScanLimit,
}
