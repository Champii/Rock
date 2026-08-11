# Owned Heap Soundness Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make owned heap values sound by rejecting illegal moves through references, adding borrowed pattern matching for non-consuming enum APIs, removing temporary heap promotion, and unifying trait-bound receiver adjustment with concrete method selection.

**Architecture:** Ownership rules are enforced at MIR borrow checking, borrowed pattern matching is represented during MIR lowering rather than hidden in codegen, stdlib APIs use receiver modes that match their ownership semantics, and receiver adjustment selection is centralized in the selection service. Codegen must only lower MIR ownership decisions; it must not create untracked heap owners.

**Tech Stack:** Rust 2021 compiler implementation in `lib/`, MIR borrow checking and lowering, LLVM codegen via `inkwell`, Rock stdlib source under `stdlib/`, integration tests under `lib/tests/integration.rs`.

**Repository Constraints:** Do not commit, stage, push, or touch `.sisyphus/` unless explicitly requested by the user. Use TDD: write each failing regression test, run it to verify the failure, then implement.

---

## Why Rust Allows Some `*self` Patterns

Rust does not move a non-`Copy` value out of a shared reference. This is rejected:

```rust
match *self {
    Some(value) => value,
    None => default,
}
```

when `T: !Copy`, because `value` would be moved out of `&Option<T>`. Rust code that wants to inspect without consuming borrows the payload instead:

```rust
match self {
    Some(value) => value, // value: &T under match ergonomics
    None => ...,
}
```

or explicitly:

```rust
match *self {
    Some(ref value) => value,
    None => ...,
}
```

Rock should follow the same semantic split: non-consuming methods pattern-match borrowed payloads, while consuming combinators use move receiver.

## File Structure

- Modify `lib/src/mir/borrowck/mod.rs` to reject non-`Copy` moves through references and add focused unit tests for the move rule.
- Modify `lib/src/mir/builder/expr.rs` and `lib/src/mir/builder/mod.rs` to lower `match *ref_expr` in borrowed mode, binding enum payloads as references rather than owned moves.
- Modify `stdlib/option.rk` and `stdlib/result.rk` so consuming combinators use move receiver, while `show`, `println`, and `inspect` remain non-consuming and use borrowed payloads.
- Modify `lib/src/codegen/mir_llvm/rvalue.rs` and tests near `lib/src/codegen/mir_llvm/mod.rs` to remove temporary reference heap promotion.
- Modify `lib/src/selection/service.rs`, `lib/src/lower/expression.rs`, and `lib/src/lower/control_flow/secondary.rs` to route trait-bound method selection through receiver adjustment candidates.
- Add integration tests in `lib/tests/integration.rs` covering owned payloads, borrowed matching, receiver adjustments, and temporary-reference codegen behavior.

---

### Task 1: Reject Non-Copy Moves Through References

**Files:**
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing borrowck unit tests**

Add tests showing that moving a non-`Copy` value through `Projection::Deref` rooted in `&T` is rejected, while copying `I64` through `&I64` is allowed.

```rust
#[test]
fn borrowck_rejects_non_copy_move_out_of_shared_reference() {
    // Construct MIR with local 1: &String and local 2: String.
    // Statement: _2 = move (*_1)
    // Expected diagnostic: Cannot move non-copy value out of a reference.
}

#[test]
fn borrowck_allows_copy_out_of_shared_reference() {
    // Construct MIR with local 1: &I64 and local 2: I64.
    // Statement: _2 = copy (*_1)
    // Expected: BorrowChecker::check_function returns Ok.
}
```

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test -p rock-lib mir::borrowck::tests::borrowck_rejects_non_copy_move_out_of_shared_reference -- --exact`

Expected: FAIL because no diagnostic is emitted.

- [ ] **Step 3: Implement move-through-reference validation**

In `MoveValidationContext::validate_move`, walk the moved place projection while tracking the current type. If a `Projection::Deref` is applied to a `Type::Reference { .. }` and the final moved type is non-`Copy`, emit a diagnostic and return.

Do not reject raw-pointer dereference moves; unsafe stdlib containers rely on raw-pointer element moves.

Diagnostic text:

```text
Cannot move non-copy value of type '<type>' out of a reference
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `cargo test -p rock-lib mir::borrowck -- --nocapture`

Expected: all borrowck tests pass.

- [ ] **Step 5: Add integration regression**

Add `compile_should_fail` for:

```rock
main = ->
    s = String::from_str "hello"
    r = &s
    moved = *r
    0
```

Expected diagnostic contains `Cannot move non-copy value`.

- [ ] **Step 6: Run integration regression**

Run the exact new integration test.

Expected: PASS.

---

### Task 2: Borrowed Pattern Matching For Referenced Enum Scrutinees

**Files:**
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing tests for borrowed enum payloads**

Add integration tests for non-consuming matches:

```rock
show_option: &Option String -> I32
show_option = opt ->
    match *opt
        Option::Some value => value.len!
        Option::None => 0

main = ->
    value = Option::Some (String::from_str "hello")
    show_option &value .println!
    show_option &value .println!
    0
```

Expected output: `5`, `5`.

- [ ] **Step 2: Run test and verify RED**

Run the exact integration test.

Expected: FAIL, either from the new move-through-reference diagnostic or from incorrect payload ownership.

- [ ] **Step 3: Add match scrutinee lowering mode**

Introduce an internal enum in MIR builder, for example:

```rust
enum MatchScrutineeMode {
    ByValue,
    Borrowed { mutable: bool },
}
```

When the HIR scrutinee is `HirExprKind::Deref(inner)` and `inner.ty` resolves to `Type::Reference`, lower the matched place without moving the referent. Use `Borrowed { mutable }` for payload bindings.

- [ ] **Step 4: Bind borrowed enum payloads as references**

In `bind_match_pattern_places`, when matching enum payloads in borrowed mode, create binding locals with type `&T` or `&mut T` and assign `Rvalue::Ref` from the payload place. Do not set `moved_enum_scrutinee` and do not mark the original source as moved.

For this task, default borrowed payload bindings to shared references. Mutable borrowed pattern bindings can be added later behind explicit syntax.

- [ ] **Step 5: Preserve by-value match behavior**

Ensure normal `match opt` still moves non-`Copy` payloads for consuming APIs. Existing Option/Result integer tests must remain unchanged.

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test -p rock-lib --test integration <new_borrowed_match_test> -- --exact
cargo test -p rock-lib --test integration test_stdlib_option_methods -- --exact
cargo test -p rock-lib --test integration test_stdlib_result_methods -- --exact
```

Expected: all pass.

---

### Task 3: Correct Option And Result Ownership APIs

**Files:**
- Modify: `stdlib/option.rk`
- Modify: `stdlib/result.rk`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing owned-payload stdlib tests**

Add tests that prove `Option<String>.show!` and `Result<String, String>.show!` are non-consuming and can be called twice, while `map` consumes the option/result.

```rock
main = ->
    value = Option::Some (String::from_str "hello")
    value.show!.println!
    value.show!.println!
    0
```

Expected output: `Some(hello)`, `Some(hello)`.

Also add a negative test:

```rock
main = ->
    value = Option::Some (String::from_str "hello")
    mapped = value.map (s -> s.len!)
    again = value.show!
    0
```

Expected: use-after-move diagnostic.

- [ ] **Step 2: Run tests and verify RED**

Run the exact new tests.

Expected: at least one fails before stdlib receiver-mode corrections.

- [ ] **Step 3: Change consuming combinators to move receivers**

In `stdlib/option.rk`, change these methods to move receiver (`~@`):

```rock
~@map: (T -> U) -> Option U
~@and_then: (T -> Option U) -> Option U
~@or: Option T -> Option T
~@flatten: Option U
~@fold: U -> (T -> U) -> U
~@unwrap_or: T -> T
~@>>=: (T -> Option U) -> Option U
~@<|>: Option T -> Option T
~@<&>: (T -> U) -> Option U
```

Use consuming `match self` or `match *self` according to method receiver lowering rules after implementation.

In `stdlib/result.rk`, do the same for `map`, `map_err`, `and_then`, `or`, `unwrap_or`, `flatten`, `fold`, and operator aliases.

- [ ] **Step 4: Keep non-consuming APIs borrowed**

Keep `@show`, `@println`, and `@inspect` as shared receiver. Their pattern bindings should receive references after Task 2.

- [ ] **Step 5: Run stdlib tests**

Run:

```bash
cargo test -p rock-lib --test integration test_stdlib_option_methods -- --exact
cargo test -p rock-lib --test integration test_stdlib_result_methods -- --exact
cargo test -p rock-lib --test integration test_stdlib_option_result_show_prints_payloads -- --exact
```

Expected: all pass.

---

### Task 4: Remove MIR Temporary Reference Heap Promotion

**Files:**
- Modify: `lib/src/codegen/mir_llvm/rvalue.rs`
- Modify: `lib/src/codegen/mir_llvm/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing codegen regression**

Add or update codegen test so a reference to a temporary owned value does not emit `mir_ref_tmp_alloc` and does not call `malloc` for reference promotion.

Test assertion:

```rust
assert!(!ir.contains("mir_ref_tmp_alloc"));
```

- [ ] **Step 2: Run test and verify RED if current IR still promotes**

Run exact codegen test.

Expected: FAIL if promotion path is reachable for a temporary source.

- [ ] **Step 3: Delete promotion path**

Remove `should_promote_mir_ref_source` and the `malloc`/load/store block in `compile_mir_ref_rvalue`. Return the MIR place address directly for thin references.

- [ ] **Step 4: Run MIR codegen tests**

Run: `cargo test -p rock-lib codegen::mir_llvm -- --nocapture`

Expected: all pass.

---

### Task 5: Unify Trait-Bound Receiver Adjustment With Concrete Selection

**Files:**
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Test: `lib/src/selection/service.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing selection and integration tests**

Add tests for:

```rock
trait Touch
    ^@touch: Unit

poke: T -> Unit where T: Touch
poke = value ->
    mut local = value
    local.touch!
```

and for shared receiver on `&T` where the call must not become `&&T`.

- [ ] **Step 2: Run tests and verify RED**

Run exact new tests.

Expected: fail due missing receiver adjustment or wrong mutability.

- [ ] **Step 3: Change bound selection API**

Replace raw `receiver: HirExpr` bound-selection calls with `receiver_candidates: &[ReceiverCandidate]`, or add parallel bound-selection methods that accept candidates. Use each candidate’s `expr`, `adjustment`, and `can_autoref_mut`.

- [ ] **Step 4: Preserve selected adjustment**

For trait methods and trait signatures, call `self_type_adjustment` using candidate mutability. If an adjustment is returned as `None`, preserve the candidate’s adjustment. Store the adjusted candidate in `SelectedMethod`.

- [ ] **Step 5: Update lowerer call sites**

At every `select_bound_method_preferring_non_ref_receiver` call site, compute `receiver_adjustment_candidates(expr.clone())` and pass candidates into selection.

- [ ] **Step 6: Run selection and integration tests**

Run:

```bash
cargo test -p rock-lib selection::service -- --nocapture
cargo test -p rock-lib --test integration <new_trait_bound_receiver_test> -- --exact
```

Expected: all pass.

---

### Task 6: Final Verification And Review

**Files:**
- No production files unless fixing issues found by verification.

- [ ] **Step 1: Run focused verification**

Run:

```bash
cargo test -p rock-lib mir::borrowck -- --nocapture
cargo test -p rock-lib codegen::mir_llvm -- --nocapture
cargo test -p rock-lib selection::service -- --nocapture
cargo test -p rock-lib --test integration test_stdlib_option_methods -- --exact
cargo test -p rock-lib --test integration test_stdlib_result_methods -- --exact
```

Expected: all pass.

- [ ] **Step 2: Run full verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

Expected: all pass.

- [ ] **Step 3: Final review**

Review the final diff for:

- No untracked scratch files.
- No codegen-owned allocation for reference promotion.
- No stdlib consuming API left as shared receiver.
- No generic trait-bound receiver selection path bypassing receiver candidates.
- No new compiler-owned stdlib injection or hardcoded stdlib operators.

Expected: no release-blocking findings remain.
