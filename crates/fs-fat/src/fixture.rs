//! In-memory FAT32 images for tests. This is not a disk formatter.

use crate::error::FatError;

const BYTES_PER_SECTOR: usize = 512;
const RESERVED_SECTORS: usize = 32;
const FAT_SECTORS: usize = 512;
const DATA_SECTORS: usize = 65_526;
const FIRST_DATA_SECTOR: usize = RESERVED_SECTORS + 2 * FAT_SECTORS;

/// Bare FAT32 volume. `HELLO.TXT` is deleted in the root. `SUBDIR/NOTE.TXT` is
/// deleted inside a directory that still exists. `hello_free` controls whether
/// the HELLO cluster is still marked free.
pub fn deleted_files_image(hello_free: bool) -> Result<Vec<u8>, FatError> {
    let sectors = RESERVED_SECTORS + 2 * FAT_SECTORS + DATA_SECTORS;
    let mut image = vec![0u8; sectors * BYTES_PER_SECTOR];
    write_boot(&mut image)?;
    put_u32(&mut image, fat_offset(0), 0x0FFF_FFF8)?;
    put_u32(&mut image, fat_offset(1), 0x0FFF_FFFF)?;
    put_u32(&mut image, fat_offset(2), 0x0FFF_FFFF)?;
    put_u32(
        &mut image,
        fat_offset(3),
        if hello_free { 0 } else { 0x0FFF_FFFF },
    )?;
    put_u32(&mut image, fat_offset(4), 0)?;
    put_u32(&mut image, fat_offset(6), 0x0FFF_FFFF)?;

    write_entry(
        &mut image,
        Entry {
            dir_cluster: 2,
            index: 0,
            name: b"HELLO   TXT",
            attr: 0x20,
            cluster: 3,
            size: 5,
            deleted: true,
        },
    )?;
    write_entry(
        &mut image,
        Entry {
            dir_cluster: 2,
            index: 1,
            name: b"SUBDIR     ",
            attr: 0x10,
            cluster: 6,
            size: 0,
            deleted: false,
        },
    )?;
    write_entry(
        &mut image,
        Entry {
            dir_cluster: 6,
            index: 0,
            name: b".          ",
            attr: 0x10,
            cluster: 6,
            size: 0,
            deleted: false,
        },
    )?;
    write_entry(
        &mut image,
        Entry {
            dir_cluster: 6,
            index: 1,
            name: b"..         ",
            attr: 0x10,
            cluster: 2,
            size: 0,
            deleted: false,
        },
    )?;
    write_entry(
        &mut image,
        Entry {
            dir_cluster: 6,
            index: 2,
            name: b"NOTE    TXT",
            attr: 0x20,
            cluster: 4,
            size: 4,
            deleted: true,
        },
    )?;
    write_bytes(&mut image, cluster_offset(3), b"hello")?;
    write_bytes(&mut image, cluster_offset(4), b"note")?;
    Ok(image)
}

/// Same filesystem behind a GPT partition starting at LBA 34.
///
/// The header fields used here are the UEFI GPT header: the `EFI PART`
/// signature, `PartitionEntryLBA` at byte 72, and `StartingLBA` at byte 32 of
/// an entry. Partition CRC32 is not filled in.
pub fn deleted_files_gpt_image(hello_free: bool) -> Result<Vec<u8>, FatError> {
    let fat = deleted_files_image(hello_free)?;
    let header_sectors = 34usize;
    let mut image = vec![0u8; header_sectors * BYTES_PER_SECTOR + fat.len()];
    put_u16(&mut image, 510, 0xAA55)?;
    // Protective MBR partition, type 0xEE, starting at LBA 1.
    put_u8(&mut image, 446 + 4, 0xEE)?;
    put_u32(&mut image, 446 + 8, 1)?;

    write_bytes(&mut image, 512, b"EFI PART")?;
    put_u64(&mut image, 512 + 24, 1)?;
    put_u64(&mut image, 512 + 72, 2)?;
    put_u32(&mut image, 512 + 80, 128)?;
    put_u32(&mut image, 512 + 84, 128)?;

    let entry = 2 * BYTES_PER_SECTOR;
    write_bytes(&mut image, entry, &[0x01; 16])?;
    put_u64(&mut image, entry + 32, 34)?;
    put_u64(
        &mut image,
        entry + 40,
        34 + (fat.len() / BYTES_PER_SECTOR) as u64,
    )?;

    let Some(dest) = image.get_mut(header_sectors * BYTES_PER_SECTOR..) else {
        return Err(FatError::Truncated);
    };
    dest.copy_from_slice(&fat);
    Ok(image)
}

fn write_boot(image: &mut [u8]) -> Result<(), FatError> {
    put_u8(image, 0, 0xEB)?;
    put_u8(image, 1, 0x58)?;
    put_u8(image, 2, 0x90)?;
    put_u16(image, 11, 512)?;
    put_u8(image, 13, 1)?;
    put_u16(image, 14, RESERVED_SECTORS as u16)?;
    put_u8(image, 16, 2)?;
    put_u16(image, 17, 0)?;
    put_u16(image, 19, 0)?;
    put_u8(image, 21, 0xF8)?;
    put_u16(image, 22, 0)?;
    put_u32(
        image,
        32,
        (RESERVED_SECTORS + 2 * FAT_SECTORS + DATA_SECTORS) as u32,
    )?;
    put_u32(image, 36, FAT_SECTORS as u32)?;
    put_u16(image, 42, 0)?;
    put_u32(image, 44, 2)?;
    write_bytes(image, 82, b"FAT32   ")?;
    put_u16(image, 510, 0xAA55)?;
    Ok(())
}

struct Entry {
    dir_cluster: u32,
    index: usize,
    name: &'static [u8; 11],
    attr: u8,
    cluster: u32,
    size: u32,
    deleted: bool,
}

fn write_entry(image: &mut [u8], entry: Entry) -> Result<(), FatError> {
    let offset = cluster_offset(entry.dir_cluster) + entry.index * 32;
    write_bytes(image, offset, entry.name)?;
    if entry.deleted {
        put_u8(image, offset, 0xE5)?;
    }
    put_u8(image, offset + 11, entry.attr)?;
    put_u16(image, offset + 20, (entry.cluster >> 16) as u16)?;
    put_u16(image, offset + 26, entry.cluster as u16)?;
    put_u32(image, offset + 28, entry.size)?;
    Ok(())
}

fn fat_offset(cluster: u32) -> usize {
    RESERVED_SECTORS * BYTES_PER_SECTOR + cluster as usize * 4
}

fn cluster_offset(cluster: u32) -> usize {
    (FIRST_DATA_SECTOR + (cluster as usize - 2)) * BYTES_PER_SECTOR
}

fn write_bytes(image: &mut [u8], offset: usize, bytes: &[u8]) -> Result<(), FatError> {
    let end = offset.checked_add(bytes.len()).ok_or(FatError::Truncated)?;
    let slot = image.get_mut(offset..end).ok_or(FatError::Truncated)?;
    slot.copy_from_slice(bytes);
    Ok(())
}

fn put_u8(image: &mut [u8], offset: usize, value: u8) -> Result<(), FatError> {
    *image.get_mut(offset).ok_or(FatError::Truncated)? = value;
    Ok(())
}

fn put_u16(image: &mut [u8], offset: usize, value: u16) -> Result<(), FatError> {
    write_bytes(image, offset, &value.to_le_bytes())
}

fn put_u32(image: &mut [u8], offset: usize, value: u32) -> Result<(), FatError> {
    write_bytes(image, offset, &value.to_le_bytes())
}

fn put_u64(image: &mut [u8], offset: usize, value: u64) -> Result<(), FatError> {
    write_bytes(image, offset, &value.to_le_bytes())
}
