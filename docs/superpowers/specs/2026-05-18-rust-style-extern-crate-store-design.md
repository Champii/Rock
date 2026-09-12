# Rust-Style Extern Crate Store Design

**Date:** 2026-05-18
**Status:** Written for user review

## Purpose

The compiler has already moved downstream dependencies to product artifacts. The remaining architectural problem is that `LoadedCrate` still models source state and artifact state in one public object. Even when artifact-loaded dependencies carry empty AST/module-cache placeholders, the type still says a dependency may have source AST, file caches, module trees, object data, resolver data, metadata, and cross-crate HIR in one storage record.

The long-term direction should be closer to Rust's split between local source input and external crate metadata. Current-crate source belongs to the frontend/session. External crates belong to an artifact-only crate store. Compiler phases should query explicit metadata, body, and link capabilities, not inspect mixed storage or source/object modes.

This slice replaces the stale thin-provider direction with a stronger target: dependencies should not expose AST or source module caches as a type-level possibility.

## Current State

`LoadedCrate` currently combines these concerns:

- `manifest`, `ast`, `root_dir`, `module_tree`, and `file_cache` for source-backed crate loading and source artifact production.
- `interface`, `resolver`, root exports, prelude exports, and infix/product metadata for artifact dependencies.
- `cross_crate_hir` for generic functions, generic impls, and trait defaults imported from product artifacts.
- `object_path` and `backend_symbols` for object-backed artifact linkage.
- `ArtifactMode` to distinguish source, object, and interface-only storage modes.

Thin provider wrappers now exist (`downstream_metadata`, `downstream_bodies`, and `downstream_link`), and many phases already call them. That was useful as an intermediate boundary, but it does not fully solve the audit issue because phases still iterate `CrateContext::crates`, tests still construct `LoadedCrate` as dependency storage, and `LoadedCrate` still makes invalid dependency states representable.

## Target Architecture

Split source input from extern artifacts.

### Current Source Crate

Introduce a source-only record named `CurrentCrateSource` for local/current source workflows. It should own only source concerns:

- crate manifest
- root directory and library path context
- parsed root AST
- source module file cache
- optional source module tree

This record is for compiling source and producing artifacts. It is not a dependency capability and should not be handed to downstream dependency consumers.

### Extern Crate Store

Introduce an artifact-only store named `ExternCrateStore`. It owns `ExternCrateRecord` values keyed by session `CrateId`, with a name index for CLI-facing names such as `stdlib` and for diagnostics/link crate names.

An external dependency record should contain only artifact-backed capabilities:

- `ExternCrateMetadata`
- `ExternCrateBodies`
- `ExternCrateLink`

It should not contain AST, source file caches, source module trees, or source root state.

### Metadata Capability

`ExternCrateMetadata` exposes product artifact interface data and resolver-facing metadata:

- artifact crate interface
- resolver tables
- root exports and ID-backed root exports
- prelude exports and ID-backed prelude exports
- infix precedence/product metadata needed by phases

Collection and lowering should use this capability to register dependency declarations, dependency resolvers, root exports, prelude exports, and artifact metadata.

### Body Capability

`ExternCrateBodies` exposes serialized/lowered cross-crate bodies without source fallback:

- generic functions
- generic impls
- trait definitions with default methods

The API should provide direct accessors for the body categories instead of exposing the raw `ArtifactCrossCrateHir` bundle as the main phase-facing interface. Where IDs are available, access should be by canonical `DefId` or by canonical ID plus diagnostic/display names. Compatibility iteration by name can remain temporarily only where current monomorphization and lowering still require it.

### Link Capability

`ExternCrateLink` exposes object-backed linking data:

- whether concrete bodies are provided by an object artifact
- required object path
- backend symbols keyed by canonical `DefId`
- object-provided concrete function/impl decisions

Codegen setup should ask the extern store for object crate names and object paths. Monomorphization should ask link capabilities whether concrete dependency functions or impl methods are object-provided.

## Data Flow

External dependency loading:

```text
rockc --extern-artifact dep=dep.rkca
    -> read product artifact
    -> validate product artifact format and sidecars
    -> remap product crate IDs into session CrateIds
    -> build ExternCrateMetadata, ExternCrateBodies, and ExternCrateLink
    -> insert an artifact-only record into ExternCrateStore
    -> collect/lower/mono/codegen query the extern store
```

Current source compilation:

```text
entry .rk file
    -> parse current source AST
    -> collect and lower current source
    -> build HIR/MIR/mono/codegen
    -> emit product artifact data from current HIR and compiler products
```

Source-backed dependency loading should not be part of the extern crate store. If source package loading remains for current-crate artifact production or old source workflows, it should stay on the source side of `CrateContext` and never masquerade as an extern dependency.

## Phase Integration

Collection should register external declarations through `ExternCrateMetadata`. It should not receive dependency ASTs, inspect dependency source caches, or branch on source/object modes.

Lowering should import dependency resolver/export/prelude data through metadata and generic/default bodies through `ExternCrateBodies`. It should not read `ArtifactCrossCrateHir` fields from mixed crate storage.

Monomorphization should use `ExternCrateBodies` for generic functions, generic impls, and trait defaults. It should use `ExternCrateLink` for object-provided concrete functions, object-provided concrete impl methods, and backend symbols.

Codegen and linking should receive link inputs from the extern store. They should not filter loaded crates by `ArtifactMode`, inspect object paths on mixed storage, or infer dependency linkage from source records.

Artifact loading should construct extern records directly from product artifacts after validation and ID remapping. The artifact loader may use temporary local variables while assembling a record, but the resulting dependency object must be artifact-only.

## Error Handling

Source-backed external dependency consumption should fail before or during extern-store insertion with the existing user-facing guidance:

```text
source-backed external dependency '<name>' is not supported during <phase>; build it as a product artifact and pass it with --extern-artifact <name>=<path>
```

Missing metadata for an extern crate is an artifact/load error, not a later phase surprise.

Missing generic/default bodies should be represented by an empty body capability when the artifact legitimately has no generic/default bodies. If a phase requests a specific body by ID and it is absent, the error should name the missing body and artifact crate.

Object-backed extern crates without object paths are hard link-capability errors.

## Scope

This slice should:

- Remove `LoadedCrate` from phase-facing dependency APIs.
- Introduce `CurrentCrateSource` and `ExternCrateRecord` as separate source-only and artifact-only records.
- Add an extern crate store keyed by session `CrateId`, with name lookup for CLI names and diagnostics.
- Route collect, lower, mono, and codegen/link setup through extern-store metadata/body/link capabilities.
- Keep dependencies artifact-only and AST-free.
- Preserve product artifact crate ID remapping, stdlib prelude imports, generic function loading, generic impl loading, trait default loading, object-backed functions/impls, and object link behavior.
- Update the roadmap/audit checklist after implementation proof.

This slice should not:

- Reintroduce source-backed downstream dependencies.
- Add sysroot discovery, implicit stdlib loading, or compiler-owned stdlib registration.
- Redesign product artifact serialization unless a minimal sidecar addition is required by the new capability API.
- Implement transitive artifact discovery beyond the already explicit artifact dependency checks.
- Complete the later type context, selection service, instance graph, or MIR/codegen boundary tasks.

## Testing

Required coverage:

- Extern crate store construction from product artifacts produces artifact-only records with metadata, body, and link capabilities.
- Source-backed dependencies cannot be inserted into or consumed from the extern store.
- Compiler phases no longer accept `LoadedCrate` for external dependencies.
- Collection registers dependency functions, externs, structs, enums, traits, impls, root exports, prelude exports, resolver aliases, and infix metadata from metadata capabilities.
- Lowering imports dependency resolver data and cross-crate generic/default bodies from capability APIs.
- Monomorphization loads generic functions, generic impls, trait defaults, object-backed concrete functions, object-backed concrete impl methods, and backend symbols from body/link capabilities.
- Codegen/link setup gets object crate names and object paths from the extern store.
- Existing product artifact, stdlib prelude, generic functions, generic impls, trait defaults, object-backed dependencies, and product crate ID remapping tests continue to pass.
- Grep gates confirm `collect`, `lower`, `mono`, and codegen setup do not read dependency AST/source-cache fields or branch on `ArtifactMode` for external dependencies.

## Completion Criteria

This slice is complete when downstream dependency phases can only observe extern crates through artifact-backed metadata, body, and link capabilities. Dependency AST/source storage should no longer be present in the external dependency type. `LoadedCrate` is not a steady-state type for this architecture and should be deleted by the end of the slice; if an implementation plan needs a temporary loader assembly helper, it must be private, short-lived, and not a dependency representation.
