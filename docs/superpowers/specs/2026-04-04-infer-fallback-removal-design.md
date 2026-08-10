# Infer Fallback Removal Design

## Goal

Remove silent compiler fallback-to-`I64` behavior for unresolved types while preserving explicit language-level literal defaults:

- integer literals default to `I64`
- float literals default to `F64`
- every other unresolved inference or generic hole becomes a hard compiler error

## Why

The compiler currently uses `I64` as a recovery value in multiple places. That mixes two different concerns:

- legitimate literal defaulting as a language rule
- accidental recovery from inference, monomorphization, or codegen bugs

This causes silent miscompilation and hides correctness issues that should stop compilation.

## Scope

Included in this slice:

- remove infer-stage `TypeVar -> I64` finalization
- add explicit literal defaulting for unresolved integer and float literals only
- fail infer finalization if any non-literal unresolved type variables remain
- remove monomorphization `I64` generic defaults and replace them with hard errors
- replace codegen-side unresolved-type `I64` fallbacks with hard errors
- add focused regression tests for literal defaulting and unresolved-type failures

Not included in this slice:

- a broader redesign of numeric literal typing beyond default `I64` / `F64`
- a new cross-phase diagnostic hierarchy
- unrelated operator or intrinsic cleanup

## Current Problem Areas

Known silent fallbacks identified during design:

- `lib/src/infer/engine.rs`
  - `InferenceEngine::finalize` maps unresolved `TypeVar` to `Type::I64`
- `lib/src/mono/specialize.rs`
  - missing generic substitutions default to `Type::I64`
- `lib/src/codegen/types.rs`
  - unresolved `TypeVar`, `Generic`, and `Error` lower to LLVM `i64`
- `lib/src/codegen/control_flow.rs`
  - array iteration fallback element type becomes `i64` / `Type::I64`
- `lib/src/codegen/expr/aggregates.rs`
  - array literal non-array fallback element type becomes `i64`
- `lib/src/codegen/expr/cast_assign.rs`
  - array assignment non-array fallback element type becomes `i64`

## Desired Semantics

### Literal Defaulting

Literal defaulting is an explicit language rule, not a generic unresolved-type fallback.

- unresolved integer literal type variables default to `I64`
- unresolved float literal type variables default to `F64`
- only literal-origin type variables are eligible for defaulting

Non-literal unresolved type variables must never be defaulted.

### Hard Errors

After literal defaulting:

- unresolved locals are errors
- unresolved expression result types are errors
- unresolved function param and return types are errors
- unresolved struct, enum, and extern field/signature types are errors
- unresolved generic substitutions during monomorphization are errors
- codegen must reject unresolved compiler types instead of repairing them

## Design

### 1. Track Literal-Origin Type Variables

The inference engine needs to distinguish type variables created for:

- integer literals
- float literals
- all other inference sites

The simplest design is to extend inference state with metadata keyed by type variable id. A minimal model is:

```rust
enum TypeVarOrigin {
    IntegerLiteral,
    FloatLiteral,
    Other,
}
```

Fresh type variable creation for literals should record the appropriate origin. Existing non-literal inference paths can continue using the default `Other` origin.

### 2. Split Defaulting From Finalization

`InferenceEngine::finalize` should stop converting unresolved `TypeVar` to `I64`.

Instead, infer finalization should become a two-step process:

1. apply literal defaults to unresolved literal-origin vars
2. walk finalized HIR and collect diagnostics for any remaining unresolved types

This keeps the language rule explicit and prevents it from acting as a blanket recovery mechanism.

### 3. Infer Finalization Returns Diagnostics

`infer::finalize` should continue returning `Result<HirProgram, Vec<ResolveError>>`, but now it should fail when unresolved types remain after literal defaulting.

The traversal should inspect:

- function params and returns
- impl method params and returns
- block types
- let statement types
- expression types
- lambda params
- struct fields
- extern signatures

Where spans are available, diagnostics should use them. When only container-level context is available, emit the best available span.

Example diagnostic direction:

- `unresolved type variable in expression`
- `could not infer type for local binding 'x'`
- `could not finalize function return type`

The exact wording can be refined during implementation, but diagnostics must be explicit and user-facing.

### 4. Monomorphization Becomes Strict

`lib/src/mono/specialize.rs` should stop building default `vec![Type::I64; ...]` type argument lists.

Instead:

- extraction helpers should return an error when not all generic parameters can be concretized
- the error should identify the target function or method and the missing generic inference
- partially inferred generic argument lists must not be specialized

This prevents codegen from seeing fake concrete instantiations.

### 5. Codegen Becomes Defensive

Codegen should reject unresolved compiler types instead of translating them to `i64`.

Relevant paths include:

- `llvm_type` in `lib/src/codegen/types.rs`
- array literal lowering
- array assignment lowering
- array iteration lowering

If unresolved types reach codegen, that indicates an earlier phase bug or a missed validation path. Returning `CodegenError` is the correct behavior.

### 6. Preserve Explicit Numeric Defaults

This design preserves these source-level behaviors:

- `x = 1` can still compile as `I64`
- `x = 1.0` can still compile as `F64`

But it rejects cases that were only succeeding due to accidental fallback, such as unresolved non-literal inference or unresolved generic specialization holes.

## Testing Strategy

Follow TDD for each behavior change.

### Integration Tests

Add focused integration coverage using `compile_should_fail(...)` and existing success helpers.

Required cases:

- ambiguous integer literal defaults to `I64`
- ambiguous float literal defaults to `F64`
- unresolved non-literal inference fails with an explicit error
- unresolved generic monomorphization fails with an explicit error

If implementation naturally exposes smaller unit-test seams in infer or mono helpers, add narrow unit tests there too, but the main regression coverage should remain user-visible integration tests.

### Verification Order

Run the smallest relevant failing test first, then the smallest passing subset, then expand only if needed.

Likely commands:

```bash
LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib test_name -- --exact
LLVM_SYS_180_PREFIX=/usr/lib/llvm18 cargo test -p rock-lib
```

## Success Criteria

- no targeted compiler phase silently defaults unresolved non-literal state to `I64`
- integer literals still default to `I64`
- float literals still default to `F64`
- unresolved non-literal inference produces hard compiler errors
- unresolved generic monomorphization produces hard compiler errors
- codegen does not repair unresolved types by lowering them as `i64`
- focused regression tests cover both preserved literal defaults and new hard-failure cases

## Implementation Notes

- Prefer extending existing `ResolveError` usage rather than introducing a new diagnostic type in this slice.
- Keep the change minimal and phase-local: infer owns literal defaulting and unresolved-type validation; mono owns generic specialization completeness; codegen only guards against invalid input.
- Do not preserve the old fallback paths behind flags or compatibility shims.
