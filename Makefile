.PHONY: build release install uninstall test lint fmt fmt-check check watch link nvim debug tmux

# --- Build ------------------------------------------------------------------

build:
	cargo build

release:
	cargo build --release

# Install the binary onto your PATH via cargo (recommended).
install:
	cargo install --path .

uninstall:
	cargo uninstall thymeleaf_ls

# --- Quality ----------------------------------------------------------------

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

# Everything CI checks, in one shot.
check: fmt-check lint test

# --- Local LSP debugging (Neovim + lsp-debug-tools.nvim) --------------------

watch:
	cargo watch -x build

# Symlink the debug binary onto PATH (~/.local/bin must exist and be on PATH).
link: build
	mkdir -p ~/.local/bin
	ln -sf "$(CURDIR)/target/debug/thymeleaf_ls" ~/.local/bin/thymeleaf_ls

nvim:
	nvim -c "so debug.lua" -c "lua restart_lsp()" index.html

# Rebuild the OCaml log parser and tail the Neovim LSP log.
debug: link
	cd ~/.local/share/nvim/lazy/lsp-debug-tools.nvim && \
		eval $$(opam env) && dune build && \
		dune exec lsp_debug -- ~/.local/state/nvim/lsp.log --name "thymeleaf_ls"

tmux:
	tmux new-window -n build
	tmux send-keys -t build "make watch" C-m

	tmux new-window -n nvim
	tmux send-keys -t nvim "make nvim" C-m

	tmux new-window -n debug
	tmux send-keys -t debug "make debug" C-m
