# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Language Server Protocol (LSP) implementation for [Thymeleaf](https://www.thymeleaf.org/) HTML templates, written in Rust. WIP but functional: completion, hover, diagnostics, fragment goto-definition/references, document/workspace symbols, document highlight, folding, document links, code actions, fragment rename, and semantic tokens are implemented. Server speaks LSP over stdin/stdout, targets `html` files.

## Commands

- `cargo build` (or `make build`) — build debug binary at `target/debug/thymeleaf_ls`
- `cargo watch -x build` (or `make watch`) — rebuild on change (needs `cargo-watch`)
- `cargo run` — run server (blocks on stdio, expects an LSP client)

Tests live in `#[cfg(test)] mod tests` blocks inside each `src/*.rs` module, plus end-to-end handler-flow tests in `main.rs` (`mod integration_tests`). Run all with `cargo test`; a single test with `cargo test <name>`.

## Architecture

Modular. `src/main.rs` holds the LSP glue; each feature lives in its own module. `Backend` implements `tower_lsp::LanguageServer` and is wired to the stdio transport in `main()` via `LspService` + `Server`. It holds shared state in `RwLock<Vault>`.

- **Runtime:** `tokio` async, `tower-lsp 0.20` for the LSP protocol layer, `tower_lsp::lsp_types` for LSP messages. `tree-sitter` + `tree-sitter-html` parse the HTML.
- **Capabilities** advertised in `initialize()`: INCREMENTAL text sync; completion (trigger chars `:` and `"`), hover, definition, references, document/workspace symbols, document highlight, document links, folding, code actions, rename (with prepare), and semantic tokens (full). Handlers are real (no stubs): they read the `Vault` and delegate to the feature modules.
- **`initialize`** resolves the workspace root from `workspace_folders` (preferred) or the deprecated `root_uri`, running rootless if neither is present (never fails on a missing root). It builds a `Vault` rooted there and eagerly indexes workspace `.html` files (`scan_workspace_html`) so cross-file fragment navigation works immediately. `did_open` upserts + publishes diagnostics; `did_change` applies each change via `Vault::apply_change` (incremental, splices the changed byte range then re-parses) + publishes; `did_close` removes the doc and clears its diagnostics.
- **Concurrency:** feature functions are synchronous and pure over `&Vault`/`&Document`; handlers take the `RwLock` read guard, call the function, and only `.await` (e.g. `publish_diagnostics`) after dropping the guard — never hold the lock across `.await`.

Feature modules follow a common shape: a small set of `pub fn`s taking `&Vault`/`&Document` + a `Position`/`Range`, returning `lsp_types` values, each with its own `#[cfg(test)] mod tests`.

Foundation:
- **`vault`** — `Vault`: `HashMap<Url, Document>` document store plus the workspace root; `upsert`/`apply_change`/`remove`/`get`/`uris`, a fragment index (`all_fragment_defs`, `find_fragment_definitions`), and `scan_workspace_html`.
- **`document`** — `Document`: text + tree-sitter parse tree; `new`/`update`/`apply_change`; position<->byte-offset conversion (UTF-16 `character` at the LSP boundary, byte offsets internally for tree-sitter), `context_at` returning a `CursorContext` (`AttrName`/`AttrValue`/`TagName`/`Text`/`Other`), `node_at`, and `attributes()` -> `Vec<AttrOccurrence>` (name/value + byte ranges).
- **`thymeleaf`** — static Standard Dialect catalog (`ThymeleafAttr`) + expression-syntax/utility-object reference; `lookup` normalizes both `th:x` and `data-th-x` spellings.

Features:
- **`completion`** — attribute-name completions (filtered by partial) in `AttrName` context; expression/utility completions inside known `th:*` values.
- **`hover`** — attribute docs on the name; contextual expression-syntax help inside `th:*` values.
- **`diagnostics`** — WARNING for unknown `th:*`/`data-th-*` attributes; ERROR for clearly unbalanced brackets in known-attribute values.
- **`navigation`** — fragment `goto` (reference -> definition) and `references` (all reference sites + definitions), parsing `~{tpl :: frag}` reference forms and `th:fragment="name(args)"` definitions.
- **`symbols`** — `document_symbols` (fragments + `id` elements) and `workspace_symbols` (fragment search across the vault).
- **`highlight`** — `document_highlight`: all occurrences of the fragment (def=WRITE/ref=READ) or attribute name under the cursor.
- **`folding`** — `folding_ranges` from tree-sitter element/script/style/comment nodes spanning multiple lines.
- **`links`** — `document_links` for `@{...}` link expressions (depth-aware brace/paren matching for path vars like `@{/o/{id}(...)}`) and concrete `href`/`src` URLs.
- **`code_actions`** — quick fixes for unknown `th:*` attributes: nearest known name by Levenshtein (distance <= 3) and "remove attribute".
- **`rename`** — `prepare_rename` + `rename`: workspace-wide fragment rename, rewriting only the name token (preserving `(args)` and `~{tpl :: }` wrappers).
- **`semantic_tokens`** — `legend()` (PROPERTY/MACRO/STRING/VARIABLE) + `semantic_tokens_full`: delta-encoded tokens for `th:*` names and expression markers. `data` is `Vec<SemanticToken>` (structs), not a flat u32 array.
- **`fragmentref`** — shared fragment-attribute parsing used by `navigation`, `rename`, and `highlight`: `is_fragment_attr`/`is_reference_attr`, `definition_name_range`/`reference_name_range` (name-token byte range within a value), and the `definition_name`/`reference_name` slice helpers. Change fragment parsing rules here only — the consumers all delegate to this module.

## Logging & debugging

- Logging via `structured_logger` → JSON to **stderr** (stdout is reserved for the LSP protocol). The log level is read from the `--level <LEVEL>` (or `--level=<LEVEL>`) CLI arg by `parse_log_level()` in `main.rs`, defaulting to `INFO`.
- Neovim debug loop uses [lsp-debug-tools.nvim](https://github.com/ThePrimeagen/lsp-debug-tools.nvim). `make nvim` opens `index.html` with `debug.lua` (calls `restart_lsp()`). `make debug` symlinks the binary to `/usr/local/bin/thymeleaf_ls` and tails the LSP log via an OCaml/dune parser. `make tmux` sets up the full watch+nvim+debug 3-window workflow.
- `index.html` is the sample template for manual testing.
