# Agent rules

RecRady is built one step at a time. Read [docs/STATUS.md](docs/STATUS.md) before changing anything. It is the source of truth for what exists and what the next step is.

The project is meant to be extended. New crates, filesystem parsers, and device adapters are expected. Extend through the existing boundaries. Do not bypass them.

## Never break

1. No write method on a source device. `BlockDevice` stays read-only. Output goes to a caller-supplied destination that is not the source.
2. No `unsafe` in workspace crates (`unsafe_code = "forbid"`). If a C library is ever required, isolate it in its own crate and say why in an ADR.
3. No panics on input. Parsers return `Result`. Clippy denies `unwrap`, `expect`, `panic`, and `indexing_slicing` outside tests.
4. Do not invent filesystem layouts from memory. Cite the spec or a reference implementation (The Sleuth Kit, libyal, or a named Rust crate) and flag uncertainty.
5. Do not copy GPL code (TestDisk, PhotoRec, and similar). Ideas are fine. Lines of code are not.
6. Dependencies must stay on the allow-list in `deny.toml`. No GPL, AGPL, or LGPL.

## How to add code

- A new capability is a new crate under `crates/`, added to the workspace `members` glob by directory. Depend on traits, not on concrete adapters.
- Filesystem and carving code depends on `BlockDevice`. It must not open files or call OS disk APIs itself.
- A new device type (raw disk, E01, ...) is an adapter in `core-device` infrastructure. It implements `BlockDevice` and does not leak OS types in the public API.
- OS-specific code stays in a small module behind `cfg`. Shared logic stays shared.
- Keep files focused. Do not grow a module into a grab-bag.
- Every behavior change needs a test, including the failure path.
- A decision with a real trade-off gets a short ADR in `docs/decisions/NNNN-title.md`.

## Done means

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --all-targets -- -D warnings` passes.
3. `cargo test --all-targets` passes.
4. `docs/STATUS.md` lists what changed and what the next step is.

Stop at the end of the step. Summarize, give implementation suggestions, and wait for approval before the next step.
