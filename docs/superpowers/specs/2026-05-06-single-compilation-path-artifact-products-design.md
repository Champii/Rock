# Single Compilation Path Artifact Products Design

**Date:** 2026-05-06
**Status:** Draft for review
**Scope:** Replace the current artifact-specific semantic pipeline with artifacts produced from the normal crate compilation path. `rock` remains an orchestration tool and `rockc` becomes the only compiler/artifact producer.

## Purpose

The compiler currently has two semantic paths for crate information:

- the normal path for the current source crate, driven by `rock_lib::compile`
- artifact-specific collection/lowering paths in `lib/src/crate_artifact`, `lib/src/lower/crates`, and `lib/src/mono/external`

The second path is redundant and behaves differently from normal compilation. It is the source of artifact-HIR parity bugs, source-bundled artifact workarounds, duplicated lowering logic, and fragile monomorphization behavior.

The goal is to make artifact production a serialization step over compiler products from the normal pipeline, not a separate compilation path.

## Goals

- Compile exactly one source crate per `rockc` invocation.
- Consume every direct dependency as a prebuilt artifact, including dependencies of dependencies.
- Produce artifacts from the same compiler products used by normal compilation.
- Remove AST and source module caches from normal artifacts.
- Make artifacts contain metadata, downstream body data, and link data rather than source syntax.
- Make `rock` Cargo-like: graph resolution, freshness, build order, command invocation, and no in-process compiler ownership.
- Keep `rock_lib` as the compiler implementation used by `rockc`, not as a direct package-manager build API.
- Keep source loading only for the crate currently being compiled and for transitional artifact-production internals until replaced.

## Non-Goals

- Do not introduce a rustc-like query system.
- Do not redesign the whole compiler around demand-driven queries.
- Do not require `rock` to understand compiler internals.
- Do not keep ASTs in `.rkca` artifacts as a long-term dependency mechanism.
- Do not preserve source-bundled artifacts as a normal downstream compilation path.
- Do not remove source parsing for the crate currently being compiled.
- Do not solve all DefId/type-identity problems in this spec, though the artifact model should make room for stable IDs.

## Rustc Inspiration Without Queries

The useful rustc model here is the crate boundary, not the query engine.

Rustc compiles one crate at a time. A downstream crate receives dependency information through crate metadata and link artifacts. It does not parse dependency source or run a separate semantic lowering path for dependencies. Metadata contains enough exported semantic information for name resolution, type checking, trait lookup, diagnostics, and downstream monomorphization when generic bodies are needed. Object code or library archives provide linkable codegen products.

Rock should follow the same high-level split:

- source belongs to the current crate compilation
- dependency information arrives as artifact metadata
- generic/default bodies needed downstream are encoded as compiler body products
- object outputs are explicit link products

This does not require queries. It requires a clear compiler-products boundary.

## Target Build Flow

### `rock`

`rock` is the package manager and build orchestrator.

It should:

- resolve the full package graph
- decide build order topologically
- decide whether artifacts are fresh
- ensure the bundled stdlib artifact exists when a crate requires stdlib
- invoke `rockc` once per crate that must be rebuilt
- pass direct dependency artifacts to each `rockc` invocation

`rock` should not call `rock_lib::compile` directly in the long-term design.

### `rockc`

`rockc` is the compiler driver.

For each crate `C`, `rock` invokes `rockc` with the source for `C` and artifacts for all direct dependencies of `C`:

```text
rockc \
  --crate-name C \
  --entry-file path/to/C/lib-or-main.rk \
  --extern-artifact dep1=path/to/dep1.rkca \
  --extern-artifact dep2=path/to/dep2.rkca \
  --emit-artifact path/to/C.rkca \
  --emit-object path/to/C.o
```

The same rule applies to root crates and dependency crates. Only `C` is compiled from source. All direct dependencies of `C` are consumed as artifacts.

## Target Compiler Pipeline

The canonical compiler path for a source crate is:

```text
load dependency artifacts
parse current crate source
expand macros
collect declarations
lower to HIR
infer and solve types/traits
build MIR
borrow check
monomorphize
codegen
emit compiler products
emit requested object/artifact/executable outputs
```

Artifact production uses the products of this path. It does not re-run a special artifact collection/lowering path after the fact.

## Compiler Products

Introduce an internal `CompilerProducts` model produced by the normal pipeline.

At first, this can be a concrete struct rather than a fully abstract database:

```rust
pub struct CompilerProducts {
    pub crate_identity: ProductCrateIdentity,
    pub identity_table: ProductIdentityTable,
    pub metadata: ProductMetadata,
    pub body_index: ProductBodies,
    pub link: ProductLinkData,
    pub dependencies: Vec<ProductDependencyIdentity>,
    pub source_fingerprint: ProductSourceFingerprint,
}
```

This type should be owned by the compiler pipeline, not by `rock`.

### Canonical Product IDs

Product-backed artifacts must introduce canonical product IDs immediately. String names may remain as lookup, alias, diagnostic, and backend-symbol metadata during migration, but serialized semantic products must not use display strings as their primary identity.

At first, the product ID model can mirror the existing compiler ID shape:

```rust
pub struct ProductCrateId(u32);

pub struct ProductLocalDefId(u32);

pub struct ProductDefId {
    pub crate_id: ProductCrateId,
    pub local_id: ProductLocalDefId,
}

pub struct ProductIdentityTable {
    pub local_crate: ProductCrateId,
    pub dependencies: Vec<ProductCrateIdentity>,
    pub display_names: BTreeMap<ProductDefId, String>,
    pub export_names: BTreeMap<String, ProductDefId>,
    pub backend_symbols: BTreeMap<ProductDefId, String>,
}
```

The normal compilation pipeline assigns these IDs. Artifact production serializes them from the normal compiler state instead of asking the old artifact builder to invent matching IDs. This avoids requiring the current source and artifact semantic paths to agree on stable IDs before the dual pipeline is removed.

The migration rule is:

- canonical `ProductDefId` keys identify serialized metadata, bodies, impl owners, trait references, and link records
- string paths identify imports, exports, source names, diagnostics, prelude aliases, and backend symbols
- transitional string-keyed internal maps are acceptable only behind product extraction/loading adapters
- new artifact sections must include a `ProductDefId` owner for semantic records; any compatibility string index must be derived secondary data

### Crate Identity

Crate identity should include:

- crate name
- crate version
- target triple
- compiler version or artifact format version
- stable package/source identity when available
- current crate object/module naming information needed by codegen and link steps

The model should eventually support multiple versions of the same crate, even if current code still keys many maps by crate name.

### Metadata Section

Metadata is the downstream semantic interface.

It should include:

- exported functions, structs, enums, traits, impls, externs, and infix operators
- item signatures and visibility
- trait impl headers and associated type definitions
- prelude exports as aliases to canonical definitions
- module export summaries
- resolver/import/export alias tables needed by downstream resolution
- enough source-name information for diagnostics

Metadata should not include ASTs.

Metadata entries should be keyed by `ProductDefId`. Export tables map user-facing names to `ProductDefId`; they are not the canonical identity of the exported item.

### Body Section

The body section contains only bodies that downstream crates may need to instantiate or reason about.

It should include:

- generic function bodies
- generic impl method bodies
- trait default method bodies
- any body explicitly marked as needed for downstream monomorphization or inlining
- body identity keyed by `ProductDefId`, not by display strings

It should not contain all source bodies by default. Non-generic, object-backed bodies are provided by the link section unless a later optimization explicitly chooses to serialize them.

### Link Section

The link section contains information needed to link compiled code:

- object file path or artifact-relative object reference
- exported backend symbols
- object-backed crate marker
- later: required native libraries or linker arguments if the language grows that feature

### Dependency Identities

Each artifact should record the exact dependency artifacts used to compile it:

- dependency crate name
- resolved artifact path or stable artifact identity
- dependency version/source identity
- artifact format version or source hash when available
- target triple compatibility information

This makes freshness and compatibility decisions a package-manager concern while still allowing `rockc` to reject incompatible artifacts.

## Artifact Format

Long-term `.rkca` contents should be:

```text
CrateArtifact
  format_version
  crate_identity
  identity_table
  dependency_identities
  metadata
  bodies
  link
  source_fingerprint
```

The following current fields should become transitional and then disappear:

- `source_bundle`
- artifact root AST
- artifact file-cache ASTs
- artifact mode driven by presence of source bundle
- separate `interface` and `cross_crate_hir` products built by separate lowerer runs

The current `interface` and `cross_crate_hir` concepts should be folded into `metadata` and `bodies` respectively, but produced from the normal pipeline rather than rebuilt by `ArtifactBuilder`.

The new artifact format should tolerate transitional string indexes only as secondary indexes. If a loader needs a string-keyed map to satisfy existing compiler phases, it should derive that map from `ProductDefId`-keyed metadata at load time or through a compatibility adapter.

## Artifact Consumption

Loaded artifacts should expose a compiler-facing dependency interface with three capability groups:

- metadata provider
- body provider
- link provider

Compiler phases should query those capabilities through `LoadedCrate` or a replacement dependency interface. They should not branch on storage mode such as source-backed, object-backed, source-bundled, or interface-only except at the artifact loading boundary.

Downstream compilation should not parse dependency source or inspect dependency ASTs. If a needed generic/default body is missing from the body section, that is an artifact-production bug or an explicit unsupported artifact mode, not a reason to fall back to source.

## Production Timing

Artifacts are emitted by `rockc` from one normal compilation run.

The minimal order is:

1. Compile current crate through inference and HIR finalization.
2. Build metadata and body products from finalized compiler state.
3. Run MIR/borrow checking and monomorphization.
4. Run codegen when object or executable output is requested.
5. Attach link data for object outputs.
6. Serialize `.rkca` if artifact output is requested.

This order can be refined later. The important invariant is that artifact products come from the same semantic state as normal compilation, not from a second lowerer/collector path.

## Migration Strategy

### Phase 1: Product Model Beside Existing Artifact Builder

Add `CompilerProducts` and produce it from the normal `compile` path. Initially this can be internal and used by tests only.

Validation:

- compare current artifact metadata with products from normal compilation for small crates
- ensure generic/default bodies selected for products match existing cross-crate HIR targets

### Phase 2: `rockc` Artifact Emission

Add explicit artifact/object emission to `rockc` using `CompilerProducts`.

Validation:

- `rockc` can compile one crate from source and emit `.rkca` plus object
- emitted artifacts contain no AST/source bundle for normal builds

### Phase 3: `rock` Subprocess Driver

Move `rock` from in-process `rock_lib::compile` calls to `rockc` subprocess invocations.

Validation:

- dependencies are built in topological order
- every crate consumes direct dependencies via `--extern-artifact`
- stdlib is passed explicitly as an artifact when required
- `cargo test -p rock` passes

### Phase 4: Replace Artifact Builder Re-Lowering

Retire `CrateContext::build_artifact` paths that re-run collection/lowering for artifact-specific products.

Validation:

- remove `artifact.cross_crate_hir = None` workaround in integration tests
- broad stdlib-backed integration tests pass using product-backed artifacts
- artifact generic/default tests pass without source bundles

### Phase 5: Remove Source-Bundled Downstream Fallbacks

Once product-backed artifacts are sufficient, remove source-bundled dependency fallback logic from downstream compilation.

Validation:

- no downstream phase reads dependency ASTs or file caches
- source bundles are absent from normal artifacts
- source loading remains only for current crate compilation and temporary artifact-production tests if any remain

## Testing Strategy

Useful test categories:

- unit tests for `CompilerProducts` extraction from normal HIR/inference state
- unit tests proving exported names and backend symbols resolve to stable `ProductDefId` records inside one artifact
- artifact roundtrip tests proving metadata/body/link sections survive serialization
- artifact roundtrip tests proving body and metadata identity is preserved by `ProductDefId`, not reconstructed from strings
- `rockc` tests for emitting artifact/object outputs from one source crate
- dependency graph tests where dependency crates also consume artifacts
- stdlib artifact tests without AST/source bundles
- broad integration suite with `artifact.cross_crate_hir = None` removed
- negative tests where missing body products fail clearly instead of silently falling back to source

## Risks

- Current semantic identity is still partly string-based internally. The mitigation is to add canonical `ProductDefId` at the product/artifact boundary immediately while leaving string-keyed compiler maps as transitional internals.
- A full compiler-wide canonical-ID migration before product artifacts would be risky because the current source and artifact semantic paths would both need to produce identical IDs. The design avoids that by assigning product IDs from the normal pipeline only.
- The normal pipeline may not currently retain every fact needed by artifact products after inference/mono; product extraction may expose missing ownership boundaries.
- Moving `rock` to subprocess `rockc` calls adds process/error propagation work.
- Removing source bundles too early would make debugging harder and may block migration. They should remain available only as a transitional or debug-only format until product artifacts are stable.

## Success Criteria

The design is successful when:

- `rockc` is the only producer of normal `.rkca` artifacts.
- `rock` invokes `rockc` for each crate and does not call `rock_lib::compile` directly.
- Each crate compilation consumes direct dependencies only as artifacts.
- Normal artifacts do not contain ASTs or source file caches.
- Artifact metadata/body/link sections are produced from the normal compiler path.
- Artifact metadata/body/link sections use canonical product IDs as primary identity, with strings only as secondary lookup/display/symbol metadata.
- Downstream phases do not branch on dependency storage mode for semantic behavior.
- The broad integration suite passes without clearing `cross_crate_hir` or using source-bundled stdlib fallback.
