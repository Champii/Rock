# Builtin Index Selection No Pseudo DefId Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop representing builtin index selection as a fabricated `HirFunction` with a sentinel `DefId`.

**Architecture:** `SelectedMethod` should carry a callable `HirFunction` only when selection resolves a real function-like method. Builtin indexing keeps its receiver, return type, origin, and `builtin_index_output`, but its callable function is absent because codegen/MIR lower builtin indexing through explicit index forms rather than a method body.

**Tech Stack:** Rust 2021, `rock-lib`, selection service, lowerer index lowering, focused selection and integration tests, `cargo fmt --all --check`, `git diff --check`.

---

## Files

- Modify: `lib/src/selection/types.rs`
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/2026-05-25-current-crate-defid-repair-cleanup.md`

---

### Task 1: Add the Red Selection Test

**Files:**
- Modify: `lib/src/selection/service.rs`

- [x] **Step 1: Write the failing test**

Add this test to `#[cfg(test)] mod tests` in `lib/src/selection/service.rs`:

```rust
    #[test]
    fn builtin_index_selection_does_not_fabricate_hir_function() {
        let traits = HashMap::new();
        let impls = Vec::new();
        let methods = HashMap::new();
        let resolver = ResolverTables::default();
        let dependency_resolvers = HashMap::new();
        let current_impl_bounds = HashMap::new();
        let service = SelectionService::new(
            &traits,
            &impls,
            &methods,
            &resolver,
            &dependency_resolvers,
            None,
            &current_impl_bounds,
        );
        let receiver = HirExpr {
            kind: HirExprKind::Var("items".to_string()),
            ty: Type::Array(Box::new(Type::I64), 4),
            span: Span::default(),
        };

        let selected = service
            .select_builtin_index_method(&[receiver], &Type::I64, |ty| ty.clone())
            .expect("array index selection should use builtin index support");

        assert!(selected.function.is_none());
        assert_eq!(selected.origin, SelectedOrigin::BuiltinIndex);
        assert_eq!(selected.builtin_index_output, Some(Type::I64));
    }
```

- [x] **Step 2: Run the focused test to verify it fails**

Run:

```bash
cargo test -p rock-lib selection::service::tests::builtin_index_selection_does_not_fabricate_hir_function -- --exact
```

Expected: FAIL to compile because `SelectedMethod::function` is still a concrete `HirFunction` and does not support `is_none()`.

---

### Task 2: Make Selected Callables Optional

**Files:**
- Modify: `lib/src/selection/types.rs`
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`

- [x] **Step 1: Change `SelectedMethod::function` type**

In `lib/src/selection/types.rs`, change:

```rust
    pub function: HirFunction,
```

to:

```rust
    pub function: Option<HirFunction>,
```

- [x] **Step 2: Wrap real selected methods in `Some`**

In `lib/src/selection/service.rs`, update every non-builtin `SelectedMethod` construction to use:

```rust
function: Some(method_func),
```

- [x] **Step 3: Remove the fake builtin index function**

In `select_builtin_index_method`, replace the entire `function: HirFunction` literal that starts with `id: DefId::new(crate::ids::CrateId(u32::MAX), crate::ids::LocalDefId(u32::MAX))` with:

```rust
function: None,
```

This removes the only production `DefId::new(CrateId(u32::MAX), LocalDefId(u32::MAX))` pseudo-function path from selection.

- [x] **Step 4: Require a real function at non-builtin lowerer call sites**

In `lib/src/lower/control_flow/secondary.rs` and `lib/src/lower/expression.rs`, update call sites that return method candidates to unwrap only real methods:

```rust
let method_func = selected.function?;
```

Use that `method_func` in the existing returned tuple. This is correct because builtin index selection is consumed only by the index-lowering path, not the concrete/bound method call paths.

- [x] **Step 5: Keep index lowering function-agnostic**

In the index operator path in `lib/src/lower/control_flow/secondary.rs`, keep consuming only these fields from `selected`:

```rust
selected.receiver
selected.builtin_index_output
selected.return_type
selected.target
```

Do not unwrap `selected.function` in this path.

---

### Task 3: Verify Selection and Lowering Behavior

**Files:**
- Test only

- [x] **Step 1: Run the new selection test**

Run:

```bash
cargo test -p rock-lib selection::service::tests::builtin_index_selection_does_not_fabricate_hir_function -- --exact
```

Expected: PASS.

- [x] **Step 2: Run focused indexing tests**

Run:

```bash
cargo test -p rock-lib test_array_indexing -- --exact
cargo test -p rock-lib test_vec_index_dispatches_through_deref_slice -- --exact
```

Expected: both PASS.

---

### Task 4: Update Roadmap and Prior Slice Checklist

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/2026-05-25-current-crate-defid-repair-cleanup.md`

- [x] **Step 1: Update master checklist evidence**

Add this evidence bullet under `## 1. Identity And Arenas`:

```markdown
- `lib/src/selection/service.rs` no longer fabricates a sentinel-`DefId` `HirFunction` for builtin index selection; builtin indexing is represented as a selected origin/output without a callable function.
```

- [x] **Step 2: Add one Done item in the master checklist**

Add this Done checkbox under `## 1. Identity And Arenas`:

```markdown
- [x] Removed the builtin index selection pseudo-function that used `CrateId(u32::MAX)` / `LocalDefId(u32::MAX)` as a fake callable identity.
```

- [x] **Step 3: Narrow the remaining generated/sentinel gap**

In `master-audit-checklist.md` and the Task 1 row of `2026-05-17-compiler-architecture-ordered-roadmap.md`, remove `builtin selection pseudo-functions` from the remaining-work text while preserving the remaining items: provisional generic owners, auto `Sized` impl provenance, inference placeholder repairs, and legacy signature fallback IDs.

- [x] **Step 4: Update the previous focused plan**

In `docs/superpowers/plans/2026-05-25-current-crate-defid-repair-cleanup.md`, update the remaining-work text that mentions builtin selection pseudo-functions so it no longer lists builtin selection after this plan completes.

---

### Task 5: Final Verification

**Files:**
- Test only

- [x] **Step 1: Run focused tests and hygiene**

Run:

```bash
cargo test -p rock-lib selection::service::tests::builtin_index_selection_does_not_fabricate_hir_function -- --exact
cargo test -p rock-lib test_array_indexing -- --exact
cargo test -p rock-lib test_vec_index_dispatches_through_deref_slice -- --exact
cargo fmt --all --check
git diff --check
```

Expected: all PASS.
