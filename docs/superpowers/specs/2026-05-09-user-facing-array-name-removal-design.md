# User-Facing Array Name Removal Design

**Date:** 2026-05-09
**Status:** Draft for review
**Scope:** Remove `Array` as a magic user-facing type name and synthetic compiler owner while keeping internal fixed-size arrays as `Type::Array(Box<Type>, usize)` and slices as `Type::Slice(Box<Type>)`. Also stop accepting bare `[T]` as a standalone user-facing type or impl target; source-level slices should be written through references such as `&[T]` and `&mut [T]`.

## Purpose

`Array` should be available as an ordinary user-defined name. Today the compiler still treats the string `"Array"` as a special stand-in for slice-backed and fixed-array-like behavior in several later phases. That makes a user-defined `struct Array` ambiguous and keeps builtin slice/array identity tied to a fake item name.

This design removes the magic `Array` name completely. Source code should express builtin sequence types through real syntax:

- `&[T]` or `&mut [T]` for borrowed slices
- `[T; N]` for fixed-size arrays

Internally, the compiler may keep the enum variant name `Type::Array` for `[T; N]` and `Type::Slice` for the unsized slice shape `[T]` that appears behind references. The removal target is the magic source/compiler string `"Array"`, not the internal variants.

## Goals

- Free `Array` as a normal user-facing type/identifier name.
- Remove parser/lower/mono/codegen behavior that treats `"Array"` as builtin slice or array identity.
- Stop accepting bare `[T]` as a standalone source type or impl target; require borrowed slice spellings such as `&[T]` or `&mut [T]`.
- Keep `Type::Array(Box<Type>, usize)` for fixed-size `[T; N]`.
- Keep `Type::Slice(Box<Type>)` for the internal unsized slice shape `[T]`, normally reached through references such as `&[T]` or `&mut [T]`.
- Replace mono instance ownership for builtin slice/fixed-array behavior with structural builtin owner identity, not fake `DefId`s or fake paths.
- Preserve existing behavior for `&[T]`, `&mut [T]`, and `[T; N]` dispatch, indexing, and intrinsics.

## Non-Goals

- Do not rename internal `Type::Array` to `Type::FixedArray` in this slice.
- Do not redesign the whole `Type` model or introduce semantic `Ty` IDs.
- Do not remove `Type::Slice` or `Type::Array`.
- Do not change fixed-size array syntax `[T; N]`.
- Do not add first-class unsized local/value support for bare `[T]`; this slice should reject or avoid user-facing bare `[T]` except as the referent inside reference forms like `&[T]`.
- Do not make a user-defined `Array` type automatically behave like slices or fixed-size arrays.
- Do not complete the broader canonical identity hardening in this slice, except where structural builtin owner identity is required to remove fake `Array` ownership.

## Current Shape

The clean tree already distinguishes real sequence types internally:

- `lib/src/types/mod.rs::Type::Slice(Box<Type>)` represents the internal slice shape `[T]`; user-facing slice values should normally be references such as `&[T]` or `&mut [T]`.
- `lib/src/types/mod.rs::Type::Array(Box<Type>, usize)` represents `[T; N]`.
- `lib/src/parser/items/parse_type.rs::parse_array_type` currently parses bare `[T]` into `ParseType::Slice` and `[T; N]` into `ParseType::Array`. This slice should constrain that behavior so bare `[T]` is only valid where the parser is building the referent of a reference type like `&[T]` or `&mut [T]`; standalone type annotations and impl targets should use borrowed slices.

The problem is not the internal variants. The problem is remaining string-based magic around `"Array"`, including examples found in the current tree:

- `lib/src/collect/context.rs::impl_type_info` maps slice/array impls to `"Array"`.
- `lib/src/lower/collect/traits.rs::impl_type_info` maps slice/array impls to `"Array"`.
- `lib/src/lower/types_helpers/helpers.rs` uses fallback lookup lists such as `"[U8]"` and `"Array"`.
- `lib/src/infer/solve.rs` maps `Type::Slice(_)` to `"Array"` for impl existence checks.
- `lib/src/mono/mod.rs` uses `"Array"` in receiver lookup and synthetic builtin owner tests.
- `lib/src/codegen/mod.rs` and `lib/src/codegen/types.rs` use `"Array"` as a method lookup type name.
- `lib/src/codegen/intrinsics.rs` treats `Type::Struct(name, _) if name == "Array"` as builtin slice storage.
- Tests under `lib/src/mono/*` still seed or construct impls with `type_name: "Array"` as a generic slice stand-in.

## Design

### User-Facing Name Rule

`Array` is just a name. If a program defines:

```text
struct Array
```

then `Array` refers to that user item and nothing else. It must not be interpreted as builtin slice storage, a fixed-size array family, or a fallback impl owner.

Compiler builtin sequence behavior must come only from real type syntax and semantic type shapes:

```text
&[T]
&mut [T]
[T; N]
```

### Bare Slice Source Rule

Bare `[T]` is an internal unsized slice shape, not a standalone user-facing value type for this language slice. Source code should not write bare `[T]` as a variable type, parameter type, return type, or impl target. Instead, source code should write borrowed slice forms:

```text
&[T]
&mut [T]
```

The parser may still build `ParseType::Slice` internally while parsing those reference forms, and lowering may still produce `Type::Slice` as the referent under `Type::Reference`. The important rule is that bare `[T]` does not escape as an accepted source-level type by itself.

### Internal Type Rule

The internal type enum remains:

```rust
Type::Slice(Box<Type>)
Type::Array(Box<Type>, usize)
```

`Type::Array` is acceptable because it is internal Rust code, not a user-facing reserved name. It means fixed-size arrays only.

### Impl Owner Identity

Builtin slice and fixed-array method/trait impls should not have a fake named owner. They should use structural identity.

The current HIR shape already has:

```rust
pub enum HirImplOwner {
    Named(String),
    BuiltinSlice,
}
```

This slice should keep or extend that concept, but it must stop feeding `"Array"` through lookup and mono as if it were a real owner path. In mono registry keys, builtin impl methods should use an explicit owner enum rather than `DefId` for fake names:

```rust
pub enum InstanceImplOwner {
    Named(DefId),
    BuiltinSlice,
}

pub enum InstanceOrigin {
    Function(DefId),
    ImplMethod {
        owner: InstanceImplOwner,
        method: String,
    },
}
```

If fixed-size arrays later require identity distinct from slices, the enum can grow a `BuiltinFixedArray` variant. This slice does not need to add it unless the current behavior requires distinguishing them for correctness.

### Collection And Lowering

`collect` and `lower` may still compute display/backend names for impls, but those names must not be `"Array"` for builtin sequence impl ownership.

For impls over borrowed slices and fixed-size arrays, HIR should use structural builtin ownership rather than `Array` strings. For example, source impls like `impl Trait for &[T]` and `impl Trait for &mut [T]` should lower through a builtin slice owner. Fixed-size array impls like `impl Trait for [T; 4]`, if supported, should lower through structural builtin ownership too.

HIR should use `HirImplOwner::BuiltinSlice` or a future structural builtin owner, not `HirImplOwner::Named("Array")` and not `type_name: "Array"` as semantic identity.

Where a string is still required for diagnostics or backend symbols, use type spelling derived from the actual type shape, preferably including the reference when the source-level type is borrowed, such as:

- `&[T]`
- `&[U8]`
- `[T; 4]`

Those strings are labels, not lookup fallbacks or canonical owners.

### Method Lookup, Inference, And Dispatch

Method and trait lookup should match builtin sequence impls by type shape:

- `Type::Slice(_)`
- `Type::Array(_, _)`
- references whose inner type is one of those shapes, where existing dispatch supports that

Lookup should not add `"Array"` as a candidate. For example, replace fallback lists like:

```rust
vec![recv_ty.to_string(), "[U8]".to_string(), "Array".to_string()]
```

with shape-aware matching that checks builtin impl owners and receiver argument patterns directly. If a string candidate is still needed for an exact concrete borrowed slice spelling, use `recv_ty.to_string()` or an explicit spelling like `"&[U8]"`, not `"Array"`.

### Codegen And Intrinsics

Codegen must not treat a user-defined `struct Array` as builtin storage. Remove branches like:

```rust
Type::Struct(name, _) if name == "Array" => { ... }
```

Builtin storage behavior should be keyed on actual semantic types:

- `Type::Slice(_)`
- `Type::Array(_, _)`
- supported references to those types

Intrinsics such as `ArrayLen`, `MakeArr`, `BorrowSlice`, and `ArrPtr` may keep their current names for now because they are intrinsic names, not type names. Their typing/codegen must not inspect `Type::Struct("Array", ...)` as magic.

## Testing Strategy

### New Regression Tests

Add a test proving `Array` is user-definable and not builtin magic:

```text
struct Array

main = ->
    0
```

If constructor syntax exists for empty structs, extend the test to construct/use it. If not, declaration success is enough for this slice.

Add a test proving a user-defined `Array` does not receive slice methods automatically:

```text
struct Array

main = ->
    # any attempted slice-only method on Array should fail resolution
    0
```

Use the compiler's existing error-test style if available. If no error-test harness exists, add a unit-level lowering/codegen regression that ensures `Type::Struct("Array", ...)` is not accepted by slice-specific branches.

Add a parser/lower regression for deprecated bare slice impl targets:

```text
impl Show for [T]
    show = -> "bad"
```

Expected behavior: reject this form or otherwise fail before it becomes a valid builtin impl. The accepted source-level form should be explicit about borrowing, such as `impl Show for &[T]`.

Add a parser/lower regression for the deprecated `Array` sugar:

```text
impl Show for Array T
    show = -> "bad"
```

Expected behavior: this is treated as a normal named generic type owner if the language supports that syntax for named generics, or rejected by parser/lower if not. It must not produce `HirImplOwner::BuiltinSlice`.

Add preservation coverage for the accepted borrowed-slice form:

```text
impl Show for &[T]
    show = -> "ok"
```

Expected behavior: this lowers through structural builtin slice ownership and dispatches for borrowed slice receivers.

### Preservation Tests

Keep and update tests for:

- `impl Show for &[T]` dispatching on borrowed slices.
- `impl Trait for &[T]` and `impl Trait for &mut [T]` custom methods on borrowed slices.
- fixed-size `[T; N]` type formatting and indexing.
- array literals and `~ArrayLen` behavior if still implemented via slice-like runtime values.
- object/artifact-backed generic slice impl dispatch.

Update mono tests that currently use `type_name: "Array"` so they use structural owners plus real borrowed slice spelling such as `"&[T]"` or `"&[U8]"`. Internal direct-HIR construction may still use `Type::Slice` for the referent inside `Type::Reference`.

## Success Criteria

- Grepping production compiler code for `"Array"` finds no magic type-name/owner handling.
- Remaining `"Array"` occurrences are limited to:
  - docs/specs/plans describing the removed behavior,
  - test source strings that define a user type named `Array`,
  - intrinsic names such as `ArrayLen` where the word is part of an intrinsic API, not a type name,
  - AST debug labels for array literals if they describe syntax rather than a type owner.
- A user-defined `Array` type is accepted as an ordinary item.
- Bare `[T]` is not accepted as a standalone user-facing type or impl target.
- `&[T]`, `&mut [T]`, and `[T; N]` behavior remains covered and passing.
- Mono instance identity for builtin sequence impls no longer requires synthetic `DefId`s generated from `"Array"` or `"BuiltinSlice"` fake item paths.

## Risks

- Method lookup currently uses string candidate lists in several phases. Removing `"Array"` without adding shape-aware matching where needed can break slice dispatch.
- Codegen still contains late method lookup and trait dispatch behavior. It must be updated consistently with lower and mono, not left as a separate `"Array"` fallback.
- Some tests may encode the old fake owner by constructing HIR directly. Those tests need semantic updates rather than mechanical string replacement.
- Artifacts may contain serialized HIR impls whose `type_name` is `"Array"` from older runs. This slice does not guarantee compatibility with stale artifacts produced before this cleanup.

## Implementation Order

1. Add regressions proving `Array` is user-definable and not builtin magic.
2. Add regressions proving bare `[T]` is not accepted as a standalone source type or impl target, while `&[T]` and `&mut [T]` remain accepted.
3. Remove parser/lower treatment of `Array T` as builtin slice sugar.
4. Remove `"Array"` receiver fallback candidates from lower/infer/mono/codegen and replace them with shape-aware builtin impl matching.
5. Introduce structural mono impl owner identity for builtin slice impls.
6. Remove `Type::Struct("Array", ...)` intrinsic/codegen special cases.
7. Update direct-HIR tests to use borrowed slice spelling such as `&[T]` or `&[U8]`, `Type::Slice`, or `Type::Array(_, N)` instead of `"Array"` as magic.
8. Run focused regressions and `cargo test -p rock-lib`.
