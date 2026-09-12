# Task 5 Final Compliance Findings and Repair Design

## Status

Complete as of 2026-07-14. This document preserves the historical findings and
approved repair design; all seven findings below are resolved. The authoritative
F1-F64 closure evidence and final gate results are recorded in
`2026-07-11-task-5-authoritative-closure-ledger.md`.

## Findings

### 1. Raw method `DefId` calls can bypass selected authority

Accepted-HIR validation rejects an explicit `HirCallTarget::Function` that
names a method, but a call can instead carry a
`HirExprKind::ResolvedVar(HirVarTarget::Function(method_id))` callee and no call
target. Mono repairs that shape through `zero_substitution_method_instance`,
which selects an instance using only the method ID.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_method_authorities_in_expr`
- `lib/src/mono/process.rs`, `zero_substitution_method_instance`
- `lib/src/mono/process.rs`, `process_expr`

### 2. Accepted `Try` receiver metadata is not authoritative

Accepted-HIR validation checks `branch_method` and the from-residual static
target but does not validate `branch_self_receiver`. Mono repairs a missing
mode through `lookup_self_receiver`. Its trait path scans impls and accepts the
first matching member ID instead of consulting the selected trait declaration.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_method_authorities_in_expr`
- `lib/src/mono/process.rs`, `lookup_self_receiver`
- `lib/src/mono/process.rs`, `process_expr`

### 3. Artifact receiver patterns lack semantic binding validation

Artifact loading verifies that every impl has one receiver-pattern row and
that every row names an impl. It does not verify that the pattern binds exactly
the generic parameters declared by that impl. Missing bindings, extra
bindings, and bindings owned by another definition can survive loading and
fail later during mono.

Relevant code:

- `lib/src/crate_artifact/load.rs`, `validate_product_method_authority_maps`
- `lib/src/hir/mod.rs`, `validate_method_authorities`

### 4. Mono suppresses overlapping-impl ambiguity

Drop, static trait method, and trait method materialization discard
lower-specificity matches before checking ambiguity. The approved policy is
that any two distinct typed impl matches are ambiguous. Rock does not gain an
implicit specialization rule as part of Task 5.

Relevant code:

- `lib/src/mono/methods.rs`, `monomorphize_drop_for_type`
- `lib/src/mono/methods.rs`, `monomorphize_static_method_call`
- `lib/src/mono/methods.rs`, `monomorphize_trait_method_call`

### 5. Mono retains method-ID repair compatibility behavior

`zero_substitution_method_instance` searches all instance records and returns
the first zero-substitution impl method or trait default with the requested
method ID. It has no exact impl, trait, dispatch, or substitution authority.

Relevant code:

- `lib/src/mono/process.rs`, `zero_substitution_method_instance`
- `lib/src/mono/process.rs`, `zero_substitution_callable_instance`

### 6. MIR constructs stale method-to-instance mappings

`callable_instances_by_def_id_for_program` indexes impl methods and trait
defaults by method `DefId`, even though `callable_key_for_def_id` rejects method
IDs before consulting the map. This is dead method-aware MIR infrastructure
and permits silent map overwrites.

Relevant code:

- `lib/src/mir/builder/mod.rs`, `callable_instances_by_def_id_for_program`
- `lib/src/mir/builder/mod.rs`, `callable_key_for_def_id`

### 7. Regression coverage is incomplete

There are no direct tests for raw method callees without call targets, invalid
Try receiver authority, malformed artifact receiver-pattern bindings,
repair.

## Approved Architecture

### Phase-typed HIR

Use distinct types for unresolved lowering HIR and accepted executable HIR.
Method-selection fields may remain optional only in the unresolved form. The
accepted form must represent method calls, static methods, and Try protocol
calls with required authority fields.

The migration should be narrow:

1. Keep syntax-directed lowering and inference on the existing unresolved
   shapes.
2. Introduce accepted expression/call authority types at the final inference
   boundary.
3. Convert unresolved HIR to accepted HIR only after strict validation.
4. Make mono consume accepted HIR, not unresolved optional fields.
5. Keep built-in indexing explicit through `BuiltinIndex`; do not fabricate
   impl or method IDs.

The accepted representation must make these states unrepresentable:

- A non-builtin method call without `HirMethodCallTarget`.
- A static method call represented as an ordinary method `DefId`.
- A Try branch without selected method authority and receiver mode.
- A Try from-residual call without selected static authority.
- A callable method value represented as an unqualified function `DefId`.

### Artifact Boundary

Artifacts continue to serialize stable product identities. Loading must verify
receiver-pattern generic ownership and exact binding coverage before producing
accepted HIR. The `effective_trait_methods` relation remains authoritative and
must not be reconstructed from names.

### Monomorphization

Mono consumes only accepted authority and exact substitutions. Remove method
ID repair. For generated obligations and trait/static dispatch, gather distinct
typed impl matches and diagnose ambiguity before selecting an implementation.
No specificity-based specialization is introduced.

### MIR and DCE

MIR receives only materialized `InstanceId` method edges. Its ordinary
function map must index only `InstanceOrigin::Function`. DCE follows resolved
MIR callable keys and must remain independent of method names, method IDs, and
receiver reconstruction.

## Error Handling

Malformed source HIR and artifacts must fail with structured diagnostics before
mono. Mono must diagnose generated-obligation ambiguity or missing exact
materialization. MIR retains invariant panics only as a final defense against
an impossible accepted-HIR violation.

## Verification

Implementation requires red-green regression tests for every finding, then:

```text
cargo fmt --all --check
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib
```

Task 5 can be marked complete only after the phase-typed boundary is in use,
all repair paths above are removed, all new tests pass, and a final CodeGraph
residue audit finds no production rediscovery path.
