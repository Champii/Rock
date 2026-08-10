# Explicit Unresolved Type Defaults Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hidden `Type::I64` unresolved type fallback in `InferenceEngine::finalize` with an explicit unresolved-type default policy at lenient finalization call sites.

**Architecture:** Strict finalization remains the user-facing path for current-crate inference ambiguity. Lenient artifact/external finalization should call a named API that supplies `Type::I64` as the unresolved default, making the fallback intentional and searchable instead of a legacy implicit behavior inside `finalize`.

**Tech Stack:** Rust 2021, existing `rock-lib` inference tests, `cargo test -p rock-lib`.

---

### Task 1: Make Lenient Type Defaulting Explicit

**Files:**
- Modify: `lib/src/infer/engine.rs`
- Modify: `lib/src/infer/finalize.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [x] **Step 1: Write the failing test**

Add a unit test in `lib/src/infer/engine.rs` that calls `finalize_with_unresolved_default` on a fresh unresolved type variable and asserts it returns the caller-provided default type.

- [x] **Step 2: Run the test to verify RED**

Run: `cargo test -p rock-lib finalize_with_unresolved_default_uses_caller_default`

Expected: FAIL to compile because the explicit defaulting API does not exist yet.

- [x] **Step 3: Implement the explicit API**

Replace `InferenceEngine::finalize` with `InferenceEngine::finalize_with_unresolved_default(&self, ty: &Type, unresolved_default: &Type) -> Type`. Thread the default through recursive finalization. Keep `finalize_strict` unchanged semantically.

- [x] **Step 4: Update lenient call sites**

In `lib/src/infer/finalize.rs`, introduce one local helper for the lenient `Type::I64` default and replace every `engine.finalize(...)` call with the explicit defaulting API. Keep strict `FinalizeCtx::finalize` on `finalize_strict`.

- [x] **Step 5: Run focused verification**

Run: `cargo test -p rock-lib finalize_with_unresolved_default_uses_caller_default`

Run: `cargo test -p rock-lib finalize`

Progress:
- RED confirmed: `cargo test -p rock-lib finalize_with_unresolved_default_uses_caller_default` failed to compile because `InferenceEngine::finalize_with_unresolved_default` did not exist.
- GREEN confirmed: `cargo test -p rock-lib finalize_with_unresolved_default_uses_caller_default` passed after adding the explicit API.
- Regression confirmed: `cargo test -p rock-lib finalize` passed.
- Full verification initially exposed four integration failures because strict finalization now returned `Type::Error` for unspanned internal type variables that previously defaulted to `I64`.
- RED confirmed: `cargo test -p rock-lib finalize_strict_with_unspanned_default_keeps_spanned_ambiguity_errors` failed to compile because the explicit strict unspanned-default API did not exist.
- GREEN confirmed: `cargo test -p rock-lib finalize_strict_with_unspanned_default_keeps_spanned_ambiguity_errors` passed after adding that API and using it from strict HIR finalization.
- Reproduced failures fixed: `test_for_loop`, `test_repeated_receiver_generic_impl_does_not_match_incompatible_receiver_args`, `test_u8_slice_trait_impl_does_not_apply_to_string_literal`, and `test_user_defined_array_does_not_receive_slice_methods` each passed with `--exact`.

- [x] **Step 6: Run full verification**

Run: `cargo fmt --all --check`

Run: `git diff --check`

Run: `cargo test -p rock-lib`

Progress:
- Format confirmed: `cargo fmt --all --check` passed after formatting.
- Diff check confirmed: `git diff --check` passed.
- Full verification confirmed: `cargo test -p rock-lib` passed with unit `1265 passed; 0 failed; 1 ignored`, integration `277 passed; 0 failed`, parser integration `1 passed; 0 failed`, and doctests `1 passed; 0 failed; 1 ignored`.

- [x] **Step 7: Update roadmap/checklist**

Record that unresolved type defaulting is now explicit at lenient finalization call sites and no longer hidden behind `InferenceEngine::finalize`.

- [ ] **Step 8: Commit**

Commit the verified slice with message `make unresolved type defaults explicit`.
