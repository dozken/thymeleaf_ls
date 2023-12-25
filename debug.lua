-- vim.lsp.set_log_level("DEBUG")

function restart_lsp()
    require("lsp-debug-tools").restart({
        expected = { 'html' },
        name = 'thymeleaf_ls',
        cmd = { 'thymeleaf_ls', "--level", "DEBUG" },
        root_dir = vim.loop.cwd(),
        -- root_dir = vim.fs.dirname(vim.fs.find({ 'pom.xml' }, { upward = true })[1]),
    });
end
