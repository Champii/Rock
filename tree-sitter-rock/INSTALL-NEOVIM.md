# Simple Neovim Installation for Rock Language

The `:TSInstall rock` command won't work because this isn't in the nvim-treesitter registry.
Use this manual installation instead:

## Method 1: Quick Manual Install (Recommended)

### Step 1: Copy the parser
```bash
# The parser is already copied to:
# ~/.local/share/nvim/site/parser/rock.so
```

### Step 2: Add to your Neovim config

Add this to `~/.config/nvim/init.lua`:

```lua
-- Add Rock filetype detection
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Configure treesitter for Rock
vim.treesitter.language.register('rock', 'rk')

-- Setup treesitter
require'nvim-treesitter.configs'.setup({
  highlight = {
    enable = true,
    additional_vim_regex_highlighting = false,
  },
})
```

### Step 3: Test it

1. Restart Neovim
2. Open any `.rk` file
3. Check if highlighting works: `:TSBufEnable highlight`

If it still doesn't work, try: `:set filetype=rock`

---

## Method 2: Using Vimscript (if you use init.vim)

Add to `~/.config/nvim/init.vim`:

```vim
" Filetype detection
autocmd BufRead,BufNewFile *.rk set filetype=rock

" Treesitter setup
lua <<EOF
require'nvim-treesitter.configs'.setup({
  highlight = {
    enable = true,
  },
})
EOF
```

---

## Troubleshooting

### No syntax highlighting?

1. **Check filetype:**
   ```vim
   :set filetype?
   ```
   Should say `filetype=rock`

2. **Enable treesitter manually:**
   ```vim
   :TSBufEnable highlight
   ```

3. **Check if parser is loaded:**
   ```vim
   :lua print(vim.inspect(vim.treesitter.language.get('rock')))
   ```

4. **Manually set the parser:**
   ```vim
   :lua vim.treesitter.language.register('rock', 'rk')
   ```

### "Parser not found" error?

The parser file needs to be in the right place. Check:

```vim
:echo stdpath('data')
```

The parser should be at: `<data>/parser/rock.so`

Where `<data>` is typically `~/.local/share/nvim`

Copy it there if needed:
```bash
cp /home/champii/prog/rust/new_lang/tree-sitter-rock/rock.so ~/.local/share/nvim/site/parser/rock.so
```

---

## Method 3: Lazy.nvim (if you use lazy)

If you use lazy.nvim for plugin management, add this to your config:

```lua
-- Add filetype detection
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Setup treesitter
{
  "nvim-treesitter/nvim-treesitter",
  opts = {
    highlight = {
      enable = true,
    },
  },
},
```

---

## Verify Installation

Open a `.rk` file with this content:

```haskell
struct Point
    x: Int
    y: Int

main = !->
    point = Point
        x: 10
        y: 20
    point.x + point.y
```

You should see:
- `struct` highlighted as a keyword
- `Point` highlighted as a type
- `x` and `y` highlighted as properties
- `main` highlighted as a function
- `!->` highlighted as an operator
- Comments in a different color

If you see this, it's working! ✅

---

## Still having issues?

Try this diagnostic command in Neovim:

```vim
:checkhealth nvim-treesitter
```

Look for `rock` in the list of installed parsers. If it's not there, make sure:
1. `rock.so` exists in `~/.local/share/nvim/site/parser/`
2. The file is readable (not corrupted)
3. You've restarted Neovim after copying the file
