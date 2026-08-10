# Legacy Signature Fallback Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the late `lower_function_sig` fallback that silently allocated a fresh current-crate-looking `DefId` when no canonical signature identity was available.

**Architecture:** Keep `lower_function_sig` as a consumer of canonical resolver identity. Missing signature identity is now a hard invariant failure instead of a post-collection current-crate ID allocation, matching the supported collect-to-lower pipeline's ID authority rule.

**Tech Stack:** Rust 2021, `rock-lib`, lowerer function-signature lowering, focused Cargo tests.

---

## File Structure

- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/bodies.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

---

### Task 1: Add Regression Coverage

**Files:**
- Modify: `lib/src/lower/function.rs`

- [x] **Step 1: Write the failing test**

Added `lower_function_sig_panics_without_current_trait_signature_identity`, which places the lowerer in a trait/generic context without resolver signature identity and expects `lower_function_sig` to reject that missing canonical identity.

Added `lower_function_sig_rejects_top_level_collision_in_trait_context`, which proves trait-context signature lowering does not borrow an unrelated top-level signature ID with the same name.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p rock-lib lower_function_sig_panics_without_current_trait_signature_identity
```

Result before implementation: FAIL because the test did not panic; `lower_function_sig` silently allocated a fresh current-crate ID.

---

### Task 2: Remove The Fallback Allocation

**Files:**
- Modify: `lib/src/lower/function.rs`

- [x] **Step 1: Replace fresh-ID fallback with invariant failure**

Changed the signature ID lookup to panic with `missing canonical function signature identity for <name>` when neither the current-trait qualified path nor the direct resolver path provides an ID.

- [x] **Step 2: Verify focused behavior**

Run:

```bash
cargo test -p rock-lib lower_function_sig_panics_without_current_trait_signature_identity
cargo test -p rock-lib type_var_signature_call_uses_signature_id_target
cargo test -p rock-lib current_trait_self_signature_call_uses_signature_return_type
cargo test -p rock-lib lower::function
```

Result: PASS.

- [x] **Step 3: Keep legacy callers explicit**

Full-suite verification showed legacy trait/impl signature callers still needed explicit IDs. Added `lower_function_sig_with_id`, kept `lower_function_sig` as the canonical-ID-resolving entry point, and changed legacy callers with known method/signature IDs to call the explicit API.

---

### Task 3: Update Tracking Docs

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [x] **Step 1: Record the completed cleanup**

Updated the Identity And Arenas evidence/done entries and Task 1 reconciliation row to show `lower_function_sig` no longer allocates fallback current-crate IDs for missing signature identity.

---

## Verification

- PASS: `cargo test -p rock-lib lower_function_sig_panics_without_current_trait_signature_identity`.
- PASS: `cargo test -p rock-lib lower_function_sig`.
- PASS: `cargo test -p rock-lib type_var_signature_call_uses_signature_id_target`.
- PASS: `cargo test -p rock-lib current_trait_self_signature_call_uses_signature_return_type`.
- PASS: `cargo test -p rock-lib lower::function`.
- PASS: `cargo test -p rock-lib build_hir_trait_keeps_declared_generic_ids_zero_based_despite_self_context`.
- PASS: `cargo test -p rock-lib test_stdlib_product_artifact_links_static_impl_method`.
- PASS: `cargo fmt --all --check`.
- PASS: `git diff --check`.
- PASS: `cargo test -p rock-lib` completed with unit `1260 passed; 0 failed; 1 ignored`, integration `277 passed; 0 failed`, parser integration `1 passed`, and doctests `1 passed; 1 ignored`.
