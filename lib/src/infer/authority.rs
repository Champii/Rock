use std::cell::RefCell;
use std::collections::HashMap;

use crate::hir::{
    HirBlock, HirExpr, HirExprKind, HirFieldLocation, HirFunction, HirGenericBounds, HirStmt,
    HirTypeBinding,
};
use crate::ids::DefId;
use crate::infer::PartialHir;
use crate::lower::ResolveError;
use crate::selection::{
    type_pattern_matches, ReceiverAdjustment, ReceiverCandidate, SelectionService,
};
use crate::types::Type;

pub(super) fn materialize_pending_method_calls(
    hir: &mut PartialHir,
    strict: bool,
) -> Result<(), Vec<ResolveError>> {
    let struct_ids = hir
        .structs
        .values()
        .map(|structure| (structure.id, structure.clone()))
        .collect();
    let bounds = HirGenericBounds::new();
    let traits = hir
        .traits
        .values()
        .map(|trait_def| (trait_def.id, trait_def.clone()))
        .collect();
    let impls = hir
        .impls
        .iter()
        .map(|(&id, impl_def)| (id, impl_def.clone()))
        .collect();
    let context = MethodAuthorityContext {
        traits: &traits,
        impls: &impls,
        sized_trait_id: hir
            .language_items
            .sized
            .as_ref()
            .map(|items| items.trait_id),
        bounds: &bounds,
        effective_trait_methods: &hir.imported_effective_trait_methods,
        structs: &struct_ids,
        engine: RefCell::new(&mut hir.engine),
        strict,
    };

    let mut errors = Vec::new();
    let mut function_ids = hir.functions.keys().copied().collect::<Vec<_>>();
    function_ids.sort();
    for id in function_ids {
        if let Some(function) = hir.functions.get_mut(&id) {
            context.materialize_function(function, &mut errors);
        }
    }
    let mut trait_ids = hir.traits.keys().copied().collect::<Vec<_>>();
    trait_ids.sort();
    for id in trait_ids {
        if let Some(trait_def) = hir.traits.get_mut(&id) {
            let mut methods = trait_def
                .methods
                .iter()
                .map(|(name, function)| (function.id, name.clone()))
                .collect::<Vec<_>>();
            methods.sort_by_key(|(method_id, _)| *method_id);
            for (_, name) in methods {
                if let Some(function) = trait_def.methods.get_mut(&name) {
                    context.materialize_function(function, &mut errors);
                }
            }
        }
    }
    let mut impl_ids = hir.impls.keys().copied().collect::<Vec<_>>();
    impl_ids.sort();
    for id in impl_ids {
        if let Some(imp) = hir.impls.get_mut(&id) {
            let mut methods = imp
                .methods
                .iter()
                .map(|(name, function)| (function.id, name.clone()))
                .collect::<Vec<_>>();
            methods.sort_by_key(|(method_id, _)| *method_id);
            for (_, name) in methods {
                if let Some(function) = imp.methods.get_mut(&name) {
                    context.materialize_function(function, &mut errors);
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn binding_sort_key(binding: &HirTypeBinding) -> (u32, u32, u32) {
    (
        binding.param.owner.crate_id.0,
        binding.param.owner.local.0,
        binding.param.index,
    )
}

struct MethodAuthorityContext<'a> {
    traits: &'a HashMap<DefId, crate::hir::HirTrait>,
    impls: &'a HashMap<DefId, crate::hir::HirImpl>,
    sized_trait_id: Option<DefId>,
    bounds: &'a HirGenericBounds,
    effective_trait_methods: &'a HashMap<(DefId, DefId), DefId>,
    structs: &'a HashMap<DefId, crate::hir::HirStruct>,
    engine: RefCell<&'a mut crate::infer::InferenceEngine>,
    strict: bool,
}

impl MethodAuthorityContext<'_> {
    fn service(&self) -> SelectionService<'_> {
        SelectionService::new(
            self.traits,
            self.impls,
            self.sized_trait_id,
            None,
            self.bounds,
        )
        .with_effective_trait_methods(self.effective_trait_methods)
    }

    fn materialize_function(&self, function: &mut HirFunction, errors: &mut Vec<ResolveError>) {
        self.materialize_block(&mut function.body, errors);
    }

    fn materialize_block(&self, block: &mut HirBlock, errors: &mut Vec<ResolveError>) {
        for stmt in &mut block.stmts {
            match stmt {
                HirStmt::Let { value, .. }
                | HirStmt::Expr(value)
                | HirStmt::Return(Some(value))
                | HirStmt::Break(Some(value)) => self.materialize_expr(value, errors),
                HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => {}
            }
        }
    }

    fn materialize_expr(&self, expr: &mut HirExpr, errors: &mut Vec<ResolveError>) {
        match &mut expr.kind {
            HirExprKind::Call(callee, args, target) => {
                for arg in args.iter_mut() {
                    self.materialize_expr(arg, errors);
                }
                if target.is_some() {
                    self.materialize_expr(callee, errors);
                    return;
                }
                let (mut receiver, method_name) = match &callee.kind {
                    HirExprKind::FieldAccess(receiver, method_name, None) => {
                        (receiver.as_ref().clone(), method_name.clone())
                    }
                    _ => {
                        self.materialize_expr(callee, errors);
                        return;
                    }
                };
                self.materialize_expr(&mut receiver, errors);
                receiver.ty = self.engine.borrow().resolve(&receiver.ty);
                let candidate = ReceiverCandidate {
                    expr: receiver.clone(),
                    adjustment: ReceiverAdjustment::None,
                    can_autoref_mut: false,
                };
                let mut selected = self.service().select_concrete_method_candidates(
                    std::slice::from_ref(&candidate),
                    &method_name,
                    |ty| self.engine.borrow().resolve(ty),
                );
                selected.sort_by_key(|selected| {
                    (selected.target.impl_id(), selected.target.method_id())
                });
                selected.dedup_by_key(|selected| {
                    (selected.target.impl_id(), selected.target.method_id())
                });
                if selected.is_empty() {
                    self.materialize_field_access(callee, errors);
                    if matches!(&callee.kind, HirExprKind::FieldAccess(_, _, Some(_))) {
                        return;
                    }
                    if !self.strict && contains_recovery_type(&receiver.ty) {
                        return;
                    }
                    if self
                        .service()
                        .mut_receiver_method_requires_mutable_receiver(
                            std::slice::from_ref(&candidate),
                            &method_name,
                            |ty| self.engine.borrow().resolve(ty),
                        )
                    {
                        errors.push(ResolveError::new(format!(
                            "Cannot call mutable receiver method '{}' without a mutable receiver",
                            method_name
                        )));
                        return;
                    }
                    let display = match &receiver.ty {
                        Type::Struct { id, .. } => self
                            .structs
                            .get(id)
                            .map(|structure| structure.name.as_str())
                            .unwrap_or("<unknown>"),
                        _ => "<unknown>",
                    };
                    errors.push(ResolveError::new(format!(
                        "Unknown field '{}' on struct '{}' (no matching typed method implementation for {})",
                        method_name, display, receiver.ty
                    )));
                    return;
                }
                if selected.len() > 1 {
                    errors.push(ResolveError::new(format!(
                        "cannot materialize method '{}' on {}: multiple matching typed implementations: {:?}",
                        method_name,
                        receiver.ty,
                        selected
                            .iter()
                            .map(|candidate| candidate.target.impl_id())
                            .collect::<Vec<_>>()
                    )));
                    return;
                }
                let selected = &mut selected[0];

                let Some(function) = selected.function.as_ref() else {
                    errors.push(ResolveError::new(format!(
                        "selected method '{}' has no executable body",
                        method_name
                    )));
                    return;
                };
                if selected.substituted_params.len() != args.len() {
                    errors.push(ResolveError::new(format!(
                        "selected method '{}' expects {} arguments but received {}",
                        method_name,
                        selected.substituted_params.len(),
                        args.len()
                    )));
                    return;
                }

                let mut substitution = selected.owner_substitution.clone();
                for (param, arg) in selected.substituted_params.iter().zip(args.iter()) {
                    let actual = self.engine.borrow().resolve(&arg.ty);
                    if !type_pattern_matches(&param.ty, &actual, &mut substitution) {
                        if !self.strict {
                            return;
                        }
                        errors.push(ResolveError::new(format!(
                            "selected method '{}' argument type {} does not match {}",
                            method_name, arg.ty, param.ty
                        )));
                        return;
                    }
                }
                let actual_return = self.engine.borrow().resolve(&expr.ty);
                let _ =
                    type_pattern_matches(&selected.return_type, &actual_return, &mut substitution);

                let owner_params = selected
                    .target
                    .owner_substitution
                    .iter()
                    .map(|binding| binding.param)
                    .collect::<std::collections::HashSet<_>>();
                let mut method_substitution = Vec::new();
                for param in &function.generic_params {
                    if owner_params.contains(&param.id)
                        || selected
                            .target
                            .trait_id()
                            .is_some_and(|id| param.id.owner == id)
                    {
                        continue;
                    }
                    let Some(ty) = substitution.get(&param.id) else {
                        if !self.strict {
                            return;
                        }
                        errors.push(ResolveError::new(format!(
                            "selected method '{}' leaves generic {:?} unresolved",
                            method_name, param
                        )));
                        return;
                    };
                    method_substitution.push(HirTypeBinding {
                        param: param.id,
                        ty: ty.clone(),
                    });
                }
                method_substitution.sort_by_key(binding_sort_key);

                let mut authority = selected.target.clone();
                authority.method_substitution = method_substitution;
                let selected_return = selected.return_type.substitute_generics(&substitution);
                {
                    let mut engine = self.engine.borrow_mut();
                    let _ = engine.unify(&expr.ty, &selected_return);
                    expr.ty = engine.resolve(&expr.ty);
                }
                expr.kind = HirExprKind::MethodCall(
                    Box::new(receiver),
                    method_name,
                    std::mem::take(args),
                    function.self_receiver,
                    Some(authority),
                );
            }
            HirExprKind::MethodCall(receiver, _, args, _, target) => {
                self.materialize_expr(receiver, errors);
                for arg in args {
                    self.materialize_expr(arg, errors);
                }
                if self.strict && target.is_none() {
                    errors.push(ResolveError::new(
                        "accepted HIR method call has no selected authority".to_string(),
                    ));
                }
            }
            HirExprKind::FieldAccess(_, _, _) => self.materialize_field_access(expr, errors),
            HirExprKind::Deref(receiver)
            | HirExprKind::Ref(_, receiver)
            | HirExprKind::Cast(receiver, _)
            | HirExprKind::TupleIndex(receiver, _) => self.materialize_expr(receiver, errors),
            HirExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.materialize_expr(condition, errors);
                self.materialize_block(then_branch, errors);
                if let Some(else_branch) = else_branch {
                    self.materialize_block(else_branch, errors);
                }
            }
            HirExprKind::Block(block) | HirExprKind::Loop(block) => {
                self.materialize_block(block, errors)
            }
            HirExprKind::While { condition, body } => {
                self.materialize_expr(condition, errors);
                self.materialize_block(body, errors);
            }
            HirExprKind::For { iter, body, .. } => {
                self.materialize_expr(iter, errors);
                self.materialize_block(body, errors);
            }
            HirExprKind::Lambda { body, .. } => self.materialize_block(body, errors),
            HirExprKind::Assign(left, right) | HirExprKind::BinOp(_, left, right) => {
                self.materialize_expr(left, errors);
                self.materialize_expr(right, errors);
            }
            HirExprKind::UnaryOp(_, inner) => self.materialize_expr(inner, errors),
            HirExprKind::ArrayRepeat(value, _) => self.materialize_expr(value, errors),
            HirExprKind::Intrinsic { args, .. }
            | HirExprKind::TupleLiteral(args)
            | HirExprKind::ArrayLiteral(args) => {
                for arg in args {
                    self.materialize_expr(arg, errors);
                }
            }
            HirExprKind::StructLiteral(_, _, fields) => {
                for field in fields {
                    self.materialize_expr(&mut field.value, errors);
                }
            }
            HirExprKind::EnumVariant(_, _, args, _) => {
                for arg in args {
                    self.materialize_expr(arg, errors);
                }
            }
            HirExprKind::Match { scrutinee, arms } => {
                self.materialize_expr(scrutinee, errors);
                for arm in arms {
                    if let Some(guard) = &mut arm.guard {
                        self.materialize_expr(guard, errors);
                    }
                    self.materialize_block(&mut arm.body, errors);
                }
            }
            HirExprKind::Range(start, end) => {
                self.materialize_expr(start, errors);
                self.materialize_expr(end, errors);
            }
            HirExprKind::Try { expr, .. } => self.materialize_expr(expr, errors),
            HirExprKind::Var(_)
            | HirExprKind::ResolvedVar(_)
            | HirExprKind::IntLiteral(_)
            | HirExprKind::FloatLiteral(_)
            | HirExprKind::BoolLiteral(_)
            | HirExprKind::StringLiteral(_)
            | HirExprKind::CharLiteral(_)
            | HirExprKind::Unit => {}
        }
    }

    fn materialize_field_access(&self, expr: &mut HirExpr, errors: &mut Vec<ResolveError>) {
        let HirExprKind::FieldAccess(receiver, field_name, location) = &mut expr.kind else {
            return;
        };
        self.materialize_expr(receiver, errors);
        if location.is_some() {
            return;
        }

        receiver.ty = self.engine.borrow().resolve(&receiver.ty);
        let receiver_ty = match &receiver.ty {
            Type::Reference { inner, .. } => inner.as_ref(),
            ty => ty,
        };
        let Type::Struct { id, args } = receiver_ty else {
            return;
        };
        let Some(structure) = self.structs.get(id) else {
            return;
        };
        let Some(field) = structure
            .fields
            .iter()
            .find(|field| field.name == *field_name)
        else {
            return;
        };
        if !field.public {
            if !self.strict {
                return;
            }
            errors.push(ResolveError::new(format!(
                "field '{}' of struct '{}' is private",
                field_name, structure.name
            )));
            return;
        }
        *location = Some(HirFieldLocation {
            owner: *id,
            field_id: field.id,
            name: field.name.clone(),
        });
        let substitution = structure
            .generic_params
            .iter()
            .enumerate()
            .filter_map(|(index, _)| {
                args.get(index).cloned().map(|ty| {
                    (
                        crate::types::GenericParamId {
                            owner: *id,
                            index: index as u32,
                        },
                        ty,
                    )
                })
            })
            .collect();
        expr.ty = field.ty.substitute_generics(&substitution);
    }
}

fn contains_recovery_type(ty: &Type) -> bool {
    let mut contains_recovery = false;
    crate::type_services::visit::visit_type(ty, &mut |nested: &Type| {
        contains_recovery |= matches!(nested, Type::TypeVar(_) | Type::Error);
    });
    contains_recovery
}
