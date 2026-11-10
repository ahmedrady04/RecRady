# Development requirements

Product requirements are tracked in [docs/STATUS.md](docs/STATUS.md) and the decision records. This file is only the toolchain.

## Common

| Tool | Version | Notes |
| --- | --- | --- |
| Rust | 1.96 | Pinned in `rust-toolchain.toml`. `rustup` installs it automatically. |
| rustfmt, clippy | same toolchain | Required components. |
| git | any current | Line endings are LF via `.gitattributes`. |
| cargo-deny | v2 | Used in CI. Optional locally. |

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Warnings are errors. `unsafe` is forbidden in workspace crates.

## macOS (current development machine)

- macOS on Apple Silicon or Intel.
- Xcode Command Line Tools (`xcode-select --install`) so Rust can link test binaries.
- No extra libraries are required for `core-device`.

## Windows (when moving the tree)

- Windows 10 or later, 64-bit.
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload (MSVC linker).
- `rustup` default host `x86_64-pc-windows-msvc`.
- Git for Windows. Do not convert line endings; `.gitattributes` already forces LF.

The same commands above must pass before a change is pushed. GitHub Actions runs them on `macos-latest` and `windows-latest`.
