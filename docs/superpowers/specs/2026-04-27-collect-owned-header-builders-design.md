# Collect-Owned Header Builders Design

**Date:** 2026-04-27
**Status:** Auto-approved for implementation
**Scope:** Make `collect` own local header traversal and header construction for root, inline, and source-backed local modules while keeping parse-type lowering, module loading, and dependency bootstrap behavior on `Lowerer` for now.

## Purpose

The independent local collection step moved pass ownership into `collect::collect`, but `LocalCollector` still delegates its real work to `Lowerer::collect_declarations`. That leaves the main local declaration walk and all local header builders in `lower/`, which is still the wrong ownership boundary for the audit's `Real Collection And Name Resolution` track.

The next narrow step is to keep the existing bootstrap `Lowerer` state and existing declaration outputs, but replace the collector's remaining reuse of lower-owned local traversal and header-building helpers with collect-owned logic.

## Goals

- Make `LocalCollector` own the local declaration walk instead of calling `Lowerer::collect_declarations`.
- Introduce a collect-owned header builder module for local declaration headers.
- Preserve current `Declarations` outputs and current `Lowerer::from_declarations` behavior.
- Preserve local inline-module behavior and local source-backed `mod foo;` behavior.
- Keep dependency crate registration and stdlib prelude bootstrap unchanged.

## Non-Goals

- Do not move dependency crate collection out of `Lowerer` yet.
- Do not add a resolver or canonical path tables yet.
- Do not change `Declarations` shape, `Lowerer::from_declarations`, or later body lowering contracts.
- Do not rewrite parse-type lowering yet.
- Do not change module IO, crate artifact indexing, or prelude policy.

## Existing Seam

Today `collect::collect` bootstraps a `Lowerer`, then hands that lowerer into `LocalCollector`. `LocalCollector::collect_local_declarations` immediately calls `self.lowerer.collect_declarations(module)`, which means:

- local traversal still lives in `lib/src/lower/collect/declarations.rs`
- local inline qualified aliases still come from `collect_declarations_qualified`
- local source-backed module handling still routes through `handle_mod_decl` into `collect_crate_declarations_qualified`
- header builders for structs, enums, traits, impls, externs, and functions still live under `lib/src/lower/**`

That is the remaining ownership violation from the last step.

## Architecture

### New Collect-Owned Header Module

Add `lib/src/collect/headers.rs`.

This module owns local header construction for the collect phase. It should provide collect-owned helpers that build the same HIR header shapes currently expected by lowering:

- `HirStruct`
- `HirEnum`
- `HirTrait`
- `HirImpl`
- `HirFunction`
- `HirFunctionSig`
- `HirExtern`

These helpers may still take `&mut Lowerer` because the current semantic state they need already lives there:

- `InferenceEngine` for fresh type variables
- known structs/enums/traits for parse-type lowering
- `function_type_vars`, `methods`, and trait-context bookkeeping
- scope registration side effects already expected by later lowering

That is acceptable in this step because ownership of the traversal and header construction moves to `collect`, while parse-type lowering remains temporarily shared.

### Temporary Shared Lowerer Surface

This step keeps the following responsibilities on `Lowerer`:

- `lower_parse_type` and related type-lowering context
- module loading and cache reuse (`load_module`, `module_file_cache`, `loaded_modules`)
- import/glob-import registration (`handle_import`, `handle_glob_import`)
- export expansion and export-alias registration (`expand_glob_exports`, `register_export_aliases`)
- dependency crate registration and stdlib prelude injection

This keeps the extraction narrow and avoids mixing it with the later crate-interface split.

### LocalCollector Traversal Modes

`LocalCollector` should own three local traversal modes.

#### Root And Inline Unqualified Traversal

This replaces `Lowerer::collect_declarations` for the root module tree.

It should preserve current behavior:

- root declarations are inserted in the existing flat local maps
- inline modules recurse into the same flat local maps
- imports, glob imports, infix precedence, standalone signatures, and impls still update the same state as today

#### Inline Qualified Alias Traversal

This replaces the local use of `Lowerer::collect_declarations_qualified` for inline modules.

It must preserve the current transitional behavior exactly:

- inline structs, enums, and functions still gain qualified aliases such as `math::Vector`
- the flat local entries gathered by unqualified traversal still remain
- the traversal stays limited to the items the current local qualified pass already handles

This step should not opportunistically broaden inline qualified behavior beyond the current contract.

#### Source-Backed Local Module Traversal

This replaces the local use of `handle_mod_decl` plus `collect_crate_declarations_qualified`.

It should still:

- load the local module with the current loader and cache behavior
- append the same `(qualified_module_name, file_path)` entry to `loaded_module_paths`
- collect qualified declarations under prefixes such as `test::io::Writer`
- preserve export-aware recursion into nested submodules
- preserve import/glob-import handling and export alias registration inside source-backed modules

The important ownership shift is that `LocalCollector` performs the traversal and calls collect-owned header builders, even if it still uses lower-owned loader and import/export helpers.

## Data Flow

After this step, `collect::collect` should behave like this:

1. Bootstrap a `Lowerer` for dependency crate registration and prelude state.
2. Register dependency crates and optional stdlib prelude exactly as today.
3. Hand that bootstrap state into `LocalCollector`.
4. Let `LocalCollector` walk local root, inline, and source-backed local modules using collect-owned traversal and collect-owned header builders.
5. Finish into the existing `LocalCollection` and assemble `Declarations` exactly as today.
6. Build the `ItemIndex` from the root AST plus source-module cache exactly as today.

The output shape remains stable; only phase ownership changes.

## Boundary Rules

- `LocalCollector::collect_local_declarations` must no longer call `Lowerer::collect_declarations`.
- The local collector path must no longer rely on lower-owned local header builder methods such as `collect_struct_new`, `collect_enum_new`, `build_hir_trait`, `collect_function_sig`, `collect_function_signature_only`, `collect_extern`, or `collect_impl`.
- Reusing `Lowerer::lower_parse_type` is allowed in this step.
- Reusing lower-owned loader/import/export helpers is allowed in this step.
- Preserve current quirks where they are part of the existing observable behavior.

## Testing

Add focused coverage in two layers.

### Header Builder Unit Coverage

Add collect-owned unit tests that exercise the new header builders directly for at least:

- struct field type lowering
- function signature/header collection with stored standalone signatures
- trait method/signature header building

These tests prove the new module is real code, not just a thin forwarding shell.

### Collector Regression Coverage

Keep the existing local-collection tests green and add at least one new regression that covers behavior the previous tests did not pin down, preferably one of:

- local source-backed module exports and imports still populate qualified declarations and aliases
- trait or impl headers collected through the local collector still preserve methods and type-var bookkeeping

The full `cargo test -p rock-lib` suite must still pass.

## Risks

- If the new header module becomes a thin wrapper around the same lower-owned methods, the ownership shift is cosmetic.
- If the local source-backed path accidentally changes export-aware recursion, `Declarations` can diverge from what later lowering expects.
- If function or impl header helpers stop updating `function_type_vars` or `methods` consistently, later generalization and method lookup will regress.

## Follow-Up Work

- Move more shared helper state out of `Lowerer` so collect no longer depends on it as a mutable state carrier.
- Split dependency crate interfaces away from local-source collection.
- Add resolver-owned canonical name and alias tables.
- Retire the legacy string-keyed declaration maps once body lowering consumes resolved IDs.
