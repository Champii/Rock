# Multi-Module Item Index Design

**Date:** 2026-04-25
**Status:** Approved for implementation planning
**Scope:** Extend collection-time item indexing to represent root, inline, and already-known source-backed modules without changing lowering behavior.

## Purpose

The compiler architecture audit calls out that collection should allocate crate, module, and definition IDs before name resolution and lowering. The current `ItemIndex` now records root-module items with `DefId`s and carries the root `IndexingIds` through `Declarations`, but it still represents only the root module body.

This design makes the next behavior-neutral step: index multiple modules when their ASTs are already available. It gives future name-resolution work a module-aware item table without adding new file IO or moving module loading responsibilities into collection.

## Goals

- Allocate stable `ModuleId`s for the root module, inline child modules, and source-backed `mod` declarations.
- Record module metadata in the item index so later phases can understand module membership and parent-child relationships.
- Recursively index inline module ASTs.
- Index source-backed module bodies only when an AST is already available from existing compiler context.
- Preserve current lowering, import/export, prelude, crate-loading, and codegen behavior.

## Non-Goals

- Do not add filesystem traversal, parsing, or path resolution to `collect::item_index`.
- Do not replace `Lowerer` module loading yet.
- Do not implement name resolution, import resolution, export resolution, or alias maps.
- Do not allocate real dependency crate IDs.
- Do not change HIR, MIR, monomorphization, or codegen identity semantics.

## Architecture

### Indexing Context

`IndexingIds` should evolve from a copyable pair of root IDs into an indexing context that owns the allocation state needed during item collection. It should provide read access to the root crate/module IDs while allocating fresh module and local definition IDs internally.

This means `IndexingIds` will no longer be a `Copy` value once it owns `IdGen<ModuleId>` and `IdGen<LocalDefId>`. `Declarations` should still carry the finalized context or equivalent root/module identity data needed by later phases. The implementation plan should choose the smallest API change that avoids exposing allocator internals.

### Module Records

Add a module table alongside item records. Each module record should include:

- `module_id: ModuleId`
- `parent: Option<ModuleId>`
- `name: Option<String>`
- `kind: ModuleKind`

`ModuleKind` should distinguish at least:

- `Root`
- `Inline`
- `SourceBacked`

The source-backed kind records the module shell allocated for `TopLevel::Mod`. If an AST for that module is available from the caller, the same module record is used while indexing its body.

### Item Records

Keep `ItemRecord` behavior stable:

- Each named item still gets a `DefId`.
- Each item still records the `ModuleId` it belongs to.
- `TopLevel::Module` and `TopLevel::Mod` remain `ItemKind::Module` entries so module declarations are visible as named items.

The change is that child-module contents can now produce additional item records with the child module's `ModuleId`.

### Source-Backed Module AST Input

`collect::item_index` may consume already available ASTs, but it must not discover or parse them. The caller should provide a read-only view of source-backed modules that are already known from current compiler context, such as the existing module file cache carried through collection.

The exact key type should match existing available data to avoid inventing a new module loader abstraction in this step. If only paths are available today, the indexer can record source-backed module shells first and defer cached-body recursion until there is a clean key path from `TopLevel::Mod` to cached ASTs.

## Traversal

1. Create an indexing context with a root `CrateId` and root `ModuleId`.
2. Register a root `ModuleRecord`.
3. Walk root top-level items in source order.
4. For normal named items, allocate a fresh `LocalDefId` and push an `ItemRecord` in the current module.
5. For inline `TopLevel::Module`, allocate a fresh `ModuleId`, push its module item in the parent module, register a child `ModuleRecord`, then recursively index its body.
6. For source-backed `TopLevel::Mod`, allocate a fresh `ModuleId`, push its module item in the parent module, register a child `ModuleRecord`, then recurse only if the caller provided a matching already-parsed AST.

Definition IDs should remain crate-local and monotonically allocated across the whole indexed crate, not restarted per module.

## Data Flow

`collect::collect` should continue to run the existing declaration collection path through `Lowerer` for now. After that, it should build the multi-module `ItemIndex` from the root AST plus whatever already-known source-backed module ASTs are available without initiating new loads.

`Declarations` should carry the resulting item index and any ID context needed by future resolver work. Existing string-keyed declaration maps remain the active lowering inputs until a later migration replaces them.

## Error Handling

This step should not introduce new user-visible errors. Missing source-backed ASTs should be represented as indexed module shells, not as failures. That keeps current behavior unchanged and avoids making the item index responsible for module discovery diagnostics.

Duplicate names should continue to be represented as multiple `DefId`s under the same source name in `ItemIndex::defs_named`; name-resolution diagnostics are a later phase.

## Testing

Add focused `rock-lib` unit tests for:

- Root and inline module records receiving distinct `ModuleId`s.
- Inline module body items being indexed under the inline module's `ModuleId`.
- Source-backed `mod` declarations receiving module shell records and module item records.
- Source-backed cached AST bodies being indexed when a clean already-parsed AST input can be provided without adding IO.
- Definition IDs remaining monotonic across root and child modules.

Existing focused item-index and collect tests should be updated for any `IndexingIds` API changes. The final implementation must pass `cargo fmt --all`, focused item-index tests, focused collect tests, and `cargo test -p rock-lib`.

## Risks

- If cached source-backed AST lookup requires path resolution that only `Lowerer` can currently perform, this increment should stop at source-backed module shell records rather than duplicating path logic.
- Making `IndexingIds` own allocators will invalidate its current `Copy` test. That test should be replaced with tests that prove root IDs remain accessible and allocations are monotonic.
- Recursive indexing must avoid introducing semantic resolution. It should only record declarations present in already-available ASTs.

## Follow-Up Work

- Add a true module/source loader abstraction so collection can index source-backed modules without depending on `Lowerer` internals.
- Add resolver tables that map module paths, imports, exports, and prelude aliases to canonical IDs.
- Move declaration collection out of `Lowerer` and into a dedicated collection phase.
- Use the multi-module item index as the source of truth for lowering and diagnostics.
