# Collect-Owned Context Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `LocalCollector`'s full `Lowerer` state bag with a narrower collect-owned context while preserving collection outputs and later lowering behavior.

**Architecture:** Add `lib/src/collect/context.rs` to hold collect-time mutable state and move the active collect-time helper methods there. `collect::collect` should still bootstrap dependency crates with `Lowerer`, then hand the relevant state into `CollectContext` so `LocalCollector` and `collect::headers` run without owning a full lowerer.

**Tech Stack:** Rust 2021, `rock-lib`, collect/lower modules, existing collect regression tests, `cargo test -p rock-lib`.

---

## File Structure

- Create: `lib/src/collect/context.rs`
  - Owns `CollectContext`, bootstrap handoff from `Lowerer`, collect-time loader/import/export helpers, and collect-time parse-type/header helpers.
- Modify: `lib/src/collect/mod.rs`
  - Declares the new module and adds a collect-level ownership regression test.
- Modify: `lib/src/collect/collector.rs`
  - Replaces the stored `Lowerer` with `CollectContext`.
- Modify: `lib/src/collect/headers.rs`
  - Uses `CollectContext` instead of `Lowerer`.

## Task 1: Introduce CollectContext And Bootstrap Handoff

**Files:**
- Create: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add a failing ownership-boundary test**

Add `mod context;` to `lib/src/collect/mod.rs` and add a focused test that references `super::context::CollectContext::from_bootstrap(...)`.

- [ ] **Step 2: Run the focused test to confirm it fails**

Run: `cargo test -p rock-lib --lib collect::tests::collect_context_imports_bootstrap_state -- --exact --nocapture`

Expected: FAIL because `collect::context` does not exist yet.

- [ ] **Step 3: Implement the minimal context skeleton**

Create `lib/src/collect/context.rs` with:

- `CollectContext::new(...)`
- `CollectContext::from_bootstrap(lowerer: Lowerer)`
- the collect-time field set needed by the current collector path

- [ ] **Step 4: Re-run the focused test and confirm it passes**

Run: `cargo test -p rock-lib --lib collect::tests::collect_context_imports_bootstrap_state -- --exact --nocapture`

Expected: PASS.

## Task 2: Move Active Collect Helpers To CollectContext

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/headers.rs`

- [ ] **Step 1: Add or update failing header-builder tests**

Update the `collect::headers` unit tests so they construct `CollectContext` instead of `Lowerer`.

- [ ] **Step 2: Run the header tests to confirm they fail for the right reason**

Run: `cargo test -p rock-lib --lib collect::headers::tests -- --nocapture`

Expected: FAIL because `CollectContext` does not yet expose the helper methods used by the builders.

- [ ] **Step 3: Move the active helper methods into CollectContext**

Implement collect-time equivalents of the helpers used by the active collect path:

- parse-type lowering
- self/parameter header helpers
- impl header type helpers
- local module loader helpers
- import/glob-import helpers
- glob-export expansion helpers

- [ ] **Step 4: Switch `collect::headers` to `&mut CollectContext` and keep all header tests green**

Run: `cargo test -p rock-lib --lib collect::headers::tests -- --nocapture`

Expected: PASS.

## Task 3: Switch LocalCollector To CollectContext

**Files:**
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Reuse the existing collect regressions as characterization coverage**

Run the narrow collect suite before the refactor:

`cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: PASS baseline.

- [ ] **Step 2: Replace `LocalCollector`'s stored `Lowerer` with `CollectContext`**

Update:

- constructor
- bootstrap import path
- header insertion helpers
- import/glob-import helpers
- source-backed module loading helpers
- finish/destructuring path

- [ ] **Step 3: Re-run the collect suite**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: PASS.

## Task 4: Full Verification

**Files:**
- Verify only unless formatting changes touched files.

- [ ] **Step 1: Run formatting**

Run: `cargo fmt --all`

- [ ] **Step 2: Run header tests**

Run: `cargo test -p rock-lib --lib collect::headers::tests -- --nocapture`

- [ ] **Step 3: Run collect tests**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

- [ ] **Step 4: Run the full package suite**

Run: `cargo test -p rock-lib`

If Cargo reports stale incremental artifacts, run `cargo clean -p rock-lib` and retry the same command.
