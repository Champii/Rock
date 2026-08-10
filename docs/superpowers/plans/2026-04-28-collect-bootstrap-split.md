# Collect-Owned Bootstrap Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the last `collect::collect -> Lowerer` bootstrap dependency by moving dependency crate registration and stdlib prelude bootstrap into `collect`.

**Architecture:** Extend `CollectContext` with collect-owned bootstrap and dependency registration helpers, switch `collect::collect` to build bootstrap state directly in `collect`, and keep `Lowerer::from_declarations` and later body-lowering behavior unchanged.

**Tech Stack:** Rust 2021, `rock-lib`, `collect`, `lower`, crate-system/artifact interfaces, focused collect tests, full `cargo test -p rock-lib`.

---

## File Structure

- Modify: `lib/src/collect/context.rs`
  - Add collect-owned bootstrap constructor, dependency registration helpers, and collect-owned source-backed dependency declaration traversal.
- Modify: `lib/src/collect/collector.rs`
  - Accept an already-bootstrapped `CollectContext` instead of importing state from `Lowerer`.
- Modify: `lib/src/collect/mod.rs`
  - Remove the bootstrap `Lowerer` path, add focused boundary tests, and wire `collect::collect` to direct collect-owned bootstrap.
- Modify: `lib/src/lower/mod.rs`
  - Remove collect-only bootstrap helpers if they become unused, while leaving body-lowering behavior unchanged.

## Task 1: Add Failing Collect-Owned Bootstrap Tests

**Files:**
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add a failing source-backed dependency bootstrap test**

Add a focused collect-level test that expects `CollectContext` to register a source-backed dependency crate without going through `Lowerer`.

- [ ] **Step 2: Add a failing stdlib prelude bootstrap test**

Add a focused collect-level test that expects `CollectContext` to register stdlib prelude exports and inject prelude aliases without going through `Lowerer`.

- [ ] **Step 3: Run the focused collect tests to confirm they fail for the missing collect-owned bootstrap API**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: FAIL because `CollectContext` does not yet expose the new bootstrap methods.

## Task 2: Implement Collect-Owned Bootstrap Helpers

**Files:**
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Add a collect-owned bootstrap constructor**

Implement a `CollectContext::bootstrap_for_collection(...)` entrypoint that initializes crate/module bootstrap state directly in `collect`.

- [ ] **Step 2: Add collect-owned dependency registration helpers**

Implement collect-owned equivalents of the dependency bootstrap surface used by `collect::collect`:

- register dependency crates
- register one loaded crate
- collect declarations from source-backed dependency crates
- inject stdlib prelude aliases from collected prelude exports

- [ ] **Step 3: Re-run the focused collect tests**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: PASS.

## Task 3: Switch `collect::collect` Off `Lowerer`

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/lower/mod.rs`

- [ ] **Step 1: Replace the bootstrap `Lowerer` construction in `collect::collect`**

Make `collect::collect` bootstrap a `CollectContext` directly, register dependencies/prelude there, then pass that context into `LocalCollector`.

- [ ] **Step 2: Remove the collector handoff that imports state from `Lowerer`**

Update `LocalCollector` to accept an already-bootstrapped `CollectContext` directly.

- [ ] **Step 3: Remove or stop using collect-only lower bootstrap helpers if they are now dead**

Keep the edit minimal and do not disturb body-lowering paths.

- [ ] **Step 4: Re-run the collect suite**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

Expected: PASS.

## Task 4: Full Verification

**Files:**
- Verify only unless formatting changes touched files.

- [ ] **Step 1: Run formatting**

Run: `cargo fmt --all`

- [ ] **Step 2: Run collect tests**

Run: `cargo test -p rock-lib --lib collect::tests -- --nocapture`

- [ ] **Step 3: Run the full package suite**

Run: `cargo test -p rock-lib`

If Cargo reports stale incremental artifacts, run `cargo clean -p rock-lib` and retry the same command.
