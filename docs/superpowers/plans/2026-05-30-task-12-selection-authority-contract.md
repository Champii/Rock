# Task 12 Selection Authority Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish ordered roadmap Task 12 by making the shared selection service's selected result an explicit frontend authority contract, while leaving fallback deletion to Task 13 and backend metadata extraction to Task 21.

**Architecture:** Extend the existing `lib/src/selection/` types with a compact `SelectionAuthority` view derived from `SelectedMethod`. Add focused service tests proving the authority view carries selected impl/method/trait/trait-arg/receiver/output facts for concrete, trait-bound, operator, index, artifact-backed, same-name, and associated-output cases. Update roadmap/audit docs only after focused and full verification pass.

**Tech Stack:** Rust 2021, existing `rock-lib` selection/lowering/product artifact tests, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Design Spec

- Approved spec: `docs/superpowers/specs/2026-05-30-task-12-selection-authority-contract-design.md`
- Design commit: `f12eaf1 design task 12 selection authority contract`

## File Structure

- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Clarify Task 2 scoped completion and later ownership for string-keyed compatibility maps.
  - Mark Task 12 complete after implementation.
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
  - Mark Task 12-specific selection authority items complete after implementation.
  - Keep Task 13/21 cleanup items as remaining work.
- Modify: `lib/src/selection/types.rs`
  - Add `SelectionAuthority` as a compact, testable authority view over `SelectedMethod`.
  - Add helper methods on `SelectedMethod` to derive authority facts.
- Modify: `lib/src/selection/service.rs`
  - Add contract tests for concrete, trait-bound, same-name, artifact-backed, operator, index, and associated-output cases.
  - Add minimal production changes only if tests expose missing selected facts.
- Modify if needed: `lib/src/selection/mod.rs`
  - Re-export `SelectionAuthority` if downstream tests need it.
- Modify if needed: `lib/src/crate_artifact/load.rs` or `lib/src/products.rs`
  - Only for focused artifact-backed selected identity contract tests if pure selection-service tests cannot prove the requirement.

---

## Task 1: Clarify Scoped Roadmap Ownership

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [ ] **Step 1: Update Task 2 wording**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, change the Task 2 status paragraph from:

```markdown
**Status:** Complete for migrated finalized HIR ownership storage in `docs/superpowers/plans/2026-05-17-authoritative-hir-id-keyed-storage.md`; string-keyed staging/compatibility maps remain in earlier phases.
```

to:

```markdown
**Status:** Complete for migrated finalized HIR ownership storage in `docs/superpowers/plans/2026-05-17-authoritative-hir-id-keyed-storage.md`; remaining string-keyed staging/compatibility maps are not open Task 2 storage work and are tracked under Tasks 4-5, 18, and 21.
```

- [ ] **Step 2: Update reconciliation table wording**

In the reconciliation table row for Task 2, change the remaining-work cell from:

```markdown
`PartialHir` and `Lowerer` still use string-keyed staging/compatibility maps
```

to:

```markdown
No remaining finalized-HIR storage work; remaining string-keyed staging/compatibility maps belong to Tasks 4-5, 18, and 21
```

- [ ] **Step 3: Run docs check**

Run:

```bash
git diff -- docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git diff --check
```

Expected: diff only clarifies Task 2 ownership; whitespace check passes.

- [ ] **Step 4: Commit roadmap clarification**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git commit -m "clarify task 2 compatibility ownership"
```

Expected: commit succeeds.

---

## Task 2: Add Selection Authority View

**Files:**
- Modify: `lib/src/selection/types.rs`
- Modify if needed: `lib/src/selection/mod.rs`

- [ ] **Step 1: Write failing authority tests**

In `lib/src/selection/types.rs`, inside `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn selected_method_authority_records_impl_trait_method_and_args() {
    let impl_id = def_id(20);
    let trait_id = def_id(10);
    let method_id = def_id(30);
    let target = HirMethodCallTarget {
        impl_id: Some(impl_id),
        trait_id: Some(trait_id),
        trait_args: vec![Type::I64],
        method_id,
        from_index_operator: false,
    };
    let selected = SelectedMethod {
        receiver: HirExpr {
            kind: crate::hir::HirExprKind::Var("value".to_string()),
            ty: Type::I64,
            span: crate::span::Span::default(),
        },
        function: None,
        impl_def: None,
        target: Some(target),
        origin: SelectedOrigin::TraitImpl { impl_id, trait_id },
        receiver_adjustment: ReceiverAdjustment::Autoderef,
        substituted_params: Vec::new(),
        return_type: Type::Bool,
        builtin_index_output: None,
    };

    let authority = selected.authority();

    assert_eq!(authority.impl_id, Some(impl_id));
    assert_eq!(authority.trait_id, Some(trait_id));
    assert_eq!(authority.method_id, Some(method_id));
    assert_eq!(authority.trait_args, vec![Type::I64]);
    assert_eq!(authority.receiver_adjustment, ReceiverAdjustment::Autoderef);
    assert_eq!(authority.return_type, Type::Bool);
    assert_eq!(authority.builtin_index_output, None);
    assert!(!authority.from_index_operator);
}

#[test]
fn selected_method_authority_records_builtin_index_without_fabricated_ids() {
    let selected = SelectedMethod {
        receiver: HirExpr {
            kind: crate::hir::HirExprKind::Var("items".to_string()),
            ty: Type::Array(Box::new(Type::I64), 4),
            span: crate::span::Span::default(),
        },
        function: None,
        impl_def: None,
        target: None,
        origin: SelectedOrigin::BuiltinIndex,
        receiver_adjustment: ReceiverAdjustment::Autoderef,
        substituted_params: Vec::new(),
        return_type: Type::Reference {
            mutable: false,
            inner: Box::new(Type::I64),
        },
        builtin_index_output: Some(Type::I64),
    };

    let authority = selected.authority();

    assert_eq!(authority.impl_id, None);
    assert_eq!(authority.trait_id, None);
    assert_eq!(authority.method_id, None);
    assert_eq!(authority.trait_args, Vec::<Type>::new());
    assert_eq!(authority.receiver_adjustment, ReceiverAdjustment::Autoderef);
    assert_eq!(authority.builtin_index_output, Some(Type::I64));
    assert_eq!(authority.return_type, Type::Reference {
        mutable: false,
        inner: Box::new(Type::I64),
    });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p rock-lib selected_method_authority_records_
```

Expected: FAIL because `SelectedMethod::authority` and `SelectionAuthority` do not exist.

- [ ] **Step 3: Add authority type and helper**

In `lib/src/selection/types.rs`, after `SelectedMethod`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionAuthority {
    pub origin: SelectedOrigin,
    pub impl_id: Option<DefId>,
    pub trait_id: Option<DefId>,
    pub method_id: Option<DefId>,
    pub trait_args: Vec<Type>,
    pub receiver_adjustment: ReceiverAdjustment,
    pub return_type: Type,
    pub builtin_index_output: Option<Type>,
    pub from_index_operator: bool,
}

impl SelectedMethod {
    pub fn authority(&self) -> SelectionAuthority {
        SelectionAuthority {
            origin: self.origin.clone(),
            impl_id: self.target.as_ref().and_then(|target| target.impl_id),
            trait_id: self.target.as_ref().and_then(|target| target.trait_id),
            method_id: self.target.as_ref().map(|target| target.method_id),
            trait_args: self
                .target
                .as_ref()
                .map(|target| target.trait_args.clone())
                .unwrap_or_default(),
            receiver_adjustment: self.receiver_adjustment,
            return_type: self.return_type.clone(),
            builtin_index_output: self.builtin_index_output.clone(),
            from_index_operator: self
                .target
                .as_ref()
                .is_some_and(|target| target.from_index_operator),
        }
    }
}
```

In `lib/src/selection/mod.rs`, update the re-export to include `SelectionAuthority`:

```rust
pub use types::{
    ReceiverAdjustment, SelectedMethod, SelectedOrigin, SelectionAuthority, SelectionDiagnostic,
    SelectionKind, SelectionRequest,
};
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test -p rock-lib selected_method_authority_records_
cargo test -p rock-lib selection_diagnostic_preserves_selected_target_identity
```

Expected: both commands pass.

- [ ] **Step 5: Commit authority view**

Run:

```bash
git add lib/src/selection/types.rs lib/src/selection/mod.rs
git commit -m "add selection authority view"
```

Expected: commit succeeds.

---

## Task 3: Cover Same-Name And Artifact-Backed Selection Authority

**Files:**
- Modify: `lib/src/selection/service.rs`

- [ ] **Step 1: Write failing same-name and dependency-resolver tests**

In `lib/src/selection/service.rs`, inside `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn selection_authority_selects_same_named_method_by_receiver_identity() {
    let left_owner = def_id(100);
    let right_owner = def_id(101);
    let left_impl_id = def_id(110);
    let right_impl_id = def_id(111);
    let left_method_id = def_id(120);
    let right_method_id = def_id(121);
    let impls = vec![
        HirImpl {
            id: left_impl_id,
            owner: HirImplOwner::Named("left::Box".to_string()),
            type_name: "left::Box".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([("value".to_string(), method(left_method_id, left_owner))]),
        },
        HirImpl {
            id: right_impl_id,
            owner: HirImplOwner::Named("right::Box".to_string()),
            type_name: "right::Box".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: None,
            trait_id: None,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([("value".to_string(), method(right_method_id, right_owner))]),
        },
    ];
    let traits = HashMap::new();
    let methods = HashMap::new();
    let mut resolver = ResolverTables::default();
    resolver.item_names_by_id.insert(left_owner, "left::Box".to_string());
    resolver.item_names_by_id.insert(right_owner, "right::Box".to_string());
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
        kind: HirExprKind::Var("box".to_string()),
        ty: Type::Struct { id: right_owner, args: Vec::new() },
        span: Span::default(),
    };

    let authority = service
        .select_concrete_method(&[receiver], "value", |ty| ty.clone())
        .expect("expected right-side selected method")
        .authority();

    assert_eq!(authority.impl_id, Some(right_impl_id));
    assert_eq!(authority.method_id, Some(right_method_id));
}

#[test]
fn selection_authority_uses_dependency_resolver_for_artifact_backed_receiver() {
    let dependency_owner = DefId::new(CrateId(7), LocalDefId(3));
    let impl_id = DefId::new(CrateId(7), LocalDefId(4));
    let method_id = DefId::new(CrateId(7), LocalDefId(5));
    let imp = HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("dep::Box".to_string()),
        type_name: "dep::Box".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: None,
        trait_id: None,
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: Vec::new(),
        methods: HashMap::from([("value".to_string(), method(method_id, dependency_owner))]),
    };
    let traits = HashMap::new();
    let methods = HashMap::new();
    let impls = vec![imp];
    let resolver = ResolverTables::default();
    let mut dependency_resolver = ResolverTables::default();
    dependency_resolver
        .item_names_by_id
        .insert(dependency_owner, "dep::Box".to_string());
    let dependency_resolvers = HashMap::from([("dep".to_string(), dependency_resolver)]);
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
        kind: HirExprKind::Var("box".to_string()),
        ty: Type::Struct { id: dependency_owner, args: Vec::new() },
        span: Span::default(),
    };

    let authority = service
        .select_concrete_method(&[receiver], "value", |ty| ty.clone())
        .expect("expected dependency-backed selected method")
        .authority();

    assert_eq!(authority.impl_id, Some(impl_id));
    assert_eq!(authority.method_id, Some(method_id));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p rock-lib selection_authority_selects_same_named_method_by_receiver_identity
cargo test -p rock-lib selection_authority_uses_dependency_resolver_for_artifact_backed_receiver
```

Expected: if both pass immediately, record that current service behavior already satisfies the contract. If either fails, implement the minimal fix in `SelectionService::type_name_for_method_lookup_in_context` or concrete selection lookup.

- [ ] **Step 3: Implement minimal service fix if needed**

If same-name selection fails because short names are preferred over resolver-qualified names, make `type_names_for_method_lookup_in_context` keep resolver-qualified names first and only use short aliases after exact ID-backed names.

If dependency-backed selection fails, ensure `type_name_for_method_lookup_in_context` searches `dependency_resolvers.values()` before falling back to structural display names.

- [ ] **Step 4: Run focused selection tests**

Run:

```bash
cargo test -p rock-lib selection_authority_selects_same_named_method_by_receiver_identity
cargo test -p rock-lib selection_authority_uses_dependency_resolver_for_artifact_backed_receiver
cargo test -p rock-lib selection_service_selects_concrete_impl_method_by_identity
```

Expected: all pass.

- [ ] **Step 5: Commit Task 3**

Run:

```bash
git add lib/src/selection/service.rs
git commit -m "cover identity-backed selection authority"
```

Expected: commit succeeds.

---

## Task 4: Cover Trait-Bound, Operator, Index, And Associated Output Authority

**Files:**
- Modify: `lib/src/selection/service.rs`

- [ ] **Step 1: Add authority assertions to existing service tests**

In `lib/src/selection/service.rs`, update these existing tests:

1. In `selection_service_selects_type_var_bound_signature_by_trait_identity`, after existing assertions, add:

```rust
let authority = selected.authority();
assert_eq!(authority.impl_id, None);
assert_eq!(authority.trait_id, Some(trait_id));
assert_eq!(authority.method_id, Some(signature_id));
assert_eq!(authority.trait_args, Vec::<Type>::new());
assert_eq!(authority.receiver_adjustment, ReceiverAdjustment::None);
```

2. In `selection_service_selects_required_trait_operator_impl`, after existing assertions, add:

```rust
let authority = selected.authority();
assert_eq!(authority.impl_id, Some(trait_impl_id));
assert_eq!(authority.trait_id, Some(trait_id));
assert_eq!(authority.method_id, Some(trait_method_id));
assert_eq!(authority.trait_args, vec![receiver_ty.clone()]);
assert_eq!(authority.return_type, receiver_ty);
```

3. In `selection_service_prefers_user_index_impl_over_builtin_output`, after existing assertions, add:

```rust
let authority = selected.authority();
assert_eq!(authority.impl_id, Some(impl_id));
assert_eq!(authority.trait_id, Some(trait_id));
assert_eq!(authority.method_id, Some(method_id));
assert_eq!(authority.trait_args, vec![Type::I64]);
assert!(authority.from_index_operator);
assert_eq!(authority.builtin_index_output, None);
```

4. In `builtin_index_selection_does_not_fabricate_hir_function`, after existing assertions, add:

```rust
let authority = selected.authority();
assert_eq!(authority.impl_id, None);
assert_eq!(authority.trait_id, None);
assert_eq!(authority.method_id, None);
assert_eq!(authority.builtin_index_output, Some(Type::I64));
```

- [ ] **Step 2: Add associated-output authority test**

In `lib/src/selection/service.rs`, add this test near the index tests:

```rust
#[test]
fn selection_authority_records_associated_output_return_type() {
    let owner = def_id(70);
    let trait_id = def_id(71);
    let impl_id = def_id(72);
    let method_id = def_id(73);
    let mut method = method(method_id, owner);
    method.name = "next".to_string();
    method.ret_type = Type::Projection {
        ty: Box::new(Type::Struct { id: owner, args: Vec::new() }),
        trait_id,
        assoc_type: crate::types::AssociatedTypeKey {
            owner: trait_id,
            assoc_type_id: crate::ids::AssocTypeId(0),
        },
        trait_args: Vec::new(),
    };
    let imp = HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("IteratorBox".to_string()),
        type_name: "IteratorBox".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: Some("Iterator".to_string()),
        trait_id: Some(trait_id),
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: vec![crate::hir::HirAssociatedTypeDef {
            id: crate::ids::AssocTypeId(0),
            name: "Item".to_string(),
            ty: Type::I64,
        }],
        bounds: Vec::new(),
        methods: HashMap::from([("next".to_string(), method.clone())]),
    };
    let traits = HashMap::new();
    let methods = HashMap::new();
    let impls = vec![imp];
    let mut resolver = ResolverTables::default();
    resolver.item_names_by_id.insert(owner, "IteratorBox".to_string());
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
        kind: HirExprKind::Var("iter".to_string()),
        ty: Type::Struct { id: owner, args: Vec::new() },
        span: Span::default(),
    };

    let authority = service
        .select_required_trait_method(&receiver, &receiver.ty, trait_id, "next")
        .expect("expected iterator selection")
        .authority();

    assert_eq!(authority.impl_id, Some(impl_id));
    assert_eq!(authority.trait_id, Some(trait_id));
    assert_eq!(authority.method_id, Some(method_id));
    assert_eq!(authority.return_type, method.ret_type);
}
```

- [ ] **Step 3: Run tests to verify red/green**

Run:

```bash
cargo test -p rock-lib selection_authority_records_associated_output_return_type
cargo test -p rock-lib selection_service_selects_type_var_bound_signature_by_trait_identity
cargo test -p rock-lib selection_service_selects_required_trait_operator_impl
cargo test -p rock-lib selection_service_prefers_user_index_impl_over_builtin_output
cargo test -p rock-lib builtin_index_selection_does_not_fabricate_hir_function
```

Expected: the associated-output test fails before `SelectionAuthority` exists if Task 2 has not run; after Task 2, all tests should pass or expose a missing selected fact. Fix only missing fact propagation in `SelectionService`.

- [ ] **Step 4: Run selection suite**

Run:

```bash
cargo test -p rock-lib selection
```

Expected: PASS.

- [ ] **Step 5: Commit Task 4**

Run:

```bash
git add lib/src/selection/service.rs
git commit -m "cover selection authority contract facts"
```

Expected: commit succeeds.

---

## Task 5: Update Task 12 Docs Closure

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-30-task-12-selection-authority-contract.md`

- [ ] **Step 1: Run final focused verification before docs**

Run:

```bash
cargo test -p rock-lib selection
cargo test -p rock-lib product_artifact
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Expected: all pass. Record exact full-suite counts from `cargo test -p rock-lib`.

- [ ] **Step 2: Update ordered roadmap Task 12**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the reconciliation table row for Task 12 from:

```markdown
| 12. Shared selection service | Complete for scoped slice | `lib/src/selection/*` exists and lowering routes method/operator/index selection through it | The service still uses some string/HIR-map lookup internally; targetless duplication cleanup remains Task 13/21 work |
```

to:

```markdown
| 12. Shared selection service | Complete | `lib/src/selection/*` owns the frontend selection authority contract, including selected impl/method/trait identities, trait args, receiver adjustment, return/output facts, diagnostics, and lowering-time method/operator/index selection | No remaining Task 12 work; targetless fallback deletion remains Task 13 and backend metadata extraction remains Task 21 |
```

Update the Task 12 status paragraph from:

```markdown
**Status:** Complete for the shared selector-service slice in `docs/superpowers/plans/2026-05-19-selection-service.md`; the service is not yet a fully ID-only authority, and targetless compatibility is tracked by Tasks 13 and 15.
```

to:

```markdown
**Status:** Complete. The shared selection service is the frontend authority for method, operator, index, and trait-bound selection facts. Targetless fallback deletion remains Task 13 cleanup, and backend metadata extraction remains Task 21.
```

- [ ] **Step 3: Update master audit selection section**

In `docs/superpowers/plans/master-audit-checklist.md`, update the `Trait And Method Selection Service` summary row main gap from:

```markdown
Lowering-time selection is centralized, but the service is still partly string/HIR-map based and codegen still rediscovers trait impls
```

to:

```markdown
Selection service owns the frontend authority contract; remaining work is deleting targetless mono/codegen fallback rediscovery under Task 13 and moving backend metadata under Task 21
```

In section `## 5. Trait And Method Selection Service`, add this Done bullet after the shared selection service bullet:

```markdown
- [x] Completed the Task 12 selection authority contract: selected results expose impl, method, trait, trait-arg, receiver-adjustment, return/output, builtin-index, and diagnostic facts for downstream consumers when present.
```

Replace these Still to do bullets:

```markdown
- [ ] Centralize impl lookup, receiver adjustment, substitution, and associated type normalization.
- [ ] Make resolved selections explicit enough that mono/codegen consume selected impl IDs, method IDs, trait IDs, trait args, receiver adjustments, and projection normalizations without repeating frontend search.
- [ ] Add selection-service contract tests that verify method/trait decisions before codegen, including artifact-backed and same-name cases.
```

with:

```markdown
- [ ] Delete targetless mono/codegen fallback rediscovery after selected targets are present on all supported call edges.
- [ ] Remove direct trait selection logic from codegen as part of Task 13/21 backend cleanup.
```

Keep any remaining Task 13/21 items that are still accurate.

- [ ] **Step 4: Update final notes in this plan**

At the end of this plan, append:

```markdown
## Final Verification Notes

```text
cargo test -p rock-lib selection: PASS
cargo test -p rock-lib product_artifact: PASS
cargo test -p rock-lib: PASS (replace with exact counts)
cargo fmt --all --check: PASS
git diff --check: PASS
Final code review: APPROVED
```
```

Replace `replace with exact counts` before committing.

- [ ] **Step 5: Request final code review**

Use the `requesting-code-review` skill. Review scope:

```text
Task 12 selection authority contract. Verify that lib/src/selection owns the frontend selected-result contract for impl/method/trait/trait-arg/receiver-adjustment/return-output/builtin-index/diagnostic facts, docs do not pull Task 13/21 cleanup into Task 12, and fallback deletion remains tracked separately.
```

Expected: reviewer returns APPROVED or findings. Fix findings before Step 6.

- [ ] **Step 6: Run docs checks**

Run:

```bash
git diff -- docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-30-task-12-selection-authority-contract.md
git diff --check
```

Expected: docs mark Task 12 complete without claiming Task 13/21 fallback cleanup is done; whitespace check passes.

- [ ] **Step 7: Commit docs closure**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-30-task-12-selection-authority-contract.md
git commit -m "mark task 12 selection authority complete"
```

Expected: commit succeeds.

- [ ] **Step 8: Final clean status**

Run:

```bash
git status --short
```

Expected: no output.

---

## Plan Self-Review

- Spec coverage: Task 1 clarifies Task 2 ownership. Tasks 2-4 implement and test the Task 12 selection authority contract. Task 5 updates docs only after verification and final review.
- Placeholder scan: no `TBD`, `TODO`, or unspecified implementation steps remain.
- Type consistency: `SelectionAuthority`, `SelectedMethod::authority`, `SelectedOrigin`, `ReceiverAdjustment`, and `HirMethodCallTarget` names match existing code and planned re-exports.

## Final Verification Notes

```text
cargo test -p rock-lib selection: PASS; unit 18 passed; 0 failed; 0 ignored; 1369 filtered out; integration 0 passed; parser integration 0 passed.
cargo test -p rock-lib product_artifact: PASS; unit 88 passed; 0 failed; 0 ignored; 1299 filtered out; integration 0 passed; parser integration 0 passed.
cargo test -p rock-lib: PASS; unit 1386 passed; 0 failed; 1 ignored; integration 277 passed; 0 failed; parser integration 1 passed; doctests 1 passed; 0 failed; 1 ignored.
cargo fmt --all --check: PASS.
git diff --check: PASS.
Final code review: APPROVED
```
