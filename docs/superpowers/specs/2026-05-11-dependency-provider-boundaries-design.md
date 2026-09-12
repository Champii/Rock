# Dependency Provider Boundaries Design

**Date:** 2026-05-11
**Status:** Draft for review

## Purpose

The artifact-only downstream dependency slice removed source-backed dependency consumption from `collect`, `lower`, and `mono`. The next cleanup should finish the provider boundary that remains in the audit checklist: compiler phases should ask for dependency capabilities instead of inspecting `LoadedCrate` storage details.

This is a narrow architectural cleanup, not an artifact schema redesign. The goal is to keep the current product artifact data model while hiding storage mode and field layout behind explicit provider APIs.

## Current State

`LoadedCrate` still stores several concerns together:

- source AST and module cache for current-crate/artifact-production paths
- artifact metadata interface
- resolver and prelude export data
- cross-crate HIR body bundles
- object/link path state
- storage mode via `ArtifactMode`

Downstream source consumption is gone, but phases still branch on or read these storage details:

- `collect` and `lower` call `downstream_interface` and inspect object-backed status for impl body ownership.
- `lower` reads `loaded_crate.cross_crate_hir` directly for generic bodies and trait defaults.
- `mono` reads object-backed status and cross-crate HIR directly.
- `lib.rs` collects object-backed crate names and object paths directly from `CrateContext`/`LoadedCrate` for codegen/linking.

This keeps dependency storage mode visible throughout the pipeline and makes future canonical identity work harder than necessary.

## Target Architecture

Introduce explicit dependency provider capabilities owned by the crate system:

- **Metadata provider:** exposes artifact interface data, resolver tables, root exports, prelude exports, and infix precedence for downstream phases.
- **Body provider:** exposes cross-crate generic function bodies, generic impl bodies, and trait default bodies.
- **Link provider:** exposes whether concrete bodies are object-provided and which object files must be linked.

Compiler phases should depend on these capabilities, not on `LoadedCrate` fields or `ArtifactMode` branches.

`LoadedCrate` may remain as the internal storage struct for this slice, but direct field access from downstream phases should shrink. The provider methods can initially be thin wrappers over existing fields.

## Scope

This slice should:

- Add provider-style methods or small provider structs in `crate_system`.
- Route `collect`, `lower`, `mono`, and codegen setup through those provider APIs.
- Keep source-backed current-crate artifact production working.
- Keep product artifact metadata, cross-crate HIR, and object link behavior unchanged.
- Keep source-backed downstream dependency rejection in place.
- Update tests to assert provider behavior and preserve artifact-only boundary regressions.
- Update the audit checklist if provider branching is meaningfully hidden from downstream phases.

This slice should not:

- Redesign the product artifact schema.
- Remove `ArtifactMode` internally if doing so creates a large migration.
- Implement `rockc` transitive artifact resolution from product metadata.
- Resume canonical identity work in the same patch set.
- Remove current-crate source parsing or source artifact production paths.

## Proposed API Shape

The exact names can follow existing module style, but the capabilities should be conceptually explicit:

```rust
impl LoadedCrate {
    pub(crate) fn downstream_metadata(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<DependencyMetadata<'_>, String>;

    pub(crate) fn downstream_bodies(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<DependencyBodies<'_>, String>;

    pub(crate) fn downstream_link(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<DependencyLink<'_>, String>;
}
```

`DependencyMetadata` should expose the current artifact interface and export/prelude data without requiring callers to know where it is stored.

`DependencyBodies` should expose cross-crate HIR body bundles, including an empty provider for artifact modes that have metadata but no body bundle.

`DependencyLink` should expose object-backed information and object paths without requiring callers to branch on `ArtifactMode`.

The provider API should reuse the existing artifact-only rejection wording for source-backed dependencies.

## Phase Changes

### Collection

Collection should request `downstream_metadata` for dependency declarations, root exports, prelude exports, and impl body ownership metadata. It should not call `downstream_interface` directly or inspect object-backed storage mode.

### Lowering

Lower registration should request `downstream_metadata`. Body lowering should request `downstream_bodies` and apply generic bodies/trait defaults from that provider. Lower should not read `loaded_crate.cross_crate_hir` directly.

### Monomorphization

Mono should request body and link providers. Generic functions and impls should come from `downstream_bodies`; object-provided concrete impl/function behavior should come from `downstream_link`.

### Codegen Setup And Linking

`lib.rs` should ask `CrateContext` for dependency link inputs rather than filtering loaded crates by object-backed mode. Existing `get_object_paths` can be replaced or backed by `DependencyLink` internally.

## Error Handling

Provider requests for source-backed downstream dependencies should return the same clear error family already used by `downstream_interface`:

```text
source-backed external dependency '<name>' is not supported during <phase>; build it as a product artifact and pass it with --extern-artifact <name>=<path>
```

Missing metadata for a non-source dependency should remain an explicit compiler/internal misuse error. Missing optional body bundles should be represented as an empty body provider unless the caller requires a specific body category.

## Testing

Required coverage:

- Source-backed downstream dependency rejection still fires through provider methods.
- Artifact metadata providers expose functions, externs, structs, enums, traits, impls, root exports, prelude exports, resolver aliases, and infix precedence.
- Body providers expose generic functions, generic impls, and trait default methods from cross-crate HIR bundles.
- Link providers expose object-backed status and object paths without downstream phases checking `ArtifactMode`.
- Current-crate source module tests continue to pass.
- Product artifact tests for source-free dependencies, stdlib product artifacts, generic functions, generic impls, and trait defaults continue to pass.
- Grep-style checks confirm `collect`, `lower`, `mono`, and codegen setup no longer branch directly on `ArtifactMode`/`is_object_backed` or read `loaded_crate.cross_crate_hir`.

Useful final commands:

```bash
cargo fmt --all --check
cargo test -p rock-lib crate_artifact
cargo test -p rock-lib products
cargo test -p rock
cargo test
```

## Completion Criteria

This slice is complete when:

- downstream phases use provider capabilities rather than direct `LoadedCrate` storage fields for metadata, bodies, and link data
- source-backed downstream dependencies remain rejected
- current-crate source artifact production remains valid
- product artifact and `rock` transitive artifact tests pass
- the audit checklist can mark provider-boundary hiding complete while keeping narrower future improvements, such as splitting `LoadedCrate` storage itself, open if still warranted
