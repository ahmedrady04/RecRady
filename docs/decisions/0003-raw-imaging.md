# ADR 0003 — RAW imaging and bad sectors

Status: accepted (2026-11-10)

## Context

A forensic image has to keep every byte at its original offset. A read of a damaged disk fails on some sectors and succeeds on others.

## Decision

`image_raw` reads through `BlockDevice` and writes only to a destination path. The destination is rejected when its canonical path is the source. On Unix a hard link is rejected too, by device and inode. On Windows that identity API is still unstable in Rust 1.96, so a differently named hard link is not detected yet.

On `DeviceError::Io`, the failed window is retried one sector at a time. Sectors that still fail are zeros in the destination, and the run is recorded on the report. Zero-fill keeps later bytes at the right offset. Skipping those bytes would shift the rest of the image. Aborting would drop everything after the first bad sector.

Other source errors (a range past the reported size) abort the run. SHA-256 is computed over the bytes written, zeros included.

## Consequences

- The report distinguishes "unreadable" from "present and zero".
- Callers that image a file must pass its path, or the same-file guard cannot run.
- E01 is out of scope. `libewf` is LGPL.
