# tree-sitter-rock

Tree-sitter grammar for the Rock programming language.

## Installation

### For Neovim

1. Clone the Rock repository:
```bash
git clone https://github.com/Rock-lang-org/Rock.git ~/Rock
```

2. Compile the grammar:
```bash
cd ~/Rock/tree-sitter-rock
make
```

3. Add to your Neovim config (init.lua):
```lua
vim.opt.runtimepath:append(vim.fn.expand("~/Rock/tree-sitter-rock"))

vim.filetype.add({
  extension = {
    rk = 'rock',
  },
})

local parser_config = require("nvim-treesitter.parsers").get_parser_configs()
parser_config.rock = {
  install_info = {
    url = vim.fn.expand("~/Rock/tree-sitter-rock"),
    files = {"src/parser.c"},
    require_generate = true,
  },
  filetype = "rock",
}
```

4. Install the parser:
```vim
:TSInstall rock
```

### Manual Installation

```bash
# Install tree-sitter CLI
npm install -g tree-sitter

# Generate parser
tree-sitter generate

# Build
tree-sitter build

# Test
tree-sitter test
```

## Usage

Once installed, Tree-sitter will automatically provide syntax highlighting for `.rk` files in Neovim with nvim-treesitter.

## Features

- Full syntax highlighting for Rock language
- Support for:
  - Functions and lambdas
  - Structs and enums
  - Traits and implementations
  - Pattern matching
  - Macros
  - Type annotations
  - Operators and punctuation

## Development

To modify the grammar:

1. Edit `grammar.js`
2. Regenerate: `tree-sitter generate`
3. Test: `tree-sitter test`

## License

MIT
