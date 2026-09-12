# ✅ FIXED! Query Error Resolved

## Problem
The query file had errors because:
1. Tree-sitter query language treats certain bare strings as node types
2. Single characters like `?`, `|`, `{`, `}` have special meanings
3. Can't match the same node twice with different captures

## Solution
Created a minimal, working query that:
- Matches entire declaration/expression nodes
- Avoids special characters that conflict with query syntax
- Uses proper tree-sitter query patterns

## What Now?

### Restart Neovim completely:
```bash
:qa
nvim examples/main_test.rk
```

### Check health:
```vim
:checkhealth nvim-treesitter
```

You should see:
```
rock             H✓ F✓ I✓ J✓
```
With **green checkmarks**!

### What Gets Highlighted:
- ✅ **Keywords**: struct, enum, if, then, else, for, while, match, return, pub, etc.
- ✅ **Types**: Point, Color, Int, String, Bool, etc.
- ✅ **Functions**: main, add, show (declarations and calls)
- ✅ **Literals**: Numbers, strings, booleans
- ✅ **Comments**: // line comments
- ✅ **Operators**: =, ->, =>, ::
- ✅ **Control flow**: if/then/else, for loops, match expressions

### If Still Not Working:
1. **Enable highlighting manually**:
   ```vim
   :TSBufEnable highlight
   ```

2. **Check filetype**:
   ```vim
   :set filetype?
   ```
   Should be `filetype=rock`

3. **Check for errors**:
   ```vim
   :messages
   ```

## Files Installed:
- ✅ `~/.local/share/nvim/site/parser/rock.so` (775KB parser)
- ✅ `~/.local/share/nvim/site/queries/rock/highlights.scm` (fixed!)
- ✅ `~/.local/share/nvim/site/queries/rock/indents.scm`
- ✅ `~/.local/share/nvim/site/queries/rock/locals.scm`

## Neovim Config (add to `~/.config/nvim/init.lua`):

```lua
-- Rock language filetype
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Register with treesitter
vim.treesitter.language.register('rock', 'rk')

-- Setup
require'nvim-treesitter.configs'.setup({
  highlight = {
    enable = true,
  },
  indent = {
    enable = true,
  },
})
```

Now your Rock language should have beautiful syntax highlighting! 🎨
