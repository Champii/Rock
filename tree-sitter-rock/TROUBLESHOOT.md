# Quick Fix for Query Errors

## Problem:
Checkhealth showed: "query error at 16:4 invalid type 'continue'"

## Solution:
Fixed and reinstalled the query file!

## Now restart Neovim and test:

1. **Quit Neovim completely**: `:qa`

2. **Restart and open a .rk file**:
   ```bash
   nvim examples/main_test.rk
   ```

3. **Check health again**:
   ```vim
   :checkhealth nvim-treesitter
   ```

You should now see:
```
rock             ✓  ✓  ✓  ✓
```
With green checkmarks!

## What was fixed:

The query was trying to match nodes like `(continue_expression)` and `(break_expression)` which are just keywords in our grammar. I updated the queries to match the actual grammar structure.

## Verify syntax highlighting:

You should now see:
- ✅ Keywords in color (struct, enum, if, then, else, etc.)
- ✅ Types highlighted (Point, Int, String, etc.)
- ✅ Functions highlighted (main, etc.)
- ✅ Numbers and strings highlighted
- ✅ Comments in comment color

If it's still not working, try:
```vim
:TSBufEnable highlight
```

And check for errors:
```vim
:messages
```
