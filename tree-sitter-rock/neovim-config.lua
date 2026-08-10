-- Complete Neovim configuration for Rock language Tree-sitter support
-- Add this to your ~/.config/nvim/init.lua

-- ============================================
-- Step 1: Filetype Detection
-- ============================================
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- ============================================
-- Step 2: Add Rock grammar to runtime path
-- ============================================
vim.opt.runtimepath:append("/home/champii/prog/rust/new_lang/tree-sitter-rock")

-- ============================================
-- Step 3: Configure Treesitter for Rock
-- ============================================

-- Register Rock parser with treesitter
vim.treesitter.language.register('rock', 'rk')

-- Setup nvim-treesitter
require'nvim-treesitter.configs'.setup({
  -- Make sure Rock is included
  ensure_installed = {},  -- Empty means use installed parsers

  highlight = {
    enable = true,
    disable = function(lang, buf)
      local max_filesize = 100 * 1024 -- 100 KB
      local ok, stats = pcall(vim.loop.fs_stat, vim.api.nvim_buf_get_name(buf))
      if ok and stats and stats.size > max_filesize then
        return true
      end
    end,
    additional_vim_regex_highlighting = false,
  },

  indent = {
    enable = true,
  },

  -- Enable textobjects for better navigation
  textobjects = {
    select = {
      enable = true,
      lookahead = true,
    },
  },
})

-- ============================================
-- Step 4: Manual parser loading (if needed)
-- ============================================
local function setup_rock_parser()
  -- Create autocmd to load Rock parser for .rk files
  vim.api.nvim_create_autocmd({"BufRead", "BufNewFile"}, {
    pattern = "*.rk",
    callback = function()
      -- Set filetype
      vim.bo.filetype = 'rock'

      -- Force parser to attach
      local lang_tree = vim.treesitter.get_parser(0, 'rock')
      if lang_tree then
        lang_tree:parse()
      end
    end,
  })
end

setup_rock_parser()

-- ============================================
-- Optional: Add custom highlight groups
-- ============================================
-- You can customize colors if you want:
-- vim.api.nvim_set_hl(0, "@keyword.rock", { link = "Keyword" })
-- vim.api.nvim_set_hl(0, "@function.rock", { link = "Function" })
-- vim.api.nvim_set_hl(0, "@type.rock", { link = "Type" })
-- vim.api.nvim_set_hl(0, "@string.rock", { link = "String" })
-- vim.api.nvim_set_hl(0, "@number.rock", { link = "Number" })
-- vim.api.nvim_set_hl(0, "@comment.rock", { link = "Comment" })

print("✅ Rock language Tree-sitter support loaded!")
