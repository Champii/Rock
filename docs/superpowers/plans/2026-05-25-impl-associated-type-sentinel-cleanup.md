# Impl Associated Type Sentinel Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the collect-time sentinel `AssocTypeId(u32::MAX - index)` path for trait impl associated types that do not match a declared trait associated type.

**Architecture:** Keep trait-declared associated type IDs as the only valid IDs for trait impl associated type definitions. If an impl targets a known trait and defines an undeclared associated type, collection records a structured error instead of fabricating an ID; inherent/non-trait impls keep their existing source-order associated type IDs.

**Tech Stack:** Rust 2021, `rock-lib`, collect header lowering in `lib/src/collect/headers.rs`, collect tests in `lib/src/collect/mod.rs`, focused Cargo tests, `cargo fmt --all --check`, `git diff --check`.

---

## Files

- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/mod.rs`
- Update: `docs/superpowers/plans/master-audit-checklist.md`
- Update: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

## Task 1: Add Regression Coverage For Unknown Trait Associated Type Definitions

- [x] **Step 1: Add a failing collect test**

Add `collect_errors_when_trait_impl_defines_unknown_associated_type` in `lib/src/collect/mod.rs`:

```rust
let errors = match collect(&program, &CrateContext::new(), false, Some("test")) {
    Ok(_) => panic!("unknown trait associated type should be a collection error"),
    Err(errors) => errors,
};

assert!(errors.iter().any(|err| err.message.contains(
    "unknown associated type 'Output'"
)));
```

- [x] **Step 2: Run the focused test and confirm RED**

Run: `cargo test -p rock-lib collect_errors_when_trait_impl_defines_unknown_associated_type`
Observed: FAIL because collection currently returns `Ok(_)`.

## Task 2: Replace Sentinel IDs With A Collection Error

- [x] **Step 1: Remove the sentinel fallback**

In `build_impl_with_id`, replace the `AssocTypeId(u32::MAX - index as u32)` branch with an error push and source-order fallback that cannot masquerade as a generated trait-associated-type ID:

```rust
let id = trait_associated_types
    .iter()
    .find(|decl| decl.name == assoc.name.name)
    .map(|decl| decl.id)
    .unwrap_or_else(|| {
        if trait_def.is_some() {
            context.push_error(format!(
                "unknown associated type '{}' for trait '{}'",
                assoc.name.name,
                trait_name.as_deref().unwrap_or("<unknown>")
            ));
        }
        AssocTypeId(index as u32)
    });
```

- [x] **Step 2: Run the focused test and confirm GREEN**

Run: `cargo test -p rock-lib collect_errors_when_trait_impl_defines_unknown_associated_type`
Expected: PASS.

## Task 3: Preserve Existing Valid Associated Type Behavior

- [x] **Step 1: Run existing associated type ID tests**

Run: `cargo test -p rock-lib associated_type`
Expected: PASS.

- [x] **Step 2: Confirm the remaining direct sentinel text is gone from `headers.rs`**

Run: `git diff -- lib/src/collect/headers.rs`
Expected: no `AssocTypeId(u32::MAX - index as u32)` remains.

## Task 4: Update Tracking Docs And Verify

- [x] **Step 1: Update roadmap/checklist evidence**

Record that impl associated type definitions for known traits now reject undeclared associated type names instead of using sentinel associated type IDs.

- [x] **Step 2: Run focused collect verification**

Run: `cargo test -p rock-lib collect_errors_when_trait_impl_defines_unknown_associated_type`
Expected: PASS.

- [x] **Step 3: Run formatting and diff checks**

Run: `cargo fmt --all --check`
Expected: exit 0.

Run: `git diff --check`
Expected: exit 0.

- [ ] **Step 4: Request focused code review**

Review the diff for missed sentinel paths, incorrect behavior for inherent impls, and test adequacy before commit.

- [ ] **Step 5: Commit verified slice**

Commit message: `reject unknown impl associated types`.
