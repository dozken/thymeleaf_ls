# Thymeleaf Language Server

[![CI](https://github.com/dozken/thymeleaf_ls/actions/workflows/ci.yml/badge.svg)](https://github.com/dozken/thymeleaf_ls/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) (LSP) implementation for [Thymeleaf](https://www.thymeleaf.org/) templates, written in Rust. It brings completion, documentation, diagnostics, and fragment navigation for the Thymeleaf Standard Dialect to any LSP-capable editor.

> **Status:** early but usable. The features below work today; the dialect catalog and expression analysis will keep growing. Feedback and contributions welcome.

## Features

| Feature | What you get |
| --- | --- |
| **Completion** | `th:*` attribute names in tags (filtered as you type) and expression syntaxes inside `th:*` values. Triggers on `:`. |
| **Hover** | Documentation for Thymeleaf attributes and expression syntaxes. |
| **Diagnostics** | Warnings for unknown `th:*` attributes; errors for unbalanced expression brackets. |
| **Goto-definition / References** | Jump between `th:fragment` definitions and `th:insert`/`th:replace`/`th:include` usages, across the whole workspace. |
| **Rename** | Rename a fragment everywhere it is defined and referenced. |
| **Code actions** | Quick-fix an unknown `th:*` attribute (nearest-match suggestion) or remove it. |
| **Document / workspace symbols** | Outline of fragments and `id` elements; workspace-wide fragment search. |
| **Document highlight** | Highlight every occurrence of the fragment/attribute under the cursor. |
| **Semantic tokens** | Highlight `th:*` names and expression markers. |
| **Folding & document links** | Element/comment folding; clickable `@{...}` and `href`/`src` URLs. |

## Installation

### From source (requires a [Rust toolchain](https://rustup.rs/))

```bash
cargo install --path .
# or, from a clone:
git clone https://github.com/dozken/thymeleaf_ls
cd thymeleaf_ls
make install        # cargo install --path .
```

This installs a `thymeleaf_ls` binary into `~/.cargo/bin` (make sure it is on your `PATH`).

### Prebuilt binaries

Download an archive for your platform from the [Releases](https://github.com/dozken/thymeleaf_ls/releases) page, extract the `thymeleaf_ls` binary, and put it somewhere on your `PATH`.

## Editor setup

The server communicates over stdio and targets HTML documents (`filetype`/`languageId` = `html`). Point your editor's LSP client at the `thymeleaf_ls` binary for HTML files.

### Neovim (0.11+)

```lua
vim.lsp.config.thymeleaf_ls = {
  cmd = { "thymeleaf_ls" },
  filetypes = { "html" },
  root_markers = { "pom.xml", "build.gradle", "build.gradle.kts", ".git" },
}
vim.lsp.enable("thymeleaf_ls")
```

<details>
<summary>Neovim with <code>nvim-lspconfig</code> (older setups)</summary>

```lua
local configs = require("lspconfig.configs")
local lspconfig = require("lspconfig")

if not configs.thymeleaf_ls then
  configs.thymeleaf_ls = {
    default_config = {
      cmd = { "thymeleaf_ls" },
      filetypes = { "html" },
      root_dir = lspconfig.util.root_pattern("pom.xml", "build.gradle", ".git"),
      settings = {},
    },
  }
end

lspconfig.thymeleaf_ls.setup({})
```
</details>

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[language-server.thymeleaf_ls]
command = "thymeleaf_ls"

[[language]]
name = "html"
language-servers = ["thymeleaf_ls"]
```

### Emacs (Eglot)

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((html-mode web-mode) . ("thymeleaf_ls"))))
```

### Sublime Text (LSP)

In `LSP.sublime-settings`:

```json
{
  "clients": {
    "thymeleaf_ls": {
      "enabled": true,
      "command": ["thymeleaf_ls"],
      "selector": "text.html"
    }
  }
}
```

### VS Code

There is no dedicated extension yet. Any generic LSP-client extension that lets you register a stdio server command for HTML files (pointing at `thymeleaf_ls`) will work.

## Configuration

The server takes a single optional CLI flag:

- `--level <LEVEL>` — log level (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`; default `INFO`). Logs are written to **stderr** as JSON; stdout is reserved for the LSP protocol.

```bash
thymeleaf_ls --level DEBUG
```

## Development

```bash
make build        # cargo build
make test         # cargo test
make lint         # cargo clippy --all-targets -- -D warnings
make fmt          # cargo fmt --all
make check        # fmt-check + lint + test (what CI runs)
```

A single test: `cargo test <name>`.

### Layout

The server is a [`tower-lsp`](https://crates.io/crates/tower-lsp) backend; each capability lives in its own module under `src/`:

- `main.rs` — LSP glue, capabilities, request handlers, incremental sync
- `document.rs` / `vault.rs` — tree-sitter-parsed documents and the workspace store
- `thymeleaf.rs` — the Standard Dialect attribute + expression catalog
- `fragmentref.rs` — shared fragment-attribute parsing
- `completion.rs`, `hover.rs`, `diagnostics.rs`, `navigation.rs`, `rename.rs`, `code_actions.rs`, `symbols.rs`, `highlight.rs`, `folding.rs`, `links.rs`, `semantic_tokens.rs` — features

See [`CLAUDE.md`](CLAUDE.md) for a deeper architecture tour and [`CONTRIBUTING.md`](CONTRIBUTING.md) to get started.

### Debugging in Neovim

`index.html` is a sample template to try the server against. The `make nvim` / `make debug` / `make tmux` targets wire up a live debug loop with [lsp-debug-tools.nvim](https://github.com/ThePrimeagen/lsp-debug-tools.nvim); `make link` symlinks the debug build onto your `PATH`.

## License

Licensed under either of

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project, as defined in the Apache-2.0 license, shall be dual-licensed as above, without any additional terms or conditions.
