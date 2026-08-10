-- ============================================
-- COMPLETE NEOVIM SETUP FOR ROCK LANGUAGE
-- Add this ENTIRE file to your ~/.config/nvim/init.lua
-- ============================================

-- Step 1: Filetype detection
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Step 2: Register Rock with treesitter
vim.treesitter.language.register('rock', 'rk')

-- Step 3: Configure nvim-treesitter
require'nvim-treesitter.configs'.setup({
  -- Ensure Rock parser is loaded
  ensure_installed = {},  -- Empty = use installed parsers

  highlight = {
    enable = true,
    disable = function(lang, buf)
      -- Don't disable for rock
      if lang == 'rock' then
        return false
      end
      -- Disable for very large files
      local ok, stats = pcall(vim.loop.fs_stat, vim.api.nvim_buf_get_name(buf))
      if ok and stats and stats.size > 100 * 1024 then
        return true
      end
    end,
    additional_vim_regex_highlighting = false,
  },

  indent = {
    enable = true,
  },

  -- Incremental selection
  incremental_selection = {
    enable = true,
    keymaps = {
      init_selection = "<CR>",
      node_incremental = "<CR>",
      scope_incremental = "<TAB>",
      node_decremental = "<S-TAB>",
    },
  },
})

-- Step 4: Define Rock highlight groups (if your theme doesn't have them)
local function define_rock_highlights()
  -- Link Rock captures to standard highlight groups
  local highlights = {
    -- Keywords
    ["@keyword.rock"] = "Keyword",
    ["@conditional.rock"] = "Conditional",
    ["@repeat.rock"] = "Repeat",
    ["@keyword.return.rock"] = "Return",
    ["@keyword.coroutine.rock"] = "Operator",
    ["@include.rock"] = "Include",

    -- Types and constructors
    ["@type.rock"] = "Type",
    ["@type.builtin.rock"] = "Type",
    ["@type.definition.rock"] = "Define",
    ["@constructor.rock"] = "Function",

    -- Functions
    ["@function.rock"] = "Function",
    ["@function.call.rock"] = "Function",
    ["@function.macro.rock"] = "Macro",
    ["@method.call.rock"] = "Function",

    -- Variables and properties
    ["@variable.rock"] = "Identifier",
    ["@variable.parameter.rock"] = "Identifier",
    ["@variable.builtin.rock"] = "Special",
    ["@property.rock"] = "Identifier",

    -- Literals
    ["@number.rock"] = "Number",
    ["@float.rock"] = "Float",
    ["@string.rock"] = "String",
    ["@character.rock"] = "Character",
    ["@boolean.rock"] = "Boolean",
    ["@constant.builtin.rock"] = "Special",

    -- Operators and punctuation
    ["@operator.rock"] = "Operator",
    ["@punctuation.delimiter.rock"] = "Delimiter",
    ["@punctuation.bracket.rock"] = "Delimiter",

    -- Macros
    ["@macro.rock"] = "Macro",

    -- Comments
    ["@comment.rock"] = "Comment",
  }

  -- Apply highlights
  for capture, group in pairs(highlights) do
    vim.api.nvim_set_hl(0, capture, { link = group, default = true })
  end
end

-- Step 5: Setup autocmd for Rock files
vim.api.nvim_create_autocmd({"BufRead", "BufNewFile"}, {
  pattern = "*.rk",
  callback = function()
    -- Set filetype
    vim.bo.filetype = 'rock'

    -- Enable treesitter highlighting
    if vim.treesitter.start then
      vim.treesitter.start()
    end

    -- Define highlights for this buffer
    define_rock_highlights()
  end,
})

-- Step 6: Force load parser on startup
vim.defer_fn(function()
  -- Try to load the parser
  local ok, parser = pcall(vim.treesitter.get_parser, 0, 'rock')
  if ok and parser then
    print("✅ Rock parser loaded successfully!")
  else
    vim.notify("Rock parser not found. Make sure rock.so is in ~/.local/share/nvim/site/parser/", vim.log.levels.WARN)
  end
end, 100)

print("🪨 Rock language support loaded!")
