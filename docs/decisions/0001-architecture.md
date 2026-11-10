# ADR 0001 — Architecture

Status: accepted (2026-11-10)

## Context

RecRady must read disks and images on Windows and macOS, stay testable without hardware, and make writes to a source device impossible rather than merely discouraged.

## Decision

Clean Architecture, one direction of dependency:

- Domain: `BlockDevice` and `DeviceError`. No filesystem, file, or OS types.
- Infrastructure: adapters such as `ImageFileDevice`. They implement the trait.
- Upper layers (imaging, filesystem parsers, carving, UI) depend on the trait only.

`read_at` takes `&self` plus an offset and either fills the buffer or returns an error. Positional reads (`pread` / `seek_read`) keep the file cursor out of the picture so several threads can scan one device.

Sector size is part of the trait now (default 512, must be a non-zero power of two). Image files do not need alignment. Physical disks on Windows (unbuffered I/O) and macOS (`/dev/rdisk`) do. Callers that need aligned I/O can enforce it later without changing the trait.

There is no write method. A future imaging crate writes only to a destination path the caller passes in, and it must refuse when that path is the source.

## Consequences

- Parsers are tested with `ImageFileDevice` and small generated images.
- Adding a real disk adapter does not change parser code.
- Concurrent carving does not need a lock around the device.
