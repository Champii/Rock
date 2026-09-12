# Task 21 Backend Metadata Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move codegen trait/member/projection support onto explicit backend metadata so the active MIR backend path no longer scans HIR impls, traits, or method aliases to rediscover selected targets.

**Architecture:** Add a focused `codegen::metadata` module that builds immutable backend selection facts from `MonomorphizedProgram` plus executable `MirProgram`. `CodeGen` consumes that metadata for exact selected method symbols, trait member validation, projection normalization, and builtin-index guards while keeping existing layout, extern, product, and link metadata on their current paths. The test-only HIR body codegen path may retain compatibility behavior, but `compile_program_from_mir` must not call HIR trait/member registration helpers.

**Tech Stack:** Rust 2021, `rock-lib`, `CodeGen`, `MonomorphizedProgram`, `MirProgram`, canonical IDs (`DefId`, `InstanceId`, `TypeId`), focused `cargo test -p rock-lib` filters, `cargo fmt --all --check`, `git diff --check`.

---

## Approved Spec

- `docs/superpowers/specs/2026-05-31-task-21-backend-metadata-cleanup-design.md`

## File Structure

- Create `lib/src/codegen/metadata.rs`: owns `BackendSelectionMetadata`, projection impl metadata, metadata builder, builtin-index selected-target collection, and unit tests for metadata-only behavior.
- Modify `lib/src/codegen/mod.rs`: register the new module, add `backend_metadata` to `CodeGen`, build/apply metadata in the MIR declaration path, and resolve MIR selected methods through metadata.
- Modify `lib/src/codegen/types.rs`: route `ProjectionProvider` through backend metadata instead of `CodeGen::find_trait_impl`; update projection tests to inject metadata.
- Modify `lib/src/codegen/expr/mod.rs`: keep test-only HIR expression compatibility on backend metadata and remove direct `trait_impls`/`trait_member_ids` dependencies from expression selected-target tests.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: mark the specific Task 21 direct codegen trait/member metadata cleanup item complete after implementation and verification.
- Do not edit `/root/new_lang2/.sisyphus/`.

## Task 1: Add Backend Selection Metadata Builder

**Files:**
- Create: `lib/src/codegen/metadata.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/metadata.rs`

- [ ] **Step 1: Write red metadata builder tests with a compiling skeleton**

Create `lib/src/codegen/metadata.rs` with this skeleton and tests:

```rust
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::hir::{HirBlock, HirExpr, HirExprKind, HirFunction, HirImpl, HirProgram, HirStmt};
use crate::ids::DefId;
use crate::mir::MirProgram;
use crate::mono::{InstanceImplOwner, InstanceOrigin, InstanceRecord, MonomorphizedProgram};
use crate::types::Type;

#[derive(Debug, Clone)]
pub(crate) struct ProjectionImplMetadata {
    pub(crate) imp: HirImpl,
}

impl ProjectionImplMetadata {
    pub(crate) fn from_impl(imp: HirImpl) -> Self {
        Self { imp }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct BackendSelectionMetadata {
    pub(crate) impl_method_backend_symbols: HashMap<(DefId, DefId), String>,
    pub(crate) trait_member_ids: HashMap<(DefId, String), DefId>,
    pub(crate) projection_impls: Vec<ProjectionImplMetadata>,
    pub(crate) builtin_index_trait_ids: HashSet<DefId>,
}

impl BackendSelectionMetadata {
    pub(crate) fn from_program(
        _program: &MonomorphizedProgram,
        _mir: &MirProgram,
    ) -> Self {
        Self::default()
    }

    pub(crate) fn record_instance(&mut self, _record: &InstanceRecord) {}

    pub(crate) fn impl_method_backend_symbol(
        &self,
        impl_id: DefId,
        method_id: DefId,
    ) -> Option<&str> {
        self.impl_method_backend_symbols
            .get(&(impl_id, method_id))
            .map(String::as_str)
    }

    pub(crate) fn trait_member_matches_target(
        &self,
        trait_id: DefId,
        member_name: &str,
        method_id: DefId,
    ) -> bool {
        self.trait_member_ids
            .get(&(trait_id, member_name.to_string()))
            .is_none_or(|selected_id| *selected_id == method_id)
    }

    pub(crate) fn projection_impl(
        &self,
        _lookup_type_names: &[String],
        _recv_ty: &Type,
        _trait_id: DefId,
        _trait_args: &[Type],
    ) -> Option<&HirImpl> {
        None
    }

    pub(crate) fn is_builtin_index_trait(&self, trait_id: DefId) -> bool {
        self.builtin_index_trait_ids.contains(&trait_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hir::{HirAssociatedTypeDef, HirFunctionSig, HirImplOwner, HirTrait};
    use crate::ids::{AssocTypeId, CrateId, HirLocalId, InstanceId, LocalDefId};
    use crate::mir::{BasicBlock, LocalDecl, MirFunction, MirFunctionId, Mutability};
    use crate::type_context::TypeContext;
    use crate::types::{AssociatedTypeKey, GenericParamId};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    fn empty_program() -> HirProgram {
        HirProgram::from_parts(
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    fn empty_mir() -> MirProgram {
        MirProgram {
            functions: BTreeMap::new(),
            type_context: TypeContext::new(),
        }
    }

    fn function(id: DefId, name: &str, body: HirExpr) -> HirFunction {
        HirFunction {
            id,
            name: name.to_string(),
            qualified_name: Some(name.to_string()),
            generic_params: Vec::new(),
            generic_param_ids: Vec::new(),
            generic_bounds: HashMap::new(),
            params: Vec::new(),
            ret_type: body.ty.clone(),
            body: HirBlock {
                stmts: vec![HirStmt::Expr(body.clone())],
                ty: body.ty,
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }

    fn int_expr(value: i64) -> HirExpr {
        HirExpr {
            kind: HirExprKind::IntLiteral(value),
            ty: Type::I64,
            span: Default::default(),
        }
    }

    #[test]
    fn metadata_records_named_impl_method_backend_symbols_from_instances() {
        let impl_id = def_id(10);
        let method_id = def_id(11);
        let instance_id = InstanceId(0);
        let mono = MonomorphizedProgram {
            program: empty_program(),
            instances: BTreeMap::from([(
                instance_id,
                InstanceRecord {
                    id: instance_id,
                    origin: InstanceOrigin::ImplMethod {
                        owner: InstanceImplOwner::Named(impl_id),
                        method: method_id,
                    },
                    substitution: Vec::new(),
                    source_name: "Box::show".to_string(),
                    backend_symbol: "Box_show_exact".to_string(),
                    declared: None,
                    body: None,
                    provided_by_object: true,
                    is_specialization: false,
                },
            )]),
            type_context: TypeContext::new(),
        };

        let metadata = BackendSelectionMetadata::from_program(&mono, &empty_mir());

        assert_eq!(
            metadata.impl_method_backend_symbol(impl_id, method_id),
            Some("Box_show_exact")
        );
    }

    #[test]
    fn metadata_records_trait_member_ids_from_traits() {
        let trait_id = def_id(20);
        let signature_id = def_id(21);
        let default_method_id = def_id(22);
        let default_method = function(default_method_id, "default_show", int_expr(1));
        let trait_def = HirTrait {
            id: trait_id,
            name: "Show".to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods: HashMap::from([("default_show".to_string(), default_method)]),
            signatures: HashMap::from([(
                "show".to_string(),
                HirFunctionSig {
                    id: signature_id,
                    name: "show".to_string(),
                    generic_params: Vec::new(),
                    generic_param_ids: Vec::new(),
                    params: Vec::new(),
                    ret: Type::I64,
                    generic_bounds: HashMap::new(),
                    self_receiver: None,
                },
            )]),
        };
        let mono = MonomorphizedProgram {
            program: HirProgram::from_parts(
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::from([("Show".to_string(), trait_def)]),
                Vec::new(),
                Vec::new(),
            ),
            instances: BTreeMap::new(),
            type_context: TypeContext::new(),
        };

        let metadata = BackendSelectionMetadata::from_program(&mono, &empty_mir());

        assert!(metadata.trait_member_matches_target(trait_id, "show", signature_id));
        assert!(metadata.trait_member_matches_target(trait_id, "default_show", default_method_id));
        assert!(!metadata.trait_member_matches_target(trait_id, "show", default_method_id));
    }

    #[test]
    fn metadata_matches_projection_impls_without_codegen_hir_scan() {
        let impl_id = def_id(30);
        let trait_id = def_id(31);
        let generic = GenericParamId {
            owner: impl_id,
            index: 0,
        };
        let assoc_type_id = AssocTypeId(0);
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::BuiltinSlice,
            type_name: "[T; 3]".to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic(generic)],
            trait_name: Some("Index".to_string()),
            trait_id: Some(trait_id),
            trait_generics: Vec::new(),
            trait_arg_types: vec![Type::I64],
            associated_types: vec![HirAssociatedTypeDef {
                id: assoc_type_id,
                name: "Output".to_string(),
                ty: Type::Bool,
            }],
            bounds: Vec::new(),
            methods: HashMap::new(),
        };
        let mono = MonomorphizedProgram {
            program: HirProgram::from_parts(
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                vec![imp],
                Vec::new(),
            ),
            instances: BTreeMap::new(),
            type_context: TypeContext::new(),
        };
        let recv_ty = Type::Array(Box::new(Type::I64), 3);

        let metadata = BackendSelectionMetadata::from_program(&mono, &empty_mir());
        let selected = metadata
            .projection_impl(&[recv_ty.to_string()], &recv_ty, trait_id, &[Type::I64])
            .expect("projection impl metadata");

        assert_eq!(selected.id, impl_id);
        assert_eq!(
            selected.associated_types[0].ty,
            Type::Bool,
            "associated type facts stay with metadata"
        );
    }

    #[test]
    fn metadata_collects_builtin_index_traits_from_selected_index_syntax() {
        let trait_id = def_id(40);
        let method_id = def_id(41);
        let body = HirExpr {
            kind: HirExprKind::Deref(Box::new(HirExpr {
                kind: HirExprKind::MethodCall(
                    Box::new(HirExpr {
                        kind: HirExprKind::Var("arr".to_string()),
                        ty: Type::Array(Box::new(Type::I64), 3),
                        span: Default::default(),
                    }),
                    "index".to_string(),
                    vec![int_expr(0)],
                    None,
                    Some(crate::hir::HirMethodCallTarget {
                        impl_id: None,
                        trait_id: Some(trait_id),
                        trait_args: vec![Type::I64],
                        method_id,
                        from_index_operator: true,
                    }),
                ),
                ty: Type::Reference {
                    mutable: false,
                    inner: Box::new(Type::I64),
                },
                span: Default::default(),
            })),
            ty: Type::I64,
            span: Default::default(),
        };
        let function = function(def_id(42), "main", body);
        let mono = MonomorphizedProgram {
            program: HirProgram::from_parts(
                HashMap::from([("main".to_string(), function)]),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                Vec::new(),
                Vec::new(),
            ),
            instances: BTreeMap::new(),
            type_context: TypeContext::new(),
        };

        let metadata = BackendSelectionMetadata::from_program(&mono, &empty_mir());

        assert!(metadata.is_builtin_index_trait(trait_id));
    }
}
```

In `lib/src/codegen/mod.rs`, add the module declaration near the other codegen modules:

```rust
mod metadata;
```

- [ ] **Step 2: Run the red metadata tests**

Run: `cargo test -p rock-lib codegen::metadata -- --nocapture`

Expected: FAIL. The tests compile, and the assertions fail because `BackendSelectionMetadata::from_program` returns empty metadata.

- [ ] **Step 3: Implement the metadata builder**

Replace the `impl BackendSelectionMetadata` block in `lib/src/codegen/metadata.rs` with this implementation:

```rust
impl BackendSelectionMetadata {
    pub(crate) fn from_program(
        program: &MonomorphizedProgram,
        _mir: &MirProgram,
    ) -> Self {
        let mut metadata = Self::default();

        for record in program.instances.values() {
            metadata.record_instance(record);
        }
        metadata.collect_trait_member_ids(&program.program);
        metadata.collect_projection_impls(&program.program);
        metadata.collect_builtin_index_trait_targets(&program.program, &program.instances);

        metadata
    }

    pub(crate) fn record_instance(&mut self, record: &InstanceRecord) {
        if let InstanceOrigin::ImplMethod {
            owner: InstanceImplOwner::Named(impl_id),
            method,
        } = &record.origin
        {
            self.impl_method_backend_symbols
                .insert((*impl_id, *method), record.backend_symbol.clone());
        }
    }

    pub(crate) fn impl_method_backend_symbol(
        &self,
        impl_id: DefId,
        method_id: DefId,
    ) -> Option<&str> {
        self.impl_method_backend_symbols
            .get(&(impl_id, method_id))
            .map(String::as_str)
    }

    pub(crate) fn trait_member_matches_target(
        &self,
        trait_id: DefId,
        member_name: &str,
        method_id: DefId,
    ) -> bool {
        self.trait_member_ids
            .get(&(trait_id, member_name.to_string()))
            .is_none_or(|selected_id| *selected_id == method_id)
    }

    pub(crate) fn projection_impl(
        &self,
        lookup_type_names: &[String],
        recv_ty: &Type,
        trait_id: DefId,
        trait_args: &[Type],
    ) -> Option<&HirImpl> {
        let receiver_arg_types = crate::selection::receiver_arg_types(recv_ty);

        self.projection_impls
            .iter()
            .find(|entry| {
                entry.imp.trait_id == Some(trait_id)
                    && lookup_type_names
                        .iter()
                        .any(|name| crate::selection::impl_matches_method_lookup_type(&entry.imp, name))
                    && Self::receiver_and_trait_args_match(
                        &entry.imp,
                        &receiver_arg_types,
                        trait_args,
                    )
            })
            .map(|entry| &entry.imp)
    }

    pub(crate) fn is_builtin_index_trait(&self, trait_id: DefId) -> bool {
        self.builtin_index_trait_ids.contains(&trait_id)
    }

    fn collect_trait_member_ids(&mut self, program: &HirProgram) {
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

    fn collect_projection_impls(&mut self, program: &HirProgram) {
        for (_, imp) in program.impls_in_order() {
            if imp.trait_id.is_some() {
                self.projection_impls
                    .push(ProjectionImplMetadata::from_impl(imp.clone()));
            }
        }
    }

    fn collect_builtin_index_trait_targets(
        &mut self,
        program: &HirProgram,
        instances: &BTreeMap<crate::ids::InstanceId, InstanceRecord>,
    ) {
        for (_, _, function) in program.functions_by_id() {
            self.collect_index_trait_targets_block(&function.body);
        }

        for (_, imp) in program.impls_in_order() {
            for function in imp.methods.values() {
                self.collect_index_trait_targets_block(&function.body);
            }
        }

        for record in instances.values() {
            if let Some(function) = record.body.as_ref().or(record.declared.as_ref()) {
                self.collect_index_trait_targets_block(&function.body);
            }
        }
    }

    fn collect_index_trait_targets_block(&mut self, block: &HirBlock) {
        for stmt in &block.stmts {
            match stmt {
                HirStmt::Let { value, .. } | HirStmt::Expr(value) => {
                    self.collect_index_trait_targets_expr(value);
                }
                HirStmt::Return(Some(value)) | HirStmt::Break(Some(value)) => {
                    self.collect_index_trait_targets_expr(value);
                }
                HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => {}
            }
        }
    }

    fn collect_index_trait_targets_expr(&mut self, expr: &HirExpr) {
        match &expr.kind {
            HirExprKind::IntLiteral(_)
            | HirExprKind::FloatLiteral(_)
            | HirExprKind::BoolLiteral(_)
            | HirExprKind::StringLiteral(_)
            | HirExprKind::CharLiteral(_)
            | HirExprKind::Unit
            | HirExprKind::Var(_)
            | HirExprKind::ResolvedVar(_) => {}
            HirExprKind::ArrayLiteral(values) | HirExprKind::TupleLiteral(values) => {
                for value in values {
                    self.collect_index_trait_targets_expr(value);
                }
            }
            HirExprKind::FieldAccess(base, _, _)
            | HirExprKind::TupleIndex(base, _)
            | HirExprKind::UnaryOp(_, base)
            | HirExprKind::Ref(_, base)
            | HirExprKind::Cast(base, _) => {
                self.collect_index_trait_targets_expr(base);
            }
            HirExprKind::Deref(base) => {
                if let HirExprKind::MethodCall(_, method, args, _, target) = &base.kind {
                    if method == "index"
                        && args.len() == 1
                        && target
                            .as_ref()
                            .is_some_and(|target| target.from_index_operator)
                    {
                        if let Some(trait_id) = target.as_ref().and_then(|target| target.trait_id) {
                            self.builtin_index_trait_ids.insert(trait_id);
                        }
                    }
                }
                self.collect_index_trait_targets_expr(base);
            }
            HirExprKind::Index(base, index)
            | HirExprKind::BinOp(_, base, index)
            | HirExprKind::Assign(base, index)
            | HirExprKind::Range(base, index) => {
                self.collect_index_trait_targets_expr(base);
                self.collect_index_trait_targets_expr(index);
            }
            HirExprKind::Call(callee, args, _) => {
                self.collect_index_trait_targets_expr(callee);
                for arg in args {
                    self.collect_index_trait_targets_expr(arg);
                }
            }
            HirExprKind::MethodCall(receiver, _, args, _, _) => {
                self.collect_index_trait_targets_expr(receiver);
                for arg in args {
                    self.collect_index_trait_targets_expr(arg);
                }
            }
            HirExprKind::StructLiteral(_, _, fields) => {
                for field in fields {
                    self.collect_index_trait_targets_expr(&field.value);
                }
            }
            HirExprKind::EnumVariant(_, _, args, _) | HirExprKind::Intrinsic { args, .. } => {
                for arg in args {
                    self.collect_index_trait_targets_expr(arg);
                }
            }
            HirExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_index_trait_targets_expr(condition);
                self.collect_index_trait_targets_block(then_branch);
                if let Some(else_branch) = else_branch {
                    self.collect_index_trait_targets_block(else_branch);
                }
            }
            HirExprKind::Match { scrutinee, arms } => {
                self.collect_index_trait_targets_expr(scrutinee);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.collect_index_trait_targets_expr(guard);
                    }
                    self.collect_index_trait_targets_block(&arm.body);
                }
            }
            HirExprKind::While { condition, body } => {
                self.collect_index_trait_targets_expr(condition);
                self.collect_index_trait_targets_block(body);
            }
            HirExprKind::For { iter, body, .. } => {
                self.collect_index_trait_targets_expr(iter);
                self.collect_index_trait_targets_block(body);
            }
            HirExprKind::Loop(body) | HirExprKind::Block(body) => {
                self.collect_index_trait_targets_block(body);
            }
            HirExprKind::Lambda { body, .. } => {
                self.collect_index_trait_targets_block(body);
            }
        }
    }

    fn receiver_and_trait_args_match(
        imp: &HirImpl,
        receiver_arg_types: &[Type],
        trait_args: &[Type],
    ) -> bool {
        if imp.receiver_arg_types.len() != receiver_arg_types.len()
            || imp.trait_arg_types.len() != trait_args.len()
        {
            return false;
        }

        let mut subst = HashMap::new();
        for (expected, actual) in imp.receiver_arg_types.iter().zip(receiver_arg_types.iter()) {
            crate::selection::infer_generic_subst_from_types(expected, actual, &mut subst);
            if expected.substitute_generics(&subst) != *actual {
                return false;
            }
        }

        imp.trait_arg_types
            .iter()
            .zip(trait_args.iter())
            .all(|(expected, actual)| expected.substitute_generics(&subst) == *actual)
    }
}
```

Remove unused imports from `metadata.rs` after the replacement. The final import block should include exactly these production imports:

```rust
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::hir::{HirBlock, HirExpr, HirExprKind, HirImpl, HirProgram, HirStmt};
use crate::ids::DefId;
use crate::mir::MirProgram;
use crate::mono::{InstanceImplOwner, InstanceOrigin, InstanceRecord, MonomorphizedProgram};
use crate::types::Type;
```

- [ ] **Step 4: Verify metadata tests pass**

Run: `cargo test -p rock-lib codegen::metadata -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Format and commit metadata builder**

Run: `cargo fmt --all --check && cargo test -p rock-lib codegen::metadata -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/metadata.rs
git commit -m "add backend selection metadata"
```

## Task 2: Resolve MIR Selected Methods Through Metadata

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Write failing MIR callable metadata tests**

In the `#[cfg(test)] mod tests` module of `lib/src/codegen/mod.rs`, add these tests after `prepare_program_declarations_registers_instance_symbols_without_bodies`:

```rust
    #[test]
    fn mir_callable_method_resolves_through_backend_metadata() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let impl_id = DefId::new(CrateId(0), LocalDefId(200));
        let method_id = DefId::new(CrateId(0), LocalDefId(201));
        codegen
            .backend_metadata
            .impl_method_backend_symbols
            .insert((impl_id, method_id), "Box_show_exact".to_string());

        let symbol = codegen
            .resolve_mir_callable_symbol(&crate::mir::MirCallable::Method {
                impl_id: Some(impl_id),
                trait_id: None,
                trait_args: Vec::new(),
                method_id,
                instance: None,
                display_name: "show".to_string(),
            })
            .unwrap();

        assert_eq!(symbol, "Box_show_exact");
    }

    #[test]
    fn mir_callable_method_missing_metadata_fails_without_name_lookup() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let impl_id = DefId::new(CrateId(0), LocalDefId(210));
        let method_id = DefId::new(CrateId(0), LocalDefId(211));
        codegen.functions.insert(
            "Box_show_exact".to_string(),
            codegen
                .module
                .add_function("Box_show_exact", context.i64_type().fn_type(&[], false), None),
        );

        let err = codegen
            .resolve_mir_callable_symbol(&crate::mir::MirCallable::Method {
                impl_id: Some(impl_id),
                trait_id: None,
                trait_args: Vec::new(),
                method_id,
                instance: None,
                display_name: "show".to_string(),
            })
            .unwrap_err();

        assert!(
            err.message.contains("Unknown MIR callable"),
            "unexpected error: {err}"
        );
    }
```

- [ ] **Step 2: Run the red MIR callable tests**

Run: `cargo test -p rock-lib mir_callable_method_ -- --nocapture`

Expected: FAIL to compile because `CodeGen` does not yet have a `backend_metadata` field.

- [ ] **Step 3: Add metadata storage to `CodeGen`**

In `lib/src/codegen/mod.rs`, add this import near the other crate imports:

```rust
use metadata::BackendSelectionMetadata;
```

Add this field to `CodeGen` after `mir_closure_metadata`:

```rust
    backend_metadata: BackendSelectionMetadata,
```

Initialize it in `CodeGen::new` after `mir_closure_metadata: HashMap::new(),`:

```rust
            backend_metadata: BackendSelectionMetadata::default(),
```

In `register_instances`, replace the direct impl-method symbol insert block:

```rust
            if let InstanceOrigin::ImplMethod {
                owner: InstanceImplOwner::Named(impl_id),
                method,
            } = record.origin
            {
                self.impl_method_backend_symbols
                    .insert((impl_id, method), record.backend_symbol.clone());
            }
```

with:

```rust
            self.backend_metadata.record_instance(record);
```

In `resolve_mir_callable_symbol`, change the method branch with `impl_id: Some` so it reads from metadata:

```rust
            MirCallable::Method {
                impl_id: Some(impl_id),
                method_id,
                ..
            } => self
                .backend_metadata
                .impl_method_backend_symbol(*impl_id, *method_id),
```

The full `let symbol = match callable { ... };` expression should still return `Option<&str>` or `Option<&String>` consistently. If the current mixed references make that awkward, convert the whole match to `Option<String>`:

```rust
        let symbol = match callable {
            MirCallable::Function(id) => self.function_symbols_by_id.get(id).cloned(),
            MirCallable::Extern(id) => self.extern_symbols_by_id.get(id).cloned(),
            MirCallable::Instance(id) => self.instance_symbols_by_id.get(id).cloned(),
            MirCallable::Method {
                instance: Some(id), ..
            } => self.instance_symbols_by_id.get(id).cloned(),
            MirCallable::Method {
                impl_id: Some(impl_id),
                method_id,
                ..
            } => self
                .backend_metadata
                .impl_method_backend_symbol(*impl_id, *method_id)
                .map(str::to_string),
            MirCallable::Method { method_id, .. } => self.function_symbols_by_id.get(method_id).cloned(),
            _ => None,
        };

        symbol.ok_or_else(|| CodegenError::from(format!("Unknown MIR callable: {:?}", callable)))
```

- [ ] **Step 4: Verify MIR callable tests pass**

Run: `cargo test -p rock-lib mir_callable_method_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Build metadata at the MIR declaration boundary**

At the start of `prepare_mir_program_declarations`, before `prepare_program_declarations_inner`, insert:

```rust
        self.backend_metadata = BackendSelectionMetadata::from_program(program, mir);
```

The method should become:

```rust
    fn prepare_mir_program_declarations(
        &mut self,
        program: &MonomorphizedProgram,
        mir: &crate::mir::MirProgram,
    ) -> Result<(), CodegenError> {
        self.backend_metadata = BackendSelectionMetadata::from_program(program, mir);
        self.prepare_program_declarations_inner(program, false, false)?;
        self.declare_mir_functions(program, mir)?;
        self.register_impl_method_aliases(&program.program);
        Ok(())
    }
```

This step intentionally leaves the old HIR scans in place. Task 4 removes them from the active MIR path after projection and builtin-index metadata are wired.

- [ ] **Step 6: Verify and commit MIR method metadata resolution**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir_callable_method_ -- --nocapture && cargo test -p rock-lib codegen::metadata -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/metadata.rs
git commit -m "resolve mir methods through backend metadata"
```

## Task 3: Move Projection And Builtin Index Queries To Metadata

**Files:**
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Test: `lib/src/codegen/types.rs`
- Test: `lib/src/codegen/expr/mod.rs`

- [ ] **Step 1: Update projection tests to use metadata injection**

In `lib/src/codegen/types.rs`, add this import inside the test module:

```rust
    use crate::codegen::metadata::ProjectionImplMetadata;
```

In `resolve_projection_type_prefers_user_index_impl_over_builtin_array_output`, replace the direct `codegen.builtin_index_trait_ids.insert(trait_id);` and `codegen.trait_impls.insert(...)` setup with:

```rust
        codegen
            .backend_metadata
            .builtin_index_trait_ids
            .insert(trait_id);
        codegen
            .backend_metadata
            .projection_impls
            .push(ProjectionImplMetadata::from_impl(HirImpl {
                id: impl_id,
                owner: HirImplOwner::Named(base_ty.to_string()),
                type_name: base_ty.to_string(),
                type_generics: vec![],
                receiver_arg_types: vec![Type::I64],
                trait_name: Some("Index".to_string()),
                trait_id: Some(trait_id),
                trait_generics: vec![],
                trait_arg_types: vec![Type::I64],
                associated_types: vec![HirAssociatedTypeDef {
                    id: assoc_type_id,
                    name: "Output".to_string(),
                    ty: Type::Bool,
                }],
                bounds: vec![],
                methods: HashMap::new(),
            }));
```

In `resolve_projection_type_rejects_assoc_type_owned_by_different_trait`, replace the direct `codegen.trait_impls.insert(...)` setup with:

```rust
        codegen
            .backend_metadata
            .projection_impls
            .push(ProjectionImplMetadata::from_impl(HirImpl {
                id: impl_id,
                owner: HirImplOwner::Named(base_ty.to_string()),
                type_name: base_ty.to_string(),
                type_generics: vec![],
                receiver_arg_types: vec![],
                trait_name: Some("Show".to_string()),
                trait_id: Some(trait_id),
                trait_generics: vec![],
                trait_arg_types: vec![],
                associated_types: vec![HirAssociatedTypeDef {
                    id: assoc_type_id,
                    name: "Output".to_string(),
                    ty: Type::Bool,
                }],
                bounds: vec![],
                methods: HashMap::new(),
            }));
```

In `lib/src/codegen/expr/mod.rs`, update `targeted_builtin_index_call_uses_hir_trait_id_when_no_user_impl_exists` and `targeted_builtin_index_call_preserves_user_impl_precedence` so they insert builtin/projection metadata through `codegen.backend_metadata` instead of `codegen.builtin_index_trait_ids` and `codegen.trait_impls`.

- [ ] **Step 2: Run red projection/index tests**

Run: `cargo test -p rock-lib resolve_projection_type_ -- --nocapture`

Expected: FAIL to compile or fail assertions because `ProjectionProvider for CodeGen` still uses the old fields.

Run: `cargo test -p rock-lib targeted_builtin_index_call_ -- --nocapture`

Expected: FAIL to compile or fail assertions because `is_builtin_index_method_call` still uses the old fields.

- [ ] **Step 3: Route `ProjectionProvider` through metadata**

In `lib/src/codegen/types.rs`, replace `find_projection_impl` with:

```rust
    fn find_projection_impl(
        &self,
        base_ty: &Type,
        trait_id: crate::ids::DefId,
        trait_args: &[Type],
    ) -> Option<crate::hir::HirImpl> {
        let lookup_names = self.get_type_names_for_method(base_ty);
        self.backend_metadata
            .projection_impl(&lookup_names, base_ty, trait_id, trait_args)
            .cloned()
    }
```

In the same impl, replace `is_builtin_index_trait` with:

```rust
    fn is_builtin_index_trait(&self, trait_id: crate::ids::DefId) -> bool {
        self.backend_metadata.is_builtin_index_trait(trait_id)
    }
```

In `lib/src/codegen/mod.rs`, keep `projection_substitution` unchanged because it still receives an already-selected metadata impl and builds the same substitution map.

- [ ] **Step 4: Route builtin index selected-target checks through metadata**

In `lib/src/codegen/mod.rs`, replace `is_builtin_index_method_call` with:

```rust
    fn is_builtin_index_method_call(
        &self,
        recv_ty: &Type,
        method_name: &str,
        args: &[HirExpr],
        target: Option<&HirMethodCallTarget>,
    ) -> bool {
        if !self.is_builtin_index_call(recv_ty, method_name, args) {
            return false;
        }

        let Some(target) = target else {
            return true;
        };
        if !target.from_index_operator {
            return false;
        }
        if target.impl_id.is_some() {
            return false;
        }
        let Some(trait_id) = target.trait_id else {
            return false;
        };
        if !self.backend_metadata.is_builtin_index_trait(trait_id) {
            return false;
        }

        let resolved_recv_ty = self.resolve_projection_type(recv_ty);
        let arg_types = args
            .iter()
            .map(|arg| self.resolve_projection_type(&arg.ty))
            .collect::<Vec<_>>();
        let lookup_names = self.get_type_names_for_method(&resolved_recv_ty);
        self.backend_metadata
            .projection_impl(&lookup_names, &resolved_recv_ty, trait_id, &arg_types)
            .is_none()
    }
```

- [ ] **Step 5: Verify projection/index tests pass**

Run: `cargo test -p rock-lib resolve_projection_type_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib targeted_builtin_index_call_ -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit projection and builtin-index metadata migration**

Run: `cargo fmt --all --check && cargo test -p rock-lib resolve_projection_type_ -- --nocapture && cargo test -p rock-lib targeted_builtin_index_call_ -- --nocapture && cargo test -p rock-lib codegen::metadata -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/types.rs lib/src/codegen/expr/mod.rs lib/src/codegen/metadata.rs
git commit -m "use backend metadata for projection codegen"
```

## Task 4: Remove Active MIR Declaration HIR Trait/Member Scans

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Write a failing active-path alias regression test**

In `lib/src/codegen/mod.rs`, add this test after `compile_mir_program_declares_instance_from_mir_body_without_hir_body`:

```rust
    #[test]
    fn compile_mir_program_does_not_register_hir_impl_method_aliases() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        let impl_id = DefId::new(CrateId(0), LocalDefId(300));
        let method_id = DefId::new(CrateId(0), LocalDefId(301));
        let main_id = DefId::new(CrateId(0), LocalDefId(302));
        let method_instance = InstanceId(0);
        let main_instance = InstanceId(1);
        let mut type_context = crate::type_context::TypeContext::new();
        let i64_type = type_id(&mut type_context, Type::I64);
        let mut method = function_with_expr(expr(HirExprKind::IntLiteral(7), Type::I64));
        method.id = method_id;
        method.name = "show".to_string();
        method.qualified_name = Some("Box_Show_show".to_string());
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Box".to_string()),
            type_name: "Box".to_string(),
            type_generics: Vec::new(),
            receiver_arg_types: Vec::new(),
            trait_name: Some("Show".to_string()),
            trait_id: Some(DefId::new(CrateId(0), LocalDefId(303))),
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::from([("show".to_string(), method.clone())]),
        };
        let main_function = function_with_expr(expr(HirExprKind::IntLiteral(0), Type::I64));
        let mono = MonomorphizedProgram {
            program: HirProgram::from_parts(
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                vec![imp],
                Vec::new(),
            ),
            instances: BTreeMap::from([
                (
                    method_instance,
                    InstanceRecord {
                        id: method_instance,
                        origin: InstanceOrigin::ImplMethod {
                            owner: InstanceImplOwner::Named(impl_id),
                            method: method_id,
                        },
                        substitution: Vec::new(),
                        source_name: "Box::show".to_string(),
                        backend_symbol: "exact_Box_show".to_string(),
                        declared: Some(method),
                        body: None,
                        provided_by_object: true,
                        is_specialization: false,
                    },
                ),
                (
                    main_instance,
                    InstanceRecord {
                        id: main_instance,
                        origin: InstanceOrigin::Function(main_id),
                        substitution: Vec::new(),
                        source_name: "main".to_string(),
                        backend_symbol: "main".to_string(),
                        declared: None,
                        body: Some(main_function),
                        provided_by_object: false,
                        is_specialization: false,
                    },
                ),
            ]),
            type_context: type_context.clone(),
        };
        let main_mir_id = crate::mir::MirFunctionId::Instance(main_instance);
        let call_block = crate::mir::BasicBlock {
            statements: Vec::new(),
            terminator: Some(crate::mir::Terminator::Call {
                func: crate::mir::Operand::Constant(crate::mir::Constant::Callable(
                    crate::mir::MirCallable::Method {
                        impl_id: Some(impl_id),
                        trait_id: None,
                        trait_args: Vec::new(),
                        method_id,
                        instance: None,
                        display_name: "show".to_string(),
                    },
                )),
                args: Vec::new(),
                destination: crate::mir::Place {
                    local: crate::mir::Local(0),
                    projection: Vec::new(),
                },
                target: crate::mir::BasicBlockId(1),
                cleanup: None,
            }),
        };
        let return_block = crate::mir::BasicBlock {
            statements: Vec::new(),
            terminator: Some(crate::mir::Terminator::Return),
        };
        let mir_function = crate::mir::MirFunction {
            id: main_mir_id.clone(),
            name: "main".to_string(),
            basic_blocks: vec![call_block, return_block],
            local_decls: vec![crate::mir::LocalDecl {
                ty: i64_type,
                mutability: crate::mir::Mutability::Mut,
                name: Some("return_place".to_string()),
                span: None,
            }],
            closure_captures: Vec::new(),
            arg_count: 0,
            ret_type: i64_type,
        };
        let mir = crate::mir::MirProgram {
            functions: BTreeMap::from([(main_mir_id, mir_function)]),
            type_context,
        };

        codegen.compile_program_from_mir(&mono, &mir).unwrap();

        assert!(codegen.functions.contains_key("exact_Box_show"));
        assert!(!codegen.functions.contains_key("Box_show"));
        assert!(!codegen.functions.contains_key("Box_Show_show"));
    }
```

- [ ] **Step 2: Run the red alias regression test**

Run: `cargo test -p rock-lib compile_mir_program_does_not_register_hir_impl_method_aliases -- --nocapture`

Expected: FAIL because `prepare_mir_program_declarations` still calls `register_impl_method_aliases`, which inserts at least one HIR-derived alias.

- [ ] **Step 3: Split active MIR declaration preparation away from HIR trait/member registration**

Replace `prepare_mir_program_declarations` in `lib/src/codegen/mod.rs` with:

```rust
    fn prepare_mir_program_declarations(
        &mut self,
        program: &MonomorphizedProgram,
        mir: &crate::mir::MirProgram,
    ) -> Result<(), CodegenError> {
        self.backend_metadata = BackendSelectionMetadata::from_program(program, mir);
        let instances = &program.instances;
        let hir_program = &program.program;

        self.register_nominal_layouts(hir_program);
        self.register_instances(instances, false);
        self.declare_runtime();
        for (_, ext) in hir_program.externs_in_order() {
            if self.functions.get(&ext.name).is_none() {
                self.declare_extern(ext);
            }
            self.extern_symbols_by_id.insert(ext.id, ext.name.clone());
        }
        self.declare_mir_functions(program, mir)?;

        Ok(())
    }
```

Do not call these helpers from `prepare_mir_program_declarations`:

```rust
self.register_index_trait_targets(...);
self.register_impl_method_aliases(...);
self.register_trait_member_ids(...);
self.register_trait_impls(...);
```

Leave `prepare_program_declarations_inner` unchanged for the test-only HIR path in this task.

- [ ] **Step 4: Verify active-path alias test passes**

Run: `cargo test -p rock-lib compile_mir_program_does_not_register_hir_impl_method_aliases -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Verify active MIR/codegen focused suite**

Run: `cargo test -p rock-lib codegen::tests::compile_mir_program -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit active path split**

Run: `cargo fmt --all --check && cargo test -p rock-lib compile_mir_program_does_not_register_hir_impl_method_aliases -- --nocapture && cargo test -p rock-lib codegen::tests::compile_mir_program -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs
git commit -m "stop mir codegen registering hir method aliases"
```

## Task 5: Remove Direct Codegen Trait Impl Lookup From Production Helpers

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/types.rs`
- Test: `lib/src/codegen/expr/mod.rs`
- Test: `lib/src/codegen/types.rs`

- [ ] **Step 1: Update expression selected-target helpers to use metadata APIs**

In `lib/src/codegen/mod.rs`, replace `selected_trait_member_matches_target` with metadata-backed logic:

```rust
    fn selected_trait_member_matches_target(
        &self,
        trait_id: DefId,
        method_name: &str,
        method_id: DefId,
    ) -> bool {
        self.backend_metadata
            .trait_member_matches_target(trait_id, method_name, method_id)
    }
```

Add this helper near `selected_trait_member_matches_target`:

```rust
    fn find_projection_metadata_impl(
        &self,
        recv_ty: &Type,
        trait_id: DefId,
        trait_args: &[Type],
    ) -> Option<&HirImpl> {
        let lookup_names = self.get_type_names_for_method(recv_ty);
        self.backend_metadata
            .projection_impl(&lookup_names, recv_ty, trait_id, trait_args)
    }
```

In `lib/src/codegen/expr/mod.rs`, replace calls to `self.find_trait_impl(...)` with `self.find_projection_metadata_impl(...)`.

Replace the selected-trait block inside method call resolution:

```rust
                            let impl_match = if selected_trait_args.is_empty() {
                                self.find_trait_impl(&resolved_recv_ty, trait_id, &arg_types)
                                    .or_else(|| {
                                        self.find_trait_impl(&resolved_recv_ty, trait_id, &[])
                                    })
                            } else {
                                self.find_trait_impl(
                                    &resolved_recv_ty,
                                    trait_id,
                                    &selected_trait_args,
                                )
                            };
```

with:

```rust
                            let impl_match = if selected_trait_args.is_empty() {
                                self.find_projection_metadata_impl(&resolved_recv_ty, trait_id, &arg_types)
                                    .or_else(|| {
                                        self.find_projection_metadata_impl(&resolved_recv_ty, trait_id, &[])
                                    })
                            } else {
                                self.find_projection_metadata_impl(
                                    &resolved_recv_ty,
                                    trait_id,
                                    &selected_trait_args,
                                )
                            };
```

Replace any remaining direct `self.trait_impls` iteration in the selected-target branch with metadata lookup through `find_projection_metadata_impl`. Keep targetless `has_array_fallback` compatibility only if the branch is reachable without a selected target; it must not run when `method_target.is_some()`.

- [ ] **Step 2: Update expression tests to inject metadata**

In `lib/src/codegen/expr/mod.rs`, add this import in the test module:

```rust
    use crate::codegen::metadata::ProjectionImplMetadata;
```

Replace every test setup that writes `codegen.trait_impls.insert(...)` with:

```rust
        codegen
            .backend_metadata
            .projection_impls
            .push(ProjectionImplMetadata::from_impl(HirImpl {
                id: impl_id,
                owner: HirImplOwner::Named("Box".to_string()),
                type_name: "Box".to_string(),
                type_generics: Vec::new(),
                receiver_arg_types: Vec::new(),
                trait_name: Some("Show".to_string()),
                trait_id: Some(trait_id),
                trait_generics: Vec::new(),
                trait_arg_types: Vec::new(),
                associated_types: Vec::new(),
                bounds: Vec::new(),
                methods: HashMap::from([("value".to_string(), actual_method)]),
            }));
```

Use the same field values currently present in each test's `HirImpl`; only change the insertion target from `codegen.trait_impls` to `codegen.backend_metadata.projection_impls.push(ProjectionImplMetadata::from_impl(...))`.

Replace every test setup that writes `codegen.trait_member_ids.insert(...)` with:

```rust
        codegen
            .backend_metadata
            .trait_member_ids
            .insert((trait_id, "show".to_string()), actual_signature_id);
```

When a selected expression test expects successful method resolution through a specific impl method, add this explicit metadata symbol setup:

```rust
        codegen
            .backend_metadata
            .impl_method_backend_symbols
            .insert((impl_id, method_id), "String_Show_show".to_string());
```

- [ ] **Step 3: Run red/green expression metadata tests**

Run: `cargo test -p rock-lib selected_method_target_ -- --nocapture`

Expected after Step 1 and Step 2: PASS. If a test fails because it relied on HIR impl-name aliasing, update that test to insert the exact `impl_method_backend_symbols` metadata and assert the same behavior through the exact symbol.

Run: `cargo test -p rock-lib selected_trait_target_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib selected_index_target_ref_does_not_use_builtin_index_pointer -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Remove old direct trait impl fields and helpers**

In `lib/src/codegen/mod.rs`, remove these fields from `CodeGen`:

```rust
    trait_impls: HashMap<(String, Vec<TypeId>, DefId, Vec<TypeId>), crate::hir::HirImpl>,
    trait_member_ids: HashMap<(DefId, String), DefId>,
    impl_method_backend_symbols: HashMap<(DefId, DefId), String>,
```

Remove their initializers from `CodeGen::new`:

```rust
            trait_impls: HashMap::new(),
            trait_member_ids: HashMap::new(),
            impl_method_backend_symbols: HashMap::new(),
```

Remove these methods from `CodeGen` if they have no call sites after the replacements:

```rust
    fn find_trait_impl(...)
    fn impl_receiver_and_trait_args_match(...)
    pub fn register_trait_impls(...)
    fn register_trait_member_ids(...)
```

Keep `register_impl_method_aliases` only if `compile_program` still needs it for test-only HIR compatibility. It must not be called by `prepare_mir_program_declarations`.

- [ ] **Step 5: Verify no active direct trait impl lookup remains**

Run: `cargo test -p rock-lib selected_method_target_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib selected_trait_target_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib targeted_builtin_index_call_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib resolve_projection_type_ -- --nocapture`

Expected: PASS.

Run: `rg "trait_impls|trait_member_ids|impl_method_backend_symbols|find_trait_impl|register_trait_impls|register_trait_member_ids" lib/src/codegen`

Expected: matches are limited to `backend_metadata`, `ProjectionImplMetadata`, explicit metadata tests, or `register_impl_method_aliases` if it remains test-only. There should be no `CodeGen` field named `trait_impls`, no `CodeGen::find_trait_impl`, and no active MIR declaration call to `register_impl_method_aliases`.

- [ ] **Step 6: Commit direct lookup removal**

Run: `cargo fmt --all --check && cargo test -p rock-lib selected_method_target_ -- --nocapture && cargo test -p rock-lib selected_trait_target_ -- --nocapture && cargo test -p rock-lib targeted_builtin_index_call_ -- --nocapture && cargo test -p rock-lib resolve_projection_type_ -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/expr/mod.rs lib/src/codegen/types.rs lib/src/codegen/metadata.rs
git commit -m "remove direct codegen trait lookup"
```

## Task 6: Full Verification And Documentation Update

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify if needed: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [ ] **Step 1: Run focused backend metadata verification**

Run: `cargo test -p rock-lib codegen::metadata -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib codegen::tests::compile_mir_program -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib selected_method_target_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib selected_trait_target_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib targeted_builtin_index_call_ -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib resolve_projection_type_ -- --nocapture`

Expected: PASS.

- [ ] **Step 2: Run behavior filters for affected language surfaces**

Run these commands serially:

```bash
cargo test -p rock-lib --test integration test_generic -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
cargo test -p rock-lib --test integration test_stdlib -- --nocapture
```

Expected: each command PASS.

- [ ] **Step 3: Run full verification**

Run: `cargo fmt --all --check`

Expected: PASS.

Run: `cargo test -p rock-lib > /tmp/rock-lib-task21-backend-metadata.log 2>&1`

Expected: PASS. If it fails, inspect `/tmp/rock-lib-task21-backend-metadata.log`, fix the failing task-owned code, and rerun the smallest failing command first.

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 4: Update the audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, change the Task 5 remaining item:

```markdown
- [ ] Remove direct trait selection logic from codegen as part of Task 21 backend cleanup.
```

to:

```markdown
- [x] Removed direct trait/member selection lookup from the active MIR codegen path by routing selected method symbols, trait member IDs, projection impl facts, and builtin-index guards through explicit backend selection metadata.
```

In the Task 6 `Still to do` list, keep broad layout/extern/product/link metadata items open. Do not mark all MIR backend metadata cleanup complete unless declarations, layouts, externs, products, symbols, and link metadata have also been moved.

- [ ] **Step 5: Commit documentation and final code state**

Run: `git status --short`

Expected: only task-owned files are modified.

Run: `git diff -- docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

Expected: documentation changes only describe the direct trait/member metadata cleanup and do not claim broad backend metadata extraction is complete.

Commit:

```bash
git add lib/src/codegen/mod.rs lib/src/codegen/expr/mod.rs lib/src/codegen/types.rs lib/src/codegen/metadata.rs docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git commit -m "complete task 21 backend metadata cleanup"
```

## Completion Review

Before claiming completion, verify these facts from code and command output:

- `compile_program_from_mir` builds `BackendSelectionMetadata` and does not call `register_impl_method_aliases`, `register_trait_impls`, `register_trait_member_ids`, or `register_index_trait_targets`.
- `resolve_mir_callable_symbol` resolves selected methods through `backend_metadata.impl_method_backend_symbol` for `(impl_id, method_id)`.
- `ProjectionProvider for CodeGen` uses `backend_metadata.projection_impl` and `backend_metadata.is_builtin_index_trait`.
- `CodeGen::find_trait_impl` and `CodeGen::trait_impls` are absent, or any remaining references are test-only compatibility outside the active MIR path and are explicitly called out in the final response.
- Full verification from Task 6 passed, including `cargo test -p rock-lib` and `git diff --check`.

After Task 21 is complete, continue with the approved next work: Tasks 14-16 cleanup follow-ups.
