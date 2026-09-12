# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rock is a compiled programming language with Haskell-inspired syntax. The compiler is written in Rust and uses LLVM for code generation via the `inkwell` crate.

### Workspace Structure

This is a Cargo workspace with three crates:
- `lib/` (rock-lib) - Core compiler library containing all compilation phases
- `rockc/` - Simple CLI compiler wrapper
- `rock/` - More featureful CLI with formatting, run, test, and expand commands

## Build Commands

```bash
# Build all workspace members (requires LLVM 18)
cargo build --release

# Build with explicit LLVM 18 path (if system has multiple LLVM versions)
LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo build --release

# Build individual crates
cargo build -p rock-lib --release
cargo build -p rockc --release
cargo build -p rock --release

# Run the compiler (pass stdlib explicitly for stdlib-backed examples)
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca

# Consume a prebuilt crate artifact
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca

# Run with debug output (ast, hir, llvm, tokens, expanded, ast-full)
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print ast

# Run tests
cargo test -p rock-lib

# Run a specific test
cargo test -p rock-lib test_hello_world

# Build with optimization level and emit LLVM IR
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca -O 3 --emit-llvm --no-link

# Format Rock source files
cargo run -p rock -- --entry-file examples/hello.rk format
```

## Compilation Pipeline

The compiler (`lib/src/lib.rs:compile`) processes Rock code through these phases:

1. **Parsing** (`new_parser/`) - Converts source text to AST
2. **Macro Expansion** (`macro_expansion/`) - Expands macro invocations
3. **Resolution/Lowering** (`resolve/`) - Lowers AST to HIR, performs type checking, name resolution
4. **Borrow Checking** (`borrow_check/`) - Validates memory safety (currently warnings only)
5. **Monomorphization** (`mono/`) - Specializes generic functions
6. **Code Generation** (`codegen/`) - Generates LLVM IR and compiles to native code

## Key Architecture Components

### Parser (`new_parser/`)

The parser uses a custom combinator-based engine (`new_parser/engine/mod.rs`) that handles indentation-based parsing. Key trait: `Parsable::parse(tokens, parse_ctx)`.

- `items/` - Individual parsers for language constructs (expressions, statements, functions, structs, enums, etc.)
- `engine/` - Parser combinators (map, and, or, many, opt, delimited, etc.)
- Tests are co-located with parsers in `items/tests/` subdirectories

### AST (`ast/`)

- `tree.rs` - Core AST definitions (Program, Module, TopLevel, FunctionDecl, StructDecl, etc.)
- `visit.rs` - Visitor pattern for AST traversal
- `debug.rs` - Debug pretty-printing

### HIR (`hir/`)

High-level Intermediate Representation - desugared, typed AST.

- `mod.rs` - HIR definitions (HirProgram, HirFunction, HirStruct, HirExpr, etc.)
- All HIR nodes carry type information via the `Type` enum

### Type System (`types/`)

- `Type` enum - Core type representation (Int, Float, Bool, String, Array, Tuple, Struct, Enum, etc.)
- Type checking happens during resolution phase

### Diagnostics (`diagnostic.rs`)

- Uses `ariadne` crate for pretty error reporting
- `Diagnostics` struct collects errors with source location information

### Code Generation (`codegen/`)

- Uses `inkwell` (LLVM Rust bindings)
- Generates LLVM IR from HIR
- Handles function calls, operators, control flow, structs, enums, etc.

## Rock Language Syntax

Rock uses significant indentation (like Python/Python) and Haskell-inspired syntax:

```rock
// Function definition (arrow syntax)
main = ->
    println "Hello, World!"
    0

// Function with parameters (defined with commas)
add = x, y -> x + y

// Function calls: single arg uses space, multiple args use commas
result = add 1, 2        // multi-arg call
val = println "hello"    // single-arg call (no comma needed)

// Struct definition
struct Point
    x: I64
    y: I64

// Enum definition
enum Option
    Some I64
    None

// Pattern matching with match
match opt
    Option::Some val => val
    Option::None => 0

// Control flow (if as expression)
result = if x > 0
    x
else
    0 - x

// Loops
while condition
    body

for item in array
    body

// Trait system
trait Show
    @show = -> ""

impl Show for Point
    @show = -> "Point"
```

## Testing

Integration tests are in `lib/tests/integration.rs`. Tests compile Rock programs and verify their output.

Test helper functions:
- `compile_and_run(source)` - Compiles and runs inline Rock code
- `compile_example(name)` - Compiles and runs files from `examples/`
- `compile_should_fail(source, expected_error)` - Verifies compilation errors

Examples for testing are in `examples/` directory (*.rk files).

## File Extensions

- `.rk` - Rock source files
- `.rs` - Rust source files

## LLVM Requirements

The compiler uses LLVM 18 for code generation via the `inkwell` crate with dynamic linking.

**If your system has multiple LLVM versions installed**, you may need to specify the LLVM 18 path:

```bash
# Set environment variable for LLVM 18 prefix
export LLVM_SYS_180_PREFIX=/usr/lib/llvm18

# Or use it inline with cargo commands
LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo build
LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test
```

**To check which LLVM version is being used:**

```bash
# Check what LLVM libraries the binary is linked against
ldd target/debug/rockc | grep LLVM
```

The project is configured to use `inkwell` with the `llvm18-0-force-dynamic` feature in `Cargo.toml`.

## Important Notes

- The compiler defaults to looking for `./src/main.rk` as entry point (see `parser/items/program.rs:18`)
- Main function must return an integer (typically 0 for success)
- The active parser is in `parser/`
- When adding new language features, you typically need to modify:
  1. Parser (`parser/items/`)
  2. AST definitions (`ast/tree.rs`)
  3. Resolution/lowering (`lower/`)
  4. HIR definitions (`hir/`)
  5. Code generation (`codegen/`)
  6. Tests (`lib/tests/integration.rs` or `parser/items/tests/`)

## Debugging

Use debug flags to inspect intermediate representations:
- `--debug-print ast` - Print compact AST
- `--debug-print ast-full` - Print full AST (derived Debug)
- `--debug-print expanded` - Print AST after macro expansion
- `--debug-print hir` - Print HIR
- `--debug-print llvm` - Print generated LLVM IR
- `--debug-print tokens` - Print token stream

## Misc

Use auggie context retrival as much as you can
Always use agent teams , but we cannot have more than 2 agents at the same time because of rate limit (so 1 main + 2 agents max at the same time)

When running the tests, redirect the output in a file and grep what you need from this file. This is to avoid running the full testsuite multiple time in a row to extract different info/results. Also dont run multiple test suites in parallel, execute ONE then work multiple time with the output. dont use sleep to wait for the end of a run, use your tools. also maybe keep a timeout to 15mn so that it doesnt go to background

Read `MEMORY.md` for durable project-specific lessons before changing compiler architecture, repository workflows, or audit/roadmap docs. Keep new durable agent lessons there; use `bd` for issue tracking, not as a replacement for project memory.


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:7510c1e2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
