# Rock Neovim Plugin

Native LSP support for Rock on Neovim 0.11 or newer, without `nvim-lspconfig`.

## Setup

Install the Rock toolchain first so `rock-lsp` is available on `PATH`. Clone this
repository and add the following to `init.lua`, replacing the checkout path:

```lua
vim.opt.runtimepath:prepend("/absolute/path/to/Rock/neovim")
require("rock").setup()
```

The runtime root is `neovim/`, not the repository root. If using a plugin manager,
configure it to add this subdirectory to `runtimepath`.

Open a `.rk` file and run `:checkhealth rock`. Use `:RockLspInfo` to inspect the
configuration, `:RockLspRestart` to restart clients, and `:help rock` for options.

By default, the plugin checks the repository's `target/release/rock-lsp`, then
`target/debug/rock-lsp`, then `PATH`. To prefer the installed toolchain explicitly:

```lua
require("rock").setup({ cmd = { "rock-lsp" } })
```

Tree-sitter highlighting is separate; see [`tree-sitter-rock/`](../tree-sitter-rock/).

## Layout

- `lua/rock/`: setup, commands, and health checks.
- `lsp/rock.lua`: native LSP defaults.
- `plugin/rock.lua`: filetype and command registration.
- `doc/`: Neovim help.
