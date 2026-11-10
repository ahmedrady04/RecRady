//! Deleted FAT32 files come back only when their clusters are still free.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::Write;

use core_device::ImageFileDevice;
use fs_fat::{Fat32Volume, SkipReason};
use tempfile::NamedTempFile;

fn open_image(bytes: &[u8]) -> (NamedTempFile, ImageFileDevice) {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    file.flush().unwrap();
    let device = ImageFileDevice::open(file.path()).unwrap();
    (file, device)
}

#[test]
fn recovers_deleted_files_whose_clusters_are_free() {
    let image = fs_fat::fixture::deleted_files_image(true).unwrap();
    let (file, device) = open_image(&image);
    let before = fs::read(file.path()).unwrap();
    let volume = Fat32Volume::open(&device).unwrap();
    let recovery = volume.recover_deleted(&device).unwrap();

    let hello = recovery
        .files
        .iter()
        .find(|item| item.relative_path == "_ELLO.TXT")
        .unwrap();
    assert_eq!(hello.bytes, b"hello");
    let note = recovery
        .files
        .iter()
        .find(|item| item.relative_path == "SUBDIR/_OTE.TXT")
        .unwrap();
    assert_eq!(note.bytes, b"note");
    assert!(recovery.skipped.is_empty());
    assert_eq!(fs::read(file.path()).unwrap(), before);
}

#[test]
fn reused_cluster_is_not_returned_as_the_file() {
    let image = fs_fat::fixture::deleted_files_image(false).unwrap();
    let (_file, device) = open_image(&image);
    let volume = Fat32Volume::open(&device).unwrap();
    let recovery = volume.recover_deleted(&device).unwrap();
    assert!(
        recovery
            .files
            .iter()
            .all(|item| item.relative_path != "_ELLO.TXT")
    );
    assert!(recovery.skipped.iter().any(|item| {
        item.relative_path == "_ELLO.TXT" && item.reason == SkipReason::ClusterReused
    }));
    assert!(recovery.files.iter().any(|item| item.bytes == b"note"));
}

#[test]
fn gpt_disk_image_uses_the_partition() {
    let image = fs_fat::fixture::deleted_files_gpt_image(true).unwrap();
    let (_file, device) = open_image(&image);
    let volume = Fat32Volume::open(&device).unwrap();
    let recovery = volume.recover_deleted(&device).unwrap();
    assert!(recovery.files.iter().any(|item| item.bytes == b"hello"));
}

#[test]
fn zeros_are_not_a_filesystem() {
    let (_file, device) = open_image(&[0u8; 4096]);
    assert!(Fat32Volume::open(&device).is_err());
}
