# ADR 0005 — FAT32 undelete

Status: accepted (2026-11-10)

## Context

Microsoft's FAT specification says `DIR_Name[0] == 0xE5` marks a directory entry free. The original first character is not stored. Formatting and deletion also clear the FAT entries for that file, so the cluster chain is gone. The FAT type itself is the cluster count in section 3.5, not the `FAT32   ` string.

## Decision

`recrady recover` reads a regular image file and writes restored files to another directory. It never opens `/dev/disk` or `\\.\PhysicalDrive`.

A deleted file is copied only when every cluster from the stored first cluster, for `DIR_FileSize` bytes, is still free. Those clusters are read in order. That is a contiguous assumption: a fragmented file whose chain was wiped cannot be rebuilt, and it is not returned if any of those clusters is allocated again.

The displayed 8.3 name uses `_` for the lost first character. Long names are not reconstructed in this step. Live directories are walked, so a file deleted inside a folder that still exists is found. A whole-disk image is accepted when an MBR or GPT partition contains the FAT32 volume. GPT header offsets follow the UEFI specification; the partition CRC is not checked.

## Consequences

- The acceptance images are bare FAT32 and GPT. A reused cluster is reported, not written out.
- exFAT and NTFS are still out of scope. An external disk formatted as either will not recover with this command.
- Fragmented deleted files stay an open limit.
