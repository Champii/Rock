# Why You're Not Seeing Colors (and how to fix it)

## The Issue

Your `:checkhealth nvim-treesitter` shows:
```
rock             H✓ Lx Ix . .   <- H has checkmark but no colors!
```

This means:
- **H✓** = Highlighting queries loaded successfully
- **Lx** = Locals query has an error (not important for colors)
- **Ix** = Indent query has an error (not important for colors)
- **.** = Folding and Jump not supported (normal)

**But you see no colors** because Neovim doesn't know which colors to use for Rock!

## Quick Fix

Add this to your `~/.config/nvim/init.lua`:

```lua
-- Rock language support
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

vim.treesitter.language.register('rock', 'rk')

require'nvim-treesitter.configs'.setup({
  highlight = {
    enable = true,
  },
})
```

Then **restart Neovim completely** and open a `.rk` file.

## If Still No Colors

Open a `.rk` file and run these commands:

```vim
" Check filetype
:set filetype?
" Should say: filetype=rock

" Enable treesitter manually
:TSBufEnable highlight

" Check if parser is attached
:lua print(vim.treesitter.get_parser(0, 'rock'):lang())
" Should print: rock

" Check what captures exist
:lua print(vim.inspect(vim.treesitter.highlighter.active[1]))
```

## Alternative: Force Highlight Groups

If colors still don't show, add this to your init.lua:

```lua
-- Define Rock highlight groups
vim.api.nvim_create_autocmd({"BufRead", "BufNewFile"}, {
  pattern = "*.rk",
  callback = function()
    -- Keywords
    vim.cmd([[
      syntax keyword rockKeyword struct enum trait impl if then else for in while loop macro return continue break infix mod extern match unsafe type pub
      syntax keyword rockConditional if then else
      syntax keyword rockRepeat for in while loop
      syntax keyword rockOperator -> => :: =
      highlight def link rockKeyword Keyword
      highlight def link rockConditional Conditional
      highlight def link rockRepeat Repeat
      highlight def link rockOperator Operator
    ]])
  end,
})
```

## Debug Steps

1. **Verify files exist:**
   ```bash
   ls -lh ~/.local/share/nvim/site/parser/rock.so
   ls -lh ~/.local/share/nvim/site/queries/rock/highlights.scm
   ```

2. **Check Neovim version:**
   ```vim
   :version
   ```
   Need 0.8+ for treesitter

3. **Check for errors:**
   ```vim
   :messages
   ```

4. **Test parser directly:**
   ```vim
   :lua vim.treesitter.inspect_tree()
   ```
   Should show the syntax tree

## Most Common Issues

### Issue: "parser not found"
**Fix:** Make sure `rock.so` is in `~/.local/share/nvim/site/parser/`

### Issue: "no colors"
**Fix:** Add the config above to your init.lua and restart Neovim

### Issue: "filetype is not rock"
**Fix:**
```vim
:set filetype=rock
```
Or add the vim.filetype.add config

### Issue: "queries have errors"
**Fix:** The Ix and Lx errors are fine - they don't affect colors. Only H✓ matters for syntax highlighting.

## What You Should See

Once it's working, open `examples/main_test.rk` and you should see:
- `struct` in keyword color (often blue/purple)
- `Point` in type color (often yellow/cyan)
- `main` in function color (often blue)
- `10` in number color (often different)
- `"hello"` in string color (often green)
- `// comment` in comment color (often grey)
- `->` in operator color (often red)

Let me know what you see after adding the config!
