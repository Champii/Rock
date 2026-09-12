# Final Installation Instructions for Rock Language Syntax Highlighting

## Problem Checkhealth shows:
```
rock             HL✅ FI✅ I✅ J✅
```
But nothing is activated (✅ = supported but not working)

## Solution: Add This Minimal Config to Your `~/.config/nvim/init.lua`

```lua
-- Filetype for Rock
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Register Rock with treesitter
vim.treesitter.language.register('rock', 'rk')

-- Setup treesitter
require'nvim-treesitter.configs'.setup({
  highlight = {
    enable = true,
  },
  indent = {
    enable = true,
  },
})
```

## Then Restart Neovim Completely

1. Quit Neovim: `:qa`
2. Restart: `nvim examples/main_test.rk`
3. Check: `:checkhealth nvim-treesitter`

You should now see:
```
rock             X  X  X  X
```
Where X are colored green/red (showing which features are active)

## Verify It's Working

Open any `.rk` file and run:
```vim
:lua print(vim.inspect(vim.treesitter.highlighter.active[1]))
```

If you see a table of highlight captures, it's working!

## What We Fixed

1. ✅ Parser binary: `~/.local/share/nvim/site/parser/rock.so` (775KB)
2. ✅ Query files: `~/.local/share/nvim/site/queries/rock/*.scm`
   - highlights.scm
   - indents.scm
   - locals.scm

3. ✅ Filetype detection: `.rk` → `rock`

4. ✅ Treesitter registration: Rock parser registered with nvim-treesitter

## Quick Test

Create `test.rk`:
```haskell
struct Point
    x: Int
    y: Int

main = ->
    point = Point
        x: 10
        y: 20
    point.x + point.y
```

You should see:
- `struct` in keyword color
- `Point` in type color
- `x`, `y` in property/variable color
- `main` in function color
- Numbers in number color
- Comments in comment color

## Still Not Working?

Try these commands in Neovim:

```vim
" Force reload
:e!

" Check if parser attaches
:lua vim.treesitter.start()

" Manually enable highlighting
:TSBufEnable highlight

" Check for errors
:messages
```

## Debug Info

Run this in Neovim to see parser info:
```vim
:lua print(vim.inspect(vim.treesitter.language.get('rock')))
```

Should show:
- `parsers` table exists
- `highlights` table with your highlight captures
- No errors

If you see errors, send them to me and I'll help fix them!
