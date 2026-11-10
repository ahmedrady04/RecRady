//! Acceptance tests for RAW imaging.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::sync::Arc;

use core_device::{BlockDevice, DeviceError, ImageFileDevice};
use core_imaging::{ImageError, ImageOptions, image_raw};
use sha2::Digest;
use tempfile::NamedTempFile;

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn image_with(data: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    file.flush().unwrap();
    file
}

/// Reader that fails any read overlapping `bad`.
struct Flaky {
    data: Vec<u8>,
    bad: Range<u64>,
    sector_size: u32,
}

impl BlockDevice for Flaky {
    fn size(&self) -> u64 {
        self.data.len() as u64
    }

    fn sector_size(&self) -> u32 {
        self.sector_size
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        let len = u64::try_from(buf.len()).map_err(|_| DeviceError::OutOfBounds {
            offset,
            len: buf.len(),
            size: self.size(),
        })?;
        let Some(end) = offset.checked_add(len) else {
            return Err(DeviceError::OutOfBounds {
                offset,
                len: buf.len(),
                size: self.size(),
            });
        };
        if end > self.size() {
            return Err(DeviceError::OutOfBounds {
                offset,
                len: buf.len(),
                size: self.size(),
            });
        }
        if offset < self.bad.end && end > self.bad.start {
            return Err(DeviceError::Io {
                offset,
                source: Arc::new(io::Error::other("bad sector")),
            });
        }
        let start = usize::try_from(offset).unwrap();
        buf.copy_from_slice(&self.data[start..start + buf.len()]);
        Ok(())
    }
}

struct Broken;

impl BlockDevice for Broken {
    fn size(&self) -> u64 {
        64
    }

    fn sector_size(&self) -> u32 {
        512
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        Err(DeviceError::OutOfBounds {
            offset,
            len: buf.len(),
            size: self.size(),
        })
    }
}

#[test]
fn copies_a_raw_image_and_records_sha256() {
    let data = pattern(4096);
    let source_file = image_with(&data);
    let before = fs::metadata(source_file.path())
        .unwrap()
        .modified()
        .unwrap();
    let source = ImageFileDevice::open(source_file.path()).unwrap();
    let dest = NamedTempFile::new().unwrap();
    let dest_path = dest.path().to_path_buf();
    drop(dest);

    let report = image_raw(
        &source,
        &dest_path,
        ImageOptions::new(Some(source_file.path())),
    )
    .unwrap();

    assert_eq!(fs::read(&dest_path).unwrap(), data);
    assert_eq!(report.bytes_written, data.len() as u64);
    assert!(report.bad_ranges.is_empty());
    assert_eq!(report.sha256_hex().len(), 64);
    let digest = sha2::Sha256::digest(&data);
    assert_eq!(report.sha256.as_slice(), digest.as_slice());
    assert_eq!(fs::read(source_file.path()).unwrap(), data);
    assert_eq!(
        fs::metadata(source_file.path())
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
    let _ = fs::remove_file(&dest_path);
}

#[test]
fn bad_sector_is_zero_only_in_the_destination() {
    let mut data = pattern(4096);
    let bad = 1024..1536;
    let source_bytes = data.clone();
    data[bad.start..bad.end].fill(0);
    let device = Flaky {
        data: source_bytes.clone(),
        bad: bad.start as u64..bad.end as u64,
        sector_size: 512,
    };
    let dest = NamedTempFile::new().unwrap();
    let dest_path = dest.path().to_path_buf();
    drop(dest);

    let report = image_raw(&device, &dest_path, ImageOptions::new(None)).unwrap();

    assert_eq!(fs::read(&dest_path).unwrap(), data);
    assert_eq!(device.data, source_bytes);
    assert_eq!(report.bad_ranges.len(), 1);
    assert_eq!(report.bad_ranges[0].offset, 1024);
    assert_eq!(report.bad_ranges[0].len, 512);
    assert_eq!(report.bad_bytes(), 512);
    let digest = sha2::Sha256::digest(&data);
    assert_eq!(report.sha256.as_slice(), digest.as_slice());
    let _ = fs::remove_file(&dest_path);
}

#[test]
fn refuses_to_overwrite_the_source() {
    let data = pattern(1024);
    let source_file = image_with(&data);
    let source = ImageFileDevice::open(source_file.path()).unwrap();
    let error = image_raw(
        &source,
        source_file.path(),
        ImageOptions::new(Some(source_file.path())),
    )
    .unwrap_err();
    assert!(matches!(error, ImageError::DestinationIsSource));
    assert_eq!(fs::read(source_file.path()).unwrap(), data);
}

#[cfg(unix)]
#[test]
fn refuses_a_hard_link_of_the_source() {
    let data = pattern(512);
    let source_file = image_with(&data);
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("same-file.img");
    fs::hard_link(source_file.path(), &link).unwrap();
    let source = ImageFileDevice::open(source_file.path()).unwrap();
    let error = image_raw(&source, &link, ImageOptions::new(Some(source_file.path()))).unwrap_err();
    assert!(matches!(error, ImageError::DestinationIsSource));
    assert_eq!(fs::read(source_file.path()).unwrap(), data);
}

#[test]
fn empty_image_has_the_empty_sha256() {
    let source_file = image_with(&[]);
    let source = ImageFileDevice::open(source_file.path()).unwrap();
    let dest = NamedTempFile::new().unwrap();
    let dest_path = dest.path().to_path_buf();
    drop(dest);
    let report = image_raw(
        &source,
        &dest_path,
        ImageOptions::new(Some(source_file.path())),
    )
    .unwrap();
    assert_eq!(report.bytes_written, 0);
    assert_eq!(
        report.sha256_hex(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(fs::read(&dest_path).unwrap(), b"");
    let _ = fs::remove_file(&dest_path);
}

#[test]
fn out_of_bounds_does_not_leave_a_destination() {
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("partial.img");
    let error = image_raw(&Broken, &dest, ImageOptions::new(None)).unwrap_err();
    assert!(matches!(
        error,
        ImageError::Device(DeviceError::OutOfBounds { .. })
    ));
    assert!(!dest.exists());
}

#[test]
fn invalid_chunk_size_writes_nothing() {
    let data = pattern(512);
    let source_file = image_with(&data);
    let source = ImageFileDevice::open(source_file.path()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("nope.img");
    let error = image_raw(
        &source,
        &dest,
        ImageOptions::new(Some(source_file.path())).with_chunk_size(100),
    )
    .unwrap_err();
    assert!(matches!(error, ImageError::InvalidChunkSize { .. }));
    assert!(!dest.exists());
    assert_eq!(fs::read(source_file.path()).unwrap(), data);
}

#[test]
fn two_bad_sectors_merge() {
    let data = pattern(2048);
    let device = Flaky {
        data,
        bad: 512..1536,
        sector_size: 512,
    };
    let dest = NamedTempFile::new().unwrap();
    let dest_path = dest.path().to_path_buf();
    drop(dest);
    let report = image_raw(&device, &dest_path, ImageOptions::new(None)).unwrap();
    assert_eq!(report.bad_ranges.len(), 1);
    assert_eq!(report.bad_ranges[0].offset, 512);
    assert_eq!(report.bad_ranges[0].len, 1024);
    let _ = fs::remove_file(&dest_path);
}
