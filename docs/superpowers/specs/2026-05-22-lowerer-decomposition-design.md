# Lowerer Decomposition Design

## Goal

Finish roadmap Task 18 by turning `Lowerer` from a broad owner of orchestration, module context, diagnostics, body lowering, type lowering, and selection setup into a set of narrow boundaries with explicit responsibilities.

## Current State

`Lowerer` already delegates some behavior to submodules and services, but the central struct still owns too many unrelated responsibilities:

- Pipeline ordering in `lib/src/lower/program.rs`, including prelude setup, trait default lowering, trait conformance, dependency body loading, local body lowering, and final `PartialHir` assembly.
- Module traversal and source graph cache lookup, including `loaded_module_paths`, `module_file_cache`, `current_module_path`, `current_qualified_module_prefix`, and `module_local_aliases`.
- Diagnostics, including current span tracking, error accumulation, and duplicate suppression.
- Body lowering state, including scope, current function, generic context, unsafe context, tuple temporaries, and inference/constraint state.
- Service access for parsed type lowering and trait/method/operator selection.

The Task 17 source loader boundary makes one important invariant non-negotiable for this work: lowering may consume graph-seeded module paths and cached ASTs, but it must not reconstruct sibling paths or perform source file IO.

## Non-Goals

- Do not redesign HIR, `Type`, `TypeId`, inference, selection, MIR, monomorphization, or codegen semantics.
- Do not remove compatibility source-name maps unless a slice can do so without broad downstream changes.
- Do not introduce filesystem IO or sysroot/stdlib discovery into lowering.
- Do not change Rock language behavior as part of decomposition.
- Do not rewrite expression or statement lowering wholesale.

## Boundary Map

### `LoweringPipeline`

`LoweringPipeline` owns phase ordering and final result assembly. It should be the only boundary that knows the high-level sequence for `lower_from_declarations`:

1. Build mutable lowering state from `collect::Declarations`.
2. Apply current-crate file/module identity.
3. Register dependency resolvers and prelude aliases when applicable.
4. Auto-implement marker traits.
5. Lower trait defaults.
6. Run trait conformance.
7. Load dependency generic bodies.
8. Lower current-crate and loaded source-module bodies.
9. Sync prelude/export aliases.
10. Return `PartialHir` or accumulated `ResolveError`s.

The existing `lower_from_declarations` public function should remain the stable entry point and delegate to this pipeline.

### `ModuleLoweringContext`

`ModuleLoweringContext` owns module traversal and module-local lookup policy. It should encapsulate:

- Current source module path and qualified module prefix.
- SourceDatabase-provided `loaded_module_paths` and `module_file_cache`.
- Current-crate-prefixed graph path lookup.
- Loaded module body traversal.
- Module-local import aliases and cleanup.

It must only resolve modules through graph/cache data received from collection. A missing module must produce the existing "Module '...' was not loaded by the source database" diagnostic rather than probing the filesystem.

### `LowerDiagnostics`

`LowerDiagnostics` owns lowering diagnostic state:

- `Vec<ResolveError>` accumulation.
- Current span tracking.
- Span-aware push helpers.
- Duplicate-message suppression for repeated dependency/source errors.

The initial slice may keep compatibility methods on `Lowerer`, but those methods should delegate to a diagnostics field so later body-lowering code can depend on a narrow diagnostic interface.

### `BodyLowerer`

`BodyLowerer` owns AST-to-HIR body conversion after declarations, modules, prelude aliases, dependency bodies, trait defaults, and conformance setup are ready. It keeps the mutable state required by expressions, statements, patterns, function bodies, generic contexts, unsafe context, inference, and constraints.

This boundary should not own high-level pipeline ordering, source module discovery, dependency provider policy, or prelude injection policy.

### Existing Services

The decomposition should preserve and reuse existing services rather than create competing abstractions:

- `TypeLowerer` remains the parsed-type conversion boundary.
- `SelectionService` remains the trait/method/operator/index selection boundary.
- `CrateContext` and dependency provider APIs remain the dependency metadata/body/link boundary.
- `SourceDatabase` and `ModuleGraph` remain the source/module loading boundary.

## Data Flow

The intended data flow for the main pipeline is:

```text
collect::Declarations + ast::Program + CrateContext + current_crate_name
    -> LoweringPipeline
    -> mutable lowering state
    -> ModuleLoweringContext for module traversal/cache lookup
    -> BodyLowerer for function/impl/trait-default bodies
    -> PartialHir or Vec<ResolveError>
```

`LoweringPipeline` should own the phase sequence. `ModuleLoweringContext` should provide loaded modules and temporary module-local aliases. `BodyLowerer` should lower bodies against already-available declarations and services.

## Implementation Strategy

Use thin, behavior-preserving slices. Each slice should compile and pass tests before the next one starts.

1. Introduce a pipeline wrapper around the existing `lower_from_declarations` sequence without changing behavior.
2. Move module graph/cache lookup and loaded-module traversal behind a module-context boundary.
3. Move diagnostics storage and push helpers behind a diagnostics boundary.
4. Move prelude/dependency/trait-default/conformance setup behind pipeline helper phases.
5. Narrow body lowering entry points so expression/statement/function lowering no longer reaches directly into pipeline or module-loading policy.
6. Remove compatibility methods only after all call sites are migrated and tests prove no behavior changed.

## Error Handling

- Preserve `ResolveError` as the user-facing lowering diagnostic type.
- Preserve spans where existing code has spans.
- Keep duplicate suppression for repeated source-backed dependency errors.
- Report module/cache misses through structured lowering errors, not panics.
- Do not add new `unwrap`, `expect`, or `panic!` calls to user-facing lowering paths.

## Testing Strategy

Each implementation slice should follow TDD:

- Add a focused regression for the boundary behavior before moving code.
- Run the focused test and confirm it fails for the expected reason.
- Move the minimal code needed to pass.
- Run the focused test again.
- Run `cargo fmt --all --check` and the smallest relevant `cargo test -p rock-lib <filter>` command.
- Before the final Task 18 commit, run `cargo test -p rock-lib` and `git diff --check`.

Regression coverage should include:

- Current-crate and nested module body lowering.
- Loaded source-module trait defaults.
- Module-local imports and glob imports.
- Stdlib prelude injection and synchronization.
- Dependency generic body loading.
- Trait conformance and default injection ordering.
- Diagnostics for missing module cache entries.

## Completion Criteria

Task 18 is complete when:

- `lower_from_declarations` delegates phase orchestration to a named pipeline boundary.
- Module graph/cache lookup and loaded-module traversal are isolated from body-lowering logic.
- Diagnostics are isolated behind a narrow diagnostics boundary or field.
- Prelude/dependency/trait setup is explicit pipeline setup, not interleaved with body lowering.
- Body lowering no longer owns module loading or high-level phase ordering policy.
- Existing public lowering entry points remain stable.
- `cargo fmt --all --check`, focused regression tests, `cargo test -p rock-lib`, and `git diff --check` pass.

## Risks

- The `Lowerer` struct is still widely referenced by expression, statement, pattern, trait, and helper modules. Decomposition must avoid a large-bang rewrite.
- Some compatibility maps are still needed by downstream phases; removing them belongs to narrower future identity tasks unless a Task 18 slice proves the removal safe.
- Module-local aliases and prelude aliases are intertwined with path lowering. Migrations must preserve same-name and shadowing tests.
- Trait conformance depends on trait default bodies being lowered before impl method bodies. The pipeline must keep this ordering explicit.
