# Final Master Audit Source Audit

Source spec: `docs/superpowers/specs/2026-04-24-compiler-architecture-audit-design.md`

Bead: `new_lang2-4ku.12`

Updated: 2026-06-18

Scope: direct source audit of all 11 compiler architecture tracks. This report audits source files against the audit design principles instead of treating `docs/superpowers/plans/master-audit-checklist.md` as authoritative.

## Verdict

`new_lang2-4ku.12` is complete. All 11 master-audit tracks satisfy the scoped audit criteria after the reopened blocker beads were completed.

The compiler now routes production semantics through canonical IDs, selected targets, `TypeId`/`TypeContext`, MIR/backend metadata, instance IDs, and dependency provider capabilities. Remaining strings, readable names, backend symbols, HIR-shaped compatibility views, and structural `Type` values are confined to display, diagnostics, serialization, artifact compatibility, local lexical metadata, test setup, or explicit conversion/compatibility boundaries. Explicit future work such as cleanup/unwind semantics and additional dataflow generalization remains tracked outside this master-audit closure and is not an active blocker for the completed boundary work.

## Track Results

| Track | Verdict | Primary Evidence |
| --- | --- | --- |
| 1. Identity And Arenas | Pass | `lib/src/ids.rs`, `lib/src/hir/mod.rs`, `lib/src/semantic_identity_audit.rs` |
| 2. Real Collection And Name Resolution | Pass | `lib/src/collect/item_index.rs`, `lib/src/collect/resolver.rs`, `lib/src/lower/resolution.rs`, `lib/src/semantic_identity_audit.rs` |
| 3. Type Context And Semantic Types | Pass | `lib/src/type_context/mod.rs`, `lib/src/hir/type_ids.rs`, `lib/src/mir/mod.rs`, `lib/src/products/type_table.rs` |
| 4. Lowerer Decomposition | Pass | `lib/src/lower/services.rs`, `lib/src/lower/body_context.rs`, `lib/src/lower/session.rs`, `lib/src/semantic_identity_audit.rs` |
| 5. Trait And Method Selection Service | Pass | `lib/src/selection/service.rs`, `lib/src/selection/matching.rs`, `lib/src/mir/identity.rs`, `lib/src/semantic_identity_audit.rs` |
| 6. MIR As Backend Boundary | Pass | `lib/src/lib.rs`, `lib/src/mir/mod.rs`, `lib/src/codegen/mod.rs`, `lib/src/codegen/mir_llvm/*` |
| 7. Monomorphization Instances | Pass | `lib/src/mono/registry.rs`, `lib/src/dce.rs`, `lib/src/mir/builder/mod.rs` |
| 8. Crate And Artifact Interface Split | Pass | `lib/src/crate_artifact/types.rs`, `lib/src/crate_system/extern_store.rs`, `lib/src/crate_system/context.rs` |
| 9. Parser, Macro, And Module Loader Cleanup | Pass | `lib/src/parser/mod.rs`, `lib/src/source_loader/mod.rs`, `lib/src/macro_expansion/*`, `lib/src/lower/module_context.rs` |
| 10. Formatter And Trivia Separation | Pass | `lib/src/fmt/mod.rs`, `lib/src/fmt/trivia.rs`, `lib/src/ast/tree.rs` |
| 11. Borrowck Indexed Dataflow | Pass | `lib/src/mir/borrowck/*`, `lib/src/mir/dataflow/*`, `lib/src/ids.rs` |

## 1. Identity And Arenas

Verdict: Pass.

Source evidence:

- `lib/src/ids.rs` defines the typed identity families used across compiler phases: `CrateId`, `ModuleId`, `LocalDefId`, `DefId`, typed item IDs, `TypeId`, `InstanceId`, `LoanId`, `MovePathId`, and `PlacePathId`.
- `lib/src/hir/mod.rs` stores top-level HIR entities in `DefId`-keyed maps and keeps `HirNameTables` documented as display, diagnostics, artifact compatibility, and tests.
- `lib/src/hir/mod.rs` exposes child identity through `HirDefinitionIndexes`, including methods, fields, variants, and associated types.
- `lib/src/semantic_identity_audit.rs` guards that HIR name tables are not read for production semantics, DCE does not select by display/backend names, and product remapping does not fabricate current-crate IDs.

Finding:

Semantic identity is ID-owned for the production paths covered by the audit. Names remain as display, diagnostics, artifact compatibility, or unresolved/error recovery metadata.

## 2. Real Collection And Name Resolution

Verdict: Pass.

Source evidence:

- `lib/src/collect/item_index.rs` allocates item and module IDs before body lowering.
- `lib/src/collect/resolver.rs` owns canonical item paths, imports, exports, module aliases, scoped aliases, and reverse-name tables.
- `lib/src/lower/resolution.rs` consumes resolver output; `semantic_identity_audit::lowering_does_not_perform_global_resolution_fallbacks` rejects old lowering-side module-prefix, dependency, and prelude fallback resolution patterns.
- `semantic_identity_audit::alias_handling_does_not_clone_hir_semantic_owners` rejects alias/prelude cloning of HIR semantic owners.

Finding:

Collection and resolver tables are the phase boundary for global item identity. Lowering consumes resolver/service results instead of reconstructing global aliases or cloning canonical definitions.

## 3. Type Context And Semantic Types

Verdict: Pass.

Source evidence:

- `lib/src/type_context/mod.rs` defines interned `Ty` values and `TypeContext` storage keyed by `TypeId`.
- `lib/src/hir/type_ids.rs` records resolved-HIR type locations against `TypeId` sidecars.
- `lib/src/mir/mod.rs` carries MIR projection and backend metadata through `TypeId`, including `MirDropGlue`, `MirProjectionResolution`, instance declarations, extern declarations, and layout fields.
- `lib/src/products/type_table.rs` serializes product artifact types through a portable product-local type table and re-interning boundary.
- `semantic_identity_audit::mir_projection_resolution_metadata_uses_type_ids` rejects structural `Type` in MIR projection metadata and codegen projection keys.

Finding:

`TypeId`/`TypeContext` are authoritative across resolved HIR sidecars, MIR/backend metadata, product artifacts, and downstream consumers. Structural `Type` remains as compatibility/conversion payloads and for local/source-level type expressions where appropriate.

## 4. Lowerer Decomposition

Verdict: Pass.

Source evidence:

- `lib/src/lower/services.rs` defines explicit service wrappers for inference, scope, items, diagnostics, modules, prelude, resolver, dependency resolvers, and constraints.
- `lib/src/lower/mod.rs` stores those service wrappers rather than raw engine/scope/item/diagnostic/module/prelude/resolver backing types.
- `lib/src/lower/body_context.rs` owns per-body state such as current function, owner, generic owner, impl bounds, unsafe state, local ID allocation, and scoped body-local scope.
- `lib/src/lower/session.rs` owns dependency registration, dependency body lowering, dependency diagnostics, and loaded-prelude policy.
- `semantic_identity_audit` covers service-owned core phase state, context-owned body state, session-owned dependency/prelude policy, provider-owned module source cache, and service-owned trait conformance policy.

Finding:

Lowering still has a central coordinator, but the master-audit blocker category of one raw god object owning every mutable phase subsystem has been replaced with explicit services and body/session contexts.

## 5. Trait And Method Selection Service

Verdict: Pass.

Source evidence:

- `lib/src/selection/service.rs` centralizes concrete, required-trait, unary operator, current-trait, bound, index, unresolved generic, and deref method selection.
- `lib/src/selection/matching.rs` no longer exposes receiver-display-string matching helpers; `semantic_identity_audit::selection_matching_does_not_branch_on_receiver_display_strings` rejects their reintroduction.
- `lib/src/lower/control_flow/secondary.rs` consumes `SelectedMethod` outputs with substituted parameter and return types instead of rediscovering method definitions.
- `lib/src/mir/identity.rs` carries selected method backend metadata, builtin-index guards, receiver adjustment, and selected target facts for backend consumers.

Finding:

Selection is service-owned for production dispatch. Downstream phases consume selected identities, selected method metadata, or MIR callables rather than reselecting from receiver display strings or method names.

## 6. MIR As Backend Boundary

Verdict: Pass.

Source evidence:

- `lib/src/lib.rs` builds MIR from monomorphized HIR, takes MIR instance bodies, prunes unreachable instances against MIR bodies, runs borrow checking and MIR agreement, and calls MIR codegen.
- `lib/src/codegen/mod.rs` exposes `compile_program_from_mir` and routes runtime body lowering through `mir_llvm::compile_program`.
- `lib/src/codegen/metadata.rs` builds backend selection metadata from `MirProgram`, not from `MonomorphizedProgram` HIR maps.
- `lib/src/codegen/mir_llvm/terminator.rs` resolves drop glue through `backend_metadata.drop_glue_backend_symbol(place_ty)` instead of trait/method-name rediscovery.
- `semantic_identity_audit` rejects production HIR body codegen modules, HIR function body emission from instance records, program-declaration setup from `MonomorphizedProgram`, and codegen-side trait/member lookup helpers.

Finding:

The active executable backend boundary is MIR plus MIR backend metadata. Backend symbols are link/codegen metadata, and unsupported cleanup/unwind paths remain explicit future runtime coverage rather than hidden frontend semantic reconstruction.

## 7. Monomorphization Instances

Verdict: Pass.

Source evidence:

- `lib/src/mono/registry.rs` defines `InstanceOrigin` with canonical `DefId`s, `InstanceKey` as origin plus `Vec<TypeId>`, and `InstanceSymbols` as separated source/backend symbol metadata.
- `InstanceRecord` keeps declaration/signature metadata separate from executable pre-MIR bodies; executable bodies are held in `PreMirInstanceBodies` and drained into MIR instance bodies.
- `lib/src/dce.rs` prunes over `MirInstanceBodies` and follows `MirCallable::Instance`, `MirCallable::Function`, and selected method instance IDs.
- `semantic_identity_audit::instance_body_backend_boundary_is_mir_native` and `semantic_identity_audit::instance_records_do_not_own_executable_hir_bodies` guard the MIR-native instance body boundary.

Finding:

Instance identity is `(InstanceOrigin, Vec<TypeId>)` with explicit `InstanceId`s. DCE and codegen consume MIR instance bodies and explicit instance/callable edges rather than name/backend-symbol reachability.

## 8. Crate And Artifact Interface Split

Verdict: Pass.

Source evidence:

- `lib/src/crate_artifact/types.rs` exposes `ArtifactCrateInterface` as `DefId`-keyed `Product*Interface` rows for functions, structs, enums, traits, impls, and externs, with canonical/root-export maps as capability metadata.
- `lib/src/crate_system/extern_store.rs` separates `ExternCrateMetadata`, `ExternBodyProviders`, and `ExternCrateLink` records.
- `ExternBodyProviders` keys generic functions, trait defaults, and generic impls by `DefId`; compatibility name helpers are lookup views over ID-keyed provider maps.
- `lib/src/crate_system/context.rs` exposes dependency metadata/body/link/source capabilities through `CrateContext` and `ExternCrateRef` helpers.
- `semantic_identity_audit` guards ID-keyed HIR-free artifact interfaces, explicit cross-crate body providers, no phase branching on dependency storage mode, and provider-neutral prelude/root export capabilities.

Finding:

Compiler phases consume dependency metadata, body, link, source, root-export, and prelude-export capabilities. Storage-mode details are hidden behind provider surfaces; remaining HIR-shaped body values are explicit provider payloads for downstream instantiation, not metadata-only interface authority.

## 9. Parser, Macro, And Module Loader Cleanup

Verdict: Pass.

Source evidence:

- `lib/src/parser/mod.rs` parses caller-provided source text/tokens and includes tests proving parser-only source parsing does not perform module/file IO.
- `lib/src/source_loader/mod.rs` owns filesystem, virtual, and artifact-backed source loading and module graph construction.
- `lib/src/macro_expansion/context.rs` and `lib/src/macro_expansion/source_map.rs` carry explicit macro config, traces, generated source IDs, and captured/generated token origins.
- `lib/src/lower/module_context.rs` resolves modules against preloaded `SourceModuleSet`/provider state rather than probing the filesystem.

Finding:

Parser IO is separated from parsing, macro expansion has explicit origin/config state, and source/module loading supports filesystem, virtual, and artifact-backed providers without lowerer-owned probing.

## 10. Formatter And Trivia Separation

Verdict: Pass.

Source evidence:

- `lib/src/fmt/mod.rs` defines explicit `FormatInput` and `FormatContext` state and routes formatting through `format(FormatInput)`.
- `lib/src/fmt/trivia.rs` stores formatter-side trivia outside semantic AST declarations.
- `lib/src/ast/tree.rs` semantic AST declarations do not carry formatter-only comment/trivia preservation fields.
- `rock/src/main.rs` uses the formatter API for the `format` subcommand.

Finding:

Formatting uses explicit formatter input/context and formatter-owned trivia data. No process-global formatter state or semantic-AST trivia ownership was found in the audited path.

## 11. Borrowck Indexed Dataflow

Verdict: Pass.

Source evidence:

- `lib/src/ids.rs` defines `LoanId`, `MovePathId`, and `PlacePathId`.
- `lib/src/mir/borrowck/location.rs` defines typed MIR locations.
- `lib/src/mir/borrowck/paths.rs` stores place and move path tables keyed by `PlacePathId` and `MovePathId`.
- `lib/src/mir/dataflow/bitset.rs` defines typed `BitSet<I>` and `LocalSet`.
- `lib/src/mir/dataflow/analyses/loans.rs` stores loan data and active loan state keyed by `LoanId`.
- `lib/src/mir/borrowck/mod.rs` builds initialization, borrow, place-path, loan-table, loan-location, and liveness indexes before checking.

Finding:

Borrow checking uses indexed loan/place/move/location tables and typed bitsets as semantic state. Structural `Place` values in loan diagnostics are diagnostic/source-report metadata, not the active identity model.

## Acceptance Assessment

The `new_lang2-4ku.12` acceptance criteria are satisfied:

- This report covers all 11 tracks.
- Each track cites direct source evidence.
- No remaining master-audit blockers were found in the current source.
- Remaining strings, readable names, backend symbols, HIR compatibility views, and structural `Type` values are classified as display, diagnostics, serialization, artifact compatibility, local lexical metadata, tests, or explicit conversion/provider boundaries.
- Fresh verification evidence is provided by `cargo test -p rock-lib semantic_identity_audit`, which passed all 27 architecture audit tests on 2026-06-18, plus the final quality gates recorded under `new_lang2-4ku.13`.

## Prior Blocker Resolution

The 2026-06-17 source audit found blockers in tracks 1-8. Those blockers were resolved by the reopened master-audit beads completed afterward:

- `new_lang2-4ku.7.4`: DCE reachability now follows MIR/instance ID edges instead of display/backend names.
- `new_lang2-4ku.7.6`: executable instance bodies are carried as MIR instance bodies; `InstanceRecord` no longer owns executable HIR bodies.
- `new_lang2-4ku.8.1`: artifact interfaces expose ID-keyed `Product*Interface` rows.
- `new_lang2-4ku.8.2`: cross-crate bodies flow through explicit provider capabilities.
- `new_lang2-4ku.8.3`: phase code queries dependency provider capabilities instead of branching on storage mode.
- `new_lang2-4ku.8.6`: prelude and root exports are provider-neutral alias-to-ID capabilities with explicit stdlib prelude injection policy.

The previous overclosed-bead trace is therefore obsolete and is superseded by this final source audit.
