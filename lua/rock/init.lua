local M = {}

M.config = nil

local function plugin_root()
  local source = debug.getinfo(1, "S").source:sub(2)
  return vim.fs.dirname(vim.fs.dirname(vim.fs.dirname(source)))
end

local function executable(path)
  return path and path ~= "" and vim.fn.executable(path) == 1
end

local function default_server()
  local root = plugin_root()
  for _, profile in ipairs({ "release", "debug" }) do
    local candidate = vim.fs.joinpath(root, "target", profile, "rock-lsp")
    if executable(candidate) then
      return candidate
    end
  end

  local from_path = vim.fn.exepath("rock-lsp")
  if executable(from_path) then
    return from_path
  end

  return "rock-lsp"
end

local function server_command(opts)
  local configured = opts.cmd or (opts.server and opts.server.cmd)
  local cmd = vim.deepcopy(configured or { default_server() })
  if type(cmd) == "string" then
    cmd = { cmd }
  end
  if type(cmd) ~= "table" or type(cmd[1]) ~= "string" then
    error("rock.nvim: cmd must be a command string or list")
  end

  local names = vim.tbl_keys(opts.extern_artifacts or {})
  table.sort(names)
  for _, name in ipairs(names) do
    local path = opts.extern_artifacts[name]
    if type(name) ~= "string" or type(path) ~= "string" then
      error("rock.nvim: extern_artifacts must map names to artifact paths")
    end
    table.insert(cmd, "--extern-artifact")
    table.insert(cmd, name .. "=" .. vim.fs.normalize(vim.fn.expand(path)))
  end

  if opts.no_prelude then
    table.insert(cmd, "--no-prelude")
  end
  return cmd
end

local function require_native_lsp()
  if vim.fn.has("nvim-0.11") == 0 or type(vim.lsp.config) ~= "table" then
    error("rock.nvim requires Neovim 0.11 or newer")
  end
end

---Configure and enable the Rock language server.
---@param opts? table
function M.setup(opts)
  require_native_lsp()
  opts = opts or {}

  local server = vim.tbl_deep_extend("force", {}, opts.server or {})
  server.cmd = server_command(opts)
  vim.lsp.config("rock", server)

  M.config = {
    autostart = opts.autostart ~= false,
    cmd = server.cmd,
    extern_artifacts = vim.deepcopy(opts.extern_artifacts or {}),
    no_prelude = opts.no_prelude == true,
  }
  if M.config.autostart then
    vim.lsp.enable("rock")
  end
end

function M.restart()
  require_native_lsp()
  for _, client in ipairs(vim.lsp.get_clients({ name = "rock" })) do
    client:stop(true)
  end
  vim.lsp.enable("rock", false)
  vim.schedule(function()
    vim.lsp.enable("rock")
  end)
end

function M.info()
  require_native_lsp()
  local clients = vim.lsp.get_clients({ name = "rock" })
  local lines = {
    "Rock LSP config:",
    vim.inspect(vim.lsp.config.rock),
    "",
    string.format("Active clients: %d", #clients),
  }
  for _, client in ipairs(clients) do
    table.insert(lines, string.format("- #%d root=%s", client.id, client.root_dir or "<none>"))
  end
  vim.notify(table.concat(lines, "\n"), vim.log.levels.INFO, { title = "rock.nvim" })
end

M._default_server = default_server
M._server_command = server_command

return M
