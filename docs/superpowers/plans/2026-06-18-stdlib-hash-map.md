# Stdlib HashMap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add stdlib hashing support and a usable generic `HashMap K, V` with insertion, lookup, length, and membership checks.

**Architecture:** Implement this as Rock stdlib code, not compiler magic. Add a `Hash` trait and primitive implementations, then build `HashMap` as an open-addressed table with linear probing, private raw storage, and `Option`-based lookups.

**Tech Stack:** Rock stdlib source files under `stdlib/`; Rust integration tests in `lib/tests/integration.rs`; verification through `cargo test -p rock-lib --test integration ...` and `cargo test -p rock-lib`.

**VCS Note:** This repository's current instructions prohibit git staging, commits, pushes, and other VCS mutations unless the current user explicitly asks. This plan uses verification checkpoints instead of commit steps.

---

## File Structure

- Create `stdlib/hash.rk`: owns the `Hash` trait and scalar/string hash implementations.
- Create `stdlib/hash_map.rk`: owns `HashMap K, V`, its private storage fields, allocation/probing/growth helpers, and public map methods.
- Modify `stdlib/lib.rk`: declares the new stdlib modules so artifact builds include them.
- Modify `stdlib/prelude.rk`: re-exports `Hash` and `HashMap` for stdlib-backed programs.
- Modify `lib/tests/integration.rs`: adds user-visible tests for hashing and `HashMap` behavior.

---

### Task 1: Add failing `Hash` trait tests

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add scalar and string hash integration tests**

Append these tests near the existing stdlib tests, close to `test_stdlib_math` or the `Vec` tests:

```rust
#[test]
fn test_hash_trait_scalar_and_str() {
    let output = compile_and_run(
        r#"
main = ->
    (42.hash!).println!
    ((-42).hash!).println!
    (true.hash!).println!
    (false.hash!).println!
    ('A'.hash!).println!
    ("abc".hash!).println!
    ("abc".hash!).println!
    ("abd".hash!).println!
    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines[0], "42");
    assert_eq!(lines[1], "-42");
    assert_eq!(lines[2], "1");
    assert_eq!(lines[3], "0");
    assert_eq!(lines[4], "65");
    assert_eq!(lines[5], lines[6]);
    assert_ne!(lines[5], lines[7]);
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
cargo test -p rock-lib --test integration test_hash_trait_scalar_and_str -- --exact --nocapture
```

Expected: FAIL during compilation of the Rock snippet because `hash` / `Hash` is not defined or not available from the prelude.

- [ ] **Step 3: Record checkpoint**

Do not stage or commit. Note the failure text in the session summary before continuing.

---

### Task 2: Implement and export `Hash`

**Files:**
- Create: `stdlib/hash.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`

- [ ] **Step 1: Create `stdlib/hash.rk`**

Create the file with this content:

```rock
// Hash trait - provides stable integer hashes for hash-table keys.

< trait Hash
    @hash: Self -> I64

impl Hash for I64
    @hash = -> self

impl Hash for I32
    @hash = -> self as I64

impl Hash for Bool
    @hash = ->
        if self
            1
        else
            0

impl Hash for Char
    @hash = -> self as I64

impl Hash for &Str
    @hash = ->
        ptr = ~ArrPtr self
        len = ~ArrayLen self
        h = 5381
        i = 0
        while i < len
            byte = unsafe *(ptr + i)
            h = ((h * 33) + (byte as I64)) % 2147483647
            i = i + 1
        h
```

- [ ] **Step 2: Export the module from `stdlib/lib.rk`**

Insert the module declaration after the existing Eq module declaration and before Bitwise:

```rock
// Eq module - equality comparison trait and implementations
< mod eq

// Hash module - hashing trait and implementations
< mod hash

// Bitwise module - bitwise trait and implementations
< mod bitwise
```

- [ ] **Step 3: Re-export `Hash` from `stdlib/prelude.rk`**

Add this line in the Traits section after `eq`:

```rock
< stdlib::hash::*
```

The surrounding section should look like:

```rock
< stdlib::ord::*
< stdlib::eq::*
< stdlib::hash::*
< stdlib::bitwise::*
```

- [ ] **Step 4: Run the focused hash test and verify it passes**

Run:

```bash
cargo test -p rock-lib --test integration test_hash_trait_scalar_and_str -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 5: Record checkpoint**

Do not stage or commit. Record that `Hash` is implemented and exported.

---

### Task 3: Add failing `HashMap` behavior tests

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add insertion, lookup, overwrite, and contains tests**

Append these tests near the `Vec` stdlib tests:

```rust
#[test]
fn test_hash_map_insert_get_len_and_contains() {
    let output = compile_and_run(
        r#"
main = ->
    mut map = HashMap::new!
    (map.len!).println!
    map.insert 10, 100
    map.insert 20, 200
    (map.len!).println!
    (map.contains_key 10).println!
    (map.contains_key 30).println!

    match (map.get 10)
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    match (map.get 20)
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    match (map.get 30)
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["0", "2", "true", "false", "100", "200", "-1"]);
}

#[test]
fn test_hash_map_insert_overwrites_without_growing_len() {
    let output = compile_and_run(
        r#"
main = ->
    mut map = HashMap::new!
    map.insert 1, 10
    map.insert 1, 99
    (map.len!).println!

    match (map.get 1)
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "99"]);
}

#[test]
fn test_hash_map_handles_collisions_and_growth() {
    let output = compile_and_run(
        r#"
main = ->
    mut map = HashMap::new!
    map.insert 1, 10
    map.insert 9, 90
    map.insert 17, 170
    map.insert 25, 250
    map.insert 33, 330
    map.insert 41, 410
    map.insert 49, 490

    (map.len!).println!

    match (map.get 1)
        Option::Some val => (*val).println!
        Option::None => (-1).println!
    match (map.get 9)
        Option::Some val => (*val).println!
        Option::None => (-1).println!
    match (map.get 49)
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["7", "10", "90", "490"]);
}
```

- [ ] **Step 2: Run one focused test and verify it fails**

Run:

```bash
cargo test -p rock-lib --test integration test_hash_map_insert_get_len_and_contains -- --exact --nocapture
```

Expected: FAIL during compilation of the Rock snippet because `HashMap` is not defined or not available from the prelude.

- [ ] **Step 3: Record checkpoint**

Do not stage or commit. Note the failure before implementation.

---

### Task 4: Implement and export `HashMap`

**Files:**
- Create: `stdlib/hash_map.rk`
- Modify: `stdlib/lib.rk`
- Modify: `stdlib/prelude.rk`

- [ ] **Step 1: Create `stdlib/hash_map.rk`**

Create the file with this content:

```rock
// HashMap[K, V] - open-addressed hash table with linear probing.

> stdlib::libc::malloc
> stdlib::libc::free
> stdlib::option::Option
> stdlib::hash::Hash
> stdlib::eq::Eq

< struct HashMap K, V
    hashes: *I64
    occupied: *Bool
    keys: *K
    values: *V
    raw_len: I64
    raw_cap: I64

impl HashMap K, V where K: Hash, K: Eq
    @len = -> self.raw_len

    @bucket_index = hash, cap ->
        idx = hash % cap
        if idx < 0
            idx + cap
        else
            idx

    ^@grow = sample_key, sample_value ->
        old_hashes = self.hashes
        old_occupied = self.occupied
        old_keys = self.keys
        old_values = self.values
        old_cap = self.raw_cap

        new_cap = if old_cap == 0
            8
        else
            old_cap * 2

        new_hashes = (malloc (new_cap * (stdlib::mem::size_of 0))) as *I64
        new_occupied = (malloc (new_cap * (stdlib::mem::size_of true))) as *Bool
        new_keys = (malloc (new_cap * (stdlib::mem::size_of sample_key))) as *K
        new_values = (malloc (new_cap * (stdlib::mem::size_of sample_value))) as *V

        i = 0
        while i < new_cap
            unsafe new_occupied[i] = false
            i = i + 1

        self.hashes = new_hashes
        self.occupied = new_occupied
        self.keys = new_keys
        self.values = new_values
        self.raw_len = 0
        self.raw_cap = new_cap

        i = 0
        while i < old_cap
            if unsafe old_occupied[i]
                old_key = unsafe old_keys[i]
                old_value = unsafe old_values[i]
                self.insert old_key, old_value
            i = i + 1

        if old_cap > 0
            free (old_hashes as *U8)
            free (old_occupied as *U8)
            free (old_keys as *U8)
            free (old_values as *U8)

    ^@insert = key, value ->
        if self.raw_cap == 0 || ((self.raw_len + 1) * 4) > (self.raw_cap * 3)
            self.grow key, value

        hash = key.hash!
        start = self.bucket_index hash, self.raw_cap
        i = 0
        done = false

        while !done && i < self.raw_cap
            idx = (start + i) % self.raw_cap
            if unsafe self.occupied[idx]
                if (unsafe self.hashes[idx]) == hash && (unsafe self.keys[idx]) == key
                    unsafe self.values[idx] = value
                    done = true
            else
                unsafe self.hashes[idx] = hash
                unsafe self.occupied[idx] = true
                unsafe self.keys[idx] = key
                unsafe self.values[idx] = value
                self.raw_len = self.raw_len + 1
                done = true
            i = i + 1

    @get: HashMap K, V -> K -> Option &V
    @get = key ->
        if self.raw_cap == 0
            Option::None
        else
            hash = key.hash!
            start = self.bucket_index hash, self.raw_cap
            i = 0
            found = false
            result = Option::None

            while !found && i < self.raw_cap
                idx = (start + i) % self.raw_cap
                if unsafe self.occupied[idx]
                    if (unsafe self.hashes[idx]) == hash && (unsafe self.keys[idx]) == key
                        result = unsafe Option::Some (&(self.values[idx]))
                        found = true
                else
                    found = true
                i = i + 1

            result

    @contains_key = key ->
        match (self.get key)
            Option::Some _ => true
            Option::None => false

    new = ->
        HashMap K, V
            hashes: (malloc 0) as *I64
            occupied: (malloc 0) as *Bool
            keys: (malloc 0) as *K
            values: (malloc 0) as *V
            raw_len: 0
            raw_cap: 0
```

- [ ] **Step 2: Export the module from `stdlib/lib.rk`**

Insert the module declaration after `vec`:

```rock
// Vec module - growable heap-allocated vector
< mod vec

// HashMap module - hash-table map
< mod hash_map
```

- [ ] **Step 3: Re-export `HashMap` from `stdlib/prelude.rk`**

Add this line in the Types section after `vec`:

```rock
< stdlib::hash_map::*
```

The surrounding section should look like:

```rock
< stdlib::option::*
< stdlib::result::*
< stdlib::vec::*
< stdlib::hash_map::*
```

- [ ] **Step 4: Run the focused map tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib --test integration test_hash_map_insert_get_len_and_contains -- --exact --nocapture
```

Expected: PASS.

```bash
cargo test -p rock-lib --test integration test_hash_map_insert_overwrites_without_growing_len -- --exact --nocapture
```

Expected: PASS.

```bash
cargo test -p rock-lib --test integration test_hash_map_handles_collisions_and_growth -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 5: Record checkpoint**

Do not stage or commit. Record the passing focused tests.

---

### Task 5: Add privacy and string-key coverage

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add private-field and `&Str` key tests**

Append these tests near the other `HashMap` tests:

```rust
#[test]
fn test_hash_map_private_storage_fields_are_rejected() {
    compile_should_fail(
        r#"
main = ->
    map = HashMap::new!
    x = map.raw_len
    0
"#,
        "Field 'raw_len' of struct 'HashMap' is private",
    );
}

#[test]
fn test_hash_map_str_keys() {
    let output = compile_and_run(
        r#"
main = ->
    mut map = HashMap::new!
    map.insert "red", 1
    map.insert "blue", 2
    map.insert "red", 3
    (map.len!).println!

    match (map.get "red")
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    match (map.get "green")
        Option::Some val => (*val).println!
        Option::None => (-1).println!

    0
"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["2", "3", "-1"]);
}
```

- [ ] **Step 2: Run the focused privacy test**

Run:

```bash
cargo test -p rock-lib --test integration test_hash_map_private_storage_fields_are_rejected -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run the focused string-key test**

Run:

```bash
cargo test -p rock-lib --test integration test_hash_map_str_keys -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 4: If string-key lookup fails because `&Str` equality is not selected, narrow the public claim**

Only if the compiler cannot dispatch `Eq for &Str` for string literal keys, remove `test_hash_map_str_keys` and keep `&Str` hashing covered by `test_hash_trait_scalar_and_str`. Do not remove `Hash for &Str`. Record the compiler diagnostic in the session summary so a follow-up bead can address string-key dispatch.

---

### Task 6: Final verification

**Files:**
- Verify: `stdlib/hash.rk`
- Verify: `stdlib/hash_map.rk`
- Verify: `stdlib/lib.rk`
- Verify: `stdlib/prelude.rk`
- Verify: `lib/tests/integration.rs`

- [ ] **Step 1: Run Rust formatting check**

Run:

```bash
cargo fmt --all --check
```

Expected: PASS.

- [ ] **Step 2: Run whitespace diff check**

Run:

```bash
git diff --check
```

Expected: PASS with no whitespace errors. This does not mutate VCS state.

- [ ] **Step 3: Run all `rock-lib` tests**

Run:

```bash
cargo test -p rock-lib
```

Expected: PASS. The baseline before this work was `1654` library tests, `284` integration tests, and doctests passing; counts may increase by the new integration tests.

- [ ] **Step 4: Inspect final diff without staging**

Run:

```bash
git diff -- stdlib/hash.rk stdlib/hash_map.rk stdlib/lib.rk stdlib/prelude.rk lib/tests/integration.rs docs/superpowers/specs/2026-06-18-stdlib-hash-map-design.md docs/superpowers/plans/2026-06-18-stdlib-hash-map.md
```

Expected: Diff contains only the spec, plan, stdlib modules/exports, and integration tests for `Hash`/`HashMap`.

- [ ] **Step 5: Update bead status only after verification passes**

If the current user explicitly wants bead updates in this session, run:

```bash
bd close new_lang2-5bg --reason "Implemented stdlib Hash trait and HashMap" --json
```

Expected: bead status becomes `closed`. If the user has not requested bead closure, leave it `in_progress` and mention that in the final handoff.

---

## Self-Review Notes

- Spec coverage: `Hash` trait, primitive and `&Str` hashing, `HashMap`, exports, private storage, `Option` lookup, overwrite semantics, collisions/growth, focused and full tests are covered.
- Placeholder scan: no placeholder or unspecified implementation steps remain.
- Type consistency: the plan consistently uses `HashMap K, V`, `Hash`, `Eq`, `Option &V`, `insert`, `get`, `contains_key`, and `len`.
