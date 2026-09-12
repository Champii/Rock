local M = {}

function M.check()
  vim.health.start("rock.nvim")

  if vim.fn.has("nvim-0.11") == 1 and type(vim.lsp.config) == "table" then
    vim.health.ok("Neovim provides vim.lsp.config and vim.lsp.enable")
  else
    vim.health.error("Neovim 0.11 or newer is required")
    return
  end

  local rock = require("rock")
  local command = rock.config and rock.config.cmd or { rock._default_server() }
  if vim.fn.executable(command[1]) == 1 then
    vim.health.ok("rock-lsp is executable: " .. command[1])
  else
    vim.health.error("rock-lsp is not executable: " .. command[1], {
      "Install a current Rock toolchain with rockup, which includes the rock-lsp shim.",
      "Or pass `cmd = { '/absolute/path/to/rock-lsp' }` to require('rock').setup().",
    })
  end

  if rock.config then
    vim.health.ok("Rock LSP configuration is enabled")
  else
    vim.health.warn("require('rock').setup() has not been called")
  end

  local clients = vim.lsp.get_clients({ name = "rock" })
  if #clients > 0 then
    vim.health.ok(string.format("%d Rock LSP client(s) active", #clients))
  else
    vim.health.info("No Rock LSP client is active; open a *.rk file to start one")
  end
end

return M
