# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Language Server Protocol (LSP) implementation for [Thymeleaf](https://www.thymeleaf.org/) HTML templates, written in Rust. WIP but functional: completion, hover, diagnostics, and fragment goto-definition/references are implemented. Server speaks LSP over stdin/stdout, targets `html` files.

## Commands

- `cargo build` (or `make build`) — build debug binary at `target/debug/thymeleaf_ls`
- `cargo watch -x build` (or `make watch`) — rebuild on change (needs `cargo-watch`)
- `cargo run` — run server (blocks on stdio, expects an LSP client)

Unit tests live in `#[cfg(test)] mod tests` blocks inside the relevant `src/*.rs` files (`thymeleaf`, `document`, `completion`, `diagnostics`, `navigation`). Run all with `cargo test`; a single test with `cargo test <name>`.

## Architecture

Modular. `src/main.rs` holds the LSP glue; each feature lives in its own module. `Backend` implements `tower_lsp::LanguageServer` and is wired to the stdio transport in `main()` via `LspService` + `Server`. It holds shared state in `RwLock<Vault>`.

- **Runtime:** `tokio` async, `tower-lsp 0.20` for the LSP protocol layer, `tower_lsp::lsp_types` for LSP messages. `tree-sitter` + `tree-sitter-html` parse the HTML.
- **Capabilities** advertised in `initialize()`: FULL text sync, completion (trigger chars `:` and `"`), hover, definition, references. Handlers are real (no stubs): they read the `Vault` and delegate to the feature modules.
- **`initialize`** builds a `Vault` rooted at `root_uri` and eagerly indexes workspace `.html` files (`scan_workspace_html`) so cross-file fragment navigation works immediately. `did_open`/`did_change` upsert into the vault and publish diagnostics; `did_close` removes the doc and clears its diagnostics.

Modules:
- **`vault`** — `Vault`: `HashMap<Url, Document>` document store plus the workspace root; `upsert`/`remove`/`get`, a fragment index (`all_fragment_defs`, `find_fragment_definitions`), and `scan_workspace_html`.
- **`document`** — `Document`: text + tree-sitter parse tree; position<->byte-offset conversion (byte offsets, not UTF-16 — see the caveat comment), `context_at` returning a `CursorContext` (`AttrName`/`AttrValue`/`TagName`/`Text`/`Other`), and `attributes()` extraction.
- **`thymeleaf`** — static Standard Dialect catalog (`ThymeleafAttr`) + expression-syntax/utility-object reference; `lookup` normalizes both `th:x` and `data-th-x` spellings.
- **`completion`** — attribute-name completions (filtered by partial) in `AttrName` context; expression/utility completions inside known `th:*` values.
- **`hover`** — attribute docs on the name; contextual expression-syntax help inside `th:*` values.
- **`diagnostics`** — WARNING for unknown `th:*`/`data-th-*` attributes; ERROR for clearly unbalanced brackets in known-attribute values.
- **`navigation`** — fragment `goto` (reference -> definition) and `references` (all reference sites + definitions), parsing `~{tpl :: frag}` reference forms and `th:fragment="name(args)"` definitions.

## Logging & debugging

- Logging via `structured_logger` → JSON to **stderr** (stdout is reserved for the LSP protocol). The log level is read from the `--level <LEVEL>` (or `--level=<LEVEL>`) CLI arg by `parse_log_level()` in `main.rs`, defaulting to `INFO`.
- Neovim debug loop uses [lsp-debug-tools.nvim](https://github.com/ThePrimeagen/lsp-debug-tools.nvim). `make nvim` opens `index.html` with `debug.lua` (calls `restart_lsp()`). `make debug` symlinks the binary to `/usr/local/bin/thymeleaf_ls` and tails the LSP log via an OCaml/dune parser. `make tmux` sets up the full watch+nvim+debug 3-window workflow.
- `index.html` is the sample template for manual testing.
