# Type Context Scaffold Design

**Date:** 2026-05-18
**Status:** Written for user review

## Purpose

Roadmap Task 9 decides the long-term type representation direction and starts the migration without destabilizing HIR, inference, artifacts, mono, or codegen in one broad rewrite.

The chosen long-term direction is an interned semantic `Ty` representation with authoritative `TypeId` identity inside a new type context. The first implementation slice should add the type-context scaffold and prove identity semantics while keeping existing phase boundaries on structural `Type`.

## Decision

Use an interned semantic type context as the long-term model.

`TypeId` should identify interned canonical semantic type nodes in a `TypeContext`. `Ty` should represent semantic type structure in the context, roughly mirroring today's structural `Type` variants at first. Equal `Ty` values intern to the same `TypeId`; distinct semantic identities intern to distinct `TypeId`s.

This is better long term than continuing with structural `Type` alone because it gives the compiler one canonical semantic type identity and gives later services a place to own type facts, projection normalization, display, and layout-facing behavior.

This is also better than a non-interned `Ty` rename because it solves identity and sharing instead of only changing names.

## Current State

`TypeId` already exists in `lib/src/ids.rs`, but it is scaffolding rather than an authoritative type identity.

`lib/src/types/mod.rs` defines `Type`, which currently carries many responsibilities:

- semantic structure for primitive, nominal, generic, projection, inference, reference, pointer, function, tuple, slice, and array types
- equality and hashing through structural derived implementations
- artifact serialization and remapping
- inference substitution helpers
- generic substitution and type-var-to-generic replacement
- ad hoc facts such as copyability and builtin index behavior
- display formatting used by diagnostics and backend names

Task 8 moved parsed type syntax conversion behind a shared type-lowering boundary. That made type ownership cleaner, but `Type` is still the representation used everywhere after lowering.

## Target Architecture

Add a new type-context module, likely `lib/src/type_context/mod.rs`, containing:

- `Ty`: an internable semantic type node.
- `TypeContext`: storage for interned `Ty` values keyed by `TypeId`.
- `intern_ty(Ty) -> TypeId`: returns an existing ID for equal `Ty` or allocates a new one.
- `ty(TypeId) -> &Ty`: resolves an ID back to its interned semantic node.
- compatibility conversions between existing `Type` and interned `Ty`/`TypeId`.

The initial `Ty` should intentionally mirror the existing `Type` structure closely. This keeps the first slice about identity and storage, not semantic redesign. Variants should cover the existing type shapes, including `TypeVar(TypeVarId)`, `Generic(GenericParamId)`, nominal `DefId`s, and associated projection identity.

## Compatibility Boundary

This slice should not migrate HIR, inference, product artifacts, MIR, mono, or codegen fields from `Type` to `TypeId`.

Instead, `Type` remains the compatibility and serialization representation at phase boundaries. `TypeContext` becomes authoritative only for values interned inside the new context. Conversion helpers should make this explicit:

- `TypeContext::intern_type(&Type) -> TypeId`
- `TypeContext::type_for(TypeId) -> Type`
- optional helper methods for converting `Type` to `Ty` and `Ty` to `Type`

This avoids a half migration where structural `Type` and `TypeId` both claim to be globally authoritative.

## Roadmap Update

The ordered roadmap should be clarified before or during this slice:

- Task 9 is the decision plus scaffold task: choose interned `Ty`/`TypeId`, add `type_context`, and prove conversion/equality/substitution compatibility.
- Task 10 remains type facts and semantic query extraction, but should depend on the Task 9 scaffold.
- Add a later explicit task to migrate selected HIR, inference, and artifact-facing phase boundaries from structural `Type` to `TypeId` after the context and services are stable.
- Keep backend/MIR migration later so codegen and MIR consume context-aware type identities only after frontend/inference representation is stable.

## Identity Requirements

The scaffold must preserve the semantic identity guarantees already established for structural `Type`:

- Same primitive, function, tuple, array, reference, pointer, slice, and `Str`/unit/never/error structure interns to the same `TypeId`.
- Different nominal `DefId`s intern to different `TypeId`s even when display names match.
- Generic identity is owner/index based through `GenericParamId`.
- Projection identity includes projection base type, trait `DefId`, `AssociatedTypeKey`, and trait args.
- `TypeVarId` identity remains typed and distinct from `TypeId`.

The scaffold should not use context-free display strings as semantic keys.

## Non-Goals

- Do not migrate HIR fields from `Type` to `TypeId` in this slice.
- Do not change product artifact serialization to store `TypeId`.
- Do not change inference unification to operate on `TypeId` yet.
- Do not move copyability, builtin index behavior, projection normalization, display formatting, or layout-facing facts yet.
- Do not change mono, MIR, or codegen type inputs.
- Do not make `TypeId` globally authoritative outside the new `TypeContext` until later phase-boundary migration work.

## Testing

Required coverage:

- Interning the same primitive type twice returns the same `TypeId`.
- Interning equal composite types such as function, tuple, array, reference, pointer, and slice types returns the same `TypeId`.
- Nominal types with different `DefId`s return different `TypeId`s.
- Generic types with different `GenericParamId` owners or indexes return different `TypeId`s.
- Projection types with different trait IDs, associated type IDs, base types, or trait args return different `TypeId`s.
- `Type -> TypeId -> Type` roundtrips preserve the existing structural `Type` value.
- Type variables roundtrip by `TypeVarId` without becoming `TypeId` aliases.
- Artifact/product tests continue to serialize structural `Type`, not `TypeId`.

Verification should include focused `type_context` tests, existing type identity tests, inference substitution/generalization tests, artifact projection/remapping tests, and the full `rock-lib` suite.

## Completion Criteria

This slice is complete when the repository has a committed type-context scaffold with interned `Ty` and `TypeId` identity, conversion tests proving compatibility with existing `Type`, and roadmap/audit docs that clearly separate the scaffold from later full phase-boundary migration.
