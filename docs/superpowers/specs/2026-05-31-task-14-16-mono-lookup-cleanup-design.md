# Task 14-16 Monomorphizer Lookup Cleanup Design

## Goal

Remove remaining string-keyed monomorphizer lookup tables from semantic control flow where callable identity, specialization, or instance reachability should be driven by canonical IDs.

This is a focused follow-up to Roadmap Tasks 14-16. The callable universe, instance call edges, and instance reachability DCE are already complete for their scoped slices; this pass narrows the remaining monomorphization compatibility paths that can still choose behavior by source names, backend symbols, or string-derived signature keys.

## Current State

`MonomorphizedProgram.instances` is the backend callable universe, generic call edges can use `HirVarTarget::Instance`, and pipeline DCE prunes instances through explicit instance/function/method/object-backed edges. The remaining audit item is that parts of `lib/src/mono/*` still use string-keyed maps during specialization and processing.

Important examples to audit include:

- `generic_functions` and `concrete_functions` lookups in mono processing.
- Method and impl lookup paths that iterate string-keyed method maps while selected IDs are available.
- Object-backed signature keys that may choose semantic behavior instead of acting as import compatibility metadata.
- Variable type maps, which may be acceptable if they remain local lexical scope metadata and do not select callable identities.

## Scope

In scope:

- Audit `lib/src/mono/mod.rs`, `lib/src/mono/process.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/external.rs`, `lib/src/mono/specialize.rs`, and `lib/src/mono/registry.rs` for string-keyed semantic lookup.
- Replace semantic callable lookup with `DefId`, selected method IDs, `InstanceId`, or instance registry queries where canonical identity is already present.
- Add regression tests proving canonical targets win over same-name or backend-symbol-compatible entries.
- Keep compatibility fallback paths only when the input genuinely lacks canonical target data, and make those paths explicit in naming and tests.
- Update `master-audit-checklist.md` and the ordered roadmap after verified implementation.

Out of scope:

- Removing backend symbols or source names from `InstanceRecord`; they remain object/link/display metadata.
- Removing syntax-local variable names or local lexical scope maps unless they select callable identity.
- Deleting legacy non-pipeline HIR/name DCE helpers; that is the next Task 14-16 cleanup item.
- Changing instance body representation away from `HirFunction`; that is a later MIR/codegen boundary item.
- Product artifact schema changes unless a narrow validation test requires documenting existing behavior.

## Design

The implementation should proceed by classifying every remaining mono string map or string lookup into one of three buckets.

1. Semantic lookup to replace: any lookup that chooses a callable, impl method, generic specialization, or instance by source name, qualified name, backend symbol, or derived signature key when a canonical ID is available.
2. Compatibility fallback to isolate: any lookup needed for unresolved legacy inputs, imported compatibility views, or tests where no ID exists yet. These paths should have explicit names and coverage proving ID-backed paths bypass them.
3. Metadata to keep: backend symbols, display names, diagnostics, artifact callable names, and lexical local variable names that do not choose callable identity.

For function calls and function values, the preferred path is to consume `HirVarTarget::Function`, `HirVarTarget::Extern`, or `HirVarTarget::Instance` directly. Name-based `HirExprKind::Var` handling should only cover unresolved/local-compatible expressions and should not override resolved targets.

For methods, selected method targets and `DefId` identities should drive specialization before any receiver/name matching. Receiver/name matching should be retained only for compatibility cases without selected targets and should be documented by tests as fallback behavior.

For object-backed and artifact-backed records, object symbols and callable names may remain as link/interface metadata. If a signature key is still needed to import a declaration, it must not become the semantic identity for choosing between current-crate callable bodies or generic specializations.

## Data Flow

Resolved HIR or selected method metadata provides canonical IDs. Monomorphization maps those IDs to instance keys and registered records. Specialized calls receive `HirVarTarget::Instance` when they target an instance, or keep direct `Function`/`Extern` targets for non-instance callables. Codegen resolves instances through the instance registry and backend-symbol maps, not by rediscovering call targets from emitted symbol strings.

## Error Handling

Missing canonical target data should either remain an explicit compatibility fallback or produce a clear compiler diagnostic/error path consistent with existing mono behavior. The implementation should not add panics to user-facing compilation paths for recoverable missing target data.

If a fallback path is retained, tests should make the fallback boundary obvious: resolved targets must not use it, while unresolved compatibility inputs continue to behave as before.

## Testing

Focused tests should cover:

- Generic function calls specialize by `DefId`/`InstanceId` instead of name.
- Generic function values specialize by canonical target instead of name.
- Static impl methods and receiver methods with same names do not collide.
- Same-name functions or methods across modules/artifacts do not select the wrong specialization.
- Object-backed declarations still link through backend symbols without using those symbols as monomorphization call targets.
- Existing Task 14-16 regression filters remain green.

Verification should include focused `cargo test -p rock-lib ...` commands for the touched mono/DCE/codegen areas, `cargo fmt --all --check`, `git diff --check`, and a final `cargo test -p rock-lib` after the implementation is complete.

## Success Criteria

- String-keyed mono maps are no longer part of semantic callable selection where canonical IDs are available.
- Any remaining string-keyed mono lookup is clearly compatibility, local-scope, diagnostic, display, artifact interface, or backend/link metadata.
- The Monomorphization Instances audit item for removing string-keyed monomorphizer semantic lookup can be checked off with evidence.
- No broader Task 14-16 items are claimed complete: legacy non-pipeline DCE helper retirement and instance body representation cleanup remain separate follow-ups.
