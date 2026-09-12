# Master Audit Checklist

Source spec: `docs/superpowers/specs/2026-04-24-compiler-architecture-audit-design.md`

Final source audit: `docs/superpowers/plans/2026-06-17-final-master-audit-source-audit.md`

Updated: 2026-06-18

Purpose: this file records the final state of the compiler architecture master audit. It replaces the earlier living tracker whose intermediate unchecked items were useful during implementation but became stale after the reopened source-audit blockers were fixed.

## Overall Status

The master-audit implementation is complete for the scoped `new_lang2-4ku` epic. All 11 implementation tracks pass the final source audit, and the remaining open non-`new_lang2-4ku` beads are follow-up work outside this master-audit closure.

Final verification is tracked by:

- `new_lang2-4ku.12`: final source audit.
- `new_lang2-4ku.13`: final quality gates.
- `new_lang2-4ku.14`: documentation reconciliation.

## Summary

| Track | Final Status | Evidence |
| --- | --- | --- |
| 1. Identity And Arenas | Complete | `lib/src/ids.rs`, `lib/src/hir/mod.rs`, `lib/src/semantic_identity_audit.rs` |
| 2. Real Collection And Name Resolution | Complete | `lib/src/collect/item_index.rs`, `lib/src/collect/resolver.rs`, `lib/src/lower/resolution.rs` |
| 3. Type Context And Semantic Types | Complete | `lib/src/type_context/mod.rs`, `lib/src/hir/type_ids.rs`, `lib/src/mir/mod.rs`, `lib/src/products/type_table.rs` |
| 4. Lowerer Decomposition | Complete | `lib/src/lower/services.rs`, `lib/src/lower/body_context.rs`, `lib/src/lower/session.rs` |
| 5. Trait And Method Selection Service | Complete | `lib/src/selection/service.rs`, `lib/src/selection/matching.rs`, `lib/src/mir/identity.rs` |
| 6. MIR As Backend Boundary | Complete | `lib/src/lib.rs`, `lib/src/codegen/mod.rs`, `lib/src/codegen/mir_llvm/*`, `lib/src/codegen/metadata.rs` |
| 7. Monomorphization Instances | Complete | `lib/src/mono/registry.rs`, `lib/src/dce.rs`, `lib/src/mir/builder/mod.rs` |
| 8. Crate And Artifact Interface Split | Complete | `lib/src/crate_artifact/types.rs`, `lib/src/crate_system/extern_store.rs`, `lib/src/crate_system/context.rs` |
| 9. Parser, Macro, And Module Loader Cleanup | Complete | `lib/src/parser/mod.rs`, `lib/src/source_loader/mod.rs`, `lib/src/macro_expansion/*`, `lib/src/lower/module_context.rs` |
| 10. Formatter And Trivia Separation | Complete | `lib/src/fmt/mod.rs`, `lib/src/fmt/trivia.rs`, `lib/src/ast/tree.rs` |
| 11. Borrowck Indexed Dataflow | Complete | `lib/src/mir/borrowck/*`, `lib/src/mir/dataflow/*`, `lib/src/ids.rs` |

## Completion Criteria

- [x] Each compiler phase has an explicit primary boundary and input/output contract in source.
- [x] Production semantic identity uses canonical IDs, selected targets, `TypeId`, `InstanceId`, MIR callables, or provider capabilities instead of source/display names.
- [x] Names remain available for display, diagnostics, serialization, artifact compatibility, local lexical metadata, or unresolved/error recovery.
- [x] Imports, exports, prelude items, and dependency aliases point to canonical definitions/capabilities rather than cloned HIR owners.
- [x] Type lowering and downstream type consumers have `TypeId`/`TypeContext` authority where phase boundaries need stable type identity.
- [x] Trait and method selection is centralized and downstream consumers use selected target/metadata outputs.
- [x] MIR is the active executable backend boundary for codegen and instance DCE.
- [x] Monomorphization is keyed by canonical origin plus `TypeId` substitution and produces explicit `InstanceId`s.
- [x] Codegen consumes MIR, MIR backend metadata, layout/ABI data, selected method facts, runtime helpers, and link symbols rather than frontend semantic lookup.
- [x] Crate and artifact loading expose metadata, body, source, root export, prelude export, and link provider capabilities instead of dependency storage modes.
- [x] Parser/module IO, macro expansion origins, formatter trivia, and borrowck dataflow state are owned by their dedicated subsystems.

## Source Evidence

- `lib/src/semantic_identity_audit.rs` contains 27 architecture audit tests that reject the high-risk regressions found by the previous source audit.
- `lib/src/hir/mod.rs` stores top-level semantic definitions by `DefId` and documents `HirNameTables` as display/diagnostic/artifact compatibility data.
- `lib/src/lower/services.rs`, `lib/src/lower/body_context.rs`, and `lib/src/lower/session.rs` split lowerer core services, per-body state, and session dependency/prelude policy.
- `lib/src/selection/service.rs` owns dispatch selection; `lib/src/mir/identity.rs` carries selected method/backend facts forward.
- `lib/src/codegen/mod.rs` routes production runtime body emission through `compile_program_from_mir`, and `lib/src/codegen/metadata.rs` derives backend metadata from `MirProgram`.
- `lib/src/dce.rs` prunes `MirInstanceBodies` through explicit MIR callable and instance edges.
- `lib/src/crate_artifact/types.rs` exposes ID-keyed product interface rows, and `lib/src/crate_system/extern_store.rs` exposes dependency body provider capabilities.

## Remaining Work Classification

No remaining unchecked item in this file is a master-audit blocker.

Future work such as richer cleanup/unwind MIR semantics, optional generic active-loan dataflow integration, additional stdlib data structures, and broader ergonomics belongs to separate beads outside the completed `new_lang2-4ku` master-audit epic. Those items should not be used to reopen this checklist unless a new source audit finds a direct contradiction of the final audit evidence above.
