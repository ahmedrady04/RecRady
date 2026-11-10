# ADR 0002 — Language, UI, and licenses

Status: accepted (2026-11-10)

## Context

The product is commercial and closed source. It has to be fast, safe on corrupt input, and buildable on Windows and macOS from one tree. An earlier note suggested a C++ core with Qt 6. That conflicts with a Rust core and Tauri UI.

## Decision

- Recovery engine: Rust.
- UI (later phase): Tauri. Not Qt. Qt 6's commercial license is a cost, and its LGPL option constrains shipping a closed-source app.
- C or C++ only as an adapter around an existing library when no acceptable Rust crate exists (possible example: libewf for E01). That code lives in its own crate. New recovery logic is not written in C++.
- License policy is `deny.toml`. Permitted: MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0. GPL, AGPL, and LGPL are rejected. PhotoRec and TestDisk are GPL: study behavior, do not copy source.
- The repo has no `LICENSE` file on purpose. Absence of a license means all rights reserved. It is not open source.

## Consequences

- One `cargo` build on both operating systems.
- Corrupt input is handled with `Result`, not with manual memory management.
- libyal (LGPL) and The Sleuth Kit (mixed licenses) are references. Linking them requires a written decision and a license review first.
- `cargo-deny` fails CI if a copyleft crate is added.
