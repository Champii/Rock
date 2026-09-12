# Pipeline Separation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split `resolve/` into three top-level pipeline stages — `collect/`, `lower/`, `infer/` — with explicit data-flow contracts between them, then delete `resolve/`.

**Architecture:** `Collector` (in `collect/`) does first-pass declaration gathering and produces `Declarations`. `Lowerer` (in `lower/`) takes `Declarations`, lowers function bodies, and produces `PartialHir` (HIR + unsolved type state). `infer::finalize` applies type substitutions and generalization to produce the final `HirProgram`. The `InferenceEngine` is threaded from `Collector` → `Declarations` → `Lowerer` → `PartialHir` → `infer::finalize` to keep type variable IDs consistent.

**Tech Stack:** Rust, Cargo workspace (`rock-lib` crate). Tests: `cargo test -p rock-lib`. No new dependencies.

---

## Task 0: Delete temp/ and fix existing warnings

**Files:**
- Delete: `lib/src/resolve/temp/` (entire directory)
- Modify: `lib/src/resolve/collect/mod.rs`
- Modify: `lib/src/resolve/crates/mod.rs`
- Modify: `lib/src/resolve/traits/mod.rs`
- Modify: `lib/src/resolve/context.rs` (line 11)
- Modify: `lib/src/resolve/lower/bodies.rs` (line 7)

**Step 1: Delete the temp directory**
```bash
rm -rf lib/src/resolve/temp
```
This is safe — `temp/` is never referenced by any `mod` declaration in the project.

**Step 2: Fix the unused `pub use *` re-exports in submodule mod.rs files**

In `lib/src/resolve/collect/mod.rs`, replace:
```rust
pub use declarations::*;
pub use traits::*;
pub use types::*;
```
with nothing — remove all three lines. These `pub use *` re-export nothing useful because the submodules only contain `impl Lowerer { ... }` blocks, which don't export standalone items.

Same for `lib/src/resolve/crates/mod.rs` — remove:
```rust
pub use bodies::*;
pub use registration::*;
```

Same for `lib/src/resolve/traits/mod.rs` — remove:
```rust
pub use conformance::*;
pub use defaults::*;
```

**Step 3: Fix unused import in context.rs**

In `lib/src/resolve/context.rs` line 11, remove:
```rust
use crate::types::Type;
```
`Type` is already in scope via `crate::hir::*`.

**Step 4: Fix unused import in lower/bodies.rs**

In `lib/src/resolve/lower/bodies.rs` line 7, remove or check:
```rust
use crate::hir::*;
```
If `hir::*` items are used in that file, keep it. If all are covered by existing imports, remove.

**Step 5: Build and confirm warnings are gone**
```bash
cargo build -p rock-lib 2>&1 | grep -E "warning|error"
```
Expected: zero unused-import warnings in those files.

**Step 6: Run tests**
```bash
cargo test -p rock-lib 2>&1 | tail -20
```
Expected: all existing tests pass.

**Step 7: Commit**
```bash
git add -A
git commit -m "chore: delete temp/ and fix unused import warnings"
```

---

## Task 1: Create `infer/` — move InferenceEngine and type finalization

The inference engine and all type-finalization logic move to `lib/src/infer/`. The `PartialHir` struct (output of `lower`, input to `infer::finalize`) is defined here.

**Files:**
- Create: `lib/src/infer/mod.rs`
- Create: `lib/src/infer/engine.rs`  (was `resolve/inference/engine.rs`)
- Create: `lib/src/infer/finalize.rs` (was `resolve/types/finalize.rs`)
- Create: `lib/src/infer/generalize.rs` (was `resolve/types/generalize.rs`)
- Create: `lib/src/infer/type_vars.rs` (was `resolve/types/type_vars.rs`)
- Create: `lib/src/infer/helpers.rs`  (was `resolve/types/helpers.rs`)
- Modify: `lib/src/lib.rs`

**Step 1: Copy engine.rs**

Copy `lib/src/resolve/inference/engine.rs` → `lib/src/infer/engine.rs`, content unchanged. The file has no references to `Lowerer`, it only imports `crate::types::{TraitBound, Type}`, so no path changes needed.

**Step 2: Define `PartialHir` in `lib/src/infer/mod.rs`**

Create `lib/src/infer/mod.rs`:
```rust
//! Type inference finalization — converts PartialHir (with type variables)
//! into a fully-typed HirProgram.

mod engine;
mod finalize;
mod generalize;
mod helpers;
mod type_vars;

pub use engine::InferenceEngine;

use std::collections::{BTreeMap, BTreeSet};

use crate::hir::{HirProgram, HirFunction, HirStruct, HirEnum, HirTrait, HirImpl, HirExtern};
use crate::lower::error::CompileError;

/// HIR with type variables still present — output of the lower stage.
pub struct PartialHir {
    pub functions: BTreeMap<String, HirFunction>,
    pub structs: BTreeMap<String, HirStruct>,
    pub enums: BTreeMap<String, HirEnum>,
    pub traits: BTreeMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    pub engine: InferenceEngine,
    /// Type variables created per function (for generalization)
    pub function_type_vars: BTreeMap<String, BTreeSet<u32>>,
    /// Import aliases to resolve after finalization
    pub import_aliases: BTreeMap<String, String>,
}

/// Finalize types: apply inference substitutions, generalize, resolve aliases.
/// Consumes PartialHir and produces a fully-typed HirProgram.
pub fn finalize(mut hir: PartialHir) -> Result<HirProgram, Vec<CompileError>> {
    generalize::generalize_all_functions(&mut hir);
    finalize::apply_finalization(&mut hir);
    resolve_import_aliases(&mut hir);

    Ok(HirProgram {
        functions: hir.functions,
        structs: hir.structs,
        enums: hir.enums,
        traits: hir.traits,
        impls: hir.impls,
        externs: hir.externs,
    })
}

fn resolve_import_aliases(hir: &mut PartialHir) {
    for (short_name, qualified_name) in &hir.import_aliases {
        if let Some(func) = hir.functions.get(qualified_name).cloned() {
            hir.functions.insert(short_name.clone(), func);
        }
    }
}
```

**Step 3: Port finalize.rs**

Create `lib/src/infer/finalize.rs`. The original `resolve/types/finalize.rs` has `impl Lowerer` methods. Convert to free functions operating on `PartialHir`:

```rust
//! Type finalization: apply inference engine substitutions to all HIR nodes.

use crate::hir::*;
use crate::types::Type;
use super::PartialHir;

pub(super) fn apply_finalization(hir: &mut PartialHir) {
    // Finalize functions
    let func_names: Vec<String> = hir.functions.keys().cloned().collect();
    for name in func_names {
        if let Some(mut func) = hir.functions.remove(&name) {
            finalize_function(&hir.engine, &mut func);
            hir.functions.insert(name, func);
        }
    }

    // Finalize impl methods
    let mut impls = std::mem::take(&mut hir.impls);
    for imp in &mut impls {
        let method_names: Vec<String> = imp.methods.keys().cloned().collect();
        for name in method_names {
            if let Some(mut func) = imp.methods.remove(&name) {
                finalize_function(&hir.engine, &mut func);
                imp.methods.insert(name, func);
            }
        }
    }
    hir.impls = impls;

    // Finalize struct fields
    let struct_names: Vec<String> = hir.structs.keys().cloned().collect();
    for name in struct_names {
        if let Some(mut s) = hir.structs.remove(&name) {
            for field in &mut s.fields {
                field.ty = hir.engine.finalize(&field.ty);
            }
            hir.structs.insert(name, s);
        }
    }

    // Finalize externs
    for ext in &mut hir.externs {
        for param in &mut ext.params {
            *param = hir.engine.finalize(param);
        }
        ext.ret = hir.engine.finalize(&ext.ret);
    }
}

// -- below: port all existing finalize_function, finalize_block, finalize_stmt,
//    finalize_expr methods from resolve/types/finalize.rs, changing
//    `self.engine.finalize(...)` to `engine.finalize(...)` (take engine as param).
//    The logic is identical — just threading `engine` explicitly instead of via self.
```

Port the remaining helper methods (`finalize_function`, `finalize_block`, `finalize_stmt`, `finalize_expr`) from `resolve/types/finalize.rs` verbatim, replacing `self.engine` with the `engine` parameter passed in.

**Step 4: Port generalize.rs**

Create `lib/src/infer/generalize.rs`. Port from `resolve/types/generalize.rs`. Same pattern: `impl Lowerer` → free functions taking `&mut PartialHir` or `&PartialHir`.

Key signature changes:
- `pub(crate) fn generalize_all_functions(&mut self)` → `pub(super) fn generalize_all_functions(hir: &mut PartialHir)`
- `pub(crate) fn generalize_single_function(&self, func: HirFunction) -> HirFunction` → `pub(super) fn generalize_single_function(engine: &super::InferenceEngine, function_type_vars: &BTreeMap<String, BTreeSet<u32>>, func: HirFunction) -> HirFunction`

Inside `generalize_all_functions`, replace `self.functions`, `self.impls`, `self.function_type_vars`, `self.engine` with `hir.functions`, `hir.impls`, `hir.function_type_vars`, `hir.engine`.

**Step 5: Port type_vars.rs and helpers.rs**

Create `lib/src/infer/type_vars.rs` and `lib/src/infer/helpers.rs`. These contain `impl Lowerer` blocks for helper methods (`collect_type_vars`, `collect_type_vars_block`, etc.).

Convert to free functions taking explicit parameters instead of `self`. Remove any dead-code methods identified in the warnings (dead methods can simply be deleted — they're never called).

Check what methods are actually called from `generalize.rs` and `finalize.rs` and only keep those.

**Step 6: Add `InferenceEngine::finalize` method if missing**

The `finalize_types` code calls `self.engine.finalize(&ty)`. Check if `InferenceEngine` has a `finalize` method. If it's just `resolve` by another name, add:
```rust
// In infer/engine.rs
pub fn finalize(&self, ty: &Type) -> Type {
    self.resolve(ty)
}
```

**Step 7: Register `infer` in lib.rs (as pub mod, not yet wired into pipeline)**

In `lib/src/lib.rs`, add:
```rust
pub mod infer;
```
Keep existing `pub mod resolve;` for now. We'll switch the pipeline in a later task.

**Step 8: Build**
```bash
cargo build -p rock-lib 2>&1 | grep -E "^error"
```
Expected: no errors (warnings about unused `infer` are OK at this stage).

**Step 9: Commit**
```bash
git add lib/src/infer/
git add lib/src/lib.rs
git commit -m "feat: add infer/ module with InferenceEngine and PartialHir"
```

---

## Task 2: Create `lower/` — move the lowering core

Move `resolve/context.rs`, `resolve/scope.rs`, `resolve/error.rs`, `resolve/intrinsics.rs`, and all of `resolve/lower/`, `resolve/traits/`, `resolve/crates/` into a top-level `lower/` module.

**Files:**
- Create: `lib/src/lower/mod.rs`
- Create: `lib/src/lower/context.rs`  (was `resolve/context.rs`)
- Create: `lib/src/lower/scope.rs`    (was `resolve/scope.rs`)
- Create: `lib/src/lower/error.rs`    (was `resolve/error.rs`)
- Create: `lib/src/lower/intrinsics.rs` (was `resolve/intrinsics.rs`)
- Create: `lib/src/lower/bodies.rs`, `control_flow.rs`, `expression.rs`, `function.rs`, `paths.rs`, `program.rs`, `statement.rs`, `types.rs`  (were `resolve/lower/*`)
- Create: `lib/src/lower/traits/mod.rs`, `conformance.rs`, `defaults.rs` (were `resolve/traits/*`)
- Create: `lib/src/lower/crates/mod.rs`, `registration.rs`, `bodies.rs` (were `resolve/crates/*`)

**Step 1: Copy error.rs**

Copy `resolve/error.rs` → `lower/error.rs`. No content changes needed (only imports `crate::diagnostic::SpannedError` and `crate::lexer::Span`, both unchanged).

**Step 2: Copy scope.rs**

Copy `resolve/scope.rs` → `lower/scope.rs`. No content changes.

**Step 3: Copy intrinsics.rs**

Copy `resolve/intrinsics.rs` → `lower/intrinsics.rs`. The file imports `crate::types::Type` — no path changes needed.

**Step 4: Copy and update context.rs → lower/mod.rs**

The `Lowerer` struct definition moves to `lib/src/lower/mod.rs` (it's the entry point for this module). Copy `resolve/context.rs` → `lower/mod.rs`.

Update the import path for the inference engine:
```rust
// Old:
use super::inference::InferenceEngine;
use super::scope::Scope;
// New (within the same lower/ module):
use crate::infer::InferenceEngine;
use crate::lower::scope::Scope;
```

Also update `collect_exports`, `path_names`, `seg_name` — these helper functions can stay in `lower/mod.rs` since they're used by `lower/program.rs`.

Add the sub-module declarations:
```rust
pub(crate) mod context;  // if split out, otherwise inline
pub(crate) mod scope;
pub(crate) mod error;
pub(crate) mod intrinsics;
pub(crate) mod bodies;
pub(crate) mod control_flow;
pub(crate) mod expression;
pub(crate) mod function;
pub(crate) mod paths;
pub(crate) mod program;
pub(crate) mod statement;
pub(crate) mod types;
pub(crate) mod traits;
pub(crate) mod crates;

pub use error::{CompileError, ResolveError};
pub use program::lower;
```

**Step 5: Add `Lowerer::from_declarations` constructor**

Add a constructor to `Lowerer` that takes `Declarations` (defined in Task 3). For now, add a placeholder that will be completed once `collect/` exists. Or, temporarily keep `Lowerer::new()` and `Lowerer::with_options()` and add `from_declarations` once `Declarations` is defined.

**Step 6: Copy all resolve/lower/* files**

Copy each file and update the one import that changes in every file:
```rust
// Old (in all lower/* files):
use super::super::context::Lowerer;
// New:
use crate::lower::Lowerer;
```

Also update references to `InferenceEngine`:
```rust
// Old:
use super::super::inference::InferenceEngine;
// New:
use crate::infer::InferenceEngine;
```

**Step 7: Copy resolve/traits/* and resolve/crates/*

Same pattern: copy files, update import paths:
```rust
// Old:
use super::super::context::Lowerer;
// New:
use crate::lower::Lowerer;
```

**Step 8: Update `lower/program.rs` to produce `PartialHir`**

The key change: `lower_program_with_crates` currently returns `Result<HirProgram, Vec<CompileError>>`. Change it to return `Result<PartialHir, Vec<CompileError>>`.

The final section of `lower_program_with_crates` currently:
```rust
// Generalize functions
self.generalize_all_functions();
// Finalize all types
self.finalize_types();
// Resolve import aliases
self.resolve_import_aliases();
Ok(HirProgram { ... })
```

Replace with:
```rust
// Don't finalize/generalize here — that's the infer stage's job.
// Just return PartialHir with all the state infer::finalize needs.
Ok(crate::infer::PartialHir {
    functions: self.functions,
    structs: self.structs,
    enums: self.enums,
    traits: self.traits,
    impls: self.impls,
    externs: self.externs,
    engine: self.engine,
    function_type_vars: self.function_type_vars,
    import_aliases: self.import_aliases,
})
```

Remove the calls to `self.generalize_all_functions()`, `self.finalize_types()`, `self.resolve_import_aliases()` from `lower_program_with_crates`.

**Step 9: Define the public `lower()` entry point**

In `lower/program.rs`:
```rust
/// Lower a program with crates, producing PartialHir for the infer stage.
pub fn lower(
    program: &ast::Program,
    decls: crate::collect::Declarations,
    crate_ctx: &CrateContext,
) -> Result<crate::infer::PartialHir, Vec<CompileError>> {
    let lowerer = Lowerer::from_declarations(decls);
    lowerer.lower_program_with_crates(program, Some(crate_ctx))
}
```

Note: `Lowerer::from_declarations` is implemented in Task 3 after `Declarations` is defined.

**Step 10: Register lower in lib.rs**
```rust
pub mod lower;
```

**Step 11: Build (expect errors about missing collect::Declarations — that's OK)**
```bash
cargo build -p rock-lib 2>&1 | grep "^error" | head -20
```
The only errors should be about `collect::Declarations` not existing yet and the missing `from_declarations` constructor. No logic errors.

**Step 12: Commit**
```bash
git add lib/src/lower/
git add lib/src/lib.rs
git commit -m "feat: add lower/ module (AST->HIR lowering, produces PartialHir)"
```

---

## Task 3: Create `collect/` — first-pass declaration gathering

The `collect` stage gathers all top-level declarations without lowering bodies. It produces `Declarations` which `lower` uses as its starting state.

**Files:**
- Create: `lib/src/collect/mod.rs`
- Create: `lib/src/collect/declarations.rs` (was `resolve/collect/declarations.rs`)
- Create: `lib/src/collect/traits.rs`       (was `resolve/collect/traits.rs`)
- Create: `lib/src/collect/types.rs`        (was `resolve/collect/types.rs`)

**Step 1: Define `Declarations` in `collect/mod.rs`**

```rust
//! First-pass declaration collection.
//!
//! Gathers all top-level declarations (structs, enums, trait defs, function
//! signatures) without lowering function bodies. Produces `Declarations`
//! which is consumed by the `lower` stage.

mod declarations;
mod traits;
mod types;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::ast;
use crate::crate_system::CrateContext;
use crate::hir::{HirStruct, HirEnum, HirTrait, HirImpl, HirExtern, HirFunction, HirFunctionSig};
use crate::infer::InferenceEngine;
use crate::lower::error::CompileError;

/// All top-level declarations gathered during the first pass.
/// Consumed by `lower::lower()`.
pub struct Declarations {
    pub structs: BTreeMap<String, HirStruct>,
    pub enums: BTreeMap<String, HirEnum>,
    pub traits: BTreeMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    /// Functions with headers but empty bodies (bodies filled in by lower)
    pub functions: BTreeMap<String, HirFunction>,
    pub function_sigs: BTreeMap<String, HirFunctionSig>,
    pub methods: BTreeMap<(String, String), HirFunction>,
    pub infix_precedence: BTreeMap<String, u8>,
    pub import_aliases: BTreeMap<String, String>,
    /// Modules that need body lowering: (module_name, file_path)
    pub loaded_module_paths: Vec<(String, PathBuf)>,
    /// The inference engine — carries type var counter so lower can continue
    /// creating fresh type vars without ID conflicts.
    pub engine: InferenceEngine,
    /// Whether to inject stdlib prelude
    pub inject_prelude: bool,
}

/// Run the first-pass declaration collection.
pub fn collect(
    program: &ast::Program,
    crate_ctx: &CrateContext,
    inject_prelude: bool,
) -> Result<Declarations, Vec<CompileError>> {
    let mut collector = Collector::new(inject_prelude);
    collector.run(program, crate_ctx)?;
    Ok(collector.into_declarations())
}
```

**Step 2: Define `Collector` struct**

The `Collector` is essentially a slimmed-down `Lowerer` that only does the collect pass. Add to `collect/mod.rs`:

```rust
/// Internal context for the collection pass.
struct Collector {
    // Shared with Lowerer: only the fields needed for declaration collection
    engine: InferenceEngine,
    scope: crate::lower::scope::Scope,
    structs: BTreeMap<String, HirStruct>,
    enums: BTreeMap<String, HirEnum>,
    traits: BTreeMap<String, HirTrait>,
    impls: Vec<HirImpl>,
    externs: Vec<HirExtern>,
    functions: BTreeMap<String, HirFunction>,
    function_sigs: BTreeMap<String, HirFunctionSig>,
    methods: BTreeMap<(String, String), HirFunction>,
    errors: Vec<CompileError>,
    file_path: std::path::PathBuf,
    current_span: Option<crate::lexer::Span>,
    inject_prelude: bool,
    current_module_path: std::path::PathBuf,
    loaded_modules: std::collections::BTreeSet<std::path::PathBuf>,
    loaded_module_paths: Vec<(String, std::path::PathBuf)>,
    import_aliases: BTreeMap<String, String>,
    current_trait: Option<String>,
    infix_precedence: BTreeMap<String, u8>,
}
```

**Step 3: Implement `Collector::run` and `Collector::into_declarations`**

```rust
impl Collector {
    fn new(inject_prelude: bool) -> Self { /* same as Lowerer::with_options */ }

    fn run(&mut self, program: &ast::Program, crate_ctx: &CrateContext) -> Result<(), Vec<CompileError>> {
        if let Some(ref fp) = program.module.filepath {
            self.file_path = fp.clone();
            self.current_module_path = fp.clone();
        }

        // Register crate declarations (no bodies)
        self.register_crate_declarations(crate_ctx);
        if self.inject_prelude {
            self.inject_stdlib_prelude_decls(crate_ctx);
        }

        // Collect declarations from user program
        self.collect_declarations(&program.module);

        if !self.errors.is_empty() {
            return Err(std::mem::take(&mut self.errors));
        }
        Ok(())
    }

    fn into_declarations(self) -> Declarations {
        Declarations {
            structs: self.structs,
            enums: self.enums,
            traits: self.traits,
            impls: self.impls,
            externs: self.externs,
            functions: self.functions,
            function_sigs: self.function_sigs,
            methods: self.methods,
            infix_precedence: self.infix_precedence,
            import_aliases: self.import_aliases,
            loaded_module_paths: self.loaded_module_paths,
            engine: self.engine,
            inject_prelude: self.inject_prelude,
        }
    }
}
```

**Step 4: Port collect submodules**

Copy `resolve/collect/declarations.rs` → `collect/declarations.rs`, `resolve/collect/traits.rs` → `collect/traits.rs`, `resolve/collect/types.rs` → `collect/types.rs`.

Update imports from:
```rust
use super::super::context::Lowerer;
```
to:
```rust
use super::Collector;
```

Change all `impl Lowerer { ... }` → `impl Collector { ... }`. The method bodies are unchanged.

Note: These methods reference `self.engine` (for fresh type vars), `self.scope`, etc. — all present in `Collector`.

**Step 5: Implement `Lowerer::from_declarations`**

Now that `Declarations` exists, add to `lib/src/lower/mod.rs`:

```rust
impl Lowerer {
    pub fn from_declarations(decls: crate::collect::Declarations) -> Self {
        // Rebuild scope from declarations
        let mut scope = Scope::new();
        use crate::types::Type;

        // Populate scope from collected functions
        for (name, func) in &decls.functions {
            let param_types: Vec<Type> = func.params.iter().map(|p| p.ty.clone()).collect();
            let func_type = Type::Function(param_types, Box::new(func.ret_type.clone()));
            scope.define(name.clone(), func_type, false);
        }
        // Populate scope from structs (constructor functions)
        for (name, _) in &decls.structs {
            scope.define(name.clone(), Type::Struct(name.clone(), vec![]), false);
        }
        // Populate scope from enums
        for (name, _) in &decls.enums {
            scope.define(name.clone(), Type::Enum(name.clone(), vec![]), false);
        }

        Self {
            engine: decls.engine,
            scope,
            structs: decls.structs,
            enums: decls.enums,
            traits: decls.traits,
            impls: decls.impls,
            externs: decls.externs,
            functions: decls.functions,
            function_sigs: decls.function_sigs,
            methods: decls.methods,
            errors: Vec::new(),
            file_path: std::path::PathBuf::new(),
            current_span: None,
            function_type_vars: BTreeMap::new(),
            current_function: None,
            inject_prelude: decls.inject_prelude,
            current_module_path: std::path::PathBuf::new(),
            loaded_modules: BTreeSet::new(),
            loaded_module_paths: decls.loaded_module_paths,
            import_aliases: decls.import_aliases,
            current_trait: None,
            infix_precedence: decls.infix_precedence,
        }
    }
}
```

**Step 6: Register collect in lib.rs**
```rust
pub mod collect;
```

**Step 7: Build**
```bash
cargo build -p rock-lib 2>&1 | grep "^error" | head -30
```
Fix any errors — most will be import path issues.

**Step 8: Commit**
```bash
git add lib/src/collect/
git add lib/src/lower/
git add lib/src/lib.rs
git commit -m "feat: add collect/ module and Declarations, wire Lowerer::from_declarations"
```

---

## Task 4: Wire up the new pipeline in lib.rs

Switch `lib.rs` from calling `resolve::lower_with_crates_and_options` to the new three-stage pipeline.

**Files:**
- Modify: `lib/src/lib.rs`

**Step 1: Update Phase 3 in `lib/src/lib.rs`**

Replace:
```rust
// Phase 3: Lower AST to HIR (name resolution + type inference)
let hir = match resolve::lower_with_crates_and_options(&ast, &crate_ctx, !config.no_prelude) {
    Ok(hir) => hir,
    Err(errors) => {
        let diagnostics = Diagnostics::from(errors);
        return Err(diagnostics);
    }
};
```

With:
```rust
// Phase 3a: Collect top-level declarations
let decls = match collect::collect(&ast, &crate_ctx, !config.no_prelude) {
    Ok(d) => d,
    Err(errors) => {
        return Err(Diagnostics::from(errors));
    }
};

// Phase 3b: Lower AST → HIR with type inference
let partial_hir = match lower::lower(&ast, decls, &crate_ctx) {
    Ok(h) => h,
    Err(errors) => {
        return Err(Diagnostics::from(errors));
    }
};

// Phase 3c: Finalize types (solve type variables, generalize)
let hir = match infer::finalize(partial_hir) {
    Ok(h) => h,
    Err(errors) => {
        return Err(Diagnostics::from(errors));
    }
};
```

**Step 2: Remove the `pub mod resolve;` declaration**

Comment it out for now (keep it until Task 5 verifies tests pass):
```rust
// pub mod resolve;  // to be deleted after pipeline migration
```

**Step 3: Build**
```bash
cargo build -p rock-lib 2>&1 | grep "^error"
```
Expected: no errors. Fix any that appear.

**Step 4: Run all tests**
```bash
cargo test -p rock-lib 2>&1 | tail -30
```
Expected: all tests pass. If any fail, debug before continuing.

**Step 5: Commit**
```bash
git add lib/src/lib.rs
git commit -m "feat: wire new collect/lower/infer pipeline in lib.rs"
```

---

## Task 5: Delete `resolve/` and clean up

**Files:**
- Delete: `lib/src/resolve/` (entire directory)
- Modify: `lib/src/lib.rs` (remove commented resolve mod)

**Step 1: Run tests one more time to confirm baseline**
```bash
cargo test -p rock-lib 2>&1 | tail -10
```

**Step 2: Delete resolve/**
```bash
rm -rf lib/src/resolve
```

**Step 3: Remove commented-out mod declaration from lib.rs**

Delete the line:
```rust
// pub mod resolve;  // to be deleted after pipeline migration
```

**Step 4: Build**
```bash
cargo build -p rock-lib 2>&1 | grep "^error"
```
Expected: no errors.

**Step 5: Run all tests**
```bash
cargo test -p rock-lib 2>&1 | tail -20
```
Expected: all tests pass, same count as before.

**Step 6: Final commit**
```bash
git add -A
git commit -m "refactor: delete resolve/ — pipeline now collect/lower/infer"
```

---

## Task 6: Final cleanup pass

Fix dead code warnings in `infer/type_vars.rs` and any remaining warnings from the migration.

**Step 1: Find remaining warnings**
```bash
cargo build -p rock-lib 2>&1 | grep "^warning" | grep -v "generated"
```

**Step 2: Remove dead methods**

In `infer/type_vars.rs`, remove any methods flagged as `dead_code` that are genuinely unused. Cross-check: if a method is `pub(crate)` but never called, delete it. If it might be useful, add `#[allow(dead_code)]` with a comment explaining why.

**Step 3: Verify tests still pass**
```bash
cargo test -p rock-lib 2>&1 | tail -10
```

**Step 4: Final commit**
```bash
git add -A
git commit -m "chore: remove dead code after pipeline separation"
```

---

## Verification Checklist

After all tasks complete:

- [ ] `cargo build -p rock-lib` compiles with zero errors
- [ ] `cargo build -p rock-lib` has no unused-import warnings in new modules
- [ ] `cargo test -p rock-lib` — same test count passes as before refactor
- [ ] `lib/src/resolve/` does not exist
- [ ] `lib/src/infer/`, `lib/src/lower/`, `lib/src/collect/` exist at top level
- [ ] `lib/src/lib.rs` Phase 3 shows the three-stage pipeline explicitly
- [ ] `lib/src/lower/traits/` and `lib/src/lower/crates/` exist (internal to lower)
- [ ] `cargo run -p rockc -- --entry-file examples/hello.rk` produces correct output
