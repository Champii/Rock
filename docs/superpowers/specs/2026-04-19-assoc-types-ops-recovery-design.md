# Assoc Types Ops Recovery Design

## Context

The `assoc-types-ops` worktree is partway through adding Rust-like operator traits backed by associated types. The current branch has three concrete failures:

1. `a[b]` lowers to an `Index` trait call, but the call does not reach a concrete impl before codegen.
2. Unary `*` does not dispatch through `Deref`, so `*boxed` still errors as a raw non-pointer dereference.
3. Two regressions keep the branch red:
   - a new artifact roundtrip test uses an invalid `Deref` impl body for its declared signature;
   - `()` panics in parser shorthand handling because `suffix_function_shorthand` unwraps an empty token list.

## Goals

- Make `Index::Output`-based indexing compile and run through normal trait method dispatch.
- Make unary `*` use `Deref::Target` when the operand is not already a reference or raw pointer.
- Keep `Deref` strict: impl bodies must explicitly return a reference when the signature says `&Self::Target`.
- Restore the branch to a passing `cargo test -p rock-lib` baseline.

## Non-Goals

- Do not add implicit borrow or coercion semantics for `Deref` impl bodies.
- Do not introduce special-case operator codegen paths that bypass the existing trait method pipeline.
- Do not refactor unrelated parser or trait infrastructure beyond what is needed for these failures.

## Design

### 1. Index Dispatch

Keep the existing lowering strategy where `a[b]` becomes a trait-style method call plus dereference of `Index::Output`.

The fix point is the dispatch pipeline after lowering:

- trait method lookup and/or monomorphization must resolve `MethodCall(recv, "index", args, ...)` into the concrete impl function before codegen;
- codegen should continue using the standard method naming and trait-impl lookup path rather than adding an `index`-only branch;
- associated type projections should remain resolved through the existing `resolve_projection_type` helper so the result type seen by later phases is concrete when the impl provides it.

This keeps `Index` aligned with the same dispatch model used by other operator traits instead of creating a new parallel path.

### 2. Deref Dispatch

Unary `*` should continue to behave as a normal dereference for built-in references and raw pointers. For non-pointer values, lowering should attempt trait-based `Deref` dispatch before reporting an error.

The intended behavior is:

- `*expr` on `&T` or `*T` keeps the existing fast path;
- `*expr` on a user type lowers through the `Deref` trait contract and uses `Target` to compute the result type;
- the trait impl body must still satisfy its explicit return type, so `@deref = -> @value` remains invalid when the signature is `-> &Self::Target`.

This preserves the language rule you chose: no implicit borrow insertion to satisfy `Deref` signatures.

### 3. Artifact Test Fixture

The new artifact roundtrip test should validate artifact serialization of associated types, not broaden language semantics. Its local `Deref` impl fixture therefore needs to match the declared signature exactly by returning `&@value`.

This is a test correction, not a compiler feature change.

### 4. Parser Regression

`suffix_function_shorthand` currently assumes there is at least one token inside `(...)` and unconditionally unwraps the last token. For `()`, that assumption is false, and the parser panics instead of letting unit syntax parse normally.

The fix is to guard the empty-token case and return a parse error from shorthand parsing so the surrounding parser can continue to the normal unit-expression path.

## Error Handling

- Preserve current diagnostics for real user errors such as invalid `Deref` impl return types.
- Replace the parser panic with a normal parse failure path.
- Avoid swallowing method-resolution failures; if trait dispatch still cannot find an impl, the existing structured diagnostic should surface.

## Testing Plan

Use the existing failing tests as the red/green cycle, then run the full library suite:

1. `cargo test -p rock-lib parser::items::tests::ast_validation::empty_tuple -- --exact`
2. `cargo test -p rock-lib crate_artifact::tests::test_artifact_roundtrip_preserves_associated_types -- --exact`
3. `cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact`
4. `cargo test -p rock-lib --test integration test_deref_target_type_drives_unary_deref -- --exact`
5. `cargo test -p rock-lib`

## Implementation Notes

- Prefer the smallest changes inside lowering, monomorphization, and codegen that bring `Index` and `Deref` back onto the shared trait-dispatch path.
- Do not add compatibility shims or alternate naming schemes unless the existing pipeline requires one concrete normalization fix.
- Keep new logic close to the existing operator lowering and trait dispatch code.
