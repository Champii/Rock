-- Neovim setup for tree-sitter-rock
-- Add this to your Neovim config (e.g., ~/.config/nvim/init.lua)

-- Step 1: Register the file type
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Step 2: Configure nvim-treesitter
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()

parser_config.rock = {
  install_info = {
    -- Change this path to where you cloned tree-sitter-rock
    url = "/home/champii/prog/rust/new_lang/tree-sitter-rock",
    files = {"src/parser.c"},
    -- Optional: generate parser from grammar.js dynamically
    generate_requires_npm = true,
    requires_generate_from_grammar = true,
  },
  filetype = "rock",
  -- Use this parser for .rk files
  used_by = { "rock" },
}

-- Step 3: Setup treesitter configuration
require("nvim-treesitter.configs").setup({
  -- Add rock to the list of parsers
  ensure_installed = {
    "rock",
    -- ... other languages
  },

  -- Install parsers synchronously (only applied to `ensure_installed`)
  sync_install = false,

  -- Automatically install missing parsers when entering buffer
  auto_install = true,

  highlight = {
    enable = true,
    -- Or use a specific list
    additional_vim_regex_highlighting = false,
  },

  indent = {
    enable = true,
  },
})

-- Alternative manual installation command:
-- Run :TSInstall rock in Neovim after adding the above config

-- To verify installation:
-- Run :TSInstallInfo rock
-- Run :TSParserInfo in a .rk file
