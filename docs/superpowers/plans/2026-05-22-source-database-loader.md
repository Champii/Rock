# Source Database Loader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Rock source IO, sibling module discovery, parsed module caching, module graph ownership, and source-backed crate loading into a dedicated `SourceDatabase` boundary.

**Architecture:** Add `lib/src/source_loader/` as the only file-backed source/module loader. Parser entry points parse provided text, while compile, collect, lower, crate-system, and product fingerprinting consume loader-owned `ModuleGraph` data and legacy cache views derived from that graph until later identity work replaces path-keyed plumbing.

**Tech Stack:** Rust 2021, existing `rock-lib` parser/lexer/AST/diagnostic infrastructure, `cargo test -p rock-lib`, `cargo fmt --all --check`.

**2026-05-25 completion update:** The scoped source database loader plan is complete for filesystem-backed current-crate and source-crate loading. Parser file-reading APIs were removed; `SourceDatabase` owns source reads, canonical path keys, sibling resolution, parsed-module caching, graph construction, loaded-file ordering, and structured loader errors; compile, collect, lower, crate-system source loading, `rockc` format/expand, and product fingerprints consume loader-owned graph/cache views. Focused verification passed for parser item tests, `source_loader`, module-related tests, product fingerprints, `test_modules`, and `test_module_glob_import_and_export`. Final verification: `cargo fmt --all && cargo fmt --all --check && git diff --check && cargo test -p rock-lib > /tmp/rock-lib-task17-final.log 2>&1`; the log shows `1251` unit tests passed with `1` ignored, `276` integration tests passed, parser integration passed, and doctests passed.

---

## File Structure

- Create: `lib/src/source_loader/mod.rs`
  - Owns source file reads, canonical cache keys, sibling module resolution, parsed module cache, load state, loaded-file ordering, `ModuleGraph`, and structured `SourceLoadError` conversion.
- Modify: `lib/src/lib.rs`
  - Exposes `source_loader`, routes compile entry parsing through `SourceDatabase`, converts loader errors to diagnostics, and fingerprints products from `ModuleGraph::loaded_files()`.
- Modify: `lib/src/parser/mod.rs`
  - Adds `parse_source(path, source, config)` and keeps `parse_string(input, config)` as the parser-only string API.
  - Removes file-reading parser APIs after all call sites are migrated.
- Modify: `lib/src/parser/items/module.rs`
  - Removes grammar-owned sibling module loading and parses inline modules as AST only.
- Modify: `lib/src/parser/engine/mod.rs`
  - Removes filesystem-coupled `ParseCtx::sibling_module_filepath` after parser module grammar no longer uses it.
- Modify: `lib/src/collect/mod.rs`
  - Adds collection entry points that accept a `ModuleGraph`, seeds collect-owned cache/path views from it, and stops declaration discovery from reparsing source files.
- Modify: `lib/src/collect/context.rs`
  - Replaces collect-owned source reparsing with lookups into the seeded graph/cache and preserves current semantic prefix/import/export behavior.
- Modify: `lib/src/collect/item_index.rs`
  - Builds source module maps from `ModuleGraph` or graph-derived loaded paths/cache instead of reconstructing file paths independently.
- Modify: `lib/src/lower/program.rs`
  - Removes lowerer-owned parser calls and body/trait-default reparsing; lower uses modules already cached by collection/source loading.
- Modify: `lib/src/lower/mod.rs`
  - Keeps path/cache fields only as graph-derived semantic state, not as source loading ownership.
- Modify: `lib/src/crate_system/context.rs`
  - Routes `CrateContext::load_crate_from_dir` through `SourceDatabase::load_source_crate`.
- Modify: `lib/src/crate_system/module_tree.rs`
  - Transforms loaded module graph data into `ModuleTree`/file cache metadata and deletes parser-backed helpers.
- Modify: `lib/src/crate_system/extern_store.rs`
  - Stores graph-derived source crate caches and builds module trees from graph data.
- Modify: `rockc/src/main.rs`
  - Updates `format` and `expand` support code to parse file-backed sources through `SourceDatabase` instead of `parser::parse`.
- Test: `lib/src/source_loader/mod.rs`
  - Unit tests for loader-owned IO, sibling resolution, caching, missing modules, circular modules, and parse errors.
- Test: `lib/src/parser/mod.rs`
  - Parser text-boundary tests proving parser APIs do not read sibling files. Existing parser item tests remain covered through `lib/src/parser/items/tests/mod.rs`.
- Test: `lib/src/lib.rs`
  - Product fingerprint tests updated to assert loader loaded-file behavior.
- Test: `lib/tests/integration.rs`
  - User-visible regressions for `name/mod.rk`, missing modules, circular modules, repeated module references, and product fingerprints.

## Task 1: Parser Text Boundary

**Files:**
- Modify: `lib/src/parser/mod.rs`
- Modify: `lib/src/parser/items/module.rs`
- Modify: `lib/src/parser/engine/mod.rs`

- [ ] **Step 1: Write parser text API tests**

Add this test module to the bottom of `lib/src/parser/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::ast::TopLevel;
    use crate::parser::{parse_source, parse_string};
    use crate::Config;

    #[test]
    fn parse_source_sets_module_filepath_without_reading_sibling_modules() {
        let path = PathBuf::from("/virtual/app/main.rk");
        let module = parse_source(path.clone(), "mod missing\nmain = -> 0\n", &Config::default())
            .expect("source text should parse without filesystem module lookup");

        assert_eq!(module.filepath, Some(path));
        match &module.top_levels[0] {
            TopLevel::Mod(ident, false) => assert_eq!(ident.name, "missing"),
            other => panic!("expected source-backed module declaration, got {other:?}"),
        }
    }

    #[test]
    fn parse_string_remains_parser_only_and_has_no_filepath() {
        let program = parse_string("main = -> 0\n", &Config::default())
            .expect("inline source should parse");

        assert_eq!(program.module.filepath, None);
    }
}
```

- [ ] **Step 2: Run tests to verify the new API is missing**

Run: `cargo test -p rock-lib parser::tests::parse_source_sets_module_filepath_without_reading_sibling_modules -- --exact`

Expected: FAIL with an unresolved import or missing function error for `parse_source`.

- [ ] **Step 3: Add text-backed parser entry points**

Replace the file-reading implementation in `lib/src/parser/mod.rs` with these parser-only helpers while leaving existing file-backed functions in place until later tasks migrate call sites:

```rust
pub fn parse_source(
    file_path: PathBuf,
    source: &str,
    config: &Config,
) -> Result<Module, ParseError> {
    let file_path_for_result = file_path.clone();
    let mut lexer = Lexer::new(file_path, source).map_err(ParseError::Lexer)?;
    let tokens = lexer.collect().map_err(ParseError::Lexer)?;

    if config.has_debug_print(DebugPrint::Tokens) {
        println!("{:#?}", tokens);
    }

    engine::reset_best_error();

    let ctx = ParseCtx::from(&tokens, config);
    let result = module_inline.process(ctx);

    match result {
        Ok((_, mut module)) => {
            module.filepath = Some(file_path_for_result);
            Ok(module)
        }
        Err(e) => Err(engine::get_best_error(e)),
    }
}

pub fn parse_string(input: &str, config: &Config) -> Result<Program, ParseError> {
    parse_source(PathBuf::new(), input, config).map(|mut module| {
        module.filepath = None;
        Program { module }
    })
}
```

For this checkpoint, keep `parse(config)` and `parse_module(file_path, config)` as temporary wrappers that call `std::fs::read_to_string` and then `parse_source`. They must be removed in Task 7 after every call site has moved to `SourceDatabase`.

- [ ] **Step 4: Stop inline module grammar from loading files**

Replace the `module` parser mapping in `lib/src/parser/items/module.rs` with AST-only construction:

```rust
pub fn module(stream: Input) -> IResult<Module> {
    (
        TokenType::Keyword("mod".to_string()),
        ident,
        TokenType::Eol,
        indented(many(top_level)),
    )
        .map(|(_, name, _, top_levels)| Module {
            name: Some(name),
            top_levels,
            is_inline: false,
            filepath: None,
        })
        .process(stream)
}
```

- [ ] **Step 5: Remove parser engine sibling path helper**

Delete `ParseCtx::sibling_module_filepath` from `lib/src/parser/engine/mod.rs` and remove the now-unused `PathBuf` import if it is only needed by that helper.

- [ ] **Step 6: Run parser-focused tests**

Run: `cargo test -p rock-lib parser::tests::parse_source_sets_module_filepath_without_reading_sibling_modules -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib parser::items::tests`

Expected: PASS.

- [ ] **Step 7: Commit parser boundary checkpoint**

Run:

```bash
git add lib/src/parser/mod.rs lib/src/parser/items/module.rs lib/src/parser/engine/mod.rs
git commit -m "separate parser from module file loading"
```

## Task 2: Source Loader Core

**Files:**
- Create: `lib/src/source_loader/mod.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add failing loader tests**

Create `lib/src/source_loader/mod.rs` with only imports plus these tests first:

```rust
#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::ast::TopLevel;
    use crate::source_loader::{SourceDatabase, SourceLoadError};
    use crate::Config;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rock_source_loader_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_std_config(entry_file: PathBuf) -> Config {
        Config {
            entry_file,
            no_std: true,
            no_prelude: true,
            ..Config::default()
        }
    }

    #[test]
    fn load_entry_records_root_source_file_and_module() {
        let dir = temp_dir("entry");
        let entry = dir.join("main.rk");
        fs::write(&entry, "main = -> 0\n").unwrap();

        let mut db = SourceDatabase::new();
        let graph = db
            .load_entry(entry.clone(), &no_std_config(entry.clone()))
            .expect("entry should load");

        assert_eq!(graph.root_path(), entry.as_path());
        assert_eq!(graph.root_module().filepath.as_ref(), Some(&entry));
        assert_eq!(graph.loaded_files(), &[entry.clone()]);
        assert!(db.module_for_path(&entry).is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sibling_resolution_prefers_flat_file_before_mod_rs_style_directory() {
        let dir = temp_dir("sibling_preference");
        let entry = dir.join("main.rk");
        fs::write(&entry, "mod util\nmain = -> util::answer!\n").unwrap();
        fs::write(dir.join("util.rk"), "answer = -> 1\n< answer\n").unwrap();
        fs::create_dir_all(dir.join("util")).unwrap();
        fs::write(dir.join("util").join("mod.rk"), "answer = -> 2\n< answer\n").unwrap();

        let mut db = SourceDatabase::new();
        let graph = db
            .load_entry(entry.clone(), &no_std_config(entry.clone()))
            .expect("entry should load");

        let util = graph
            .module_by_qualified_name("util")
            .expect("util module should be loaded");
        assert_eq!(util.path, dir.join("util.rk"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sibling_resolution_supports_directory_mod_file() {
        let dir = temp_dir("directory_mod");
        let entry = dir.join("main.rk");
        fs::write(&entry, "mod util\nmain = -> util::answer!\n").unwrap();
        fs::create_dir_all(dir.join("util")).unwrap();
        fs::write(dir.join("util").join("mod.rk"), "answer = -> 2\n< answer\n").unwrap();

        let mut db = SourceDatabase::new();
        let graph = db
            .load_entry(entry.clone(), &no_std_config(entry.clone()))
            .expect("entry should load");

        assert!(graph.module_for_path(&dir.join("util").join("mod.rk")).is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_module_reports_both_searched_paths() {
        let dir = temp_dir("missing_module");
        let entry = dir.join("main.rk");
        fs::write(&entry, "mod missing\nmain = -> 0\n").unwrap();

        let mut db = SourceDatabase::new();
        let errors = db
            .load_entry(entry.clone(), &no_std_config(entry.clone()))
            .expect_err("missing module should fail");

        match &errors[..] {
            [SourceLoadError::MissingModule { module, searched, span }] => {
                assert_eq!(module, "missing");
                assert_eq!(searched, &vec![dir.join("missing.rk"), dir.join("missing").join("mod.rk")]);
                assert!(span.is_some());
            }
            other => panic!("expected one missing module error, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn circular_module_load_reports_stack() {
        let dir = temp_dir("cycle");
        let entry = dir.join("main.rk");
        fs::write(&entry, "mod a\nmain = -> 0\n").unwrap();
        fs::write(dir.join("a.rk"), "mod b\n").unwrap();
        fs::write(dir.join("b.rk"), "mod a\n").unwrap();

        let mut db = SourceDatabase::new();
        let errors = db
            .load_entry(entry.clone(), &no_std_config(entry.clone()))
            .expect_err("cycle should fail");

        assert!(errors.iter().any(|error| match error {
            SourceLoadError::CircularModule { path, stack } => {
                path.ends_with(Path::new("a.rk")) && stack.iter().any(|entry| entry.ends_with(Path::new("b.rk")))
            }
            _ => false,
        }));

        let _ = fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: Expose the module and run failing tests**

Add this line to `lib/src/lib.rs` near other public modules:

```rust
pub mod source_loader;
```

Run: `cargo test -p rock-lib source_loader -- --exact`

Expected: FAIL with missing `SourceDatabase`, `SourceLoadError`, and related methods.

- [ ] **Step 3: Implement loader data types**

Add these public types above the tests in `lib/src/source_loader/mod.rs`:

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ast;
use crate::lexer::Span;
use crate::parser::{self, ParseError};
use crate::Config;

#[derive(Debug, Clone)]
pub struct SourceDatabase {
    files: BTreeMap<PathBuf, SourceFile>,
    modules: BTreeMap<PathBuf, ast::Module>,
    load_states: BTreeMap<PathBuf, ModuleLoadState>,
    loaded_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub display_path: PathBuf,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ModuleGraph {
    root_path: PathBuf,
    root_module: ast::Module,
    modules_by_qualified_name: BTreeMap<String, LoadedModule>,
    modules_by_path: BTreeMap<PathBuf, String>,
    loaded_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub qualified_name: String,
    pub path: PathBuf,
    pub canonical_path: PathBuf,
    pub module: ast::Module,
}

#[derive(Debug, Clone)]
pub enum SourceLoadError {
    Io { path: PathBuf, message: String },
    MissingModule { module: String, searched: Vec<PathBuf>, span: Option<Span> },
    Parse { path: PathBuf, error: ParseError },
    CircularModule { path: PathBuf, stack: Vec<PathBuf> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleLoadState {
    Loading,
    Loaded,
}
```

- [ ] **Step 4: Implement graph accessors and cache views**

Add these methods in `lib/src/source_loader/mod.rs`:

```rust
impl ModuleGraph {
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn root_module(&self) -> &ast::Module {
        &self.root_module
    }

    pub fn loaded_files(&self) -> &[PathBuf] {
        &self.loaded_files
    }

    pub fn modules(&self) -> impl Iterator<Item = &LoadedModule> {
        self.modules_by_qualified_name.values()
    }

    pub fn module_by_qualified_name(&self, name: &str) -> Option<&LoadedModule> {
        self.modules_by_qualified_name.get(name)
    }

    pub fn module_for_path(&self, path: &Path) -> Option<&LoadedModule> {
        self.modules_by_path
            .get(path)
            .and_then(|name| self.modules_by_qualified_name.get(name))
            .or_else(|| {
                path.canonicalize()
                    .ok()
                    .and_then(|canonical| self.modules_by_path.get(&canonical))
                    .and_then(|name| self.modules_by_qualified_name.get(name))
            })
    }

    pub fn loaded_module_paths(&self) -> Vec<(String, PathBuf)> {
        self.modules_by_qualified_name
            .values()
            .map(|module| (module.qualified_name.clone(), module.path.clone()))
            .collect()
    }

    pub fn module_file_cache(&self) -> std::collections::HashMap<PathBuf, ast::Module> {
        let mut cache = std::collections::HashMap::new();
        for module in self.modules_by_qualified_name.values() {
            cache.insert(module.path.clone(), module.module.clone());
            cache.insert(module.canonical_path.clone(), module.module.clone());
        }
        cache
    }
}

impl SourceDatabase {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            modules: BTreeMap::new(),
            load_states: BTreeMap::new(),
            loaded_files: Vec::new(),
        }
    }

    pub fn loaded_files(&self) -> &[PathBuf] {
        &self.loaded_files
    }

    pub fn module_for_path(&self, path: &Path) -> Option<&ast::Module> {
        self.canonical_key(path)
            .ok()
            .and_then(|key| self.modules.get(&key))
            .or_else(|| self.modules.get(path))
    }
}

impl Default for SourceDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 5: Implement file loading, sibling resolution, and recursive graph construction**

Add these methods to `impl SourceDatabase`:

```rust
pub fn load_entry(
    &mut self,
    path: PathBuf,
    config: &Config,
) -> Result<ModuleGraph, Vec<SourceLoadError>> {
    let root_prefix = config.current_crate_name.clone();
    self.load_graph(path, root_prefix, config)
}

pub fn load_source_crate(
    &mut self,
    lib_path: PathBuf,
    crate_name: &str,
    config: &Config,
) -> Result<ModuleGraph, Vec<SourceLoadError>> {
    self.load_graph(lib_path, Some(crate_name.to_string()), config)
}

fn load_graph(
    &mut self,
    root_path: PathBuf,
    root_prefix: Option<String>,
    config: &Config,
) -> Result<ModuleGraph, Vec<SourceLoadError>> {
    let mut errors = Vec::new();
    let mut graph = ModuleGraph {
        root_path: root_path.clone(),
        root_module: ast::Module {
            name: None,
            top_levels: Vec::new(),
            is_inline: true,
            filepath: Some(root_path.clone()),
        },
        modules_by_qualified_name: BTreeMap::new(),
        modules_by_path: BTreeMap::new(),
        loaded_files: Vec::new(),
    };
    let mut stack = Vec::new();
    let root_module = self.load_module_recursive(
        root_path.clone(),
        root_prefix.clone(),
        config,
        &mut stack,
        &mut graph,
        &mut errors,
    );

    if let Some(root_module) = root_module {
        graph.root_module = root_module;
    }
    graph.loaded_files = self.loaded_files.clone();

    if errors.is_empty() {
        Ok(graph)
    } else {
        Err(errors)
    }
}

fn load_module_recursive(
    &mut self,
    path: PathBuf,
    qualified_name: Option<String>,
    config: &Config,
    stack: &mut Vec<PathBuf>,
    graph: &mut ModuleGraph,
    errors: &mut Vec<SourceLoadError>,
) -> Option<ast::Module> {
    let canonical_path = match self.canonical_key(&path) {
        Ok(path) => path,
        Err(error) => {
            errors.push(error);
            return None;
        }
    };

    if self.load_states.get(&canonical_path) == Some(&ModuleLoadState::Loading) {
        errors.push(SourceLoadError::CircularModule {
            path: path.clone(),
            stack: stack.clone(),
        });
        return self.modules.get(&canonical_path).cloned();
    }

    if let Some(module) = self.modules.get(&canonical_path).cloned() {
        if let Some(qualified_name) = qualified_name {
            self.record_loaded_module(graph, qualified_name, path, canonical_path, module.clone());
        }
        return Some(module);
    }

    self.load_states
        .insert(canonical_path.clone(), ModuleLoadState::Loading);
    stack.push(path.clone());

    let source = match self.read_source_file(path.clone(), canonical_path.clone()) {
        Ok(source) => source,
        Err(error) => {
            errors.push(error);
            stack.pop();
            self.load_states.remove(&canonical_path);
            return None;
        }
    };

    let module = match parser::parse_source(path.clone(), &source.text, config) {
        Ok(module) => module,
        Err(error) => {
            errors.push(SourceLoadError::Parse { path: path.clone(), error });
            stack.pop();
            self.load_states.remove(&canonical_path);
            return None;
        }
    };

    self.modules.insert(canonical_path.clone(), module.clone());
    if !self.loaded_files.contains(&path) {
        self.loaded_files.push(path.clone());
    }

    if let Some(qualified_name) = qualified_name.clone() {
        self.record_loaded_module(
            graph,
            qualified_name,
            path.clone(),
            canonical_path.clone(),
            module.clone(),
        );
    }

    self.load_children(&module, qualified_name.as_deref(), config, stack, graph, errors);
    stack.pop();
    self.load_states
        .insert(canonical_path, ModuleLoadState::Loaded);

    Some(module)
}
```

Then add helper methods used above:

```rust
fn canonical_key(&self, path: &Path) -> Result<PathBuf, SourceLoadError> {
    path.canonicalize().map_err(|error| SourceLoadError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn read_source_file(
    &mut self,
    original_path: PathBuf,
    canonical_path: PathBuf,
) -> Result<SourceFile, SourceLoadError> {
    if let Some(file) = self.files.get(&canonical_path).cloned() {
        return Ok(file);
    }

    let text = std::fs::read_to_string(&original_path).map_err(|error| SourceLoadError::Io {
        path: original_path.clone(),
        message: error.to_string(),
    })?;
    let file = SourceFile {
        original_path: original_path.clone(),
        canonical_path: canonical_path.clone(),
        display_path: original_path,
        text,
    };
    self.files.insert(canonical_path, file.clone());
    Ok(file)
}

fn record_loaded_module(
    &self,
    graph: &mut ModuleGraph,
    qualified_name: String,
    path: PathBuf,
    canonical_path: PathBuf,
    module: ast::Module,
) {
    graph.modules_by_path.insert(path.clone(), qualified_name.clone());
    graph
        .modules_by_path
        .insert(canonical_path.clone(), qualified_name.clone());
    graph.modules_by_qualified_name.insert(
        qualified_name.clone(),
        LoadedModule {
            qualified_name,
            path,
            canonical_path,
            module,
        },
    );
}

fn load_children(
    &mut self,
    module: &ast::Module,
    prefix: Option<&str>,
    config: &Config,
    stack: &mut Vec<PathBuf>,
    graph: &mut ModuleGraph,
    errors: &mut Vec<SourceLoadError>,
) {
    for top_level in &module.top_levels {
        match top_level {
            ast::TopLevel::Module(ast::ModuleDecl(inline)) => {
                let next_prefix = inline.name.as_ref().and_then(|name| {
                    prefix
                        .map(|prefix| format!("{}::{}", prefix, name.name))
                        .or_else(|| Some(name.name.clone()))
                });
                self.load_children(inline, next_prefix.as_deref(), config, stack, graph, errors);
            }
            ast::TopLevel::Mod(ident, _) => {
                let Some(parent_path) = module.filepath.as_ref() else {
                    continue;
                };
                let Some(child_path) = self.resolve_sibling_module(parent_path, ident, errors) else {
                    continue;
                };
                let qualified_name = prefix
                    .map(|prefix| format!("{}::{}", prefix, ident.name))
                    .unwrap_or_else(|| ident.name.clone());
                self.load_module_recursive(
                    child_path,
                    Some(qualified_name),
                    config,
                    stack,
                    graph,
                    errors,
                );
            }
            _ => {}
        }
    }
}

fn resolve_sibling_module(
    &self,
    parent_path: &Path,
    ident: &ast::Ident,
    errors: &mut Vec<SourceLoadError>,
) -> Option<PathBuf> {
    let base = parent_path.parent().unwrap_or_else(|| Path::new("."));
    let flat = base.join(format!("{}.rk", ident.name));
    if flat.exists() {
        return Some(flat);
    }

    let directory_mod = base.join(&ident.name).join("mod.rk");
    if directory_mod.exists() {
        return Some(directory_mod);
    }

    errors.push(SourceLoadError::MissingModule {
        module: ident.name.clone(),
        searched: vec![flat, directory_mod],
        span: Some(ident.span.clone()),
    });
    None
}
```

- [ ] **Step 6: Run loader tests**

Run: `cargo test -p rock-lib source_loader`

Expected: PASS.

- [ ] **Step 7: Commit loader core checkpoint**

Run:

```bash
git add lib/src/lib.rs lib/src/source_loader/mod.rs
git commit -m "add source database module loader"
```

## Task 3: Compile Pipeline And Product Fingerprints Use Loader Files

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `rockc/src/main.rs`

- [ ] **Step 1: Add product fingerprint regression for loaded source modules**

Add this test to `#[cfg(test)] mod tests` in `lib/src/lib.rs` near existing product source fingerprint tests:

```rust
#[test]
fn product_source_fingerprint_uses_source_loader_loaded_files() {
    let base = std::env::temp_dir().join(format!(
        "rock_fingerprint_loaded_modules_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let entry_file = base.join("main.rk");
    fs::write(&entry_file, "mod util\nmain = -> util::answer!\n").unwrap();
    fs::write(base.join("util.rk"), "answer = -> 0\n< answer\n").unwrap();
    fs::write(base.join("unloaded.rk"), "answer = -> 1\n").unwrap();

    let output = compile_with_products(&Config {
        entry_file,
        output_dir: base.join("build"),
        no_std: true,
        no_prelude: true,
        no_link: true,
        current_crate_name: Some("demo".to_string()),
        ..Config::default()
    })
    .unwrap();

    let products = output.products.expect("products should be emitted");
    assert_eq!(
        products.crate_identity.source_fingerprint.loaded_files,
        vec![PathBuf::from("main.rk"), PathBuf::from("util.rk")]
    );

    let _ = fs::remove_dir_all(&base);
}
```

- [ ] **Step 2: Run failing fingerprint test**

Run: `cargo test -p rock-lib product_source_fingerprint_uses_source_loader_loaded_files -- --exact`

Expected: FAIL until `compile_impl` passes loader loaded files into fingerprinting.

- [ ] **Step 3: Convert loader errors to diagnostics**

Add this helper in `lib/src/lib.rs` near other private diagnostic helpers:

```rust
fn source_load_errors_to_diagnostics(errors: Vec<source_loader::SourceLoadError>) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    for error in errors {
        match error {
            source_loader::SourceLoadError::Io { path, message } => {
                diagnostics.push(diagnostic::Diagnostic::new(
                    format!("Failed to read source file {}: {}", path.display(), message),
                    Span::default(),
                ));
            }
            source_loader::SourceLoadError::MissingModule { module, searched, span } => {
                let searched = searched
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(diagnostic::Diagnostic::new(
                    format!("Module '{}' not found; searched: {}", module, searched),
                    span.unwrap_or_default(),
                ));
            }
            source_loader::SourceLoadError::Parse { error, .. } => {
                diagnostics.merge(Diagnostics::from(error));
            }
            source_loader::SourceLoadError::CircularModule { path, stack } => {
                let stack = stack
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                diagnostics.push(diagnostic::Diagnostic::new(
                    format!("Circular module load detected at {} via {}", path.display(), stack),
                    Span::default(),
                ));
            }
        }
    }
    diagnostics
}
```

If `Diagnostics` does not have `extend`, replace the parse branch with a loop over `Diagnostics::from(error).0` and push each diagnostic.

- [ ] **Step 4: Route compile entry through SourceDatabase**

In `compile_impl` in `lib/src/lib.rs`, replace Phase 1 parser usage with loader usage:

```rust
// Phase 1: Load and parse current-crate source graph.
let mut source_db = source_loader::SourceDatabase::new();
let source_graph = match source_db.load_entry(config.entry_file.clone(), config) {
    Ok(graph) => graph,
    Err(errors) => return Err(source_load_errors_to_diagnostics(errors)),
};
let ast = Program {
    module: source_graph.root_module().clone(),
};
```

Keep the rest of the phase order unchanged.

- [ ] **Step 5: Change product fingerprinting to accept loaded files**

Replace `product_source_fingerprint(config, &ast.module)` calls with:

```rust
let source_fingerprint = product_source_fingerprint(config, source_graph.loaded_files());
```

Replace the helper signature and body in `lib/src/lib.rs` with:

```rust
fn product_source_fingerprint(config: &Config, source_files: &[PathBuf]) -> ProductSourceFingerprint {
    let root = config.entry_file.parent().unwrap_or_else(|| Path::new("."));
    let mut source_files = source_files.to_vec();
    if source_files.is_empty() {
        source_files.push(config.entry_file.clone());
    }

    let mut loaded_files = relative_fingerprint_paths(root, &source_files);
    loaded_files.sort();
    loaded_files.dedup();

    let source_hash = stable_file_fingerprint(root, &loaded_files);
    let manifest_path = config
        .entry_file
        .parent()
        .map(|parent| parent.join("rock.toml"))
        .filter(|path| path.exists());
    let manifest_hash = manifest_path.as_ref().and_then(|path| {
        let manifest_files = relative_fingerprint_paths(root, std::slice::from_ref(path));
        stable_file_fingerprint(root, &manifest_files)
    });

    ProductSourceFingerprint {
        manifest_hash,
        source_hash,
        loaded_files,
    }
}
```

Delete `collect_loaded_module_source_files` after the helper no longer uses AST traversal.

- [ ] **Step 6: Update `rockc` format and expand to use SourceDatabase**

In `rockc/src/main.rs`, add a helper near `format_config`:

```rust
fn load_program_for_source_command(rockc_config: &rock_lib::Config) -> Result<Program, Diagnostics> {
    let mut source_db = rock_lib::source_loader::SourceDatabase::new();
    source_db
        .load_entry(rockc_config.entry_file.clone(), rockc_config)
        .map(|graph| Program {
            module: graph.root_module().clone(),
        })
        .map_err(|errors| {
            let mut diagnostics = Diagnostics::default();
            for error in errors {
                diagnostics.push(rock_lib::diagnostic::Diagnostic::new(
                    format!("Failed to load source graph: {error:?}"),
                    Default::default(),
                ));
            }
            diagnostics
        })
}
```

Replace both `rock_lib::parser::parse(&rockc_config).map_err(Diagnostics::from)?` calls with:

```rust
load_program_for_source_command(&rockc_config)?
```

- [ ] **Step 7: Run compile and fingerprint tests**

Run: `cargo test -p rock-lib product_source_fingerprint_uses_source_loader_loaded_files -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_modules -- --exact`

Expected: PASS.

- [ ] **Step 8: Commit compile pipeline checkpoint**

Run:

```bash
git add lib/src/lib.rs rockc/src/main.rs
git commit -m "load compile sources through source database"
```

## Task 4: Declaration Collection Consumes ModuleGraph

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/item_index.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add a collection regression that fails if a cached module is not used**

Add this test to `#[cfg(test)] mod tests` in `lib/src/collect/mod.rs`:

```rust
#[test]
fn collect_uses_source_graph_for_source_backed_modules() {
    let temp_dir = std::env::temp_dir().join(format!(
        "rock_collect_source_graph_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let entry = temp_dir.join("main.rk");
    std::fs::write(&entry, "mod util\n> util::answer\nmain = -> answer!\n").unwrap();
    std::fs::write(temp_dir.join("util.rk"), "answer = -> 42\n< answer\n").unwrap();

    let config = crate::Config {
        entry_file: entry.clone(),
        no_std: true,
        no_prelude: true,
        current_crate_name: Some("demo".to_string()),
        ..crate::Config::default()
    };
    let mut db = crate::source_loader::SourceDatabase::new();
    let graph = db.load_entry(entry.clone(), &config).unwrap();
    std::fs::remove_file(temp_dir.join("util.rk")).unwrap();
    let program = crate::ast::Program {
        module: graph.root_module().clone(),
    };

    let decls = collect_with_source_graph(
        &program,
        &graph,
        &crate::crate_system::CrateContext::new(),
        false,
        Some("demo"),
    )
    .expect("collection should use preloaded graph instead of reading util.rk again");

    assert!(decls.functions.contains_key("demo::util::answer"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

- [ ] **Step 2: Run failing collection test**

Run: `cargo test -p rock-lib collect_uses_source_graph_for_source_backed_modules -- --exact`

Expected: FAIL with missing `collect_with_source_graph`.

- [ ] **Step 3: Add graph seeding to CollectContext**

Add this method to `impl CollectContext` in `lib/src/collect/context.rs`:

```rust
pub(crate) fn seed_source_graph(&mut self, graph: &crate::source_loader::ModuleGraph) {
    for (qualified_name, path) in graph.loaded_module_paths() {
        if !self
            .loaded_module_paths
            .iter()
            .any(|(name, existing_path)| name == &qualified_name && existing_path == &path)
        {
            self.loaded_module_paths.push((qualified_name, path));
        }
    }

    for (path, module) in graph.module_file_cache() {
        self.module_file_cache.entry(path).or_insert(module);
    }
}
```

- [ ] **Step 4: Add collection entry point with a source graph**

Add a new function in `lib/src/collect/mod.rs` next to `collect`:

```rust
pub fn collect_with_source_graph(
    program: &ast::Program,
    source_graph: &crate::source_loader::ModuleGraph,
    crate_ctx: &CrateContext,
    inject_prelude: bool,
    current_crate_name: Option<&str>,
) -> Result<Declarations, Vec<ResolveError>> {
    collect_impl(program, Some(source_graph), crate_ctx, inject_prelude, current_crate_name)
}
```

Rename the current `collect` body to `collect_impl` with this signature:

```rust
fn collect_impl(
    program: &ast::Program,
    source_graph: Option<&crate::source_loader::ModuleGraph>,
    crate_ctx: &CrateContext,
    inject_prelude: bool,
    current_crate_name: Option<&str>,
) -> Result<Declarations, Vec<ResolveError>> {
```

Then make the public `collect` call the implementation without a graph:

```rust
pub fn collect(
    program: &ast::Program,
    crate_ctx: &CrateContext,
    inject_prelude: bool,
    current_crate_name: Option<&str>,
) -> Result<Declarations, Vec<ResolveError>> {
    collect_impl(program, None, crate_ctx, inject_prelude, current_crate_name)
}
```

Inside `collect_impl`, immediately after `bootstrap_for_collection`, seed the graph:

```rust
if let Some(source_graph) = source_graph {
    context.seed_source_graph(source_graph);
}
```

- [ ] **Step 5: Make collect source module discovery use graph-seeded paths and cache**

Replace path construction in `discover_source_module_for_indexing` with a lookup from `loaded_module_paths` first:

```rust
let file_path = context
    .loaded_module_paths
    .iter()
    .find(|(name, _)| name == &qualified_module_name)
    .map(|(_, path)| path.clone())
    .unwrap_or_else(|| {
        let base_path = &context.current_module_path;
        let mut file_path = base_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        file_path.push(module_name);
        if file_path.extension().is_none() {
            file_path.set_extension("rk");
        }
        file_path
    });
```

Keep the later `context.current_module_path = file_path` behavior so nested module prefixes still match existing behavior.

- [ ] **Step 6: Remove reparsing from collect context module loading**

Change `CollectContext::load_module` so it returns cached modules only and reports a source loader boundary error if the module was not seeded:

```rust
let canonical_path = file_path
    .canonicalize()
    .unwrap_or_else(|_| file_path.clone());
if let Some(cached) = self
    .module_file_cache
    .get(&canonical_path)
    .or_else(|| self.module_file_cache.get(&file_path))
    .cloned()
{
    self.loaded_modules.insert(canonical_path);
    return Ok(cached);
}

Err(format!(
    "Module '{}' was not loaded by the source database; expected {}",
    module_name,
    file_path.display()
))
```

Apply the same cache-only behavior to `CollectContext::load_external_crate_module`, using the external crate module path it already computes.

Delete `use crate::parser;` and `use crate::Config;` from `lib/src/collect/context.rs` after no parser/config calls remain.

- [ ] **Step 7: Prefer graph-derived module maps during indexing**

Add this helper to `lib/src/collect/item_index.rs`:

```rust
pub fn source_module_map_from_graph(
    graph: &crate::source_loader::ModuleGraph,
) -> SourceModuleMap {
    graph
        .modules()
        .map(|loaded| {
            (
                loaded
                    .qualified_name
                    .split("::")
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                loaded.module.clone(),
            )
        })
        .collect()
}
```

In `collect_impl`, build `source_modules` with the graph when provided:

```rust
let source_modules = if let Some(source_graph) = source_graph {
    source_module_map_from_graph(source_graph)
} else {
    source_module_map_from_loaded_modules(
        &program.module,
        current_crate_name,
        &context.loaded_module_paths,
        &context.module_file_cache,
    )
};
```

- [ ] **Step 8: Use collect_with_source_graph from compile_impl**

In `lib/src/lib.rs`, replace the existing `collect::collect` call with:

```rust
let decls = match collect::collect_with_source_graph(
    &ast,
    &source_graph,
    crate_ctx,
    !config.no_prelude,
    config.current_crate_name.as_deref(),
) {
```

- [ ] **Step 9: Run collection and module integration tests**

Run: `cargo test -p rock-lib collect_uses_source_graph_for_source_backed_modules -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_module_glob_import_and_export -- --exact`

Expected: PASS.

- [ ] **Step 10: Commit collection checkpoint**

Run:

```bash
git add lib/src/collect/mod.rs lib/src/collect/context.rs lib/src/collect/item_index.rs lib/src/lib.rs
git commit -m "collect declarations from loaded source graph"
```

## Task 5: Lowering Uses Cached Loaded Modules Only

**Files:**
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/mod.rs`

- [ ] **Step 1: Add lower regression for deleted source file after collection**

Add this test to `#[cfg(test)] mod tests` in `lib/src/lower/mod.rs`:

```rust
#[test]
fn lower_from_declarations_uses_preloaded_module_cache() {
    let temp_dir = std::env::temp_dir().join(format!(
        "rock_lower_source_graph_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let entry = temp_dir.join("main.rk");
    std::fs::write(&entry, "mod util\n> util::answer\nmain = -> answer!\n").unwrap();
    std::fs::write(temp_dir.join("util.rk"), "answer = -> 42\n< answer\n").unwrap();

    let config = crate::Config {
        entry_file: entry.clone(),
        no_std: true,
        no_prelude: true,
        current_crate_name: Some("demo".to_string()),
        ..crate::Config::default()
    };
    let mut db = crate::source_loader::SourceDatabase::new();
    let graph = db.load_entry(entry.clone(), &config).unwrap();
    let program = crate::ast::Program {
        module: graph.root_module().clone(),
    };
    let decls = crate::collect::collect_with_source_graph(
        &program,
        &graph,
        &crate::crate_system::CrateContext::new(),
        false,
        Some("demo"),
    )
    .unwrap();
    std::fs::remove_file(temp_dir.join("util.rk")).unwrap();

    let lowered = crate::lower::program::lower_from_declarations(
        &program,
        decls,
        &crate::crate_system::CrateContext::new(),
        Some("demo"),
    );

    assert!(lowered.is_ok(), "lowering should not re-read util.rk: {lowered:?}");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

- [ ] **Step 2: Run failing lower test**

Run: `cargo test -p rock-lib lower_from_declarations_uses_preloaded_module_cache -- --exact`

Expected: FAIL if lower still reparses deleted module files.

- [ ] **Step 3: Add cached-module lookup helpers to Lowerer**

In `impl Lowerer` in `lib/src/lower/program.rs`, add helpers before `load_module_by_path`:

```rust
fn cached_module_for_path(&self, path: &std::path::Path) -> Option<ast::Module> {
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    self.module_file_cache
        .get(&canonical_path)
        .or_else(|| self.module_file_cache.get(path))
        .cloned()
}

fn loaded_path_for_module_name(&self, qualified_module_name: &str, fallback_path: PathBuf) -> PathBuf {
    self.loaded_module_paths
        .iter()
        .find(|(name, _)| name == qualified_module_name)
        .map(|(_, path)| path.clone())
        .unwrap_or(fallback_path)
}
```

- [ ] **Step 4: Make lower module loading cache-only**

Replace parser-backed `load_module` body with cache-only lookup:

```rust
let base_path = &self.current_module_path;
let mut file_path = base_path
    .parent()
    .unwrap_or(std::path::Path::new("."))
    .to_path_buf();
file_path.push(module_name);
if file_path.extension().and_then(|s| s.to_str()) != Some("rk") {
    file_path.set_extension("rk");
}

self.cached_module_for_path(&file_path).ok_or_else(|| {
    format!(
        "Module '{}' was not loaded by the source database; expected {}",
        module_name,
        file_path.display()
    )
})
```

Apply the same cache-only rule to `load_external_crate_module`.

- [ ] **Step 5: Remove body and trait-default reparsing**

In `lower_module_bodies_qualified`, replace the `TopLevel::Mod` parser fallback with:

```rust
let canonical_path = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
let loaded_module = self
    .module_file_cache
    .get(&canonical_path)
    .or_else(|| self.module_file_cache.get(&file_path))
    .cloned();
```

In `lower_loaded_module_bodies`, replace the whole parser/config block with:

```rust
if let Some(loaded_module) = self.cached_module_for_path(file_path) {
    let old_path = self.current_module_path.clone();
    self.current_module_path = file_path.clone();
    self.lower_module_bodies_qualified(&loaded_module, Some(module_name));
    self.current_module_path = old_path;
}
```

In `lower_loaded_module_trait_defaults`, replace the parser/config block with:

```rust
if let Some(loaded_module) = self.cached_module_for_path(file_path) {
    let old_path = self.current_module_path.clone();
    self.current_module_path = file_path.clone();
    self.scope.push();
    let added_aliases = self.inject_module_local_aliases(&loaded_module, module_name);
    self.lower_trait_default_bodies(&loaded_module);
    for alias in added_aliases {
        self.module_local_aliases.remove(&alias);
    }
    self.scope.pop();
    self.current_module_path = old_path;
}
```

Delete `use crate::parser;` and `use crate::Config;` from `lib/src/lower/program.rs` after no parser/config calls remain.

- [ ] **Step 6: Run lower and module tests**

Run: `cargo test -p rock-lib lower_from_declarations_uses_preloaded_module_cache -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_module_glob_import_and_export -- --exact`

Expected: PASS.

- [ ] **Step 7: Commit lower checkpoint**

Run:

```bash
git add lib/src/lower/program.rs lib/src/lower/mod.rs
git commit -m "lower modules from source database cache"
```

## Task 6: Crate-System Source Loading Uses SourceDatabase

**Files:**
- Modify: `lib/src/crate_system/context.rs`
- Modify: `lib/src/crate_system/module_tree.rs`
- Modify: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add crate-system regression for source crate file cache**

Add this test to `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn load_crate_from_dir_uses_source_database_module_graph() {
    let dir = std::env::temp_dir().join(format!(
        "rock_crate_source_graph_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("rock.toml"),
        "[crate]\nname = \"dep\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/lib.rk\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("src").join("lib.rk"), "mod util\n< util::*\n").unwrap();
    std::fs::write(dir.join("src").join("util.rk"), "answer = -> 42\n< answer\n").unwrap();

    let mut ctx = crate::crate_system::CrateContext::new();
    ctx.load_crate_from_dir(dir.clone()).expect("source crate should load");
    let source = ctx.source_crate("dep").expect("dep should be registered");

    assert!(source.file_cache.values().any(|module| {
        module
            .filepath
            .as_ref()
            .is_some_and(|path| path.ends_with("util.rk"))
    }));
    assert!(source.build_module_tree().is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run failing crate-system test**

Run: `cargo test -p rock-lib load_crate_from_dir_uses_source_database_module_graph -- --exact`

Expected: FAIL until crate loading uses `SourceDatabase` and graph-derived caches.

- [ ] **Step 3: Replace module_tree parser helpers with graph transforms**

Delete `parse_module_file` and `collect_module_file_cache` from `lib/src/crate_system/module_tree.rs`.

Add these helpers:

```rust
pub(super) fn build_module_tree_from_graph(
    graph: &crate::source_loader::ModuleGraph,
    crate_name: &str,
) -> Result<ModuleTree, String> {
    let mut modules = BTreeMap::new();
    build_module_nodes(graph.root_module(), crate_name, &mut modules)?;
    for loaded in graph.modules() {
        build_module_nodes(&loaded.module, &loaded.qualified_name, &mut modules)?;
    }
    Ok(ModuleTree { modules })
}

pub(super) fn module_file_cache_from_graph(
    graph: &crate::source_loader::ModuleGraph,
) -> std::collections::HashMap<PathBuf, Module> {
    graph.module_file_cache()
}
```

- [ ] **Step 4: Route CrateContext::load_crate_from_dir through SourceDatabase**

In `lib/src/crate_system/context.rs`, remove the `collect_module_file_cache` import. Replace parser-backed loading in `load_crate_from_dir` with:

```rust
let mut source_config = crate::Config {
    entry_file: lib_path.clone(),
    current_crate_name: Some(manifest.crate_.name.clone()),
    no_prelude: true,
    ..crate::Config::default()
};
source_config.no_std = true;

let mut source_db = crate::source_loader::SourceDatabase::new();
let graph = source_db
    .load_source_crate(lib_path.clone(), &manifest.crate_.name, &source_config)
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| format!("{error:?}"))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
let ast = graph.root_module().clone();
let file_cache = super::module_tree::module_file_cache_from_graph(&graph);
```

Replace the existing `collect` call with `collect_with_source_graph`:

```rust
collect(
    &Program { module: ast.clone() },
    self,
    false,
    Some(&manifest.crate_.name),
)
```

becomes:

```rust
collect_with_source_graph(
    &Program { module: ast.clone() },
    &graph,
    self,
    false,
    Some(&manifest.crate_.name),
)
```

Update the import at the top to:

```rust
use crate::collect::{collect_with_source_graph, collect};
```

Keep `collect` imported only if another function in the file still uses it.

- [ ] **Step 5: Store graph-derived module tree in CurrentCrateSource**

In `load_crate_from_dir`, after creating `CurrentCrateSource`, set both graph-derived fields:

```rust
let name = manifest.crate_.name.clone();
let mut source = CurrentCrateSource::new(manifest, crate_dir, ast);
source.file_cache = file_cache;
source.module_tree = Some(super::module_tree::build_module_tree_from_graph(&graph, &name)?);
self.source_crates.insert(name, source);
```

In `lib/src/crate_system/extern_store.rs`, update `build_module_tree` to return the existing graph-derived tree when present:

```rust
pub(crate) fn build_module_tree(&self) -> Result<ModuleTree, String> {
    if let Some(tree) = &self.module_tree {
        return Ok(tree.clone());
    }
    crate::crate_system::module_tree::build_module_tree(&self.ast, &self.manifest.crate_.name)
}
```

Derive `Clone` for `ModuleTree` and `ModuleNode` in `lib/src/crate_system/mod.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ModuleTree {
    pub modules: BTreeMap<String, ModuleNode>,
}

#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub name: String,
    pub qualified_name: String,
    pub submodules: Vec<String>,
}
```

- [ ] **Step 6: Run crate-system and artifact declaration tests**

Run: `cargo test -p rock-lib load_crate_from_dir_uses_source_database_module_graph -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib collect_artifact_declarations -- --exact`

Expected: PASS if matching tests exist; if the command reports no tests, run `cargo test -p rock-lib crate_system` instead.

- [ ] **Step 7: Commit crate-system checkpoint**

Run:

```bash
git add lib/src/crate_system/context.rs lib/src/crate_system/module_tree.rs lib/src/crate_system/extern_store.rs lib/src/crate_system/mod.rs lib/src/collect/mod.rs lib/src/crate_system/tests.rs
git commit -m "load source crates through source database"
```

## Task 7: Remove Parser File-Reading APIs And Leftover Reparse Call Sites

**Files:**
- Modify: `lib/src/parser/mod.rs`
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/crate_system/module_tree.rs`
- Modify: `rockc/src/main.rs`
- Modify tests that still call parser file APIs.

- [ ] **Step 1: Search for parser file API call sites**

Run: `rg "parse_module|parser::parse\(|pub fn parse\(" lib rock rockc --glob '*.rs'`

Expected before edits: only `lib/src/parser/mod.rs` wrappers and any unconverted call sites from previous tasks.

- [ ] **Step 2: Delete parser file-reading wrappers**

Remove the `parse(config: &Config) -> Result<Program, ParseError>` function and the `parse_module(file_path: PathBuf, config: &Config) -> Result<Module, ParseError>` function from `lib/src/parser/mod.rs`.

Keep only `parse_source` and `parse_string` as public parser entry points.

- [ ] **Step 3: Replace any remaining file-backed parser use with SourceDatabase**

For each remaining file-backed parse call, use this pattern:

```rust
let mut source_db = crate::source_loader::SourceDatabase::new();
let graph = source_db.load_entry(entry_path.clone(), &config)?;
let program = crate::ast::Program {
    module: graph.root_module().clone(),
};
```

For tests that only parse inline snippets, use `parser::parse_string` or `parser::parse_source` with explicit source text.

- [ ] **Step 4: Verify no parser-owned file loading remains**

Run: `rg "std::fs::read_to_string|parse_module|parser::parse\(|sibling_module_filepath" lib/src/parser lib/src/collect lib/src/lower lib/src/crate_system rockc/src/main.rs --glob '*.rs'`

Expected: no matches for `parse_module`, `parser::parse(`, or `sibling_module_filepath`. Any `std::fs::read_to_string` match must be outside parser/collect/lower/crate-system source loading or must be the `SourceDatabase` read.

- [ ] **Step 5: Run parser, collect, lower, and module tests**

Run: `cargo test -p rock-lib parser::items::tests`

Expected: PASS.

Run: `cargo test -p rock-lib source_loader`

Expected: PASS.

Run: `cargo test -p rock-lib module`

Expected: PASS.

- [ ] **Step 6: Commit parser API removal checkpoint**

Run:

```bash
git add lib/src/parser/mod.rs lib/src/collect/context.rs lib/src/lower/program.rs lib/src/crate_system/module_tree.rs rockc/src/main.rs
git commit -m "remove parser file loading APIs"
```

## Task 8: Integration Regressions And Final Verification

**Files:**
- Modify: `lib/tests/integration.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/source_loader/mod.rs`

- [ ] **Step 1: Add integration test for `name/mod.rk` sibling modules**

Add this test to `lib/tests/integration.rs` near `test_modules`:

```rust
#[test]
fn test_module_directory_mod_file_resolution() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = test_temp_dir(id);
    std::fs::create_dir_all(dir.join("utils")).unwrap();

    std::fs::write(
        dir.join("utils").join("mod.rk"),
        "answer = -> 42\n< answer\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("test.rk"),
        "mod utils\n> utils::answer\nmain = ->\n    (answer!).println!\n    0\n",
    )
    .unwrap();

    let config = test_config(dir.join("test.rk"), dir.clone());
    rock_lib::compile(&config).expect("Compilation failed");
    let output = Command::new(dir.join("test"))
        .output()
        .expect("Failed to run compiled binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(stdout.trim(), "42");
}
```

- [ ] **Step 2: Add integration test for structured missing module diagnostics**

Add this test to `lib/tests/integration.rs`:

```rust
#[test]
fn test_missing_module_reports_all_searched_paths() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = test_temp_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.rk"), "mod missing\nmain = -> 0\n").unwrap();

    let config = test_config(dir.join("test.rk"), dir.clone());
    let diagnostics = rock_lib::compile(&config).expect_err("missing module should fail");
    let text = diagnostics
        .0
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("missing.rk"), "diagnostics: {text}");
    assert!(text.contains("missing/mod.rk"), "diagnostics: {text}");

    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 3: Add integration test for circular modules**

Add this test to `lib/tests/integration.rs`:

```rust
#[test]
fn test_circular_module_load_reports_diagnostic() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = test_temp_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.rk"), "mod a\nmain = -> 0\n").unwrap();
    std::fs::write(dir.join("a.rk"), "mod b\n").unwrap();
    std::fs::write(dir.join("b.rk"), "mod a\n").unwrap();

    let config = test_config(dir.join("test.rk"), dir.clone());
    let diagnostics = rock_lib::compile(&config).expect_err("cycle should fail");
    let text = diagnostics
        .0
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("Circular module load"), "diagnostics: {text}");
    assert!(text.contains("a.rk"), "diagnostics: {text}");
    assert!(text.contains("b.rk"), "diagnostics: {text}");

    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 4: Run new integration tests**

Run: `cargo test -p rock-lib --test integration test_module_directory_mod_file_resolution -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_missing_module_reports_all_searched_paths -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_circular_module_load_reports_diagnostic -- --exact`

Expected: PASS.

- [ ] **Step 5: Run targeted regression commands from the spec**

Run: `cargo test -p rock-lib source_loader`

Expected: PASS.

Run: `cargo test -p rock-lib module`

Expected: PASS.

Run: `cargo test -p rock-lib product_source_fingerprint`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_modules -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_module_glob_import_and_export -- --exact`

Expected: PASS.

- [ ] **Step 6: Run final verification**

Run: `cargo fmt --all --check`

Expected: PASS.

Run: `cargo test -p rock-lib`

Expected: PASS.

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 7: Commit final source loader verification checkpoint**

Run:

```bash
git add lib/tests/integration.rs lib/src/lib.rs lib/src/source_loader/mod.rs
git commit -m "verify source database module loading"
```

## Self-Review Notes

- Spec coverage: Tasks 1 and 7 remove parser-owned file loading; Task 2 owns file IO, canonical paths, parsed module cache, sibling resolution, load state, loaded-file ordering, missing/parse/circular errors; Tasks 4 and 5 move collect/lower off parser reparsing; Task 6 routes source-backed crates through the loader; Task 3 fingerprints from loader loaded files; Task 8 adds integration regressions and final verification.
- Placeholder scan: The plan avoids deferred implementation markers and includes concrete signatures, snippets, test names, commands, and expected results.
- Type consistency: `SourceDatabase`, `ModuleGraph`, `LoadedModule`, and `SourceLoadError` names are introduced in Task 2 before later tasks reference them. Later tasks use `collect_with_source_graph`, `ModuleGraph::loaded_files`, `ModuleGraph::loaded_module_paths`, and `ModuleGraph::module_file_cache` exactly as defined earlier.
