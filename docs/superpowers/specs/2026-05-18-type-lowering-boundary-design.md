# Type Lowering Boundary Design

**Date:** 2026-05-18
**Status:** Written for user review

## Purpose

Roadmap Task 8 extracts parsed type syntax conversion out of the monolithic `Lowerer` while keeping the existing ID-backed structural `Type` representation. This is a boundary and ownership slice, not the `Ty` or type-context migration planned for Task 9.

The compiler currently has two parsed-type lowering implementations:

- `Lowerer` lowers parsed types while lowering bodies, annotations, impl receiver types, and trait-related signatures.
- `CollectContext` lowers parsed types while collecting headers, declarations, where clauses, and signatures.

Both implementations encode similar rules for builtins, slices, nominal lookup, generics, and associated-type projections. Keeping them separate risks drift and keeps parsed type lowering owned by phase contexts that already have too many responsibilities.

## Current State

`lib/src/lower/types.rs` implements `Lowerer::lower_parse_type` and `Lowerer::lower_parse_type_inner`. It uses `Lowerer` state directly for diagnostics, current module prefix, current trait, generic owner and parameter state, resolver-backed nominal lookup, import/module-local aliases, and suffix fallback behavior.

`lib/src/collect/context.rs` has a second implementation of `lower_parse_type` and `lower_parse_type_inner`. It uses collection state directly for diagnostics, current module prefix, current trait, generic state, nominal lookup, and where-clause/header collection support.

The two implementations are not equivalent in every lookup path. `Lowerer` has newer resolver-aware helpers for nominal and trait lookup, while collection has older direct map paths plus collection-specific alias data. Task 8 should make the shared policy explicit without widening into a full name-resolution rewrite.

## Target Architecture

Introduce a dedicated parsed-type lowering service that owns conversion from `ast::ParseType` / `ast::ParseTypeInner` to semantic `Type` values.

The service should be stateless or near-stateless per call:

- It receives a parsed type node.
- It receives a mutable context adapter that exposes only the operations type lowering needs.
- It returns a `Type`.
- It emits diagnostics through the adapter.

The boundary can live in a new focused module such as `lib/src/type_lowering.rs`, or under `lib/src/lower/type_lowering.rs` if keeping it close to existing lowering code is less disruptive. The important boundary is the API shape: parsed-type lowering logic should not be an inherent behavior of `Lowerer` or `CollectContext`.

## Context Adapter

Add a small trait for the operations needed by parsed-type lowering. The exact names can be chosen during implementation, but the adapter should cover these responsibilities:

- Emit type-lowering diagnostics with the current span behavior of the caller.
- Return the current module prefix, if any.
- Return the current trait name, if lowering `Self::Assoc`.
- Look up structs, enums, and traits by source or resolved name using the caller's existing lookup behavior.
- Resolve current generic parameters by name, including existing implicit single-uppercase generic creation where that is currently supported.
- Expose trait generic parameter count and associated type IDs through cloned or borrowed `HirTrait` data.

The first implementation should prefer simple APIs over lifetime-heavy abstractions. Returning cloned `HirStruct`, `HirEnum`, or `HirTrait` values is acceptable in this slice if it keeps the boundary small and avoids broad borrow conflicts.

## Data Flow

Collection flow:

```text
CollectContext caller
    -> context adapter for collection state
    -> TypeLowerer lowers ast::ParseType
    -> Type returned to header/declaration collection
    -> diagnostics appended to CollectContext errors
```

Lowering flow:

```text
Lowerer caller
    -> context adapter for lowering state
    -> TypeLowerer lowers ast::ParseType
    -> Type returned to body/signature/impl lowering
    -> diagnostics appended to Lowerer errors
```

Existing call sites may continue calling `lower_parse_type` and `lower_parse_type_inner` during the transition. Those methods should become thin adapters that delegate to the shared service rather than owning conversion logic.

## Compatibility Requirements

Task 8 should preserve current user-visible behavior, including:

- builtin type names and aliases such as `I64`, `i64`, `Int`, `F64`, and `Float`
- `Unit`, tuple, function, reference, pointer, slice, array, and `Str` handling
- bare slice rejection unless the slice appears behind a reference or raw pointer
- bare `Str` rejection unless it appears behind a reference or raw pointer
- current-module nominal type lookup
- resolver/import/module-local alias lookup where each caller currently supports it
- suffix fallback behavior where it already exists
- generic parameter lookup by canonical `GenericParamId`
- implicit single-uppercase generic creation where it currently exists
- associated type projection lowering using trait `DefId`, `AssociatedTypeKey`, projection base type, and trait args

Behavioral cleanup should be limited to removing duplicated policy. Changes to ambiguity handling, suffix fallback removal, diagnostic wording, or `Ty`/interning belong to later tasks unless required to preserve existing tests.

## Implementation Shape

The implementation should proceed in small steps:

1. Add the shared type-lowering service and context adapter trait.
2. Implement the adapter for `Lowerer`.
3. Convert `Lowerer::lower_parse_type` and `Lowerer::lower_parse_type_inner` into delegating wrappers.
4. Implement the adapter for `CollectContext`.
5. Replace collection's duplicated parsed-type lowering implementation with delegating wrappers.
6. Add focused tests around shared behavior and known differences between collection and lowering paths.

This keeps the first slice reviewable and avoids pulling broader lowerer decomposition into the type-lowering boundary work.

## Non-Goals

- Do not introduce a `Ty` layer or make `TypeId` authoritative in this slice.
- Do not intern types.
- Do not redesign product artifact type serialization or remapping.
- Do not remove all string alias compatibility maps.
- Do not remove suffix fallback behavior unless a focused test proves it is already dead or actively wrong.
- Do not move type facts such as copy semantics, layout-facing facts, display formatting, or builtin index behavior into services yet; that is later type-context work.
- Do not complete the broader source/path/name resolution extraction from `Lowerer`.

## Testing

Required focused coverage:

- Existing `lower::types` tests continue to pass through the shared service.
- Collection and lowering both reject bare slices and bare `Str` consistently.
- Borrowed and raw-pointer slice/`Str` types continue to lower without diagnostics.
- Nominal struct and enum lowering uses canonical `DefId` identity and preserves resolver-preferred lookup.
- Generic parameter lowering preserves owner/index identities and existing implicit generic behavior.
- Associated type projection lowering preserves trait IDs, associated type IDs, base type lowering, and generated trait args.
- Where-clause and generic bound tests continue to cover collection-side use.
- Artifact-loaded and same-name type declaration tests continue to cover dependency/current-crate lookup behavior.

Verification commands should include the smallest relevant focused tests first, then:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

## Completion Criteria

This slice is complete when parsed type syntax conversion is implemented once behind a dedicated type-lowering boundary and both `CollectContext` and `Lowerer` use that boundary. The compiler should still emit structural `Type` values, and Task 9 should remain the first task that decides whether to introduce an interned or non-interned `Ty` / type-context model.
