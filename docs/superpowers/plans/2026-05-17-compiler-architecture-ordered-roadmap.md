# Compiler Architecture Ordered Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the compiler architecture audit into a single dependency-ordered task queue for the remaining work, rebaselined after the authoritative HIR ID-keyed storage slice.

**Architecture:** Treat current-crate ID authority, ID-owned HIR storage, and the first high-value HIR resolved references as baseline. Next, finish resolver alias and phase-boundary cleanup, then move semantic services out of backend phases, then make MIR/codegen/borrowck backend boundaries clean, and leave parser/macro/formatter cleanup until the identity-sensitive compiler core is stable. This order preserves the completed canonical identity work and avoids reopening Phase 6 semantic type identity unless a later task explicitly replaces the structural `Type` model with a type context.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId` indexes, `Type`, future `Ty`/type context, `InstanceRegistry`, MIR, borrowck dataflow, product artifacts, parser/macro/fmt modules, focused Cargo tests, `cargo fmt --all --check`, `cargo test -p rock-lib`, `cargo test -p rock`.

---

## Inputs Read

Source audit:
- `docs/superpowers/specs/2026-04-24-compiler-architecture-audit-design.md`
- `docs/superpowers/plans/master-audit-checklist.md`

Post-audit identity and collection work:
- `docs/superpowers/specs/2026-04-25-multi-module-item-index-design.md`
- `docs/superpowers/specs/2026-04-27-independent-local-collection-design.md`
- `docs/superpowers/specs/2026-04-27-collect-owned-context-design.md`
- `docs/superpowers/specs/2026-04-27-collect-owned-header-builders-design.md`
- `docs/superpowers/specs/2026-04-28-collect-bootstrap-split-design.md`
- `docs/superpowers/specs/2026-04-28-artifact-interface-collect-split-design.md`
- `docs/superpowers/specs/2026-04-28-lower-bootstrap-helper-cleanup-design.md`
- `docs/superpowers/specs/2026-04-29-canonical-definition-cross-crate-identity-design.md`
- `docs/superpowers/specs/2026-05-10-hir-id-keyed-definition-indexes-design.md`
- `docs/superpowers/specs/2026-05-11-canonical-identity-completion-design.md`
- `docs/superpowers/specs/2026-05-12-canonical-dependency-prelude-artifact-resolution-design.md`
- `docs/superpowers/specs/2026-05-16-phase-6-task-6-stale-identity-audit-design.md`
- `docs/superpowers/specs/2026-05-17-authoritative-hir-id-keyed-storage-design.md`
- `docs/superpowers/plans/2026-04-27-collect-owned-context.md`
- `docs/superpowers/plans/2026-04-27-collect-owned-header-builders.md`
- `docs/superpowers/plans/2026-04-28-collect-bootstrap-split.md`
- `docs/superpowers/plans/2026-04-28-artifact-interface-collect-split.md`
- `docs/superpowers/plans/2026-04-28-lower-bootstrap-helper-cleanup.md`
- `docs/superpowers/plans/2026-04-29-canonical-definition-cross-crate-identity.md`
- `docs/superpowers/plans/2026-05-02-hir-identity-defids.md`
- `docs/superpowers/plans/2026-05-04-canonical-defid-hardening.md`
- `docs/superpowers/plans/2026-05-09-adapted-canonical-identity-hardening.md`
- `docs/superpowers/plans/2026-05-10-hir-id-keyed-definition-indexes.md`
- `docs/superpowers/plans/2026-05-12-first-class-child-definition-identity.md`
- `docs/superpowers/plans/2026-05-13-semantic-type-identity.md`
- `docs/superpowers/plans/2026-05-15-trait-projection-identity-completion.md`
- `docs/superpowers/plans/2026-05-15-trait-projection-identity-completion-checklist.md`
- `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit.md`
- `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`
- `docs/superpowers/plans/2026-05-17-authoritative-hir-id-keyed-storage.md`

Post-audit artifact and dependency work:
- `docs/superpowers/specs/2026-05-04-artifact-only-external-dependency-boundary-design.md`
- `docs/superpowers/specs/2026-05-06-single-compilation-path-artifact-products-design.md`
- `docs/superpowers/specs/2026-05-07-extern-product-artifact-loading-design.md`
- `docs/superpowers/specs/2026-05-07-rock-product-artifact-subprocess-driver-design.md`
- `docs/superpowers/specs/2026-05-07-rock-subprocess-driver-design.md`
- `docs/superpowers/specs/2026-05-08-old-crate-artifact-deletion-design.md`
- `docs/superpowers/specs/2026-05-08-stdlib-product-artifact-migration-design.md`
- `docs/superpowers/specs/2026-05-11-artifact-only-downstream-dependencies-design.md`
- `docs/superpowers/specs/2026-05-11-dependency-provider-boundaries-design.md`
- `docs/superpowers/plans/2026-05-01-rockc-cli-dependency-surface-cleanup.md`
- `docs/superpowers/plans/2026-05-06-compiler-products-identity-spine.md`
- `docs/superpowers/plans/2026-05-07-extern-product-artifact-loading.md`
- `docs/superpowers/plans/2026-05-07-rock-product-artifact-subprocess-driver.md`
- `docs/superpowers/plans/2026-05-07-rockc-product-artifact-emission.md`
- `docs/superpowers/plans/2026-05-08-old-crate-artifact-deletion.md`
- `docs/superpowers/plans/2026-05-08-stdlib-product-artifact-migration.md`
- `docs/superpowers/plans/2026-05-11-artifact-only-downstream-dependencies.md`
- `docs/superpowers/plans/2026-05-11-dependency-provider-boundaries.md`
- `docs/superpowers/plans/2026-05-11-product-artifact-crate-id-remapping.md`

Post-audit backend, language, and cleanup work:
- `docs/superpowers/specs/2026-04-29-monomorphization-instance-registry-design.md`
- `docs/superpowers/specs/2026-04-30-synthetic-array-owner-removal-design.md`
- `docs/superpowers/specs/2026-05-09-borrowed-slices-and-str-design.md`
- `docs/superpowers/specs/2026-05-09-user-facing-array-name-removal-design.md`
- `docs/superpowers/plans/2026-04-30-synthetic-array-owner-removal.md`
- `docs/superpowers/plans/2026-05-01-monomorphization-instance-registry.md`
- `docs/superpowers/plans/2026-05-09-borrowed-slices-and-str.md`
- `docs/superpowers/plans/2026-05-09-user-facing-array-name-removal.md`
- `docs/superpowers/plans/2026-05-12-monomorphization-instance-identity-cleanup.md`

Related older docs consulted where they define active borrowck/operator context:
- `docs/superpowers/specs/2026-04-13-borrow-checker-remaining-work-design.md`
- `docs/superpowers/specs/2026-04-19-assoc-types-ops-recovery-design.md`
- `docs/superpowers/specs/2026-04-20-builtin-index-design.md`
- `docs/superpowers/specs/2026-04-20-stdlib-deref-vec-index-design.md`
- `docs/superpowers/plans/2026-04-10-mir-reference-lifetime-cleanup.md`
- `docs/superpowers/plans/2026-04-13-borrow-checker-remaining-work.md`
- `docs/superpowers/plans/2026-04-19-assoc-types-ops-recovery.md`
- `docs/superpowers/plans/2026-04-20-builtin-index-implementation.md`
- `docs/superpowers/plans/2026-04-21-stdlib-deref-vec-index.md`

---

## Ordering Principles

1. **Identity before movement:** do not move large phase boundaries while semantic identity is still partly name-backed or table ownership is unclear.
2. **Current crate before dependencies:** make current-crate IDs, HIR storage, and aliases authoritative before hardening dependency/provider behavior around the same abstractions.
3. **Selection before mono/codegen cleanup:** codegen and mono cannot stop reconstructing semantics until a shared selection result exists.
4. **Instance graph before DCE and MIR codegen:** reachability and backend lowering need explicit callable identity and call edges.
5. **MIR completeness before MIR codegen:** codegen cannot consume MIR until MIR represents runtime-relevant operations, not placeholders.
6. **Frontend cleanup after core identity:** parser/module loader, macro source maps, and formatter/trivia are important, but they do not unblock the identity-sensitive compiler core as directly as HIR/type/selection/mono work.

---

## Completed Baseline

These are not tasks in this roadmap; they are the foundation the roadmap assumes.

- Typed ID scaffolding exists in `lib/src/ids.rs`, including `CrateId`, `ModuleId`, `LocalDefId`, `DefId`, child IDs, `TypeVarId`, `TypeId`, and `InstanceId`.
- Collection builds multi-module item indexes and resolver tables.
- HIR entities carry `DefId`s, and derived ID-keyed indexes exist for many top-level and child definitions.
- Product artifacts replaced the old crate artifact path, and dependency consumption is now product-artifact based.
- Product artifact crate IDs are remapped into the consumer session.
- `Type` semantic identity is ID-backed for nominal types, generic params, type vars, trait bounds, and projections.
- The Phase 6 stale identity audit found and fixed remaining mono/pattern stale identity bugs, then classified all remaining broad search hits as `Allowed` or `Fixed`.
- `InstanceRegistry` exists and carries `InstanceKey`, `InstanceOrigin`, and `InstanceRecord` for specialized and object-backed records.
- MIR is built and borrow checking runs before monomorphization/codegen.
- Collection/index allocation is the only authority for current-crate top-level, impl, method, field, variant, associated-type, extern, trait-member, impl-member, signature, and explicit generated definition IDs before HIR body lowering.
- `HirProgram` owns migrated core definitions by canonical ID, with source names retained as display/diagnostic metadata and compatibility lookup views.
- HIR method locations, struct literals/patterns, enum variants/patterns, direct calls/references, local references, params, lets, pattern bindings, loop variables, and closure captures carry canonical IDs or scoped `HirLocalId`s where lowering resolves them.
- Product artifact loading validates present callable, name, struct owner, field owner, variant, method target, and dependency target sidecars before remapping.
- Artifact declaration collection now builds the current source-crate item index before collecting declarations and consumes indexed `LocalCollector` IDs plus crate-qualified inline-module resolver aliases instead of the legacy provisional-ID context collector.

---

## 2026-05-18 Rebaseline Notes

- Roadmap Task 1 is complete for current-crate ID allocation.
- Roadmap Task 2 is complete for migrated core HIR ownership storage.
- Roadmap Task 3 is complete for HIR semantic references: method locations, struct and enum literals/patterns, direct top-level function/extern references and calls, local references/calls, params, lets, pattern bindings, loop variables, and closure captures now carry canonical IDs or scoped `HirLocalId`s where lowering resolves them. Remaining compatibility strings are display/diagnostic metadata or belong to later alias, type, selection, instance, lowerer, and backend tracks.
- Roadmap Tasks 4-5 have landed for the planned resolver/alias subset: resolver helper APIs, lowerer ID-first alias consumption, and product prelude ID persistence/loading. Later roadmap work continues with type/context, selection, instance, MIR, and backend boundaries.
- Roadmap Tasks 6-7 have landed for the dependency storage boundary: external dependencies now live in an artifact-only extern crate store keyed by session `CrateId`; current source input lives in `CurrentCrateSource`; collect/lower/mono/codegen consume metadata/body/link capabilities without `LoadedCrate` or `ArtifactMode`.
- Roadmap Task 8 has landed for the type-lowering ownership boundary: parsed type syntax conversion now lives behind a shared type-lowering service used by both collection and lowering while retaining the existing structural `Type` representation.
- Roadmap Task 9 has landed for the interned `Ty` / `TypeId` scaffold: `TypeContext` now owns canonical semantic type nodes, while structural `Type` remains the compatibility/serialization boundary until Task 11.
- Roadmap Task 10 has landed for type service extraction: pure facts, display formatting, projection normalization, and layout-facing shape checks now live behind explicit services while structural `Type` remains the compatibility boundary for Task 11.
- Roadmap Task 11 has landed for the full planned `TypeId` phase-boundary migration: resolved HIR owns a `TypeContext` plus explicit HIR type-ID sidecar, mono/MIR/borrowck/codegen consume context-owned `TypeId`s at compiler-owned semantic boundaries, and product artifacts serialize compatibility type payloads through a portable artifact-local type table.
- Roadmap Task 12 has landed for the shared selection-service slice: lowering now routes trait/method/operator/index dispatch through `lib/src/selection/`, selected-target fallback cleanup completed in Task 13, and direct codegen trait/member backend metadata cleanup completed in the 2026-05-31 Task 21 follow-up while broader backend metadata extraction remains future work.
- Roadmap Task 13 has landed for targetless mono/lowering selection fallback cleanup: `HirMethodCallTarget` values are authoritative for semantic method/operator/index/trait dispatch, and targetless dispatch no longer rediscovers generic methods by name or receiver fallback. Direct codegen trait/member metadata cleanup completed in the Task 21 backend metadata cleanup follow-up.
- Roadmap Task 14 has landed for instance-registry callable authority: all emitted callable declarations and bodies now enter codegen through `MonomorphizedProgram.instances`, while backend-symbol call-target cleanup completed in Task 15.
- Roadmap Task 15 has landed for the scoped instance-call-edge slice: monomorphization no longer uses emitted backend symbols to recover callable targets, and generic function/method call edges flow through `InstanceId`-backed HIR/MIR callables.
- Roadmap Task 16 has landed for the scoped instance-reachability DCE slice: the pipeline prunes `MonomorphizedProgram.instances` through explicit `InstanceId`, direct `DefId`, receiver/trait target, and method-target edges without source-name or backend-symbol reachability aliases.
- Roadmap Task 17 has landed for the scoped filesystem-backed source database loader slice: parser file-reading APIs are removed, `SourceDatabase` owns current-crate/source-crate file IO and module graphs, and compile/collect/lower/crate-system/product fingerprint paths consume graph-derived caches while virtual/artifact-backed loader sources remain future work.
- Roadmap Task 18 has landed for the scoped lowerer-decomposition slice: `LoweringPipeline`, `ModuleLoweringContext`, `LowerDiagnostics`, and `BodyLowerer` now own phase ordering, graph/cache-backed module context helpers, diagnostics state, and body traversal entry points while `Lowerer` remains the compatibility state shell for later slices.
- Roadmap Task 19 has landed for the scoped MIR canonical runtime slice: MIR functions, callables, aggregates, enum matches, closures, casts, bounds checks, drops, and agreement checks now carry canonical/runtime forms consumed by the Task 21 MIR backend switch.
- Roadmap Task 20 has landed for the scoped borrowck indexed dataflow slice: borrow creation locations use typed `Location` values, `LoanTable` stores immutable `LoanId`-keyed facts, `LoanState` stores path-sensitive active/owner state with dense sets, indexed place/move-path tables model path-sensitive state, active conflict/provenance checks consume the indexed state while preserving diagnostics, and the later cleanup removed the legacy borrow liveness helper.
- Roadmap Task 21 has landed for the MIR-backed executable-body codegen slice: the active compile path builds monomorphized MIR, runs borrowck and agreement on that MIR, and emits LLVM bodies from MIR statements/terminators without an active HIR body fallback.
- Roadmap Task 22 has landed for the macro expansion architecture slice: macro expansion now uses an explicit context, source-map/origin records, expansion-traced diagnostics, and config-threaded generated parsing without `Config::default()` reparse paths in `lib/src/macro_expansion/**`; parser IO, module-loader, and formatter/trivia cleanup remain separate future work.
- The artifact collection indexed-ID cleanup has landed for the Task 1 follow-up: `collect_artifact_declarations` now preserves standalone function signatures and inline-module glob export aliases with item-index IDs, routes through `LocalCollector`, and no longer runs an artifact-only fresh current-ID repair pass.
- The Task 1 current-crate ID completion cleanup has landed: collection/header provisional IDs, post-header method ID repair, inference method ID repair, trait-default placeholder identity, lowering placeholder identity recovery for binary and unary operators, range `DefId(0, 0)` fallback, public lower-entrypoint legacy collection bypasses, and product-emission acceptance of current-crate sentinel IDs are removed or converted to indexed collection, explicit intrinsic handling, diagnostics, or guardrails.

---

## 2026-05-25 Code-Verified Reconciliation

- 2026-06-05 HIR compatibility-name-map cleanup retired inline semantic reads from migrated mono, MIR, DCE, and codegen consumers by routing display aliases through explicit `HirProgram` helper APIs; remaining string maps are compatibility/display/unresolved-name metadata, not semantic owner authority for those paths.

This table is the authoritative task-level status as checked against the implementation, not just against prior plan completion notes. `Complete for scoped slice` means the focused implementation plan landed, but the broader audit task still has follow-up work listed in the final column.

| Task | Verified status | Code evidence | Remaining work verified in code |
| --- | --- | --- | --- |
| 1. Current-crate ID allocation | Complete | `lib/src/collect/item_index.rs` allocates indexed `DefId`s; `lib/src/collect/mod.rs::collect_artifact_declarations` builds the item index before source-crate declaration collection and uses `LocalCollector` with indexed IDs and crate-qualified inline-module resolver aliases; collection preallocates trait method/signature and impl method IDs before header construction; `lib/src/lower/mod.rs::def_id_for_name` panics instead of fabricating missing IDs; `Lowerer::from_declarations` trusts collected extern IDs instead of repairing `DefId(0, 0)` placeholders; builtin index selection no longer fabricates a sentinel-`DefId` pseudo-function; collection function/member headers receive explicit IDs; `lower_function_sig` no longer mints fallback current-crate IDs for missing signature identity; signature-backed generics are owned/remapped by explicit signature IDs instead of sentinel generic owners; body lowering rejects sentinel generic owners instead of repairing them; inference validates method IDs instead of repairing them; trait defaults use explicit generated current-crate IDs; missing binary and unary operators recover with diagnostics/`Type::Error`; public lower wrappers route through indexed collection; range syntax is treated as an intrinsic non-nominal iterator form instead of fabricating a `Range` `DefId`; product emission preserves real local ID zero and rejects current-crate sentinel IDs before remapping | No remaining Task 1 current-crate ID allocation work; compatibility string references, broader path-resolution extraction, generated metadata modeling, and backend metadata cleanup remain under later tasks |
| 2. HIR ID-keyed storage | Complete for scoped slice | `lib/src/hir/mod.rs` owns core finalized HIR definitions by `DefId` and exposes ID accessors | No remaining finalized-HIR storage work; remaining strings are explicit display/diagnostic/artifact/unresolved-name compatibility metadata or later Task 18/21 state-shell/backend metadata work |
| 3. HIR semantic references | Complete | HIR method locations, struct/enum literals/patterns, field/variant sidecars, direct function/extern refs, local references, params, lets, pattern bindings, loop variables, closure captures, and known direct call edges carry resolved IDs, selected targets, `HirCallTarget`, `HirVarTarget`, or scoped `HirLocalId`s where lowering resolves them | Remaining compatibility strings are display/diagnostic metadata or belong to Tasks 4-5, 11-15, 18, and 21 |
| 4. ID-backed alias interfaces | Complete | Persistent import/export/prelude/module-local/root alias interfaces are ID-backed in resolver/product/artifact metadata; product artifact format is `23` | Remaining strings are display/diagnostic metadata or collection-local syntax/display maps, not persistent alias contracts |
| 5. Source/path resolution out of `Lowerer` | Complete | Body/type lowering routes source/path/name lookup through `LowerResolutionContext`; lower-side value, nominal type, trait, import, prelude, dependency, module-prefix, and artifact-root-export resolution policy is centralized behind intent-specific APIs | Broader source-loader ownership and lowerer state-shell decomposition remain Task 17/18 work |
| 6. Provider capability split | Complete for scoped slice | `ExternCrateStore`/crate context expose dependency metadata/body/link capability paths without `LoadedCrate` storage | No remaining work for the artifact-backed dependency storage slice; source-provider/module-loader work remains Task 17 |
| 7. Explicit body/link providers | Complete for scoped slice | Cross-crate body and link access go through extern provider APIs used by collect/lower/mono/codegen setup | No remaining work for explicit body/link capability APIs found in this reconciliation |
| 8. Type lowering boundary | Complete | `lib/src/type_lowering.rs` is used by collection and lowering | No remaining work for parsed-type lowering extraction; it intentionally still emits structural `Type` |
| 9. Interned `Ty` / type context scaffold | Complete | `lib/src/type_context/mod.rs` defines `Ty`, `TypeContext`, interning, and compatibility conversion helpers | No remaining work for scaffolding; downstream migration is Task 11 follow-up |
| 10. Type facts services | Complete | `lib/src/type_services/*` owns facts, display, projection normalization, and layout-shape services | No remaining work for service extraction found in this reconciliation |
| 11. `TypeId` phase boundaries | Complete | `ResolvedHirProgram` owns `TypeContext`/`HirTypeIds`; mono instance identity and substitutions use `TypeId`; MIR locals/returns/casts/callables carry `TypeId`; borrowck/agreement/codegen query type shape through the owning type context; product artifacts convert through a portable artifact-local type table at the explicit compatibility boundary | No remaining Type Context/Semantic Types work; future artifact evolution should be driven by explicit dependency capability needs, not type identity gaps |
| 12. Shared selection service | Complete | `lib/src/selection/*` owns the frontend selection authority contract, including selected impl/method/trait identities, trait args, receiver adjustment, return/output facts, diagnostics, and lowering-time method/operator/index selection | No remaining Task 12 work; targetless mono/lowering fallback cleanup is complete under Task 13 and direct codegen trait/member backend metadata cleanup is complete under the 2026-05-31 Task 21 follow-up |
| 13. Remove mono/codegen selection duplication | Complete | `HirMethodCallTarget` values are authoritative for mono/lowering semantic method/operator/index/trait dispatch, and targetless dispatch no longer rediscovers generic methods by name or receiver fallback | No remaining Task 13 targetless fallback work; direct codegen trait/member metadata cleanup is complete under the Task 21 backend metadata cleanup follow-up |
| 14. Instance registry callable universe | Complete for scoped slice | `MonomorphizedProgram.instances` drives emitted callable declarations/bodies and MIR builder instances | Codegen/MIR still need `MonomorphizedProgram.program` metadata for layouts, externs, products, symbols, linking, and remaining compatibility declarations/aliases; direct trait/member selection metadata is no longer rediscovered by active MIR codegen |
| 15. `InstanceId` call edges | Complete for scoped slice | `HirVarTarget::Instance`, `MirCallable::Instance`, `InstanceId -> backend_symbol` resolution, `process_call_does_not_resolve_callee_by_backend_symbol`, and static impl `ResolvedVar(Function(method_id))` specialization cover known generic function/method call edges without backend-symbol mono lookup. Resolved generic function calls, function values, function-valued arguments, and extern targets now use canonical `DefId`/`InstanceId` lookup before name compatibility, so string-keyed mono maps no longer drive semantic callable selection when canonical IDs are available | No remaining Task 15 backend-symbol call-target lookup found in monomorphization; active MIR codegen no longer relies on generated-name HIR aliases for selected trait/member dispatch, and DCE name/backend-symbol alias cleanup is covered by Task 16 |
| 16. Instance reachability DCE | Complete for scoped slice | `prune_unreachable_instances` is the compile-pipeline DCE authority and follows `HirVarTarget::Instance`, direct `Function(DefId)`, selected trait/impl targets, receiver field-method targets, object-backed leaves, imported custom-operator function IDs, and static impl method function-value IDs without source-name/backend-symbol reachability aliases | Legacy HIR/name pruning helpers have been removed; direct trait/member backend metadata cleanup is complete, while broader HIR-body instance payload and MIR/codegen metadata cleanup remains future work |
| 17. Source/module loader boundary | Complete for scoped slice | Parser consumes source text/tokens only; `SourceDatabase` owns filesystem reads, canonical paths, sibling module discovery, parsed-module caching, module graph construction, loaded-file ordering, and structured load errors; compile/collect/lower/crate-system/product fingerprint paths consume graph-derived module data | Loader is filesystem-only; virtual/artifact-backed sources are absent, and `Lowerer` still carries graph-derived backing state behind the Task 18 module-context compatibility boundary |
| 18. Lowerer decomposition | Complete for scoped slice | `LoweringPipeline` owns `lower_from_declarations` phase ordering; `ModuleLoweringContext` owns graph/cache-backed lookup, loaded-module traversal, loaded-root helpers, module-local alias cleanup, and qualified-prefix scoping; `LowerDiagnostics` owns diagnostics; `BodyLowerer` owns root/loaded body traversal | `Lowerer` remains the compatibility state shell with inference, scope, HIR staging maps, module backing state, prelude/artifact policy, aliases, constraints, and ID allocation pending later decomposition slices |
| 19. MIR canonical/runtime-complete boundary | Complete for scoped slice | MIR functions are keyed by `MirFunctionId`; callable constants carry canonical function/extern/instance/method identities; aggregates carry struct/enum/variant IDs and payloads; matches lower through discriminants, switches, and downcast-ready projections; closures use stable IDs; casts, bounds checks, drops, and agreement checks are explicit MIR forms | Readable names remain display metadata by design; unsupported advanced match-pattern cases still fail loudly; executable backend consumption landed under Task 21, while advanced runtime parity remains future work |
| 20. Borrowck indexed dataflow | Complete for scoped slice | Borrow collection stores typed `Location` values; active borrow checking builds `LoanTable`, groups loans by `Location`, propagates `LoanState`, uses `LocalSet` reference liveness, resolves provenance/conflicts through indexed `LoanId` facts without `HashMap<LoanId, Loan>` active state, and models places/moves through indexed `PlacePathId` / `MovePathId` tables | Active-loan propagation is still a local statement-precise fixpoint; optional follow-up is moving it into the generic MIR dataflow engine if that can preserve current before/after-statement precision |
| 21. MIR codegen boundary | Complete for executable-body and direct trait/member metadata slices | Main compile path builds monomorphized MIR, borrow-checks it, runs MIR agreement, and calls `compile_program_from_mir`; MIR backend modules lower executable statements/terminators, closures, calls, aggregates, casts, checks, and projections; active MIR codegen consumes explicit backend selection metadata for selected method symbols, trait member IDs, projection impl facts, and builtin-index guards; `compile_program` is test-only | HIR/mono remain metadata inputs for declarations, layouts, externs, products, symbols, and linking by design; broader explicit backend-metadata extraction remains future work |
| 22. Macro expansion context/source map | Complete for scoped slice | Macro expansion uses explicit context/config, source-map/origin records, expansion traces, generated parser config, and proc macro protocol v2 | No remaining work for the scoped macro architecture slice; parser/source-loader and formatter cleanup remain Tasks 17 and 23 |
| 23. Formatter trivia split | In progress | Formatter uses explicit `FormatContext`, `FormatInput`, and formatter-side `FormatTrivia` for standalone comments/blank lines; `LambdaDecl` no longer stores shorthand formatting tokens | Inline/trailing comment trivia, remaining formatter/trivia limitations, and replacement of AST `Display`-based formatting remain future work |

---

## Ordered Task Queue

### Task 1: Make Current-Crate ID Allocation Single-Source

**Status:** Complete. The initial supported-path slice landed in `docs/superpowers/plans/2026-05-17-authoritative-hir-id-keyed-storage.md`, artifact declaration collection follow-up landed in `docs/superpowers/plans/2026-05-25-artifact-collection-indexed-ids.md`, and full current-crate sentinel/provisional cleanup is tracked in `docs/superpowers/plans/2026-05-25-task-1-current-crate-id-completion.md`.

**Goal:** Ensure collection/index allocation is the only authority for current-crate `DefId`s, including methods and child items.

**Why Now:** Authoritative HIR storage and alias tables are unsafe to make canonical while provisional or repair ID passes can still assign identities after declarations are built.

**Primary Files:** `lib/src/collect/mod.rs`, `lib/src/collect/context.rs`, `lib/src/collect/item_index.rs`, `lib/src/hir/mod.rs`.

**Work:** Remove provisional `CrateId(u32::MAX)` and post-declaration repair paths by allocating every current-crate item/member ID during collection. Treat any missing canonical identity as a collection error before lowering begins.

**Depends On:** Existing item index and child ID work.

**Verification Focus:** Same-name definitions across modules, methods/defaults/fields/variants/associated types, artifact round-trip ID stability, and no synthetic/fallback `DefId` creation in supported current-crate paths.

### Task 2: Make HIR ID-Keyed Storage Authoritative

**Status:** Complete for migrated finalized HIR ownership storage in `docs/superpowers/plans/2026-05-17-authoritative-hir-id-keyed-storage.md`; remaining strings are explicit display/diagnostic/artifact/unresolved-name compatibility metadata or later Task 18/21 state-shell/backend metadata work, not open Task 2 ownership storage work.

**Goal:** Replace `HirProgram` string-keyed ownership maps with ID-keyed tables and preserve source names as metadata.

**Why Now:** Many consumers already have ID-keyed accessors, but storage still routes through names. Making storage authoritative unlocks clean resolver aliases, DCE, mono, and codegen migration.

**Primary Files:** `lib/src/hir/mod.rs`, `lib/src/collect/mod.rs`, `lib/src/lower/mod.rs`, `lib/src/mono/*`, `lib/src/codegen/*`, `lib/src/mir/builder/*`, `lib/src/products.rs`.

**Work:** Store functions, structs, enums, traits, impls, externs, fields, variants, and associated items by canonical IDs. Keep display/source names in side tables or fields. Provide compatibility accessors only for unmigrated source-name lookup paths, not semantic ownership.

**Depends On:** Task 1.

**Verification Focus:** Existing same-name alias tests, product emission, codegen top-level lookup, mono input partitioning, MIR building, and no duplicate semantic entity under an alias name.

### Task 3: Convert HIR Semantic References To IDs

**Status:** Complete. HIR semantic references now carry authoritative `DefId`, `InstanceId`, field/variant IDs, selected method targets, call targets, or scoped `HirLocalId`s where lowering resolves them. Remaining compatibility strings are display/diagnostic metadata or belong to Tasks 4-5, 11-15, 18, and 21.

**Goal:** Replace remaining string-valued HIR semantic references with explicit IDs or scoped local IDs where practical.

**Why Now:** ID-keyed storage is only half the boundary; HIR expressions and patterns still carry strings for variables, method calls, struct literals, enum patterns, and method locations.

**Primary Files:** `lib/src/hir/mod.rs`, `lib/src/lower/**`, `lib/src/infer/**`, `lib/src/mono/**`, `lib/src/codegen/**`, `lib/src/mir/builder/**`.

**Work:** Introduce resolved references for top-level functions, methods, structs, enum variants, fields, locals, and pattern targets. Keep source names for diagnostics and display, but do not require later phases to reconstruct targets from strings.

**Depends On:** Task 2.

**Verification Focus:** Same-name functions/methods/variants, pattern lowering, field access, method-call targets, diagnostics retaining source names, and artifact-loaded HIR references.

### Task 4: Replace Persistent String Alias Interfaces With ID-Backed Aliases

**Status:** Complete. Persistent import/export/prelude/module-local/root alias interfaces are ID-backed through resolver/product/artifact metadata, product artifact format `23` rejects old string-only alias contracts, and remaining strings are display/diagnostic metadata.

**Goal:** Make import/export/prelude/module-local aliases persist as canonical ID references rather than string-to-string maps.

**Why Now:** Alias maps are a major reason lower/mono/codegen still accept source names as semantic inputs.

**Primary Files:** `lib/src/collect/resolver.rs`, `lib/src/collect/context.rs`, `lib/src/collect/mod.rs`, `lib/src/lower/mod.rs`, `lib/src/lower/paths.rs`, `lib/src/products.rs`, `lib/src/crate_artifact/load.rs`.

**Work:** Replace `HashMap<String, String>` alias interfaces with ID-backed alias tables at collection/resolver/product boundaries. Lowering may still resolve source names, but it should ask resolver tables for IDs rather than translating strings through compatibility maps.

**Depends On:** Tasks 1-3.

**Verification Focus:** Import aliases, export aliases, prelude aliases, artifact root exports, ambiguous export collisions, and dependency alias round trips.

### Task 5: Split Source/Path Name Resolution Out Of `Lowerer`

**Status:** Complete. Body/type lowering consumes source/path/name resolution through `LowerResolutionContext`; semantic aliases and canonical path/type/trait/import/prelude/dependency/artifact lookups are resolved behind lower-side intent APIs. Broader source-loader ownership and full `Lowerer` state-shell decomposition remain Task 17/18 work.

**Goal:** Make body/type lowering consume resolved names/IDs rather than performing broad source/path resolution inside `Lowerer`.

**Why Now:** Once HIR storage and aliases are ID-backed, `Lowerer` can stop owning resolver policy and become closer to AST-to-HIR body conversion.

**Primary Files:** `lib/src/lower/paths.rs`, `lib/src/lower/types.rs`, `lib/src/lower/mod.rs`, `lib/src/collect/resolver.rs`, `lib/src/collect/context.rs`.

**Work:** Move expression-path, type-name, trait-name, prelude, dependency, and module alias lookup behind resolver APIs. Lowering should receive or request resolved IDs through a narrow interface and emit diagnostics when resolution is absent.

**Depends On:** Tasks 1-4.

**Verification Focus:** Path resolution in local modules, dependencies, prelude imports, artifact aliases, same-name declarations, and errors for unresolved canonical IDs.

### Task 6: Split `LoadedCrate` Into Provider Capabilities

**Status:** Complete for artifact-backed external dependencies in `docs/superpowers/plans/2026-05-18-rust-style-extern-crate-store.md`.

**Goal:** Separate dependency metadata, interface, body-provider, source-provider, and link-provider capabilities from the monolithic `LoadedCrate` storage object.

**Why Now:** Trait selection, mono, and codegen should ask dependencies for capabilities, not inspect source/object/interface storage mode.

**Primary Files:** `lib/src/crate_system/mod.rs`, `lib/src/crate_system/context.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/mono/external.rs`, `lib/src/lib.rs`.

**Work:** Replace direct `LoadedCrate` storage access with provider traits or structs keyed by session `CrateId`. Move `ArtifactMode` and source-vs-object branching behind provider construction.

**Depends On:** Tasks 1-5, especially ID-backed dependency aliases.

**Verification Focus:** Object-backed artifacts, interface-only artifacts, generic body loading, link inputs, stdlib prelude exports, and product artifact crate ID remapping.

### Task 7: Make Dependency Body And Link Providers Explicit

**Status:** Complete for explicit body/link capability APIs in `docs/superpowers/plans/2026-05-18-rust-style-extern-crate-store.md`.

**Goal:** Replace thin wrappers around cross-crate HIR and link metadata with explicit body/link provider APIs.

**Why Now:** Monomorphization and codegen need stable interfaces before instance registry authority and MIR-backed codegen can be completed.

**Primary Files:** `lib/src/crate_system/mod.rs`, `lib/src/crate_system/context.rs`, `lib/src/mono/external.rs`, `lib/src/codegen/mod.rs`, `lib/src/products.rs`.

**Work:** Provide direct methods for generic function bodies, generic impl bodies, trait default bodies, object symbols, backend symbols, and link objects by canonical ID. Keep storage-mode details private to crate loading.

**Depends On:** Task 6.

**Verification Focus:** Artifact-backed generic functions, generic impls, trait defaults, object symbols, and no source-backed dependency body fallback in downstream compilation.

### Task 8: Extract Type Lowering Into A Type-Lowering Boundary

**Status:** Complete for parsed-type lowering extraction in `docs/superpowers/plans/2026-05-18-type-lowering-boundary.md`.

**Goal:** Move parsed type syntax conversion out of `Lowerer` while keeping the existing ID-backed `Type` representation.

**Why Now:** Phase 6 made `Type` identity sound. The next step is ownership: parsed type lowering should be its own boundary before a full `Ty`/type-context migration.

**Primary Files:** `lib/src/lower/types.rs`, `lib/src/lower/mod.rs`, `lib/src/types/mod.rs`, `lib/src/collect/headers.rs`, `lib/src/collect/context.rs`.

**Work:** Create a type-lowering service that consumes resolver/type declaration context and emits semantic `Type` values plus diagnostics. Remove generic type construction and projection lowering policy from `Lowerer` state.

**Depends On:** Tasks 4-5.

**Verification Focus:** Nominal/generic/projection lowering, where clauses, artifact-loaded types, same-name type declarations, and Phase 6 stale identity tests.

### Task 9: Decide And Scaffold Interned `Ty` / Type Context

**Status:** Complete for interned `Ty` / `TypeId` scaffolding in `docs/superpowers/plans/2026-05-19-type-context-scaffold.md`.

**Goal:** Choose interned `Ty` / `TypeId` as the long-term type identity model and add a semantic type-context scaffold without migrating all phase boundaries yet.

**Why Now:** Do this only after `Type` identity and type-lowering ownership are stable; otherwise the compiler would have two competing type truth sources.

**Primary Files:** `lib/src/type_context/**`, `lib/src/types/mod.rs`, `lib/src/ids.rs`, focused tests near the new type context.

**Work:** Introduce `Ty`, `TypeContext`, interning by canonical semantic structure, `Type -> TypeId -> Type` compatibility helpers, and tests proving nominal/generic/projection identity. Keep HIR, inference, artifacts, mono, MIR, and codegen on structural `Type` during this slice.

**Depends On:** Task 8.

**Verification Focus:** Interning equality/hash behavior, nominal/generic/projection identity, type-var roundtrips, compatibility conversions, and no context-free display strings as semantic keys.

### Task 10: Move Type Facts Into Dedicated Services

**Status:** Complete for type fact, display, projection normalization, and layout-shape service extraction in `docs/superpowers/plans/2026-05-19-type-services.md`.

**Goal:** Remove ad hoc semantic facts from `Type` helpers and move them to type/layout/trait services.

**Why Now:** `Type` currently mixes semantic structure with copy semantics, builtin indexing knowledge, display formatting, and layout-facing behavior.

**Primary Files:** `lib/src/types/mod.rs`, `lib/src/codegen/types.rs`, `lib/src/lower/types_helpers/**`, `lib/src/infer/**`, future type/layout service modules.

**Work:** Move copy semantics, builtin index behavior, projection normalization, layout-facing facts, and symbol/display formatting into explicit services that can use HIR/type context.

**Depends On:** Task 9.

**Verification Focus:** Copyability tests, indexing/deref/operator tests, codegen type layout tests, projection tests, and display diagnostics.

### Task 11: Migrate `TypeId` Across Type-Carrying Phase Boundaries

**Status:** Complete. Compiler-owned phase boundaries after HIR finalization use context-owned `TypeId` identity across mono instance keys/substitutions, MIR type-bearing runtime forms, borrowck/MIR agreement type queries, and codegen type/layout/ABI APIs. Product artifacts now serialize type payloads through a portable artifact-local type table and re-intern decoded types into the consumer context during artifact loading.

**Goal:** Move selected HIR, inference, and artifact-facing type fields from structural `Type` to context-owned `TypeId` after the scaffold and type fact services are stable.

**Why Now:** Task 9 makes canonical type identity representable, and Task 10 moves semantic facts behind services. Only then can phase boundaries consume `TypeId` without immediately needing structural `Type` everywhere.

**Primary Files:** `lib/src/type_context/**`, `lib/src/types/mod.rs`, `lib/src/hir/mod.rs`, `lib/src/infer/**`, `lib/src/crate_artifact/load.rs`, `lib/src/products.rs`, `lib/src/lower/**`, `lib/src/mono/**`.

**Work:** Pick narrow type-carrying boundaries and replace stored structural `Type` fields with `TypeId` plus explicit `TypeContext` access. Preserve product artifact compatibility through explicit artifact-boundary conversion and portable type-table serialization.

**Depends On:** Tasks 9-10.

**Verification Focus:** Equality/hash behavior after migration, artifact serialization/remapping compatibility, inference substitution/generalization, mono substitution, no context-free display strings as semantic keys, and no accidental `TypeId` persistence in current product artifacts.

### Task 12: Build A Shared Trait And Method Selection Service

**Status:** Complete. The shared selection service is the frontend authority for method, operator, index, and trait-bound selection facts. Targetless mono/lowering fallback deletion is complete under Task 13, and direct codegen trait/member backend metadata cleanup is complete under the Task 21 follow-up while broader backend metadata extraction remains future work.

**Goal:** Centralize impl lookup, receiver adjustment, generic substitution, associated type normalization, trait default resolution, and method selection.

**Why Now:** Lowering, mono, and codegen still repeat selection logic. Mono and codegen cannot become mechanical until selection results are explicit.

**Primary Files:** `lib/src/lower/control_flow/secondary.rs`, `lib/src/lower/expression.rs`, `lib/src/lower/traits/**`, `lib/src/infer/**`, `lib/src/mono/methods.rs`, `lib/src/codegen/expr/mod.rs`, `lib/src/codegen/mod.rs`.

**Work:** Add a semantic selection layer that returns selected impl ID, method ID, trait ID, trait args, receiver adjustment, normalized return/projection facts, default-method origin, and diagnostics. Lowering records the selection; mono/codegen consume it.

**Depends On:** Tasks 3, 8, and 10.

**Verification Focus:** Same-name traits/methods, generic trait bounds, trait defaults, artifact-backed methods, stdlib operator traits, associated type outputs, and selected-target diagnostics before codegen.

### Task 13: Remove Mono And Codegen Selection Duplication

**Status:** Complete. Present selected targets are authoritative, targetless mono/lowering semantic dispatch fallback has been removed, and direct codegen trait/member metadata cleanup is complete under the Task 21 backend metadata cleanup follow-up.

**Goal:** Make mono/codegen consume selected targets rather than redoing trait/method lookup.

**Why Now:** This is the cleanup pass that proves the selection service is actually authoritative.

**Primary Files:** `lib/src/mono/methods.rs`, `lib/src/mono/process.rs`, `lib/src/codegen/expr/mod.rs`, `lib/src/codegen/mod.rs`, `lib/src/codegen/types.rs`.

**Work:** Delete or narrow semantic fallback paths in mono/codegen. Keep backend symbol lookup and LLVM declaration maps, but only after selected identity exists.

**Depends On:** Task 12.

**Verification Focus:** Same-name method dispatch, selected trait default methods, artifact-qualified impl owners, builtin index/unary ops, and codegen errors when selected targets are missing.

### Task 14: Make `InstanceRegistry` The Sole Callable Universe

**Status:** Complete for the callable-universe slice in `docs/superpowers/plans/2026-05-20-instance-registry-callable-universe.md`; HIR/mono metadata cleanup remains later backend-boundary work.

**Goal:** Represent all emitted callable bodies as `InstanceRecord`s, not a mix of HIR functions, impl methods, specialized bodies, and object-backed records.

**Why Now:** A single callable universe is required for DCE, MIR/codegen, and backend symbol hygiene.

**Primary Files:** `lib/src/mono/registry.rs`, `lib/src/mono/process.rs`, `lib/src/mono/specialize.rs`, `lib/src/mono/external.rs`, `lib/src/codegen/mod.rs`, `lib/src/hir/mod.rs`.

**Work:** Model current-crate concrete functions, impl methods, trait defaults, generic specializations, object-backed declarations, and artifact-backed bodies uniformly as instances with `InstanceId`s and metadata.

**Depends On:** Tasks 6-7 and 12-13.

**Verification Focus:** Current-crate functions, impl methods, trait defaults, generic methods, object-backed artifacts, duplicate specialization reuse, and no direct codegen compilation from HIR maps.

### Task 15: Replace Backend-Symbol Call Targets With `InstanceId` Edges

**Status:** Complete for scoped slice. `HirVarTarget::Instance`, `MirCallable::Instance`, and instance symbol maps cover known generic function/method call edges, and monomorphization no longer recovers call targets from emitted backend symbols.

**Goal:** Make backend symbols outputs of instance lowering, not call targets used during monomorphization.

**Why Now:** Instance registry authority is incomplete while calls are rewritten to `HirExprKind::Var(record.backend_symbol)` or searched by backend symbol.

**Primary Files:** `lib/src/mono/registry.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/specialize.rs`, `lib/src/mono/process.rs`, `lib/src/codegen/mod.rs`.

**Work:** Add resolved callable references or `InstanceId` call targets in monomorphized HIR/MIR input. Backend symbols remain in records for object output and LLVM symbol maps.

**2026-05-25 verification note:** Removed `lookup_specialized_function` and added `process_call_does_not_resolve_callee_by_backend_symbol`, proving a `Var(record.backend_symbol)` no longer recovers an instance body or drives generic argument specialization. Static impl calls with `ResolvedVar(Function(method_id))` also specialize by method ID before targetless name compatibility. Resolved generic function calls, function values, function-valued arguments, and extern targets now use canonical `DefId`/`InstanceId` lookup before name compatibility, so string-keyed mono maps no longer drive semantic callable selection when canonical IDs are available. Focused Task 15 regressions, `cargo fmt --all --check`, `git diff --check`, and full `cargo test -p rock-lib` passed after this update. DCE backend-symbol/name alias removal is covered by Task 16, and generated-name HIR codegen compatibility for selected trait/member dispatch is complete under the Task 21 backend metadata cleanup follow-up.

**Depends On:** Task 14.

**Verification Focus:** Generic function reuse, method specialization reuse, artifact-backed calls, backend symbol remapping, and no semantic lookup by emitted symbol names.

### Task 16: Replace HIR/Name-Based DCE With Instance Reachability

**Status:** Complete for scoped slice. The compile pipeline uses instance-table reachability DCE after monomorphization, and reachable callable edges are discovered through explicit `InstanceId`, direct `DefId`, selected method/trait identity, receiver type, and artifact/object-backed instance metadata rather than source names or backend symbols.

**Goal:** Run dead-code elimination over resolved `DefId`/`InstanceId` call edges after monomorphization.

**Why Now:** DCE cannot be authoritative until all callable bodies and call edges are explicit.

**Primary Files:** `lib/src/dce.rs`, `lib/src/mono/registry.rs`, `lib/src/lib.rs`, `lib/src/codegen/mod.rs`.

**Work:** Build a reachability graph over `DefId` and `InstanceId`, including function values, wrappers, trait impls/defaults, object-backed declarations, artifact bodies, and runtime helpers. Retire HIR string-name DCE.

**2026-05-25 verification note:** Removed name/backend-symbol reachability from instance DCE, covered direct instance edges, direct function references, trait-target receiver impl edges, field-access receiver method edges, object-backed declarations, imported custom operators lowered as `ResolvedVar(Function(DefId))`, static impl method specialization by `DefId` for calls and function values, specialized static impl constructor ABI preservation, and product artifact method-ID validation that rejects receiver methods as function targets while accepting static impl method IDs with callable-name validation. Verified with focused red/green regressions plus `cargo fmt --all`, `cargo fmt --all --check`, `git diff --check`, and full `cargo test -p rock-lib > /tmp/rock-lib-task16-final.log 2>&1` (`1251` unit tests passed with `1` ignored; `276` integration tests passed; parser integration and doctests passed).

**2026-06-03 verification note:** Removed the legacy non-pipeline HIR/name DCE helpers and helper-only tests. `prune_unreachable_instances` remains the DCE authority for the compile pipeline, and instance body representation cleanup remains future work.

**Depends On:** Tasks 14-15.

**Verification Focus:** Unused current functions, unused specializations, trait/default reachability, runtime helper retention, object-backed symbol retention, and artifact body reachability.

### Task 17: Introduce A Source And Module Loader Boundary

**Status:** Complete for the scoped filesystem-backed source database slice. Parser file IO is removed, `SourceDatabase` owns source/module file loading and graph construction, and compile/collect/lower/crate-system/product fingerprint paths consume loader graph/cache views; virtual/artifact-backed sources and broader lowerer ownership cleanup remain open.

**Goal:** Move filesystem IO, module discovery, source caches, virtual sources, and artifact-provided source/module bodies out of parser and lowering.

**Why Now:** After identity and dependency providers are stable, module loading can become a clean frontend service without changing semantic identity at the same time.

**Primary Files:** `lib/src/parser/mod.rs`, `lib/src/parser/items/module.rs`, `lib/src/parser/engine/mod.rs`, `lib/src/collect/context.rs`, `lib/src/lower/program.rs`, `lib/src/crate_system/module_tree.rs`, future source loader module.

**Work:** Parser consumes source text/tokens. Loader owns canonical paths, sibling module discovery, source caches, virtual files, artifact-provided modules, and structured diagnostics for IO/parse failures.

**2026-05-25 verification note:** Removed parser-owned file-loading APIs and sibling-module parser helpers; added `SourceDatabase`, `ModuleGraph`, loaded-file fingerprinting, graph-backed collection/lowering/source-crate loading, `rockc` source-command loading, structured missing/circular/parse diagnostics, and integration regressions for `name/mod.rk`, missing modules, circular modules, module globs, and product fingerprints. Focused Task 17 regressions passed, and final verification passed with `cargo fmt --all`, `cargo fmt --all --check`, `git diff --check`, and `cargo test -p rock-lib > /tmp/rock-lib-task17-final.log 2>&1` (`1251` unit tests passed with `1` ignored; `276` integration tests passed; parser integration and doctests passed).

**Depends On:** Tasks 1-7.

**Verification Focus:** Multi-module parsing, module cache behavior, artifact-provided modules, parser no-file-IO unit tests, and no panic/unwrap path for empty `mod` IO failures.

### Task 18: Finish `Lowerer` Decomposition

**Status:** Complete for the scoped lowerer-decomposition slice. Named boundaries now own phase ordering, graph/cache-backed module context helpers, diagnostics state, and body traversal entry points; `Lowerer` remains the compatibility state shell for later slices.

**Goal:** Split `Lowerer` into narrow body lowering, scope, diagnostics, module context, type lowering, and trait/method selection interfaces.

**Why Now:** Earlier tasks extracted resolver, type, dependency, selection, and loader responsibilities. This pass removes the remaining god-object structure.

**Primary Files:** `lib/src/lower/mod.rs`, `lib/src/lower/program.rs`, `lib/src/lower/**`, new lower submodules as needed.

**Work:** Move prelude/artifact policy, module loading, trait conformance/default setup, type lowering, and selection setup out of the core body-lowering context. Keep body lowering focused on resolved AST to HIR plus constraints.

**2026-05-25 verification note:** Added and verified `LoweringPipeline`, `ModuleLoweringContext`, `LowerDiagnostics`, and `BodyLowerer` boundaries. Follow-up refinements moved loaded-root classification, module-local alias cleanup, and qualified module-prefix scoping behind `ModuleLoweringContext`. Focused Task 18 regressions passed, final verification passed with `cargo fmt --all --check && cargo test -p rock-lib > /tmp/rock-lib-task18-final.log 2>&1 && git diff --check` (`1253` unit tests passed with `1` ignored; `276` integration tests passed; parser integration and doctests passed), and final code review returned `STATUS: PASS` with no Critical or Important findings.

**Depends On:** Tasks 5-13 and 17.

**Verification Focus:** End-to-end compilation, module bodies, trait defaults, conformance checks, source/artifact dependencies, diagnostics, and no broad mutable name maps in the core lowerer.

### Task 19: Make MIR Identities Canonical And Runtime-Complete

**Status:** Complete for the scoped MIR canonical runtime slice. MIR now carries canonical identities and explicit runtime forms for the planned Task 19 surface while readable names remain display metadata, executable backend consumption landed under Task 21, and advanced runtime parity remains future work.

**Goal:** Prepare MIR to become the executable backend boundary by replacing name-bearing identities and placeholder lowering.

**Why Now:** Codegen cannot move to MIR until MIR represents runtime semantics with canonical references.

**Primary Files:** `lib/src/mir/mod.rs`, `lib/src/mir/builder/**`, `lib/src/hir/mod.rs`, `lib/src/mono/registry.rs`.

**Work:** Encode calls, resolved callees, places, projections, aggregate construction, enum variants/discriminants, matches, closures, bounds checks, drops, casts, runtime helper requirements, and instance/callable references. Replace function/aggregate strings with canonical IDs.

**2026-05-25 verification note:** Verified `MirFunctionId`-keyed programs, canonical callable constants, method call targets, struct/enum aggregate identities, field identities, discriminant/switch/downcast match MIR, closure IDs, runtime casts, bounds-check assertions, enum/closure drop needs, and MIR agreement checks. Final cleanup extended agreement checks to reject callable unit placeholders both as direct `Constant::Unit` operands and as unit-typed local/temp call operands. Focused MIR and runtime behavior filters passed, final verification passed with `cargo fmt --all && cargo fmt --all --check && cargo test -p rock-lib > /tmp/rock-lib-task19-final.log 2>&1 && git diff --check` (`1255` unit tests passed with `1` ignored; `276` integration tests passed; parser integration and doctests passed), and final code review returned `STATUS: PASS` after the agreement-check fix.

**Depends On:** Tasks 3, 12, and 14-15.

**Verification Focus:** MIR dumps, borrowck behavior, method calls, enum matches, closures, array/slice operations, drops, runtime checks, and MIR/codegen agreement scaffolding.

### Task 20: Convert Borrowck To Indexed Dataflow

**Status:** Complete for the scoped borrowck indexed dataflow slice. Borrow creation, active-loan propagation, provenance, conflict checking, place modeling, and move tracking now use typed locations, compiler-wide `LoanId`s, indexed loan/place/move tables, dense loan/local sets, and statement-precise liveness.

**Goal:** Replace `HashMap`/`HashSet` loan state with indexed tables, typed locations, and bitset dataflow.

**Why Now:** Borrowck should operate over stable MIR IDs and runtime-complete places before investing in indexed dataflow.

**Primary Files:** `lib/src/mir/borrowck/**`, `lib/src/mir/dataflow/**`, `lib/src/ids.rs`, `lib/src/mir/mod.rs`.

**Work:** Introduce typed MIR `Location`, replace active loan maps with indexed `LoanTable` / `LoanState`, preserve origin spans through loan metadata, adopt indexed `PlacePathTable` / `MovePathTable` path modeling, remove the legacy `BorrowId` helper path, and keep borrow diagnostics stable. Remaining follow-up is optional generic active-loan propagation through the MIR dataflow engine if it can preserve statement-level before/after precision.

**2026-05-25 verification note:** Verified that the Task 20 audit patterns have no remaining active-loan production hits, `mir::dataflow` and `mir::borrowck` focused suites pass, borrow/closure/array/vector-index integration filters pass, final verification passed with `cargo fmt --all && cargo fmt --all --check && cargo test -p rock-lib > /tmp/rock-lib-task20-final.log 2>&1 && git diff --check` (`1255` unit tests passed with `1` ignored; `276` integration tests passed; parser integration and doctests passed), and final code review returned `STATUS: PASS`.

**2026-06-04 verification note:** Adopted indexed `PlacePathTable` / `MovePathTable` modeling with `PlacePathId` / `MovePathId` for partial moves, drops, path-aware initialization, and loan checks. Focused borrowck/dataflow verification and full `cargo test -p rock-lib` passed for that place/move path model slice.

**2026-06-05 verification note:** Removed the legacy `compute_live_borrows` / `BorrowId` / `LiveBorrowSet` helper path so `LoanId` / `LoanTable` / `LoanState` remains the sole borrow identity model in `lib/src/mir`. Verification for this cleanup was rerun with focused borrowck/dataflow/borrow filters, formatting and diff checks, a no-hit legacy-symbol search under `lib/src/mir`, and full `cargo test -p rock-lib`.

**Depends On:** Task 19.

**Verification Focus:** Borrowck regression suite, liveness, conflicting loan diagnostics, moved/borrowed places, closures if supported, and stable span/origin reporting.

### Task 21: Move Codegen To Consume MIR

**Status:** Complete for the executable-body slice in `docs/superpowers/plans/2026-05-23-mir-backed-codegen.md` and direct trait/member metadata slice in `docs/superpowers/plans/2026-05-31-task-21-backend-metadata-cleanup.md`; executable LLVM body emission now consumes monomorphized MIR, active MIR codegen consumes explicit backend selection metadata for selected trait/member facts, and HIR/mono remain retained for declarations, layouts, products, externs, symbols, linking, and remaining compatibility metadata.

**Goal:** Make codegen lower MIR plus layouts, ABI signatures, instances, runtime helpers, and link metadata to LLVM.

**Why Now:** This is the largest backend boundary shift and should happen only after MIR and instance identity are complete.

**Primary Files:** `lib/src/codegen/**`, `lib/src/mir/**`, `lib/src/mono/registry.rs`, `lib/src/lib.rs`.

**Work:** Replace HIR expression/codegen lowering with MIR lowering. Remove frontend semantic selection from codegen. Keep LLVM mechanics, runtime declarations, object emission, symbol maps, and linking in codegen.

**2026-05-25 verification note:** Verified that `compile_impl` builds monomorphized MIR and calls `CodeGen::compile_program_from_mir`; HIR body lowering remains only in helper definitions and test-only paths. A final review finding on eager MIR lowering for `&&` / `||` was fixed by lowering short-circuit operators to MIR control flow, with `test_logical_operators_short_circuit_rhs_bounds_checks` proving RHS bounds checks are skipped. Focused Task 21 suites passed (`mir::builder`, `mir::agreement`, `mir::borrowck`, and `codegen`), behavior filters passed for borrow, closure, enum, array, vec-index, generic, and stdlib cases, final verification passed with `cargo fmt --all && cargo fmt --all --check && cargo test -p rock-lib > /tmp/rock-lib-task21-final.log 2>&1 && git diff --check` (`1255` unit tests passed with `1` ignored; `277` integration tests passed; parser integration and doctests passed), and final re-review returned `STATUS: PASS`.

**2026-05-31 verification note:** Added explicit backend selection metadata for selected method symbols, trait member IDs, projection impl facts, and builtin-index guards. Active MIR codegen no longer registers or rediscovers trait/member selection facts directly from HIR trait tables for selected dispatch, while broader declaration, layout, extern, product, symbol, and link metadata still comes from `MonomorphizedProgram.program` and remains future backend-metadata work.

**Depends On:** Tasks 12-15 and 19.

**Verification Focus:** Full integration suite, MIR/codegen agreement tests, artifact-backed compilation, operator/index/trait dispatch, closure/lambda codegen, enum matches, drops, bounds checks, and runtime helper emission.

### Task 22: Clean Up Macro Expansion Context And Source Mapping

**Status:** Complete for the macro expansion architecture slice. Macro expansion now has explicit context/config threading, source-map/origin records, expansion-traced diagnostics, and generated-token parsing through the caller's parser config. Parser IO and source/module loader cleanup remain Task 17 work.

**Goal:** Give macro expansion explicit config, diagnostics, and source mapping instead of reparsing generated fragments with `Config::default()`.

**Why Now:** Parser and source loader boundaries should exist before macro expansion stops borrowing parser internals.

**Primary Files:** `lib/src/macro_expansion/mod.rs`, `lib/src/macro_expansion/**`, `lib/src/parser/**`, source loader modules from Task 17.

**Work:** Add macro expansion context, preserve invocation/generated-token source mapping, pass explicit parser config/source context to any reparse path, and report diagnostics through structured spans.

**Depends On:** Task 17.

**Verification Focus:** Macro expansion diagnostics, generated expression spans, config-sensitive parsing, multi-module macro behavior, and no `Config::default()` reparse path.

### Task 23: Separate Formatter Trivia From Semantic AST

**Status:** In progress. Formatter process-global state has been replaced by explicit `FormatContext`/`FormatInput`, standalone comments and blank lines use formatter-side `FormatTrivia`, and lambda shorthand formatting tokens are no longer stored in `LambdaDecl`; inline/trailing comment trivia, remaining formatter/trivia limitations, and AST `Display`-based formatting cleanup remain future work.

**Goal:** Replace global formatter state and AST-embedded formatting-only tokens with explicit formatter context and trivia preservation.

**Why Now:** This is important but largely independent of compiler semantic identity, so it can wait until the core architecture stops shifting.

**Primary Files:** `lib/src/fmt/**`, `lib/src/ast/tree.rs`, `lib/src/lexer/**`, parser/source loader modules from Task 17.

**Work:** Introduce formatter input/context, preserve comments/whitespace as trivia or a side table, remove lambda shorthand formatting-token storage from semantic AST data, and replace global formatter state.

**Depends On:** Task 17.

**Verification Focus:** Formatter golden tests, comments/whitespace preservation, lambda shorthand formatting, reentrant formatter tests, and no process-global formatter state.

---

## Initial Focused Plans Written

1. `Authoritative HIR ID-Keyed Storage`: complete in `docs/superpowers/plans/2026-05-17-authoritative-hir-id-keyed-storage.md`; covers Tasks 1-3 enough to make HIR ownership ID-first.
2. `ID-Backed Resolver And Alias Interfaces`: complete in `docs/superpowers/plans/2026-05-18-id-backed-resolver-and-alias-interfaces.md`; covers the planned Tasks 4-5 resolver/alias subset while full source/path extraction and compatibility map removal remain future work.
3. `Rust-Style Extern Crate Store`: complete in `docs/superpowers/plans/2026-05-18-rust-style-extern-crate-store.md`; covers Tasks 6-7 and makes artifact/dependency capabilities clean before selection/mono work.
4. `Type Lowering Boundary`: complete in `docs/superpowers/plans/2026-05-18-type-lowering-boundary.md`; covers Task 8 and keeps `Ty`/type-context migration deferred to Task 9.
5. `Type Context Scaffold`: complete in `docs/superpowers/plans/2026-05-19-type-context-scaffold.md`; covers Task 9 by adding interned `Ty` / `TypeId` identity and structural `Type` compatibility conversions while deferring phase-boundary migration to Task 11.
6. `Type Services`: complete in `docs/superpowers/plans/2026-05-19-type-services.md`; covers Task 10 by moving type facts, display formatting, projection normalization, and layout-facing shape checks behind explicit services while deferring `TypeId` phase-boundary migration to Task 11.
7. `TypeId HIR Boundary`: complete in `docs/superpowers/plans/2026-05-19-type-id-hir-boundary.md`; covers Task 11's first narrow boundary by attaching `TypeContext` and HIR type-ID sidecar data to `ResolvedHirProgram` while preserving structural `Type` compatibility for products and downstream phases.
8. `Shared Selection Service`: complete in `docs/superpowers/plans/2026-05-19-selection-service.md`; covers Task 12 by centralizing method, trait-bound, operator, and index selection behind `lib/src/selection/`, with Task 13 now covered by `docs/superpowers/plans/2026-05-19-selection-fallback-cleanup.md`.
9. `Selection Fallback Cleanup`: complete for Task 13 targetless mono/lowering fallback cleanup in `docs/superpowers/plans/2026-05-19-selection-fallback-cleanup.md` and `docs/superpowers/plans/2026-05-31-task-13-selection-fallback-cleanup.md`; direct codegen trait/member metadata cleanup is complete in `docs/superpowers/plans/2026-05-31-task-21-backend-metadata-cleanup.md`.
10. `Instance Registry Callable Universe`: complete for callable body/declaration authority in `docs/superpowers/plans/2026-05-20-instance-registry-callable-universe.md`; backend-symbol call-target cleanup, instance DCE, MIR executable-body codegen, and direct trait/member backend metadata cleanup are complete, while broader backend metadata cleanup remains future work.
11. `Instance Call Edges`: complete for the scoped Task 15 plan in `docs/superpowers/plans/2026-05-20-instance-call-edges.md`; monomorphization no longer has backend-symbol semantic lookup, DCE backend-symbol/name alias cleanup completed under Task 16, and HIR-codegen generated-name compatibility for selected trait/member dispatch completed under the Task 21 backend metadata cleanup follow-up.
12. `Instance Reachability DCE`: complete for the scoped Task 16 plan in `docs/superpowers/plans/2026-05-21-instance-reachability-dce.md`; pipeline DCE is instance-table authoritative and no longer keeps callables reachable through name/backend-symbol aliases, and the legacy HIR/name DCE helpers have been removed.
13. `Source Database Loader`: complete for the scoped filesystem-backed Task 17 plan in `docs/superpowers/plans/2026-05-22-source-database-loader.md`; parser file IO is removed, source/module loading goes through `SourceDatabase`, and remaining virtual/artifact-backed loader sources plus lowerer ownership cleanup are future work.
14. `Macro Expansion Architecture`: complete for the Task 22 scoped macro architecture slice in `docs/superpowers/plans/2026-05-24-macro-expansion-architecture.md` and `docs/superpowers/plans/2026-05-24-macro-architecture-spec-completion.md`.
15. `Artifact Collection Indexed IDs`: complete for the Task 1 follow-up in `docs/superpowers/plans/2026-05-25-artifact-collection-indexed-ids.md`; artifact declaration collection now uses `LocalCollector` indexed IDs with crate-qualified inline-module aliases and no longer uses the legacy provisional-ID context collector.
16. `Current-Crate ID Completion`: complete for Task 1 in `docs/superpowers/plans/2026-05-25-task-1-current-crate-id-completion.md`; collection/header provisional IDs, method-ID repair passes, lowering identity placeholders, and product-emission current-crate sentinel acceptance are removed or guarded.

The focused implementation plans are not equivalent to full audit completion. The code-verified reconciliation table above is the source of truth for which broad roadmap tasks remain partial.

---

## Work Not Reopened By This Roadmap

- Phase 6 semantic type identity is treated as complete for its intended scope. This roadmap does not reintroduce string-bearing equality identity for `Type`.
- Full downstream `TypeId` phase-boundary migration and product artifact type-table serialization are complete for the Type Context/Semantic Types track.
- Stdlib/sysroot discovery remains out of compiler ownership unless a separate approved plan changes that policy.
- Broad formatter/trivia, remaining source-loader ownership, full MIR backend metadata, and optional generic active-loan dataflow integration remain separate follow-up work after identity and callable ownership stabilize.

---

## Roadmap Completion Definition

The architecture audit can be considered complete only when:

- HIR and resolver identity are ID-authoritative, with names retained as metadata.
- Type and trait facts are owned by explicit services or contexts.
- Trait/method selection produces resolved selections consumed by mono and codegen.
- All callable bodies and calls are represented through `InstanceId`/resolved call edges.
- Codegen consumes MIR rather than HIR/monomorphized HIR.
- Borrowck uses indexed dataflow over stable MIR IDs.
- Crate/artifact dependency behavior is hidden behind metadata/body/link/source providers.
- Parser/module IO, macro source mapping, and formatter trivia have their own boundaries.
- `master-audit-checklist.md` has no `In progress` or `Not started` track with remaining unchecked items.
