# Body Lowering Provisional Generic Owners Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop body lowering from silently repairing sentinel generic owners and instead treat `CrateId(u32::MAX)` generic ownership as an invariant failure before body lowering proceeds.

**Architecture:** Function headers and signature handoff are responsible for canonical generic ownership before body lowering. `lower_function_body_qualified` should validate its stored `HirFunction` inputs and reject provisional owners instead of substituting them to `func.id` late.

**Tech Stack:** Rust 2021, existing `rock-lib` lowerer unit tests, `cargo test -p rock-lib`.

---

### Task 1: Reject Provisional Generic Owners In Body Lowering

**Files:**
- Modify: `lib/src/lower/bodies.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [x] **Step 1: Write the failing test**

Add a unit test in `lib/src/lower/bodies.rs` that stores a `HirFunction` with a `GenericParamId` owned by `DefId::new(CrateId(u32::MAX), ...)`, calls `lower_function_body_qualified`, and expects a panic that names the invariant failure.

- [x] **Step 2: Run the test to verify RED**

Run: `cargo test -p rock-lib lower_function_body_rejects_provisional_generic_owner`

Expected: FAIL because current body lowering silently remaps the provisional generic owner.

- [x] **Step 3: Implement minimal validation**

Replace the late substitution block in `lower_function_body_qualified` with a validation pass that collects generic IDs from params, return type, `generic_param_ids`, and generic bounds. Panic if any owner uses `CrateId(u32::MAX)`.

- [x] **Step 4: Run focused verification**

Run: `cargo test -p rock-lib lower_function_body_rejects_provisional_generic_owner`

Expected: PASS.

- [x] **Step 5: Run regression verification**

Run: `cargo test -p rock-lib lower_function_body`

Run: `cargo fmt --all --check`

Run: `git diff --check`

Run: `cargo test -p rock-lib`

Progress:
- RED confirmed: `cargo test -p rock-lib lower_function_body_rejects_provisional_generic_owner` failed because the test did not panic.
- GREEN confirmed: `cargo test -p rock-lib lower_function_body_rejects_provisional_generic_owner` passed after replacing the repair with validation.
- Regression confirmed: `cargo test -p rock-lib lower_function_body` passed.
- Format confirmed: `cargo fmt --all --check` passed after formatting.
- Diff check confirmed: `git diff --check` passed.
- Full verification confirmed: `cargo test -p rock-lib` passed with unit `1263 passed; 0 failed; 1 ignored`, integration `277 passed; 0 failed`, parser integration `1 passed; 0 failed`, and doctests `1 passed; 0 failed; 1 ignored`.

- [x] **Step 6: Update roadmap/checklist**

Record that body lowering no longer owns sentinel generic-owner repair. Keep remaining generated/sentinel work scoped to collect/generated impl/product/inference paths.

- [ ] **Step 7: Commit**

Commit the verified slice with message `reject provisional generic owners in body lowering`.
