# MIR Canonical Runtime Design

## Goal

Finish roadmap Task 19 by making MIR a canonical, runtime-complete compiler boundary while preserving the current HIR-based codegen path until Task 21.

## Current State

MIR is already used by borrow checking and debug output, but it is not yet a complete executable backend boundary:

- `MirProgram` is keyed by function display names instead of canonical identities.
- `MirFunction`, closure metadata, and aggregate kinds still carry string identities.
- Function and extern references can lower to `Constant::Unit` placeholders.
- Method calls synthesize unit-valued callable temporaries and discard the selected method target.
- Enum variant construction and `match` expressions lower to placeholder unit values in MIR.
- Closure MIR records generated names but does not provide a canonical runtime identity model.
- Runtime checks such as bounds checks are still codegen-owned rather than represented as MIR requirements.
- Cast and drop modeling is partial and enough for current borrowck, but not enough for MIR codegen.

The existing HIR and monomorphization layers already provide canonical identities such as `DefId`, `FieldId`, `VariantId`, and `InstanceId`. Task 19 should make MIR consume those identities directly instead of deriving semantics from names.

## Non-Goals

- Do not move codegen to consume MIR; that remains Task 21.
- Do not redesign HIR, type inference, selection, monomorphization, or LLVM codegen as part of this task.
- Do not remove readable names from debug output; names remain useful metadata, but not identity.
- Do not preserve `Unit` placeholders for runtime semantics once a Task 19 slice owns that construct.
- Do not broaden filesystem/source loading behavior or add source IO from MIR or borrow checking.

## Architecture

Use an identity-spine-first approach. Add MIR identity and runtime requirement forms first, then migrate builder output area by area.

### MIR Identity Spine

Introduce MIR-facing identity types in or near `lib/src/mir/mod.rs` for runtime entities that need canonical equality:

- Function identity backed by `DefId` for concrete HIR functions and externs.
- Instance identity backed by `InstanceId` for monomorphized callables.
- Callable identity for function, extern, instance, closure, intrinsic, and runtime helper targets.
- Aggregate identity for structs and enum variants backed by `DefId` plus `VariantId` where applicable.
- Field and projection identity backed by `FieldId` where HIR exposes stable field identity.

Readable names should remain as display metadata on MIR functions, locals, closures, and debug dumps. Runtime equality, lookup, and call resolution must use canonical identities.

### MIR Program Shape

`MirProgram` should move from string-keyed functions toward canonical function keys. The design should keep compatibility accessors during migration so borrowck and debug-print callers can iterate functions without caring about the backing map.

`MirFunction` should expose a canonical key and an optional display name. Existing debug output can keep using display names, but tests should assert that canonical keys drive uniqueness.

### Callable Lowering

The MIR builder should lower HIR callables into explicit MIR callable values instead of placeholder units:

- `HirVarTarget::Function(DefId)` becomes a callable identity.
- `HirVarTarget::Extern(DefId)` becomes a callable identity.
- `HirVarTarget::Instance(InstanceId)` becomes a callable identity.
- `HirMethodCallTarget` lowers to a canonical method or instance target using the selected impl, trait, method, and monomorphized instance information that already exists in HIR/mono.
- Intrinsics and runtime helpers get explicit MIR identities or runtime requirement records rather than unit fallbacks.

`Terminator::Call` should point at a callable operand that retains the canonical target. Method calls should stop synthesizing unit-valued method temporaries once this slice owns method call lowering.

### Aggregates, Enums, And Matches

Struct aggregate MIR should carry the struct `DefId` and field ordering that is already resolved by HIR. `AggregateKind::Struct(String)` should be replaced or wrapped by a canonical aggregate kind.

Enum variant construction should lower to a MIR aggregate with both enum `DefId` and `VariantId`, preserving payload operands. `match` lowering should become explicit MIR control flow:

- Lower the scrutinee once.
- Read or compute a discriminant value.
- Branch through `SwitchInt` where possible.
- Access variant payloads through `Projection::Downcast` plus field projections.
- Preserve guard and arm body ordering.

Diagnostic wording and spans may change when MIR becomes more precise, but such changes must be intentional and covered by tests.

### Closures, Drops, Casts, And Runtime Checks

Closure MIR should carry canonical closure identity or a stable local MIR closure key plus capture metadata. Generated strings may remain display labels only.

Drop modeling should stay compatible with borrowck while recording enough identity/type information for future MIR codegen. Scope-exit drops should remain explicit, and new cleanup/unwind targets should be represented only when needed by a concrete runtime construct.

Casts should represent runtime-relevant casts explicitly. Pointer casts can keep their current MIR form, but numeric and other runtime casts should not degrade to a plain copy where codegen will need a different operation.

Bounds checks and other runtime helper requirements should be represented in MIR as explicit assertions, helper-call requirements, or dedicated terminators/rvalues. Codegen can continue to emit checks from HIR until Task 21, but MIR should record the same requirement for agreement tests.

## Data Flow

The intended Task 19 flow is:

```text
HIR + mono identities
    -> MIR identity/callable/aggregate/runtime requirement forms
    -> MIR builder produces canonical runtime-complete MIR
    -> borrowck and MIR debug output consume canonical MIR
    -> agreement tests compare MIR requirements with existing HIR codegen behavior
    -> Task 21 later moves codegen to consume MIR
```

MIR should consume identity data already produced by HIR, selection, and monomorphization. It should not re-resolve names or re-infer semantic targets.

## Implementation Strategy

Implement in behavior-preserving slices. Each slice should compile and pass focused tests before the next starts.

1. Add canonical MIR identity and callable data types while retaining display names for dumps.
2. Re-key or wrap `MirProgram` functions behind canonical accessors.
3. Migrate function, extern, instance, and method call lowering away from unit placeholders.
4. Migrate struct aggregate MIR to canonical struct identity and field metadata.
5. Add enum variant aggregate MIR and discriminant/downcast match lowering.
6. Add closure identity/capture representation sufficient for borrowck and future codegen.
7. Add explicit MIR runtime requirements for bounds checks, relevant casts, drops, and helper calls.
8. Add MIR/codegen agreement scaffolding while leaving codegen on HIR.
9. Remove or fail tests for any remaining placeholder `Unit` lowering in runtime-owned MIR constructs.

If a runtime area proves too large, split the implementation plan into sub-tasks under this design rather than weakening the end-state.

## Error Handling And Diagnostics

- Preserve existing successful program behavior.
- Preserve existing diagnostics unless canonical/runtime-complete MIR exposes a more precise source span or message.
- When diagnostics change intentionally, update tests to assert the new behavior.
- Do not silently lower unsupported runtime semantics to `Unit`; use focused failures during development and complete the MIR form before the slice is accepted.
- Keep span and type context available for borrowck diagnostics.

## Testing Strategy

Task 19 should include both unit-level MIR shape tests and behavior-level integration coverage.

Regression coverage should include:

- MIR function identity uniqueness by `DefId` or `InstanceId` rather than display name.
- Function, extern, instance, and method calls carrying canonical callable targets.
- Struct aggregates carrying canonical struct identity and field order.
- Enum variant construction carrying enum and variant identity plus payload operands.
- `match` lowering using discriminant branches and downcast payload access.
- Closure MIR using stable identity and capture metadata without string identity semantics.
- Bounds checks and runtime helper requirements recorded in MIR.
- Casts represented as runtime operations where codegen needs runtime conversion.
- Drop insertion and cleanup behavior preserved for borrowck.

Useful verification commands include:

```bash
cargo test -p rock-lib mir::builder
cargo test -p rock-lib mir::borrowck
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_enum -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
cargo test -p rock-lib codegen::expr
cargo fmt --all --check
cargo test -p rock-lib
git diff --check
```

## Completion Criteria

Task 19 is complete when:

- MIR function, callable, aggregate, enum, field, and runtime helper identities are canonical where HIR/mono provides canonical IDs.
- Name strings are debug metadata only, not semantic identity.
- Function, extern, instance, and method calls no longer lower to unit placeholders.
- Struct and enum aggregate MIR is runtime-complete enough for future MIR codegen.
- `match` MIR represents discriminants, branching, and payload projection explicitly.
- Closure MIR has stable identity and capture metadata suitable for borrowck and future codegen.
- Runtime checks, relevant casts, and drops are represented in MIR where future codegen needs them.
- Borrowck behavior and diagnostics remain stable except for intentional precision improvements covered by tests.
- MIR/codegen agreement scaffolding exists, while codegen still consumes HIR until Task 21.
- Focused MIR/borrowck/runtime tests, `cargo fmt --all --check`, `cargo test -p rock-lib`, and `git diff --check` pass.

## Risks

- MIR is currently used by borrowck before monomorphization/codegen, so identity changes must avoid invalidating current borrowck assumptions.
- Codegen still owns runtime behavior in Task 19, so agreement scaffolding must prevent MIR from drifting without prematurely switching backends.
- Enum matches and closures may require more HIR identity data than MIR currently records; implementation should add narrow data flow rather than re-resolving names.
- Existing debug tests or snapshots may depend on readable names; display metadata should preserve readability while canonical IDs drive semantics.
