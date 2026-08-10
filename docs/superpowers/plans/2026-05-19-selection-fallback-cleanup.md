# Selection Fallback Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Roadmap Task 13 by making mono and codegen treat present `HirMethodCallTarget` values as authoritative identity contracts instead of falling back to semantic rediscovery.

**Architecture:** Keep Task 13 narrow: selected targets hard-fail or remain unresolved when their identity cannot be matched, while targetless compatibility paths remain for Tasks 14-15. Add targeted guards in mono, add trait-member identity validation in codegen, and update audit docs only after focused and full verification pass.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `HirMethodCallTarget`, monomorphization `InstanceRegistry`, LLVM codegen dispatch maps, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## File Structure

- Modify: `lib/src/mono/methods.rs`
  - Add failing tests for selected trait impl method-ID mismatch.
  - Guard selected impl-backed trait method monomorphization by exact `(impl_id, method_id)`.
- Modify: `lib/src/mono/process.rs`
  - Add failing test proving selected inherent targets do not enter trait fallback specialization.
  - Branch selected method calls by target shape before invoking mono specialization helpers.
- Modify: `lib/src/codegen/mod.rs`
  - Add a trait-member ID registry keyed by `(trait_id, method_name)` and populate it from `HirProgram::traits_by_id()`.
  - Keep backend declaration maps and targetless compatibility maps unchanged.
- Modify: `lib/src/codegen/expr/mod.rs`
  - Add failing test for selected trait-only member ID mismatch.
  - Validate trait-only selected targets against the codegen trait-member ID registry before impl lookup.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Mark Roadmap Task 13 complete for selected-target fallback cleanup.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Update Trait And Method Selection Service evidence and leave Tasks 14-15 as remaining backend callable-edge work.

## Task 1: Guard Mono Selected Trait Impl Method IDs

**Files:**
- Modify: `lib/src/mono/methods.rs`

- [ ] **Step 1: Write a failing selected trait impl mismatch test**

In `lib/src/mono/methods.rs`, inside the existing `#[cfg(test)] mod tests`, add this test after `test_monomorphize_trait_method_call_matches_short_impl_name_for_qualified_receiver`:

```rust
    #[test]
    fn selected_trait_impl_method_call_rejects_wrong_method_id() {
        let mut mono = Monomorphizer::new();
        let impl_id = DefId::new(CrateId(0), LocalDefId(710));
        let trait_id = DefId::new(CrateId(0), LocalDefId(711));
        let actual_method_id = DefId::new(CrateId(0), LocalDefId(712));
        let selected_method_id = DefId::new(CrateId(0), LocalDefId(713));
        let mut method = option_println_method();
        method.id = actual_method_id;
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

        let recv = qualified_option_receiver();
        let mut expr = HirExpr {
            kind: HirExprKind::MethodCall(
                Box::new(recv.clone()),
                "println".to_string(),
                vec![],
                Some(SelfReceiverMode::Move),
                Some(HirMethodCallTarget {
                    impl_id: Some(impl_id),
                    trait_id: Some(trait_id),
                    trait_args: Vec::new(),
                    method_id: selected_method_id,
                    from_index_operator: false,
                }),
            ),
            ty: Type::I32,
            span: Span::default(),
        };

        mono.monomorphize_trait_method_call("Option", "println", &[recv], &mut expr);

        assert!(matches!(expr.kind, HirExprKind::MethodCall(..)));
        assert!(mono.instances.records().next().is_none());
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p rock-lib selected_trait_impl_method_call_rejects_wrong_method_id`

Expected: FAIL because the call is rewritten to `HirExprKind::Call` even though the selected method ID does not match the impl method.

- [ ] **Step 3: Add the selected impl method-ID guard**

In `lib/src/mono/methods.rs`, inside `monomorphize_trait_method_call`, replace this block:

```rust
                        if let Some(method) = imp.methods.get(method_name) {
                            matched_method_on_lookup_name = true;
                            if !method.generic_params.is_empty() || !imp.type_generics.is_empty() {
```

with this block:

```rust
                        if let Some(method) = imp.methods.get(method_name) {
                            if selected_target.as_ref().is_some_and(|target| {
                                target.impl_id.is_some() && method.id != target.method_id
                            }) {
                                continue;
                            }

                            matched_method_on_lookup_name = true;
                            if !method.generic_params.is_empty() || !imp.type_generics.is_empty() {
```

This only requires exact impl method IDs when the selected target names a concrete impl. Trait-bound targets without `impl_id` keep using selected trait identity plus method name because their `method_id` can be the trait signature/default ID rather than the concrete impl method ID.

- [ ] **Step 4: Run the focused mono test**

Run: `cargo test -p rock-lib selected_trait_impl_method_call_rejects_wrong_method_id`

Expected: PASS.

- [ ] **Step 5: Run nearby mono selected-target regressions**

Run these one at a time:

```bash
cargo test -p rock-lib monomorphize_same_named_methods_on_different_impls_use_distinct_method_origins
cargo test -p rock-lib monomorphize_same_owner_distinct_methods_use_distinct_method_origins
cargo test -p rock-lib test_monomorphize_trait_method_call_matches_short_impl_name_for_qualified_receiver
```

Expected: PASS for each command.

- [ ] **Step 6: Commit Task 1**

Run:

```bash
git add lib/src/mono/methods.rs
git commit -m "guard selected trait impl method ids"
```

## Task 2: Branch Mono Method Processing By Selected Target Shape

**Files:**
- Modify: `lib/src/mono/process.rs`

- [ ] **Step 1: Extend test imports**

In `lib/src/mono/process.rs`, inside `#[cfg(test)] mod tests`, extend imports from:

```rust
    use crate::hir::{HirBlock, HirFunction, HirImpl, HirImplOwner, HirMethodCallTarget};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use std::collections::HashMap;
```

to:

```rust
    use crate::hir::{
        HirBlock, HirFunction, HirImpl, HirImplOwner, HirMethodCallTarget, HirParam,
    };
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::mono::{InstanceImplOwner, InstanceOrigin};
    use crate::types::{GenericParamId, Type};
    use std::collections::HashMap;
```

- [ ] **Step 2: Write a failing selected inherent branch test**

In `lib/src/mono/process.rs`, add these helpers and test at the end of the existing tests module:

```rust
    fn generic_self_method(type_name: &str, impl_id: DefId, method_id: DefId) -> HirFunction {
        let generic = GenericParamId {
            owner: impl_id,
            index: 0,
        };
        HirFunction {
            id: method_id,
            name: "map".to_string(),
            qualified_name: Some(format!("{}_map", type_name)),
            generic_params: Vec::new(),
            generic_param_ids: Vec::new(),
            generic_bounds: HashMap::new(),
            params: vec![HirParam {
                name: "self".to_string(),
                ty: Type::Struct {
                    id: DefId::new(CrateId(0), LocalDefId(700)),
                    args: vec![Type::Generic(generic)],
                },
                mutable: false,
                is_ref: false,
            }],
            ret_type: Type::I64,
            body: HirBlock {
                stmts: Vec::new(),
                ty: Type::I64,
            },
            is_curried: false,
            is_method: true,
            self_receiver: Some(SelfReceiverMode::Move),
            is_unsafe: false,
        }
    }

    fn generic_impl_for_process_test(
        type_name: &str,
        impl_id: DefId,
        trait_id: Option<DefId>,
        method_id: DefId,
    ) -> HirImpl {
        HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named(type_name.to_string()),
            type_name: type_name.to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(GenericParamId {
                owner: impl_id,
                index: 0,
            })],
            trait_name: trait_id.map(|_| "MapTrait".to_string()),
            trait_id,
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([(
                "map".to_string(),
                generic_self_method(type_name, impl_id, method_id),
            )]),
        }
    }

    #[test]
    fn process_selected_inherent_method_skips_trait_specialization_path() {
        let mut mono = Monomorphizer::new();
        let impl_id = DefId::new(CrateId(0), LocalDefId(701));
        let inherent_method_id = DefId::new(CrateId(0), LocalDefId(702));
        let trait_method_id = DefId::new(CrateId(0), LocalDefId(703));
        let trait_id = DefId::new(CrateId(0), LocalDefId(704));
        mono.generic_impls.insert(
            "Box".to_string(),
            generic_impl_for_process_test("Box", impl_id, None, inherent_method_id),
        );
        mono.trait_impls.insert(
            "MapTrait".to_string(),
            vec![generic_impl_for_process_test(
                "Box",
                impl_id,
                Some(trait_id),
                trait_method_id,
            )],
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
                Some(HirMethodCallTarget {
                    impl_id: Some(impl_id),
                    trait_id: None,
                    trait_args: Vec::new(),
                    method_id: inherent_method_id,
                    from_index_operator: false,
                }),
            ),
            ty: Type::I64,
            span: Default::default(),
        };

        mono.process_expr(&mut expr);

        let origins = mono
            .instances
            .records()
            .map(|record| record.origin.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            origins,
            vec![InstanceOrigin::ImplMethod {
                owner: InstanceImplOwner::Named(impl_id),
                method: inherent_method_id,
            }]
        );
    }
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p rock-lib process_selected_inherent_method_skips_trait_specialization_path`

Expected: FAIL because `process_expr` still sends the selected inherent call through trait monomorphization before standalone monomorphization.

- [ ] **Step 4: Branch selected method processing by target shape**

In `lib/src/mono/process.rs`, replace the block beginning at `if let Some(ref type_name) = recv_type_name {` in the `HirExprKind::MethodCall` arm with:

```rust
                if let Some(ref type_name) = recv_type_name {
                    if !(selected_target.is_none()
                        && Self::is_builtin_index_dispatch(
                            &processed_recv.ty,
                            &method_name_clone,
                            &all_args,
                        ))
                    {
                        match selected_target.as_ref() {
                            Some(target) if target.trait_id.is_some() => {
                                self.monomorphize_trait_method_call(
                                    type_name,
                                    &method_name_clone,
                                    &all_args,
                                    expr,
                                );
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
                            Some(_) => {}
                            None => {
                                self.monomorphize_trait_method_call(
                                    type_name,
                                    &method_name_clone,
                                    &all_args,
                                    expr,
                                );
                                if matches!(expr.kind, HirExprKind::MethodCall(..)) {
                                    self.monomorphize_standalone_method_call(
                                        type_name,
                                        &method_name_clone,
                                        &all_args,
                                        expr,
                                    );
                                }
                            }
                        }
                    }
                }
```

This keeps targetless compatibility while preventing selected inherent targets from entering trait fallback selection.

- [ ] **Step 5: Run focused mono process tests**

Run these one at a time:

```bash
cargo test -p rock-lib process_selected_inherent_method_skips_trait_specialization_path
cargo test -p rock-lib test_lookup_self_receiver_uses_targeted_trait_impl
cargo test -p rock-lib test_lookup_self_receiver_does_not_pick_first_same_named_trait_method
```

Expected: PASS for each command.

- [ ] **Step 6: Commit Task 2**

Run:

```bash
git add lib/src/mono/process.rs
git commit -m "branch mono processing by selected target"
```

## Task 3: Validate Codegen Trait-Only Selected Member IDs

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/expr/mod.rs`

- [ ] **Step 1: Write the failing codegen trait-member mismatch test**

In `lib/src/codegen/expr/mod.rs`, inside the existing tests module, add this test after `selected_trait_target_matches_nominal_receiver_alias_by_id`:

```rust
    #[test]
    fn selected_trait_target_rejects_wrong_trait_member_id() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let struct_id = DefId::new(CrateId(0), LocalDefId(1));
        let impl_id = DefId::new(CrateId(0), LocalDefId(99));
        let trait_id = DefId::new(CrateId(0), LocalDefId(100));
        let actual_signature_id = DefId::new(CrateId(0), LocalDefId(101));
        let selected_signature_id = DefId::new(CrateId(0), LocalDefId(102));
        let method_id = DefId::new(CrateId(0), LocalDefId(103));
        let mut actual_method = test_function(method_id, "show");
        actual_method.qualified_name = Some("String_Show_show".to_string());

        codegen
            .struct_names_by_id
            .insert(struct_id, "String".to_string());
        codegen
            .trait_member_ids
            .insert((trait_id, "show".to_string()), actual_signature_id);
        codegen.trait_impls.insert(
            ("String".to_string(), Vec::new(), trait_id, Vec::new()),
            HirImpl {
                id: impl_id,
                owner: HirImplOwner::Named("String".to_string()),
                type_name: "String".to_string(),
                type_generics: Vec::new(),
                receiver_arg_types: Vec::new(),
                trait_name: Some("Show".to_string()),
                trait_id: Some(trait_id),
                trait_generics: Vec::new(),
                trait_arg_types: Vec::new(),
                associated_types: Vec::new(),
                bounds: Vec::new(),
                methods: HashMap::from([("show".to_string(), actual_method)]),
            },
        );
        codegen.functions.insert(
            "String_Show_show".to_string(),
            codegen.module.add_function(
                "String_Show_show",
                context.i64_type().fn_type(&[], false),
                None,
            ),
        );
        let function =
            codegen
                .module
                .add_function("test_fn", context.void_type().fn_type(&[], false), None);
        let entry = context.append_basic_block(function, "entry");
        codegen.current_function = Some(function);
        codegen.builder.position_at_end(entry);

        let call = expr(
            HirExprKind::MethodCall(
                Box::new(expr(
                    HirExprKind::Var("value".to_string()),
                    Type::Struct {
                        id: struct_id,
                        args: Vec::new(),
                    },
                )),
                "show".to_string(),
                Vec::new(),
                None,
                Some(HirMethodCallTarget {
                    impl_id: None,
                    trait_id: Some(trait_id),
                    trait_args: Vec::new(),
                    method_id: selected_signature_id,
                    from_index_operator: false,
                }),
            ),
            Type::I64,
        );

        let err = codegen.compile_expr(&call).unwrap_err();

        assert!(
            err.message.contains("could not be resolved by identity"),
            "unexpected error: {err}"
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p rock-lib selected_trait_target_rejects_wrong_trait_member_id`

Expected: FAIL because codegen currently accepts the trait-only target by trait ID and method name without validating the selected trait member ID.

- [ ] **Step 3: Add the codegen trait-member registry field**

In `lib/src/codegen/mod.rs`, add this field after `trait_impls`:

```rust
    /// Trait member IDs keyed by selected trait identity and source method name.
    trait_member_ids: HashMap<(DefId, String), DefId>,
```

In `CodeGen::new`, initialize it after `trait_impls: HashMap::new(),`:

```rust
            trait_member_ids: HashMap::new(),
```

- [ ] **Step 4: Populate trait-member IDs from HIR traits**

In `lib/src/codegen/mod.rs`, add this method near `register_trait_impls`:

```rust
    fn register_trait_member_ids(&mut self, program: &HirProgram) {
        for (trait_id, _, trait_def) in program.traits_by_id() {
            for (name, method) in &trait_def.methods {
                self.trait_member_ids
                    .insert((trait_id, name.clone()), method.id);
            }
            for (name, signature) in &trait_def.signatures {
                self.trait_member_ids
                    .insert((trait_id, name.clone()), signature.id);
            }
        }
    }
```

In `CodeGen::compile`, before `self.register_trait_impls(&impls);`, call:

```rust
        self.register_trait_member_ids(program);
```

- [ ] **Step 5: Add the selected trait-member validator**

In `lib/src/codegen/mod.rs`, add this helper near `find_trait_impl`:

```rust
    fn selected_trait_member_matches_target(
        &self,
        trait_id: DefId,
        method_name: &str,
        method_id: DefId,
    ) -> bool {
        self.trait_member_ids
            .get(&(trait_id, method_name.to_string()))
            .is_none_or(|selected_id| *selected_id == method_id)
    }
```

The `is_none_or` fallback preserves hand-built tests and legacy HIR fragments that do not populate trait declarations, while real `compile` inputs validate selected trait member identity.

- [ ] **Step 6: Use the validator in targeted codegen**

In `lib/src/codegen/expr/mod.rs`, inside the `from_selected_trait` block, replace:

```rust
                        target.trait_id.and_then(|trait_id| {
                            let selected_trait_args = target
```

with:

```rust
                        target.trait_id.and_then(|trait_id| {
                            if !self.selected_trait_member_matches_target(
                                trait_id,
                                method,
                                target.method_id,
                            ) {
                                return None;
                            }

                            let selected_trait_args = target
```

- [ ] **Step 7: Run focused codegen tests**

Run these one at a time:

```bash
cargo test -p rock-lib selected_trait_target_rejects_wrong_trait_member_id
cargo test -p rock-lib selected_trait_target_matches_nominal_receiver_alias_by_id
cargo test -p rock-lib selected_method_target_does_not_fallback_to_direct_alias
cargo test -p rock-lib selected_method_target_does_not_use_impl_name_when_method_id_differs
cargo test -p rock-lib selected_index_target_ref_does_not_use_builtin_index_pointer
```

Expected: PASS for each command.

- [ ] **Step 8: Commit Task 3**

Run:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/expr/mod.rs
git commit -m "validate selected trait members in codegen"
```

## Task 4: Update Roadmap And Audit Evidence

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Run implementation proof before editing docs**

Run these commands one at a time:

```bash
cargo test -p rock-lib selected_trait_impl_method_call_rejects_wrong_method_id
cargo test -p rock-lib process_selected_inherent_method_skips_trait_specialization_path
cargo test -p rock-lib selected_trait_target_rejects_wrong_trait_member_id
cargo test -p rock-lib selected_method_target_does_not_fallback_to_direct_alias
cargo test -p rock-lib selected_method_target_does_not_use_impl_name_when_method_id_differs
cargo test -p rock-lib selected_index_target_ref_does_not_use_builtin_index_pointer
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_same_name_trait_default_methods_select_bound_trait_body -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_uses_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact
cargo test -p rock-lib --test integration test_unary_not_generic_dispatches_through_stdlib_trait_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 2: Update ordered roadmap rebaseline note**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, add a new note after the Task 12 note:

```markdown
- Roadmap Task 13 has landed for selected-target fallback cleanup: mono and codegen now treat present `HirMethodCallTarget` values as authoritative identity contracts, while targetless compatibility paths remain for the Task 14-15 instance/call-edge migrations.
```

- [ ] **Step 3: Mark Task 13 complete in the ordered queue**

In the same roadmap file, under `### Task 13: Remove Mono And Codegen Selection Duplication`, add this status line after the heading:

```markdown
**Status:** Complete for selected-target fallback cleanup in `docs/superpowers/plans/2026-05-19-selection-fallback-cleanup.md`.
```

- [ ] **Step 4: Update master audit selection summary**

In `docs/superpowers/plans/master-audit-checklist.md`, change the Trait And Method Selection Service summary row from:

```markdown
| Trait And Method Selection Service | In progress | `lib/src/selection/`, `4ad4f9c`, `405a26d`, `3057204` | Shared selection service has landed; mono/codegen fallback rediscovery paths still need deletion or narrowing |
```

to:

```markdown
| Trait And Method Selection Service | In progress | `lib/src/selection/`, `lib/src/mono/methods.rs`, `lib/src/mono/process.rs`, `lib/src/codegen/expr/mod.rs`, `lib/src/codegen/mod.rs` | Selected-target fallback cleanup has landed; remaining work is broader instance/call-edge authority and direct codegen trait-selection removal in Tasks 14-15 and 21 |
```

- [ ] **Step 5: Update master audit selection details**

In the same file, under `## 5. Trait And Method Selection Service`, add this `Done` item after the shared selection-service item:

```markdown
- [x] Narrowed mono/codegen fallback rediscovery so present `HirMethodCallTarget` values are treated as authoritative selected identities instead of falling back to source-name or receiver-name lookup.
```

Replace this `Still to do` item:

```markdown
- [ ] Delete or narrow mono/codegen fallback rediscovery paths now that selected method targets are produced by the shared selection service.
```

with:

```markdown
- [ ] Remove remaining targetless compatibility paths after Tasks 14-15 replace backend-symbol call targets with instance/call-edge identity.
```

- [ ] **Step 6: Verify docs diff whitespace**

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 7: Commit Task 4**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for selection fallback cleanup"
```

## Task 5: Final Verification

**Files:**
- Verify: Rust formatting, focused regressions, integration regressions, full `rock-lib` suite, clean worktree.

- [ ] **Step 1: Format and check formatting**

Run:

```bash
cargo fmt --all
cargo fmt --all --check
```

Expected: both commands exit successfully.

- [ ] **Step 2: Run focused mono and codegen tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib selected_trait_impl_method_call_rejects_wrong_method_id
cargo test -p rock-lib process_selected_inherent_method_skips_trait_specialization_path
cargo test -p rock-lib selected_trait_target_rejects_wrong_trait_member_id
cargo test -p rock-lib selected_method_target_does_not_fallback_to_direct_alias
cargo test -p rock-lib selected_method_target_does_not_use_impl_name_when_method_id_differs
cargo test -p rock-lib selected_index_target_ref_does_not_use_builtin_index_pointer
cargo test -p rock-lib static_impl_method_call_without_selected_target_specializes_by_name
cargo test -p rock-lib targetless_show_method_does_not_return_default_value
```

Expected: PASS for each command.

- [ ] **Step 3: Run integration regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_same_name_trait_default_methods_select_bound_trait_body -- --exact
cargo test -p rock-lib --test integration test_same_name_generic_trait_default_methods_select_bound_trait_body -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_uses_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_matches_generic_impl_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact
cargo test -p rock-lib --test integration test_unary_neg_dispatches_through_stdlib_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_unary_not_generic_dispatches_through_stdlib_trait_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 4: Run the full documented library suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 5: Run final diff and status checks**

Run:

```bash
git diff --check
git status --short
```

Expected: `git diff --check` exits successfully and `git status --short` shows a clean worktree.

## Self-Review Notes

- Spec coverage: Tasks 1-2 cover mono selected-target authority; Task 3 covers codegen selected-target authority; Task 4 updates roadmap/audit evidence only after proof; Task 5 performs final verification.
- Scope control: The plan keeps targetless compatibility, backend symbol maps, and static impl method lookup for Tasks 14-15 instead of deleting all fallback lookup.
- Type/product compatibility: The plan does not persist `TypeId`, change product artifact schemas, or change structural `Type` compatibility.
