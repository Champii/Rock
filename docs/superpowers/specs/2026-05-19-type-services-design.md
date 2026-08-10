# Type Services Design

**Date:** 2026-05-19
**Status:** Written for user review

## Purpose

Roadmap Task 10 moves semantic type facts out of ad hoc `Type` helper methods and duplicated phase-local logic. Task 9 added the interned `Ty` / `TypeContext` scaffold, but structural `Type` remains the compiler phase-boundary representation until Task 11.

The Task 10 slice should make type facts explicit services while preserving current behavior and avoiding a `TypeId` phase-boundary migration.

## Current State

`lib/src/types/mod.rs` owns both type structure and semantic facts:

- numeric category checks such as `is_integer`, `is_float`, and `is_numeric`
- concreteness checks used by lowering/codegen decisions
- builtin indexing output and support checks
- conservative copyability and reference-containment checks
- display formatting through `fmt::Display for Type`

Projection normalization is also duplicated in later phases. `Lowerer` has projection resolution logic under `lib/src/lower/types_helpers/helpers.rs`, while codegen has similar logic in `lib/src/codegen/types.rs`. Both paths know how to resolve associated type projections through impl-associated types and builtin `Index` output behavior.

Codegen also owns LLVM type lowering, but some of its type-shape questions are semantic facts rather than LLVM mechanics. Examples include slice/fat-pointer recognition and projection-normalized shape checks.

## Target Architecture

Add a `type_services` module that owns explicit semantic services over structural `Type` for now:

- `type_services::facts`: pure structural facts such as numeric categories, concreteness, copyability, reference containment, and builtin index output.
- `type_services::projection`: projection normalization behind a provider trait that can be implemented by lowering and codegen contexts.
- `type_services::display`: diagnostic/display formatting for `Type`, with `Display for Type` delegating during the transition.
- `type_services::layout`: backend-facing shape facts such as slice, string slice, reference, pointer, and fat-pointer classification. LLVM type construction stays in codegen.

This keeps `Type` as data and makes semantic queries explicit. The services should consume `&Type` and return structural `Type` or value facts because HIR, inference, artifacts, mono, MIR, and codegen still store structural `Type` in this slice.

## Service Responsibilities

### Pure Type Facts

`type_services::facts` should provide a small stateless service, such as `TypeFacts`, with methods for:

- `is_integer`, `is_signed_integer`, `is_unsigned_integer`, `is_float`, and `is_numeric`
- `is_type_var`
- `is_concrete`
- `builtin_index_output` and `has_builtin_index_impl`
- `is_copy`
- `contains_reference`

Initial implementations should match the existing `Type` helper semantics exactly. Compatibility wrappers on `Type` may remain during Task 10, but they should delegate to `TypeFacts` so facts have one implementation.

### Projection Normalization

`type_services::projection` should centralize associated type projection normalization. The service should handle:

- recursive normalization of nested type structures
- impl-associated type lookup through a provider trait
- generic substitution for impl type generics and trait generics
- builtin `Index` projection output fallback through `TypeFacts::builtin_index_output`
- preserving unresolved projections structurally when no impl or builtin output applies

Lowering and codegen may need separate provider adapters because their available data differs. The shared service should own the algorithm, not the phase-specific storage.

### Display Formatting

`type_services::display` should own type rendering policy for diagnostics and compatibility formatting. The existing `fmt::Display for Type` should delegate to the service instead of carrying formatting policy inline.

Task 10 should not redesign user-facing type names. Formatting output should remain byte-for-byte compatible with existing tests unless a test explicitly proves a stale or incorrect display behavior.

Backend symbol names are not diagnostic formatting. Task 10 may introduce the display boundary, but backend symbol naming should only move if the call site is already type-display policy and not instance/backend identity policy.

### Layout-Facing Shape Facts

`type_services::layout` should expose projection-normalized type-shape predicates used by backend-facing code, while keeping LLVM-specific type construction in `lib/src/codegen/types.rs`.

The initial target is shape classification, not a full layout engine. Examples:

- slice payload shape
- string slice shape
- fat pointer shape for references and pointers to slices or `Str`
- array/slice element type facts needed by builtin indexing or codegen shape checks

## Compatibility Boundary

Task 10 does not migrate stored type fields from `Type` to `TypeId`. The services should be written so they can later gain `TypeContext` / `TypeId` entry points, but current call sites should stay on structural `Type`.

Keeping compatibility wrappers on `Type` is acceptable in this slice because many call sites still use structural `Type`; the important part is that wrapper implementations delegate to the new services and no longer own fact logic.

## Non-Goals

- Do not migrate HIR, inference, product artifacts, mono, MIR, or codegen stored fields from `Type` to `TypeId`.
- Do not remove `fmt::Display for Type`; delegate it to the display service instead.
- Do not change product artifact serialization.
- Do not rewrite trait/method selection. Task 12 owns the broader selection service.
- Do not turn codegen LLVM type construction into a general layout engine in this slice.
- Do not change current copyability, indexing, projection, or display behavior except where an existing regression test proves a bug.

## Implementation Order

The implementation should be staged to keep each commit reviewable:

1. Add `type_services::facts` and move `Type` helper logic behind it, preserving `Type` wrapper methods.
2. Migrate direct call sites that are cheap and low-risk, especially inference, MIR builder, borrowck liveness, and builtin index checks.
3. Add `type_services::display` and make `Display for Type` delegate to it while preserving existing output.
4. Add `type_services::projection` and migrate the duplicated lower/codegen projection normalization algorithm through provider adapters.
5. Add `type_services::layout` for projection-normalized backend shape facts and migrate codegen shape predicates without moving LLVM lowering.
6. Update roadmap and audit trackers to mark Task 10 complete and keep Task 11 as the phase-boundary `TypeId` migration.

## Testing

Required focused coverage:

- Numeric, copyability, concreteness, reference containment, and builtin index facts match current `Type` behavior.
- Existing `Type` wrapper methods delegate to the same facts as direct service calls.
- Projection normalization resolves impl-associated types and builtin `Index` output through the shared service.
- Unresolved projections remain structural and preserve `trait_id`, `AssociatedTypeKey`, base type, and trait args.
- Display output for primitive, nominal, generic, projection, function, tuple, slice, array, reference, pointer, never, unit, `Str`, `Char`, `TypeVar`, and error types remains compatible.
- Layout shape predicates recognize slice/fat-pointer/string/reference cases after projection normalization.

Verification should include focused service tests, existing type identity tests, indexing/deref/operator integration tests, projection tests, display tests, codegen type tests, and the full `cargo test -p rock-lib` suite.

## Completion Criteria

This slice is complete when semantic type facts live behind explicit services, current behavior is preserved by focused tests and the full `rock-lib` suite, duplicated projection normalization has one shared algorithm, and roadmap/audit docs clearly separate completed Task 10 service extraction from Task 11 `TypeId` phase-boundary migration.
