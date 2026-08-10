# Lowerer Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish roadmap Task 18 by splitting lowering orchestration, module context, diagnostics, and body lowering into named boundaries while preserving existing lowering behavior.

**Architecture:** Keep `Lowerer` as the compatibility state shell during this slice, but move responsibilities behind narrow modules. Add `LoweringPipeline` for phase ordering, `ModuleLoweringContext` for graph/cache-backed module lookup and traversal, `LowerDiagnostics` for error state, and `BodyLowerer` for body traversal entry points.

**Tech Stack:** Rust 2021, `rock-lib`, existing AST/HIR/lower modules, `cargo fmt --all --check`, focused `cargo test -p rock-lib <filter>`, final `cargo test -p rock-lib`.

---

## Approved Spec

- `docs/superpowers/specs/2026-05-22-lowerer-decomposition-design.md`

## File Structure

- Create `lib/src/lower/pipeline.rs`: owns `LoweringPipeline`, the `lower_from_declarations` phase sequence, and final `PartialHir` assembly.
- Create `lib/src/lower/module_context.rs`: owns graph/cache-backed module lookup helpers and loaded-module traversal helpers.
- Create `lib/src/lower/diagnostics.rs`: owns `LowerDiagnostics` and compatibility accessors used by `Lowerer`.
- Create `lib/src/lower/body_lowerer.rs`: owns module body traversal entry points while using existing expression, statement, impl, and function body lowering logic.
- Modify `lib/src/lower/mod.rs`: register new modules, replace raw diagnostics fields, initialize new state, and expose compatibility methods.
- Modify `lib/src/lower/program.rs`: delegate `lower_from_declarations` to `LoweringPipeline`, delegate module lookup/traversal to `ModuleLoweringContext`, and delegate module body lowering to `BodyLowerer`.
- Modify `lib/src/lower/traits/conformance.rs`: route conformance error extension through `LowerDiagnostics` compatibility methods.
- Modify lower tests that inspect `lowerer.errors` directly to use `lowerer.errors()` after diagnostics storage moves.

## Task 1: Introduce `LoweringPipeline`

**Files:**
- Create: `lib/src/lower/pipeline.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/program.rs`
- Test: `lib/src/lower/pipeline.rs`

- [ ] **Step 1: Write the failing pipeline boundary test**

Add this complete test module to the bottom of new file `lib/src/lower/pipeline.rs` before adding the implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::Program;
    use crate::crate_system::CrateContext;
    use crate::source_loader::SourceDatabase;

    #[test]
    fn lowering_pipeline_lowers_from_declarations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rock_lower_pipeline_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let entry = temp_dir.join("main.rk");
        std::fs::write(
            &entry,
            "answer: I64\nanswer = -> 42\nmain: I64\nmain = -> answer!\n",
        )
        .unwrap();

        let config = crate::Config {
            entry_file: entry.clone(),
            no_std: true,
            no_prelude: true,
            current_crate_name: Some("demo".to_string()),
            ..crate::Config::default()
        };
        let mut db = SourceDatabase::new();
        let graph = db.load_entry(entry, &config).unwrap();
        let program = Program {
            module: graph.root_module().clone(),
        };
        let crate_ctx = CrateContext::new();
        let decls = crate::collect::collect_with_source_graph(
            &program,
            &graph,
            &crate_ctx,
            false,
            Some("demo"),
        )
        .unwrap();

        let lowered = LoweringPipeline::new(&program, &crate_ctx, Some("demo"))
            .lower_from_declarations(decls)
            .expect("pipeline should lower function bodies");

        assert!(
            lowered.functions["main"].body.stmts.len() > 0,
            "pipeline should lower the root main body"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
```

- [ ] **Step 2: Register the module and verify the test fails**

Add this line to `lib/src/lower/mod.rs` with the other lower submodules:

```rust
pub(crate) mod pipeline;
```

Run: `cargo test -p rock-lib lowering_pipeline_lowers_from_declarations -- --nocapture`

Expected: FAIL to compile with an unresolved `LoweringPipeline` type in `lib/src/lower/pipeline.rs`.

- [ ] **Step 3: Implement `LoweringPipeline`**

Replace `lib/src/lower/pipeline.rs` with this complete implementation plus the test from Step 1:

```rust
use crate::ast;
use crate::collect::Declarations;
use crate::crate_system::CrateContext;
use crate::infer::PartialHir;
use crate::lower::{Lowerer, ResolveError};

pub(crate) struct LoweringPipeline<'a> {
    program: &'a ast::Program,
    crate_ctx: &'a CrateContext,
    current_crate_name: Option<&'a str>,
}

impl<'a> LoweringPipeline<'a> {
    pub(crate) fn new(
        program: &'a ast::Program,
        crate_ctx: &'a CrateContext,
        current_crate_name: Option<&'a str>,
    ) -> Self {
        Self {
            program,
            crate_ctx,
            current_crate_name,
        }
    }

    pub(crate) fn lower_from_declarations(
        self,
        decls: Declarations,
    ) -> Result<PartialHir, Vec<ResolveError>> {
        let mut lowerer = Lowerer::from_declarations(decls);
        self.configure_current_crate(&mut lowerer);
        self.inject_prelude(&mut lowerer);
        self.prepare_traits(&mut lowerer);
        self.lower_dependency_bodies(&mut lowerer);
        self.lower_current_crate_bodies(&mut lowerer);
        self.finish(lowerer)
    }

    fn configure_current_crate(&self, lowerer: &mut Lowerer) {
        lowerer.current_crate_name = self.current_crate_name.map(ToString::to_string);

        if let Some(ref file_path) = self.program.module.filepath {
            lowerer.file_path = file_path.clone();
            lowerer.current_module_path = file_path.clone();
        }

        lowerer.register_current_crate_root();
    }

    fn inject_prelude(&self, lowerer: &mut Lowerer) {
        if lowerer.inject_prelude && self.crate_ctx.has_extern_crate("stdlib") {
            lowerer.inject_stdlib_prelude(self.crate_ctx);
        }
    }

    fn prepare_traits(&self, lowerer: &mut Lowerer) {
        lowerer.auto_impl_sized();
        lowerer.lower_crate_trait_bodies(self.crate_ctx);
        lowerer.lower_trait_default_bodies(&self.program.module);
        lowerer.lower_loaded_module_trait_defaults();
        lowerer.check_trait_conformance();
    }

    fn lower_dependency_bodies(&self, lowerer: &mut Lowerer) {
        lowerer.lower_crate_module_bodies(self.crate_ctx);

        if lowerer.inject_prelude {
            lowerer.sync_prelude_functions();
        }
    }

    fn lower_current_crate_bodies(&self, lowerer: &mut Lowerer) {
        lowerer.lower_module_bodies(&self.program.module);
        lowerer.lower_loaded_module_bodies();
        lowerer.sync_export_alias_functions();
    }

    fn finish(self, lowerer: Lowerer) -> Result<PartialHir, Vec<ResolveError>> {
        if !lowerer.errors.is_empty() {
            return Err(lowerer.errors);
        }

        Ok(PartialHir {
            functions: lowerer.functions,
            structs: lowerer.structs,
            enums: lowerer.enums,
            traits: lowerer.traits,
            impls: lowerer.impls,
            externs: lowerer.externs,
            engine: lowerer.engine,
            function_type_vars: lowerer.function_type_vars,
            import_aliases: lowerer.import_aliases,
            loaded_module_paths: lowerer.loaded_module_paths,
            constraint_store: lowerer.constraint_store,
            resolver: lowerer.resolver,
            current_def_ids: lowerer.current_def_ids,
            root_crate_id: lowerer.root_crate_id,
            local_def_ids: lowerer.local_def_ids,
        })
    }
}
```

- [ ] **Step 4: Delegate the public entry point**

In `lib/src/lower/program.rs`, add this import near the other lower imports:

```rust
use crate::lower::pipeline::LoweringPipeline;
```

Replace the body of `lower_from_declarations` with this exact delegation:

```rust
pub fn lower_from_declarations(
    program: &ast::Program,
    decls: crate::collect::Declarations,
    crate_ctx: &CrateContext,
    current_crate_name: Option<&str>,
) -> Result<crate::infer::PartialHir, Vec<ResolveError>> {
    LoweringPipeline::new(program, crate_ctx, current_crate_name).lower_from_declarations(decls)
}
```

- [ ] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib lowering_pipeline_lowers_from_declarations -- --nocapture`

Expected: PASS with 1 matching unit test.

- [ ] **Step 6: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib lower_from_declarations -- --nocapture && git diff --check`

Expected: PASS. Some test binaries may report 0 matching tests; the command must exit 0.

- [ ] **Step 7: Commit Task 1**

Run:

```bash
git add lib/src/lower/pipeline.rs lib/src/lower/mod.rs lib/src/lower/program.rs
git commit -m "introduce lowerer pipeline boundary"
```

## Task 2: Introduce `ModuleLoweringContext` Lookup Helpers

**Files:**
- Create: `lib/src/lower/module_context.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/program.rs`
- Test: `lib/src/lower/module_context.rs`

- [ ] **Step 1: Write the failing module lookup tests**

Create `lib/src/lower/module_context.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Ident, Module};
    use crate::lexer::Span;
    use crate::lower::Lowerer;

    fn empty_module(path: &std::path::Path) -> Module {
        Module {
            name: Some(Ident {
                name: "io".to_string(),
                span: Span::default(),
            }),
            top_levels: Vec::new(),
            is_inline: false,
            filepath: Some(path.to_path_buf()),
        }
    }

    #[test]
    fn module_context_prefers_current_crate_prefixed_graph_path() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rock_module_context_prefers_graph_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let alias_path = temp_dir.join("alias.rk");
        let graph_path = temp_dir.join("graph.rk");

        let mut lowerer = Lowerer::new();
        lowerer.current_crate_name = Some("test".to_string());
        lowerer
            .loaded_module_paths
            .push(("math::io".to_string(), alias_path.clone()));
        lowerer
            .loaded_module_paths
            .push(("test::math::io".to_string(), graph_path.clone()));
        lowerer
            .module_file_cache
            .insert(alias_path.clone(), empty_module(&alias_path));
        lowerer
            .module_file_cache
            .insert(graph_path.clone(), empty_module(&graph_path));

        let path = ModuleLoweringContext::loaded_path_for_module_name(&lowerer, "math::io")
            .expect("graph path should resolve");

        assert_eq!(path, graph_path);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn module_context_does_not_use_cache_without_loaded_path() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rock_module_context_requires_graph_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let cached_path = temp_dir.join("util.rk");

        let mut lowerer = Lowerer::new();
        lowerer
            .module_file_cache
            .insert(cached_path.clone(), empty_module(&cached_path));

        assert!(ModuleLoweringContext::loaded_path_for_module_name(&lowerer, "util").is_none());
        assert!(ModuleLoweringContext::cached_module_for_qualified_name(&lowerer, "util").is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
```

- [ ] **Step 2: Register the module and verify the tests fail**

Add this line to `lib/src/lower/mod.rs`:

```rust
pub(crate) mod module_context;
```

Run: `cargo test -p rock-lib module_context_ -- --nocapture`

Expected: FAIL to compile with unresolved `ModuleLoweringContext`.

- [ ] **Step 3: Implement lookup helpers**

Insert this implementation above the test module in `lib/src/lower/module_context.rs`:

```rust
use std::path::{Path, PathBuf};

use crate::ast;
use crate::lower::Lowerer;

pub(crate) struct ModuleLoweringContext;

impl ModuleLoweringContext {
    pub(crate) fn cached_module_for_qualified_name(
        lowerer: &Lowerer,
        qualified_name: &str,
    ) -> Option<ast::Module> {
        Self::loaded_path_for_module_name(lowerer, qualified_name)
            .and_then(|path| Self::cached_module_for_path(lowerer, &path))
    }

    pub(crate) fn cached_module_for_path(
        lowerer: &Lowerer,
        path: &Path,
    ) -> Option<ast::Module> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        lowerer
            .module_file_cache
            .get(&canonical_path)
            .or_else(|| lowerer.module_file_cache.get(path))
            .cloned()
    }

    pub(crate) fn loaded_path_for_module_name(
        lowerer: &Lowerer,
        qualified_module_name: &str,
    ) -> Option<PathBuf> {
        Self::loaded_path_for_current_crate_module_name(lowerer, qualified_module_name)
            .or_else(|| Self::loaded_path_for_exact_module_name(lowerer, qualified_module_name))
    }

    pub(crate) fn loaded_path_for_exact_module_name(
        lowerer: &Lowerer,
        qualified_module_name: &str,
    ) -> Option<PathBuf> {
        lowerer
            .loaded_module_paths
            .iter()
            .find(|(name, _)| name == qualified_module_name)
            .map(|(_, path)| path.clone())
    }

    fn loaded_path_for_current_crate_module_name(
        lowerer: &Lowerer,
        qualified_module_name: &str,
    ) -> Option<PathBuf> {
        let crate_name = lowerer.current_crate_name.as_ref()?;
        if qualified_module_name == crate_name
            || qualified_module_name.starts_with(&format!("{}::", crate_name))
        {
            return None;
        }

        let first_segment = qualified_module_name
            .split("::")
            .next()
            .unwrap_or(qualified_module_name);
        if lowerer
            .loaded_module_paths
            .iter()
            .any(|(name, _)| name == first_segment)
        {
            return None;
        }

        Self::loaded_path_for_exact_module_name(
            lowerer,
            &format!("{}::{}", crate_name, qualified_module_name),
        )
    }

    pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
        let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
        let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
        left == right
    }
}
```

- [ ] **Step 4: Delegate existing `Lowerer` helpers**

In `lib/src/lower/program.rs`, add:

```rust
use crate::lower::module_context::ModuleLoweringContext;
```

Replace the helper bodies in `impl Lowerer` with these delegations:

```rust
fn cached_module_for_qualified_name(&self, qualified_name: &str) -> Option<ast::Module> {
    ModuleLoweringContext::cached_module_for_qualified_name(self, qualified_name)
}

fn cached_module_for_path(&self, path: &std::path::Path) -> Option<ast::Module> {
    ModuleLoweringContext::cached_module_for_path(self, path)
}

fn should_skip_loaded_module_path(&self, file_path: &std::path::Path) -> bool {
    !self.file_path.as_os_str().is_empty()
        && ModuleLoweringContext::same_path(file_path, &self.file_path)
}

fn loaded_path_for_module_name(&self, qualified_module_name: &str) -> Option<PathBuf> {
    ModuleLoweringContext::loaded_path_for_module_name(self, qualified_module_name)
}
```

Delete the old `loaded_path_for_exact_module_name`, `loaded_path_for_current_crate_module_name`, and free `same_path` implementations from `program.rs` after the delegations compile.

- [ ] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib module_context_ -- --nocapture`

Expected: PASS with the two new module-context tests.

- [ ] **Step 6: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib load_module_prefers_current_crate_prefixed_graph_path -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 7: Commit Task 2**

Run:

```bash
git add lib/src/lower/module_context.rs lib/src/lower/mod.rs lib/src/lower/program.rs
git commit -m "extract lowerer module lookup context"
```

## Task 3: Move Loaded-Module Traversal Behind `ModuleLoweringContext`

**Files:**
- Modify: `lib/src/lower/module_context.rs`
- Modify: `lib/src/lower/program.rs`
- Test: `lib/src/lower/module_context.rs`

- [ ] **Step 1: Add a failing traversal test**

Append this test to the existing `tests` module in `lib/src/lower/module_context.rs`:

```rust
#[test]
fn module_context_iterates_loaded_modules_from_cache_and_skips_root() {
    let temp_dir = std::env::temp_dir().join(format!(
        "rock_module_context_iter_loaded_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let root_path = temp_dir.join("main.rk");
    let child_path = temp_dir.join("child.rk");
    let child = empty_module(&child_path);

    let mut lowerer = Lowerer::new();
    lowerer.file_path = root_path.clone();
    lowerer.current_module_path = root_path.clone();
    lowerer
        .loaded_module_paths
        .push(("demo".to_string(), root_path.clone()));
    lowerer
        .loaded_module_paths
        .push(("demo::child".to_string(), child_path.clone()));
    lowerer.module_file_cache.insert(child_path.clone(), child);

    let mut seen = Vec::new();
    ModuleLoweringContext::for_each_loaded_module(&mut lowerer, |_, module_name, module| {
        seen.push((module_name.to_string(), module.filepath.clone()));
    });

    assert_eq!(seen, vec![("demo::child".to_string(), Some(child_path))]);
    assert!(lowerer.errors.is_empty(), "unexpected errors: {:?}", lowerer.errors);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p rock-lib module_context_iterates_loaded_modules_from_cache_and_skips_root -- --nocapture`

Expected: FAIL to compile with unresolved `for_each_loaded_module`.

- [ ] **Step 3: Implement traversal helper**

Add this method inside `impl ModuleLoweringContext` in `lib/src/lower/module_context.rs`:

```rust
pub(crate) fn for_each_loaded_module<F>(lowerer: &mut Lowerer, mut visit: F)
where
    F: FnMut(&mut Lowerer, &str, &ast::Module),
{
    let module_paths = lowerer.loaded_module_paths.clone();

    for (module_name, file_path) in &module_paths {
        if !lowerer.file_path.as_os_str().is_empty()
            && Self::same_path(file_path, &lowerer.file_path)
        {
            continue;
        }

        match Self::cached_module_for_path(lowerer, file_path) {
            Some(loaded_module) => {
                let old_path = lowerer.current_module_path.clone();
                lowerer.current_module_path = file_path.clone();
                visit(lowerer, module_name, &loaded_module);
                lowerer.current_module_path = old_path;
            }
            None => lowerer.push_missing_cached_module_error(module_name, file_path, None),
        }
    }
}
```

Change `push_missing_cached_module_error` in `lib/src/lower/program.rs` from private to `pub(crate)` so `module_context.rs` can report the same diagnostic:

```rust
pub(crate) fn push_missing_cached_module_error(
    &mut self,
    module_name: &str,
    file_path: &std::path::Path,
    span: Option<crate::lexer::Span>,
) {
```

- [ ] **Step 4: Delegate loaded body traversal**

Replace `lower_loaded_module_bodies` in `lib/src/lower/program.rs` with:

```rust
pub(crate) fn lower_loaded_module_bodies(&mut self) {
    ModuleLoweringContext::for_each_loaded_module(self, |lowerer, module_name, loaded_module| {
        lowerer.lower_module_bodies_qualified(loaded_module, Some(module_name));
    });
}
```

Replace `lower_loaded_module_trait_defaults` in `lib/src/lower/program.rs` with:

```rust
pub(crate) fn lower_loaded_module_trait_defaults(&mut self) {
    ModuleLoweringContext::for_each_loaded_module(self, |lowerer, module_name, loaded_module| {
        lowerer.scope.push();
        let added_aliases = lowerer.inject_module_local_aliases(loaded_module, module_name);
        lowerer.lower_trait_default_bodies(loaded_module);
        for alias in added_aliases {
            lowerer.module_local_aliases.remove(&alias);
        }
        lowerer.scope.pop();
    });
}
```

- [ ] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib module_context_iterates_loaded_modules_from_cache_and_skips_root -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run module behavior verification**

Run: `cargo test -p rock-lib lower_loaded_module -- --nocapture`

Expected: PASS for all matching tests, including directory-backed loaded modules and trait defaults.

- [ ] **Step 7: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib lower_from_declarations_accepts_graph_only_current_crate_prefixed_nested_inline_module_path -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 8: Commit Task 3**

Run:

```bash
git add lib/src/lower/module_context.rs lib/src/lower/program.rs
git commit -m "move loaded module traversal into module context"
```

## Task 4: Introduce `LowerDiagnostics`

**Files:**
- Create: `lib/src/lower/diagnostics.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify tests under `lib/src/lower/**` that read `lowerer.errors` directly
- Test: `lib/src/lower/diagnostics.rs`

- [ ] **Step 1: Write failing diagnostics tests**

Create `lib/src/lower/diagnostics.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::lexer::Span;

    #[test]
    fn diagnostics_attach_current_span_and_deduplicate_messages() {
        let mut diagnostics = LowerDiagnostics::new();
        diagnostics.set_current_span(Some(Span {
            file_path: "main.rk".into(),
            start: 3,
            end: 9,
        }));

        diagnostics.push("same message".to_string());
        diagnostics.push_once("same message".to_string());
        diagnostics.push_once("other message".to_string());

        assert_eq!(diagnostics.errors().len(), 2);
        assert_eq!(diagnostics.errors()[0].message, "same message");
        assert_eq!(diagnostics.errors()[0].span.as_ref().unwrap().start, 3);
        assert_eq!(diagnostics.errors()[1].message, "other message");
    }

    #[test]
    fn diagnostics_push_with_explicit_span_overrides_current_span() {
        let mut diagnostics = LowerDiagnostics::new();
        diagnostics.set_current_span(Some(Span {
            file_path: "main.rk".into(),
            start: 1,
            end: 2,
        }));
        diagnostics.push_with_span(
            "explicit".to_string(),
            Span {
                file_path: "child.rk".into(),
                start: 10,
                end: 14,
            },
        );

        assert_eq!(diagnostics.errors()[0].span.as_ref().unwrap().file_path, "child.rk");
        assert_eq!(diagnostics.errors()[0].span.as_ref().unwrap().start, 10);
    }
}
```

- [ ] **Step 2: Register the module and verify tests fail**

Add this line to `lib/src/lower/mod.rs`:

```rust
pub(crate) mod diagnostics;
```

Run: `cargo test -p rock-lib diagnostics_ -- --nocapture`

Expected: FAIL to compile with unresolved `LowerDiagnostics`.

- [ ] **Step 3: Implement diagnostics storage**

Insert this implementation above the tests in `lib/src/lower/diagnostics.rs`:

```rust
use crate::lexer::Span;
use crate::lower::ResolveError;

#[derive(Debug, Default, Clone)]
pub(crate) struct LowerDiagnostics {
    errors: Vec<ResolveError>,
    current_span: Option<Span>,
}

impl LowerDiagnostics {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_current_span(&mut self, span: Option<Span>) {
        self.current_span = span;
    }

    pub(crate) fn current_span(&self) -> Option<&Span> {
        self.current_span.as_ref()
    }

    pub(crate) fn push(&mut self, message: String) {
        self.errors.push(ResolveError {
            message,
            span: self.current_span.clone(),
        });
    }

    pub(crate) fn push_once(&mut self, message: String) {
        if !self.errors.iter().any(|error| error.message == message) {
            self.push(message);
        }
    }

    pub(crate) fn push_with_span(&mut self, message: String, span: Span) {
        self.errors.push(ResolveError {
            message,
            span: Some(span),
        });
    }

    pub(crate) fn extend(&mut self, errors: Vec<ResolveError>) {
        self.errors.extend(errors);
    }

    pub(crate) fn has_message(&self, message: &str) -> bool {
        self.errors.iter().any(|error| error.message == message)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(crate) fn errors(&self) -> &[ResolveError] {
        &self.errors
    }

    pub(crate) fn into_errors(self) -> Vec<ResolveError> {
        self.errors
    }
}
```

- [ ] **Step 4: Replace raw fields on `Lowerer`**

In `lib/src/lower/mod.rs`, add:

```rust
use diagnostics::LowerDiagnostics;
```

Replace these fields in `pub struct Lowerer`:

```rust
pub(crate) errors: Vec<ResolveError>,
pub(crate) current_span: Option<Span>,
```

with:

```rust
pub(crate) diagnostics: LowerDiagnostics,
```

In `Lowerer::new`, `Lowerer::with_options`, and `Lowerer::from_declarations`, replace `errors: Vec::new(),` and `current_span: None,` with:

```rust
diagnostics: LowerDiagnostics::new(),
```

- [ ] **Step 5: Add compatibility methods on `Lowerer`**

Replace the existing error helper methods in `impl Lowerer` in `lib/src/lower/mod.rs` with:

```rust
pub(crate) fn push_error(&mut self, message: String) {
    self.diagnostics.push(message);
}

pub(crate) fn push_error_once(&mut self, message: String) {
    self.diagnostics.push_once(message);
}

pub(crate) fn push_error_with_span(&mut self, message: String, span: Span) {
    self.diagnostics.push_with_span(message, span);
}

pub(crate) fn extend_errors(&mut self, errors: Vec<ResolveError>) {
    self.diagnostics.extend(errors);
}

pub(crate) fn has_errors(&self) -> bool {
    !self.diagnostics.is_empty()
}

pub(crate) fn errors(&self) -> &[ResolveError] {
    self.diagnostics.errors()
}

pub(crate) fn set_current_span(&mut self, span: Option<Span>) {
    self.diagnostics.set_current_span(span);
}
```

- [ ] **Step 6: Migrate production call sites**

Make these exact production changes:

- In `lib/src/lower/program.rs`, change duplicate check in `push_missing_cached_module_error` to:

```rust
if self.diagnostics.has_message(&message) {
    return;
}
```

- In `lib/src/lower/pipeline.rs`, change final error handling to:

```rust
if lowerer.has_errors() {
    return Err(lowerer.diagnostics.into_errors());
}
```

- In `lib/src/lower/program.rs` legacy `lower_program_with_crates`, change final error handling to:

```rust
if self.has_errors() {
    return Err(self.diagnostics.into_errors());
}
```

- In `lib/src/lower/traits/conformance.rs`, replace:

```rust
self.errors.extend(conformance_errors);
```

with:

```rust
self.extend_errors(conformance_errors);
```

- In `lib/src/lower/paths.rs`, replace:

```rust
self.current_span = Some(ident.span.clone());
```

with:

```rust
self.set_current_span(Some(ident.span.clone()));
```

- In `lib/src/lower/paths.rs`, replace:

```rust
self.current_span = Some(inner.span.clone());
```

with:

```rust
self.set_current_span(Some(inner.span.clone()));
```

- In `lib/src/lower/expression.rs`, replace:

```rust
self.current_span = Some(lit.span.clone());
```

with:

```rust
self.set_current_span(Some(lit.span.clone()));
```

- In `lib/src/lower/bodies.rs`, replace:

```rust
self.current_span = Some(fd.name.span.clone());
```

with:

```rust
self.set_current_span(Some(fd.name.span.clone()));
```

- In `lib/src/lower/bodies.rs`, replace:

```rust
self.current_span = Some(method_ident.span.clone());
```

with:

```rust
self.set_current_span(Some(method_ident.span.clone()));
```

Use `grep` to locate these direct references before editing:

Run: `rg "self\.current_span|self\.errors|lowerer\.errors" lib/src/lower`

Expected after migration: only test references remain, and those should be converted in the next step.

- [ ] **Step 7: Migrate lowerer tests that inspect errors**

In lower tests, replace direct field reads with accessor calls:

```rust
lowerer.errors
```

becomes:

```rust
lowerer.errors()
```

Examples:

```rust
assert!(lowerer.errors().is_empty());
assert_eq!(lowerer.errors().len(), 1);
assert!(lowerer.errors()[0].message.contains("Missing stdlib prelude declaration"));
```

- [ ] **Step 8: Run focused verification**

Run: `cargo test -p rock-lib diagnostics_ -- --nocapture`

Expected: PASS with the diagnostics tests.

- [ ] **Step 9: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib lowerer_from_declarations -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 10: Commit Task 4**

Run:

```bash
git add lib/src/lower/diagnostics.rs lib/src/lower/mod.rs lib/src/lower/program.rs lib/src/lower/traits/conformance.rs lib/src/lower
git commit -m "isolate lowerer diagnostics state"
```

## Task 5: Add `BodyLowerer` For Module Body Traversal

**Files:**
- Create: `lib/src/lower/body_lowerer.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/program.rs`
- Test: `lib/src/lower/body_lowerer.rs`

- [ ] **Step 1: Write a failing body-boundary test**

Create `lib/src/lower/body_lowerer.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::Program;
    use crate::crate_system::CrateContext;
    use crate::lower::Lowerer;
    use crate::source_loader::SourceDatabase;

    #[test]
    fn body_lowerer_lowers_inline_and_source_backed_module_bodies() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rock_body_lowerer_modules_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let entry = temp_dir.join("main.rk");
        let helper = temp_dir.join("helper.rk");
        std::fs::write(&entry, "mod helper\nmain: I64\nmain = -> helper::answer!\n").unwrap();
        std::fs::write(&helper, "answer: I64\nanswer = -> 7\n< answer\n").unwrap();

        let config = crate::Config {
            entry_file: entry,
            no_std: true,
            no_prelude: true,
            current_crate_name: Some("demo".to_string()),
            ..crate::Config::default()
        };
        let mut db = SourceDatabase::new();
        let graph = db.load_entry(config.entry_file.clone(), &config).unwrap();
        let program = Program {
            module: graph.root_module().clone(),
        };
        let crate_ctx = CrateContext::new();
        let decls = crate::collect::collect_with_source_graph(
            &program,
            &graph,
            &crate_ctx,
            false,
            Some("demo"),
        )
        .unwrap();
        let mut lowerer = Lowerer::from_declarations(decls);
        lowerer.current_crate_name = Some("demo".to_string());
        lowerer.file_path = config.entry_file.clone();
        lowerer.current_module_path = config.entry_file.clone();

        BodyLowerer::new(&mut lowerer).lower_root_and_loaded_modules(&program.module);

        assert!(
            lowerer.functions["main"].body.stmts.len() > 0,
            "root body should be lowered"
        );
        assert!(
            lowerer.functions["demo::helper::answer"].body.stmts.len() > 0,
            "source-backed helper body should be lowered"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
```

- [ ] **Step 2: Register the module and verify the test fails**

Add this line to `lib/src/lower/mod.rs`:

```rust
pub(crate) mod body_lowerer;
```

Run: `cargo test -p rock-lib body_lowerer_lowers_inline_and_source_backed_module_bodies -- --nocapture`

Expected: FAIL to compile with unresolved `BodyLowerer`.

- [ ] **Step 3: Implement `BodyLowerer` as the body traversal boundary**

Insert this implementation above the tests in `lib/src/lower/body_lowerer.rs`:

```rust
use crate::ast;
use crate::lower::module_context::ModuleLoweringContext;
use crate::lower::Lowerer;

pub(crate) struct BodyLowerer<'a> {
    lowerer: &'a mut Lowerer,
}

impl<'a> BodyLowerer<'a> {
    pub(crate) fn new(lowerer: &'a mut Lowerer) -> Self {
        Self { lowerer }
    }

    pub(crate) fn lower_root_and_loaded_modules(&mut self, root_module: &ast::Module) {
        self.lower_module(root_module);
        self.lower_loaded_modules();
    }

    pub(crate) fn lower_module(&mut self, module: &ast::Module) {
        self.lower_module_qualified(module, None);
    }

    pub(crate) fn lower_module_qualified(
        &mut self,
        module: &ast::Module,
        module_prefix: Option<&str>,
    ) {
        self.lowerer.lower_module_bodies_qualified_impl(module, module_prefix);
    }

    pub(crate) fn lower_loaded_modules(&mut self) {
        ModuleLoweringContext::for_each_loaded_module(
            self.lowerer,
            |lowerer, module_name, loaded_module| {
                lowerer.lower_module_bodies_qualified_impl(loaded_module, Some(module_name));
            },
        );
    }
}
```

- [ ] **Step 4: Rename the old body implementation and delegate compatibility methods**

In `lib/src/lower/program.rs`, add:

```rust
use crate::lower::body_lowerer::BodyLowerer;
```

Rename the existing implementation method:

```rust
pub(crate) fn lower_module_bodies_qualified(
```

to:

```rust
pub(crate) fn lower_module_bodies_qualified_impl(
```

Then replace the compatibility methods with:

```rust
pub(crate) fn lower_module_bodies(&mut self, module: &ast::Module) {
    BodyLowerer::new(self).lower_module(module);
}

pub(crate) fn lower_module_bodies_qualified(
    &mut self,
    module: &ast::Module,
    module_prefix: Option<&str>,
) {
    BodyLowerer::new(self).lower_module_qualified(module, module_prefix);
}

pub(crate) fn lower_loaded_module_bodies(&mut self) {
    BodyLowerer::new(self).lower_loaded_modules();
}
```

Inside `lower_module_bodies_qualified_impl`, update recursive calls from `lower_module_bodies_qualified` to `lower_module_bodies_qualified_impl` so recursion does not repeatedly allocate compatibility wrappers:

```rust
Some(ref p) => self.lower_module_bodies_qualified_impl(&module_decl.0, Some(p)),
None => self.lower_module_bodies_qualified_impl(&module_decl.0, None),
```

and:

```rust
self.lower_module_bodies_qualified_impl(&loaded_module, Some(&new_prefix));
```

- [ ] **Step 5: Update the pipeline to use `BodyLowerer` explicitly**

In `lib/src/lower/pipeline.rs`, add:

```rust
use crate::lower::body_lowerer::BodyLowerer;
```

Replace `lower_current_crate_bodies` with:

```rust
fn lower_current_crate_bodies(&self, lowerer: &mut Lowerer) {
    BodyLowerer::new(lowerer).lower_root_and_loaded_modules(&self.program.module);
    lowerer.sync_export_alias_functions();
}
```

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib body_lowerer_lowers_inline_and_source_backed_module_bodies -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run body/module behavior verification**

Run: `cargo test -p rock-lib lower_from_declarations_resolves_nested -- --nocapture`

Expected: PASS for matching nested-source module tests.

- [ ] **Step 8: Run task verification**

Run: `cargo fmt --all --check && cargo test -p rock-lib lower_loaded_module -- --nocapture && git diff --check`

Expected: PASS.

- [ ] **Step 9: Commit Task 5**

Run:

```bash
git add lib/src/lower/body_lowerer.rs lib/src/lower/mod.rs lib/src/lower/program.rs lib/src/lower/pipeline.rs
git commit -m "add body lowerer boundary"
```

## Task 6: Final Cleanup And Full Verification

**Files:**
- Modify: `lib/src/lower/pipeline.rs`
- Modify: `lib/src/lower/module_context.rs`
- Modify: `lib/src/lower/diagnostics.rs`
- Modify: `lib/src/lower/body_lowerer.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/mod.rs`
- Test: existing `rock-lib` tests

- [x] **Step 1: Search for decomposition leaks**

Run:

```bash
rg "current_qualified_module_prefix|module_file_cache|loaded_module_paths|module_local_aliases" lib/src/lower --glob '*.rs'
```

Expected: Remaining production hits are either in `module_context.rs`, `body_lowerer.rs`, compatibility wrappers in `program.rs`, or existing path-resolution code that still legitimately consumes module-local aliases. If new direct module lookup logic appears outside those files, move it behind `ModuleLoweringContext` before continuing.

- [x] **Step 2: Search for raw diagnostics leaks**

Run:

```bash
rg "self\.errors|lowerer\.errors|current_span" lib/src/lower --glob '*.rs'
```

Expected: No production direct access to raw diagnostics fields. Tests should use `lowerer.errors()`.

- [x] **Step 3: Search for forbidden source path reconstruction in lowering**

Run:

```bash
rg "parent\(\)|set_extension|join\(" lib/src/lower --glob '*.rs'
```

Expected: No production module-loading sibling path reconstruction. Hits for temp test paths or unrelated type/string helpers are acceptable only if they do not load source modules.

- [x] **Step 4: Run focused boundary tests**

Run:

```bash
cargo test -p rock-lib lowering_pipeline_lowers_from_declarations -- --nocapture
cargo test -p rock-lib module_context_ -- --nocapture
cargo test -p rock-lib diagnostics_ -- --nocapture
cargo test -p rock-lib body_lowerer_lowers_inline_and_source_backed_module_bodies -- --nocapture
```

Expected: all commands PASS.

- [x] **Step 5: Run full verification**

Run:

```bash
cargo fmt --all --check
cargo test -p rock-lib
git diff --check
```

Expected: `cargo fmt --all --check` exits 0, `cargo test -p rock-lib` exits 0, and `git diff --check` exits 0.

- [x] **Step 6: Request final review**

Use the requesting-code-review skill with this context:

```text
Description: Finished roadmap Task 18 Lowerer decomposition. Added LoweringPipeline, ModuleLoweringContext, LowerDiagnostics, and BodyLowerer boundaries while preserving lowering behavior.

Spec: docs/superpowers/specs/2026-05-22-lowerer-decomposition-design.md
Plan: docs/superpowers/plans/2026-05-22-lowerer-decomposition.md

Review focus:
- lower_from_declarations phase ordering still matches previous behavior
- module lookup/traversal stays graph/cache-only with no filesystem probing
- diagnostics remain span-aware and deduplicated where expected
- BodyLowerer does not own module loading or pipeline setup policy
- no broad behavior changes or compatibility map removals slipped in
```

Expected: reviewer returns `STATUS: PASS` or findings. Fix Critical and Important findings before continuing.

- [ ] **Step 7: Commit final cleanup**

If Step 6 review passes and there are uncommitted changes, run:

```bash
git add lib/src/lower
git commit -m "finish lowerer decomposition boundaries"
```

If there are no uncommitted changes after prior task commits, do not create an empty commit.

2026-05-25 note: skipped because commits were not requested in this session.

## Completion Check

- [x] `LoweringPipeline` owns `lower_from_declarations` phase ordering.
- [x] `ModuleLoweringContext` owns graph/cache-backed module lookup and loaded-module traversal helpers.
- [x] `LowerDiagnostics` owns diagnostic storage and push helpers.
- [x] `BodyLowerer` owns root and loaded module body traversal entry points.
- [x] No source-module filesystem probing was added to lowering.
- [x] Existing public lowering entry points still compile and pass tests.
- [x] Final verification commands pass: `cargo fmt --all --check`, `cargo test -p rock-lib`, `git diff --check`.

## 2026-05-25 Completion Note

Task 18 is complete for the scoped lowerer-decomposition slice. `LoweringPipeline` now owns the `lower_from_declarations` phase sequence, `ModuleLoweringContext` owns graph/cache-backed lookup, loaded-module traversal, loaded-root classification, module-local alias cleanup, and qualified module prefix scoping, `LowerDiagnostics` owns error/span state, and `BodyLowerer` owns root and loaded-module body traversal entry points while preserving the existing lowering behavior.

Verification:
- Focused boundary and behavior filters passed, including `lowering_pipeline_lowers_from_declarations`, `module_context_`, `diagnostics_`, `body_lowerer_lowers_inline_and_source_backed_module_bodies`, `lower_loaded_module`, `lower_from_declarations_resolves_nested`, `lower_from_declarations_accepts_graph_only_current_crate_prefixed_nested_inline_module_path`, and `lowerer_from_declarations`.
- Final verification passed with `cargo fmt --all --check && cargo test -p rock-lib > /tmp/rock-lib-task18-final.log 2>&1 && git diff --check`.
- `/tmp/rock-lib-task18-final.log`: unit tests `1253 passed; 0 failed; 1 ignored`; integration tests `276 passed; 0 failed`; parser integration test `1 passed`; doctests `1 passed; 1 ignored`.
- Final code review returned `STATUS: PASS` with no Critical or Important findings.
