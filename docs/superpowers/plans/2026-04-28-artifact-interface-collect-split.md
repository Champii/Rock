# Artifact Interface Collect Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move artifact interface declaration/bootstrap ownership from `Lowerer` to `collect` while preserving the later artifact-specific ABI/body refinement path.

**Architecture:** Add a collect-owned artifact declaration helper that bootstraps dependency crates except the crate currently being built, seeds current-crate source/module state, and collects crate-qualified declarations into `Declarations`. Then rebuild a `Lowerer` from those declarations with `Lowerer::from_declarations(...)` for the existing later artifact refinement steps.

**Tech Stack:** Rust 2021, `rock-lib`, `collect`, `crate_artifact`, `lower`, focused crate-artifact tests, full `cargo test -p rock-lib`.

---

## File Structure

- Modify: `lib/src/collect/mod.rs`
  - Add a collect-owned artifact declaration entry point and focused boundary tests.
- Modify: `lib/src/collect/context.rs`
  - Add the artifact-specific dependency bootstrap helper that can skip the crate currently being built.
- Modify: `lib/src/crate_artifact/build.rs`
  - Replace the bootstrap declaration `Lowerer` path with collect-owned declarations plus `Lowerer::from_declarations(...)` re-entry.
- Modify: `lib/src/crate_artifact/tests.rs`
  - Add focused regression coverage for the self-skip artifact bootstrap seam if the most natural test lives with existing artifact tests.

## Task 1: Add Failing Coverage For The Artifact Collect Seam

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Add a focused failing test for self-skip artifact bootstrap**

Add coverage proving the new artifact declaration path does not preload the current crate as a dependency before collecting it.

- [ ] **Step 2: Run the narrow test to verify it fails for the missing helper**

Run: `cargo test -p rock-lib crate_artifact::tests::test_build_artifact_interface_does_not_double_register_current_crate -- --exact`

Expected: FAIL because artifact interface building still bootstraps declarations through a `Lowerer` and has no collect-owned self-skip path.

## Task 2: Add The Collect-Owned Artifact Declaration Helper

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add dependency bootstrap that can skip one crate name**

Extend collect-owned bootstrap helpers with an artifact-safe path that registers dependency crates except `self`.

- [ ] **Step 2: Add a collect-owned artifact declaration entry point**

Create a helper that:

- bootstraps `CollectContext`
- seeds current-crate root/module cache state
- registers dependencies except the crate being built
- collects current-crate qualified declarations
- returns `Declarations`

- [ ] **Step 3: Run the focused test again**

Run: `cargo test -p rock-lib crate_artifact::tests::test_build_artifact_interface_does_not_double_register_current_crate -- --exact`

Expected: PASS.

## Task 3: Switch Artifact Interface Building To Collect

**Files:**
- Modify: `lib/src/crate_artifact/build.rs`

- [ ] **Step 1: Replace the declaration bootstrap `Lowerer` with collect-owned declarations**

Make `build_interface(...)` gather declarations through the new collect helper.

- [ ] **Step 2: Re-enter the later artifact refinement pass through `Lowerer::from_declarations(...)`**

Preserve the current behavior that infers concrete impl method ABIs and checks conformance before harvesting the exported interface.

- [ ] **Step 3: Run the focused artifact interface regression test**

Run: `cargo test -p rock-lib crate_artifact::tests::test_build_stdlib_artifact_exports -- --exact`

Expected: PASS.

## Task 4: Broaden Verification

**Files:**
- Verify only unless formatting changes touched files.

- [ ] **Step 1: Run formatting**

Run: `cargo fmt --all`

- [ ] **Step 2: Run focused artifact tests**

Run: `cargo test -p rock-lib crate_artifact::tests::test_build_artifact_interface_does_not_double_register_current_crate -- --exact`

Run: `cargo test -p rock-lib crate_artifact::tests::test_build_stdlib_artifact_exports -- --exact`

Run: `cargo test -p rock-lib crate_artifact::tests::test_compile_with_stdlib_artifact -- --exact`

- [ ] **Step 3: Run the full package suite**

Run: `cargo test -p rock-lib`

If Cargo reports stale incremental artifacts, run `cargo clean -p rock-lib` and retry the same command.
