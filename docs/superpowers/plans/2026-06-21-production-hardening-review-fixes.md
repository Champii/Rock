# Production Hardening Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the review findings that block merging the RAII/implicit-receiver branch into long-term production compiler code.

**Architecture:** Address root causes in small clusters with failing tests first. MIR ownership fixes must preserve move/drop invariants; operator lowering must preserve actual expression types and prove coercions; stdlib safe constructors must not expose null or invalid ZST heap pointers; codegen must fail loudly when required drop glue is missing.

**Tech Stack:** Rust `rock-lib` compiler code, Rock stdlib, integration tests in `lib/tests/integration.rs`, focused unit tests near MIR/codegen where practical.

---

### Task 1: Move/Drop MIR Correctness

**Files:**
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/mir/builder/blocks.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing tests**

Add tests that expose duplicated ownership through move receivers or by-value method args, partial moves from direct-`Drop` types, and array element moves from cleanup arrays.

- [ ] **Step 2: Verify red**

Run focused integration tests. Expected: existing branch either double-drops/prints wrong counts or accepts code that must be rejected.

- [ ] **Step 3: Implement ownership-correct call operands**

In method call MIR lowering, choose `Operand::Move` for move receivers and by-value non-`Copy` method args, while keeping borrowed/reference args as `Copy`.

- [ ] **Step 4: Reject partial moves from direct-`Drop` types**

Emit a structured lowering/borrow diagnostic when moving a field/descendant out of a type with a direct `Drop` impl, rather than skipping the parent destructor.

- [ ] **Step 5: Make array element moves conservative**

Reject moves from arrays whose element type needs cleanup until robust per-index drop flags exist.

- [ ] **Step 6: Verify green**

Run the focused tests again and then the relevant borrow/drop integration filter.

### Task 2: Generic Operator RHS Type Safety

**Files:**
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/selection/service.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing tests**

Add a negative test for `left: T, right: U, where T: Eq` using `left == right`; it must reject unrelated RHS types. Add a positive same-type generic comparison to protect the intended borrowed RHS behavior.

- [ ] **Step 2: Verify red**

Run the negative focused test. Expected: it currently compiles or fails for the wrong reason because autoref stamps the expected type.

- [ ] **Step 3: Preserve actual ref type**

Change operator autoref to construct `&actual_arg_type` instead of assigning the expected parameter type to the ref expression.

- [ ] **Step 4: Apply trait receiver substitution in selection**

When selecting trait signatures for generics, substitute hidden `Self`/trait generics into params before returning `substituted_params`.

- [ ] **Step 5: Verify green**

Run the new negative and positive tests plus existing generic operator tests.

### Task 3: Safe Allocation And ZST Policy

**Files:**
- Modify: `stdlib/alloc.rk`
- Modify: `stdlib/raw_buffer.rk`
- Modify: `stdlib/box_type.rk`
- Modify: `stdlib/vec.rk`
- Modify: `stdlib/hash_map.rk`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing tests**

Add tests for zero-sized `Box`, `Vec`, and `HashMap` behavior or explicit rejection. Add tests for safe constructors not returning null-backed owned containers where observable.

- [ ] **Step 2: Verify red**

Run the focused ZST tests. Expected: current code traps, misbehaves, or compiles unsafe ZST container operations.

- [ ] **Step 3: Choose conservative ZST rejection**

For this branch, reject heap/container element sizes of zero in safe constructors and growth paths rather than inventing sentinel pointer semantics.

- [ ] **Step 4: Add checked allocation wrapper**

Keep raw `Global::alloc` unsafe, but make safe owned abstractions use a checked helper that traps/aborts on null for non-zero allocations.

- [ ] **Step 5: Verify green**

Run focused allocation/ZST tests and stdlib container tests.

### Task 4: Drop Glue And Read-Only API Hardening

**Files:**
- Modify: `lib/src/codegen/mir_llvm/terminator.rs`
- Modify: `stdlib/hash_map.rk`
- Modify: call sites/tests in `lib/tests/integration.rs`

- [ ] **Step 1: Write failing tests**

Add a unit test or integration path that verifies missing required drop glue becomes an error. Add tests showing `HashMap::get` and `contains_key` accept borrowed keys without consuming owned probes.

- [ ] **Step 2: Verify red**

Run focused tests. Expected: missing drop glue is currently silent; borrowed lookup signatures do not exist.

- [ ] **Step 3: Make missing drop glue fatal**

If backend metadata says a drop glue symbol exists but codegen cannot find it, return `CodegenError`.

- [ ] **Step 4: Add borrowed HashMap lookup APIs**

Change `get` and `contains_key` to accept `&K` and update internal comparisons/call sites accordingly.

- [ ] **Step 5: Verify green**

Run focused codegen/HashMap tests and the artifact filter.

### Task 5: Final Verification

**Files:**
- All touched files

- [ ] **Step 1: Search for review leftovers**

Search for `EqRef`, stale `TODO`, silent drop-glue branches, and null-backed safe constructor paths.

- [ ] **Step 2: Run formatting and whitespace checks**

Run `cargo fmt --all --check` and `git diff --check`.

- [ ] **Step 3: Run full tests**

Run `cargo test -p rock-lib` and `cargo test -p rock artifact`.
