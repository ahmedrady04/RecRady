# RecRady

Commercial, cross-platform data recovery. The engine is Rust. The UI (later) is Tauri. Windows and macOS share one core; only raw device access is platform-specific.

This repository is proprietary. There is no open-source license. All rights reserved.

## Principles

1. Source devices are read-only. No API can write to a source. Recovered data goes to a different destination.
2. Parsers run on disk images so they can be tested without hardware.
3. Platform code sits behind the `BlockDevice` trait. Everything above it is shared.
4. Input is untrusted. Bad data returns an error. It does not panic.
5. Every parser and carver has tests against fixture images.
6. Key decisions live in `docs/decisions/`.

## Layout

```
crates/core-device/     BlockDevice trait + RAW image reader
crates/core-imaging/    RAW copy, bad sectors, SHA-256
crates/recrady/         `recrady image` for regular files only
docs/decisions/         Architecture decision records
docs/research/          Competitor and license notes
docs/STATUS.md          What is done and what is next
AGENTS.md               Rules for anyone changing this repo
```

Filesystem parsers, carving, and the UI are not in the tree yet. Add a crate when its phase starts. Do not add empty crates.

## Build and test

Requirements are in [REQUIREMENTS.md](REQUIREMENTS.md).

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run -p recrady -- image source.img copy.img
```

`recrady image` reads a regular file and writes a different file. It refuses `/dev/disk`, `\\.\PhysicalDrive`, and directories.

Continuous integration runs the same commands on macOS and Windows, plus a license check (`cargo-deny`) that rejects copyleft dependencies.
