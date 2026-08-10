# Instance Registry Callable Universe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Roadmap Task 14 by making `MonomorphizedProgram.instances` the complete backend callable universe for codegen declarations and body emission.

**Architecture:** Mono registers every emitted callable as an `InstanceRecord`: concrete current-crate functions, concrete impl methods, concrete trait defaults, generic specializations, and object-backed declarations. Codegen keeps using `HirProgram` for metadata, externs, trait lookup, and compatibility tables, but declares and compiles callable bodies only by iterating instance records. Backend-symbol call targets remain a temporary Task 15 compatibility path.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId` indexes, `InstanceRegistry`, `MonomorphizedProgram`, LLVM codegen, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## File Structure

- Modify: `lib/src/mono/registry.rs`
  - Add `InstanceOrigin::TraitDefault { trait_id, method }`.
  - Add registry unit coverage for trait-default identity and zero-substitution reuse.
- Modify: `lib/src/hir/mod.rs`
  - Add a shared HIR concreteness predicate used before registering callable instances.
- Modify: `lib/src/mono/mod.rs`
  - Add helper methods for concrete function, impl method, and trait default instance registration.
- Modify: `lib/src/mono/process.rs`
  - Register current-crate concrete functions and impl methods during non-crate mono processing.
- Modify: `lib/src/mono/external.rs`
  - Register current-crate concrete functions, impl methods, trait defaults, and object-backed declarations in crate-aware mono processing.
  - Add focused mono tests for concrete function, impl method, and trait-default records.
- Modify: `lib/src/codegen/mod.rs`
  - Remove direct callable declaration and compilation loops over HIR functions and impl methods.
  - Compile non-object instance bodies and error on malformed non-object records.
  - Add codegen tests proving HIR maps are not callable emission sources.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Mark Roadmap Task 14 complete.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Update Monomorphization Instances evidence and remaining Task 15-16 gaps.

## Task 1: Add Trait Default Instance Identity

**Files:**
- Modify: `lib/src/mono/registry.rs`

- [ ] **Step 1: Write failing trait-default registry tests**

In `lib/src/mono/registry.rs`, inside `#[cfg(test)] mod tests`, add this helper after `impl_method_record`:

```rust
    fn trait_default_key(trait_id: DefId, method: DefId) -> InstanceKey {
        InstanceKey::new(
            InstanceOrigin::TraitDefault { trait_id, method },
            Vec::new(),
        )
    }
```

Then add these tests after `instance_registry_ignores_backend_symbol_for_identity`:

```rust
    #[test]
    fn instance_registry_reuses_zero_substitution_function_instance() {
        let mut registry = InstanceRegistry::new();
        let function_id = DefId::new(CrateId(0), LocalDefId(10));
        let key = InstanceKey::new(InstanceOrigin::Function(function_id), Vec::new());

        let first = registry.intern(key.clone(), |id| InstanceRecord {
            id,
            origin: key.origin.clone(),
            substitution: key.substitution.clone(),
            source_name: "main".to_string(),
            backend_symbol: "main".to_string(),
            declared: None,
            body: None,
            provided_by_object: false,
        });

        let second = registry.intern(key, |_| {
            panic!("zero-substitution function should reuse existing instance id");
        });

        assert_eq!(first, second);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn instance_registry_distinguishes_trait_default_methods_by_trait_and_method_id() {
        let mut registry = InstanceRegistry::new();
        let show_trait = DefId::new(CrateId(0), LocalDefId(20));
        let debug_trait = DefId::new(CrateId(0), LocalDefId(21));
        let fmt_method = DefId::new(CrateId(0), LocalDefId(22));
        let display_method = DefId::new(CrateId(0), LocalDefId(23));

        let show_fmt = trait_default_key(show_trait, fmt_method);
        let debug_fmt = trait_default_key(debug_trait, fmt_method);
        let show_display = trait_default_key(show_trait, display_method);

        let show_fmt_id = registry.intern(show_fmt.clone(), |id| {
            impl_method_record(id, &show_fmt, "Show::fmt", "Show_fmt")
        });
        let debug_fmt_id = registry.intern(debug_fmt.clone(), |id| {
            impl_method_record(id, &debug_fmt, "Debug::fmt", "Debug_fmt")
        });
        let show_display_id = registry.intern(show_display.clone(), |id| {
            impl_method_record(id, &show_display, "Show::display", "Show_display")
        });

        assert_ne!(show_fmt_id, debug_fmt_id);
        assert_ne!(show_fmt_id, show_display_id);
        assert_eq!(registry.len(), 3);
    }
```

- [ ] **Step 2: Run tests to verify the new trait-default test fails**

Run: `cargo test -p rock-lib instance_registry_distinguishes_trait_default_methods_by_trait_and_method_id`

Expected: FAIL to compile with a message like `no variant named TraitDefault found for enum InstanceOrigin`.

- [ ] **Step 3: Add the trait-default origin variant**

In `lib/src/mono/registry.rs`, replace `InstanceOrigin` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstanceOrigin {
    Function(DefId),
    ImplMethod {
        owner: InstanceImplOwner,
        method: DefId,
    },
    TraitDefault {
        trait_id: DefId,
        method: DefId,
    },
}
```

- [ ] **Step 4: Run focused registry tests**

Run these one at a time:

```bash
cargo test -p rock-lib instance_registry_reuses_zero_substitution_function_instance
cargo test -p rock-lib instance_registry_distinguishes_trait_default_methods_by_trait_and_method_id
cargo test -p rock-lib instance_registry_ignores_backend_symbol_for_identity
```

Expected: PASS for each command.

- [ ] **Step 5: Commit Task 1**

Run:

```bash
git add lib/src/mono/registry.rs
git commit -m "add trait default instance identity"
```

## Task 2: Register Concrete Current-Crate Functions As Instances

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add the failing crate-aware mono test**

In `lib/src/mono/external.rs`, inside `#[cfg(test)] mod tests`, add this helper after `println_method`:

```rust
    fn concrete_function(id: DefId, name: &str, ret_type: Type) -> HirFunction {
        HirFunction {
            id,
            name: name.to_string(),
            qualified_name: None,
            generic_params: Vec::new(),
            generic_param_ids: Vec::new(),
            generic_bounds: HashMap::new(),
            params: Vec::new(),
            ret_type: ret_type.clone(),
            body: HirBlock {
                stmts: Vec::new(),
                ty: ret_type,
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }
```

Add this test after `wrapped_process_with_crates`:

```rust
    #[test]
    fn process_with_crates_records_current_crate_concrete_function_instance() {
        let function_id = DefId::new(CrateId(0), LocalDefId(30));
        let function = concrete_function(function_id, "main", Type::I32);
        let program = crate::hir::HirProgram::from_parts(
            HashMap::from([("main".to_string(), function)]),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            vec![],
            vec![],
        );
        let ctx = CrateContext::new();
        let mut mono = Monomorphizer::new();
        mono.resolver
            .item_paths
            .insert("main".to_string(), function_id);

        let output = wrapped_process_with_crates(&mut mono, program, &ctx);

        let record = output
            .instances
            .values()
            .find(|record| record.origin == crate::mono::InstanceOrigin::Function(function_id))
            .expect("current-crate concrete function should be an instance");
        assert_eq!(record.substitution, Vec::<Type>::new());
        assert_eq!(record.source_name, "main");
        assert_eq!(record.backend_symbol, "main");
        assert!(record.body.is_some());
        assert!(!record.provided_by_object);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p rock-lib process_with_crates_records_current_crate_concrete_function_instance`

Expected: FAIL with `current-crate concrete function should be an instance`.

- [ ] **Step 3: Add shared HIR concreteness helpers**

In `lib/src/hir/mod.rs`, after `impl HirProgram`, add:

```rust
pub fn hir_function_is_codegen_concrete(function: &HirFunction) -> bool {
    function
        .params
        .iter()
        .all(|param| hir_type_is_codegen_concrete(&param.ty))
        && hir_type_is_codegen_concrete(&function.ret_type)
        && hir_block_is_codegen_concrete(&function.body)
}

fn hir_type_is_codegen_concrete(ty: &Type) -> bool {
    match ty {
        Type::TypeVar(_) | Type::Generic(_) | Type::Projection { .. } => false,
        Type::Slice(inner) | Type::Pointer(inner) => hir_type_is_codegen_concrete(inner),
        Type::Array(inner, _) => hir_type_is_codegen_concrete(inner),
        Type::Reference { inner, .. } => hir_type_is_codegen_concrete(inner),
        Type::Tuple(elems) => elems.iter().all(hir_type_is_codegen_concrete),
        Type::Function(args, ret) => {
            args.iter().all(hir_type_is_codegen_concrete) && hir_type_is_codegen_concrete(ret)
        }
        Type::Struct { args, .. } | Type::Enum { args, .. } => {
            args.iter().all(hir_type_is_codegen_concrete)
        }
        _ => true,
    }
}

fn hir_block_is_codegen_concrete(block: &HirBlock) -> bool {
    hir_type_is_codegen_concrete(&block.ty)
        && block.stmts.iter().all(hir_stmt_is_codegen_concrete)
}

fn hir_stmt_is_codegen_concrete(stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Let { ty, value, .. } => {
            hir_type_is_codegen_concrete(ty) && hir_expr_is_codegen_concrete(value)
        }
        HirStmt::Expr(expr) | HirStmt::Return(Some(expr)) | HirStmt::Break(Some(expr)) => {
            hir_expr_is_codegen_concrete(expr)
        }
        HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => true,
    }
}

fn hir_expr_is_codegen_concrete(expr: &HirExpr) -> bool {
    if !hir_type_is_codegen_concrete(&expr.ty) {
        return false;
    }

    match &expr.kind {
        HirExprKind::IntLiteral(_)
        | HirExprKind::FloatLiteral(_)
        | HirExprKind::BoolLiteral(_)
        | HirExprKind::StringLiteral(_)
        | HirExprKind::CharLiteral(_)
        | HirExprKind::Unit
        | HirExprKind::Var(_)
        | HirExprKind::ResolvedVar(_) => true,
        HirExprKind::ArrayLiteral(elems) | HirExprKind::TupleLiteral(elems) => {
            elems.iter().all(hir_expr_is_codegen_concrete)
        }
        HirExprKind::StructLiteral(_, _, fields) => fields
            .iter()
            .all(|field| hir_expr_is_codegen_concrete(&field.value)),
        HirExprKind::EnumVariant(_, _, args, _) | HirExprKind::Intrinsic { args, .. } => {
            args.iter().all(hir_expr_is_codegen_concrete)
        }
        HirExprKind::FieldAccess(base, _, _)
        | HirExprKind::TupleIndex(base, _)
        | HirExprKind::Deref(base)
        | HirExprKind::Ref(_, base)
        | HirExprKind::UnaryOp(_, base)
        | HirExprKind::Cast(base, _) => hir_expr_is_codegen_concrete(base),
        HirExprKind::Index(base, index)
        | HirExprKind::BinOp(_, base, index)
        | HirExprKind::Range(base, index)
        | HirExprKind::Assign(base, index) => {
            hir_expr_is_codegen_concrete(base) && hir_expr_is_codegen_concrete(index)
        }
        HirExprKind::Call(func, args) => {
            hir_expr_is_codegen_concrete(func) && args.iter().all(hir_expr_is_codegen_concrete)
        }
        HirExprKind::MethodCall(recv, _, args, _, _) => {
            hir_expr_is_codegen_concrete(recv) && args.iter().all(hir_expr_is_codegen_concrete)
        }
        HirExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            hir_expr_is_codegen_concrete(condition)
                && hir_block_is_codegen_concrete(then_branch)
                && else_branch
                    .as_ref()
                    .is_none_or(|block| hir_block_is_codegen_concrete(block))
        }
        HirExprKind::Match { scrutinee, arms } => {
            hir_expr_is_codegen_concrete(scrutinee)
                && arms.iter().all(|arm| {
                    hir_pattern_is_codegen_concrete(&arm.pattern)
                        && arm.guard.as_ref().is_none_or(hir_expr_is_codegen_concrete)
                        && hir_block_is_codegen_concrete(&arm.body)
                })
        }
        HirExprKind::While { condition, body } => {
            hir_expr_is_codegen_concrete(condition) && hir_block_is_codegen_concrete(body)
        }
        HirExprKind::For { iter, body, .. } => {
            hir_expr_is_codegen_concrete(iter) && hir_block_is_codegen_concrete(body)
        }
        HirExprKind::Loop(body) | HirExprKind::Block(body) => hir_block_is_codegen_concrete(body),
        HirExprKind::Lambda {
            params,
            body,
            captures,
        } => {
            params.iter().all(|param| hir_type_is_codegen_concrete(&param.ty))
                && captures
                    .iter()
                    .all(|capture| hir_type_is_codegen_concrete(&capture.ty))
                && hir_block_is_codegen_concrete(body)
        }
    }
}

fn hir_pattern_is_codegen_concrete(pattern: &HirPattern) -> bool {
    match pattern {
        HirPattern::Wildcard | HirPattern::Binding(_, _) | HirPattern::Literal(_) => true,
        HirPattern::Tuple(patterns) | HirPattern::Or(patterns) => {
            patterns.iter().all(hir_pattern_is_codegen_concrete)
        }
        HirPattern::Struct(_, _, type_args, fields) => {
            type_args.iter().all(hir_type_is_codegen_concrete)
                && fields
                    .iter()
                    .all(|field| hir_pattern_is_codegen_concrete(&field.pattern))
        }
        HirPattern::Enum(_, _, _, patterns) => {
            patterns.iter().all(hir_pattern_is_codegen_concrete)
        }
    }
}
```

- [ ] **Step 4: Add function instance registration helpers**

In `lib/src/mono/mod.rs`, extend the `use` block from:

```rust
use crate::hir::*;
```

to:

```rust
use crate::hir::{self, *};
```

Then add these methods inside `impl Monomorphizer`, after `method_instance_origin`:

```rust
    fn register_function_instance(
        &mut self,
        source_name: &str,
        backend_symbol: String,
        func: &HirFunction,
    ) -> InstanceId {
        let origin = self.function_instance_origin(source_name, func);
        let key = InstanceKey::new(origin.clone(), Vec::new());
        let body = func.clone();

        self.instances.intern(key, |id| InstanceRecord {
            id,
            origin,
            substitution: Vec::new(),
            source_name: source_name.to_string(),
            backend_symbol,
            declared: None,
            body: Some(body),
            provided_by_object: false,
        })
    }

    fn should_register_function_instance(func: &HirFunction) -> bool {
        func.generic_params.is_empty() && hir::hir_function_is_codegen_concrete(func)
    }
```

- [ ] **Step 5: Register concrete functions in non-crate mono processing**

In `lib/src/mono/process.rs`, replace the final `program.functions` rebuild loop:

```rust
        for (name, func) in std::mem::take(&mut self.concrete_functions) {
            program.names.functions_by_name.insert(name, func.id);
            program.functions.insert(func.id, func);
        }
```

with:

```rust
        for (name, func) in std::mem::take(&mut self.concrete_functions) {
            if Self::should_register_function_instance(&func) {
                self.register_function_instance(&name, name.clone(), &func);
            }
            program.names.functions_by_name.insert(name, func.id);
            program.functions.insert(func.id, func);
        }
```

- [ ] **Step 6: Register concrete functions in crate-aware mono processing**

In `lib/src/mono/external.rs`, replace the final `program.functions` rebuild loop:

```rust
        for (name, func) in std::mem::take(&mut self.concrete_functions) {
            program.names.functions_by_name.insert(name, func.id);
            program.functions.insert(func.id, func);
        }
```

with:

```rust
        for (name, func) in std::mem::take(&mut self.concrete_functions) {
            if Self::should_register_function_instance(&func) {
                self.register_function_instance(&name, name.clone(), &func);
            }
            program.names.functions_by_name.insert(name, func.id);
            program.functions.insert(func.id, func);
        }
```

- [ ] **Step 7: Run focused mono function tests**

Run these one at a time:

```bash
cargo test -p rock-lib process_with_crates_records_current_crate_concrete_function_instance
cargo test -p rock-lib instance_registry_reuses_zero_substitution_function_instance
```

Expected: PASS for each command.

- [ ] **Step 8: Commit Task 2**

Run:

```bash
git add lib/src/hir/mod.rs lib/src/mono/mod.rs lib/src/mono/process.rs lib/src/mono/external.rs
git commit -m "register concrete function instances"
```

## Task 3: Register Concrete Impl Methods And Trait Defaults

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add failing impl-method and trait-default mono tests**

In `lib/src/mono/external.rs`, inside `#[cfg(test)] mod tests`, add these tests after `process_with_crates_records_current_crate_concrete_function_instance`:

```rust
    #[test]
    fn process_with_crates_records_current_crate_concrete_impl_method_instance() {
        let impl_id = DefId::new(CrateId(0), LocalDefId(40));
        let method_id = DefId::new(CrateId(0), LocalDefId(41));
        let mut method = println_method(Type::I64);
        method.id = method_id;
        method.qualified_name = Some("Box_println".to_string());
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Box".to_string()),
            type_name: "Box".to_string(),
            type_generics: vec![],
            receiver_arg_types: vec![],
            trait_name: None,
            trait_id: None,
            trait_generics: vec![],
            trait_arg_types: vec![],
            associated_types: vec![],
            bounds: vec![],
            methods: HashMap::from([("println".to_string(), method)]),
        };
        let program = crate::hir::HirProgram::from_parts(
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            vec![imp],
            vec![],
        );
        let ctx = CrateContext::new();
        let mut mono = Monomorphizer::new();

        let output = wrapped_process_with_crates(&mut mono, program, &ctx);

        let record = output
            .instances
            .values()
            .find(|record| {
                record.origin
                    == crate::mono::InstanceOrigin::ImplMethod {
                        owner: crate::mono::InstanceImplOwner::Named(impl_id),
                        method: method_id,
                    }
            })
            .expect("current-crate concrete impl method should be an instance");
        assert_eq!(record.substitution, Vec::<Type>::new());
        assert_eq!(record.source_name, "Box::println");
        assert_eq!(record.backend_symbol, "Box_println");
        assert!(record.body.is_some());
        assert!(!record.provided_by_object);
    }

    #[test]
    fn process_with_crates_records_concrete_trait_default_instance() {
        let trait_id = DefId::new(CrateId(0), LocalDefId(50));
        let method_id = DefId::new(CrateId(0), LocalDefId(51));
        let default_method = concrete_function(method_id, "default_value", Type::I32);
        let trait_def = crate::hir::HirTrait {
            id: trait_id,
            name: "Provider".to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods: HashMap::from([("default_value".to_string(), default_method)]),
            signatures: HashMap::new(),
        };
        let program = crate::hir::HirProgram::from_parts(
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::from([("Provider".to_string(), trait_def)]),
            vec![],
            vec![],
        );
        let ctx = CrateContext::new();
        let mut mono = Monomorphizer::new();

        let output = wrapped_process_with_crates(&mut mono, program, &ctx);

        let record = output
            .instances
            .values()
            .find(|record| {
                record.origin
                    == crate::mono::InstanceOrigin::TraitDefault {
                        trait_id,
                        method: method_id,
                    }
            })
            .expect("concrete trait default should be an instance");
        assert_eq!(record.substitution, Vec::<Type>::new());
        assert_eq!(record.source_name, "Provider::default_value");
        assert_eq!(record.backend_symbol, "Provider_default_value");
        assert!(record.body.is_some());
        assert!(!record.provided_by_object);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run these one at a time:

```bash
cargo test -p rock-lib process_with_crates_records_current_crate_concrete_impl_method_instance
cargo test -p rock-lib process_with_crates_records_concrete_trait_default_instance
```

Expected: FAIL with the matching `should be an instance` assertion for each command.

- [ ] **Step 3: Add impl-method and trait-default registration helpers**

In `lib/src/mono/mod.rs`, inside `impl Monomorphizer`, add these methods after `should_register_function_instance`:

```rust
    fn impl_method_backend_symbol(
        imp: &HirImpl,
        method_name: &str,
        method: &HirFunction,
    ) -> String {
        method
            .qualified_name
            .clone()
            .unwrap_or_else(|| format!("{}_{}", imp.type_name, method_name))
    }

    fn register_impl_method_instance(
        &mut self,
        imp: &HirImpl,
        method_name: &str,
        method: &HirFunction,
    ) -> InstanceId {
        let origin = self.method_instance_origin(imp, method);
        let key = InstanceKey::new(origin.clone(), Vec::new());
        let backend_symbol = Self::impl_method_backend_symbol(imp, method_name, method);
        let body = method.clone();

        self.instances.intern(key, |id| InstanceRecord {
            id,
            origin,
            substitution: Vec::new(),
            source_name: format!("{}::{}", imp.type_name, method_name),
            backend_symbol,
            declared: None,
            body: Some(body),
            provided_by_object: false,
        })
    }

    fn should_register_method_instance(method: &HirFunction) -> bool {
        method.generic_params.is_empty() && hir::hir_function_is_codegen_concrete(method)
    }

    fn trait_default_backend_symbol(
        trait_name: &str,
        method_name: &str,
        method: &HirFunction,
    ) -> String {
        method
            .qualified_name
            .clone()
            .unwrap_or_else(|| format!("{}_{}", trait_name, method_name))
    }

    fn register_trait_default_instance(
        &mut self,
        trait_id: DefId,
        trait_name: &str,
        method_name: &str,
        method: &HirFunction,
    ) -> InstanceId {
        let origin = InstanceOrigin::TraitDefault {
            trait_id,
            method: method.id,
        };
        let key = InstanceKey::new(origin.clone(), Vec::new());
        let backend_symbol = Self::trait_default_backend_symbol(trait_name, method_name, method);
        let body = method.clone();

        self.instances.intern(key, |id| InstanceRecord {
            id,
            origin,
            substitution: Vec::new(),
            source_name: format!("{}::{}", trait_name, method_name),
            backend_symbol,
            declared: None,
            body: Some(body),
            provided_by_object: false,
        })
    }

    fn register_concrete_trait_defaults(&mut self, program: &HirProgram) {
        for (trait_id, trait_name, trait_def) in program.traits_by_id() {
            for (method_name, method) in &trait_def.methods {
                if Self::should_register_method_instance(method) {
                    self.register_trait_default_instance(
                        trait_id,
                        trait_name,
                        method_name,
                        method,
                    );
                }
            }
        }
    }
```

- [ ] **Step 4: Register concrete impl methods in non-crate mono processing**

In `lib/src/mono/process.rs`, inside the `for imp in program.impls.values_mut()` loop, replace:

```rust
                    let processed = self.process_function(func);
                    self.current_impl_type = None;
                    imp.methods.insert(name, processed);
```

with:

```rust
                    let processed = self.process_function(func);
                    self.current_impl_type = None;
                    if Self::should_register_method_instance(&processed) {
                        self.register_impl_method_instance(imp, &name, &processed);
                    }
                    imp.methods.insert(name, processed);
```

Then add this call after the impl-method processing loop and before the final `program.functions.clear()`:

```rust
        self.register_concrete_trait_defaults(&program);
```

- [ ] **Step 5: Register concrete impl methods in crate-aware mono processing**

In `lib/src/mono/external.rs`, inside the `for imp in program.impls.values_mut()` loop, replace:

```rust
                    let processed = self.process_function(func);
                    self.current_impl_type = None;
                    imp.methods.insert(name, processed);
```

with:

```rust
                    let processed = self.process_function(func);
                    self.current_impl_type = None;
                    if Self::should_register_method_instance(&processed) {
                        self.register_impl_method_instance(imp, &name, &processed);
                    }
                    imp.methods.insert(name, processed);
```

Then add this call after the impl-method processing loop and before `program.functions.clear()`:

```rust
        self.register_concrete_trait_defaults(program);
```

- [ ] **Step 6: Run focused mono instance tests**

Run these one at a time:

```bash
cargo test -p rock-lib process_with_crates_records_current_crate_concrete_impl_method_instance
cargo test -p rock-lib process_with_crates_records_concrete_trait_default_instance
cargo test -p rock-lib process_with_crates_records_object_backed_instances_without_re_emitting
cargo test -p rock-lib process_with_crates_uses_artifact_backend_symbols_for_object_backed_methods
```

Expected: PASS for each command.

- [ ] **Step 7: Commit Task 3**

Run:

```bash
git add lib/src/mono/mod.rs lib/src/mono/process.rs lib/src/mono/external.rs
git commit -m "register concrete method instances"
```

## Task 4: Make Codegen Emit Callable Bodies Only From Instances

**Files:**
- Modify: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Add failing codegen emission-source tests**

In `lib/src/codegen/mod.rs`, inside `#[cfg(test)] mod tests`, extend the mono import from:

```rust
    use crate::mono::{InstanceId, InstanceOrigin, InstanceRecord};
```

to:

```rust
    use crate::mono::{InstanceId, InstanceOrigin, InstanceRecord, MonomorphizedProgram};
```

Then add these tests after `register_index_trait_targets_collects_instance_bodies`:

```rust
    #[test]
    fn compile_program_does_not_emit_hir_function_without_instance_record() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let program = MonomorphizedProgram {
            program: program_with_expr(expr(HirExprKind::IntLiteral(0), Type::I32)),
            instances: BTreeMap::new(),
        };

        codegen.compile_program(&program).unwrap();

        assert!(codegen.module.get_function("main").is_none());
    }

    #[test]
    fn compile_program_errors_for_non_object_instance_without_body() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let function_id = DefId::new(CrateId(0), LocalDefId(60));
        let program = MonomorphizedProgram {
            program: HirProgram::from_parts(
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                Vec::new(),
                Vec::new(),
            ),
            instances: BTreeMap::from([(
                InstanceId(0),
                InstanceRecord {
                    id: InstanceId(0),
                    origin: InstanceOrigin::Function(function_id),
                    substitution: Vec::new(),
                    source_name: "missing_body".to_string(),
                    backend_symbol: "missing_body".to_string(),
                    declared: None,
                    body: None,
                    provided_by_object: false,
                },
            )]),
        };

        let err = codegen.compile_program(&program).unwrap_err();

        assert!(err
            .message
            .contains("non-object instance 'missing_body' has no body"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run these one at a time:

```bash
cargo test -p rock-lib compile_program_does_not_emit_hir_function_without_instance_record
cargo test -p rock-lib compile_program_errors_for_non_object_instance_without_body
```

Expected: first test FAILS because `main` is still emitted from `program.functions_by_id()`. Second test FAILS because malformed records currently do not return an error.

- [ ] **Step 3: Remove private codegen concreteness helpers**

In `lib/src/codegen/mod.rs`, delete the private method definitions named:

```text
function_is_codegen_concrete
type_is_codegen_concrete
block_is_codegen_concrete
stmt_is_codegen_concrete
expr_is_codegen_concrete
pattern_is_codegen_concrete
```

These helpers were moved to `lib/src/hir/mod.rs` in Task 2. After Task 4, codegen should not need the private functions for direct impl-method emission.

- [ ] **Step 4: Remove direct HIR callable declaration loops**

In `lib/src/codegen/mod.rs`, inside `compile_program`, delete the direct declaration blocks that start with these comments:

```rust
        // Declare all internal functions (forward declarations)
```

and:

```rust
        // Declare impl methods
```

Do not delete `self.register_instances(instances);`, `self.register_trait_member_ids(program);`, `self.register_trait_impls(&impls);`, runtime declaration, or extern declaration setup.

- [ ] **Step 5: Replace callable body compilation with instance-only compilation**

In `lib/src/codegen/mod.rs`, inside `compile_program`, replace the body compilation blocks that start with:

```rust
        // Compile function bodies
```

and continue through the end of the impl-method body loop with this block:

```rust
        // Compile callable bodies from the authoritative instance registry.
        for record in instances.values() {
            if record.provided_by_object {
                continue;
            }

            let Some(body) = &record.body else {
                return Err(CodegenError::from(format!(
                    "non-object instance '{}' has no body",
                    record.backend_symbol
                )));
            };

            self.compile_function(&record.backend_symbol, body)?;
        }
```

- [ ] **Step 6: Run focused codegen tests**

Run these one at a time:

```bash
cargo test -p rock-lib compile_program_does_not_emit_hir_function_without_instance_record
cargo test -p rock-lib compile_program_errors_for_non_object_instance_without_body
cargo test -p rock-lib register_index_trait_targets_collects_instance_bodies
```

Expected: PASS for each command.

- [ ] **Step 7: Run a compiler smoke test through codegen**

Run: `cargo test -p rock-lib --test integration test_hello_world -- --exact`

Expected: PASS.

- [ ] **Step 8: Commit Task 4**

Run:

```bash
git add lib/src/codegen/mod.rs
git commit -m "emit callable bodies from instances"
```

## Task 5: Update Trackers And Run Final Verification

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Update the roadmap rebaseline note**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, add this bullet after the Task 13 rebaseline note:

```markdown
- Roadmap Task 14 has landed for instance-registry callable authority: all emitted callable declarations and bodies now enter codegen through `MonomorphizedProgram.instances`, while backend-symbol call targets remain scoped to Task 15.
```

- [ ] **Step 2: Mark Task 14 complete in the roadmap**

In the Task 14 section of `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, add this status line immediately under the heading:

```markdown
**Status:** Complete in `docs/superpowers/plans/2026-05-20-instance-registry-callable-universe.md`.
```

- [ ] **Step 3: Update the audit summary row**

In `docs/superpowers/plans/master-audit-checklist.md`, update the `Monomorphization Instances` summary row so the main gap cell reads:

```markdown
Backend-symbol/name maps still mediate some calls; remaining work is Task 15 call-edge identity, Task 16 instance reachability, and longer-term MIR/codegen body ownership
```

- [ ] **Step 4: Update Monomorphization Instances evidence and checkboxes**

In `docs/superpowers/plans/master-audit-checklist.md`, under `## 7. Monomorphization Instances`, add these evidence bullets after the existing `lib/src/codegen/mod.rs` evidence:

```markdown
- Concrete current-crate functions, impl methods, trait defaults, generic specializations, and object-backed declarations are represented as `InstanceRecord`s before codegen emission.
- Codegen no longer declares or compiles callable bodies by directly iterating HIR function or impl-method maps.
```

Add these completed checklist entries after the current instance-registry done items:

```markdown
- [x] Made the instance registry the authoritative backend input for emitted callable declarations and bodies.
- [x] Modeled concrete current-crate functions, impl methods, concrete trait defaults, specialized generics, and object-backed declarations as instance records.
- [x] Stopped codegen from using direct HIR function and impl-method loops as callable body emission sources.
```

Remove this remaining-work line from the same section:

```markdown
- [ ] Make the instance registry the authoritative backend input for all emitted callable bodies, not just specialized or object-backed instances.
```

Keep the remaining Task 15 and Task 16 lines about backend-symbol lookup, string-keyed mono maps, explicit call edges, DCE, and long-term MIR/codegen body ownership.

- [ ] **Step 5: Run formatting checks**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 6: Run focused regression tests**

Run these one at a time:

```bash
cargo test -p rock-lib instance_registry_reuses_zero_substitution_function_instance
cargo test -p rock-lib instance_registry_distinguishes_trait_default_methods_by_trait_and_method_id
cargo test -p rock-lib process_with_crates_records_current_crate_concrete_function_instance
cargo test -p rock-lib process_with_crates_records_current_crate_concrete_impl_method_instance
cargo test -p rock-lib process_with_crates_records_concrete_trait_default_instance
cargo test -p rock-lib compile_program_does_not_emit_hir_function_without_instance_record
cargo test -p rock-lib compile_program_errors_for_non_object_instance_without_body
cargo test -p rock-lib --test integration test_hello_world -- --exact
```

Expected: PASS for each command.

- [ ] **Step 7: Run full library tests**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 8: Run final diff hygiene check**

Run: `git diff --check`

Expected: no output.

- [ ] **Step 9: Commit Task 5**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update instance registry audit trackers"
```

## Final Verification Before Completion

- [ ] Run: `git status --short`

Expected: no tracked or untracked changes except intentionally ignored user files.

- [ ] Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] Run: `git diff --check`

Expected: no output.

- [ ] Report the final commit range and verification evidence.
