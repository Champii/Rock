# Task 13 Selection Fallback Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete roadmap Task 13 by removing targetless mono/lowering semantic selection fallback while leaving backend metadata cleanup to Task 21.

**Architecture:** Harden monomorphization so `HirMethodCallTarget` is the only semantic method/operator/index/trait dispatch authority. Keep direct function IDs, static impl method IDs, object-backed functions, and `InstanceId` call edges working. Use focused red/green tests before each production edit and update docs only after full verification and review.

**Tech Stack:** Rust 2021, `rock-lib`, existing HIR/mono tests, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Design Spec

- Approved spec: `docs/superpowers/specs/2026-05-31-task-13-selection-fallback-cleanup-design.md`
- Design commit: `d445d46 design task 13 selection fallback cleanup`

## File Structure

- Modify: `lib/src/mono/process.rs`
  - Remove process-level targetless semantic method dispatch rediscovery.
  - Add tests proving targetless generic method calls are not specialized by name.
- Modify: `lib/src/mono/methods.rs`
  - Require selected targets in generic standalone/trait method monomorphization helpers.
  - Keep static impl method call/value specialization by direct method ID or supported static-name path.
  - Update legacy qualified receiver tests to use explicit selected targets.
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Mark Task 13 targetless fallback cleanup complete after verification.
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
  - Mark Task 13 targetless fallback cleanup done while preserving Task 21 backend cleanup.
- Modify: `docs/superpowers/plans/2026-05-31-task-13-selection-fallback-cleanup.md`
  - Append final verification notes.

---

## Task 1: Remove Process-Level Targetless Method Dispatch

**Files:**
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/methods.rs`

- [ ] **Step 1: Write failing process-level targetless dispatch test**

In `lib/src/mono/process.rs`, inside `#[cfg(test)] mod tests`, add this test after `process_selected_generic_method_call_uses_instance_target`:

```rust
#[test]
fn process_targetless_generic_method_call_does_not_specialize_by_name() {
    let mut mono = Monomorphizer::new();
    let impl_id = DefId::new(CrateId(0), LocalDefId(941));
    let method_id = DefId::new(CrateId(0), LocalDefId(942));
    mono.resolver
        .item_names_by_id
        .insert(DefId::new(CrateId(0), LocalDefId(700)), "Box".to_string());
    mono.generic_impls.insert(
        "Box".to_string(),
        generic_impl_for_process_test("Box", impl_id, None, method_id),
    );
    let recv = HirExpr {
        kind: HirExprKind::Var("box".to_string()),
        ty: Type::Struct {
            id: DefId::new(CrateId(0), LocalDefId(700)),
            args: vec![Type::I64],
        },
        span: Default::default(),
    };
    let mut expr = HirExpr {
        kind: HirExprKind::MethodCall(
            Box::new(recv),
            "map".to_string(),
            Vec::new(),
            Some(SelfReceiverMode::Move),
            None,
        ),
        ty: Type::I64,
        span: Default::default(),
    };

    mono.process_expr(&mut expr);

    assert!(
        matches!(expr.kind, HirExprKind::MethodCall(..)),
        "targetless semantic method dispatch must not be rediscovered by name"
    );
    assert!(mono.instances.records().next().is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p rock-lib process_targetless_generic_method_call_does_not_specialize_by_name
```

Expected: FAIL because `process_expr` currently monomorphizes targetless method calls through the `None` fallback branch.

- [ ] **Step 3: Remove targetless semantic dispatch from `process_expr`**

In `lib/src/mono/process.rs`, replace the current `if let Some(ref type_name) = recv_type_name { ... }` block in the `HirExprKind::MethodCall` arm with:

```rust
if let Some(ref type_name) = recv_type_name {
    match selected_target.as_ref() {
        Some(target) if target.trait_id.is_some() => {
            self.monomorphize_trait_method_call(type_name, &method_name_clone, &all_args, expr);
        }
        Some(target) if target.impl_id.is_some() => {
            if matches!(expr.kind, HirExprKind::MethodCall(..)) {
                self.monomorphize_standalone_method_call(
                    type_name,
                    &method_name_clone,
                    &all_args,
                    expr,
                );
            }
        }
        Some(_) | None => {}
    }
}
```

This removes the `None => { monomorphize_trait_method_call; monomorphize_standalone_method_call; }` fallback and makes missing targets a non-specializing method call at this phase.

- [ ] **Step 4: Remove unused builtin-index fallback helper**

In `lib/src/mono/methods.rs`, delete the unused import:

```rust
use crate::type_services::facts::TypeFacts;
```

Delete this helper because it only supported the removed targetless process fallback guard:

```rust
pub(super) fn is_builtin_index_dispatch(
    recv_ty: &Type,
    method_name: &str,
    args: &[HirExpr],
) -> bool {
    method_name == "index"
        && args.len() == 2
        && matches!(recv_ty, Type::Slice(_) | Type::Array(_, _))
        && TypeFacts::has_builtin_index_impl(recv_ty, &args[1].ty)
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p rock-lib process_targetless_generic_method_call_does_not_specialize_by_name
cargo test -p rock-lib process_selected_generic_method_call_uses_instance_target
cargo test -p rock-lib process_selected_inherent_method_skips_trait_specialization_path
```

Expected: all pass.

- [ ] **Step 6: Commit process fallback cleanup**

Run:

```bash
git add lib/src/mono/process.rs lib/src/mono/methods.rs
git commit -m "remove process targetless method fallback"
```

Expected: commit succeeds.

---

## Task 2: Require Selected Targets In Generic Method Helpers

**Files:**
- Modify: `lib/src/mono/methods.rs`

- [ ] **Step 1: Write failing standalone helper targetless test**

In `lib/src/mono/methods.rs`, inside `#[cfg(test)] mod tests`, add this test after `static_impl_method_call_without_selected_target_specializes_by_name`:

```rust
#[test]
fn monomorphize_standalone_method_call_without_selected_target_does_not_specialize_by_name() {
    let mut mono = Monomorphizer::new();
    let impl_id = def_id(90);
    let method_id = def_id(91);
    mono.generic_impls.insert(
        "Option".to_string(),
        generic_map_impl("Option", impl_id, method_id),
    );
    let (recv, func_arg, mut expr) = map_expr_for_type("Option");

    mono.monomorphize_standalone_method_call(
        "Option",
        "map",
        &[recv, func_arg],
        &mut expr,
    );

    assert!(
        matches!(expr.kind, HirExprKind::MethodCall(..)),
        "targetless standalone methods must not specialize by type and method name"
    );
    assert!(mono.instances.records().next().is_none());
}
```

- [ ] **Step 2: Write failing trait helper targetless test**

In the same test module, add this test after the standalone targetless test:

```rust
#[test]
fn monomorphize_trait_method_call_without_selected_target_does_not_specialize_by_name() {
    let mut mono = Monomorphizer::new();
    let impl_id = def_id(92);
    let trait_id = def_id(93);
    let method_id = def_id(94);
    let mut method = option_println_method();
    method.id = method_id;
    mono.trait_impls.insert(
        "Show".to_string(),
        vec![HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Option".to_string()),
            type_name: "Option".to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(GenericParamId {
                owner: def_id(0),
                index: 0,
            })],
            trait_name: Some("Show".to_string()),
            trait_id: Some(trait_id),
            trait_generics: vec![],
            trait_arg_types: vec![],
            associated_types: vec![],
            bounds: vec![],
            methods: HashMap::from([("println".to_string(), method)]),
        }],
    );
    let recv = HirExpr {
        kind: HirExprKind::Var("some".to_string()),
        ty: enum_ty("Option", vec![Type::I64]),
        span: Span::default(),
    };
    let mut expr = HirExpr {
        kind: HirExprKind::MethodCall(
            Box::new(recv.clone()),
            "println".to_string(),
            vec![],
            Some(SelfReceiverMode::Move),
            None,
        ),
        ty: Type::I32,
        span: Span::default(),
    };

    mono.monomorphize_trait_method_call("Option", "println", &[recv], &mut expr);

    assert!(
        matches!(expr.kind, HirExprKind::MethodCall(..)),
        "targetless trait methods must not specialize by receiver and method name"
    );
    assert!(mono.instances.records().next().is_none());
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test -p rock-lib monomorphize_standalone_method_call_without_selected_target_does_not_specialize_by_name
cargo test -p rock-lib monomorphize_trait_method_call_without_selected_target_does_not_specialize_by_name
```

Expected: both FAIL because helper functions currently specialize targetless calls by name/receiver fallback.

- [ ] **Step 4: Harden standalone method helper**

In `lib/src/mono/methods.rs`, at the start of `monomorphize_standalone_method_call`, replace the current `selected_target` extraction and fallback-based `found` construction with selected-target-only lookup:

```rust
let selected_target = match &expr.kind {
    HirExprKind::MethodCall(_, _, _, _, Some(target)) => target.clone(),
    _ => return,
};
let exact_impl = selected_target.impl_id.and_then(|impl_id| {
    self.generic_impls.values().find_map(|imp| {
        (imp.id == impl_id).then(|| {
            imp.methods.get(method_name).and_then(|method| {
                (method.id == selected_target.method_id).then_some((imp.clone(), method.clone()))
            })
        })?
    })
});
let found = exact_impl.and_then(|(imp, method)| {
    if !imp.type_generics.is_empty() || !method.generic_params.is_empty() {
        let receiver_type_args = self.extract_type_args_from_receiver(&args[0].ty, &imp.type_generics);
        if receiver_type_args.len() == imp.type_generics.len() {
            let type_args = self.combine_impl_and_method_type_args(
                &method,
                &receiver_type_args,
                &imp.type_generics,
                imp.id,
                args,
            );
            Some((
                method.clone(),
                type_args,
                imp.type_generics.clone(),
                type_name.to_string(),
                imp.clone(),
            ))
        } else {
            None
        }
    } else {
        None
    }
});
```

Keep the existing `if let Some((method, type_args, type_generics, impl_type_name, imp)) = found { ... }` body after this block unchanged.

- [ ] **Step 5: Harden trait method helper**

In `lib/src/mono/methods.rs`, at the start of `monomorphize_trait_method_call`, replace target extraction with:

```rust
let selected_target = match &expr.kind {
    HirExprKind::MethodCall(_, _, _, _, Some(target)) => target.clone(),
    _ => return,
};
```

Then update helper calls in that function from `selected_target.as_ref()` to `Some(&selected_target)` and from `selected_target.as_ref().and_then(...)` to direct selected-target access. The loop should retain receiver/type matching, but only after the selected target has matched the impl/trait identity:

```rust
if !Self::impl_matches_method_target(&imp, Some(&selected_target)) {
    continue;
}
let selected_impl = selected_target
    .impl_id
    .is_some_and(|impl_id| impl_id == imp.id);
if selected_impl || self.receiver_type_matches(&imp, &args[0].ty, &lookup_type_name) {
    if !Self::impl_trait_args_match_selected_target(&imp, &args[0].ty, Some(&selected_target)) {
        continue;
    }
    if let Some(method) = imp.methods.get(method_name) {
        if selected_target.impl_id.is_some() && method.id != selected_target.method_id {
            continue;
        }

        matched_method_on_lookup_name = true;
        // Keep the existing generic specialization body here.
    }
}
```

- [ ] **Step 6: Convert qualified receiver compatibility tests to selected-target tests**

Update `test_monomorphize_standalone_method_call_matches_short_impl_name_for_qualified_receiver` so the impl and method have explicit IDs and the expression carries a target:

```rust
let impl_id = DefId::new(CrateId(0), LocalDefId(60));
let method_id = DefId::new(CrateId(0), LocalDefId(61));
let mut method = option_map_method();
method.id = method_id;
mono.generic_impls.insert(
    "Option".to_string(),
    HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Option".to_string()),
        type_name: "Option".to_string(),
        type_generics: vec!["T".to_string()],
        receiver_arg_types: vec![Type::Generic(GenericParamId {
            owner: def_id(0),
            index: 0,
        })],
        trait_name: None,
        trait_id: None,
        trait_generics: vec![],
        trait_arg_types: vec![],
        associated_types: vec![],
        bounds: vec![],
        methods: HashMap::from([("map".to_string(), method)]),
    },
);
```

Change that test's `HirExprKind::MethodCall` target from `None` to:

```rust
Some(HirMethodCallTarget {
    impl_id: Some(impl_id),
    trait_id: None,
    trait_args: Vec::new(),
    method_id,
    from_index_operator: false,
})
```

Update `test_monomorphize_trait_method_call_matches_short_impl_name_for_qualified_receiver` the same way with explicit `impl_id`, `trait_id`, and `method_id`, then set its target to:

```rust
Some(HirMethodCallTarget {
    impl_id: Some(impl_id),
    trait_id: Some(trait_id),
    trait_args: Vec::new(),
    method_id,
    from_index_operator: false,
})
```

Keep each test's assertion that the selected qualified receiver still monomorphizes to `HirExprKind::Call`.

- [ ] **Step 7: Run focused tests**

Run:

```bash
cargo test -p rock-lib monomorphize_standalone_method_call_without_selected_target_does_not_specialize_by_name
cargo test -p rock-lib monomorphize_trait_method_call_without_selected_target_does_not_specialize_by_name
cargo test -p rock-lib test_monomorphize_standalone_method_call_matches_short_impl_name_for_qualified_receiver
cargo test -p rock-lib test_monomorphize_trait_method_call_matches_short_impl_name_for_qualified_receiver
cargo test -p rock-lib selected_trait_impl_method_call_rejects_wrong_method_id
```

Expected: all pass.

- [ ] **Step 8: Commit helper hardening**

Run:

```bash
git add lib/src/mono/methods.rs
git commit -m "require selected targets for generic method mono"
```

Expected: commit succeeds.

---

## Task 3: Verify Explicit Callable Paths Still Work

**Files:**
- Modify only if tests expose a regression: `lib/src/mono/process.rs`, `lib/src/mono/methods.rs`

- [ ] **Step 1: Run existing explicit-callable focused tests**

Run:

```bash
cargo test -p rock-lib process_static_impl_resolved_function_target_uses_method_id_not_name
cargo test -p rock-lib process_static_impl_resolved_function_value_uses_method_instance_target
cargo test -p rock-lib static_impl_method_call_without_selected_target_specializes_by_name
cargo test -p rock-lib process_generic_function_call_uses_instance_target
cargo test -p rock-lib process_call_does_not_resolve_callee_by_backend_symbol
cargo test -p rock-lib process_reused_generic_method_call_keeps_instance_target
```

Expected: all pass. These tests prove Task 13 did not break supported non-dispatch callable edges.

- [ ] **Step 2: Fix only regressions if any focused test fails**

If a direct static impl method call/value test fails, keep the `monomorphize_static_impl_method_call`, `monomorphize_static_impl_method_call_by_id`, and `monomorphize_static_impl_method_value_by_id` paths intact. Do not reintroduce targetless `HirExprKind::MethodCall` semantic dispatch.

If an `InstanceId` or direct function test fails, restore only direct callable handling in `process_expr`. Do not restore the removed `None` method-call fallback.

- [ ] **Step 3: Run mono-focused suite**

Run:

```bash
cargo test -p rock-lib mono
```

Expected: PASS.

- [ ] **Step 4: Commit preservation fixes only if files changed**

If Step 2 required edits, run:

```bash
git add lib/src/mono/process.rs lib/src/mono/methods.rs
git commit -m "preserve explicit callable mono paths"
```

If no files changed, do not create an empty commit. Record in the task result that existing explicit-callable tests passed without code changes.

---

## Task 4: Final Verification And Documentation Closure

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-31-task-13-selection-fallback-cleanup.md`

- [ ] **Step 1: Run final verification before docs**

Run:

```bash
cargo test -p rock-lib selection
cargo test -p rock-lib product_artifact
cargo test -p rock-lib mono
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Expected: all pass. Record exact full-suite counts from `cargo test -p rock-lib`.

- [ ] **Step 2: Update ordered roadmap Task 13 row**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the reconciliation table row for Task 13 from:

```markdown
| 13. Remove mono/codegen selection duplication | Partial | Present `HirMethodCallTarget` values are treated as authoritative by migrated mono/codegen paths | Targetless compatibility remains in lowering/mono/codegen, and codegen still has direct trait impl registration/search paths tracked under Task 21 |
```

to:

```markdown
| 13. Remove mono/codegen selection duplication | Complete | `HirMethodCallTarget` values are authoritative for mono/lowering semantic method/operator/index/trait dispatch, and targetless dispatch no longer rediscover generic methods by name/receiver fallback | No remaining Task 13 targetless fallback work; direct codegen trait/member metadata cleanup remains Task 21 |
```

- [ ] **Step 3: Update ordered roadmap Task 13 status paragraph**

In the same file, update the Task 13 status paragraph from:

```markdown
**Status:** Partial overall. Selected-target fallback cleanup in `docs/superpowers/plans/2026-05-19-selection-fallback-cleanup.md` is complete for present `HirMethodCallTarget` values; targetless compatibility and codegen-side trait rediscovery remain open.
```

to:

```markdown
**Status:** Complete. Present selected targets are authoritative, and targetless mono/lowering semantic dispatch fallback has been removed. Direct codegen trait/member metadata cleanup remains Task 21.
```

- [ ] **Step 4: Update master audit selection section**

In `docs/superpowers/plans/master-audit-checklist.md`, in section `## 5. Trait And Method Selection Service`, add this Done bullet after the Task 12 authority-contract bullet:

```markdown
- [x] Completed Task 13 targetless fallback cleanup: monomorphization no longer rediscover semantic method/operator/index/trait dispatch from missing `HirMethodCallTarget` values by name or receiver fallback.
```

Replace this Still to do bullet:

```markdown
- [ ] Delete targetless mono/codegen fallback rediscovery after selected targets are present on all supported call edges.
```

with:

```markdown
- [ ] Remove direct trait selection logic from codegen as part of Task 21 backend cleanup.
```

If that exact Task 21 codegen bullet already exists immediately below, keep only one copy.

- [ ] **Step 5: Append final verification notes to this plan**

At the end of this plan, append a `## Final Verification Notes` section. Include one line for each command from Step 1. For `cargo test -p rock-lib`, copy the exact unit, integration, parser integration, and doctest counts from Step 1 output. End the note with `Final code review: PENDING` before requesting review.

- [ ] **Step 6: Request final code review**

Use the `requesting-code-review` skill. Review scope:

```text
Task 13 selection fallback cleanup. Verify targetless mono/lowering semantic dispatch fallback is removed without reintroducing name/receiver rediscovery, explicit direct callable paths still work, docs leave codegen trait/member metadata cleanup to Task 21, and Task 12/11 are not reopened.
```

Expected: reviewer returns APPROVED or findings. Fix findings before Step 7.

- [ ] **Step 7: Mark final review approved in this plan**

After approval, change `Final code review: PENDING` to:

```text
Final code review: APPROVED
```

- [ ] **Step 8: Run docs checks**

Run:

```bash
git diff -- docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-31-task-13-selection-fallback-cleanup.md
git diff --check
```

Expected: docs mark Task 13 fallback cleanup complete without claiming Task 21 backend cleanup is done; whitespace check passes.

- [ ] **Step 9: Commit docs closure**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-31-task-13-selection-fallback-cleanup.md
git commit -m "mark task 13 selection fallback cleanup complete"
```

Expected: commit succeeds.

- [ ] **Step 10: Final clean status**

Run:

```bash
git status --short
```

Expected: no output.

---

## Plan Self-Review

- Spec coverage: Task 1 removes process-level targetless semantic dispatch; Task 2 removes direct helper rediscovery by name/receiver fallback; Task 3 protects supported non-dispatch callable paths; Task 4 updates docs only after verification and final review.
- Placeholder scan: no `TBD`, `TODO`, or unspecified implementation steps remain.
- Type consistency: plan uses existing `HirMethodCallTarget`, `HirExprKind::MethodCall`, `HirVarTarget::Instance`, `HirCallTarget::Instance`, `InstanceOrigin`, and existing mono test helpers.
- Scope check: codegen trait/member metadata cleanup remains Task 21 and is not included in Task 13 implementation.

## Final Verification Notes

- `cargo test -p rock-lib selection`: PASS, 21 selection-focused tests passed.
- `cargo test -p rock-lib product_artifact`: PASS, 88 product-artifact tests passed.
- `cargo test -p rock-lib mono`: PASS, 81 mono-filtered unit tests passed, 1 ignored; 1 mono-filtered integration test passed.
- `cargo test -p rock-lib`: PASS, 1394 unit tests passed, 0 failed, 1 ignored; 277 integration tests passed; `test_parse_struct_with_fields` passed; doctests passed with 1 passed and 1 ignored.
- `cargo fmt --all --check`: PASS.
- `git diff --check`: PASS.
- Final code review: APPROVED
