//! The command copies regular files and refuses devices and directories.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::Write;
use std::path::Path;

use recrady::{CliError, image_files};
use tempfile::NamedTempFile;

#[test]
fn copies_a_regular_file() {
    let mut source = NamedTempFile::new().unwrap();
    source.write_all(b"raw-image-bytes").unwrap();
    source.flush().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("copy.img");

    let report = image_files(source.path(), &destination).unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"raw-image-bytes");
    assert_eq!(report.bytes_written, 15);
    assert!(report.bad_ranges.is_empty());
    assert_eq!(report.sha256_hex().len(), 64);
    assert_eq!(fs::read(source.path()).unwrap(), b"raw-image-bytes");
}

#[test]
fn refuses_disk_paths_without_opening_them() {
    for path in ["/dev/disk2", "/dev/rdisk0"] {
        let error = image_files(
            Path::new(path),
            Path::new("/tmp/recrady-should-not-create.img"),
        )
        .unwrap_err();
        assert!(matches!(error, CliError::DevicePath(_)), "{path}: {error}");
    }
    assert!(!Path::new("/tmp/recrady-should-not-create.img").exists());
}

#[test]
fn refuses_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("copy.img");
    let error = image_files(dir.path(), &destination).unwrap_err();
    assert!(matches!(error, CliError::NotAFile(_)));
    assert!(!destination.exists());
}

#[test]
fn refuses_to_overwrite_the_source_file() {
    let mut source = NamedTempFile::new().unwrap();
    source.write_all(b"keep").unwrap();
    source.flush().unwrap();
    let error = image_files(source.path(), source.path()).unwrap_err();
    assert!(matches!(error, CliError::Image(_)));
    assert_eq!(fs::read(source.path()).unwrap(), b"keep");
}
