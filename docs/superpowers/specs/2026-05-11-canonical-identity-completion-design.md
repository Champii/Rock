# Canonical Identity Completion Design

## Goal

Finish the canonical identity migration so compiler semantics use stable typed IDs instead of string names. Names remain valid source syntax, diagnostics, import/export metadata, and backend symbol input, but semantic identity must flow through canonical IDs once names are resolved.

This is one roadmap implemented as several independently testable slices. Each slice must leave the compiler building and the relevant regression tests passing.

## Current State

The compiler already has the core identity spine:

- `DefId { crate_id: CrateId, local: LocalDefId }` is the canonical definition identity type.
- Collection allocates canonical `ModuleId` and `DefId` values for current-crate items.
- Resolver tables map canonical module/item paths and aliases to IDs, with reverse lookup tables for diagnostics and display names.
- Core HIR declarations store `DefId` values.
- HIR has ID-keyed indexes for top-level definitions, impls, externs, trait defaults, and impl methods.
- The supported `collect -> lower_from_declarations -> mono` named-item path no longer creates fallback `DefId`s when canonical identity is missing.
- Product artifacts and provider APIs now carry downstream dependency interface, body, and link data without source-backed downstream compilation.

The remaining issue is not lack of IDs; it is that several consumers still treat names as semantic keys, and loaded product artifacts can still reuse producer-local crate IDs without a downstream remap.

## Definition Of Complete

Canonical identity is complete when:

- Two definitions with the same textual name or same producer-local ID cannot collide across modules, crates, artifacts, prelude exports, methods, trait defaults, or monomorphized instances.
- Lowering, inference, monomorphization, and codegen choose semantic entities by ID, not by display name, backend symbol, or map key string.
- Legacy string maps are either removed or explicitly reduced to source-level lookup, diagnostics, import/export syntax, or backend symbol generation.
- Product artifact loading remaps producer-local identities into the consumer's dependency identity space before exposing metadata, bodies, resolver tables, or link records.
- Type and trait projection identity references canonical definition IDs rather than carrying semantic type or trait names as strings.

## Non-Goals

- Do not revive source-backed downstream dependency compilation.
- Do not add compiler-owned stdlib discovery, implicit stdlib loading, or unqualified stdlib injection beyond the existing explicit-artifact prelude path.
- Do not redesign parser IO, macro expansion, formatter trivia, MIR-backed codegen, or borrowck dataflow as part of this work.
- Do not split `LoadedCrate` further unless a specific identity slice needs a narrower provider interface.
- Do not preserve deprecated fallback behavior once a domain is migrated to canonical IDs.

## Invariants

- `collect` owns source-level canonical table construction for the current crate.
- Product artifact loading owns dependency artifact ID remapping before downstream compiler phases observe dependency data; artifact-encoded crate numbers are artifact-local, not global identities.
- `lower` consumes canonical identity; it does not allocate IDs for missing named items.
- `mono` specializes by canonical semantic origin plus substitution, not by a function or method name string.
- Backend symbols are generated from canonical identity and display metadata, but backend symbols are never semantic identity.
- Names may be duplicate aliases for one `DefId`; aliases do not create cloned definitions.
- Missing canonical identity in a migrated domain is an error or panic in tests, not a reason to create a new ID.

## Phased Architecture

### Phase 1: Product Artifact CrateId Remapping

Problem: independently built product artifacts commonly serialize their local crate as `CrateId(0)`. Loading two such artifacts into one compiler invocation can expose identical `DefId` values for unrelated dependency definitions. A naive fresh-`CrateId`-per-artifact fix is also wrong because shared transitive dependencies would get split into different identities when multiple artifacts reference the same dependency.

Design:

- Rust precedent: rustc treats encoded crate numbers as metadata-local. When metadata is decoded, rustc maps each external crate identity to a session-local `CrateNum`; cross-crate `DefId` values are decoded through that map instead of trusting serialized crate numbers as global IDs.
- Treat `ProductCrateId` values in artifacts like rustc treats encoded crate numbers: they are local to the producer artifact and must be decoded through the consumer session's crate map.
- Define a stable `ProductCrateIdentity` key for crate identity, including at least crate name, version, target triple, and artifact format version. When product/source hashes or explicit build disambiguators are added to artifacts, they become part of the identity key before two artifacts can be treated as the same crate instance.
- Maintain a consumer-session crate identity table that maps each `ProductCrateIdentity` to one `CrateId`. Reuse the same `CrateId` when multiple artifacts reference the same crate identity; allocate a distinct `CrateId` for distinct identities.
- Build a crate-ID remap for each loaded artifact from producer-local `ProductCrateId` values to consumer-session `CrateId` values. The artifact's local crate maps through `products.crate_identity`; referenced dependency crate IDs map through artifact dependency identity metadata.
- Build a `ProductIdentityRemap` from every producer `ProductDefId` to a consumer `DefId` by applying the crate-ID remap and preserving the producer-local definition index inside the remapped crate.
- Apply the remap consistently to artifact interface metadata, resolver tables, prelude exports, cross-crate HIR bodies, impl owners, trait default methods, extern metadata, and product link records.
- Keep producer-local IDs private to artifact deserialization after remapping. Downstream compiler phases only see consumer-session `DefId`s.
- Treat unresolved product IDs or product crate IDs referenced by metadata, bodies, resolver tables, or link data as artifact-load errors.
- Do not rewrite backend object symbols as part of ID remapping. Link symbols are ABI/backend names, not semantic identity.

Primary tests:

- Loading two different product artifacts with the same producer-local `ProductDefId` gives distinct dependency `DefId.crate_id` values.
- Loading two artifacts that reference the same transitive `ProductCrateIdentity` maps that shared dependency to the same consumer-session `CrateId`.
- Loading two artifacts with the same crate name but different version, target, artifact format version, product/source hash, or explicit build disambiguator maps them to distinct consumer-session `CrateId`s.
- Resolver aliases and reverse names point to the remapped IDs.
- Generic cross-crate HIR bodies and object link records use the same remapped IDs as the interface.
- Monomorphized instance keys for same-local-ID dependency functions do not collide.

### Phase 2: Canonical Dependency, Prelude, And Artifact Resolution

Problem: dependency crate registration, stdlib prelude injection, and artifact-backed exports still use string maps as active semantic lookup paths in some places.

Design:

- Route dependency, prelude, and artifact item lookup through resolver/provider metadata that maps names and aliases to canonical IDs.
- Preserve string names as syntax and display metadata, but require a resolver-backed `DefId` before inserting semantic HIR declarations for migrated item kinds.
- Make stdlib prelude aliases resolve to the canonical `DefId` of their exported source item.
- Make artifact root exports resolve to the same canonical IDs as their exported definitions.
- Keep legacy string maps only for unmigrated local scopes and temporary body-lowering lookup, with explicit comments and tests documenting the boundary.

Primary tests:

- Collection records dependency functions, stdlib prelude aliases, and artifact exports in canonical resolver tables.
- Lowering imported/prelude names produces HIR declarations whose IDs match the dependency resolver IDs.
- Missing dependency/prelude canonical identity fails instead of silently falling back to a string map.

### Phase 3: Authoritative ID-Keyed HIR Consumers

Problem: `HirProgram` still owns many semantic entities through `HashMap<String, _>`. ID-keyed indexes exist, but consumers often still choose entities by string keys.

Design:

- Identify consumers that already have or can cheaply obtain `DefId` and switch them to ID-index lookups.
- Make string-keyed maps compatibility/storage views while canonical indexes become the authoritative lookup path for migrated consumers.
- Thread canonical IDs through lower, infer, mono, and codegen calls that currently pass names as semantic handles.
- Retain display-name lookup only for diagnostics, debug printing, and generated symbol text.

Primary tests:

- Aliased names that share one `DefId` resolve to one semantic HIR entity.
- Duplicate display names across dependencies do not cause HIR lookup collisions.
- Existing current-crate, dependency-artifact, and prelude programs still compile through the ID-index lookup path.

### Phase 4: First-Class Child Definition Identity

Problem: methods, trait defaults, associated items, fields, and variants are not all represented as first-class canonical semantic identities. Some method paths are represented as strings inside monomorphization instance identity, and field/variant lookup still depends on owner-local strings after the owner is selected.

Design:

- Assign canonical `DefId`s to methods, trait default methods, impl methods, and associated items because they participate in cross-phase lookup and specialization.
- Use canonical owner IDs plus method/associated-item IDs, or direct child `DefId`s, for method identity; do not use bare method names as semantic identity.
- Assign owner-scoped `FieldId` and `VariantId` values for struct fields and enum variants, keyed by the already selected owner `DefId`.
- Use field and variant names only for source lookup and diagnostics; after lookup, lower/typecheck/codegen should carry the owner ID plus `FieldId` or `VariantId`.
- Record child identity tables in HIR so future work does not reintroduce string-only identity accidentally.

Primary tests:

- Two impls with the same method name for different owners produce distinct canonical method identities.
- Trait default and impl override methods can be selected and specialized by canonical identity.
- Field and variant lookup produces owner ID plus `FieldId` or `VariantId`, and same-named fields or variants under different owners do not collide.

### Phase 5: Monomorphization Instance Identity Cleanup

Problem: `InstanceOrigin::ImplMethod` still contains `method: String`, and not all specialization entry points are guaranteed to key only by canonical semantic identity plus substitution.

Design:

- Replace method-name string identity in `InstanceOrigin::ImplMethod` with canonical method or associated-item identity.
- Ensure all function and method specializations intern through `InstanceKey { origin, substitution }`, where `origin` is fully canonical.
- Keep backend symbols in `InstanceRecord`, separate from `InstanceKey`.
- Ensure object-backed dependency declarations and cross-crate generic bodies share the same canonical origins.

Primary tests:

- Two methods with the same name and substitution but different owners do not collide.
- Object-backed and generic artifact methods intern with the same canonical origin shape as current-crate methods.
- Backend symbol changes do not change instance identity.

### Phase 6: Semantic Type Identity

Problem: the current semantic `Type` representation still carries type, trait, and projection names as strings in several places. That means definition identity can be canonical while type identity remains name-bearing.

Design:

- Introduce or complete a semantic `Ty` representation where nominal types, traits, impl owners, associated types, and projections refer to canonical IDs.
- Make type lowering resolve source names once, producing ID-backed semantic types.
- Move inference, trait selection, monomorphization, and codegen consumers from string-bearing `Type` identity to ID-backed semantic type identity.
- Keep source names attached only as display/debug metadata where diagnostics need them.

Primary tests:

- Two dependency crates exporting same-named types produce distinct semantic types.
- Trait bounds and projections resolve by canonical trait/type IDs.
- Codegen uses backend symbols derived from canonical IDs without using type names as semantic keys.

## Migration Order

The implementation order is fixed:

1. Product artifact `CrateId` remapping.
2. Canonical dependency, prelude, and artifact resolution.
3. Authoritative ID-keyed HIR consumers.
4. First-class child definition identity.
5. Monomorphization instance identity cleanup.
6. Semantic type identity.

This order keeps dependency identity sound before migrating more consumers to ID-keyed lookup. It also keeps semantic type migration last because type identity depends on stable definition, trait, method, and artifact identities.

## Compatibility And Cleanup

- Compatibility means existing supported programs keep compiling through the new canonical path.
- It does not mean preserving old fallback allocation, source-backed downstream compilation, or string-based semantic lookup for migrated domains.
- String maps can remain temporarily as storage or display views only when the corresponding ID-keyed path is tested as authoritative.
- After each phase, update `docs/superpowers/plans/master-audit-checklist.md` to mark completed audit items and record any deliberately deferred identity boundary.

## Verification Strategy

Each phase needs focused unit tests for the exact identity invariant it changes, then the smallest relevant integration or artifact regression. The final completion pass must run:

- `cargo fmt --all --check`
- `cargo test -p rock-lib`
- artifact-backed dependency regressions for product artifacts, stdlib prelude, cross-crate generic bodies, trait defaults, and object linking

The completion claim must cite those fresh verification results.

## Risks

- Artifact remapping can miss nested IDs inside HIR bodies or impl/method metadata. The remap code should be centralized and tests should include nested generic bodies and methods.
- Moving HIR consumers to ID indexes can expose places where names are still the only available handle. Those cases should either thread `DefId` from the caller or remain explicitly out of scope for that phase.
- Semantic type migration is larger than the earlier slices. It should start only after definition and instance identity are stable.
- Broad lowerer decomposition can distract from identity completion. Refactor only the pieces required to remove identity leakage.

## Success Criteria

The roadmap is complete when the master audit checklist can mark these items done:

- Current-crate, dependency, and prelude resolution flow through canonical IDs.
- HIR consumers no longer use strings as primary semantic identity for migrated definitions.
- Methods and associated items used across phases have canonical identity.
- Monomorphization instance identity contains no method-name string semantic key.
- Type lowering and semantic type consumers reference canonical IDs for nominal types, traits, and projections.
- Product artifact dependencies cannot collide through producer-local crate IDs.
