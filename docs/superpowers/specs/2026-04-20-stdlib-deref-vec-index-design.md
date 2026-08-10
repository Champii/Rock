# Stdlib Deref And Vec Index Design

## Context

The `assoc-types-ops` branch already moved indexing onto the real stdlib `Index` trait and removed the legacy builtin indexing path. Follow-up inspection found three missing pieces in the user-facing story:

- stdlib exposes `Index`, but not a matching stdlib `Deref` trait
- `Vec T` still uses non-Rust-like `get` semantics
- projection syntax coverage is implicit in integration tests, but there is no explicit parser coverage for `Self::Target` / `Self::Output`

There is also a larger design question behind `Vec` indexing. A narrow fix would teach only `[]` to consult `Deref` once. That would work, but it would become technical debt if method lookup and other operators later need the same receiver adaptation rule.

The better model is Rust-like autoderef:

- receiver adaptation is a shared mechanism
- it repeatedly follows `Deref::Target`
- operators and method lookup can consume the same receiver candidates
- trait resolution remains exact once a receiver candidate is chosen

This design adopts that model in a narrow, reusable form instead of adding a `[]`-specific special case.

## Goals

- Add a real stdlib `Deref` trait and make it available from the prelude.
- Change `Vec.get` to Rust-like reference semantics.
- Make `Vec[i]` work through `Vec: Deref<Target = [T]>` rather than a direct `Vec: Index` impl.
- Introduce a shared autoderef-style receiver resolver that can be reused across operators.
- Extend builtin `[T]` indexing to bounds-check all element types so `Vec[i]` inherits Rust-like out-of-bounds trapping.
- Add explicit parser coverage for `Self::Target` and `Self::Output`.
- Add integration coverage for stdlib `Deref`, `Vec.get`, `Vec[i]`, and associated-type projection usage.

## Non-Goals

- Adding `IndexMut`
- Implementing an unconstrained trait-graph search for receiver lookup
- Broadening projection syntax beyond the explicit `Self::Assoc` forms already in use
- Reworking the slice type model beyond what is needed for `Vec` to deref to `[T]`
- Reintroducing any legacy builtin indexing fallback

## Decisions

### Stdlib `Deref`

Add a new stdlib trait:

```rock
trait Deref
    type Target
    @deref: Self -> &Self::Target
```

This trait lives in `stdlib/deref.rk`, is exported from `stdlib/lib.rk`, and re-exported from `stdlib/prelude.rk`.

This matches the current compiler-side unary `*` model, which already consults `Deref` when the operand is not a built-in reference or raw pointer.

### `Vec.get` semantics

Change `Vec.get` from returning `Option T` to returning `Option &T`.

Target behavior:

- in-bounds: `Option::Some (&element)`
- out-of-bounds: `Option::None`

This is the necessary bridge between the stdlib collection API and the reference-returning `Deref` / `Index` contracts.

### `Vec` uses `Deref`, not direct `Index`

Add:

```rock
impl Deref for Vec T
    type Target = [T]
    @deref: Self -> &Self::Target
```

Do not add a direct `impl Index I64 for Vec T`.

The intended model is Rust-like:

- `Vec<T>` exposes a borrowed slice target
- indexing works against that target
- `vec[i]` is valid because receiver resolution can see `[T]` through `Deref::Target`

This keeps the language rule honest and avoids baking a special one-off indexing rule into `Vec` itself.

This requires the implementation to produce a stable borrowed slice view of the vector's current `raw_ptr` and `raw_len`, not a dangling reference to a temporary value. If the current lowering/codegen path cannot express that purely in stdlib code, the compiler may add the smallest supporting change necessary to let a `Deref` impl return `&[T]` soundly.

### Shared autoderef-style receiver resolution

Introduce one shared receiver adaptation helper for operator and method lookup.

Conceptually, given a receiver type `T`, it yields candidates in this order:

- `T`
- `Deref::Target(T)`
- `Deref::Target(Deref::Target(T))`
- and so on

The helper must be:

- deterministic
- cycle-safe
- bounded by a fixed recursion limit as a termination guard

That bound is not a temporary half-step. It is a safety cap on a well-defined autoderef rule, not a placeholder for arbitrary future trait chasing.

Important constraint:

- this is not a general "search every trait chain until something works" engine
- only the language-defined receiver adaptation path is followed
- exact trait lookup still happens independently on each concrete candidate receiver type

This helper should be used for:

- unary `*` now
- `[]` lookup now
- later method lookup if that work is taken on, without changing the underlying model again

### `[]` uses autoderef candidates

The current compiler path resolves `receiver[index]` by checking builtin index support and direct `Index` impls on the receiver type.

Under the new model, `[]` should instead:

1. build receiver candidates via the shared autoderef helper
2. for each candidate, check builtin `Index` support or a matching user impl
3. stop at the first successful candidate
4. use that candidate type when forming the `Index::Output` projection

This keeps `a[b]` on the existing trait-based lowering model while making `Vec[i]` work via `Vec -> Deref::Target -> [T] -> Index`.

### Builtin `[T]` bounds semantics

Today, builtin bounds checks exist only for `[U8]` indexing in codegen.

Extend builtin `[T]` indexing so all slice/array element types are bounds-checked for `I64` indices:

- negative index traps
- index greater than or equal to length traps
- in-bounds access succeeds normally

Use the same runtime failure shape already used for `[U8]` indexing:

- same user-facing message shape
- same non-zero exit behavior
- no second indexing-failure mechanism

This ensures that `Vec[i]`, when reached through `Deref<Target = [T]>`, inherits Rust-like out-of-bounds behavior instead of silently keeping the current unchecked generic `[T]` semantics.

## Architecture

### Stdlib changes

Files involved:

- create `stdlib/deref.rk`
- modify `stdlib/lib.rk`
- modify `stdlib/prelude.rk`
- modify `stdlib/vec.rk`

Responsibilities:

- `deref.rk` declares the user-facing trait
- `lib.rk` exports the module
- `prelude.rk` makes the trait available automatically when stdlib is imported
- `vec.rk` adopts Rust-like `get` semantics and exposes `Deref<Target = [T]>`

### Compiler changes

Expected files are in the existing operator-lowering and type-resolution path, especially around:

- `lib/src/lower/control_flow/secondary.rs`
- `lib/src/lower/expression.rs`
- `lib/src/lower/types_helpers/helpers.rs`
- `lib/src/codegen/types.rs`
- `lib/src/codegen/expr/access.rs`

Responsibilities:

- add the shared autoderef-style receiver helper close to the existing trait lookup helpers
- reuse it in unary `*` lowering and `[]` lowering
- preserve associated-type projection resolution for whichever receiver candidate wins
- extend builtin array/slice codegen bounds checks from `[U8]` to all `[T]`

The important constraint is to keep one operator model:

- `a[b]` still lowers through the trait-style `index` method plus dereference
- the compiler must not restore a separate user-visible indexing path for `Vec`
- builtin indexing remains a compiler-backed implementation of real `Index` semantics, not a competing language feature

## Testing Strategy

### Parser tests

Add explicit parser tests in `lib/src/parser/items/tests/path/test_type_path.rs` for:

- `Self::Target`
- `Self::Output`

These tests only need to validate path parsing and segment preservation. Nearby AST-validation tests already cover trait and impl associated-type declarations at a broader level.

### Integration tests

Add focused integration coverage in `lib/tests/integration.rs` for:

- stdlib `Deref` being available from the prelude without redefining the trait inline
- `Vec.get` returning `Option::Some` for in-bounds access and `Option::None` for out-of-bounds access under the new reference-returning semantics
- `Vec[i]` working through the shared autoderef receiver chain into `[T]` indexing
- associated-type projection syntax appearing in real trait signatures through stdlib `Deref` / `Index`

Add an out-of-bounds indexing test if the existing harness can assert the runtime failure reliably without making the suite brittle. If not, implementation should still reuse the established failure path, and the missing direct assertion should be called out explicitly when reporting verification.

## Rationale

This keeps the language model coherent without overreaching.

The wrong outcome here would be another local exception: first builtin indexing, then a `[]`-specific deref special case, then something similar for methods later. The better outcome is one reusable receiver adaptation rule based on repeated `Deref::Target`, with exact trait lookup applied to each concrete candidate.

That gives `Vec` the Rust-like shape you asked for:

- `get` is the non-panicking optional API
- `vec[i]` works because `Vec` dereferences to `[T]`
- out-of-bounds indexing traps through the slice indexing path

The explicit parser tests close a real coverage gap without turning this follow-up into a broader projection-syntax project.
