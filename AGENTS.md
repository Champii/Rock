# AGENTS.md

## Purpose
- This repo implements the Rock language compiler in Rust.
- This file is for agentic coding tools working in this repository.
- Prefer repo-specific facts over generic Rust assumptions.
- Read `MEMORY.md` for durable project-specific lessons before changing compiler architecture, repository workflows, or audit/roadmap docs.
- No Cursor rules were found in `.cursor/rules/` or `.cursorrules`.
- No Copilot instructions were found in `.github/copilot-instructions.md`.

## Workspace
- `lib/` (`rock-lib`) contains the compiler implementation.
- `rockc/` is the thin compiler CLI and is the safest default for compile/run workflows.
- `rock/` is a richer CLI with formatting and macro expansion commands.
- `tree-sitter-rock/` is a separate grammar project, not a Cargo workspace member.
- Workspace members are `lib`, `rock`, and `rockc`.

## Environment
- LLVM 18 is required for code generation via `inkwell`.
- The workspace uses `inkwell` with `llvm18-0-force-dynamic`.
- The default Rock entry point is `./src/main.rk`.
- Rock `main` functions should return an integer, usually `0`.

```bash
cargo build --release
cargo test -p rock-lib
```

## Build And Run
- Common build and run commands:
- Pass `--extern-artifact stdlib=build/stdlib.rkca` to `rockc` when compiling stdlib-backed examples directly.
- `rockc` accepts external dependencies only as prebuilt artifacts via `--extern-artifact name=path`.

```bash
cargo build --release
cargo build -p rock-lib --release
cargo build -p rockc --release
cargo build -p rock --release
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print ast
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print ast-full
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print expanded
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print hir
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print mir
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print llvm
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca --debug-print tokens
cargo run -p rockc -- --entry-file examples/hello.rk --extern-artifact stdlib=build/stdlib.rkca -O 3 --emit-llvm --no-link
```

## Formatting And Linting
- No repo-specific `rustfmt.toml`, `clippy.toml`, or CI lint workflow was found.
- Use standard Rust hygiene for Rust code changes.
- Format Rock source files through the `rock` CLI.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo run -p rock -- --entry-file examples/hello.rk format
```

## Testing
- Main documented test suite:

```bash
cargo test -p rock-lib
```

- Useful focused commands, especially for a single test:

```bash
cargo test -p rock-lib test_hello_world
cargo test -p rock-lib --test integration test_hello_world -- --exact
cargo test -p rock-lib test_multiple_imports -- --exact
cargo test -p rock-lib test_hello_world -- --exact --nocapture
```

- Integration tests live in `lib/tests/integration.rs`.
- Parser and parser-engine tests are heavily co-located under `lib/src/parser/**/tests/`.
- Prefer the smallest relevant test command for the code you changed.
- When documenting or running `cargo test ... -- --exact`, use the fully qualified test path if a short name is ambiguous or fails to match.
- Do not run multiple test suites in parallel.
- For longer runs, save output once and inspect the log instead of rerunning immediately.

```bash
cargo test -p rock-lib > /tmp/rock-lib-tests.log 2>&1
```

## tree-sitter-rock
- If you work in `tree-sitter-rock/`, use that subproject's own commands.
- Do not confuse grammar changes with compiler-workspace changes.

```bash
tree-sitter generate
tree-sitter build
tree-sitter test
make
```

## Architecture
- Keep compiler logic in `lib/`.
- Keep CLI glue in `rockc/` and `rock/`.
- The active parser is under `lib/src/parser/`.
- The compiler pipeline in practice is parser -> macro expansion -> lowering / resolution -> inference -> MIR / borrow checking -> monomorphization -> codegen.
- Diagnostics are structured and span-aware; reuse existing diagnostic types instead of inventing ad hoc reporting.
- `rockc` converts CLI args into `rock_lib::Config` and delegates to `rock_lib::compile`.
- `rock` contains utility workflows, but some subcommands are still incomplete.
- Operator syntax is dynamically defined by the current program and its explicit dependencies. Compiler code must not hardcode stdlib operator symbols, trait names, operator existence, or operator meanings. If the shipped stdlib wants `+`, `-`, `*`, comparisons, bitwise operators, or unary operators on primitive types, those meanings must live in stdlib impl bodies that call explicit intrinsics such as `~I64Add`, not in parser/lowerer/compiler fallback logic.

## When Adding Language Features
- Expect to touch more than one compiler phase.
- Common update path is `lib/src/parser/`, `lib/src/ast/tree.rs`, `lib/src/lower/`, `lib/src/hir/`, `lib/src/codegen/`, nearby parser tests, and `lib/tests/integration.rs`.
- Do not stop at parser changes if the feature affects typing, lowering, or code generation.
- Add a local parser test for syntax behavior and an integration test for user-visible behavior when possible.
- Keep new logic in the phase that already owns that responsibility.

## Imports
- Match the existing grouping style.
- Put `std` imports first, external crates second, and `crate::...` imports after that.
- Separate major import groups with a blank line.
- Use nested brace imports when they improve clarity.
- Broad `*` imports appear mainly in tests; prefer explicit imports in production code.

## Formatting
- Follow standard Rustfmt-style formatting.
- Use 4-space indentation.
- Keep trailing commas in multiline structs, enums, matches, and calls.
- Split dense expressions across lines when they become hard to scan.
- Use blank lines to separate distinct phases or logical blocks in longer functions.
- Keep edits small and local when possible.

## Rock Syntax
- Rock function-call arguments are comma-separated. Write calls like `max 5, 7` and `~I64Lt a, b`, not space-separated forms like `max 5 7`.

## Documentation
- For beginner-guide and README work, write in a RustBook-like style: detailed, explicit, and approachable for readers with some programming experience.
- Every explained Rock feature should have concrete code, and every Rock code fence should be locally understandable with user-defined types, functions, and variables declared in the same fence unless the prompt explicitly allows a fragment convention.
- Exclude compiler-internal syntax such as `lang` markers from user-facing guides unless the task is explicitly about compiler internals.
- User-facing docs should prefer the `rock` CLI for application workflows. Use `rockc` only for compiler-contributor or artifact-level workflows, and explain why when it appears.
- Keep README links pointed at this repository's published docs (`Champii/Rock` / `champii.github.io/Rock`) rather than stale `new_lang` URLs, and remove outdated roadmaps instead of preserving them.

## Types And Data Modeling
- Rust edition is 2021 across the workspace.
- Prefer enums and structs for compiler data over loose tuples or maps.
- Use explicit types for compiler state, AST, HIR, MIR, and diagnostics.
- `PathBuf`, `String`, and `Vec<_>` are common owned types in stored state.
- Derive standard traits such as `Debug`, `Clone`, and `PartialEq` when appropriate.
- Use type aliases sparingly, only when they clarify a recurring signature.

## Naming
- Use `PascalCase` for types, enums, and traits.
- Use `snake_case` for functions, variables, modules, and tests.
- Test names are descriptive and often start with `test_`.
- Match existing acronym casing like `Ast`, `AstFull`, `Hir`, `Mir`, and `Llvm`.

## Error Handling
- In library code, prefer structured compiler errors over opaque boxed errors.
- Reuse existing diagnostics types and preserve span information.
- Prefer concrete error types such as `Diagnostics`, `ParseError`, and phase-specific errors.
- Avoid introducing `anyhow`, `eyre`, or `thiserror` unless the repo adopts them first.
- `unwrap`, `expect`, and `panic!` are acceptable in tests and hard invariants.
- Avoid adding new `unwrap` or `expect` calls in user-facing compilation paths unless the invariant is guaranteed.
- CLI entrypoints usually report errors and exit with status `1`.

## Comments And Docs
- Use `//!` and `///` docs for modules and public APIs when the concept needs explanation.
- Use regular `//` comments for intent, invariants, parser edge cases, and phase boundaries.
- Prefer comments that explain why, not comments that restate the code.
- Keep comments brief and technical.

## CLI Caveats
- Prefer `rockc` for reliable compiler execution.
- In `rock/src/main.rs`, `Run` and `Test` subcommands are still `todo!()`.
- `rock` is useful for formatting and macro expansion, but not every advertised command is complete.

## Agent Workflow
- Parallelize code reading when useful, but keep test execution serialized.
- Verify changes with the smallest relevant command first.
- Current user/session instructions override the beads session-completion block. Do not commit, stage, push, amend, or otherwise touch VCS state unless the current user prompt explicitly asks for it.
- Do not inspect or modify `.sisyphus/` unless the current user prompt explicitly asks; recent subagent tasks repeatedly prohibited touching it.
- Preserve existing module boundaries and do not move compiler logic into CLI crates.
- If a change affects parsing behavior, inspect nearby parser tests before introducing new helpers or abstractions.
- When updating plans, audits, or checklist docs, re-read the changed summary/checklist lines to avoid stale wording or overclaiming completed scope.
- This project is in a prototyping phase: do not preserve deprecated code paths, backward-compatibility shims, or transitional aliases unless explicitly requested.
- Treat `stdlib` like any other external crate. Do not add compiler-owned stdlib loading, sysroot discovery, builtin stdlib registration, or unqualified stdlib injection.
- The only stdlib-specific handling allowed in the compiler is automatic import/injection of the stdlib prelude, and only when `stdlib` was explicitly passed to `rockc`.

<!-- BEGIN BEADS INTEGRATION v:1 profile:full hash:0a1bbe8a -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Quality
- Use `--acceptance` and `--design` fields when creating issues
- Use `--validate` to check description completeness

### Lifecycle
- `bd defer <id>` / `bd supersede <id>` for issue management
- `bd stale` / `bd orphans` / `bd lint` for hygiene
- `bd human <id>` to flag for human decisions
- `bd formula list` / `bd mol pour <name>` for structured workflows

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- No manual export/import needed!

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

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
