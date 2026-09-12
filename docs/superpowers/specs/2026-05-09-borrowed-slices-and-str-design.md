# Borrowed Slices And Str Design

**Date:** 2026-05-09
**Status:** Draft for review
**Scope:** Fix two pre-canonical-identity type-system issues: reject bare slice source types like `[T]` outside references, and separate text strings from byte slices by modeling `Str` like Rust's `str` while keeping Rock's uppercase `Str` spelling.

## Purpose

The compiler is moving toward canonical identities, but two type-surface issues should be fixed first.

First, the deprecated `Array` user-facing model has been replaced by Rust-style borrowed slices and fixed arrays, but the compiler can still parse and partially accept bare `[T]`. That is wrong for this stage: source-level slice views should be written behind references, such as `&[T]` or `&mut [T]`. Fixed arrays remain `[T; N]`.

Second, ordinary `[U8]` currently carries string behavior in several places. That conflates byte slices with text. The desired model follows Rust's split between `&[u8]` and `&str`, while preserving Rock's uppercase builtin spelling: `&[U8]` is bytes, and `&Str` is text.

## Goals

- Reject bare `[T]` as a standalone source type.
- Keep `&[T]`, `&mut [T]`, and `[T; N]` valid.
- Keep internal `Type::Slice(Box<Type>)` for the unsized slice shape and `Type::Array(Box<Type>, usize)` for fixed arrays.
- Make `Str` a distinct unsized string slice type, not an alias for `[U8]` or `&[U8]`.
- Require string slice values to be referenced as `&Str`, matching Rust's `&str` model while retaining Rock's uppercase spelling.
- Lower string literals to `&Str`.
- Remove string-specific method, trait, indexing, and display behavior from ordinary `[U8]` and `&[U8]`.
- Keep byte slices byte-oriented: indexing `[U8]` or `[U8; N]` returns `U8`.

## Non-Goals

- Do not rename internal `Type::Array` in this change.
- Do not implement full DST support beyond the existing slice/string reference representation needed here.
- Do not introduce compatibility aliases such as bare `Str -> &Str` or `Str -> &[U8]`.
- Do not preserve deprecated `Array` source behavior.
- Do not complete canonical definition identity hardening in this slice.
- Do not implement UTF-8 character indexing for `Str` yet.

## Source Type Rules

### Slices And Fixed Arrays

The accepted user-facing forms are:

```text
&[T]
&mut [T]
[T; N]
```

Bare `[T]` remains an internal unsized slice referent. It can appear syntactically while parsing the pointee of `&[T]` or `&mut [T]`, but it must not be accepted as a standalone parameter type, return type, local type, struct field type, or impl target.

Lowering should continue to own this validation. The parser can still produce `ParseType::Slice`, and lowering should accept it only when lowering the pointee of a reference.

### String Slices

`Str` is a distinct unsized string slice type. Source APIs should write borrowed string slices as:

```text
&Str
```

Bare `Str` should follow the same unsized-source rule as bare `[T]`: it is not accepted as a standalone value type. Unlike the old behavior, bare `Str` must not lower to `&[U8]` or `&Str` implicitly.

String literals lower to `&Str`.

## Internal Type Model

The existing internal enum can stay structurally close to its current shape:

```rust
Type::Slice(Box<Type>)
Type::Array(Box<Type>, usize)
Type::Str
Type::Reference { mutable, inner }
```

The semantic meanings are:

- `Type::Slice(T)` is the unsized slice referent `[T]`.
- `Type::Array(T, N)` is the sized fixed array `[T; N]`.
- `Type::Str` is the unsized string slice referent `Str`.
- `Type::Reference { inner: Type::Slice(T), .. }` is `&[T]` or `&mut [T]`.
- `Type::Reference { inner: Type::Str, .. }` is `&Str` or `&mut Str`.

`Type::Str` should be treated more like an unsized referent than a standalone copyable fat-pointer value. Copy/reference behavior should belong to `&Str`, not bare `Str`.

## Compiler Phase Changes

### Parser

Keep parsing `[T]` as `ParseType::Slice` and `[T; N]` as `ParseType::Array`. No broad parser redesign is required.

If parser tests currently call bare `[T]` an accepted type, update them to assert only the parse shape, not source-level validity, or move validity coverage to lowering tests.

### Collection And Lowering

Lowering should validate unsized source contexts:

- Reject `ParseType::Slice` unless `allow_bare_slice` is enabled because it is being lowered as a reference pointee.
- Add the same context rule for `ParseTypeInner` named `Str`: lower bare `Str` to an error unless it is being lowered as a reference pointee.
- Lower `&Str` to `Type::Reference { mutable: false, inner: Type::Str }`.
- Lower `&mut Str` similarly with `mutable: true`. This only establishes type syntax; it does not add string mutation APIs.
- Lower string literals to `&Str`.

Both the main lowerer and collection-time type lowering need the same rules because this repo has both paths.

### Method And Trait Lookup

Remove the special equivalence between `[U8]` and strings.

Lookup should no longer add `[U8]` as a string fallback for slices or fixed arrays. `&[U8]` can still match byte-slice impls by its actual shape, but it should not inherit text behavior.

String-specific impls should be keyed by `&Str` or `Str`'s structural referent as appropriate for the existing impl lookup machinery. If lookup needs display strings, those strings are labels, not aliases.

### Indexing

Byte slices and fixed byte arrays should index to bytes:

```text
&[U8]  [I64] -> U8
[U8; N][I64] -> U8
```

`Str` should not support integer indexing for now. This follows Rust's UTF-8 rule: a byte offset is not necessarily a character boundary, and the compiler should not pretend string indexing returns a `Char` until UTF-8-aware APIs exist.

Existing string literal bounds checks tied to `[U8]` indexing should be removed or deferred to future explicit UTF-8/string APIs.

### Codegen And ABI

`&Str` can reuse the existing fat slice runtime representation `{ ptr, len }`, since string literals and string functions already operate on pointer-plus-length values.

Codegen should key behavior on semantic type shapes:

- `Type::Reference { inner: Type::Str, .. }` for borrowed string slices.
- `Type::Reference { inner: Type::Slice(_), .. }` for borrowed slices.
- `Type::Array(_, _)` for fixed arrays.

It should not use `[U8]` as a proxy for text. Intrinsics such as `~ArrPtr` and `~ArrayLen` may continue to operate on fat slice-like values if they are extended to accept `&Str`, but they must not make all `&[U8]` values string-like.

## Stdlib Surface

String APIs move to `&Str`:

```text
String::from_str: &Str -> String
string_len: &Str -> I64
str_raw_ptr: &Str -> *U8
string_concat: &Str -> &Str -> &Str
```

`Show` and `Eq` string behavior should be implemented for `&Str`, not `&[U8]`.

Byte-slice APIs should remain byte-oriented. If an existing function is really a text function but is named or typed as `[U8]`, update it to `&Str`. If a function manipulates raw bytes, keep it on `&[U8]` and avoid string semantics.

Substring and character APIs need care. Because `Str` is UTF-8 text, any function returning `&Str` from offsets should either prove it preserves UTF-8 boundaries or be deferred. Concatenating two existing `&Str` values may return `&Str` because concatenation preserves UTF-8 validity. A byte-oriented substring version can operate on `&[U8]` instead. This change should not add unchecked `&Str` substring APIs.

## Diagnostics

Use structured lowering diagnostics already present in this area.

Bare `[T]` should produce a message equivalent to:

```text
bare slice type [T] must be written behind a reference, such as &[T] or &mut [T]
```

Bare `Str` should produce a parallel message:

```text
bare string slice type Str must be written behind a reference, such as &Str
```

Rejected string indexing should make the UTF-8 reason explicit:

```text
cannot index Str by integer; string slices are UTF-8 text, use an explicit string or byte API
```

## Testing Strategy

### Focused Unit Tests

- Lowering rejects bare `[I64]`.
- Lowering accepts `&[I64]` and `&mut [I64]`.
- Lowering preserves `[I64; 4]` as `Type::Array(I64, 4)`.
- Lowering rejects bare `Str`.
- Lowering accepts `&Str` as a reference to `Type::Str`.
- String literals lower to `&Str`.
- Builtin index output for `Type::Slice(U8)` and `Type::Array(U8, N)` is `U8`, not `Char`.
- Builtin index output for `Type::Str` is absent.

### Integration Tests

- `String::from_str "hello"` still compiles and runs.
- `Show`/`println` on string literals uses the `&Str` impl.
- Equality on string literals uses the `&Str` impl.
- A byte slice `&[U8]` does not use the string `Show` impl.
- Indexing `&[U8]` returns `U8`.
- Indexing `&Str` is rejected with the UTF-8 diagnostic.
- Bare `[T]` in a user signature is rejected.
- Bare `Str` in a user signature is rejected.
- `&[T]`, `&mut [T]`, and `[T; N]` behavior remains covered.

## Success Criteria

- No compiler path lowers bare source `Str` to `&[U8]` or `&Str` implicitly.
- No compiler path treats `[U8]` or `&[U8]` as text for method lookup, trait lookup, indexing, or codegen.
- String literals have type `&Str`.
- Byte-slice indexing returns `U8`.
- String indexing is rejected until UTF-8-aware APIs are added.
- Existing fixed-array and borrowed-slice behavior continues to work.

## Risks

- The compiler currently has several string and `[U8]` special cases across lowering, inference, mono, and codegen. Missing one can preserve old behavior accidentally.
- Some stdlib functions currently typed as `&[U8]` are text APIs. Moving them to `&Str` may expose ownership/lifetime assumptions around returned heap-backed string slices.
- Rejecting `Str` indexing can break examples that relied on byte-like string indexing. That is intentional unless and until explicit UTF-8 APIs are designed.
- `Type::Str` currently behaves like a fat value in parts of the compiler. Treating it consistently as an unsized referent may require small ABI helper adjustments.
