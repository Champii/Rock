# Crate Artifact Design

## Goal

Replace stdlib-only caching with a generic crate artifact model that works for any dependency crate.

- `rockc` stays low-level and explicit.
- Dependency source paths remain the active interface until the artifact format exists.
- `rock` will later be responsible for building, caching, and passing crate artifacts to `rockc`.

## Current Implementation

An initial artifact slice now exists in `lib/src/crate_artifact.rs`.

It currently provides:

- serializable crate artifact structs
- artifact build helpers on `CrateContext`
- binary artifact read/write helpers
- dependency identity capture
- per-module export indexes
- resolved stdlib prelude export capture
- object output metadata passthrough
- source fingerprint generation
- a typed interface payload for exported declarations, impl-visible method signatures, operator precedence, module/export indexes, and stdlib prelude metadata
- a typed cross-crate HIR bundle for generic functions, generic impls, and trait default methods that consumers still need to instantiate
- source bundles only for explicit non-object artifacts that still need the old no-object fallback path
- `rock build` support for recursively materializing cached path-dependency artifacts and compiling the root crate with `extern_artifact` inputs
- `rock artifact` support for emitting the current crate artifact into its local `build/artifacts/` cache
- dependency object-file production and reuse through artifact metadata during `rock build`
- linker-safe internal symbol export for artifact-built dependency objects

Default object-backed artifacts are now source-free. Normal dependency loads use the typed interface plus the Phase 11 cross-crate HIR bundle instead of dependency AST/file caches. Source bundles remain only for explicit non-object artifacts.

## Current State

The compiler currently has two useful building blocks:

1. `lib/src/crate_system/mod.rs`
- `LoadedCrate` already stores manifest data, parsed root AST, optional object path, module tree, and a per-crate `file_cache` of parsed modules.
- `CrateContext` already manages dependency loading and topological ordering.

2. `lib/src/collect/mod.rs` and `lib/src/infer/mod.rs`
- The compiler already has a natural interface boundary between parsing/lowering and later compilation.
- `Declarations` and `PartialHir` show which pieces of dependency information are actually useful downstream.

This means the compiler already has in-memory crate reuse, but not a stable serialized dependency interface.

## Non-Goals

- Do not serialize the full AST, `Lowerer`, `Declarations`, `PartialHir`, or `CrateContext` as-is.
- Do not make `rockc` discover dependencies automatically.
- Do not add stdlib-only artifact behavior.
- Do not introduce backward-compatible transitional formats.

## Artifact Boundary

The artifact should describe the public dependency interface the downstream crate needs, plus optional compiled outputs.

It should not describe arbitrary compiler internals.

### Proposed top-level shape

```text
CrateArtifact
- format_version
- crate_identity
- dependency_identities
- interface
- prelude_exports
- module_index
- object_output
- source_fingerprint
```

## Proposed Fields

### `format_version`

- Monotonic artifact schema version.
- Increment whenever deserialization expectations change.
- No compatibility shims: old artifacts become invalid and must be rebuilt.

### `crate_identity`

- crate name
- crate version
- canonical crate root path or package root path
- artifact key inputs relevant to semantic compatibility

This is used for validation and cache lookup.

### `dependency_identities`

- direct dependency names
- the resolved identity/hash of each dependency artifact

This lets `rock` invalidate dependents when an upstream crate changes.

### `interface`

Serialized information needed for downstream resolution and type checking.

This should include:

- exported functions and signatures
- exported extern signatures
- exported structs, enums, traits, impl-visible method metadata
- infix precedence declarations
- import/export alias information that downstream lowering needs

This is the real replacement for reparsing dependency source trees during downstream compilation.

### `prelude_exports`

- only needed for crates named `stdlib`
- records the resolved exports of `stdlib::prelude`

This preserves the one allowed stdlib-specific compiler behavior without making stdlib loading special.

### `module_index`

- module name to exported-item mapping
- qualified-path lookup metadata
- optional file/module fingerprints for development tooling

This replaces the need to keep dependency source ASTs around just to answer module/export questions.

### `object_output`

- optional path to an already built object file
- optional metadata about how it was produced

This lets downstream linking reuse compiled dependency code when available.

### `source_fingerprint`

- hash of the crate manifest
- hash of the crate root and loaded module files
- relevant compiler-option fingerprint

This is the minimal invalidation key for crate-level incremental compilation.

## What Should Stay Out Of The Artifact

- parser token streams
- full ASTs for dependency bodies
- temporary lowering state
- inference-engine internals
- borrow-check or MIR temporary state
- ad hoc stdlib shortcuts

If a downstream compile needs these, the boundary is too low-level.

## Compiler Integration Direction

### Current API

Today `rockc` effectively consumes dependency source directories through explicit paths.

```text
rockc
  -> CrateContext::load_crate_from_dir
  -> parse dependency source
  -> collect declarations
  -> lower against in-memory dependency state
```

### Future API

Later `rockc` should consume dependency artifacts instead.

```text
rockc
  -> load artifact metadata/interface
  -> use dependency interface during collect/lower/infer
  -> link dependency object outputs when present
```

The main crate being compiled can still be source-based.

## Transition Plan

### Step 1

Define serializable artifact structs in a dedicated module.

### Step 2

Build an in-memory conversion from loaded dependency state to the artifact interface.

### Step 3

Teach `rock` to emit artifact files and manage their cache keys.

### Step 4

Teach `rockc` to accept artifact inputs for dependencies while still accepting source paths during the transition.

### Step 5

Remove source-based dependency loading from normal `rock` workflows once artifact loading is solid.

## Why This Matches The Stdlib Direction

With this design:

- `stdlib` is just another dependency artifact
- there is no stdlib-only cache path
- there is no compiler-owned stdlib loading logic
- the only remaining stdlib-specific compiler behavior is conditional prelude injection

That keeps the compiler model coherent while giving `rock` a clean place to implement crate-based incremental compilation.

## Remaining Roadmap

The current implementation is usable, but it still relies on serialized dependency source bundles.

The remaining work is:

### Phase 9: `rock run`

Implement `rock run` on top of the existing artifact-aware build path.

- Build the current crate first, exactly like `rock build`.
- Execute the produced binary after a successful build.
- Support argument forwarding after `--`.
- Propagate the child process exit status.

Validation:

- running a crate with no extra args
- running a crate with forwarded args
- build failure stops before execution
- child exit code is preserved

### Phase 10: dependency object reuse

Status: completed

Make artifacts carry real compiled outputs that `rock` can link directly.

- Compile dependency crates to object files as part of artifact production.
- Persist the object file path in `object_output`.
- Rebuild the artifact when the object file is missing or stale.
- Link dependency object files during the root crate build instead of recompiling dependency code implicitly.

Validation:

- transitive dependency graph links successfully from cached objects
- deleting a cached object triggers rebuild
- changing a dependency rebuilds its object and relinks the root crate

### Phase 11: define the cross-crate generic boundary

Status: completed

Before removing source bundles, decide how exported generic functions and default trait methods cross crate boundaries.

Current constraint:

- the compiler still lowers and monomorphizes dependency bodies in the consuming crate
- this is why the current artifact includes a serialized source bundle

Required design decision:

- either move generic instantiation ownership to dependency compilation
- or serialize a typed crate-boundary IR for the exported generic/default bodies that consumers still need

This phase is the real blocker for removing dependency source from artifacts.

Completed shape:

- artifacts now carry a typed cross-crate HIR bundle for generic functions, traits with default methods, and generic impls
- artifact-backed crates use that bundle for generic/default lowering and monomorphization
- when a dependency object file is present, consumers no longer need raw dependency AST bodies for those generic/default cases
- when no dependency object file is present yet, consumers still fall back to the serialized source bundle for concrete dependency bodies only

Validation:

- cross-crate generic functions still compile
- cross-crate trait default methods still compile
- generic/default behavior still works for file-backed modules like `stdlib::show`
- monomorphization no longer depends on raw dependency AST loading for generic/default bodies when typed artifact data is available

### Phase 12: interface-only artifact payload

Status: completed

Replace the default dependency payload with a smaller typed interface.

- serialize exported declarations and signatures
- serialize traits, impl-visible method signatures, operator precedence, module/export indexes, and stdlib prelude export metadata
- load dependency interfaces without AST/file caches for normal builds
- reserve any richer body payload only for the generic-boundary solution from Phase 11

Completed shape:

- artifacts now serialize exported declaration metadata directly in `interface`
- artifact loads reconstruct dependency manifests from artifact metadata instead of requiring a source bundle
- object-backed or source-free artifact dependencies register declarations and answer glob-import/export lookups from typed interface data and `module_index`
- the serialized source bundle remains only as a fallback for artifacts with no dependency object file, where concrete dependency bodies still need AST lowering

Validation:

- root crate resolution and type checking succeed from interface-only artifacts
- normal dependency builds no longer require dependency source trees at compile time
- stdlib prelude injection still works from interface-only stdlib artifacts

### Phase 13: remove the serialized source bundle from the normal path

Status: completed

Once Phases 10 through 12 are complete:

- stop emitting dependency source bundles in default artifacts
- make `rock build` use source-free dependency artifacts by default
- keep the low-level `rockc` artifact input explicit
- do not add compatibility shims for the old source-bundle artifact format; bump the artifact version and rebuild

Completed shape:

- object-backed artifacts no longer serialize a source bundle by default
- `rock build` and `rock artifact` now produce source-free cached dependency artifacts
- the artifact format version was bumped again so cached artifacts rebuild cleanly
- `rock` cache reuse accepts missing dependency source files when a valid cached artifact and object file already exist
- explicit non-object artifacts may still retain a source bundle for the old fallback path, but that is no longer the default workflow

Validation:

- a workspace can build from cached artifacts without dependency source reparsing
- deleting dependency source while keeping valid artifacts still allows downstream compilation where expected

## Recommended Execution Order

1. Implement `rock run`.
2. Add dependency object reuse.
3. Define the cross-crate generic/default-method boundary.
4. Introduce the interface-only artifact payload.
5. Remove serialized source bundles from the default path.
