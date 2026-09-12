# Pipeline Separation Design

**Date:** 2026-02-19
**Branch:** split_resolve_working
**Status:** Approved

## Problem

The `resolve/` module is a misnomer — it performs name resolution, type inference, HIR lowering, trait conformance checking, type finalization, and generalization all in one monolithic `Lowerer` god struct (~5500 lines before the recent split). The submodule split improved file organization but the concerns remain tangled.

Additionally, several warnings exist: unused `pub use *` re-exports in submodule `mod.rs` files (because everything is `impl Lowerer` blocks), unused imports, dead code in `type_vars.rs`, and a leftover `temp/` directory not compiled into the build.

## Goal

A clear three-stage pipeline at `lib/src/` top level, with each stage having a defined input/output contract. `lib.rs` reads as an explicit pipeline. The `resolve/` folder is eliminated entirely.

## Pipeline

```
ast::Program
    ↓
[collect]  →  Declarations
    ↓
[lower]    →  PartialHir
    ↓
[infer]    →  HirProgram
```

`lib.rs` Phase 3:

```rust
let decls       = collect::collect(&ast, &crate_ctx, !config.no_prelude)?;
let partial_hir = lower::lower(&ast, decls, &crate_ctx)?;
let hir         = infer::finalize(partial_hir)?;
```

## Stage Contracts

### Stage 1: `collect`

**Input:** `&ast::Program`, `&CrateContext`, `inject_prelude: bool`
**Output:** `Result<Declarations, Vec<CompileError>>`

```rust
pub struct Declarations {
    pub structs: BTreeMap<String, HirStruct>,
    pub enums: BTreeMap<String, HirEnum>,
    pub traits: BTreeMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    pub function_sigs: BTreeMap<String, HirFunctionSig>,
    pub infix_precedence: BTreeMap<String, u8>,
    pub import_aliases: BTreeMap<String, String>,
}
```

Pure data — no runtime state (no `Scope`), no type inference. Gathers all top-level declarations: struct/enum/trait definitions, function signatures without bodies, impl headers, extern declarations. Handles crate registration and prelude injection at the declaration level.

### Stage 2: `lower`

**Input:** `&ast::Program`, `Declarations`, `&CrateContext`
**Output:** `Result<PartialHir, Vec<CompileError>>`

```rust
pub struct PartialHir {
    pub functions: BTreeMap<String, HirFunction>,  // bodies with type vars
    pub structs: BTreeMap<String, HirStruct>,
    pub enums: BTreeMap<String, HirEnum>,
    pub traits: BTreeMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    pub engine: InferenceEngine,                   // constraint solution state
}
```

`Lowerer::new(decls)` rebuilds the lexical scope from `Declarations` internally. Lowers all function bodies AST→HIR. Trait conformance checking and default method injection stay internal to this stage (too interleaved with body lowering to separate cleanly). Carries the `InferenceEngine` (with all solved unification constraints) forward to the next stage.

### Stage 3: `infer`

**Input:** `PartialHir`
**Output:** `Result<HirProgram, Vec<CompileError>>`

Applies `InferenceEngine` substitutions to all type variables in the HIR. Generalizes unconstrained type variables to generic parameters (`generalize_all_functions`). Resolves import aliases. Strips `engine` and returns the final fully-typed `HirProgram`.

## New `lib/src/` Layout

```
lib/src/
├── ast/
├── borrow_check/
├── codegen/
├── collect/                 ← NEW (was resolve/collect/)
│   ├── mod.rs              (Collector struct + pub collect() fn)
│   ├── declarations.rs
│   ├── traits.rs
│   └── types.rs
├── crate_system/
├── hir/
├── infer/                   ← NEW (was resolve/inference/ + resolve/types/)
│   ├── mod.rs              (pub finalize() fn)
│   ├── engine.rs           (InferenceEngine)
│   ├── finalize.rs
│   ├── generalize.rs
│   ├── type_vars.rs
│   └── helpers.rs
├── lower/                   ← NEW (was resolve/lower/ + resolve/context.rs + ...)
│   ├── mod.rs              (Lowerer struct + pub lower() fn)
│   ├── scope.rs            (Scope — internal to lower)
│   ├── error.rs
│   ├── intrinsics.rs
│   ├── bodies.rs
│   ├── control_flow.rs
│   ├── expression.rs
│   ├── function.rs
│   ├── paths.rs
│   ├── program.rs
│   ├── statement.rs
│   ├── types.rs
│   ├── traits/             (internal — conformance + defaults)
│   │   ├── mod.rs
│   │   ├── conformance.rs
│   │   └── defaults.rs
│   └── crates/             (internal — registration + body loading)
│       ├── mod.rs
│       ├── registration.rs
│       └── bodies.rs
├── macro_expansion/
├── mono/
├── new_parser/
├── types/
└── lib.rs
```

`resolve/` is **eliminated**. `resolve/temp/` is **deleted** (was never compiled).

## File Migration Map

| Old | New | Notes |
|-----|-----|-------|
| `resolve/context.rs` | `lower/mod.rs` | Lowerer struct, `new(decls)` constructor |
| `resolve/scope.rs` | `lower/scope.rs` | Internal to lower |
| `resolve/error.rs` | `lower/error.rs` | Move |
| `resolve/intrinsics.rs` | `lower/intrinsics.rs` | Move |
| `resolve/collect/*` | `collect/` | Collector struct, produces Declarations |
| `resolve/lower/*` | `lower/` | Flat move |
| `resolve/traits/*` | `lower/traits/` | Internal to lower phase |
| `resolve/crates/*` | `lower/crates/` | Internal to lower phase |
| `resolve/inference/engine.rs` | `infer/engine.rs` | Top-level |
| `resolve/types/finalize.rs` | `infer/finalize.rs` | Move |
| `resolve/types/generalize.rs` | `infer/generalize.rs` | Move |
| `resolve/types/type_vars.rs` | `infer/type_vars.rs` | Move, clean dead code |
| `resolve/types/helpers.rs` | `infer/helpers.rs` | Move |
| `resolve/temp/` | **deleted** | Never compiled |

## Warnings to Fix

- Remove `pub use declarations::*`, `pub use traits::*`, `pub use types::*` from `collect/mod.rs` (nothing to re-export from impl blocks)
- Same for `crates/mod.rs` and `traits/mod.rs`
- Remove unused `use crate::types::Type` in `context.rs`
- Remove unused `use crate::hir::*` in `lower/bodies.rs`
- Clean dead code methods in `type_vars.rs`
- Fix unreachable pattern in `resolve/mod.rs`
