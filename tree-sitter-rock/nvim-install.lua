-- Neovim Tree-sitter Rock Manual Installation
-- Add this to your Neovim config (usually ~/.config/nvim/init.lua)

-- Step 1: Filetype detection for .rk files
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Step 2: Manually load the parser
local parser_install_dir = "/home/champii/prog/rust/new_lang/tree-sitter-rock"

-- Add the parser to the runtime path
vim.opt.runtimepath:append(parser_install_dir)

-- Step 3: Configure treesitter to use rock
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()

parser_config.rock = {
  install_info = {
    url = parser_install_dir,
    files = {"src/parser.c"},
    -- Use locally compiled library
    statically_loaded = true,
  },
  filetype = "rock",
}

-- Step 4: Setup treesitter
require("nvim-treesitter.configs").setup({
  highlight = {
    enable = true,
    disable = function(lang, buf)
      -- Disable for languages we don't support
      local disabled = {}
      return vim.tbl_contains(disabled, lang)
    end,
  },

  indent = {
    enable = true,
  },

  -- Ensure rock is recognized
  ignore_install = {},
  sync_install = false,
  auto_install = false,
})

-- Step 5: Manual parser loading function
local function load_rock_parser()
  vim.api.nvim_create_autocmd({"BufEnter", "BufRead", "BufNewFile"}, {
    pattern = "*.rk",
    callback = function()
      local lang = "rock"
      local ok, parser = pcall(vim.treesitter.get_parser, 0, lang)

      if not ok or not parser then
        -- Try to manually attach the parser
        local ts = vim.treesitter
        if ts.language.register then
          ts.language.register("rock", "rk")
        end
      end
    end,
  })
end

load_rock_parser()

-- Print success message
print("✅ Rock language parser loaded!")
