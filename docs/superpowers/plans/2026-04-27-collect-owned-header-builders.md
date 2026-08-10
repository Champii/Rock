# Collect-Owned Header Builders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `collect` own local header traversal and local header construction without changing dependency bootstrap behavior or the `Declarations` contract.

**Architecture:** Add a collect-owned `headers` module that builds local HIR headers using the existing `Lowerer` semantic state only for parse-type lowering and related bookkeeping. Then switch `LocalCollector` to walk root, inline, and source-backed local modules itself while preserving existing import, export, loader, and declaration-map behavior.

**Tech Stack:** Rust 2021, `rock-lib`, collect/lower modules, existing `Hir*` header types, `cargo test -p rock-lib`.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-04-27-collect-owned-header-builders-design.md`.

This plan includes:

- a new collect-owned header builder module
- local root and inline traversal in `LocalCollector`
- local source-backed `mod foo;` traversal in `LocalCollector`
- preservation of current declaration outputs and lower bootstrap state

This plan does not include:

- dependency crate collection migration
- resolver work
- changes to `Lowerer::from_declarations`
- changes to parse-type lowering ownership

## File Structure

- Create: `lib/src/collect/headers.rs`
  - Owns collect-owned header builder helpers for structs, enums, functions, traits, impls, and externs.
- Modify: `lib/src/collect/mod.rs`
  - Declares the new module.
  - Adds focused regression coverage for the new local collector ownership boundary.
- Modify: `lib/src/collect/collector.rs`
  - Replaces local delegation to `Lowerer::collect_declarations` with collect-owned traversal.
  - Handles local inline qualification and local source-backed module traversal.
- Keep unchanged for this step: `lib/src/lower/collect/declarations.rs`
  - Remains for the legacy lower-owned paths until a later cleanup step.

## Task 1: Add Failing Tests For Collect-Owned Builders

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Create: `lib/src/collect/headers.rs`

- [ ] **Step 1: Add the new internal module declaration**

In `lib/src/collect/mod.rs`, add this line after `mod collector;`:

```rust
mod headers;
```

- [ ] **Step 2: Add a failing header-builder test module**

Create `lib/src/collect/headers.rs` with a test module that defines local helpers for:

- `ident(name: &str) -> crate::ast::Ident`
- `type_inner(name: &str) -> crate::ast::ParseTypeInner`
- `named_type(name: &str) -> crate::ast::ParseType`
- `ident_pattern(name: &str) -> crate::ast::Pattern`

and add this failing test skeleton:

```rust
    #[test]
    fn build_struct_lowers_field_types() {
        todo!("write the first failing header-builder test")
    }
```

- [ ] **Step 3: Run the focused test and confirm it fails**

Run: `cargo test -p rock-lib --lib collect::headers::tests::build_struct_lowers_field_types -- --exact --nocapture`

Expected: FAIL because the new header builder test still contains `todo!()`.

If Cargo reports stale or incremental linker artifacts, run `cargo clean -p rock-lib` and retry the same command before reporting concern.

- [ ] **Step 4: Replace the placeholder test with a real failing test and add the production skeleton**

Replace the placeholder with a real assertion-based test using `crate::ast::StructDeclField`, and add this production skeleton above the test module:

```rust
use crate::ast;
use crate::hir::{HirEnum, HirExtern, HirFunction, HirFunctionSig, HirImpl, HirStruct, HirTrait};
use crate::lower::Lowerer;

pub(crate) fn build_struct(_lowerer: &mut Lowerer, _decl: &ast::StructDecl) -> HirStruct {
    todo!()
}

pub(crate) fn build_enum(_lowerer: &mut Lowerer, _decl: &ast::EnumDecl) -> HirEnum {
    todo!()
}

pub(crate) fn build_trait(_lowerer: &mut Lowerer, _decl: &ast::TraitDecl) -> HirTrait {
    todo!()
}

pub(crate) fn build_function_sig(
    _lowerer: &mut Lowerer,
    _sig: &ast::FunctionSig,
) -> HirFunctionSig {
    todo!()
}

pub(crate) fn build_function_header(
    _lowerer: &mut Lowerer,
    _decl: &ast::FunctionDecl,
) -> HirFunction {
    todo!()
}

pub(crate) fn build_function_header_with_sig(
    _lowerer: &mut Lowerer,
    _decl: &ast::FunctionDecl,
    _sig: &HirFunctionSig,
) -> HirFunction {
    todo!()
}

pub(crate) fn build_extern(
    _lowerer: &mut Lowerer,
    _sig: &ast::FunctionSig,
) -> HirExtern {
    todo!()
}

pub(crate) fn build_impl(_lowerer: &mut Lowerer, _decl: &ast::Impl) -> HirImpl {
    todo!()
}
```

## Task 2: Implement Collect-Owned Header Builders

**Files:**
- Modify: `lib/src/collect/headers.rs`

- [ ] **Step 1: Add failing unit tests for struct and function headers**

Add tests that verify:

- `build_struct` lowers `StructDeclField.ty` via `lower_parse_type`
- a standalone `FunctionSig` followed by `FunctionDecl` reuses signature types and removes the stored signature entry
- a trait header preserves method/signature names

Use explicit AST constructors already defined in `lib/src/ast/tree.rs`, including:

- `StructDeclField`
- `Pattern { binding, kind: PatternKind::Ident(...) }`
- `HashMap<Ident, FunctionDecl>` and `HashMap<Ident, FunctionSig>` for trait methods/signatures

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test -p rock-lib --lib collect::headers::tests -- --nocapture`

Expected: FAIL because the builder functions still contain `todo!()`.

- [ ] **Step 3: Implement the builders by moving local header logic into `collect/headers.rs`**

Write collect-owned helper functions that mirror the existing lower-owned behavior using `Lowerer` only for shared state. The module should own the logic that currently lives in:

- `lib/src/lower/collect/types.rs`
- `lib/src/lower/function.rs`
- `lib/src/lower/collect/traits.rs`

Do not call these lower-owned local builder methods from the new module:

- `collect_struct_new`
- `collect_enum_new`
- `build_hir_trait`
- `collect_function_sig`
- `collect_function_signature_only`
- `collect_extern`
- `collect_impl`

Preserve current bookkeeping side effects on:

- `scope`
- `function_sigs`
- `function_type_vars`
- `methods`
- `impls`
- `externs`

- [ ] **Step 4: Run the focused tests and confirm they pass**

Run: `cargo test -p rock-lib --lib collect::headers::tests -- --nocapture`

Expected: PASS for the new header tests.

## Task 3: Switch LocalCollector To Collect-Owned Traversal

**Files:**
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add a failing collector regression for local source-backed collection details**

Add a regression to `lib/src/collect/mod.rs` that proves a local source-backed module still preserves:

- qualified function headers such as `test::io::helper`
- root-level import aliases that target that qualified header

Use a temporary `io.rk` file and a root `TopLevel::Mod(ident("io"), false)` declaration.

- [ ] **Step 2: Run the focused regression and capture the baseline**

Run: `cargo test -p rock-lib --lib collect::tests::collect_source_backed_local_module_preserves_exports_and_import_aliases -- --exact --nocapture`

Expected: PASS before the traversal switch.

- [ ] **Step 3: Replace `LocalCollector` traversal with collect-owned logic**

Update `lib/src/collect/collector.rs` so `collect_local_declarations` no longer calls `self.lowerer.collect_declarations(module)`. Add helpers with this shape:

```rust
impl LocalCollector {
    pub(crate) fn collect_local_declarations(&mut self, module: &ast::Module) {
        self.collect_module(module);
    }

    fn collect_module(&mut self, module: &ast::Module) {
        // root and inline flat traversal
    }

    fn collect_inline_qualified(&mut self, module: &ast::Module, prefix: &str) {
        // mirror current local inline qualification behavior
    }

    fn collect_source_module(&mut self, ident: &ast::Ident) {
        // load file, update loaded_module_paths, recurse with export-aware qualification
    }

    fn collect_export_qualified(
        &mut self,
        module: &ast::Module,
        prefix: &str,
        exports: &std::collections::HashMap<String, Option<String>>,
    ) {
        // mirror current local source-backed qualified behavior
    }
}
```

Use the new `collect::headers` helpers for all local header construction in these traversal methods.

Keep these shared lowerer helpers in place for this step:

- `lower_parse_type`
- `load_module`
- `current_module_prefix`
- `handle_import`
- `handle_glob_import`
- `expand_glob_exports`
- `register_export_aliases`

- [ ] **Step 4: Run focused collector tests and confirm they pass**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: PASS for the existing collector tests plus the new source-backed regression.

## Task 4: Full Verification And Cleanup

**Files:**
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Run formatting**

Run: `cargo fmt --all`

Expected: success with no errors.

- [ ] **Step 2: Run the narrow verification commands**

Run: `cargo test -p rock-lib --lib collect::headers::tests -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Run the full package test suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

If Cargo reports stale or incremental linker artifacts, run `cargo clean -p rock-lib` and retry the same command before reporting concern.

- [ ] **Step 4: Review the final diff for scope discipline**

Check that the diff is limited to:

```text
docs/superpowers/specs/2026-04-27-collect-owned-header-builders-design.md
docs/superpowers/plans/2026-04-27-collect-owned-header-builders.md
lib/src/collect/mod.rs
lib/src/collect/collector.rs
lib/src/collect/headers.rs
```

and any unavoidable test-only touch-ups driven by compilation errors.
