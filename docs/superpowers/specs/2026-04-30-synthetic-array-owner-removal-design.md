# Synthetic Array Owner Removal Design

**Date:** 2026-04-30
**Status:** Draft for review
**Scope:** Remove the synthetic `Array` owner from impl identity and dispatch, and replace it with structural identity for slice-backed impls.

## Purpose

`Array` is currently doing two unrelated jobs: it stands in for slice-backed impl ownership, and it leaks into mono and artifact identity as if it were a real item. That is brittle. It makes identity depend on a fake name, creates alias edge cases, and obscures the fact that slice-backed impls are type-shaped, not item-shaped.

This slice removes that synthetic owner entirely. Slice-backed impls should be identified structurally from their receiver shape and trait context, while real named items continue to use real `DefId`s.

## Goals

- Remove `Array` as a synthetic owner concept.
- Remove `impl Show for Array T` parser sugar.
- Keep fixed arrays as `Type::Array` and slices as `Type::Slice` in the type system.
- Represent slice-backed impl identity structurally, not by fake item path.
- Preserve current dispatch behavior for `Show`, `Deref`, and indexing.

## Non-Goals

- Do not redesign the entire `Type` model.
- Do not change parser syntax for fixed arrays or slices beyond removing `Array T` impl sugar.
- Do not rewrite monomorphization caching beyond the identity change needed for impl ownership.
- Do not remove real `DefId`-based ownership for ordinary named structs, enums, traits, or methods.

## Current Shape

Today, slice-backed impls are still forced through a synthetic `Array` owner in several places:

- lower records slice impls with an `owner_path`
- mono resolves method origins by owner `DefId`
- object/artifact paths may serialize that owner as if it were a real item
- tests and dispatch helpers still accept `Array` as a stand-in for slice-backed impls

That works, but only because the compiler is treating a structural type category like a named item.

## Design

### Structural Impl Identity

Replace the fake owner string with a structural impl identity that describes what the impl actually is.

The identity should distinguish:

- real named item owners, via `DefId`
- builtin owners, via explicit variants

Use separate builtin variants for the receiver families we support, such as `Slice` and `FixedArray`. The origin must be stable and serializable without being a fake item name.

### HIR and Lowering

`HirImpl` should stop pretending slice-backed impls have a named owner path.

Instead, it should carry an ownership form that can express either:

- `Named(DefId)`
- `BuiltinSlice`
- `BuiltinFixedArray`

Lowering should emit the builtin slice form for impls like `impl Show for [T]` and `impl Deref for [T]`. No lowering path should synthesize `Array` as a name.

### Monomorphization

`InstanceOrigin::ImplMethod` should stop requiring a named owner `DefId` for builtin impls.

The origin should carry the explicit builtin identity instead. That lets mono keep one stable cache key per concrete instantiation without inventing a fake owner item.

### Dispatch

Method lookup should continue to match on receiver type shape, but once a builtin impl is found, it should resolve through the explicit builtin origin instead of converting the receiver into `Array`.

### Parser

Remove acceptance of `impl Show for Array T`.

The only source forms for slice-backed impls should be the real slice forms already supported by the language, such as `impl Show for [T]`.

## Migration Plan

1. Remove `Array T` parsing and parser tests.
2. Replace `owner_path` usage for slice-backed impls with structural builtin identity in HIR.
3. Update lowerer impl construction to emit builtin slice ownership.
4. Update mono instance origins and registry keys to use the explicit builtin origin.
5. Remove the synthetic `Array` branches from dispatch helpers and tests.
6. Verify artifact loading/serialization still preserves the new structural identity.

## Testing

Add or update coverage for:

- parser rejection of `impl Show for Array T`
- `impl Show for [T]` and `impl Deref for [T]` still dispatching correctly
- `Vec` methods still resolving through real item identity
- fixed array behavior remaining unchanged
- mono instance caching for slice-backed impls remaining stable across repeated calls

Verification should include:

- focused parser tests
- focused mono tests
- `cargo test -p rock-lib`

## Risks

- Artifact and mono serialization must agree on the new structural identity shape.
- If any dispatch helper still assumes `Array` is a named owner, slice-backed method resolution will fail.
- Removing the syntax may break tests and examples that still use the legacy form.

## Follow-Up

- delete any remaining `Array` owner references after the structural identity lands
- keep built-in fixed-array `Type::Array` semantics unchanged
- tighten tests so no code path can reintroduce a synthetic owner name
