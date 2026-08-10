# Stdlib HashMap Design

## Context

Bead `new_lang2-5bg` asks for missing stdlib data structures similar to Rust's standard library, including maps. The current Rock stdlib already provides `Vec`, `Option`, `Result`, `Eq`, `Ord`, `Index`, raw allocation through `libc`, and prelude re-exports. It does not provide hashing traits or map/set types.

The first implementation slice will add hashing support and a usable `HashMap`. `HashSet`, `BTreeMap`, iterators, removal, and entry APIs are follow-up work.

## Goals

- Add a stdlib `Hash` trait that key types can implement.
- Add primitive `Hash` implementations for common scalar keys.
- Add a generic `HashMap K, V` with a small, useful API.
- Keep the implementation in Rock stdlib files unless compiler limitations make that impossible.
- Verify behavior through user-visible integration tests.

## Non-Goals

- Do not add compiler-owned stdlib loading or special-case collection support.
- Do not add a full Rust-compatible collections API in this slice.
- Do not add `HashSet`, `BTreeMap`, `VecDeque`, `LinkedList`, or `BinaryHeap` yet.
- Do not add iterator protocols unless needed by `HashMap` basics.
- Do not optimize for production hash-table performance before the API works.

## API

Add `stdlib/hash.rk`:

```rock
< trait Hash
    @hash: Self -> I64
```

Implement `Hash` for `I64`, `I32`, `Bool`, `Char`, and `&Str`.

Add `stdlib/hash_map.rk` with public `HashMap K, V` and private backing storage. The initial method set is:

```rock
impl HashMap K, V where K: Hash, K: Eq
    new = -> HashMap K, V
    @len = -> I64
    ^@insert = key, value -> Unit
    @get = key -> Option &V
    @contains_key = key -> Bool
```

`insert` overwrites an existing key without increasing `len`. `get` returns `Option::Some &V` for present keys and `Option::None` for missing keys.

## Implementation Shape

Use a simple open-addressed table with linear probing:

- Store hashes, occupied flags, keys, and values in heap-backed arrays.
- Start with a small capacity, such as 8.
- Grow when insertion would exceed a conservative load threshold.
- Use positive modulo normalization so negative hash values map to valid bucket indexes.
- Probe until an empty bucket or matching key is found.

## Module Exports

- Add `< mod hash` and `< mod hash_map` to `stdlib/lib.rk`.
- Re-export both from `stdlib/prelude.rk` so `Hash` and `HashMap` are available in stdlib-backed programs.

## Error Handling And Safety

- Backing fields remain private.
- `get` uses `Option` instead of trapping for missing keys.
- Unsafe pointer operations, if needed, stay confined to the stdlib implementation.
- Allocation failure behavior follows the existing stdlib pattern and does not introduce new diagnostics in this slice.

## Tests

Add integration tests in `lib/tests/integration.rs` for:

- Creating a map, inserting `I64` keys, and reading values back.
- Missing keys returning `Option::None`.
- Overwriting an existing key without increasing `len`.
- `contains_key` returning true and false appropriately.
- At least one collision/probing or repeated-insert scenario.
- Private backing fields rejecting access, if the final representation has private fields.

Run focused integration tests first, then `cargo test -p rock-lib`.

## Follow-Up Work

Create separate beads for:

- `HashSet` built on the same `Hash` foundation.
- `BTreeMap` once the desired tree representation and ordering API are clear.
- Removal, capacity management, and iterator-style APIs for `HashMap`.
- Additional `Hash` implementations for `String`, references, tuples, and user-defined derivation if the language gains derive-like support.
