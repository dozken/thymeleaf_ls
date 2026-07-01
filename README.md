# Thymeleaf Language Server Protocol (LSP)

## Note
⚠️ **Work in Progress:** This project is currently under development and is in the early stages. Contributions and feedback are highly appreciated as we work towards improving and expanding the Thymeleaf Language Server Protocol.

## Overview

This is a Language Server Protocol (LSP) implementation for Thymeleaf, a modern server-side Java template engine for web and standalone environments. The Thymeleaf LSP aims to provide enhanced development support for Thymeleaf templates in various integrated development environments (IDEs) and text editors.

## How to use in Neovim with lsp-debug-tools
1. Build project in watch mode and link it to bin
    ```bash
    cargo watch -x build  
    ln -s ./target/debug/thymeleaf_ls /usr/local/bin/thymeleaf_ls  
    ```

2. Install [lsp-debug-tools.nvim](https://github.com/ThePrimeagen/lsp-debug-tools.nvim) using your favorite plugin manager.   
    Using [lazy.nvim](https://github.com/folke/lazy.nvim)

    ```lua
    -- init.lua:
    {
        'ThePrimeagen/lsp-debug-tools.nvim'
    }

    -- plugins/lsp-debug-tools.lua:
    return {
        'ThePrimeagen/lsp-debug-tools.nvim'
    }
    ```


3. Build the log parser, your path to plugin may vary.
    ```bash
    cd ~/.local/share/nvim/lazy/lsp-debug-tools.nvim
    dune build
    dune exec lsp_debug -- ~/.local/state/nvim/lsp.log --name "thymeleaf_ls"
    ```
4. In Neovim
    ```lua
    :so debug.lua

    :lua restart_lsp()
    ```
5. Open an html file and start debugging.    

## Features

- [x] **Code Completion:** Context-aware suggestions — attribute names in start tags, expression syntaxes inside `th:*` values; filters the Standard Dialect catalog by the partial name typed; triggers on `:`.
- [x] **Documentation:** Hover docs for Thymeleaf attributes and expression syntaxes.
- [x] **Navigation:** Goto-definition and find-references for fragments across the workspace.
- [x] **Error Checking:** Real-time diagnostics — unknown `th:*` attributes and clearly unbalanced expression brackets.
- [x] **Syntax Highlighting:** Semantic tokens for `th:*` attribute names and expression markers.
- [x] **Document Symbols / Outline:** Fragments and `id` elements; plus workspace symbol search.
- [x] **Document Highlight:** Highlights all occurrences of the fragment/attribute under the cursor.
- [x] **Folding:** Element, `<script>`/`<style>`, and comment folding ranges.
- [x] **Document Links:** Clickable `@{...}` link expressions and concrete `href`/`src` URLs.
- [x] **Code Actions:** Quick fixes for unknown `th:*` attributes (nearest-match suggestion, remove attribute).
- [x] **Rename:** Fragment rename (with prepare-rename) across all workspace files.
- [x] **Incremental Sync:** `TextDocumentSyncKind::INCREMENTAL`.

## TODO

- [x] CI/CD (GitHub Actions: fmt + clippy + build + test)
- [x] Tests (unit + integration via `cargo test`)
- [ ] Trigger completion in element attributes only, e.g. `<div th:_ />`
- [ ] Negotiate `positionEncoding` (positions are UTF-16 at the LSP boundary)
- [ ] Sample of features
