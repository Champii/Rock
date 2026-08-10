# Complete Task 5 Lower Resolution Boundary Design

## Goal

Finish roadmap Task 5 by splitting source/path/name resolution policy out of `Lowerer` for body and type lowering. The previous ID-backed alias work completed the strict alias/path-resolution subset; this design completes the broader Task 5 checklist item without pulling in unrelated Task 17 source-loader or Task 18 lowerer-decomposition work.

The end state is that body/type lowering consumes resolved names or typed resolution results through a dedicated boundary instead of directly searching scope, aliases, module prefixes, dependency maps, prelude metadata, artifact maps, and fallback string maps inside lowering routines.

## Scope

In scope:

- Add a focused lower-side name-resolution component under `lib/src/lower/`.
- Move expression-path, type-name, trait-name, import, prelude, dependency, module-prefix, and artifact-root-export lookup policy behind intent-specific APIs.
- Keep source/display strings only for diagnostics, HIR display fields, and compatibility views explicitly derived from canonical IDs.
- Remove broad semantic resolution helper APIs from general `Lowerer` use once call sites migrate.
- Update the ordered roadmap and master audit checklist when the boundary is verified.

Out of scope:

- Removing all `Lowerer` fields.
- Replacing the module loader or source database model.
- Moving module traversal/cache ownership out of `Lowerer` beyond resolution-facing APIs.
- Retiring HIR string-keyed staging maps unrelated to source/path/name resolution.
- Replacing structural `Type` with `TypeId` or `Ty`.

Those remain under Task 17, Task 18, Task 11, Task 13, or Task 21 as currently documented.

## Recommended Approach

Use a narrow resolver-boundary extraction rather than a full lowerer rewrite or a new AST pre-resolution pass.

This approach finishes Task 5 by centralizing policy and moving body/type lowering onto typed resolver APIs. It avoids cross-cutting source-loader and lowerer-state migration, which would mix roadmap tasks and make the implementation harder to verify.

## Architecture

Introduce a dedicated lowering resolution boundary in `lib/src/lower/resolution.rs`. It provides a read-only policy layer over existing canonical data:

- local scope bindings
- current-crate `ResolverTables`
- dependency `ResolverTables`
- current crate name and current qualified module prefix
- module graph/cache lookup views already prepared by collection/source loading
- ID-backed stdlib prelude exports
- ID-backed artifact root exports
- HIR declaration indexes held during lowering

`Lowerer` may still own this data initially, but body/type lowering should ask the resolution boundary for named decisions. `Lowerer` becomes a state holder and mutation coordinator rather than the place where each lowering routine reimplements resolution policy.

## Components

### `LowerResolutionContext`

`LowerResolutionContext` is a lightweight read-only view constructed from `Lowerer` when resolving a name/path. It should not lower AST or mutate HIR. It owns precedence rules and canonical lookup chains.

Initial APIs should be intent-specific, for example:

- `resolve_value_path(path, mode) -> LowerValueResolution`
- `resolve_type_name(name) -> LowerTypeResolution`
- `resolve_trait_name(name) -> LowerTraitResolution`
- `resolve_import_source(path) -> LowerImportResolution`
- `resolve_module_path(path) -> LowerModuleResolution`
- `canonical_name_for_id(id) -> Option<&str>`
- `canonical_name_for_resolution(...) -> Option<String>`

The final API names may differ, but each method should encode the caller intent instead of exposing raw resolver maps.

### Resolution Result Types

Use explicit result enums so lowering can distinguish successful canonical resolution from unresolved names without guessing:

- local binding with `HirLocalId`, type, mutability, and source name
- function target with `DefId`, canonical name, and function type
- extern target with `DefId`, canonical name, and extern type
- struct/enum/trait target with `DefId` and generic metadata
- module target or artifact root export target
- unresolved result with source text/span context for diagnostics

Results should retain source/display strings only as metadata. Semantic identity should come from IDs.

### `Lowerer` Facade During Migration

Short-term, `Lowerer` can expose small facade methods that delegate into `LowerResolutionContext`. Existing methods such as `resolve_item_def_id`, `resolve_module_alias_or_item_def_id`, `canonical_name_for_alias_or_item_lossy`, `trait_by_name`, `struct_by_resolved_name`, and `enum_by_resolved_name` should either migrate into the resolution module, become private implementation details, or be deleted after callers move.

The migration should avoid creating another broad helper with the same problem. New APIs should answer concrete lowering questions.

## Data Flow

1. Collection builds declarations, current-crate IDs, resolver tables, and ID-backed alias/export metadata.
2. `LoweringPipeline` initializes `Lowerer` from collection outputs and dependency/prelude/artifact metadata.
3. Body/type lowering encounters a source name or path.
4. The lowering routine asks `LowerResolutionContext` for an intent-specific resolution.
5. The resolution boundary applies precedence in one place:
   - local scope when local names are valid for the construct
   - scoped module-local aliases before root items where module-local precedence applies
   - current-crate canonical resolver paths
   - dependency resolver paths
   - prelude exports only where prelude policy allows
   - artifact root exports only for import/glob-import resolution
6. Lowering receives a typed result and emits HIR IDs/targets or a diagnostic.
7. Later phases consume HIR semantic IDs and do not reconstruct targets from source names.

## Error Handling

Resolution APIs should return unresolved or ambiguous result variants with enough source context for existing diagnostics. They should not fabricate IDs, silently choose by suffix when canonical IDs were expected, or fall back to display names for semantic ownership.

Stale product aliases and ambiguous product identities remain product/artifact boundary concerns. Loading or product emission should drop/reject them before lowering; lowering should not repair them.

Existing suffix or display-name fallback logic must be audited. If any fallback remains, it must be explicitly classified as display/diagnostic compatibility and not as semantic resolution.

## Testing Strategy

Add focused unit tests for the resolution boundary and keep existing path/type lowering tests as end-to-end coverage.

Required behavior tests:

- local variables shadow import/module aliases where expression resolution permits locals
- module-local aliases shadow root items for value paths and type annotations
- dependency aliases resolve by `DefId`, not display string
- prelude exports resolve only through ID-backed prelude metadata
- artifact root exports resolve only through `ArtifactExport` IDs
- unresolved canonical IDs produce diagnostics instead of fallback IDs

Required architecture checks:

- direct `dependency_resolvers` access outside the resolution module is removed or justified
- direct `resolver.import_aliases`, `resolver.export_aliases`, or `resolver.module_aliases` policy checks outside the resolution module are removed or justified
- broad `Lowerer` semantic resolution helpers are deleted, private to resolution, or replaced with intent-specific calls
- suffix-name type lookup fallbacks are removed or marked as non-semantic compatibility

Verification commands:

```bash
cargo test -p rock-lib alias
cargo test -p rock-lib product_artifact
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Final review should specifically inspect that Task 5 is now marked complete only after the broader checklist item is satisfied.

## Documentation Updates

After implementation and review:

- Update `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` so Task 5 is `Complete`, not scoped.
- Update `docs/superpowers/plans/master-audit-checklist.md` to check off splitting source/path/name resolution out of `Lowerer`.
- Keep Task 17/18 remaining work documented for module loader ownership and broader `Lowerer` state decomposition.
- Record verification evidence in the implementation plan.

## Completion Criteria

- Body/type lowering routes source/path/name resolution through the dedicated lower resolution boundary.
- Resolution precedence is centralized and tested.
- No supported lowering path fabricates semantic IDs or reconstructs semantic targets from display strings.
- Remaining direct `Lowerer` state ownership is not source/path/name resolution policy, or is documented as Task 17/18 follow-up.
- Ordered roadmap and master checklist mark Task 5 complete with broader lowerer/source-loader work still scoped to later tasks.
- Focused tests, full `cargo test -p rock-lib`, formatting, diff whitespace, and final code review pass.
