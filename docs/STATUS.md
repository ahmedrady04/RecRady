# Status

Last updated: 2026-11-10. Step 1 is in the tree.

## Done

- Cargo workspace (`edition` 2024, Rust 1.96, shared lints, `cargo-deny`).
- `core-device`:
  - `BlockDevice` — read-only port (`size`, `sector_size`, `read_at`).
  - `ImageFileDevice` — RAW image adapter. Opens with `File::open` (read-only).
  - Bounds checks use `checked_add`. A read past the end is `DeviceError::OutOfBounds`.
  - Positional reads: `pread` on Unix, looped `seek_read` on Windows.
- Integration tests for short, unaligned, empty, past-the-end, overflow, missing file, bad sector size, concurrent reads, and "source bytes unchanged".
- CI: macOS + Windows (fmt, clippy, test) and Ubuntu (`cargo-deny`).

## Not started

- `core-imaging` (RAW imaging, bad sectors, SHA-256).
- FAT / exFAT / NTFS parsers.
- Carving, preview, UI, privileged helper.
- Code signing and installers (phase 8).

## Next step

Competitor and license research in `docs/research/competitors.md`: feasibility matrix, license matrix, UX gaps. No product code in that step.

## Open decisions

- Public GitHub repo while the product is commercial. Consider making it private before carving algorithms land.
- C libraries (libewf, libyal) only inside an isolated crate, and only if no suitable Rust crate exists. LGPL needs a legal check before use.
