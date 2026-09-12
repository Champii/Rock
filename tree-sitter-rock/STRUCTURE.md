# Tree-sitter-rock File Structure

```
tree-sitter-rock/
├── grammar.js              # Main grammar definition for Rock language
├── package.json            # NPM package configuration
├── Makefile               # Build automation
├── README.md              # General documentation
├── QUICKSTART.md          # Quick installation guide
├── INSTALL.md             # Detailed installation instructions
├── neovim-setup.lua       # Neovim configuration snippet
├── .gitignore             # Git ignore rules
│
├── queries/               # Tree-sitter query files
│   ├── highlights.scm     # Syntax highlighting queries
│   ├── indents.scm        # Indentation queries
│   └── locals.scm         # Local variable/scope queries
│
└── test/
    └── example.rk         # Example Rock file for testing
```

## File Descriptions

### Core Files

**grammar.js**
- Defines the complete syntax tree structure for Rock
- Written in JavaScript for tree-sitter
- Contains rules for all language constructs:
  - Declarations (functions, structs, enums, traits)
  - Statements and expressions
  - Patterns and matching
  - Macros
  - Types and type annotations

### Query Files

**queries/highlights.scm**
- Maps syntax nodes to Neovim highlight groups
- Defines what gets colored and how
- Covers:
  - Keywords, operators, punctuation
  - Literals (numbers, strings, booleans)
  - Identifiers and types
  - Functions and methods
  - Comments

**queries/indents.scm**
- Defines indentation rules for automatic indentation
- Handles:
  - Function bodies
  - Struct/enum fields
  - Match arms
  - If/then/else blocks
  - Loops

**queries/locals.scm**
- Tracks variable scopes and definitions
- Enables features like:
  - Go-to-definition
  - Local variable renaming
  - Scope awareness

### Configuration Files

**package.json**
- NPM package metadata
- Defines file extensions (.rk)
- Specifies which queries to use

**neovim-setup.lua**
- Ready-to-use Neovim configuration
- Just copy into your Neovim config
- Sets up filetype detection and parser

### Documentation

**QUICKSTART.md**
- 5-minute setup guide
- Essential commands
- Quick troubleshooting

**INSTALL.md**
- Detailed installation instructions
- Multiple installation methods
- Advanced configuration

**README.md**
- Project overview
- Features list
- Development instructions

## Generated Files (after build)

After running `tree-sitter generate` and `tree-sitter build`:

```
tree-sitter-rock/
├── src/
│   ├── parser.c          # Generated C parser
│   └── tree_sitter/      # tree-sitter headers
└── build/
    └── rock.so           # Compiled parser binary
```

## Usage in Neovim

1. Parser is loaded automatically for `.rk` files
2. Syntax highlighting via `highlights.scm`
3. Indentation via `indents.scm`
4. Text objects and motions via nvim-treesitter-textobjects

## Development Workflow

1. Edit `grammar.js` to modify syntax rules
2. Run `make` or `tree-sitter generate && tree-sitter build`
3. Test with `tree-sitter test` or open a `.rk` file in Neovim
4. Edit query files to adjust highlighting/indentation

## Customization

### Colors
Modify highlight groups in Neovim:
```lua
vim.api.nvim_set_hl(0, "@function.rock", { link = "Function" })
```

### Indentation
Edit `queries/indents.scm` to change indentation behavior

### Highlighting
Edit `queries/highlights.scm` to adjust what gets highlighted
