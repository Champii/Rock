# Source Database Loader Design

## Goal

Roadmap Task 17 introduces a long-term source and module loader boundary for the Rock compiler.

After this task, filesystem IO, source text ownership, sibling module discovery, canonical path tracking, parsed module caching, and source-backed crate module graph loading should be owned by a dedicated source database. Parser code should parse provided text/tokens into AST only. Lowering and crate-system code should consume loaded module data instead of reparsing files or constructing ad hoc parser configs.

## Current State

Source and module ownership is currently split across phases:

- `lib/src/parser/mod.rs` reads files in `parse_module` before lexing/parsing.
- `lib/src/parser/items/module.rs` performs sibling module lookup during grammar parsing and can `panic!`/`unwrap()` when an empty `mod name` declaration has no sibling file.
- `lib/src/parser/engine/mod.rs` exposes `sibling_module_filepath`, coupling parser context to filesystem paths.
- `lib/src/lower/program.rs` resolves modules, checks file existence, caches modules, detects cycles, and reparses files with manually constructed `Config` values.
- `lib/src/crate_system/module_tree.rs` reparses source modules and builds a separate file cache for source-backed crates.
- `lib/src/lib.rs` derives product source fingerprints by walking AST module file paths rather than consuming a loader-owned loaded-file list.

This makes parser behavior depend on the filesystem, duplicates module loading logic, hides IO failures behind panics or string errors, and forces lowering to own frontend source policy.

## Scope

In scope:

- Add a new source loader module, likely `lib/src/source_loader/`.
- Introduce a `SourceDatabase` that owns source text, canonical paths, parsed modules, module graph state, and loaded-file order.
- Remove direct file-reading parser APIs from normal compiler flow. Parser entry points should parse source text/tokens, not paths.
- Remove parser-owned sibling module loading from the `mod` grammar. Empty `mod name` declarations remain AST declarations for the loader/lowering boundary.
- Route current-crate module loading through `SourceDatabase` before collection/lowering.
- Route source-backed crate loading in `CrateContext::load_crate_from_dir` and `crate_system::module_tree` through the same loader.
- Replace lowerer-owned reparsing and ad hoc `Config` construction with access to loaded modules from the source database/module graph.
- Make product source fingerprints consume the loader's loaded file list.
- Add structured loader diagnostics for missing modules, IO errors, parse errors, and circular module loads.
- Update tests to use loader APIs for file-backed parsing and parser text APIs for parser-only behavior.

Out of scope:

- Do not introduce source/file IDs or migrate `Span.file_path` in this task.
- Do not change product artifact schema.
- Do not reintroduce compiler-owned stdlib/sysroot discovery.
- Do not revive source-backed external dependencies as a supported downstream compilation path. Existing source-crate paths may still be used by tests/internal context setup, but product artifacts remain the external dependency boundary.
- Do not refactor macro expansion or formatter trivia beyond what is needed to consume parser text APIs.

## Design

### Source Database

Add a source database that is the only owner of file IO and parsed module caching.

Suggested core types:

```rust
pub struct SourceDatabase {
    files: BTreeMap<PathBuf, SourceFile>,
    modules: BTreeMap<PathBuf, ast::Module>,
    load_states: BTreeMap<PathBuf, ModuleLoadState>,
    loaded_files: Vec<PathBuf>,
}

pub struct SourceFile {
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub display_path: PathBuf,
    pub text: String,
}

pub struct ModuleGraph {
    pub root_path: PathBuf,
    pub root_module: ast::Module,
    pub modules: BTreeMap<String, LoadedModule>,
    pub loaded_files: Vec<PathBuf>,
}

pub struct LoadedModule {
    pub qualified_name: String,
    pub path: PathBuf,
    pub module: ast::Module,
}
```

Exact fields can be adjusted during implementation, but ownership should stay clear: source text, canonical paths, parsed modules, and loaded-file order live in source loading, not parser or lowerer.

`SourceDatabase` should provide operations similar to:

```rust
impl SourceDatabase {
    pub fn load_entry(&mut self, path: PathBuf, config: &Config) -> Result<ModuleGraph, SourceLoadErrors>;
    pub fn load_source_crate(&mut self, lib_path: PathBuf, crate_name: &str, config: &Config) -> Result<ModuleGraph, SourceLoadErrors>;
    pub fn module_for_path(&self, path: &Path) -> Option<&ast::Module>;
    pub fn loaded_files(&self) -> &[PathBuf];
}
```

The loader should resolve `mod name` declarations to the current Rock sibling-module convention:

1. `name.rk` next to the containing file.
2. `name/mod.rk` under the containing directory.

It should canonicalize paths when possible and use canonical paths as cache keys. If canonicalization fails for a missing file, diagnostics should still include the searched display paths.

### Parser Boundary

Parser code should parse provided source, not read files. The main parser file API should be replaced by source/text APIs such as:

```rust
pub fn parse_source(path: PathBuf, source: &str, config: &Config) -> Result<ast::Module, ParseError>;
pub fn parse_string(input: &str, config: &Config) -> Result<ast::Program, ParseError>;
```

`parse_source` lexes `source` using `path` for spans and parses a module. It does not inspect sibling files.

The grammar rule for `mod name` should stop calling `parse_module`, `sibling_module_filepath`, `unwrap`, or `panic!`. Empty module declarations should remain represented in AST so the loader/lowerer boundary can resolve them. If the current AST representation cannot distinguish `mod name` from `mod name` with an empty inline block well enough, add the smallest AST-side marker needed to preserve that distinction.

No direct parser file-reading compatibility wrapper should remain for normal compiler or test use. Tests that need file-backed parsing should instantiate `SourceDatabase` and call loader APIs.

### Lowering Boundary

Lowering should receive loaded modules instead of loading them. The current responsibilities in `Lowerer` that should move out are:

- Resolving module files relative to `current_module_path`.
- Checking for file existence.
- Detecting circular module imports by path.
- Constructing temporary parser `Config` values.
- Calling parser APIs to reparse module files.

Lowering may still maintain semantic module context such as current qualified module prefix, scope aliases, and body lowering order. That is not source loading. It should ask a loaded module graph for module ASTs by qualified name/path.

The loaded graph should preserve existing behavior for:

- `mod name` declarations in current modules.
- Inline nested modules.
- Glob imports from loaded modules.
- Module-local aliases and exports.
- Product source fingerprints.

### Crate-System Boundary

`CrateContext::load_crate_from_dir` and `crate_system::module_tree` should use the source database for source-backed crate module graphs. `module_tree` should stop parsing files directly and should instead transform a loaded module graph into crate-system metadata/cache structures.

This task does not make source-backed dependencies a supported downstream dependency mode. Source crates that remain in `CrateContext` should still produce the existing source-backed dependency error in phases where product artifacts are required.

### Product Fingerprints

Product source fingerprints should use the source database loaded-file list instead of recursively walking AST modules for `filepath` values. This keeps fingerprinting tied to the loader's view of which files were loaded and avoids duplicate AST traversal policy in `lib/src/lib.rs`.

Loaded file paths should remain stable across checkout roots by preserving the existing relative-path behavior where possible.

## Error Handling

Introduce structured loader errors, for example:

```rust
pub enum SourceLoadError {
    Io { path: PathBuf, message: String },
    MissingModule { module: String, searched: Vec<PathBuf>, span: Option<Span> },
    Parse { path: PathBuf, error: ParseError },
    CircularModule { path: PathBuf, stack: Vec<PathBuf> },
}
```

The final shape can follow existing diagnostics conventions, but the behavior should be explicit:

- Missing sibling modules report both searched paths and, when available, the `mod` declaration span.
- IO errors include the path and OS error text.
- Parse errors preserve the parser error and source path.
- Circular loads report the repeated path and useful load-stack context.
- Parser grammar must not panic or unwrap for module IO.
- Lowering should convert loader errors into existing `ResolveError`/diagnostic structures without losing spans when spans are available.

## Testing

Use TDD for each behavior change.

Focused tests should cover:

- Parser text APIs parse modules without reading sibling files.
- Empty `mod name` grammar no longer loads files or panics when the file is absent.
- `SourceDatabase` loads an entry file and records source text/canonical path/module AST.
- Sibling module resolution prefers `name.rk` and supports `name/mod.rk`.
- Loaded modules are cached and not reparsed for repeated imports/globs.
- Circular module loads produce structured loader errors.
- Missing modules report both searched paths.
- Lowering consumes loaded modules without reparsing through parser file APIs.
- Source-backed crate loading uses the same source database/module graph.
- Product source fingerprints include loaded modules and ignore unloaded `.rk` files as today.

Regression/integration tests should cover:

- Existing module tests in `lib/tests/integration.rs` still pass.
- Existing source fingerprint tests still pass.
- Existing product artifact tests still pass.
- Full `cargo test -p rock-lib` passes.

Useful verification commands:

```bash
cargo fmt --all --check
cargo test -p rock-lib parser::items::tests
cargo test -p rock-lib source_loader
cargo test -p rock-lib module
cargo test -p rock-lib product_source_fingerprint
cargo test -p rock-lib --test integration test_modules -- --exact
cargo test -p rock-lib --test integration test_module_glob_import_and_export -- --exact
cargo test -p rock-lib
git diff --check
```

## Risks And Mitigations

- **Large blast radius:** Parser, lowerer, crate-system, product fingerprints, and tests all touch module loading. Mitigate by landing in small TDD steps with compatibility at the data boundary, not compatibility file IO wrappers.
- **Accidental source-ID migration:** Keep `Span.file_path: PathBuf` unchanged. Source IDs can be a later task.
- **Behavior changes in module resolution:** Preserve the current `name.rk` then `name/mod.rk` order and add tests.
- **Duplicate parse/cache bugs:** Use canonical paths as cache keys and loaded states to avoid reparse loops.
- **Source-backed dependency confusion:** Keep product artifacts as the external dependency boundary; this task only cleans source loading ownership.

## Success Criteria

- Parser module parsing has no filesystem IO path.
- The grammar for `mod name` does not load sibling files, unwrap, or panic.
- Current-crate source loading and module graph construction go through `SourceDatabase`.
- Lowering consumes loaded modules instead of reparsing files with ad hoc configs.
- Crate-system source module graph/cache construction uses the same loader.
- Product source fingerprints are based on the loader's loaded-file list.
- Missing/circular/parse module load failures are structured and tested.
- `cargo test -p rock-lib` passes.
