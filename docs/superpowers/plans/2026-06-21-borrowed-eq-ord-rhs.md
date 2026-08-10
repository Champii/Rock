# Borrowed Eq Ord RHS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Eq` and `Ord` operators borrow their RHS like Rust-style operator traits, then remove `EqRef`.

**Architecture:** Keep the change operator-specific rather than adding broad non-receiver autoref. `Eq` and `Ord` signatures explicitly require `&Self` for the RHS, and operator lowering uses existing argument coercion so selected operator methods can borrow the RHS without moving it.

**Tech Stack:** Rock stdlib, Rust compiler lowering/selection code in `rock-lib`, integration tests in `lib/tests/integration.rs`.

---

### Task 1: Add Failing Coverage

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add integration tests**

Add tests covering generic `T: Eq` with non-`Copy` values, `HashMap` custom keys using only `Eq`, and `Ord` borrowed RHS behavior.

- [ ] **Step 2: Run focused tests**

Run: `cargo test -p rock-lib --test integration test_generic_eq_bound_borrows_rhs_without_move -- --exact`
Expected before implementation: FAIL because `Eq` still takes RHS by value or `EqRef` is still required for `HashMap` custom keys.

### Task 2: Update Stdlib Traits And Call Sites

**Files:**
- Modify: `stdlib/eq.rk`
- Modify: `stdlib/ord.rk`
- Modify: `stdlib/hash_map.rk`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Change `Eq` signatures**

Change `Eq` to `@==: &Self -> Bool` and `@!=: &Self -> Bool`; update primitive and reference impls to dereference borrowed RHS.

- [ ] **Step 2: Delete `EqRef`**

Remove the `EqRef` trait and all impls from `stdlib/eq.rk`.

- [ ] **Step 3: Change `Ord` signatures**

Change `Ord` to `@<: &Self -> Bool`, `@<=: &Self -> Bool`, `@>: &Self -> Bool`, `@>=: &Self -> Bool`; update impls to dereference borrowed RHS.

- [ ] **Step 4: Update `HashMap`**

Import/use `Eq`, constrain `K: Hash, K: Eq`, and compare stored/probe keys through normal equality with borrowed RHS.

### Task 3: Teach Operator Lowering To Borrow RHS

**Files:**
- Modify: `lib/src/lower/expression.rs`

- [ ] **Step 1: Remove eager same-type unification when selected method expects reference RHS**

For trait-backed operators, let selected method parameters drive RHS coercion before reporting same-type mismatch.

- [ ] **Step 2: Add operator RHS autoref helper**

When a selected trait operator method has a reference RHS parameter and the current RHS is not already compatible, wrap the RHS in `HirExprKind::Ref(false, ...)` before `selected_method_call_types` coerces it.

### Task 4: Verify And Clean Up

**Files:**
- Search all repo Rock/Rust files for `EqRef`, `eq_ref`, and `ne_ref`.

- [ ] **Step 1: Run focused integration tests**

Run focused tests for the new `Eq`, `Ord`, and `HashMap` behavior.

- [ ] **Step 2: Run formatting and whitespace checks**

Run: `cargo fmt --all --check`
Run: `git diff --check`

- [ ] **Step 3: Run full relevant tests**

Run: `cargo test -p rock-lib`
