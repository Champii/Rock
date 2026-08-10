# Auto Sized Impl Provenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure lowerer-generated auto `Sized` impls are created only for current-crate nominal owners with explicit current-definition provenance.

**Architecture:** Treat dependency `Sized` impls as dependency metadata, not consumer-generated declarations. `auto_impl_sized` should resolve each nominal owner first, skip non-current owners, then allocate a generated current-crate impl ID and record it in `current_def_ids` for current owners only.

**Tech Stack:** Rust 2021, `rock-lib`, lowerer trait conformance tests in `lib/src/lower/traits/conformance.rs`, focused Cargo tests, `cargo fmt --all --check`, `git diff --check`.

---

## Files

- Modify: `lib/src/lower/traits/conformance.rs`
- Update: `docs/superpowers/plans/master-audit-checklist.md`
- Update: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

## Task 1: Add Foreign Owner Regression Coverage

- [x] **Step 1: Add a failing auto Sized test**

Add `auto_impl_sized_skips_foreign_structs_without_current_provenance` near existing `auto_impl_sized_*` tests:

```rust
let foreign_point_id = DefId::new(CrateId(7), LocalDefId(21));
lowerer
    .structs
    .insert("dep::Point".to_string(), point_struct(foreign_point_id));
lowerer
    .resolver
    .item_paths
    .insert("dep::Point".to_string(), foreign_point_id);
lowerer
    .resolver
    .item_names_by_id
    .insert(foreign_point_id, "dep::Point".to_string());

lowerer.auto_impl_sized();

assert!(lowerer
    .impls
    .iter()
    .all(|imp| !(imp.type_name == "dep::Point" && imp.trait_id == Some(sized_id))));
```

- [x] **Step 2: Run the focused test and confirm RED**

Run: `cargo test -p rock-lib auto_impl_sized_skips_foreign_structs_without_current_provenance`
Expected: FAIL because the current lowerer generates a consumer-owned `Sized` impl for `dep::Point`.

## Task 2: Require Current Owner Provenance Before Allocating Generated Impl IDs

- [x] **Step 1: Move owner resolution before generated ID allocation**

In `auto_impl_sized`, resolve `owner_id` before calling `self.local_def_ids.fresh()`:

```rust
let Some(owner_id) = self.resolve_owner_def_id(&name) else {
    continue;
};
if !self.current_def_ids.contains(&owner_id) {
    continue;
}
let Some(owner_path) = self.try_canonical_owner_path(&name) else {
    continue;
};
let id = DefId::new(self.root_crate_id, self.local_def_ids.fresh());
self.current_def_ids.insert(id);
```

- [x] **Step 2: Preserve current owner behavior**

Run: `cargo test -p rock-lib auto_impl_sized_uses_stdlib_prelude_export_id`
Expected: PASS.

- [x] **Step 3: Run the new focused test and confirm GREEN**

Run: `cargo test -p rock-lib auto_impl_sized_skips_foreign_structs_without_current_provenance`
Expected: PASS.

## Task 3: Update Tracking Docs And Verify

- [x] **Step 1: Update roadmap/checklist evidence**

Record that auto `Sized` impl generation requires current-owner provenance before allocating a generated current-crate impl ID.

- [x] **Step 2: Run focused auto Sized verification**

Run: `cargo test -p rock-lib auto_impl_sized`
Expected: PASS.

- [x] **Step 3: Run formatting and diff checks**

Run: `cargo fmt --all --check`
Expected: exit 0.

Run: `git diff --check`
Expected: exit 0.

- [x] **Step 4: Request focused code review**

Review the diff for skipped dependency behavior, generated ID provenance, and test adequacy before commit.

- [ ] **Step 5: Commit verified slice**

Commit message: `require provenance for auto sized impls`.
