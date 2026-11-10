//! `recover` writes deleted FAT32 files into another directory.
#![allow(clippy::unwrap_used)]

use std::fs;

use recrady::recover_files;

#[test]
fn writes_deleted_files_next_to_the_image() {
    let image = fs_fat::fixture::deleted_files_image(true).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("disk.img");
    let output = dir.path().join("restored");
    fs::write(&source, &image).unwrap();

    let recovery = recover_files(&source, &output).unwrap();

    assert!(recovery.files.iter().any(|file| file.bytes == b"hello"));
    assert_eq!(fs::read(output.join("_ELLO.TXT")).unwrap(), b"hello");
    assert_eq!(fs::read(output.join("SUBDIR/_OTE.TXT")).unwrap(), b"note");
    assert_eq!(fs::read(&source).unwrap(), image);
}
