# Artifact Interface Collect Split Design

**Date:** 2026-04-28
**Status:** Auto-approved for implementation
**Scope:** Move artifact interface declaration/bootstrap ownership from `Lowerer` to `collect`, while preserving the later artifact-specific impl-ABI/body refinement step by re-entering `Lowerer` through `Lowerer::from_declarations(...)`.

## Purpose

The main compile pipeline no longer constructs a bootstrap `Lowerer` during declaration collection, but artifact interface building still does. `lib/src/crate_artifact/build.rs::build_interface(...)` currently creates a fresh `Lowerer`, registers dependency crates through lower-owned helpers, copies current-crate module caches into that lowerer, and then runs lower-owned declaration collection before starting the later artifact-specific refinement pass.

That leaves artifact building as the remaining prominent declaration-time path that depends on the lowering god object for work that now belongs to `collect`. The next narrow step is to move just the artifact interface declaration/bootstrap phase onto collect-owned state.

## Goals

- Remove the declaration/bootstrap `Lowerer` construction from artifact interface building.
- Reuse collect-owned dependency bootstrap and crate declaration collection for artifact interface gathering.
- Preserve current artifact interface contents for functions, structs, enums, traits, externs, impls, root exports, and infix precedence.
- Preserve the later artifact-specific impl ABI/body refinement pass by rebuilding a `Lowerer` from collected declarations.

## Non-Goals

- Do not change `collect::collect(...)` for the main compile pipeline.
- Do not move artifact cross-crate HIR construction off `Lowerer` in this step.
- Do not add resolver tables or canonical ID-based resolution yet.
- Do not change artifact file format, object/source/interface mode behavior, or later codegen/mono logic.

## Why This Is The Next Step

The audit track still calls out that collection is not yet a true independent phase and that crate/artifact concerns leak into lowering. After `collect-bootstrap-split`, the cleanest remaining declaration-time boundary leak is artifact interface building:

- it bootstraps dependencies via `Lowerer`
- it uses lower-owned declaration collection to populate the artifact interface
- only after that does it perform the artifact-specific later-lowering work it actually still needs

This is a narrower and safer next step than jumping to resolver tables. It removes a real boundary violation without changing the later artifact pipeline yet.

## Architecture

### Add An Artifact-Specific Collect Entry Point

Add a collect-owned helper dedicated to artifact interface gathering. This helper should:

1. bootstrap a `CollectContext` for the crate being built
2. register dependency crates from `CrateContext`, excluding the crate currently being built
3. seed the current crate root path and current crate file cache into the collect context
4. collect qualified declarations from the crate AST using the collect-owned crate declaration path
5. return a `Declarations` value that can be consumed by later phases

This should live alongside existing collect-owned helpers, not inside `crate_artifact`.

### Self-Skip Dependency Bootstrap

Artifact interface building differs from the main compile pipeline in one important way: the `CrateContext` passed into artifact building already contains the crate currently being built.

If the new helper naively reuses `register_crate_functions(...)`, it will preload the current crate as a dependency and then collect it again locally, which can duplicate impls/externs and blur ownership.

The artifact helper must therefore bootstrap dependencies with an explicit `skip_crate_name` rule.

### Keep Current-Crate Declaration Ownership In Collect

The current crate should be gathered with collect-owned declaration traversal using the crate-qualified path, not with the local collector used by normal compilation. Artifact interfaces want crate-qualified names like `dep::answer`, exported source-backed module declarations, and stdlib prelude export discovery, all of which already align with `CollectContext::collect_crate_declarations(...)`.

### Re-Enter Lowering Only For Later Artifact Refinement

Once declarations are collected, artifact interface building should construct the later refinement `Lowerer` with `Lowerer::from_declarations(decls)`.

That lowerer still owns the later artifact-only steps in this slice:

- `auto_impl_sized()`
- trait default body lowering for the current crate
- conformance checks
- current-crate body lowering used to infer concrete impl method ABIs for the exported interface

This keeps the slice behavior-neutral while moving declaration ownership to the correct phase.

## Boundary Rules

- `crate_artifact::build_interface(...)` must no longer create a bootstrap `Lowerer` for declaration collection.
- collect-owned code may add a small artifact-specific helper instead of widening the generic `collect::collect(...)` API unnecessarily.
- `Lowerer::from_declarations(...)` remains the re-entry point for later artifact refinement.
- `build_cross_crate_hir(...)` stays unchanged in this step.

## Testing

Add focused coverage for the new seam:

- a crate-artifact test proving collect-owned artifact declaration bootstrap skips the crate currently being built instead of double-registering it
- existing artifact interface regression coverage must stay green, especially stdlib export/interface tests and artifact-backed compile/runtime tests
- full `cargo test -p rock-lib` must still pass

## Risks

- Reusing the wrong collect entry point could silently duplicate current-crate impls or externs.
- Artifact interface building relies on collect-time module cache/root-path setup for source-backed submodules; missing one of those seeds could break exported module harvesting.
- `Lowerer::from_declarations(...)` currently rebuilds body-lowering state from declarations only; this step must avoid accidentally depending on dropped collect-only details.

## Follow-Up Work

- remove or shrink lower-owned declaration helpers that are no longer used by artifact interface building
- consider whether `build_cross_crate_hir(...)` can later reuse the collect/lower split as well
- continue into resolver-owned path/alias tables after the remaining declaration-time lower dependencies are gone
