# Collect-Owned Bootstrap Split Design

**Date:** 2026-04-28
**Status:** Auto-approved for implementation
**Scope:** Remove the last `collect::collect -> Lowerer` bootstrap dependency by moving dependency crate registration and stdlib prelude bootstrap into `collect`, while keeping `Lowerer::from_declarations` and later body lowering behavior unchanged.

## Purpose

The previous refactors moved local declaration gathering and local collect-time helper state into `collect`, but `collect::collect` still constructs a bootstrap `Lowerer` just to register dependency crates, stdlib prelude aliases, crate module caches, and artifact module summaries before handing that state back into `CollectContext`.

That leaves the collection phase with one remaining hard dependency on the lowering god object. The next narrow step is to move that dependency bootstrap into `collect` as well, so the main collection entrypoint no longer constructs `Lowerer` at all.

## Goals

- Remove `Lowerer::bootstrap_for_collection(...)` from `collect::collect`.
- Make `CollectContext` own dependency crate registration and stdlib prelude bootstrap for the collect phase.
- Preserve current `Declarations` shape and `Lowerer::from_declarations` behavior.
- Preserve source-backed dependency, interface-only dependency, artifact-module-summary, and stdlib prelude behavior visible to later lowering.

## Non-Goals

- Do not add resolver tables or canonical path/alias resolution.
- Do not change `Lowerer::from_declarations` behavior.
- Do not change body lowering, trait default lowering, monomorphization, or codegen.
- Do not rewrite artifact interface building in this step unless a minimal compatibility adjustment is required.

## Why This Is The Next Step

The `Real Collection And Name Resolution` track still has one obvious violation of the intended boundary:

- `collect::collect` constructs a `Lowerer`
- the bootstrap `Lowerer` registers dependency crates and prelude aliases
- `CollectContext` then imports that state back out of `Lowerer`

That is now unnecessary indirection. `CollectContext` already owns the active collect-time maps, loader helpers, and header builders used by local collection. Dependency bootstrap should feed that same state directly.

## Architecture

### Collect-Owned Bootstrap Context

Extend `CollectContext` with a collect-owned bootstrap constructor, for example `CollectContext::bootstrap_for_collection(...)`, that initializes:

- current crate name
- current module path
- current root module entry in `loaded_module_paths`

This replaces the current `Lowerer::bootstrap_for_collection(...)` use in `collect::collect`.

### Move Dependency Registration Into Collect

Move the collect-time dependency bootstrap surface currently used through `Lowerer` into `CollectContext`:

- register loaded dependency crates
- copy interface-only declarations into collect maps/scope
- collect declarations from source-backed dependency ASTs into collect maps
- copy `module_file_cache`, `artifact_module_index`, and `stdlib_prelude_exports`
- inject prelude short-name aliases from `stdlib_prelude_exports`

The main `collect::collect` data flow becomes:

1. Create `CollectContext` directly.
2. Register dependency crates into that context.
3. Optionally inject stdlib prelude aliases into that context.
4. Run `LocalCollector` on local source declarations using that same context.
5. Produce `Declarations` from collect-owned state.

### Source-Backed Dependency Collection

Source-backed dependency crates still need declaration collection for:

- exported root items
- exported inline/source-backed submodules
- stdlib prelude exports gathered from `stdlib::prelude`

This behavior should move into `collect` in a behavior-preserving form. The simplest path is to add collect-owned equivalents of the current dependency declaration helpers used by `register_loaded_crate(...)`, keeping the current export gating and qualified-name behavior.

### Lowerer Stays The Body-Lowering Owner

This step does not change the later lower pipeline:

- `Lowerer::from_declarations(...)` still reconstructs body-lowering state from collected declarations
- `lower_from_declarations(...)` still injects prelude aliases for body lowering
- crate trait/default/body lowering still lives in `lower`

Only the collect-time bootstrap ownership moves.

## Boundary Rules

- `collect::collect` must no longer construct `Lowerer`.
- `CollectContext` must become the owner of collect-time dependency bootstrap state.
- `Lowerer` may keep its existing dependency registration helpers for other current call sites in this step.
- `Declarations` field layout stays unchanged.
- Existing string-keyed declaration maps remain active until later resolver work replaces them.

## Testing

Add focused coverage for the new boundary:

- a collect-level test proving `CollectContext` can bootstrap source-backed dependency declarations without going through `Lowerer`
- a collect-level test proving `CollectContext` can bootstrap stdlib prelude aliases from dependency metadata without going through `Lowerer`
- existing collect regressions must stay green
- full `cargo test -p rock-lib` must still pass

## Risks

- This step duplicates more dependency-bootstrap logic between `collect` and `lower`; that duplication is acceptable only as a narrow transitional step.
- Missing one collect-time side effect during the move could silently break source-backed dependency qualification, prelude aliases, or artifact glob/export behavior.
- If this step starts changing body-lowering bootstrap too, scope will sprawl beyond the current architecture slice.

## Follow-Up Work

- remove or shrink the lower-owned collect/bootstrap helpers once no active collect path needs them
- split artifact-interface building off lower-owned declaration collection where appropriate
- start resolver-owned canonical path and alias tables after collection no longer depends on `Lowerer`
