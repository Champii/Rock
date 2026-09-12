# Task 14-16 Mono Lookup Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove remaining string-keyed monomorphizer lookup from semantic callable selection when canonical IDs are already available.

**Architecture:** Add ID-keyed generic function lookup alongside existing name compatibility maps, then route resolved function call/value specialization through `DefId` before falling back to legacy name lookup. Keep backend symbols, display names, artifact names, and local lexical variable maps as metadata/compatibility, and document any remaining name fallbacks explicitly.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId`/`HirVarTarget`, `InstanceRegistry`, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## File Structure

- Modify `lib/src/mono/mod.rs`
  - Add ID-keyed generic function storage fields.
  - Add helpers for registering generic functions and looking them up by canonical `DefId`.
- Modify `lib/src/mono/process.rs`
  - Populate the new ID-keyed generic function storage when classifying functions.
  - Route `HirVarTarget::Function(def_id)` function values and callees through ID-keyed lookup.
  - Keep `HirExprKind::Var(name)` and unresolved compatibility inputs on the existing name fallback path.
- Modify `lib/src/mono/external.rs`
  - Populate ID-keyed generic function storage in crate-aware processing and external generic loading.
- Modify `lib/src/mono/methods.rs`
  - Rename the name-based static impl method fallback to make it explicit compatibility behavior.
  - Keep selected/static method-ID paths as the preferred semantic path.
- Modify `docs/superpowers/plans/master-audit-checklist.md`
  - Mark only the string-keyed mono semantic lookup item complete after verification.
- Modify `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Record this focused Task 14-16 follow-up after verification.

## Task 1: Add A Failing Resolved Function Target Regression

**Files:**
- Modify: `lib/src/mono/process.rs`

- [ ] **Step 1: Add a test helper for generic functions**

In `lib/src/mono/process.rs`, inside `#[cfg(test)] mod tests`, add this helper near `generic_static_impl_for_process_test`:

```rust
    fn generic_identity_for_process_test(id: DefId, name: &str) -> HirFunction {
        let generic_id = GenericParamId {
            owner: id,
            index: 0,
        };
        HirFunction {
            id,
            name: name.to_string(),
            qualified_name: None,
            generic_params: vec!["T".to_string()],
            generic_param_ids: vec![generic_id],
            generic_bounds: HashMap::new(),
            params: vec![HirParam {
                name: "value".to_string(),
                local_id: crate::ids::HirLocalId(0),
                ty: Type::Generic(generic_id),
                mutable: false,
                is_ref: false,
            }],
            ret_type: Type::Generic(generic_id),
            body: HirBlock {
                stmts: vec![HirStmt::Return(Some(HirExpr {
                    kind: HirExprKind::Var("value".to_string()),
                    ty: Type::Generic(generic_id),
                    span: Default::default(),
                }))],
                ty: Type::Generic(generic_id),
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }
```

- [ ] **Step 2: Add failing test for resolved function-value specialization by ID**

Add this test after `process_generic_function_call_uses_instance_target`:

```rust
    #[test]
    fn process_resolved_generic_function_value_uses_def_id_not_reference_name() {
        let mut mono = Monomorphizer::new();
        let identity_id = DefId::new(CrateId(0), LocalDefId(970));
        let wrong_id = DefId::new(CrateId(0), LocalDefId(971));
        let identity = generic_identity_for_process_test(identity_id, "identity");
        let wrong = generic_identity_for_process_test(wrong_id, "wrong_alias");

        mono.generic_functions
            .insert("identity".to_string(), identity.clone());
        mono.generic_functions
            .insert("wrong_alias".to_string(), wrong.clone());
        mono.resolver
            .item_paths
            .insert("identity".to_string(), identity_id);
        mono.resolver
            .item_paths
            .insert("wrong_alias".to_string(), wrong_id);

        let mut expr = HirExpr {
            kind: HirExprKind::ResolvedVar(HirVarRef {
                name: "wrong_alias".to_string(),
                target: HirVarTarget::Function(identity_id),
            }),
            ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
            span: Default::default(),
        };

        mono.process_expr(&mut expr);

        let HirExprKind::ResolvedVar(reference) = &expr.kind else {
            panic!("function value should remain a resolved var");
        };
        let HirVarTarget::Instance(instance_id) = reference.target else {
            panic!("resolved generic function value should target an instance");
        };
        let record = mono
            .instances
            .record(instance_id)
            .expect("instance is recorded");
        assert_eq!(record.origin, InstanceOrigin::Function(identity_id));
        assert_ne!(record.origin, InstanceOrigin::Function(wrong_id));
    }
```

- [ ] **Step 3: Add failing test for resolved generic call specialization by ID**

Add this test after `process_resolved_generic_function_value_uses_def_id_not_reference_name`:

```rust
    #[test]
    fn process_resolved_generic_call_uses_def_id_not_reference_name() {
        let mut mono = Monomorphizer::new();
        let identity_id = DefId::new(CrateId(0), LocalDefId(972));
        let wrong_id = DefId::new(CrateId(0), LocalDefId(973));
        let identity = generic_identity_for_process_test(identity_id, "identity");
        let wrong = generic_identity_for_process_test(wrong_id, "wrong_alias");

        mono.generic_functions
            .insert("identity".to_string(), identity.clone());
        mono.generic_functions
            .insert("wrong_alias".to_string(), wrong.clone());
        mono.resolver
            .item_paths
            .insert("identity".to_string(), identity_id);
        mono.resolver
            .item_paths
            .insert("wrong_alias".to_string(), wrong_id);

        let mut expr = HirExpr {
            kind: HirExprKind::Call(
                Box::new(HirExpr {
                    kind: HirExprKind::ResolvedVar(HirVarRef {
                        name: "wrong_alias".to_string(),
                        target: HirVarTarget::Function(identity_id),
                    }),
                    ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
                    span: Default::default(),
                }),
                vec![HirExpr {
                    kind: HirExprKind::IntLiteral(42),
                    ty: Type::I64,
                    span: Default::default(),
                }],
                Some(HirCallTarget::Function(identity_id)),
            ),
            ty: Type::I64,
            span: Default::default(),
        };

        mono.process_expr(&mut expr);

        let HirExprKind::Call(callee, _, target) = &expr.kind else {
            panic!("expression should remain a call");
        };
        let HirExprKind::ResolvedVar(reference) = &callee.kind else {
            panic!("callee should be instance-backed");
        };
        let HirVarTarget::Instance(instance_id) = reference.target else {
            panic!("callee should target a monomorphized instance");
        };
        let record = mono
            .instances
            .record(instance_id)
            .expect("instance is recorded");
        assert_eq!(target, &Some(HirCallTarget::Instance(instance_id)));
        assert_eq!(record.origin, InstanceOrigin::Function(identity_id));
        assert_ne!(record.origin, InstanceOrigin::Function(wrong_id));
    }
```

- [ ] **Step 4: Run the focused failing tests**

Run:

```bash
cargo test -p rock-lib mono::process::tests::process_resolved_generic_function_value_uses_def_id_not_reference_name -- --exact
cargo test -p rock-lib mono::process::tests::process_resolved_generic_call_uses_def_id_not_reference_name -- --exact
```

Expected: both tests fail because current processing derives the generic function lookup from `reference.name` instead of `HirVarTarget::Function(identity_id)`.

## Task 2: Add ID-Keyed Generic Function Lookup

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add ID-keyed fields to `Monomorphizer`**

In `lib/src/mono/mod.rs`, extend `Monomorphizer` after `external_generic_functions`:

```rust
    /// Generic functions keyed by canonical function identity for semantic lookup.
    generic_functions_by_id: HashMap<DefId, HirFunction>,
    /// External generic functions keyed by canonical function identity for semantic lookup.
    external_generic_functions_by_id: HashMap<DefId, HirFunction>,
    /// Display/backend source names for generic functions keyed by canonical function identity.
    generic_function_names_by_id: HashMap<DefId, String>,
```

Initialize these fields in both `Monomorphizer::new()` and `Monomorphizer::with_type_context(...)`:

```rust
            generic_functions_by_id: HashMap::new(),
            external_generic_functions_by_id: HashMap::new(),
            generic_function_names_by_id: HashMap::new(),
```

- [ ] **Step 2: Add registration and lookup helpers**

In `impl Monomorphizer` in `lib/src/mono/mod.rs`, add these helpers after `function_instance_origin`:

```rust
    fn register_generic_function(&mut self, name: String, func: HirFunction) {
        self.generic_function_names_by_id
            .entry(func.id)
            .or_insert_with(|| name.clone());
        self.generic_functions_by_id.insert(func.id, func.clone());
        self.generic_functions.insert(name, func);
    }

    fn register_external_generic_function(&mut self, name: String, func: HirFunction) {
        self.generic_function_names_by_id
            .entry(func.id)
            .or_insert_with(|| name.clone());
        self.external_generic_functions_by_id
            .insert(func.id, func.clone());
        self.external_generic_functions.insert(name, func);
    }

    fn generic_function_by_id(&self, id: DefId) -> Option<(String, HirFunction)> {
        let func = self
            .generic_functions_by_id
            .get(&id)
            .or_else(|| self.external_generic_functions_by_id.get(&id))?
            .clone();
        let name = self
            .generic_function_names_by_id
            .get(&id)
            .cloned()
            .or_else(|| func.qualified_name.clone())
            .unwrap_or_else(|| func.name.clone());
        Some((name, func))
    }

    fn generic_function_by_compat_name(&self, name: &str) -> Option<(String, HirFunction)> {
        let lookup_name = self.canonical_function_name(name).to_string();
        self.generic_functions
            .get(&lookup_name)
            .cloned()
            .map(|func| (lookup_name.clone(), func))
            .or_else(|| {
                self.external_generic_functions
                    .get(&lookup_name)
                    .cloned()
                    .map(|func| (lookup_name, func))
            })
    }
```

- [ ] **Step 3: Populate ID-keyed fields in non-crate processing**

In `lib/src/mono/process.rs`, replace this block in `process`:

```rust
            } else {
                self.generic_functions.insert(name, func);
            }
```

with:

```rust
            } else {
                self.register_generic_function(name, func);
            }
```

- [ ] **Step 4: Populate ID-keyed fields in crate-aware processing**

In `lib/src/mono/external.rs`, replace the same generic insertion in `process_with_crates_impl`:

```rust
            } else {
                self.generic_functions.insert(name, func);
            }
```

with:

```rust
            } else {
                self.register_generic_function(name, func);
            }
```

In `load_external_generic_functions`, replace this loop:

```rust
                for (qualified_name, func) in dep.bodies().generic_functions() {
                    self.external_generic_functions
                        .insert(qualified_name.clone(), func.clone());
                }
```

with:

```rust
                for (qualified_name, func) in dep.bodies().generic_functions() {
                    self.register_external_generic_function(qualified_name.clone(), func.clone());
                }
```

Leave the existing alias loop unchanged; it should continue to populate `self.function_aliases` for compatibility name resolution.

- [ ] **Step 5: Run compile-focused tests**

Run:

```bash
cargo test -p rock-lib mono::process::tests::process_generic_function_call_uses_instance_target -- --exact
cargo test -p rock-lib mono::external::tests::load_external_generic_functions_registers_module_aliases_for_generic_functions -- --exact
```

Expected: both pass after the helper refactor.

## Task 3: Route Resolved Function Targets Through ID-Keyed Lookup

**Files:**
- Modify: `lib/src/mono/process.rs`

- [ ] **Step 1: Update function-value specialization for resolved function targets**

In `process_expr`, in the `HirExprKind::ResolvedVar(reference)` arm, replace the current `lookup_name` match and name-map lookup with:

```rust
                let generic_target = match reference.target {
                    HirVarTarget::Function(id) if self.is_static_impl_method_id(id) => {
                        if !self.contains_generic(&expr.ty) {
                            if let Some((instance_id, specialized_func_type)) =
                                self.monomorphize_static_impl_method_value_by_id(id, &expr.ty)
                            {
                                reference.target = HirVarTarget::Instance(instance_id);
                                expr.ty = specialized_func_type;
                            }
                        }
                        return;
                    }
                    HirVarTarget::Function(id) => self.generic_function_by_id(id),
                    HirVarTarget::Extern(_) => self.generic_function_by_compat_name(&reference.name),
                    HirVarTarget::Instance(_) | HirVarTarget::Local(_) => return,
                };

                if let Some((lookup_name, generic_func)) = generic_target {
                    if !self.contains_generic(&expr.ty) {
                        if let Some(type_args) =
                            self.extract_type_args_from_expr_type(&generic_func, &expr.ty)
                        {
                            if let Some((instance_id, specialized_func_type, _)) = self
                                .monomorphize_with_type_args(
                                    &lookup_name,
                                    &generic_func,
                                    &type_args,
                                )
                            {
                                expr.kind = HirExprKind::ResolvedVar(HirVarRef {
                                    name: lookup_name.clone(),
                                    target: HirVarTarget::Instance(instance_id),
                                });
                                expr.ty = specialized_func_type;
                            }
                        }
                    }
                }
```

- [ ] **Step 2: Update call specialization for resolved function targets**

In the `HirExprKind::Call(func, args, target)` arm, replace the `func_name`/`func_def_id` generic function lookup section with this ID-first shape:

```rust
                let generic_target = match &func.kind {
                    HirExprKind::ResolvedVar(reference) => match reference.target {
                        HirVarTarget::Function(id) => self.generic_function_by_id(id),
                        HirVarTarget::Extern(_) => self.generic_function_by_compat_name(&reference.name),
                        HirVarTarget::Instance(_) | HirVarTarget::Local(_) => None,
                    },
                    HirExprKind::Var(name) => self.generic_function_by_compat_name(name),
                    _ => None,
                };
                let func_def_id = match &func.kind {
                    HirExprKind::ResolvedVar(reference) => match reference.target {
                        HirVarTarget::Function(id) => Some(id),
                        HirVarTarget::Extern(_)
                        | HirVarTarget::Instance(_)
                        | HirVarTarget::Local(_) => None,
                    },
                    _ => None,
                };
                let fallback_name = match &func.kind {
                    HirExprKind::Var(name) => Some(name.clone()),
                    HirExprKind::ResolvedVar(reference) => Some(reference.name.clone()),
                    _ => None,
                };
```

Then replace the generic lookup block under `if let Some(ref name) = func_name` with:

```rust
                if let Some(name) = fallback_name {
                    let mut resolved_static_impl = false;
                    if let Some(method_id) = func_def_id {
                        if let Some((instance_id, specialized_func_type, specialized_ret_type)) =
                            self.monomorphize_static_impl_method_call_by_id(
                                method_id,
                                &args_for_mono,
                                &expr.ty,
                            )
                        {
                            *func = Box::new(self.instance_callable_expr(
                                name.clone(),
                                instance_id,
                                specialized_func_type,
                                func.span.clone(),
                            ));
                            expr.ty = specialized_ret_type;
                            resolved_static_impl = true;
                        }
                    }

                    if !resolved_static_impl {
                        if let Some((lookup_name, generic_func)) = generic_target {
                            if let Some((instance_id, specialized_func_type, specialized_ret_type)) =
                                self.monomorphize_call(&lookup_name, &generic_func, &args_for_mono)
                            {
                                *func = Box::new(self.instance_callable_expr(
                                    lookup_name.clone(),
                                    instance_id,
                                    specialized_func_type,
                                    func.span.clone(),
                                ));
                                expr.ty = specialized_ret_type;
                            }
                        } else if func_def_id.is_none() {
                            if let Some((instance_id, specialized_func_type, specialized_ret_type)) =
                                self.monomorphize_static_impl_method_call(
                                    &name,
                                    &args_for_mono,
                                    &expr.ty,
                                )
                            {
                                *func = Box::new(self.instance_callable_expr(
                                    name.clone(),
                                    instance_id,
                                    specialized_func_type,
                                    func.span.clone(),
                                ));
                                expr.ty = specialized_ret_type;
                            }
                        }
                    }
                }
```

- [ ] **Step 3: Run the focused tests from Task 1**

Run:

```bash
cargo test -p rock-lib mono::process::tests::process_resolved_generic_function_value_uses_def_id_not_reference_name -- --exact
cargo test -p rock-lib mono::process::tests::process_resolved_generic_call_uses_def_id_not_reference_name -- --exact
```

Expected: both pass.

## Task 4: Make Remaining Name Fallbacks Explicit Compatibility Paths

**Files:**
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/process.rs`

- [ ] **Step 1: Rename the static impl name fallback**

In `lib/src/mono/methods.rs`, rename:

```rust
    pub(super) fn monomorphize_static_impl_method_call(
```

to:

```rust
    pub(super) fn monomorphize_static_impl_method_call_by_compat_name(
```

Keep the function body unchanged.

- [ ] **Step 2: Update call sites and tests**

In `lib/src/mono/process.rs`, update the fallback call site to:

```rust
                            self.monomorphize_static_impl_method_call_by_compat_name(
                                &name,
                                &args_for_mono,
                                &expr.ty,
                            )
```

In `lib/src/mono/methods.rs` tests, rename direct test calls from:

```rust
            .monomorphize_static_impl_method_call("Foo_make", &args, &Type::Bool)
```

to:

```rust
            .monomorphize_static_impl_method_call_by_compat_name("Foo_make", &args, &Type::Bool)
```

- [ ] **Step 3: Add a short comment above the renamed function**

Add this comment immediately above the renamed function:

```rust
    /// Compatibility fallback for unresolved static impl callables that still arrive by name.
    /// Resolved `DefId` targets must use `monomorphize_static_impl_method_call_by_id`.
```

- [ ] **Step 4: Run static impl compatibility tests**

Run:

```bash
cargo test -p rock-lib mono::methods::tests::static_impl_method_call_without_selected_target_specializes_by_name -- --exact
cargo test -p rock-lib mono::process::tests::process_static_impl_resolved_function_target_uses_method_id_not_name -- --exact
cargo test -p rock-lib mono::process::tests::process_static_impl_resolved_function_value_uses_method_instance_target -- --exact
```

Expected: all pass. The first test proves the named fallback still exists for unresolved compatibility input; the latter two prove resolved targets use method IDs.

## Task 5: Remove Redundant Resolved-Target Name Lookups In Nested Call Arguments

**Files:**
- Modify: `lib/src/mono/process.rs`

- [ ] **Step 1: Locate nested function-value argument handling**

In `process_expr`, locate the block that starts with:

```rust
                let callee_func = match &func.kind {
```

and contains:

```rust
                                if let Some(generic_func) =
                                    self.generic_functions.get(arg_name).cloned()
```

- [ ] **Step 2: Replace resolved function-target lookup with the helper**

When the argument is `HirExprKind::ResolvedVar(reference)`, use this shape before falling back to name compatibility:

```rust
                        let generic_target = match reference.target {
                            HirVarTarget::Function(id) => self.generic_function_by_id(id),
                            HirVarTarget::Extern(_) => {
                                self.generic_function_by_compat_name(&reference.name)
                            }
                            HirVarTarget::Instance(_) | HirVarTarget::Local(_) => None,
                        };
```

When the argument is `HirExprKind::Var(arg_name)`, keep:

```rust
                        let generic_target = self.generic_function_by_compat_name(arg_name);
```

Then call `monomorphize_with_type_args(&lookup_name, &generic_func, &type_args)` using the `(lookup_name, generic_func)` pair from the helper.

- [ ] **Step 3: Add a regression test for function-valued resolved arguments**

In `lib/src/mono/process.rs`, add this test near the Task 1 tests:

```rust
    #[test]
    fn process_resolved_generic_function_argument_uses_def_id_not_reference_name() {
        let mut mono = Monomorphizer::new();
        let identity_id = DefId::new(CrateId(0), LocalDefId(974));
        let wrong_id = DefId::new(CrateId(0), LocalDefId(975));
        let identity = generic_identity_for_process_test(identity_id, "identity");
        let wrong = generic_identity_for_process_test(wrong_id, "wrong_alias");

        mono.generic_functions
            .insert("identity".to_string(), identity.clone());
        mono.generic_functions
            .insert("wrong_alias".to_string(), wrong.clone());
        mono.resolver
            .item_paths
            .insert("identity".to_string(), identity_id);
        mono.resolver
            .item_paths
            .insert("wrong_alias".to_string(), wrong_id);

        let mut expr = HirExpr {
            kind: HirExprKind::Call(
                Box::new(HirExpr {
                    kind: HirExprKind::Var("apply".to_string()),
                    ty: Type::Function(
                        vec![Type::Function(vec![Type::I64], Box::new(Type::I64))],
                        Box::new(Type::I64),
                    ),
                    span: Default::default(),
                }),
                vec![HirExpr {
                    kind: HirExprKind::ResolvedVar(HirVarRef {
                        name: "wrong_alias".to_string(),
                        target: HirVarTarget::Function(identity_id),
                    }),
                    ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
                    span: Default::default(),
                }],
                None,
            ),
            ty: Type::I64,
            span: Default::default(),
        };

        mono.process_expr(&mut expr);

        let HirExprKind::Call(_, args, _) = &expr.kind else {
            panic!("expression should remain a call");
        };
        let HirExprKind::ResolvedVar(reference) = &args[0].kind else {
            panic!("argument should be rewritten to a resolved instance var");
        };
        let HirVarTarget::Instance(instance_id) = reference.target else {
            panic!("argument should target a monomorphized instance");
        };
        let record = mono
            .instances
            .record(instance_id)
            .expect("instance is recorded");
        assert_eq!(record.origin, InstanceOrigin::Function(identity_id));
        assert_ne!(record.origin, InstanceOrigin::Function(wrong_id));
    }
```

- [ ] **Step 4: Run the nested-argument regression**

Run:

```bash
cargo test -p rock-lib mono::process::tests::process_resolved_generic_function_argument_uses_def_id_not_reference_name -- --exact
```

Expected: pass.

## Task 6: Audit Remaining Mono String Lookup Hits And Document Classification

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Search remaining mono string-keyed lookup sites**

Run:

```bash
rg "generic_functions\.get|external_generic_functions\.get|concrete_functions\.get|monomorphize_static_impl_method_call_by_compat_name|impl_signature_key|var_types" lib/src/mono
```

Expected: output remains, but every remaining semantic function specialization should either call `generic_function_by_id` or be an explicit compatibility/local metadata path.

- [ ] **Step 2: Add classification comments only where useful**

If a remaining string lookup is not self-evidently compatibility metadata, add a short comment like:

```rust
                // Compatibility path for unresolved `Var` expressions; resolved function targets use `generic_function_by_id`.
```

Do not add comments to obvious local lexical maps such as `var_types` unless a test or reviewer would otherwise confuse them with callable identity lookup.

- [ ] **Step 3: Run focused mono regression filters**

Run:

```bash
cargo test -p rock-lib mono::process::tests::process_generic_function_call_uses_instance_target -- --exact
cargo test -p rock-lib mono::process::tests::process_call_does_not_resolve_callee_by_backend_symbol -- --exact
cargo test -p rock-lib mono::process::tests::process_static_impl_resolved_function_target_uses_method_id_not_name -- --exact
cargo test -p rock-lib mono::process::tests::process_static_impl_resolved_function_value_uses_method_instance_target -- --exact
cargo test -p rock-lib mono::methods::tests::monomorphize_same_named_methods_on_different_impls_use_distinct_method_origins -- --exact
```

Expected: all pass.

## Task 7: Update Roadmap And Audit Checklist

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [ ] **Step 1: Update master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, under `## 7. Monomorphization Instances`, add evidence that resolved generic function calls/values use ID-keyed generic function lookup. Then change the first `Still to do` item from unchecked to checked only if the implementation evidence supports it:

```markdown
- [x] Removed remaining string-keyed monomorphizer lookup tables from semantic control flow where canonical function, method, or instance identities are available; remaining string-keyed mono paths are compatibility, local lexical metadata, diagnostics/display, artifact interface, or backend/link metadata.
```

Keep these items unchecked:

```markdown
- [ ] Retire legacy non-pipeline HIR/name pruning helpers once their compatibility tests are no longer needed.
- [ ] Carry instance bodies toward the eventual MIR/codegen boundary instead of storing instance bodies as `HirFunction` records long term.
```

- [ ] **Step 2: Update ordered roadmap**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the Task 14-16 rows or verification notes to mention this follow-up:

```markdown
Resolved generic function calls and function values now use canonical `DefId`/`InstanceId` lookup before name compatibility, so string-keyed mono maps no longer drive semantic callable selection when canonical IDs are available.
```

Do not claim that legacy HIR/name DCE helper retirement or instance body representation cleanup is complete.

- [ ] **Step 3: Verify documentation diff**

Run:

```bash
git diff -- docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git diff --check
```

Expected: docs accurately distinguish the completed string-lookup cleanup from remaining Task 14-16 follow-ups.

## Task 8: Final Verification And Commit

**Files:**
- All modified implementation and documentation files.

- [ ] **Step 1: Run formatting check**

Run:

```bash
cargo fmt --all --check
```

Expected: pass. If it fails, run `cargo fmt --all`, then rerun `cargo fmt --all --check`.

- [ ] **Step 2: Run focused regression filters**

Run:

```bash
cargo test -p rock-lib mono::process::tests::process_resolved_generic_function_value_uses_def_id_not_reference_name -- --exact
cargo test -p rock-lib mono::process::tests::process_resolved_generic_call_uses_def_id_not_reference_name -- --exact
cargo test -p rock-lib mono::process::tests::process_resolved_generic_function_argument_uses_def_id_not_reference_name -- --exact
cargo test -p rock-lib mono::process::tests::process_call_does_not_resolve_callee_by_backend_symbol -- --exact
cargo test -p rock-lib mono::methods::tests::monomorphize_same_named_methods_on_different_impls_use_distinct_method_origins -- --exact
```

Expected: all pass.

- [ ] **Step 3: Run broader mono/integration checks**

Run:

```bash
cargo test -p rock-lib generic -- --nocapture
cargo test -p rock-lib static_impl -- --nocapture
cargo test -p rock-lib --test integration test_generic -- --nocapture
cargo test -p rock-lib --test integration test_stdlib -- --nocapture
```

Expected: all pass.

- [ ] **Step 4: Run full library tests**

Run:

```bash
cargo test -p rock-lib > /tmp/rock-lib-task14-16-mono-lookup-cleanup.log 2>&1
```

Expected: exit code 0. If it fails, inspect `/tmp/rock-lib-task14-16-mono-lookup-cleanup.log`, fix the failure, and rerun the smallest failing test first before rerunning the full command.

- [ ] **Step 5: Run diff checks**

Run:

```bash
git diff --check
git status --short --branch
```

Expected: no whitespace errors; only intended files modified.

- [ ] **Step 6: Commit**

Run:

```bash
git add lib/src/mono/mod.rs lib/src/mono/process.rs lib/src/mono/external.rs lib/src/mono/methods.rs docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git commit -m "complete task 14-16 mono lookup cleanup"
```

Expected: commit succeeds.
