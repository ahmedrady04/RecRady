# Status

Last updated: 2026-11-10. Steps 1 through 3 are in the tree.

## Done

- Cargo workspace (`edition` 2024, Rust 1.96, shared lints, `cargo-deny`).
- `core-device`:
  - `BlockDevice` — read-only port (`size`, `sector_size`, `read_at`).
  - `ImageFileDevice` — RAW image adapter. Opens with `File::open` (read-only).
  - Bounds checks use `checked_add`. A read past the end is `DeviceError::OutOfBounds`.
  - Positional reads: `pread` on Unix, looped `seek_read` on Windows.
- Integration tests for short, unaligned, empty, past-the-end, overflow, missing file, bad sector size, concurrent reads, and "source bytes unchanged".
- CI: macOS + Windows (fmt, clippy, test) and Ubuntu (`cargo-deny`).
- Competitor and license research: `docs/research/competitors.md`.
- `core-imaging`: RAW copy to a different file, sector retry, zero-fill of unreadable ranges, SHA-256 of the bytes written. Same-file and hard-link destinations are refused. See `docs/decisions/0003-raw-imaging.md`.

## Not started

- FAT / exFAT / NTFS parsers.
- Carving, preview, UI, privileged helper.
- Code signing and installers (phase 8).

## Next step

FAT32 deleted-file recovery on fixture images. Acceptance tests are in `docs/research/competitors.md`: a deleted file whose clusters are still free comes back intact, and the `0xE5` first-byte loss is handled when there is no long name. No exFAT until that FAT case is proven.

## Open decisions

- Public GitHub repo while the product is commercial. Consider making it private before carving algorithms land.
- No LGPL/GPL/CPL linked in. Confirmed 2026-11-10 for TSK (CPL-1.0 / IBM-PL), libyal and libewf (LGPL-3.0-or-later), TestDisk/PhotoRec (GPL-2.0). A lawyer before any exception.
- exFAT metadata recovery is best effort. The Microsoft spec marks unused entry fields as undefined.
