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

- [ ] **Code Completion:** Get intelligent code completion suggestions for Thymeleaf attributes and expressions as you type.  
- [ ] **Documentation:** Access documentation and tooltips for Thymeleaf attributes and expressions right in your editor.
- [ ] **Navigation:** Easily navigate through Thymeleaf templates with support for jumping to declarations and references.
- [ ] **Syntax Highlighting:** Enjoy syntax highlighting for Thymeleaf expressions and tags in your IDE or text editor.
- [ ] **Error Checking:** Receive real-time feedback on syntax errors and other issues in your Thymeleaf templates.

## TODO

- [ ] CI/CD
- [ ] Tests
- [ ] Sample of features
