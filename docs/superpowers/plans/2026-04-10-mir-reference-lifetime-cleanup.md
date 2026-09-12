# MIR Reference Lifetime Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make reference lifetimes a first-class MIR pass so borrow checking uses explicit lifetime boundaries instead of heuristic loan pruning.

**Architecture:** MIR lowering keeps building plain control flow and reference assignments. A dedicated reference-lifetime pass computes the last real use of each reference across copies and control-flow edges, then inserts or rewrites `StorageDead` for reference locals. Borrow checking then consumes that finalized MIR and only reacts to explicit `StorageDead` and `Ref` statements.

**Tech Stack:** Rust, existing MIR builder/passes, integration tests in `lib/tests/integration.rs`.

---

### Task 1: Lock the two behavioral regressions in tests

**Files:**
- Modify: `lib/tests/integration.rs`
- Modify: `examples/mir_tests/reference_lifetime_cleanup.rk`
- Modify: `examples/mir_tests/mut_and_shared.rk`

- [ ] **Step 1: Add or keep one test that should pass when the lifetime pass shortens references correctly**

```rust
#[test]
fn test_reference_lifetime_cleanup() {
    compile_example_should_pass("mir_tests/reference_lifetime_cleanup");
}
```

- [ ] **Step 2: Add or keep one test that should fail when shared and mutable borrows overlap**

```rust
#[test]
fn test_borrow_mut_and_shared() {
    compile_example_should_fail("mir_tests/mut_and_shared", "cannot borrow");
}
```

- [ ] **Step 3: Run the two tests and confirm the current state before changing production code**

Run: `cargo test -p rock-lib test_reference_lifetime_cleanup --test integration -- --exact --nocapture`

Expected: `PASS`

Run: `cargo test -p rock-lib test_borrow_mut_and_shared --test integration -- --exact --nocapture`

Expected: `FAIL` with a borrow error

### Task 2: Make reference lifetimes a dedicated MIR pass

**Files:**
- Modify: `lib/src/mir/passes/reference_lifetimes.rs`

- [ ] **Step 1: Keep the pass focused on reference locals only**

```rust
fn reference_locals(func: &MirFunction) -> Vec<Local> {
    func.local_decls
        .iter()
        .enumerate()
        .filter_map(|(idx, decl)| matches!(decl.ty, Type::Reference { .. }).then_some(Local(idx)))
        .collect()
}
```

- [ ] **Step 2: Compute last use through reference-copy chains, but do not treat non-reference destinations as lifetime roots**

```rust
fn latest_reachable_use(
    local: Local,
    direct_uses: &HashMap<Local, UseLocation>,
    copy_graph: &HashMap<Local, Vec<Local>>,
) -> Option<UseLocation> {
    // Walk only through locals whose type is a reference.
}
```

- [ ] **Step 3: Rewrite or insert `StorageDead` for reference locals at the computed last use**

```rust
match last_use {
    Some(UseLocation::Statement { block_idx, stmt_idx }) => insert_after_stmt(block_idx, stmt_idx, StorageDead(local));
    Some(UseLocation::Terminator { block_idx }) => insert_at_block_end(block_idx, StorageDead(local));
    None => {}
}
```

- [ ] **Step 4: Run the cleanup test after the pass change**

Run: `cargo test -p rock-lib test_reference_lifetime_cleanup --test integration -- --exact --nocapture`

Expected: `PASS`

### Task 3: Remove borrow-checker lifetime heuristics

**Files:**
- Modify: `lib/src/mir/passes/borrow_check/mod.rs`

- [ ] **Step 1: Delete the use-count pruning path from borrow checking**

```rust
// Remove total_use_counts, remaining_uses, and prune_dead_reference_loans.
// Keep only explicit StorageDead handling and aliasing checks for Ref statements.
```

- [ ] **Step 2: Keep loan collection simple: `Rvalue::Ref` creates a loan, everything else is just normal use checking**

```rust
if let StatementKind::Assign(_, Rvalue::Ref(mutability, place)) = &stmt.kind {
    let kind = LoanKind::from(*mutability);
    LoanAnalysis::check_aliasing(place, kind, &active_loans)?;
}
```

- [ ] **Step 3: Remove any now-unused helpers and imports**

```rust
// Delete collect_total_use_counts, prune_dead_reference_loans,
// prune_loans_dead_before_stmt, consume_statement_uses, consume_terminator_uses
// if they are no longer referenced.
```

- [ ] **Step 4: Run the shared/mutable borrow test and confirm it now fails for the right reason**

Run: `cargo test -p rock-lib test_borrow_mut_and_shared --test integration -- --exact --nocapture`

Expected: `FAIL` with a borrow conflict

### Task 4: Verify the pipeline end to end

**Files:**
- Modify: none

- [ ] **Step 1: Run both targeted regressions back to back**

Run: `cargo test -p rock-lib test_reference_lifetime_cleanup --test integration -- --exact --nocapture`

Run: `cargo test -p rock-lib test_borrow_mut_and_shared --test integration -- --exact --nocapture`

Expected: cleanup passes, shared/mutable fails

- [ ] **Step 2: Run the smallest broader suite that covers the same MIR path**

Run: `cargo test -p rock-lib --test integration`

Expected: no new failures in MIR/borrow tests

- [ ] **Step 3: Inspect warnings for anything introduced by the refactor**

Expected: only pre-existing warnings remain, or unused helpers are removed if they became dead code
