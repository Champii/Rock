# Builtin Index Design

## Goal

Make `Index` a real user-facing trait with associated `Output`, while preserving builtin indexing behavior for compiler-owned types through synthesized builtin impl support instead of a parallel legacy indexing path.

This design applies to the `assoc-types-ops` recovery work and supersedes the temporary assumption that builtin indexing can continue to rely on `HirExprKind::Index` while custom types move to trait-based operators.

## Decisions

### Public model

- `Index<Idx>` is a real trait in stdlib.
- `a[b]` is defined in terms of calling `index` and dereferencing the returned reference.
- User-defined types and builtin compiler-owned types both participate in the same language-level model.

Proposed trait shape:

```rock
trait Index Idx
    type Output
    @index: Self -> Idx -> &Self::Output
```

### Builtin types covered now

The compiler synthesizes builtin `Index` support for these receiver/index pairs:

- `Array<T>` with `Idx = I64`, `Output = T`
- `*T` with `Idx = I64`, `Output = T`
- `[U8]` with `Idx = I64`, `Output = Char`

Notes:

- `[U8]` keeps current Rock behavior of returning `Char`, not `U8`.
- Raw pointer indexing remains `unsafe`-only.
- Mutable indexing is out of scope. `IndexMut` is deferred.

### Lowering model

- `a[b]` always lowers through the trait-style operator path.
- The lowered form remains:
  - inner expression: `MethodCall(base, "index", [idx], Shared)`
  - method return type: `&<Base as Index<Idx>>::Output`
  - outer expression: `Deref(method_call)`
- The compiler must not restore builtin arrays or pointers to a separate early `HirExprKind::Index` lowering path.

This keeps the user-visible operator model uniform and avoids maintaining two competing semantics for indexing.

## Architecture

### 1. Stdlib trait declaration

Stdlib declares `Index` normally, like other traits.

The trait is user-visible and can be implemented by user-defined types.

### 2. Synthesized builtin impl registry

The compiler adds a builtin-aware implementation source for compiler-owned indexable types.

This registry is not a parallel language feature. It is the compiler-side backing for builtin impls whose bodies cannot be expressed today in pure Rock because they must produce true references into compiler-owned layouts.

Required capabilities:

- answer whether a receiver type has a builtin `Index` impl for a given index type
- provide the associated `Output` type for that impl
- provide the builtin dispatch identity used by monomorphization and codegen
- preserve current safety rules for raw pointers

The builtin registry must be queried through the same trait-resolution path used for normal impls. Code outside trait resolution should not invent its own indexability rules.

### 3. Associated-type resolution

Projection resolution for `<Receiver as Index<Idx>>::Output` must resolve builtin impls the same way it resolves user impls.

Examples:

- `<[I64] as Index<I64>>::Output => I64`
- `<*Point as Index<I64>>::Output => Point`
- `<[U8] as Index<I64>>::Output => Char`

This resolution must happen early enough that later field and method access on `a[b]` see the concrete element type instead of an unresolved projection.

### 4. Monomorphization and call dispatch

Builtin `index` methods need a stable dispatch story so the trait-method pipeline can carry them end to end.

Monomorphization must be able to recognize when a trait-method call resolves to:

- a user impl method with a normal body
- a builtin synthesized impl method backed directly by compiler codegen

Builtin dispatch must not require fake user-authored HIR bodies that manufacture references from temporary values.

### 5. MIR place support

Assignments and borrows rely on place lowering, not just value lowering.

Because `a[b]` now lowers as `Deref(MethodCall("index"))`, MIR lowering must recognize the builtin indexed place shape for supported builtin `Index` impls so these forms continue to work:

- `arr[i] = v`
- `unsafe ptr[i] = v`
- `&arr[i]`

This should be implemented as recognition of the builtin trait-lowered shape, not by reverting source lowering back to a dedicated builtin index expression.

### 6. Codegen

Codegen must compile builtin trait-dispatched `index` operations to real element-addressing logic that returns an addressable location, not a copied value.

Requirements by receiver type:

- `Array<T>`
  - compute element pointer from the fat-pointer payload
  - preserve existing bounds checks where they already exist
- `*T`
  - compute pointer offset with the existing pointer indexing semantics
  - preserve unsafe-only behavior from earlier phases
- `[U8]`
  - preserve current string-like indexing behavior returning `Char`
  - retain current bounds-check behavior for string indexing

The existing low-level addressing logic may be reused, but the entry point must be the trait-method dispatch pipeline rather than a special early HIR node.

## Safety And Semantics

### Raw pointers

- `ptr[i]` remains unsafe-only.
- The new builtin `Index` support does not make raw pointer indexing safe.
- Safety errors remain a lowering/type-check concern, not a codegen-only concern.

### `[U8]` semantics

- `[U8]` indexing returns `Char`.
- This preserves current tests and current stdlib expectations.
- This is intentionally not Rust byte-slice behavior.

### Mutable indexing

- No `IndexMut` in this change.
- If mutable indexing still works for some builtin forms during the transition, that support is considered compatibility behavior, not completion of the `Index` design.
- A real mutable indexing model is a separate design.

## Non-Goals

- Adding `IndexMut`
- Making raw pointer indexing safe
- Changing `[U8]` indexing to return `U8`
- Preserving a permanent separate builtin indexing path alongside trait lowering
- Reworking unrelated operator traits

## Testing Strategy

At minimum, implementation must cover:

- custom associated-type `Index` dispatch still works
- array reads through `arr[i]`
- array element method/field access after indexing
- `[U8]` indexing returns `Char`
- raw pointer indexing reads remain unsafe-only and still work inside `unsafe`
- projection resolution for builtin `Index::Output`

If mutable builtin indexing continues to be supported by current code paths, keep coverage for:

- `arr[i] = v`
- `unsafe ptr[i] = v`

These tests validate that MIR place lowering and codegen still understand builtin trait-lowered indexing.

## Recommended Implementation Order

1. Add stdlib `Index` trait declaration if not already present in the branch.
2. Introduce builtin `Index` impl metadata/query helpers in compiler trait resolution.
3. Teach associated-type projection resolution to resolve builtin `Index::Output`.
4. Teach monomorphization and/or method dispatch to recognize builtin synthesized `index` methods.
5. Teach MIR place lowering to recognize builtin `Deref(MethodCall("index"))` shapes.
6. Route codegen for builtin trait-dispatched `index` through existing element-addressing logic.
7. Re-run focused indexing tests, then full `cargo test -p rock-lib`.

## Rationale

This design keeps the language model honest.

Users see one `Index` concept, not one trait for custom types and another hidden builtin mechanism for arrays and pointers. The compiler still needs synthesized builtin support because compiler-owned layouts cannot currently expose reference-producing indexing bodies in pure Rock, but that support is now explicitly the backend for real trait impl semantics rather than a competing feature.
