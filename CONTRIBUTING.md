# Contributing to thymeleaf_ls

Thanks for your interest in contributing! `thymeleaf_ls` is a
[tower-lsp](https://github.com/ebkalderon/tower-lsp) language server for
Thymeleaf templates, written in Rust.

## Prerequisites

Install a stable Rust toolchain via [rustup](https://rustup.rs/):

```sh
rustup toolchain install stable
rustup default stable
```

## Building

```sh
cargo build
```

## Running tests

```sh
cargo test
```

## Project layout

- The server is built on `tower-lsp`.
- Each feature lives in its own `src/*.rs` module, with its unit tests
  co-located in an in-module `#[cfg(test)] mod tests` block.
- Fragment-attribute parsing is centralized in `src/fragmentref.rs`. If your
  change touches how fragment references are parsed, that is the place to look.

## Before you open a PR

CI must pass before a PR can be merged. Please run these locally first:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- `cargo fmt --all -- --check` — code must be formatted.
- `cargo clippy --all-targets -- -D warnings` — no clippy warnings allowed.
- `cargo test` — all tests must pass.

## License

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the Apache-2.0
license, shall be dual-licensed under the terms of the **MIT OR Apache-2.0**
licenses, without any additional terms or conditions.
