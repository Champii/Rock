# Quick Start - Tree-sitter-rock for Neovim

## 5-Minute Setup

### 1. Install Dependencies
```bash
# Install tree-sitter CLI (if not already installed)
npm install -g tree-sitter

# Generate and compile the parser
cd /home/champii/prog/rust/new_lang/tree-sitter-rock
npm install
npx tree-sitter generate
npx tree-sitter build
```

### 2. Add to Neovim Config

Add this to your `~/.config/nvim/init.lua`:

```lua
-- Filetype detection
vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

-- Treesitter parser config
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()
parser_config.rock = {
  install_info = {
    url = "/home/champii/prog/rust/new_lang/tree-sitter-rock",
    files = {"src/parser.c"},
  },
  filetype = "rock",
}

-- Ensure rock is highlighted
require'nvim-treesitter.configs'.setup({
  highlight = {
    enable = true,
  },
})
```

### 3. Test It

Open any `.rk` file in Neovim:
```bash
nvim examples/main_test.rk
```

You should see beautiful syntax highlighting!

## What's Highlighted

- **Keywords**: `struct`, `enum`, `trait`, `impl`, `if`, `then`, `else`, `for`, `while`, `match`, `return`, etc.
- **Types**: `Point`, `Result`, `Int`, `String`, etc.
- **Functions**: Function definitions and calls
- **Literals**: Numbers, strings, booleans, characters
- **Comments**: `// line comments`
- **Operators**: `->`, `=>`, `::`, `.`, `=`, etc.
- **Macros**: `macro`, `%` invocation
- **Pattern matching**: Match arms and patterns

## Verifying Installation

In Neovim, run:
```vim
:checkhealth nvim-treesitter
```

Look for `rock` in the list of installed parsers.

## Example File

```haskell
// This is a comment
struct Point
    x: Int
    y: Int

enum Result T, E
    Ok T
    Err E

main = ->
    point = Point
        x: 10
        y: 20

    if point.x > 0
    then point.x
    else 0

    result = Ok 42
```

## Troubleshooting

**No syntax highlighting?**
1. Check filetype: `:set filetype?` (should be `rock`)
2. Enable treesitter: `:TSBufEnable highlight`
3. Check parser: `:TSParserInfo`

**Need to reinstall?**
```vim
:TSUninstall rock
:TSInstall rock
```

## Files Created

- `grammar.js` - Tree-sitter grammar definition
- `queries/highlights.scm` - Syntax highlighting rules
- `queries/indents.scm` - Indentation rules
- `queries/locals.scm` - Variable scope tracking
- `neovim-setup.lua` - Neovim configuration
- `test/example.rk` - Example Rock code

## Next Steps

- Customize colors in your Neovim theme
- Add LSP support for your compiler
- Configure autocomplete with nvim-cmp
