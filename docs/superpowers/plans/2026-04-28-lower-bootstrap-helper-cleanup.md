# Lower Bootstrap Helper Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the dead lower bootstrap bridge helpers and verify the bootstrap cleanup seam no longer emits dead-code warnings.

**Architecture:** Keep collect as the only owner of collect-time bootstrap construction, delete the obsolete lower/bootstrap bridge, and replace the old bridge test with collect-owned state-carriage coverage. Preserve all active lower-owned body-lowering helpers and compiler behavior.

**Tech Stack:** Rust 2021, `rock-lib`, `collect`, focused warning-gate verification, full `cargo test -p rock-lib`.

---

## File Structure

- Modify: `lib/src/lower/mod.rs`
  - Remove the dead `Lowerer::bootstrap_for_collection(...)` helper.
- Modify: `lib/src/collect/context.rs`
  - Remove the dead `CollectContext::from_bootstrap(...)` helper and any now-unused imports.
- Modify: `lib/src/collect/mod.rs`
  - Replace the obsolete bridge test with collect-owned coverage.

## Task 1: Establish The Warning-Gate Red State

**Files:**
- Verify only

- [ ] **Step 1: Run a focused warning gate before code changes**

Run: `RUSTFLAGS="-D warnings" cargo test -p rock-lib collect::tests::collect_module_defines_local_collector -- --exact`

Expected: FAIL because the crate still emits dead-code warnings for the bridge helpers.

## Task 2: Remove The Dead Bootstrap Bridge

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Remove the lower-owned bootstrap constructor**

Delete `Lowerer::bootstrap_for_collection(...)` from `lib/src/lower/mod.rs`.

- [ ] **Step 2: Remove the collect-side bridge importer**

Delete `CollectContext::from_bootstrap(...)` from `lib/src/collect/context.rs`.

- [ ] **Step 3: Remove any now-unused imports**

Clean up the now-unused `Lowerer` import in `collect/context.rs` if nothing else still references it.

## Task 3: Replace Obsolete Bridge Test Coverage

**Files:**
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Replace the bridge-specific test**

Replace `collect_context_imports_bootstrap_state` with a collect-owned version that:

- creates a `CollectContext` directly
- seeds `import_aliases`, `stdlib_prelude_exports`, `artifact_module_index`, and `export_function_aliases`
- converts it through `into_local_collection()`
- asserts those fields are preserved in the resulting collection

- [ ] **Step 2: Keep the test narrow**

Do not add new production helpers for the sake of the test.

## Task 4: Verify The Cleanup

**Files:**
- Verify only unless formatting changes touched files.

- [ ] **Step 1: Re-run the focused warning gate**

Run: `RUSTFLAGS="-D warnings" cargo test -p rock-lib collect::tests::collect_module_defines_local_collector -- --exact`

Expected: PASS.

- [ ] **Step 2: Run focused collect tests**

Run: `cargo test -p rock-lib collect::tests::collect_context_imports_bootstrap_state -- --exact`

Run: `cargo test -p rock-lib collect::tests::collect_context_bootstraps_source_backed_dependency_crate -- --exact`

Run: `cargo test -p rock-lib collect::tests::collect_context_bootstraps_interface_stdlib_prelude -- --exact`

- [ ] **Step 3: Run formatting and the full package suite**

Run: `cargo fmt --all`

Run: `cargo test -p rock-lib`

If Cargo reports stale incremental artifacts, run `cargo clean -p rock-lib` and retry the same command.
