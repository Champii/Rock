# Strict Inference Redesign

## Goal

Fix the remaining compiler correctness bugs exposed by strict unresolved-type finalization by separating declared generics from inference variables.

The compiler should preserve these language rules:

- integer literals default to `I64`
- float literals default to `F64`
- valid generic helpers remain polymorphic
- unresolved non-literal compiler state is always a hard error

## Why

The current lowering/inference model mixes two different concepts:

- declared generics from explicit signatures, represented as `Type::Generic(...)`
- fresh unknowns created during body inference, represented as `Type::TypeVar(...)`

That works for simple cases but breaks under strict finalization because callback-heavy helpers and intrinsic wrappers are inferred against rigid declared generics instead of fresh inference variables. The result is under-solved bodies, stale aliases, and a long tail of hidden fallback dependencies.

The fix is to make body inference operate only on fresh type variables and treat declared generic names as metadata that is re-applied after inference.

## Problem Summary

Strict finalization exposed valid stdlib helpers still carrying unresolved non-literal type variables.

Representative hotspots:

- `stdlib/fp.rk` pipe operator
- `stdlib/option.rk` higher-order helpers
- `stdlib/result.rk` higher-order helpers
- `stdlib/string.rk` intrinsic wrappers
- `stdlib/mem.rk` intrinsic wrappers
- `stdlib/vec.rk` methods depending on those helpers

The repeated failures pointed to architectural problems rather than isolated bugs.

## Core Invariant

During body lowering and inference, all unknowns must be `TypeVar`s.

Declared generic names must not act as mutable unknowns during inference.

After inference:

- surviving header-linked type variables are mapped back to declared generic names when a signature exists
- surviving inferred-generic type variables get synthesized names when no signature exists
- remaining non-literal unresolved type variables are compiler errors

## Design

### 1. Signature Instantiation For Body Inference

When lowering a function or method with an explicit signature:

- collect the declared generic names from the signature
- create a fresh type-variable substitution for each declared generic name
- apply that substitution to the body-facing parameter and return types
- record which fresh type variables correspond to which declared generic names

This yields two views of the same function:

- an inference view using fresh `TypeVar`s
- a declaration view using stable generic names

The inference view is used for body lowering and unification.

### 2. Canonical Function Ownership

There should be one canonical `HirFunction` per function body.

Exported names must not own stale cloned `HirFunction` values. Instead:

- canonical functions live under their true lowered identity
- export aliases map names to canonical functions
- when body lowering/generalization/finalization updates a function, aliases observe the updated canonical function automatically

This removes a class of stale-pre-generalization bugs for crate helpers.

### 3. Stable Header Variable Tracking

Header-linked type-variable tracking must use the same canonical identity throughout lowering, inference, and finalization.

That means:

- qualified crate functions must keep their tracked header vars under the qualified name
- impl methods must use the same qualified key in both lowering-side and inference-side generalization
- explicit-signature functions must record instantiated fresh header vars so later generalization can map them back to declared generic names

This removes the current mismatch where valid generic functions are skipped by generalization simply because later phases look them up under a different key.

### 4. Generalization Uses Declared Metadata First

Generalization should stop guessing when a function already had an explicit generic signature.

If declared generic metadata exists:

- reuse the declared generic names and ordering
- map surviving linked type vars back to those names
- preserve composite generic structure such as `Array<T>`, `Option<T>`, `Result<T, E>`, `T -> U`, `*T`, and references

If declared generic metadata does not exist:

- use the existing inferred-generic naming strategy (`T`, `U`, `V`, ...)

This keeps explicit signatures stable and avoids accidental renaming or partial rigidification.

### 5. Call-Site Instantiation Remains Local

Direct calls to already-generalized named functions still need per-call instantiation.

That logic should stay local to call lowering:

- instantiate named generic callee types to fresh type vars per call
- do not broaden `unify` so that `Type::Generic(...)` behaves like a mutable unknown globally

This keeps the distinction between declared polymorphism and local inference intact.

### 6. Strict Finalization Remains Simple

Finalization order remains:

1. apply literal defaults (`I64` for integer literals, `F64` for float literals)
2. finalize/generalize valid polymorphic functions
3. reject all remaining unresolved non-literal compiler state

With the redesigned invariant, strict finalization should no longer need special cases for stdlib helpers.

## Scope

Included in this redesign pass:

- explicit-signature instantiation redesign
- canonical/export alias cleanup for functions
- stable header-var tracking for crate functions and impl methods
- generalization that prefers declared generic metadata when available
- preserving strict unresolved-type errors and literal defaulting

Not included:

- new fallback behavior
- language syntax changes for generics
- broad monomorphization redesign beyond what this invariant requires

## Expected Outcomes

After implementation:

- `stdlib/fp.rk` pipe operator typechecks under strict finalization
- `stdlib/option.rk` and `stdlib/result.rk` higher-order helpers typecheck under strict finalization
- `stdlib/string.rk` and `stdlib/mem.rk` intrinsic wrappers typecheck without fallback
- `stdlib/vec.rk` methods stop inheriting unresolved generic state from those helpers
- integer and float literal defaulting tests keep passing
- unresolved non-literal inference tests keep failing correctly
- no compiler phase silently repairs unresolved non-literal types with `I64`

## Testing Strategy

Required verification should cover both preserved behavior and the previously failing helper families:

- literal defaulting integration tests
- unresolved non-literal inference regression tests
- helper regression tests for generic helper inference under strict finalization
- focused stdlib-backed tests that exercise `|>`, option/result combinators, and intrinsic wrappers

Implementation should proceed incrementally with failing tests first for each newly isolated root cause.
