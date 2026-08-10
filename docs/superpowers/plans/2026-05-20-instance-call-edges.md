# Instance Call Edges Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace known monomorphized backend-symbol call targets with explicit `InstanceId` call edges.

**Architecture:** Use the existing `HirExprKind::ResolvedVar(HirVarRef)` compatibility shape and extend `HirVarTarget` with `Instance(InstanceId)` instead of adding a parallel call-expression enum. Move mono registry IDs onto the canonical `crate::ids::InstanceId`, have monomorphization return instance IDs for selected generic callables, and have codegen resolve those IDs through the registered `InstanceRecord.backend_symbol` map.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId` and `InstanceId`, mono `InstanceRegistry`, inkwell codegen, serde derives, focused Cargo unit and integration tests.

---

## Scope

This plan implements `docs/superpowers/specs/2026-05-20-instance-call-edges-design.md`.

In scope:
- Known monomorphized generic function calls use `HirVarTarget::Instance`.
- Known monomorphized generic function values use `HirVarTarget::Instance`.
- Known monomorphized generic impl-method, trait-method, and static impl-method calls use `HirVarTarget::Instance`.
- Codegen resolves instance targets only through registered instance records.
- Direct function and extern `DefId` targets keep their current paths.

Out of scope:
- No DCE rewrite.
- No MIR codegen rewrite.
- No product artifact schema change.
- No broad removal of source-name maps used for diagnostics, local variables, or template lookup.

---

## File Structure

- Read `lib/src/ids.rs`: use the existing canonical `InstanceId`; no new ID type.
- Modify `lib/src/mono/registry.rs`: remove the duplicate local `InstanceId` and import `crate::ids::InstanceId`.
- Modify `lib/src/mono/mod.rs`: re-export canonical `InstanceId` through `crate::mono`, add helper methods for instance-backed HIR callable expressions, and add an instance-body lookup helper.
- Modify `lib/src/hir/mod.rs`: import `InstanceId`, add `HirVarTarget::Instance(InstanceId)`, and update HIR tests.
- Modify `lib/src/products.rs`: make product ID remap code leave local mono instance targets untouched.
- Modify `lib/src/mir/builder/expr.rs`: make the existing resolved-var match exhaustive for the new target.
- Modify `lib/src/mono/specialize.rs`: make function specialization helpers return `InstanceId` plus function type metadata.
- Modify `lib/src/mono/process.rs`: rewrite generic call callees and function-value arguments to instance-backed `ResolvedVar` expressions.
- Modify `lib/src/mono/methods.rs`: rewrite generic method specialization call sites to instance-backed `ResolvedVar` callees.
- Modify `lib/src/mono/external.rs`: update tests that currently assert specialized callees are backend-symbol `Var` expressions.
- Modify `lib/src/codegen/mod.rs`: store `InstanceId -> backend_symbol` while registering instances.
- Modify `lib/src/codegen/expr/mod.rs`: materialize instance-backed callables through the registered backend symbol and update expression traversal.
- Modify `lib/src/codegen/expr/call.rs`: resolve direct instance-backed calls through the registered backend symbol and fail clearly when missing.
- Test in `lib/src/mono/process.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/external.rs`, `lib/src/codegen/expr/mod.rs`, and `lib/tests/integration.rs`.

---

### Task 1: Canonical Instance Target Data Model

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/products.rs`
- Modify: `lib/src/mir/builder/expr.rs`

- [ ] **Step 1: Write failing data-model tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `lib/src/mono/registry.rs`:

```rust
#[test]
fn mono_reexports_canonical_instance_id() {
    let id: crate::ids::InstanceId = crate::mono::InstanceId(7);

    assert_eq!(id.0, 7);
}
```

Add this test to the existing `#[cfg(test)] mod tests` in `lib/src/hir/mod.rs`:

```rust
#[test]
fn hir_var_target_can_reference_monomorphized_instance() {
    let target = HirVarTarget::Instance(crate::ids::InstanceId(9));

    assert_eq!(target, HirVarTarget::Instance(crate::ids::InstanceId(9)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib mono_reexports_canonical_instance_id -- --exact`

Expected: FAIL before implementation because `crate::mono::InstanceId` is a distinct type from `crate::ids::InstanceId`.

Run: `cargo test -p rock-lib hir_var_target_can_reference_monomorphized_instance -- --exact`

Expected: FAIL before implementation because `HirVarTarget::Instance` is not defined.

- [ ] **Step 3: Use canonical `InstanceId` in the mono registry**

In `lib/src/mono/registry.rs`, replace the ID import and local type with this import:

```rust
use crate::ids::{DefId, InstanceId};
```

Delete this local duplicate type from `lib/src/mono/registry.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(pub u32);
```

In `lib/src/mono/mod.rs`, replace the current registry re-export block with this:

```rust
pub use crate::ids::InstanceId;
pub use registry::{
    InstanceImplOwner, InstanceKey, InstanceOrigin, InstanceRecord, InstanceRegistry,
    MonomorphizedProgram,
};
```

- [ ] **Step 4: Add instance-backed HIR targets**

In `lib/src/hir/mod.rs`, extend the ID import:

```rust
use crate::ids::{AssocTypeId, DefId, FieldId, InstanceId, TypeVarId, VariantId};
```

Replace `HirVarTarget` with this enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirVarTarget {
    Function(DefId),
    Extern(DefId),
    Instance(InstanceId),
}
```

- [ ] **Step 5: Make existing phase matches exhaustive**

In `lib/src/products.rs`, update `remap_var_target_product_ids`:

```rust
fn remap_var_target_product_ids(
    target: &mut HirVarTarget,
    id_remap: &BTreeMap<ProductDefId, BTreeSet<ProductDefId>>,
) {
    match target {
        HirVarTarget::Function(id) | HirVarTarget::Extern(id) => {
            remap_product_def_id_owner(id, id_remap);
        }
        HirVarTarget::Instance(_) => {}
    }
}
```

In `lib/src/products.rs`, update `remap_var_target_owned_type_ids`:

```rust
fn remap_var_target_owned_type_ids(target: &mut HirVarTarget, old_id: DefId, new_id: DefId) {
    match target {
        HirVarTarget::Function(id) | HirVarTarget::Extern(id) => {
            remap_def_id_if_matches(id, old_id, new_id);
        }
        HirVarTarget::Instance(_) => {}
    }
}
```

In `lib/src/mir/builder/expr.rs`, update the resolved-var target match:

```rust
let is_known_target = match reference.target {
    HirVarTarget::Function(id) => self.program.function_by_id(id).is_some(),
    HirVarTarget::Extern(id) => self.program.extern_by_id(id).is_some(),
    HirVarTarget::Instance(_) => false,
};
```

- [ ] **Step 6: Run focused tests**

Run: `cargo test -p rock-lib mono_reexports_canonical_instance_id -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib hir_var_target_can_reference_monomorphized_instance -- --exact`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add lib/src/mono/registry.rs lib/src/mono/mod.rs lib/src/hir/mod.rs lib/src/products.rs lib/src/mir/builder/expr.rs
git commit -m "add instance-backed hir targets"
```

---

### Task 2: Generic Function Calls Use Instance Targets

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/specialize.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Write failing mono tests for generic function calls**

Add this helper inside the existing `#[cfg(test)] mod tests` in `lib/src/mono/specialize.rs`:

```rust
fn call_expr(callee: HirExpr, args: Vec<HirExpr>, ret_ty: Type) -> HirExpr {
    HirExpr {
        kind: HirExprKind::Call(Box::new(callee), args),
        ty: ret_ty,
        span: Default::default(),
    }
}
```

Add this test to `lib/src/mono/specialize.rs`:

```rust
#[test]
fn monomorphize_call_returns_instance_id_for_new_specialization() {
    let mut mono = Monomorphizer::new();
    let id = def_id(10);
    let generic = generic_function(
        id,
        Type::Generic(GenericParamId { owner: id, index: 0 }),
        Type::Generic(GenericParamId { owner: id, index: 0 }),
    );
    mono.resolver.item_paths.insert("identity".to_string(), id);
    let args = vec![HirExpr {
        kind: HirExprKind::IntLiteral(21),
        ty: Type::I64,
        span: Default::default(),
    }];

    let (instance_id, func_ty, ret_ty) = mono
        .monomorphize_call("identity", &generic, &args)
        .expect("generic call should specialize");

    assert_eq!(ret_ty, Type::I64);
    assert_eq!(func_ty, Type::Function(vec![Type::I64], Box::new(Type::I64)));
    let record = mono.instances.record(instance_id).expect("instance is recorded");
    assert_eq!(record.origin, crate::mono::InstanceOrigin::Function(id));
    assert_eq!(record.substitution, vec![Type::I64]);
}
```

Add this test to the existing `#[cfg(test)] mod tests` in `lib/src/mono/process.rs`:

```rust
#[test]
fn process_generic_function_call_uses_instance_target() {
    let mut mono = Monomorphizer::new();
    let id = DefId::new(CrateId(0), LocalDefId(910));
    let generic = HirFunction {
        id,
        name: "identity".to_string(),
        qualified_name: None,
        generic_params: vec!["T".to_string()],
        generic_param_ids: vec![GenericParamId { owner: id, index: 0 }],
        generic_bounds: HashMap::new(),
        params: vec![HirParam {
            name: "value".to_string(),
            ty: Type::Generic(GenericParamId { owner: id, index: 0 }),
            mutable: false,
            is_ref: false,
        }],
        ret_type: Type::Generic(GenericParamId { owner: id, index: 0 }),
        body: HirBlock {
            stmts: vec![HirStmt::Return(Some(HirExpr {
                kind: HirExprKind::Var("value".to_string()),
                ty: Type::Generic(GenericParamId { owner: id, index: 0 }),
                span: Default::default(),
            }))],
            ty: Type::Generic(GenericParamId { owner: id, index: 0 }),
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };
    mono.generic_functions.insert("identity".to_string(), generic);
    mono.resolver.item_paths.insert("identity".to_string(), id);
    let mut expr = HirExpr {
        kind: HirExprKind::Call(
            Box::new(HirExpr {
                kind: HirExprKind::Var("identity".to_string()),
                ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
                span: Default::default(),
            }),
            vec![HirExpr {
                kind: HirExprKind::IntLiteral(21),
                ty: Type::I64,
                span: Default::default(),
            }],
        ),
        ty: Type::I64,
        span: Default::default(),
    };

    mono.process_expr(&mut expr);

    let HirExprKind::Call(callee, _) = &expr.kind else {
        panic!("expression should remain a call");
    };
    let HirExprKind::ResolvedVar(reference) = &callee.kind else {
        panic!("callee should be an instance-backed resolved var");
    };
    let HirVarTarget::Instance(instance_id) = reference.target else {
        panic!("callee should target a monomorphized instance");
    };
    let record = mono.instances.record(instance_id).expect("instance is recorded");
    assert_ne!(reference.name, record.backend_symbol);
    assert_eq!(record.source_name, "identity");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib monomorphize_call_returns_instance_id_for_new_specialization -- --exact`

Expected: FAIL before implementation because `monomorphize_call` returns a backend symbol string.

Run: `cargo test -p rock-lib process_generic_function_call_uses_instance_target -- --exact`

Expected: FAIL before implementation because the processed callee is `HirExprKind::Var(record.backend_symbol)`.

- [ ] **Step 3: Add mono helper methods for instance callables**

Add these methods to the existing `impl Monomorphizer` in `lib/src/mono/mod.rs` near `lookup_specialized_function`:

```rust
fn lookup_instance_function(&self, instance_id: InstanceId) -> Option<HirFunction> {
    self.instances
        .record(instance_id)
        .and_then(|record| record.body.clone())
}

fn instance_callable_expr(
    &self,
    display_name: String,
    instance_id: InstanceId,
    ty: Type,
    span: crate::lexer::Span,
) -> HirExpr {
    HirExpr {
        kind: HirExprKind::ResolvedVar(HirVarRef {
            name: display_name,
            target: HirVarTarget::Instance(instance_id),
        }),
        ty,
        span,
    }
}
```

Keep `lookup_specialized_function` for compatibility during this task, then stop using it for new instance-targeted callees in `process_expr`.

- [ ] **Step 4: Change function specialization return values**

In `lib/src/mono/specialize.rs`, change both `monomorphize_call` and `monomorphize_with_type_args` signatures from:

```rust
) -> Option<(String, Type, Type)> {
```

to:

```rust
) -> Option<(crate::mono::InstanceId, Type, Type)> {
```

In each reused-instance return path, return the existing instance ID:

```rust
return Some((instance_id, func_type, body.ret_type.clone()));
```

In each newly-created-instance return path, return the newly interned ID:

```rust
return Some((instance_id, func_type, body.ret_type.clone()));
```

Remove fallback tuple returns that expose `specialized_name` after successful interning. If record lookup unexpectedly fails after interning, keep returning the interned ID with the already computed `specialized_func_type` and `specialized_ret_type`:

```rust
Some((instance_id, specialized_func_type, specialized_ret_type))
```

- [ ] **Step 5: Rewrite generic calls and function values to instance targets**

In `lib/src/mono/process.rs`, replace assignments like this:

```rust
expr.kind = HirExprKind::Var(specialized_name);
```

with this shape:

```rust
expr.kind = HirExprKind::ResolvedVar(HirVarRef {
    name: lookup_name.clone(),
    target: HirVarTarget::Instance(instance_id),
});
```

In `HirExprKind::Call`, replace callee rewrites like this:

```rust
*func = Box::new(HirExpr {
    kind: HirExprKind::Var(specialized_name.clone()),
    ty: specialized_func_type,
    span: func.span.clone(),
});
```

with this shape:

```rust
*func = Box::new(self.instance_callable_expr(
    lookup_name.clone(),
    instance_id,
    specialized_func_type,
    func.span.clone(),
));
```

In the call-argument specialization block, replace argument rewrites like this:

```rust
arg.kind = HirExprKind::Var(spec_name);
arg.ty = spec_func_type;
```

with this shape:

```rust
arg.kind = HirExprKind::ResolvedVar(HirVarRef {
    name: arg_name.to_string(),
    target: HirVarTarget::Instance(instance_id),
});
arg.ty = spec_func_type;
```

Update the callee-function lookup in `HirExprKind::Call` to use instance IDs directly:

```rust
let callee_func = match &func.kind {
    HirExprKind::Var(callee_name) => self
        .lookup_specialized_function(callee_name)
        .or_else(|| self.concrete_functions.get(callee_name).cloned())
        .or_else(|| self.generic_functions.get(callee_name).cloned())
        .or_else(|| self.external_generic_functions.get(callee_name).cloned()),
    HirExprKind::ResolvedVar(reference) => match reference.target {
        HirVarTarget::Instance(instance_id) => self.lookup_instance_function(instance_id),
        HirVarTarget::Function(_) => self
            .concrete_functions
            .get(&reference.name)
            .cloned()
            .or_else(|| self.generic_functions.get(&reference.name).cloned())
            .or_else(|| self.external_generic_functions.get(&reference.name).cloned()),
        HirVarTarget::Extern(_) => None,
    },
    _ => None,
};
```

- [ ] **Step 6: Update external mono test expectations**

In `lib/src/mono/external.rs`, update the assertion around the trait-default generic call. Replace this assertion shape:

```rust
let HirExprKind::Var(specialized_name) = &callee.kind else {
    panic!("trait default call callee should be specialized");
};
assert_ne!(specialized_name, "identity");
assert!(specialized_name.starts_with("identity_mono_"));
```

with this instance-target assertion:

```rust
let HirExprKind::ResolvedVar(reference) = &callee.kind else {
    panic!("trait default call callee should be a resolved instance target");
};
let HirVarTarget::Instance(instance_id) = reference.target else {
    panic!("trait default call callee should target an instance");
};
assert_eq!(reference.name, "identity");
```

Replace the specialization backend-symbol assertion with this exact check:

```rust
let specialization = output
    .instances
    .get(&instance_id)
    .expect("trait default should register the generic specialization it calls");
assert_eq!(specialization.origin, crate::mono::InstanceOrigin::Function(generic_id));
assert_eq!(specialization.substitution, vec![Type::I32]);
assert!(specialization.backend_symbol.starts_with("identity_mono_"));
```

- [ ] **Step 7: Run focused mono tests**

Run: `cargo test -p rock-lib process_generic_function_call_uses_instance_target -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib monomorphize_call_returns_instance_id_for_new_specialization -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib process_with_crates_processes_concrete_trait_default_body_before_registration -- --exact`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add lib/src/mono/mod.rs lib/src/mono/specialize.rs lib/src/mono/process.rs lib/src/mono/external.rs
git commit -m "target generic function calls by instance"
```

---

### Task 3: Generic Method Calls Use Instance Targets

**Files:**
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/process.rs`

- [ ] **Step 1: Write failing method specialization tests**

Add this test to the existing `#[cfg(test)] mod tests` in `lib/src/mono/process.rs` after `process_selected_inherent_method_skips_trait_specialization_path`:

```rust
#[test]
fn process_selected_generic_method_call_uses_instance_target() {
    let mut mono = Monomorphizer::new();
    let impl_id = DefId::new(CrateId(0), LocalDefId(921));
    let method_id = DefId::new(CrateId(0), LocalDefId(922));
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
            Some(HirMethodCallTarget {
                impl_id: Some(impl_id),
                trait_id: None,
                trait_args: Vec::new(),
                method_id,
                from_index_operator: false,
            }),
        ),
        ty: Type::I64,
        span: Default::default(),
    };

    mono.process_expr(&mut expr);

    let HirExprKind::Call(callee, _) = &expr.kind else {
        panic!("method call should lower to a direct call");
    };
    let HirExprKind::ResolvedVar(reference) = &callee.kind else {
        panic!("method callee should be an instance-backed resolved var");
    };
    let HirVarTarget::Instance(instance_id) = reference.target else {
        panic!("method callee should target an instance");
    };
    let record = mono.instances.record(instance_id).expect("instance is recorded");
    assert_eq!(
        record.origin,
        InstanceOrigin::ImplMethod {
            owner: InstanceImplOwner::Named(impl_id),
            method: method_id,
        }
    );
}
```

Add this test to the same module:

```rust
#[test]
fn process_reused_generic_method_call_keeps_instance_target() {
    let mut mono = Monomorphizer::new();
    let impl_id = DefId::new(CrateId(0), LocalDefId(931));
    let method_id = DefId::new(CrateId(0), LocalDefId(932));
    mono.resolver
        .item_names_by_id
        .insert(DefId::new(CrateId(0), LocalDefId(700)), "Box".to_string());
    mono.generic_impls.insert(
        "Box".to_string(),
        generic_impl_for_process_test("Box", impl_id, None, method_id),
    );

    let mut first = HirExpr {
        kind: HirExprKind::MethodCall(
            Box::new(HirExpr {
                kind: HirExprKind::Var("first".to_string()),
                ty: Type::Struct {
                    id: DefId::new(CrateId(0), LocalDefId(700)),
                    args: vec![Type::I64],
                },
                span: Default::default(),
            }),
            "map".to_string(),
            Vec::new(),
            Some(SelfReceiverMode::Move),
            Some(HirMethodCallTarget {
                impl_id: Some(impl_id),
                trait_id: None,
                trait_args: Vec::new(),
                method_id,
                from_index_operator: false,
            }),
        ),
        ty: Type::I64,
        span: Default::default(),
    };
    let mut second = first.clone();

    mono.process_expr(&mut first);
    mono.process_expr(&mut second);

    let first_id = match &first.kind {
        HirExprKind::Call(callee, _) => match &callee.kind {
            HirExprKind::ResolvedVar(reference) => match reference.target {
                HirVarTarget::Instance(id) => id,
                _ => panic!("first call should target an instance"),
            },
            _ => panic!("first call callee should be resolved"),
        },
        _ => panic!("first method call should lower to a direct call"),
    };
    let second_id = match &second.kind {
        HirExprKind::Call(callee, _) => match &callee.kind {
            HirExprKind::ResolvedVar(reference) => match reference.target {
                HirVarTarget::Instance(id) => id,
                _ => panic!("second call should target an instance"),
            },
            _ => panic!("second call callee should be resolved"),
        },
        _ => panic!("second method call should lower to a direct call"),
    };

    assert_eq!(first_id, second_id);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib process_selected_generic_method_call_uses_instance_target -- --exact`

Expected: FAIL before implementation because method specialization uses `HirExprKind::Var(record.backend_symbol)`.

Run: `cargo test -p rock-lib process_reused_generic_method_call_keeps_instance_target -- --exact`

Expected: FAIL before implementation for the same reason.

- [ ] **Step 3: Rewrite standalone generic methods to instance targets**

In `lib/src/mono/methods.rs`, in `monomorphize_standalone_method_call`, replace the reused-instance callee construction:

```rust
kind: HirExprKind::Var(record.backend_symbol.clone()),
```

with this resolved target:

```rust
kind: HirExprKind::ResolvedVar(HirVarRef {
    name: record.source_name.clone(),
    target: HirVarTarget::Instance(instance_id),
}),
```

In the newly-created instance path, use the `instance_id` returned by `intern` and build the callee with `HirVarTarget::Instance(instance_id)`:

```rust
let callee = self.instance_callable_expr(
    method
        .qualified_name
        .clone()
        .unwrap_or_else(|| format!("{}::{}", type_name, method_name)),
    instance_id,
    Type::Function(param_types, Box::new(concrete_ret_ty.clone())),
    expr.span.clone(),
);
expr.kind = HirExprKind::Call(Box::new(callee), call_args);
```

- [ ] **Step 4: Rewrite trait generic methods to instance targets**

In `lib/src/mono/methods.rs`, in `monomorphize_trait_method_call`, replace the reused-instance callee construction with this target:

```rust
kind: HirExprKind::ResolvedVar(HirVarRef {
    name: record.source_name.clone(),
    target: HirVarTarget::Instance(instance_id),
}),
```

In the newly-created instance path, change `let _instance_id =` to `let instance_id =` and build the direct call callee with `self.instance_callable_expr`:

```rust
let callee = self.instance_callable_expr(
    method
        .qualified_name
        .clone()
        .unwrap_or_else(|| format!("{}::{}", type_name, method_name)),
    instance_id,
    Type::Function(param_types, Box::new(specialized_ret_ty.clone())),
    expr.span.clone(),
);
expr.kind = HirExprKind::Call(Box::new(callee), call_args);
```

- [ ] **Step 5: Rewrite static impl method specialization return values**

In `lib/src/mono/methods.rs`, change `monomorphize_static_impl_method_call` from:

```rust
) -> Option<(String, Type, Type)> {
```

to:

```rust
) -> Option<(crate::mono::InstanceId, Type, Type)> {
```

In the reused path, keep the `instance_id` and body together:

```rust
let (instance_id, body) = if let Some(instance_id) = self.instances.get(&instance_key) {
    let body = self
        .instances
        .record(instance_id)
        .and_then(|record| record.body.clone())?;
    (instance_id, body)
} else {
    let specialized_func = self.specialize_impl_method(
        &method,
        &type_args,
        &imp.type_generics,
        &specialized_name,
        &imp.type_name,
        imp.id,
    );
    let instance_id = self.instances.intern(instance_key, |id| crate::mono::InstanceRecord {
        id,
        origin: origin.clone(),
        substitution: type_args.clone(),
        source_name: method
            .qualified_name
            .clone()
            .unwrap_or_else(|| format!("{}::{}", imp.type_name, method_name)),
        backend_symbol: specialized_name.clone(),
        declared: None,
        body: Some(specialized_func.clone()),
        provided_by_object: false,
        is_specialization: true,
    });
    (instance_id, specialized_func)
};
```

Return the instance ID:

```rust
Some((
    instance_id,
    Type::Function(param_types, Box::new(ret_type.clone())),
    ret_type,
))
```

In `lib/src/mono/process.rs`, update the static impl callee rewrite to use `self.instance_callable_expr(name.clone(), instance_id, specialized_func_type, func.span.clone())`.

- [ ] **Step 6: Run focused method tests**

Run: `cargo test -p rock-lib process_selected_generic_method_call_uses_instance_target -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib process_reused_generic_method_call_keeps_instance_target -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_generic_function_argument_in_monomorphized_method_call -- --exact`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add lib/src/mono/methods.rs lib/src/mono/process.rs
git commit -m "target generic method calls by instance"
```

---

### Task 4: Codegen Resolves Instance Targets Through Records

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/expr/call.rs`

- [ ] **Step 1: Write failing codegen tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `lib/src/codegen/expr/mod.rs`:

```rust
#[test]
fn direct_instance_call_uses_registered_backend_symbol() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let function_id = DefId::new(CrateId(0), LocalDefId(940));
    let instance_id = InstanceId(0);
    let declared = test_function(function_id, "identity");
    codegen.register_instances(&BTreeMap::from([(
        instance_id,
        InstanceRecord {
            id: instance_id,
            origin: InstanceOrigin::Function(function_id),
            substitution: vec![Type::I64],
            source_name: "identity".to_string(),
            backend_symbol: "identity_mono_0".to_string(),
            declared: None,
            body: Some(declared),
            provided_by_object: false,
            is_specialization: true,
        },
    )]));
    let wrapper = codegen
        .module
        .add_function("test_fn", context.void_type().fn_type(&[], false), None);
    let entry = context.append_basic_block(wrapper, "entry");
    codegen.current_function = Some(wrapper);
    codegen.builder.position_at_end(entry);
    let call = expr(
        HirExprKind::Call(
            Box::new(expr(
                HirExprKind::ResolvedVar(HirVarRef {
                    name: "identity".to_string(),
                    target: HirVarTarget::Instance(instance_id),
                }),
                Type::Function(Vec::new(), Box::new(Type::I64)),
            )),
            Vec::new(),
        ),
        Type::I64,
    );

    assert!(codegen.compile_expr(&call).is_ok());
}

#[test]
fn instance_callable_materializes_registered_backend_symbol() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let function_id = DefId::new(CrateId(0), LocalDefId(941));
    let instance_id = InstanceId(0);
    let declared = test_function(function_id, "identity");
    codegen.register_instances(&BTreeMap::from([(
        instance_id,
        InstanceRecord {
            id: instance_id,
            origin: InstanceOrigin::Function(function_id),
            substitution: vec![Type::I64],
            source_name: "identity".to_string(),
            backend_symbol: "identity_mono_0".to_string(),
            declared: None,
            body: Some(declared),
            provided_by_object: false,
            is_specialization: true,
        },
    )]));
    let wrapper = codegen
        .module
        .add_function("test_fn", context.void_type().fn_type(&[], false), None);
    let entry = context.append_basic_block(wrapper, "entry");
    codegen.current_function = Some(wrapper);
    codegen.builder.position_at_end(entry);
    let callable = expr(
        HirExprKind::ResolvedVar(HirVarRef {
            name: "identity".to_string(),
            target: HirVarTarget::Instance(instance_id),
        }),
        Type::Function(Vec::new(), Box::new(Type::I64)),
    );

    let value = codegen.compile_expr(&callable).unwrap();

    assert!(value.is_some());
}

#[test]
fn missing_instance_callable_target_is_codegen_error() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let function = codegen
        .module
        .add_function("test_fn", context.void_type().fn_type(&[], false), None);
    let entry = context.append_basic_block(function, "entry");
    codegen.current_function = Some(function);
    codegen.builder.position_at_end(entry);
    let callable = expr(
        HirExprKind::ResolvedVar(HirVarRef {
            name: "identity".to_string(),
            target: HirVarTarget::Instance(InstanceId(77)),
        }),
        Type::Function(Vec::new(), Box::new(Type::I64)),
    );

    let err = codegen.compile_expr(&callable).unwrap_err();

    assert!(
        err.message.contains("Unknown instance target"),
        "unexpected error: {err}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib direct_instance_call_uses_registered_backend_symbol -- --exact`

Expected: FAIL before implementation because codegen does not resolve `HirVarTarget::Instance`.

Run: `cargo test -p rock-lib instance_callable_materializes_registered_backend_symbol -- --exact`

Expected: FAIL before implementation because callable materialization does not resolve `HirVarTarget::Instance`.

Run: `cargo test -p rock-lib missing_instance_callable_target_is_codegen_error -- --exact`

Expected: FAIL before implementation because there is no explicit missing-instance error.

- [ ] **Step 3: Store instance backend symbols during registration**

In `lib/src/codegen/mod.rs`, add this field to `CodeGen` near `extern_symbols_by_id`:

```rust
instance_symbols_by_id: HashMap<InstanceId, String>,
```

Initialize it in `CodeGen::new`:

```rust
instance_symbols_by_id: HashMap::new(),
```

At the start of the `for record in instances.values()` loop in `register_instances`, insert:

```rust
self.instance_symbols_by_id
    .insert(record.id, record.backend_symbol.clone());
```

- [ ] **Step 4: Add a shared codegen resolver for resolved-var references**

In `lib/src/codegen/mod.rs`, add this method to `impl<'ctx> CodeGen<'ctx>`:

```rust
fn callable_symbol_for_reference(
    &self,
    reference: &HirVarRef,
) -> Result<String, CodegenError> {
    match reference.target {
        HirVarTarget::Function(id) => Ok(self
            .function_symbols_by_id
            .get(&id)
            .cloned()
            .unwrap_or_else(|| reference.name.clone())),
        HirVarTarget::Extern(id) => Ok(self
            .extern_symbols_by_id
            .get(&id)
            .cloned()
            .unwrap_or_else(|| reference.name.clone())),
        HirVarTarget::Instance(id) => self
            .instance_symbols_by_id
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                CodegenError::from(format!(
                    "Unknown instance target {:?} for callable '{}'",
                    id, reference.name
                ))
            }),
    }
}
```

- [ ] **Step 5: Use the resolver in callable materialization**

In `lib/src/codegen/expr/mod.rs`, replace the `HirExprKind::ResolvedVar(reference)` symbol match with:

```rust
let symbol = self.callable_symbol_for_reference(reference)?;
if self.functions.contains_key(&symbol) {
    return self.materialize_named_callable(&symbol, &expr.ty);
}

Err(CodegenError::with_span(
    format!("Unknown variable: {}", reference.name),
    expr.span.clone(),
))
```

In `collect_index_trait_targets_expr`, keep `HirExprKind::ResolvedVar(_)` as a leaf, because instance targets do not contain nested expressions.

- [ ] **Step 6: Use the resolver in direct calls**

In `lib/src/codegen/expr/call.rs`, replace the `func_name` initialization with:

```rust
let func_name = match &func_expr.kind {
    HirExprKind::Var(name) => Some(name.clone()),
    HirExprKind::ResolvedVar(reference) => Some(self.callable_symbol_for_reference(reference)?),
    _ => None,
};
```

This must return an error immediately for a missing instance target. Do not continue to the indirect-call fallback when `HirVarTarget::Instance` is present and cannot be resolved.

- [ ] **Step 7: Run focused codegen tests**

Run: `cargo test -p rock-lib direct_instance_call_uses_registered_backend_symbol -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib instance_callable_materializes_registered_backend_symbol -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib missing_instance_callable_target_is_codegen_error -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib selected_method_target_uses_exact_instance_backend_symbol -- --exact`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/expr/mod.rs lib/src/codegen/expr/call.rs
git commit -m "resolve instance call targets in codegen"
```

---

### Task 5: Integration Verification And Cleanup

**Files:**
- Modify: `lib/tests/integration.rs`
- Format may touch previously edited Rust files from Tasks 1-4: `lib/src/hir/mod.rs`, `lib/src/products.rs`, `lib/src/mir/builder/expr.rs`, `lib/src/mono/registry.rs`, `lib/src/mono/mod.rs`, `lib/src/mono/specialize.rs`, `lib/src/mono/process.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/external.rs`, `lib/src/codegen/mod.rs`, `lib/src/codegen/expr/mod.rs`, `lib/src/codegen/expr/call.rs`

- [ ] **Step 1: Add a compact integration regression**

Add this test near `test_generic_function_argument_in_monomorphized_method_call` in `lib/tests/integration.rs`:

```rust
#[test]
fn test_generic_instance_call_edges_for_function_and_method() {
    let output = compile_and_run(
        r#"
identity = x -> x

struct Holder T
    value: T

impl Holder T
    new = value ->
        Holder
            value: value

    @apply = f -> f self.value

main = ->
    h = Holder::new 42
    (h.apply identity).println!
    0
"#,
    );

    assert_eq!(output.trim(), "42");
}
```

- [ ] **Step 2: Run the new integration test**

Run: `cargo test -p rock-lib --test integration test_generic_instance_call_edges_for_function_and_method -- --exact`

Expected: PASS.

- [ ] **Step 3: Run focused existing regressions**

Run: `cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_generic_function_argument_app_ir_does_not_define_stdlib_object_symbols -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib selected_method_target_uses_exact_instance_backend_symbol -- --exact`

Expected: PASS.

- [ ] **Step 4: Format and check whitespace**

Run: `cargo fmt --all --check`

Expected: PASS. If this fails, run `cargo fmt --all`, then rerun `cargo fmt --all --check` and expect PASS.

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run: `cargo test -p rock-lib > /tmp/rock-lib-task15-final.log 2>&1`

Expected: PASS. The log should contain all `rock-lib` unit, integration, parser, and doc test phases with `0 failed`.

- [ ] **Step 6: Commit**

```bash
git add lib/tests/integration.rs lib/src
git commit -m "verify instance call edge regressions"
```

---

## Final Verification

Run these commands after all task commits:

```bash
cargo fmt --all --check
cargo test -p rock-lib monomorphize_call_returns_instance_id_for_new_specialization -- --exact
cargo test -p rock-lib process_generic_function_call_uses_instance_target -- --exact
cargo test -p rock-lib process_selected_generic_method_call_uses_instance_target -- --exact
cargo test -p rock-lib direct_instance_call_uses_registered_backend_symbol -- --exact
cargo test -p rock-lib instance_callable_materializes_registered_backend_symbol -- --exact
cargo test -p rock-lib missing_instance_callable_target_is_codegen_error -- --exact
cargo test -p rock-lib --test integration test_generic_instance_call_edges_for_function_and_method -- --exact
cargo test -p rock-lib > /tmp/rock-lib-task15-final.log 2>&1
git diff --check
```

Expected final state:
- All listed focused tests pass.
- `/tmp/rock-lib-task15-final.log` shows `0 failed` for all `rock-lib` phases.
- `git diff --check` reports no whitespace errors.
- No monomorphized generic function or method call introduced by Task 15 is represented as `HirExprKind::Var(record.backend_symbol)`.
- Backend symbols remain present only as `InstanceRecord.backend_symbol`, codegen declaration keys, LLVM function names, product object symbols, diagnostics, or compatibility aliases.
