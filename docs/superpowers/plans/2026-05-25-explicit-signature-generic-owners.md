# Explicit Signature Generic Owners Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the lowerer-side sentinel-owner repair for signature-backed function generics by making explicit signature IDs the generic owner and remapping by signature identity when a declaration consumes a signature.

**Architecture:** `lower_function_sig_with_id` should treat its explicit `DefId` as the signature identity for standalone signature generics whenever no enclosing generic owner exists. `lower_function_decl_header_with_sig_and_id` should remap generic IDs owned by `sig.id` to the concrete function ID, instead of scanning for `CrateId(u32::MAX)` provisional owners.

**Tech Stack:** Rust 2021, existing `rock-lib` lowerer unit tests, `cargo test -p rock-lib`.

---

### Task 1: Make Signature Generic Ownership Explicit

**Files:**
- Modify: `lib/src/lower/function.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [x] **Step 1: Write failing tests**

Add tests in `lib/src/lower/function.rs` that assert `lower_function_sig_with_id` uses the explicit signature ID for inferred generic parameters and that `lower_function_decl_header_with_sig_and_id` remaps signature-owned generics to the concrete function ID.

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test -p rock-lib lower_function_sig_with_id_uses_explicit_id_for_signature_generics lower_function_decl_header_remaps_signature_owned_generics_to_function_id`

Expected: FAIL because generics are still owned by the resolver-selected ID or unchanged signature ID.

- [x] **Step 3: Implement minimal lowerer change**

Change `lower_function_sig_with_id` so the no-enclosing-owner path uses `signature_id` as the temporary generic owner. Change `lower_function_decl_header_with_sig_and_id` so it collects generic IDs whose owner is `sig.id` and remaps those to the passed function ID.

- [x] **Step 4: Run focused verification**

Run: `cargo test -p rock-lib lower_function_sig_with_id_uses_explicit_id_for_signature_generics lower_function_decl_header_remaps_signature_owned_generics_to_function_id`

Expected: PASS.

- [x] **Step 5: Run regression verification**

Run: `cargo test -p rock-lib lower_function_sig`

Run: `cargo test -p rock-lib collect_remaps_signature_backed_function_generics_to_function_owner`

Run: `cargo fmt --all --check`

Run: `git diff --check`

Progress:
- RED confirmed: `cargo test -p rock-lib lower_function_sig_with_id_uses_explicit_id_for_signature_generics` failed because generic owner was resolver `LocalDefId(11)` instead of explicit signature `LocalDefId(12)`.
- RED confirmed: `cargo test -p rock-lib lower_function_decl_header_remaps_signature_owned_generics_to_function_id` failed because generic owner stayed signature `LocalDefId(13)` instead of function `LocalDefId(14)`.
- GREEN confirmed: both focused tests passed after the lowerer change.
- Regression confirmed: `cargo test -p rock-lib lower_function_sig` passed.
- Regression confirmed: `cargo test -p rock-lib collect_remaps_signature_backed_function_generics_to_function_owner` passed.
- Format confirmed: `cargo fmt --all --check` passed after formatting.
- Diff check confirmed: `git diff --check` passed.
- Full verification confirmed: `cargo test -p rock-lib` passed with unit `1262 passed; 0 failed; 1 ignored`, integration `277 passed; 0 failed`, parser integration `1 passed; 0 failed`, and doctests `1 passed; 0 failed; 1 ignored`.

- [x] **Step 6: Update roadmap/checklist**

Record that explicit signature generic ownership no longer depends on sentinel `CrateId(u32::MAX)` repair in the lowerer.

- [ ] **Step 7: Commit**

Commit the verified slice with message `remove signature generic sentinel remap`.
