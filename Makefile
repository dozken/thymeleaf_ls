build:
	cargo build

watch:
	cargo watch -x build

nvim:
	nvim -c "so debug.lua" -c "lua restart_lsp()" index.html

debug:
	ln -sf ~/projects/thymeleaf_ls/target/debug/thymeleaf_ls /usr/local/bin/thymeleaf_ls

	cd ~/.local/share/nvim/lazy/lsp-debug-tools.nvim 

	eval $(opam env)          
	dune build

	dune exec lsp_debug -- ~/.local/state/nvim/lsp.log --name "thymeleaf_ls"

tmux:
	tmux new-window -n build
	tmux send-keys -t build "make watch" C-m

	tmux new-window -n nvim
	tmux send-keys -t nvim "make nvim" C-m

	tmux new-window -n debug
	tmux send-keys -t debug "make debug" C-m
