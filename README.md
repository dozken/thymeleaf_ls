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

- [x] **Code Completion:** Get intelligent code completion suggestions for Thymeleaf attributes and expressions as you type.
    - [x] Trigger on `:`
    - [x] Context-aware: attribute names offered in start tags, expression syntaxes offered inside `th:*` values
    - [x] Filters the Standard Dialect catalog by the partial name being typed
- [x] **Documentation:** Access documentation and tooltips (hover) for Thymeleaf attributes and expression syntaxes right in your editor.
- [x] **Navigation:** Jump to fragment definitions (goto-definition) and list fragment references across the workspace.
- [ ] **Syntax Highlighting:** Enjoy syntax highlighting for Thymeleaf expressions and tags in your IDE or text editor.
- [x] **Error Checking:** Real-time diagnostics — flags unknown `th:*` attributes and clearly unbalanced expression brackets.

## TODO

- [ ] CI/CD
- [x] Tests (unit tests via `cargo test`)
- [ ] Trigger completion in element attributes only, e.g. `<div th:_ />`
- [ ] UTF-16 position encoding (currently byte offsets; off for multi-byte lines)
- [ ] Syntax highlighting
- [ ] Sample of features
