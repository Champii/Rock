# Installation Guide for Neovim

This guide will help you install tree-sitter-rock for syntax highlighting in Neovim.

## Prerequisites

1. Neovim (0.8+ recommended)
2. Node.js (for compiling the grammar)
3. nvim-treesitter plugin

## Quick Installation

### Option 1: Using Neovim (Recommended)

1. **Add the parser configuration to your Neovim config:**

   Copy the contents of `neovim-setup.lua` into your Neovim config file (usually `~/.config/nvim/init.lua` or `~/.config/nvim/lua/plugins/treesitter.lua`).

   Make sure to update the path in the `url` field to point to this directory.

2. **Install the parser in Neovim:**

   Open Neovim and run:
   ```vim
   :TSInstall rock
   ```

3. **Verify installation:**

   Open a `.rk` file and check that syntax highlighting is working. You can verify with:
   ```vim
   :TSInstallInfo rock
   :TSParserInfo
   ```

### Option 2: Manual Installation

1. **Compile the grammar manually:**

   ```bash
   cd tree-sitter-rock
   npm install
   npx tree-sitter generate
   npx tree-sitter build
   ```

2. **Copy the compiled parser:**

   ```bash
   mkdir -p ~/.local/share/nvim/site/parser/
   cp rock.so ~/.local/share/nvim/site/parser/
   ```

3. **Configure Neovim:**

   Add to your Neovim config:
   ```lua
   vim.filetype.add({
     extension = {
       rk = 'rock',
     },
   })
   ```

## Testing

After installation, create a test file `test.rk`:

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

Open it in Neovim and you should see:
- `struct` highlighted as a keyword
- `Point` highlighted as a type
- `x` and `y` highlighted as properties
- Comments (if any) in comment color
- Numbers highlighted appropriately

## Troubleshooting

### Parser not found error

Make sure the parser is in the correct directory. Run:
```vim
:echo stdpath('data')
```

The parser should be in `data/parser/rock.so`.

### Syntax highlighting not working

1. Check if the parser is installed:
   ```vim
   :TSInstallInfo
   ```

2. Check if treesitter is enabled:
   ```vim
   :TSBufEnable highlight
   ```

3. Verify filetype:
   ```vim
   :set filetype?
   ```
   Should show `filetype=rock`

### Need to regenerate after grammar changes

```bash
npx tree-sitter generate
npx tree-sitter build
```

Then reload the parser in Neovim:
```vim
:edit!
:TSBufReload
```

## Advanced Configuration

To customize highlighting colors, add this to your Neovim config:

```lua
vim.api.nvim_set_hl(0, "@keyword.rock", { link = "Keyword" })
vim.api.nvim_set_hl(0, "@function.rock", { link = "Function" })
vim.api.nvim_set_hl(0, "@type.rock", { link = "Type" })
vim.api.nvim_set_hl(0, "@variable.rock", { link = "Identifier" })
vim.api.nvim_set_hl(0, "@operator.rock", { link = "Operator" })
vim.api.nvim_set_hl(0, "@string.rock", { link = "String" })
vim.api.nvim_set_hl(0, "@number.rock", { link = "Number" })
vim.api.nvim_set_hl(0, "@comment.rock", { link = "Comment" })
```

## Updating

To update the grammar after changes:

```bash
cd tree-sitter-rock
git pull
npx tree-sitter generate
npx tree-sitter build
```

Then reload in Neovim or reinstall with `:TSInstall! rock`.
