//! Integration tests for `ImageFileDevice` against generated RAW images.
//!
//! Fixture helpers live outside `#[test]` functions, so the workspace rule that
//! allows unwrap only inside test functions does not cover them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::thread;

use core_device::{BlockDevice, DEFAULT_SECTOR_SIZE, DeviceError, ImageFileDevice};
use tempfile::NamedTempFile;

/// Deterministic pattern so every offset has a predictable, non-zero-heavy byte.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn image_with(data: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(data).unwrap();
    file.flush().unwrap();
    file
}

fn open(file: &NamedTempFile) -> ImageFileDevice {
    ImageFileDevice::open(file.path()).unwrap()
}

#[test]
fn reports_size_and_default_sector_size() {
    let img = image_with(&pattern(4096));
    let dev = open(&img);
    assert_eq!(dev.size(), 4096);
    assert_eq!(dev.sector_size(), DEFAULT_SECTOR_SIZE);
}

#[test]
fn reads_whole_image() {
    let data = pattern(2048);
    let img = image_with(&data);
    let mut buf = vec![0u8; data.len()];
    open(&img).read_at(0, &mut buf).unwrap();
    assert_eq!(buf, data);
}

#[test]
fn reads_unaligned_offset_and_length() {
    let data = pattern(1024);
    let img = image_with(&data);
    let mut buf = [0u8; 7];
    open(&img).read_at(3, &mut buf).unwrap();
    assert_eq!(&buf, &data[3..10]);
}

#[test]
fn reads_last_byte() {
    let data = pattern(1000);
    let img = image_with(&data);
    let mut buf = [0u8; 1];
    open(&img).read_at(999, &mut buf).unwrap();
    assert_eq!(buf[0], data[999]);
}

#[test]
fn read_crossing_end_is_out_of_bounds() {
    let img = image_with(&pattern(100));
    let mut buf = [0u8; 10];
    let err = open(&img).read_at(95, &mut buf).unwrap_err();
    assert!(matches!(
        err,
        DeviceError::OutOfBounds {
            offset: 95,
            len: 10,
            size: 100
        }
    ));
}

#[test]
fn read_starting_past_end_is_out_of_bounds() {
    let img = image_with(&pattern(100));
    let mut buf = [0u8; 1];
    let err = open(&img).read_at(200, &mut buf).unwrap_err();
    assert!(matches!(err, DeviceError::OutOfBounds { .. }));
}

#[test]
fn zero_length_read_inside_and_at_end_succeeds() {
    let img = image_with(&pattern(100));
    let dev = open(&img);
    dev.read_at(0, &mut []).unwrap();
    dev.read_at(50, &mut []).unwrap();
    dev.read_at(100, &mut []).unwrap();
}

#[test]
fn zero_length_read_past_end_is_out_of_bounds() {
    let img = image_with(&pattern(100));
    let err = open(&img).read_at(101, &mut []).unwrap_err();
    assert!(matches!(err, DeviceError::OutOfBounds { .. }));
}

#[test]
fn huge_offset_does_not_overflow() {
    let img = image_with(&pattern(100));
    let mut buf = [0u8; 16];
    let err = open(&img).read_at(u64::MAX, &mut buf).unwrap_err();
    assert!(matches!(err, DeviceError::OutOfBounds { .. }));
}

#[test]
fn empty_image_has_zero_size() {
    let img = image_with(&[]);
    let dev = open(&img);
    assert_eq!(dev.size(), 0);
    dev.read_at(0, &mut []).unwrap();
    let err = dev.read_at(0, &mut [0u8; 1]).unwrap_err();
    assert!(matches!(err, DeviceError::OutOfBounds { .. }));
}

#[test]
fn missing_file_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = ImageFileDevice::open(dir.path().join("missing.img")).unwrap_err();
    assert!(matches!(err, DeviceError::Io { .. }));
}

#[test]
fn sector_size_must_be_power_of_two() {
    let img = image_with(&pattern(16));
    for bad in [0, 3, 1000] {
        let err = open(&img).with_sector_size(bad).unwrap_err();
        assert!(matches!(err, DeviceError::InvalidSectorSize(s) if s == bad));
    }
    assert_eq!(
        open(&img).with_sector_size(4096).unwrap().sector_size(),
        4096
    );
}

#[test]
fn concurrent_reads_return_correct_data() {
    const CHUNK: usize = 512;
    const THREADS: usize = 8;
    let data = pattern(CHUNK * THREADS * 4);
    let img = image_with(&data);
    let dev = Arc::new(open(&img));
    let expected = Arc::new(data);

    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let dev = Arc::clone(&dev);
            let expected = Arc::clone(&expected);
            thread::spawn(move || {
                let mut buf = [0u8; CHUNK];
                for round in 0..100 {
                    let offset = ((t + round * THREADS) % (expected.len() / CHUNK)) * CHUNK;
                    dev.read_at(offset as u64, &mut buf).unwrap();
                    assert_eq!(&buf[..], &expected[offset..offset + CHUNK]);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn reading_never_modifies_the_source() {
    let data = pattern(8192);
    let img = image_with(&data);
    let before = fs::metadata(img.path()).unwrap().modified().unwrap();

    let dev = open(&img);
    let mut buf = [0u8; 512];
    for offset in (0..data.len() as u64).step_by(512) {
        dev.read_at(offset, &mut buf).unwrap();
    }
    drop(dev);

    assert_eq!(fs::read(img.path()).unwrap(), data);
    assert_eq!(
        fs::metadata(img.path()).unwrap().modified().unwrap(),
        before
    );
}
