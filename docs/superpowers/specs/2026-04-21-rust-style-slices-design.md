# Rust-Style Slices And Fixed Arrays Design

## Context

The current compiler overloads `Type::Array(_)` to mean the language's only array-like type. In practice that type behaves like a slice value: codegen lowers it to a `{ptr, len}` pair, indexing uses runtime length metadata, and stdlib code uses intrinsics like `~MakeArr`, `~ArrPtr`, and `~ArrayLen` to build and inspect it.

That model was sufficient for the first round of collection and indexing work, but it breaks down once we want a more Rust-like story:

- `Vec<T>` should deref to a borrowed slice `&[T]` without storing a mirror `[T]` field
- slices and fixed arrays should be distinct concepts
- references and raw pointers to slices should be fat, while references and pointers to sized types stay thin
- generic behavior such as `Show` should live on slices and be reachable from fixed arrays through coercion, not by pretending arrays and slices are the same type

Today, the compiler cannot express that cleanly because `Type::Reference` always lowers to a thin pointer and there is no first-class distinction between slice DSTs and fixed arrays.

## Goals

- Introduce a Rust-style distinction between slice DSTs and fixed arrays.
- Make references and raw pointers metadata-aware so `&[T]` and `*[T]` are fat while sized pointees remain thin.
- Remove the need for `Vec<T>` to store a duplicate slice field solely to implement `Deref<Target = [T]>`.
- Support array-to-slice coercion so traits implemented for slices can be used by fixed arrays without const generics.
- Keep the language coherent by using explicit coercions and receiver adjustment rather than ad hoc trait forwarding.
- Preserve the current branch's `Deref` and indexing direction while replacing the temporary slice workaround.

## Non-Goals

- Implementing full user-facing const generics.
- Making user code generic over arbitrary array lengths in this project.
- Adding every Rust DST feature at once.
- Reworking unrelated collection or ownership features beyond what this model requires.
- Introducing compiler magic that makes fixed arrays directly satisfy slice trait impls without coercion.

## Decisions

### Distinct slice and fixed-array types

The compiler should split the current overloaded array representation into two concepts:

- slice DST: `[T]`
- fixed array: `[T; N]`

Internally, these should be distinct type variants rather than different interpretations of the same `Type::Array(_)` node.

The exact Rust-side type names are an implementation detail, but the model should be equivalent to:

- `Slice(T)` for unsized slice values
- `Array(T, N)` for sized fixed arrays

`N` does not need full const-generic expressiveness yet. For this refactor it only needs to support compiler-known lengths, especially:

- array literals
- layout and indexing
- borrow/coercion into slices

This keeps the type system honest without forcing the much larger const-generics project into the same change.

### Metadata-aware references and raw pointers

References and raw pointers should no longer have one uniform runtime representation.

The rule should become:

- thin for sized pointees
- fat for slice DST pointees

Concretely:

- `&I64`, `&Struct`, `&[T; N]` stay thin
- `&[T]` is fat: `(data_ptr, len)`
- `*T` stays thin when `T` is sized
- `*[T]` is fat: `(data_ptr, len)`

This is the key change that allows stdlib code such as `Vec::deref` to synthesize a real borrowed slice directly from `raw_ptr` and `raw_len`, instead of taking the address of a stored `[T]` value.

### Slice values vs slice references

The existing runtime form for the current array-like value is already close to a slice value representation: codegen emits `{ptr, len}` for the overloaded `Type::Array(_)` case.

That representation should be retained for slice values and reused for fat slice references and raw slice pointers where appropriate. The important design change is not inventing new metadata, but routing it through the right type categories:

- slices are unsized views
- fixed arrays are sized storage
- references and pointers to slices carry metadata directly

### Array-to-slice coercion instead of const-generic trait impls

To avoid requiring const generics just to preserve ergonomic generic behavior like `Show`, this design uses coercion rather than array-specific trait impl duplication.

The primary coercion is:

- `&[T; N] -> &[T]`

This should be available in the usual borrow/receiver situations so code like fixed-array method calls and trait-based formatting can flow through slice impls naturally.

This means the generic printing implementation should conceptually live on slices, not on fixed arrays. Fixed arrays should reach that behavior because they can be borrowed/coerced to slices, not because the compiler pretends arrays directly implement every slice trait.

### Receiver adjustment remains small and explicit

The branch already introduced a bounded autoderef candidate builder. This refactor should expand that idea into a small receiver-adjustment model with three explicit steps:

- peel shared references where the language already permits it
- follow `Deref::Target`
- apply array-to-slice coercion on borrowed receivers when valid

This remains:

- deterministic
- cycle-safe
- bounded

It is still not a general trait-search engine.

Trait resolution itself should remain exact once a concrete adjusted receiver type is chosen.

### `Vec<T>` owns storage, slices borrow it

`Vec<T>` should stop storing a duplicate `raw_view: [T]` field.

After this refactor, `Vec<T>` should hold only its real ownership state:

- data pointer
- length
- capacity

Then:

- `impl Deref for Vec T` returns `&[T]` by constructing a fat slice reference from `raw_ptr` and `raw_len`
- `Vec.get` returns `Option &T` using the real borrowed view semantics
- `vec[i]` continues to work through `Vec -> Deref<Target = [T]> -> Index`

This restores the intended Rust-like ownership split:

- `Vec<T>` owns the buffer
- `[T]` is a view of elements
- `&[T]` is the borrowed shared view

## Architecture

### Type-system layer

The compiler must separate slice types from fixed-array types everywhere the current `Type::Array(_)` appears.

Expected impact areas include:

- AST/lowered type conversion
- type substitution and projection resolution
- monomorphization naming and matching
- trait self-type lowering and conformance checks
- diagnostic formatting and type names

The existing special handling for `"Array"` impls will need to be replaced or split so it no longer conflates slices and fixed arrays.

### ABI and codegen layer

Codegen needs a shared notion of runtime representation for:

- slice values
- fixed arrays
- thin refs/pointers
- fat refs/pointers

That implies helper APIs for operations such as:

- build a fat slice ref from `(data_ptr, len)`
- extract `(data_ptr, len)` from a slice ref/value
- coerce `&[T; N]` to `&[T]`
- index through either a fixed array or a slice using the correct base pointer and bounds metadata

The current `llvm_type(Type::Reference { .. }) -> ptr` rule will need to become representation-aware.

### Lowering and coercion layer

The lowering pipeline must learn where coercions and receiver adjustments are legal.

Important sites:

- `&expr`
- unary `*`
- method calls
- indexing
- function arguments
- return values
- match/assignment coercion boundaries if already supported elsewhere in the language

The language rule set should remain narrow:

- autoderef
- array-to-slice coercion

No general implicit conversion system should be introduced.

### Stdlib surface layer

Stdlib code should move to the new model rather than preserving compatibility shims.

Important consequences:

- `stdlib/vec.rk` loses `raw_view`
- generic `Show` logic should target slices rather than the overloaded current array model
- `String::from_str`, `string_concat`, substring helpers, and `Str`/`[U8]` behavior must be checked against the new slice/value distinction
- `~MakeArr`, `~ArrPtr`, and `~ArrayLen` should be redefined against the real slice model, not the old overloaded one

The special string path `impl Show for [U8]` should remain, but it must be phrased in terms of true slices.

## Testing Strategy

### Type and lowering tests

Add focused unit coverage for:

- slice vs fixed-array type formatting and substitution
- coercion selection from `&[T; N]` to `&[T]`
- receiver adjustment choosing autoderef plus slice coercion candidates in the intended order

### Integration tests

Add user-visible coverage for:

- `Vec<T>` deref without a stored slice field
- `Vec[i]` continuing to work through `Deref<Target = [T]>`
- `Show` on slices
- `Show` on fixed arrays via slice coercion
- passing a fixed array where a slice reference is expected
- string-oriented `[U8]` behavior still working through the new slice model

### ABI-sensitive verification

Because this refactor changes the runtime shape of some references and pointers, add focused coverage for:

- functions returning `&[T]`
- functions accepting `&[T]`
- raw-slice-pointer creation and consumption if exposed
- index and pointer helpers that operate on slice metadata

## Migration Plan Shape

This should be implemented in phases rather than as a single patch.

Recommended sequencing:

1. Introduce separate internal slice and fixed-array types while keeping behavior as close as possible.
2. Make codegen and reference/pointer ABI representation-aware.
3. Add array-to-slice coercion and extend receiver adjustment.
4. Move stdlib `Show`, `Vec`, and string-adjacent APIs onto the new model.
5. Remove temporary compatibility paths and the `Vec.raw_view` field.

This order reduces the chance of mixing semantic changes, ABI changes, and stdlib rewrites in one step.

## Rationale

The current model solved an immediate problem but encoded two mismatches:

- arrays and slices are the same internal type
- all references are thin even when the pointee logically needs metadata

Those mismatches are exactly why `Vec` had to grow `raw_view`.

The agreed direction fixes the root cause instead of preserving the workaround. It also avoids prematurely taking on const generics by using the same practical escape hatch Rust itself relies on heavily for ergonomics: fixed arrays borrow and coerce to slices, and generic behavior lives on slices.

That gives the codebase a clearer long-term model:

- fixed arrays are storage
- slices are views
- fat references exist when metadata is required
- `Vec<T>` derefs to a real borrowed slice
- trait behavior follows real coercions rather than special compiler forwarding
