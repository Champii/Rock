# Product Zero ID Placeholder Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop product emission from treating `DefId(CrateId(0), LocalDefId(0))` as a placeholder identity for impls and externs.

**Architecture:** Product IDs should reserve requested canonical IDs unless there is a real collision or a true sentinel crate ID such as `ProductCrateId(u32::MAX)`. Remove impl/extern special-casing for local ID zero and let `reserve_product_id` handle collisions uniformly.

**Tech Stack:** Rust 2021, `rock-lib`, product tests in `lib/src/products.rs`, focused Cargo tests, `cargo fmt --all --check`, `git diff --check`.

---

## Files

- Modify: `lib/src/products.rs`
- Update: `docs/superpowers/plans/master-audit-checklist.md`
- Update: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

## Task 1: Add RED Coverage For Real Local ID Zero

- [x] **Step 1: Add a real impl ID zero test**

Add `compiler_products_preserve_real_impl_id_zero_without_collision`:

```rust
let impl_id = DefId::new(CrateId(0), LocalDefId(0));
let hir = ResolvedHirProgram::new(
    HirProgram::from_parts(HashMap::new(), HashMap::new(), HashMap::new(), HashMap::new(), vec![impl_def], Vec::new()),
    ResolverTables::default(),
    BTreeSet::from([impl_id]),
    CrateId(0),
    local_def_ids_after(1),
);

let product_id = ProductDefId::from(impl_id);
assert!(products.metadata.impls.contains_key(&product_id));
```

- [x] **Step 2: Add a real extern ID zero test**

Add `compiler_products_preserve_real_extern_id_zero_without_collision` with a single current-crate extern at `DefId(0, 0)` and assert `products.metadata.externs` contains `ProductDefId(0, 0)`.

- [x] **Step 3: Update backend symbol remap expectation**

Change the backend-symbol local-zero test to assert backend symbols stay attached to real `ProductDefId(0, 0)` when there is no collision.

- [x] **Step 4: Run focused tests and confirm RED**

Run: `cargo test -p rock-lib compiler_products_preserve_real_impl_id_zero_without_collision`
Expected: FAIL because impl ID zero is forced through fallback.

Run: `cargo test -p rock-lib compiler_products_preserve_real_extern_id_zero_without_collision`
Expected: FAIL because extern ID zero is forced through fallback.

## Task 2: Remove Local ID Zero Placeholder Fallbacks

- [x] **Step 1: Remove impl placeholder inclusion/fallback**

In `CompilerProducts::from_resolved_hir`, remove `placeholder_impl_id` and use normal duplicate detection:

```rust
for (index, (id, imp)) in hir.program.impls_in_order().enumerate() {
    if seen_impl_ids.insert(imp.id) {
        impls.push((id, index, imp));
    }
}
```

Call `reserve_product_id(..., false)` for impls.

- [x] **Step 2: Remove extern local-zero placeholder fallback**

Delete the `is_placeholder` calculation for externs and call `reserve_product_id(..., false)`.

- [x] **Step 3: Run focused tests and confirm GREEN**

Run: `cargo test -p rock-lib compiler_products_preserve_real_impl_id_zero_without_collision`
Expected: PASS.

Run: `cargo test -p rock-lib compiler_products_preserve_real_extern_id_zero_without_collision`
Expected: PASS.

Run: `cargo test -p rock-lib compiler_products_preserve_backend_symbols_for_real_local_id_zero`
Expected: PASS with the updated real-ID-zero expectation.

## Task 3: Update Tracking Docs And Verify

- [x] **Step 1: Update roadmap/checklist evidence**

Record that product emission no longer treats `DefId(0, 0)` as an impl/extern placeholder; fallback allocation is now limited to actual collisions and sentinel product crate IDs.

- [x] **Step 2: Run focused product verification**

Run: `cargo test -p rock-lib compiler_products_preserve_real_impl_id_zero_without_collision`
Expected: PASS.

Run: `cargo test -p rock-lib compiler_products_preserve_real_extern_id_zero_without_collision`
Expected: PASS.

Run: `cargo test -p rock-lib compiler_products_preserve_backend_symbols_for_real_local_id_zero`
Expected: PASS.

- [x] **Step 3: Run formatting and diff checks**

Run: `cargo fmt --all --check`
Expected: exit 0.

Run: `git diff --check`
Expected: exit 0.

- [x] **Step 4: Request focused code review**

Review the diff for product ID collisions, backend symbol remapping, and test adequacy before commit.

- [ ] **Step 5: Commit verified slice**

Commit message: `preserve product local id zero`.
