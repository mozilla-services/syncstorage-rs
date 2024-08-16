local dap = require("dap")

dap.configurations.rust = {
    {
        name = "syncserver",
        type = "lldb",
        request = "launch",
        program = function()
            return vim.fn.getcwd() .. "/target/debug/syncserver"
        end,
        cwd = "${workspaceFolder}",
        stopOnEntry = false,
    }
}
