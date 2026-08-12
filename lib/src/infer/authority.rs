use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use crate::hir::{
    HirBlock, HirCallTarget, HirClosureCapture, HirClosureCaptureKind, HirExpr, HirExprKind,
    HirFieldLocation, HirFunction, HirGenericBounds, HirStaticMethodTarget, HirStmt,
    HirTypeBinding, HirVarRef, HirVarTarget,
};
use crate::ids::{DefId, HirLocalId};
use crate::infer::{ConstraintStore, PartialHir};
use crate::language_items::TryLanguageItems;
use crate::lower::ResolveError;
use crate::selection::{
    type_pattern_matches, ReceiverAdjustment, ReceiverCandidate, SelectedMethod, SelectionService,
};
use crate::types::{
    CallableKind, CaptureKind, FunctionCapture, FunctionSafety, GenericParamId, TraitBound, Type,
};

pub(super) fn materialize_pending_authorities(
    hir: &mut PartialHir,
    strict: bool,
    try_strict: bool,
) -> Result<(), Vec<ResolveError>> {
    materialize_pending(hir, strict, try_strict)
}

pub(super) fn propagate_function_instances(hir: &mut PartialHir) -> Result<(), Vec<ResolveError>> {
    let monomorphic = hir
        .functions
        .iter()
        .filter_map(|(&id, function)| {
            let ambiguous = has_ambiguous_deferred_method(hir, &function.body);
            ambiguous.then_some(id)
        })
        .collect::<HashSet<_>>();
    let mut errors = Vec::new();
    let mut function_ids = hir.functions.keys().copied().collect::<Vec<_>>();
    function_ids.sort();
    for _ in 0..3 {
        for id in &function_ids {
            if let Some(mut body) = hir.functions.get(id).map(|function| function.body.clone()) {
                propagate_block(hir, &mut body, &monomorphic, &mut errors);
                if let Some(function) = hir.functions.get_mut(id) {
                    let _ = hir.engine.unify(&function.ret_type, &body.ty);
                    function.ret_type = hir.engine.resolve(&function.ret_type);
                    function.body = body;
                }
            }
        }
    }

    let mut impl_ids = hir.impls.keys().copied().collect::<Vec<_>>();
    impl_ids.sort();
    for impl_id in impl_ids {
        let mut method_names = hir.impls[&impl_id]
            .methods
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        method_names.sort();
        for method_name in method_names {
            if let Some(mut body) = hir
                .impls
                .get(&impl_id)
                .and_then(|imp| imp.methods.get(&method_name))
                .map(|method| method.body.clone())
            {
                propagate_block(hir, &mut body, &monomorphic, &mut errors);
                if let Some(method) = hir
                    .impls
                    .get_mut(&impl_id)
                    .and_then(|imp| imp.methods.get_mut(&method_name))
                {
                    method.body = body;
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

fn collect_type_vars(ty: &Type, vars: &mut HashSet<crate::ids::TypeVarId>) {
    crate::type_services::visit::visit_type(ty, &mut |nested: &Type| {
        if let Type::TypeVar(id) = nested {
            vars.insert(*id);
        }
    });
}

fn propagate_block(
    hir: &mut PartialHir,
    block: &mut HirBlock,
    monomorphic: &HashSet<DefId>,
    errors: &mut Vec<ResolveError>,
) {
    for stmt in &mut block.stmts {
        match stmt {
            HirStmt::Let { value, .. }
            | HirStmt::Expr(value)
            | HirStmt::Return(Some(value))
            | HirStmt::Break(Some(value)) => propagate_expr(hir, value, monomorphic, errors),
            HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => {}
        }
    }
    block.ty = hir.engine.resolve(&block.ty);
}

fn propagate_expr(
    hir: &mut PartialHir,
    expr: &mut HirExpr,
    monomorphic: &HashSet<DefId>,
    errors: &mut Vec<ResolveError>,
) {
    match &mut expr.kind {
        HirExprKind::Call(callee, args, _) => {
            // Establish the call-site scheme before descending into deferred
            // arguments so their method authority can use the parameter type.
            propagate_call_instance(hir, &expr.ty, callee, args, monomorphic, errors);
            propagate_expr(hir, callee, monomorphic, errors);
            for arg in args.iter_mut() {
                propagate_expr(hir, arg, monomorphic, errors);
            }
            propagate_call_instance(hir, &expr.ty, callee, args, monomorphic, errors);
        }
        HirExprKind::MethodCall(receiver, _, args, _, target) => {
            if let Some(target) = target.as_ref() {
                propagate_method_arguments(hir, receiver, args, target);
            }
            propagate_expr(hir, receiver, monomorphic, errors);
            for arg in args {
                propagate_expr(hir, arg, monomorphic, errors);
            }
            if let Some(target) = target {
                propagate_method_result(hir, expr.ty.clone(), target, errors);
            }
        }
        HirExprKind::Try { expr, .. } => propagate_expr(hir, expr, monomorphic, errors),
        HirExprKind::FieldAccess(receiver, _, _)
        | HirExprKind::Deref(receiver)
        | HirExprKind::Ref(_, receiver)
        | HirExprKind::Cast(receiver, _)
        | HirExprKind::TupleIndex(receiver, _) => {
            propagate_expr(hir, receiver, monomorphic, errors)
        }
        HirExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            propagate_expr(hir, condition, monomorphic, errors);
            propagate_block(hir, then_branch, monomorphic, errors);
            if let Some(else_branch) = else_branch {
                propagate_block(hir, else_branch, monomorphic, errors);
            }
        }
        HirExprKind::Match { scrutinee, arms } => {
            propagate_expr(hir, scrutinee, monomorphic, errors);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    propagate_expr(hir, guard, monomorphic, errors);
                }
                propagate_block(hir, &mut arm.body, monomorphic, errors);
            }
        }
        HirExprKind::While { condition, body } => {
            propagate_expr(hir, condition, monomorphic, errors);
            propagate_block(hir, body, monomorphic, errors);
        }
        HirExprKind::For { iter, body, .. } => {
            propagate_expr(hir, iter, monomorphic, errors);
            propagate_block(hir, body, monomorphic, errors);
        }
        HirExprKind::Block(body) | HirExprKind::Loop(body) | HirExprKind::UnsafeBlock(body) => {
            propagate_block(hir, body, monomorphic, errors)
        }
        HirExprKind::Lambda { body, .. } => propagate_block(hir, body, monomorphic, errors),
        HirExprKind::Assign(left, right) | HirExprKind::BinOp(_, left, right) => {
            propagate_expr(hir, left, monomorphic, errors);
            propagate_expr(hir, right, monomorphic, errors);
        }
        HirExprKind::UnaryOp(_, inner) | HirExprKind::ArrayRepeat(inner, _) => {
            propagate_expr(hir, inner, monomorphic, errors)
        }
        HirExprKind::Intrinsic { args, .. }
        | HirExprKind::TupleLiteral(args)
        | HirExprKind::ArrayLiteral(args) => {
            for arg in args {
                propagate_expr(hir, arg, monomorphic, errors);
            }
        }
        HirExprKind::StructLiteral(_, _, fields) => {
            for field in fields {
                propagate_expr(hir, &mut field.value, monomorphic, errors);
            }
        }
        HirExprKind::EnumVariant(_, _, args, _) => {
            for arg in args {
                propagate_expr(hir, arg, monomorphic, errors);
            }
        }
        HirExprKind::Range(start, end) => {
            propagate_expr(hir, start, monomorphic, errors);
            propagate_expr(hir, end, monomorphic, errors);
        }
        HirExprKind::Var(_)
        | HirExprKind::ResolvedVar(_)
        | HirExprKind::IntLiteral(_)
        | HirExprKind::FloatLiteral(_)
        | HirExprKind::BoolLiteral(_)
        | HirExprKind::StringLiteral(_)
        | HirExprKind::CharLiteral(_)
        | HirExprKind::Unit => {}
    }
    expr.ty = hir.engine.resolve(&expr.ty);
}

fn propagate_method_arguments(
    hir: &mut PartialHir,
    receiver: &HirExpr,
    args: &[HirExpr],
    target: &crate::hir::HirMethodCallTarget,
) {
    let Some(method) = find_method(hir, target.method_id()) else {
        return;
    };
    let substitutions = target
        .owner_substitution
        .iter()
        .chain(&target.method_substitution)
        .map(|binding| (binding.param, binding.ty.clone()))
        .collect::<HashMap<_, _>>();
    let (receiver_param, params) = if method.self_receiver.is_some() {
        (method.params.first(), &method.params[1..])
    } else {
        (None, &method.params[..])
    };
    if let Some(param) = receiver_param {
        let mut expected = param.ty.substitute_generics(&substitutions);
        if matches!(hir.engine.resolve(&receiver.ty), Type::TypeVar(_))
            && !method.ret_type.contains_reference()
        {
            if let Type::Reference { inner, .. } = expected {
                expected = *inner;
            }
        }
        let _ = hir.engine.unify(&receiver.ty, &expected);
    }
    for (arg, param) in args.iter().zip(params) {
        let expected = param.ty.substitute_generics(&substitutions);
        let _ = hir.engine.unify(&arg.ty, &expected);
    }
}

fn propagate_call_instance(
    hir: &mut PartialHir,
    result_ty: &Type,
    callee: &HirExpr,
    args: &mut [HirExpr],
    monomorphic: &HashSet<DefId>,
    _errors: &mut Vec<ResolveError>,
) {
    let HirExprKind::ResolvedVar(reference) = &callee.kind else {
        return;
    };
    let crate::hir::HirVarTarget::Function(function_id) = reference.target else {
        return;
    };
    let Some(function) = hir.functions.get(&function_id).cloned() else {
        return;
    };
    if function.is_method
        || hir
            .impls
            .values()
            .any(|imp| imp.methods.values().any(|method| method.id == function_id))
    {
        return;
    }
    if !function.generic_params.is_empty() {
        return;
    }
    if function.params.iter().any(|param| param.is_ref) {
        return;
    }
    if monomorphic.contains(&function_id) {
        let source_params = function
            .params
            .iter()
            .map(|param| param.ty.clone())
            .collect::<Vec<_>>();
        for (arg, expected) in args.iter().zip(source_params.iter()) {
            let _ = hir.engine.unify(&arg.ty, expected);
        }
        let source_type = Type::function_with_safety(
            function
                .params
                .iter()
                .map(|param| param.ty.clone())
                .collect(),
            function.ret_type.clone(),
            crate::types::FunctionSafety::from_is_unsafe(function.is_unsafe),
        );
        let _ = hir.engine.unify(&callee.ty, &source_type);
        return;
    }
    let source_params = function
        .params
        .iter()
        .map(|param| hir.engine.resolve(&param.ty))
        .collect::<Vec<_>>();
    let source_ret = hir.engine.resolve(&function.ret_type);
    let mut source_vars = HashSet::new();
    for ty in source_params.iter().chain(std::iter::once(&source_ret)) {
        crate::type_services::visit::visit_type(ty, &mut |nested: &Type| {
            if let Type::TypeVar(id) = nested {
                source_vars.insert(*id);
            }
        });
    }
    let fresh_substitution = source_vars
        .into_iter()
        .map(|id| {
            let kind = hir.engine.kind_of_type_var(id);
            (id, hir.engine.fresh_type_var_of_kind(kind))
        })
        .collect::<HashMap<_, _>>();
    let instance_params = source_params
        .iter()
        .map(|ty| ty.substitute(&fresh_substitution))
        .collect::<Vec<_>>();
    let instance_ret = source_ret.substitute(&fresh_substitution);
    let instance_type = Type::function_with_safety(
        instance_params,
        instance_ret,
        crate::types::FunctionSafety::from_is_unsafe(function.is_unsafe),
    );
    let _ = hir.engine.unify(&callee.ty, &instance_type);
    let Type::Function { params, ret, .. } = instance_type else {
        return;
    };
    let instance_ret = *ret;
    for (arg, expected) in args.iter().zip(params.iter()) {
        let _ = hir.engine.unify(&arg.ty, expected);
    }
    let result_type = if args.len() < params.len() {
        Type::function_with_safety(
            params[args.len()..].to_vec(),
            instance_ret.clone(),
            crate::types::FunctionSafety::from_is_unsafe(function.is_unsafe),
        )
    } else {
        instance_ret
    };
    let _ = hir.engine.unify(result_ty, &result_type);
}

fn propagate_method_result(
    hir: &mut PartialHir,
    result_ty: Type,
    target: &crate::hir::HirMethodCallTarget,
    errors: &mut Vec<ResolveError>,
) {
    let Some(method) = find_method(hir, target.method_id()) else {
        return;
    };
    let substitutions = target
        .owner_substitution
        .iter()
        .chain(&target.method_substitution)
        .map(|binding| (binding.param, binding.ty.clone()))
        .collect::<HashMap<_, _>>();
    let method_result = method.ret_type.substitute_generics(&substitutions);
    if !matches!(method_result, Type::Function { .. }) {
        return;
    }
    if let Err(error) = hir.engine.unify(&method_result, &result_ty) {
        errors.push(ResolveError::new(format!(
            "inferred method result for '{}' is incompatible with its solved scheme: {error}",
            method.name
        )));
    }
}

fn find_method(hir: &PartialHir, method_id: Option<DefId>) -> Option<HirFunction> {
    let method_id = method_id?;
    hir.impls
        .values()
        .flat_map(|imp| imp.methods.values())
        .find(|method| method.id == method_id)
        .cloned()
        .or_else(|| {
            hir.traits
                .values()
                .flat_map(|trait_def| trait_def.methods.values())
                .find(|method| method.id == method_id)
                .cloned()
        })
}

fn has_ambiguous_deferred_method(hir: &PartialHir, block: &HirBlock) -> bool {
    block.stmts.iter().any(|stmt| match stmt {
        HirStmt::Let { value, .. }
        | HirStmt::Expr(value)
        | HirStmt::Return(Some(value))
        | HirStmt::Break(Some(value)) => has_ambiguous_deferred_method_expr(hir, value),
        HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => false,
    })
}

fn has_ambiguous_deferred_method_expr(hir: &PartialHir, expr: &HirExpr) -> bool {
    let direct = match &expr.kind {
        HirExprKind::Call(callee, _, None) => match &callee.kind {
            HirExprKind::FieldAccess(receiver, method_name, None)
                if contains_recovery_type(&receiver.ty) =>
            {
                let bounds = HirGenericBounds::new();
                let selected = SelectionService::new(
                    &hir.traits,
                    &hir.impls,
                    hir.language_items
                        .sized
                        .as_ref()
                        .map(|items| items.trait_id),
                    None,
                    &bounds,
                )
                .with_effective_trait_methods(&hir.imported_effective_trait_methods)
                .select_inferred_method_candidates(receiver, method_name, |ty| ty.clone());
                selected.len() > 1
            }
            _ => false,
        },
        _ => false,
    };
    if direct {
        return true;
    }

    match &expr.kind {
        HirExprKind::Call(callee, args, _) => {
            has_ambiguous_deferred_method_expr(hir, callee)
                || args
                    .iter()
                    .any(|arg| has_ambiguous_deferred_method_expr(hir, arg))
        }
        HirExprKind::MethodCall(receiver, _, args, _, _) => {
            has_ambiguous_deferred_method_expr(hir, receiver)
                || args
                    .iter()
                    .any(|arg| has_ambiguous_deferred_method_expr(hir, arg))
        }
        HirExprKind::Try { expr, .. }
        | HirExprKind::FieldAccess(expr, _, _)
        | HirExprKind::Deref(expr)
        | HirExprKind::Ref(_, expr)
        | HirExprKind::Cast(expr, _)
        | HirExprKind::TupleIndex(expr, _) => has_ambiguous_deferred_method_expr(hir, expr),
        HirExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            has_ambiguous_deferred_method_expr(hir, condition)
                || has_ambiguous_deferred_method(hir, then_branch)
                || else_branch
                    .as_ref()
                    .is_some_and(|branch| has_ambiguous_deferred_method(hir, branch))
        }
        HirExprKind::Match { scrutinee, arms } => {
            has_ambiguous_deferred_method_expr(hir, scrutinee)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|guard| has_ambiguous_deferred_method_expr(hir, guard))
                        || has_ambiguous_deferred_method(hir, &arm.body)
                })
        }
        HirExprKind::While { condition, body } => {
            has_ambiguous_deferred_method_expr(hir, condition)
                || has_ambiguous_deferred_method(hir, body)
        }
        HirExprKind::For { iter, body, .. } => {
            has_ambiguous_deferred_method_expr(hir, iter)
                || has_ambiguous_deferred_method(hir, body)
        }
        HirExprKind::Block(body) | HirExprKind::Loop(body) | HirExprKind::UnsafeBlock(body) => {
            has_ambiguous_deferred_method(hir, body)
        }
        HirExprKind::Lambda { body, .. } => has_ambiguous_deferred_method(hir, body),
        HirExprKind::Assign(left, right) | HirExprKind::BinOp(_, left, right) => {
            has_ambiguous_deferred_method_expr(hir, left)
                || has_ambiguous_deferred_method_expr(hir, right)
        }
        HirExprKind::UnaryOp(_, inner) | HirExprKind::ArrayRepeat(inner, _) => {
            has_ambiguous_deferred_method_expr(hir, inner)
        }
        HirExprKind::Intrinsic { args, .. }
        | HirExprKind::TupleLiteral(args)
        | HirExprKind::ArrayLiteral(args)
        | HirExprKind::EnumVariant(_, _, args, _) => args
            .iter()
            .any(|arg| has_ambiguous_deferred_method_expr(hir, arg)),
        HirExprKind::StructLiteral(_, _, fields) => fields
            .iter()
            .any(|field| has_ambiguous_deferred_method_expr(hir, &field.value)),
        HirExprKind::Range(start, end) => {
            has_ambiguous_deferred_method_expr(hir, start)
                || has_ambiguous_deferred_method_expr(hir, end)
        }
        HirExprKind::Var(_)
        | HirExprKind::ResolvedVar(_)
        | HirExprKind::IntLiteral(_)
        | HirExprKind::FloatLiteral(_)
        | HirExprKind::BoolLiteral(_)
        | HirExprKind::StringLiteral(_)
        | HirExprKind::CharLiteral(_)
        | HirExprKind::Unit => false,
    }
}

fn materialize_pending(
    hir: &mut PartialHir,
    strict: bool,
    try_strict: bool,
) -> Result<(), Vec<ResolveError>> {
    let struct_ids = hir
        .structs
        .values()
        .map(|structure| (structure.id, structure.clone()))
        .collect();
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
    let functions = hir.functions.clone();
    let mut context = MethodAuthorityContext {
        functions,
        traits: &traits,
        impls: &impls,
        sized_trait_id: hir
            .language_items
            .sized
            .as_ref()
            .map(|items| items.trait_id),
        bounds: HirGenericBounds::new(),
        constraint_store: &mut hir.constraint_store,
        effective_trait_methods: &hir.imported_effective_trait_methods,
        structs: &struct_ids,
        engine: RefCell::new(&mut hir.engine),
        try_protocol: hir.language_items.try_protocol.clone(),
        unsafe_context: Cell::new(false),
        mutable_locals: HashSet::new(),
        local_bindings: HashMap::new(),
        next_local_id: 0,
        strict,
        try_strict,
    };

    let mut errors = Vec::new();
    let mut function_ids = hir.functions.keys().copied().collect::<Vec<_>>();
    function_ids.sort();
    for id in function_ids {
        if let Some(function) = hir.functions.get_mut(&id) {
            context.materialize_function(function, &mut errors);
            context.functions.insert(id, function.clone());
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
    functions: HashMap<DefId, HirFunction>,
    traits: &'a HashMap<DefId, crate::hir::HirTrait>,
    impls: &'a HashMap<DefId, crate::hir::HirImpl>,
    sized_trait_id: Option<DefId>,
    bounds: HirGenericBounds,
    constraint_store: &'a mut ConstraintStore,
    effective_trait_methods: &'a HashMap<(DefId, DefId), DefId>,
    structs: &'a HashMap<DefId, crate::hir::HirStruct>,
    engine: RefCell<&'a mut crate::infer::InferenceEngine>,
    try_protocol: Option<TryLanguageItems<DefId>>,
    unsafe_context: Cell<bool>,
    mutable_locals: HashSet<HirLocalId>,
    local_bindings: HashMap<HirLocalId, (String, Type, bool)>,
    next_local_id: u32,
    strict: bool,
    try_strict: bool,
}

impl MethodAuthorityContext<'_> {
    fn service(&self) -> SelectionService<'_> {
        SelectionService::new(
            self.traits,
            self.impls,
            self.sized_trait_id,
            None,
            &self.bounds,
        )
        .with_effective_trait_methods(self.effective_trait_methods)
    }

    fn materialize_function(&mut self, function: &mut HirFunction, errors: &mut Vec<ResolveError>) {
        self.bounds = function.generic_bounds.clone();
        self.mutable_locals.clear();
        self.local_bindings.clear();
        self.next_local_id = 0;
        self.collect_local_bindings(function);
        let previous = self.unsafe_context.replace(function.is_unsafe);
        self.materialize_block(&mut function.body, errors);
        let _ = self
            .engine
            .borrow_mut()
            .unify(&function.ret_type, &function.body.ty);
        function.ret_type = self.resolved_type(&function.ret_type);
        function.body.ty = self.resolved_type(&function.body.ty);
        self.unsafe_context.set(previous);
    }

    fn materialize_direct_function_call(
        &mut self,
        result_ty: &Type,
        callee: &HirExpr,
        args: &[HirExpr],
        function_id: DefId,
    ) {
        let Some(function) = self.functions.get(&function_id).cloned() else {
            return;
        };
        if function.is_method
            || !function.generic_params.is_empty()
            || function.params.iter().any(|param| param.is_ref)
        {
            return;
        }
        let source_params = function
            .params
            .iter()
            .map(|param| self.resolved_type(&param.ty))
            .collect::<Vec<_>>();
        let source_ret = self.resolved_type(&function.ret_type);
        let mut vars = HashSet::new();
        for ty in source_params.iter().chain(std::iter::once(&source_ret)) {
            collect_type_vars(ty, &mut vars);
        }
        let substitutions = vars
            .into_iter()
            .map(|id| {
                let kind = self.engine.borrow().kind_of_type_var(id);
                let fresh = self.engine.borrow_mut().fresh_type_var_of_kind(kind);
                (id, fresh)
            })
            .collect::<HashMap<_, _>>();
        let params = source_params
            .iter()
            .map(|ty| ty.substitute(&substitutions))
            .collect::<Vec<_>>();
        let ret = source_ret.substitute(&substitutions);
        let instance = Type::function_with_safety(
            params.clone(),
            ret.clone(),
            FunctionSafety::from_is_unsafe(function.is_unsafe),
        );
        let mut engine = self.engine.borrow_mut();
        let _ = engine.unify(&callee.ty, &instance);
        for (arg, expected) in args.iter().zip(params.iter()) {
            let _ = engine.unify(&arg.ty, expected);
        }
        let result = if args.len() < params.len() {
            Type::function_with_safety(
                params[args.len()..].to_vec(),
                ret,
                FunctionSafety::from_is_unsafe(function.is_unsafe),
            )
        } else {
            ret
        };
        let _ = engine.unify(result_ty, &result);
    }

    fn materialize_block(&mut self, block: &mut HirBlock, errors: &mut Vec<ResolveError>) {
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

    fn collect_local_bindings(&mut self, function: &HirFunction) {
        for param in &function.params {
            self.record_local_binding(param.local_id, &param.name, param.ty.clone(), param.mutable);
        }
        self.collect_local_bindings_block(&function.body);
    }

    fn record_local_binding(&mut self, local_id: HirLocalId, name: &str, ty: Type, mutable: bool) {
        self.next_local_id = self.next_local_id.max(local_id.0.saturating_add(1));
        if mutable {
            self.mutable_locals.insert(local_id);
        }
        self.local_bindings
            .insert(local_id, (name.to_string(), ty, mutable));
    }

    fn collect_local_bindings_block(&mut self, block: &HirBlock) {
        for stmt in &block.stmts {
            match stmt {
                HirStmt::Let {
                    name,
                    local_id,
                    ty,
                    value,
                    mutable,
                } => {
                    self.record_local_binding(*local_id, name, ty.clone(), *mutable);
                    self.collect_local_bindings_expr(value);
                }
                HirStmt::Expr(expr) | HirStmt::Return(Some(expr)) | HirStmt::Break(Some(expr)) => {
                    self.collect_local_bindings_expr(expr)
                }
                HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => {}
            }
        }
    }

    fn collect_local_bindings_expr(&mut self, expr: &HirExpr) {
        match &expr.kind {
            HirExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_local_bindings_expr(condition);
                self.collect_local_bindings_block(then_branch);
                if let Some(else_branch) = else_branch {
                    self.collect_local_bindings_block(else_branch);
                }
            }
            HirExprKind::Match { scrutinee, arms } => {
                self.collect_local_bindings_expr(scrutinee);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.collect_local_bindings_expr(guard);
                    }
                    self.collect_local_bindings_block(&arm.body);
                }
            }
            HirExprKind::While { condition, body } => {
                self.collect_local_bindings_expr(condition);
                self.collect_local_bindings_block(body);
            }
            HirExprKind::For { iter, body, .. } => {
                self.collect_local_bindings_expr(iter);
                self.collect_local_bindings_block(body);
            }
            HirExprKind::Block(body) | HirExprKind::Loop(body) | HirExprKind::UnsafeBlock(body) => {
                self.collect_local_bindings_block(body)
            }
            HirExprKind::Lambda { params, body, .. } => {
                for param in params {
                    self.record_local_binding(
                        param.local_id,
                        &param.name,
                        param.ty.clone(),
                        param.mutable,
                    );
                }
                self.collect_local_bindings_block(body);
            }
            HirExprKind::Call(callee, args, _) | HirExprKind::MethodCall(callee, _, args, _, _) => {
                self.collect_local_bindings_expr(callee);
                for arg in args {
                    self.collect_local_bindings_expr(arg);
                }
            }
            HirExprKind::Try { expr, .. }
            | HirExprKind::FieldAccess(expr, _, _)
            | HirExprKind::Deref(expr)
            | HirExprKind::Ref(_, expr)
            | HirExprKind::Cast(expr, _)
            | HirExprKind::TupleIndex(expr, _) => self.collect_local_bindings_expr(expr),
            HirExprKind::Assign(left, right) | HirExprKind::BinOp(_, left, right) => {
                self.collect_local_bindings_expr(left);
                self.collect_local_bindings_expr(right);
            }
            HirExprKind::UnaryOp(_, inner) | HirExprKind::ArrayRepeat(inner, _) => {
                self.collect_local_bindings_expr(inner)
            }
            HirExprKind::Intrinsic { args, .. }
            | HirExprKind::TupleLiteral(args)
            | HirExprKind::ArrayLiteral(args)
            | HirExprKind::EnumVariant(_, _, args, _) => {
                for arg in args {
                    self.collect_local_bindings_expr(arg);
                }
            }
            HirExprKind::StructLiteral(_, _, fields) => {
                for field in fields {
                    self.collect_local_bindings_expr(&field.value);
                }
            }
            HirExprKind::Range(start, end) => {
                self.collect_local_bindings_expr(start);
                self.collect_local_bindings_expr(end);
            }
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

    fn resolved_type(&self, ty: &Type) -> Type {
        self.engine.borrow().resolve(ty)
    }

    fn expr_can_autoref_mut_receiver(&self, expr: &HirExpr) -> bool {
        match &expr.kind {
            HirExprKind::ResolvedVar(HirVarRef {
                target: HirVarTarget::Local(local_id),
                ..
            }) => {
                self.mutable_locals.contains(local_id)
                    && !matches!(self.resolved_type(&expr.ty), Type::Reference { .. })
            }
            HirExprKind::FieldAccess(base, _, _) | HirExprKind::TupleIndex(base, _) => {
                self.expr_can_autoref_mut_receiver(base)
            }
            HirExprKind::Deref(inner) => matches!(
                self.resolved_type(&inner.ty),
                Type::Reference { mutable: true, .. }
            ),
            _ => false,
        }
    }

    fn is_borrowable_local_expr(expr: &HirExpr) -> bool {
        matches!(
            expr.kind,
            HirExprKind::FieldAccess(_, _, _)
                | HirExprKind::TupleIndex(_, _)
                | HirExprKind::ResolvedVar(HirVarRef {
                    target: HirVarTarget::Local(_),
                    ..
                })
        )
    }

    fn push_receiver_candidate(
        &self,
        candidates: &mut Vec<ReceiverCandidate>,
        seen: &mut HashSet<Type>,
        expr: HirExpr,
        adjustment: ReceiverAdjustment,
        can_autoref_mut: bool,
    ) {
        let resolved = self.resolved_type(&expr.ty);
        if seen.insert(resolved) {
            candidates.push(ReceiverCandidate {
                expr,
                adjustment,
                can_autoref_mut,
            });
        }
    }

    fn builtin_shared_ref_deref(&self, expr: HirExpr) -> Option<HirExpr> {
        let Type::Reference {
            mutable: false,
            inner,
        } = self.resolved_type(&expr.ty)
        else {
            return None;
        };
        Some(HirExpr {
            ty: *inner,
            kind: HirExprKind::Deref(Box::new(expr.clone())),
            span: expr.span,
        })
    }

    fn builtin_mut_ref_deref(&self, expr: HirExpr) -> Option<HirExpr> {
        let Type::Reference {
            mutable: true,
            inner,
        } = self.resolved_type(&expr.ty)
        else {
            return None;
        };
        Some(HirExpr {
            ty: *inner,
            kind: HirExprKind::Deref(Box::new(expr.clone())),
            span: expr.span,
        })
    }

    fn mut_ref_to_shared_ref(&self, expr: HirExpr) -> Option<HirExpr> {
        let Type::Reference {
            mutable: true,
            inner,
        } = self.resolved_type(&expr.ty)
        else {
            return None;
        };
        Some(HirExpr {
            ty: Type::Reference {
                mutable: false,
                inner,
            },
            kind: expr.kind,
            span: expr.span,
        })
    }

    fn array_ref_to_slice_ref(&self, expr: HirExpr, mutable: bool) -> Option<HirExpr> {
        let Type::Reference { inner, .. } = self.resolved_type(&expr.ty) else {
            return None;
        };
        let Type::Array(element, _) = inner.as_ref() else {
            return None;
        };
        Some(HirExpr {
            ty: Type::Reference {
                mutable,
                inner: Box::new(Type::Slice(element.clone())),
            },
            kind: HirExprKind::Intrinsic {
                name: "ArrayRefToSlice".to_string(),
                args: vec![expr],
            },
            span: Default::default(),
        })
    }

    fn array_value_to_slice_ref(&self, expr: HirExpr, mutable: bool) -> Option<HirExpr> {
        if !Self::is_borrowable_local_expr(&expr) {
            return None;
        }
        let Type::Array(element, _) = self.resolved_type(&expr.ty) else {
            return None;
        };
        let borrowed = HirExpr {
            ty: Type::Reference {
                mutable,
                inner: Box::new(expr.ty.clone()),
            },
            kind: HirExprKind::Ref(mutable, Box::new(expr)),
            span: Default::default(),
        };
        Some(HirExpr {
            ty: Type::Reference {
                mutable,
                inner: Box::new(Type::Slice(element)),
            },
            kind: HirExprKind::Intrinsic {
                name: "ArrayRefToSlice".to_string(),
                args: vec![borrowed],
            },
            span: Default::default(),
        })
    }

    fn apply_receiver_adjustment(&self, expr: HirExpr, adjustment: ReceiverAdjustment) -> HirExpr {
        match adjustment {
            ReceiverAdjustment::AutorefShared | ReceiverAdjustment::AutorefMut => {
                let resolved = self.resolved_type(&expr.ty);
                if !matches!(resolved, Type::Str | Type::Slice(_)) {
                    return expr;
                }
                let mutable = matches!(adjustment, ReceiverAdjustment::AutorefMut);
                HirExpr {
                    ty: Type::Reference {
                        mutable,
                        inner: Box::new(expr.ty.clone()),
                    },
                    kind: HirExprKind::Ref(mutable, Box::new(expr)),
                    span: Default::default(),
                }
            }
            ReceiverAdjustment::MutToSharedRef => {
                self.mut_ref_to_shared_ref(expr.clone()).unwrap_or(expr)
            }
            ReceiverAdjustment::None
            | ReceiverAdjustment::BuiltinDeref
            | ReceiverAdjustment::TraitDeref
            | ReceiverAdjustment::ArrayRefToSliceRef
            | ReceiverAdjustment::ArrayValueToMutSliceRef
            | ReceiverAdjustment::ArrayValueToSliceRef
            | ReceiverAdjustment::ArrayValueToSliceValue => expr,
        }
    }

    fn trait_deref_candidate(&mut self, expr: HirExpr) -> Option<HirExpr> {
        let can_autoref_mut = self.expr_can_autoref_mut_receiver(&expr);
        let candidate = ReceiverCandidate {
            expr: HirExpr {
                ty: self.resolved_type(&expr.ty),
                ..expr.clone()
            },
            adjustment: ReceiverAdjustment::None,
            can_autoref_mut,
        };
        let mut selected = self.service().select_concrete_method_candidates(
            std::slice::from_ref(&candidate),
            "*",
            |ty| self.resolved_type(ty),
        );
        if can_autoref_mut {
            let has_mutable = selected.iter().any(|candidate| {
                candidate.function.as_ref().is_some_and(|function| {
                    function.self_receiver == Some(crate::types::ReceiverMode::Mut)
                })
            });
            if has_mutable {
                selected.retain(|candidate| {
                    candidate.function.as_ref().is_some_and(|function| {
                        function.self_receiver == Some(crate::types::ReceiverMode::Mut)
                    })
                });
            }
        }
        if selected.len() != 1 {
            return None;
        }
        let selected = selected.pop().expect("one deref candidate");
        let function = selected.function.clone()?;
        let return_type = self.resolved_type(&selected.return_type);
        let Type::Reference { inner, .. } = &return_type else {
            return None;
        };
        self.record_selected_bounds(&selected, &HashMap::new(), "trait deref");
        let receiver =
            self.apply_receiver_adjustment(selected.receiver.clone(), selected.receiver_adjustment);
        let target =
            selected.target_with_substitution(&HashMap::new(), |ty| self.resolved_type(ty));
        Some(HirExpr {
            ty: *inner.clone(),
            kind: HirExprKind::Deref(Box::new(HirExpr {
                ty: return_type,
                kind: HirExprKind::MethodCall(
                    Box::new(receiver),
                    function.name,
                    Vec::new(),
                    function.self_receiver,
                    Some(target),
                ),
                span: expr.span.clone(),
            })),
            span: expr.span,
        })
    }

    fn receiver_adjustment_candidates(&mut self, expr: HirExpr) -> Vec<ReceiverCandidate> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        let can_autoref_mut = self.expr_can_autoref_mut_receiver(&expr);
        self.push_receiver_candidate(
            &mut candidates,
            &mut seen,
            expr.clone(),
            ReceiverAdjustment::None,
            can_autoref_mut,
        );
        if let Some(mut_slice) = self.array_value_to_slice_ref(expr.clone(), true) {
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                mut_slice,
                ReceiverAdjustment::ArrayValueToMutSliceRef,
                false,
            );
        }
        if let Some(deref) = self.builtin_shared_ref_deref(expr.clone()) {
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                deref,
                ReceiverAdjustment::BuiltinDeref,
                false,
            );
        }
        if let Some(deref) = self.builtin_mut_ref_deref(expr.clone()) {
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                deref,
                ReceiverAdjustment::BuiltinDeref,
                true,
            );
        }
        if let Some(shared_ref) = self.mut_ref_to_shared_ref(expr.clone()) {
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                shared_ref.clone(),
                ReceiverAdjustment::MutToSharedRef,
                false,
            );
            if let Some(deref) = self.builtin_shared_ref_deref(shared_ref.clone()) {
                self.push_receiver_candidate(
                    &mut candidates,
                    &mut seen,
                    deref,
                    ReceiverAdjustment::BuiltinDeref,
                    false,
                );
            }
            if let Some(slice_ref) = self.array_ref_to_slice_ref(shared_ref, false) {
                self.push_receiver_candidate(
                    &mut candidates,
                    &mut seen,
                    slice_ref.clone(),
                    ReceiverAdjustment::ArrayRefToSliceRef,
                    false,
                );
                if let Some(slice_value) = self.builtin_shared_ref_deref(slice_ref) {
                    self.push_receiver_candidate(
                        &mut candidates,
                        &mut seen,
                        slice_value,
                        ReceiverAdjustment::BuiltinDeref,
                        false,
                    );
                }
            }
        }
        if !contains_recovery_type(&self.resolved_type(&expr.ty)) {
            if let Some(deref) = self.trait_deref_candidate(expr.clone()) {
                self.push_receiver_candidate(
                    &mut candidates,
                    &mut seen,
                    deref,
                    ReceiverAdjustment::TraitDeref,
                    can_autoref_mut,
                );
            }
        }
        if let Some(slice_ref) = self.array_ref_to_slice_ref(expr.clone(), false) {
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                slice_ref,
                ReceiverAdjustment::ArrayRefToSliceRef,
                false,
            );
        }
        if let Some(slice_ref) = self.array_value_to_slice_ref(expr.clone(), false) {
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                slice_ref,
                ReceiverAdjustment::ArrayValueToSliceRef,
                false,
            );
        }
        if let Some(slice_ref) = self.array_value_to_slice_ref(expr, false) {
            let slice_value = HirExpr {
                ty: match self.resolved_type(&slice_ref.ty) {
                    Type::Reference { inner, .. } => *inner,
                    ty => ty,
                },
                kind: HirExprKind::Deref(Box::new(slice_ref)),
                span: Default::default(),
            };
            self.push_receiver_candidate(
                &mut candidates,
                &mut seen,
                slice_value,
                ReceiverAdjustment::ArrayValueToSliceValue,
                false,
            );
        }
        candidates
    }

    fn record_selected_bounds(
        &mut self,
        selected: &SelectedMethod,
        substitution: &HashMap<GenericParamId, Type>,
        context: &str,
    ) {
        for (ty, bound) in &selected.pending_impl_bounds {
            self.record_trait_bound(
                ty.substitute_generics(substitution),
                TraitBound {
                    trait_id: bound.trait_id,
                    type_args: bound
                        .type_args
                        .iter()
                        .map(|arg| arg.substitute_generics(substitution))
                        .collect(),
                },
                selected.receiver.span.clone(),
                context,
            );
        }
        if let Some(function) = &selected.function {
            for (generic_param, bounds) in &function.generic_bounds {
                let subject = substitution
                    .get(generic_param)
                    .cloned()
                    .unwrap_or(Type::Generic(*generic_param));
                for bound in bounds {
                    self.record_trait_bound(
                        subject.clone(),
                        TraitBound {
                            trait_id: bound.trait_id,
                            type_args: bound
                                .type_args
                                .iter()
                                .map(|arg| arg.substitute_generics(substitution))
                                .collect(),
                        },
                        selected.receiver.span.clone(),
                        context,
                    );
                }
            }
        }
    }

    fn record_trait_bound(
        &mut self,
        ty: Type,
        bound: TraitBound,
        span: crate::lexer::Span,
        context: &str,
    ) {
        let ty = self.resolved_type(&ty);
        if let Type::TypeVar(id) = ty {
            self.engine.borrow_mut().add_bound(id, bound.clone());
            self.constraint_store
                .add_trait(Type::TypeVar(id), bound, span, context);
        } else {
            self.constraint_store.add_trait(ty, bound, span, context);
        }
    }

    fn select_deferred_method(
        &mut self,
        receiver: &HirExpr,
        method_name: &str,
        _expected_ty: Option<&Type>,
        errors: &mut Vec<ResolveError>,
    ) -> Option<SelectedMethod> {
        let receiver_candidates = self.receiver_adjustment_candidates(receiver.clone());
        if let Type::TypeVar(id) = self.resolved_type(&receiver.ty) {
            let bounds = self.engine.borrow().get_bounds(id);
            if !bounds.is_empty() {
                if let Ok(selected) = self
                    .service()
                    .select_bound_method_preferring_non_ref_receiver(
                        &receiver_candidates,
                        &bounds,
                        method_name,
                        Type::TypeVar(id),
                        matches!(receiver.kind, HirExprKind::Call(_, _, _)),
                    )
                {
                    return Some(selected);
                }
            }
        }
        if let Type::Generic(param) = self.resolved_type(&receiver.ty) {
            let bounds = self.bounds.get(&param).cloned().unwrap_or_default();
            if !bounds.is_empty() {
                if let Ok(selected) = self
                    .service()
                    .select_bound_method_preferring_non_ref_receiver(
                        &receiver_candidates,
                        &bounds,
                        method_name,
                        Type::Generic(param),
                        matches!(receiver.kind, HirExprKind::Call(_, _, _)),
                    )
                {
                    return Some(selected);
                }
            }
        }
        let mut deferred = None;
        for candidate in &receiver_candidates {
            let mut selected = self.service().select_concrete_method_candidates(
                std::slice::from_ref(candidate),
                method_name,
                |ty| self.resolved_type(ty),
            );
            let has_proven = selected
                .iter()
                .any(|candidate| candidate.pending_impl_bounds.is_empty());
            if has_proven {
                selected.retain(|candidate| candidate.pending_impl_bounds.is_empty());
            }
            match selected.len() {
                0 => {}
                1 if selected[0].pending_impl_bounds.is_empty() => {
                    let mut selected = selected.pop();
                    if let Some(selected) = selected.as_mut() {
                        if candidate.adjustment != ReceiverAdjustment::None {
                            selected.receiver_adjustment = candidate.adjustment;
                        }
                    }
                    return selected;
                }
                1 => {
                    if deferred.is_none() {
                        deferred = selected.pop();
                    }
                }
                _ => {
                    if !self.strict && contains_recovery_type(&candidate.expr.ty) {
                        continue;
                    }
                    errors.push(ResolveError::new(format!(
                        "Ambiguous selection for '{}' on type {}",
                        method_name, candidate.expr.ty
                    )));
                    return None;
                }
            }
        }
        if deferred.is_none()
            && (contains_recovery_type(&self.resolved_type(&receiver.ty))
                || matches!(self.resolved_type(&receiver.ty), Type::Generic(_)))
        {
            let mut inferred =
                self.service()
                    .select_inferred_method_candidates(receiver, method_name, |ty| {
                        self.resolved_type(ty)
                    });
            if let Some(expected) = _expected_ty.map(|ty| self.resolved_type(ty)) {
                if !contains_recovery_type(&expected) {
                    inferred.retain(|candidate| {
                        let mut substitution = candidate.owner_substitution.clone();
                        type_pattern_matches(&candidate.return_type, &expected, &mut substitution)
                    });
                }
            }
            inferred.sort_by_key(|candidate| {
                (candidate.target.impl_id(), candidate.target.method_id())
            });
            inferred.dedup_by_key(|candidate| {
                (candidate.target.impl_id(), candidate.target.method_id())
            });
            if inferred.len() == 1 {
                let mut selected = inferred.into_iter().next();
                if let Some(selected) = selected.as_mut() {
                    let target_key = (selected.target.impl_id(), selected.target.method_id());
                    for candidate in &receiver_candidates {
                        let concrete = self.service().select_concrete_method_candidates(
                            std::slice::from_ref(candidate),
                            method_name,
                            |ty| self.resolved_type(ty),
                        );
                        if concrete.iter().any(|candidate| {
                            (candidate.target.impl_id(), candidate.target.method_id()) == target_key
                        }) {
                            if candidate.adjustment != ReceiverAdjustment::None {
                                selected.receiver_adjustment = candidate.adjustment;
                            }
                            break;
                        }
                    }
                    selected.receiver.kind = receiver.kind.clone();
                    selected.receiver.span = receiver.span.clone();
                }
                return selected;
            }
            if inferred.len() > 1 {
                if !self.strict && contains_recovery_type(&receiver.ty) {
                    return None;
                }
                if let Some(expected) = _expected_ty.map(|ty| self.resolved_type(ty)) {
                    if !contains_recovery_type(&expected) {
                        let narrowed = inferred
                            .iter()
                            .filter(|candidate| {
                                let mut substitution = candidate.owner_substitution.clone();
                                type_pattern_matches(
                                    &candidate.return_type,
                                    &expected,
                                    &mut substitution,
                                )
                            })
                            .count();
                        if narrowed == 0 {
                            return None;
                        }
                    }
                }
                errors.push(ResolveError::new(format!(
                    "Ambiguous selection for '{}' on type {}",
                    method_name, receiver.ty
                )));
                return None;
            }
        }
        if deferred.is_some() {
            return deferred;
        }
        if self
            .service()
            .mut_receiver_method_requires_mutable_receiver(
                &receiver_candidates,
                method_name,
                |ty| self.resolved_type(ty),
            )
        {
            errors.push(ResolveError::new(format!(
                "Cannot call mutable receiver method '{}' without a mutable receiver",
                method_name
            )));
        }
        None
    }

    fn materialize_expr(&mut self, expr: &mut HirExpr, errors: &mut Vec<ResolveError>) {
        match &mut expr.kind {
            HirExprKind::Call(callee, args, target) => {
                for arg in args.iter_mut() {
                    self.materialize_expr(arg, errors);
                }
                if target.is_some() {
                    if let Some(crate::hir::HirCallTarget::Function(function_id)) = target.as_ref()
                    {
                        self.materialize_direct_function_call(&expr.ty, callee, args, *function_id);
                    }
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
                let Some(mut selected) =
                    self.select_deferred_method(&receiver, &method_name, Some(&expr.ty), errors)
                else {
                    if !self.strict && contains_recovery_type(&receiver.ty) {
                        return;
                    }
                    self.materialize_field_access(callee, errors);
                    return;
                };
                selected.receiver = receiver.clone();
                let adjusted_receiver = match selected.receiver_adjustment {
                    ReceiverAdjustment::TraitDeref => self
                        .trait_deref_candidate(receiver.clone())
                        .unwrap_or_else(|| receiver.clone()),
                    adjustment => self.apply_receiver_adjustment(receiver.clone(), adjustment),
                };

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
                self.record_selected_bounds(&selected, &substitution, "deferred method call");
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
                    Box::new(adjusted_receiver),
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
            HirExprKind::UnsafeBlock(block) => {
                let previous = self.unsafe_context.replace(true);
                self.materialize_block(block, errors);
                self.unsafe_context.set(previous);
            }
            HirExprKind::While { condition, body } => {
                self.materialize_expr(condition, errors);
                self.materialize_block(body, errors);
            }
            HirExprKind::For { iter, body, .. } => {
                self.materialize_expr(iter, errors);
                self.materialize_block(body, errors);
            }
            HirExprKind::Lambda { body, .. } => {
                let previous = self.unsafe_context.replace(false);
                self.materialize_block(body, errors);
                self.unsafe_context.set(previous);
            }
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
            HirExprKind::Try { .. } => self.materialize_try(expr, errors),
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

    fn materialize_try(&mut self, expr: &mut HirExpr, errors: &mut Vec<ResolveError>) {
        let HirExprKind::Try {
            expr: operand,
            branch_method,
            branch_self_receiver,
            from_residual_target,
            output_ty,
            residual_ty,
            return_ty,
            ..
        } = &mut expr.kind
        else {
            unreachable!("Try materialization called for a non-Try expression");
        };

        self.materialize_expr(operand, errors);
        let Some(protocol) = self.try_protocol.as_ref() else {
            if self.try_strict && self.strict {
                errors.push(ResolveError::new(
                    "Cannot use '?' because the Try language-item protocol is unavailable"
                        .to_string(),
                ));
            }
            return;
        };
        let Some(try_trait) = self.traits.get(&protocol.try_trait_id) else {
            if self.try_strict && self.strict {
                errors.push(ResolveError::new(
                    "Cannot use '?' because the Try language-item protocol is unavailable"
                        .to_string(),
                ));
            }
            return;
        };

        if branch_method.is_none() {
            let carrier_ty = self.engine.borrow().resolve(&operand.ty);
            if try_carrier_has_unknown_head(&carrier_ty) {
                if self.try_strict {
                    errors.push(ResolveError::new(
                        "Cannot use '?' because its carrier type is unresolved or generic"
                            .to_string(),
                    ));
                }
                return;
            }

            let receiver = operand.as_ref().clone();
            let selected = match SelectionService::new(
                self.traits,
                self.impls,
                self.sized_trait_id,
                None,
                &self.bounds,
            )
            .with_effective_trait_methods(self.effective_trait_methods)
            .select_required_trait_member(
                &receiver,
                &carrier_ty,
                protocol.try_trait_id,
                protocol.branch_method_id,
            ) {
                Ok(selected) => selected,
                Err(error) => {
                    let message = match error {
                        crate::selection::SelectionDiagnostic::NoImplementation { .. } => {
                            format!("Cannot use '?' on non-carrier type {carrier_ty}")
                        }
                        _ => error.message(),
                    };
                    if self.try_strict && self.strict {
                        errors.push(ResolveError::new(message));
                    }
                    return;
                }
            };

            if let Some(function) = selected.function.as_ref() {
                if function.is_unsafe && !self.unsafe_context.get() {
                    if self.try_strict && self.strict {
                        errors.push(ResolveError::new(format!(
                            "Call to unsafe function '{}' requires an unsafe block",
                            function.name
                        )));
                    }
                    return;
                }
            }

            let Some(selected_output) =
                self.selected_try_associated_type(&selected, try_trait, protocol.output_id)
            else {
                if self.try_strict && self.strict {
                    errors.push(ResolveError::new(
                        "Cannot use '?' because the Try language-item output type is unavailable"
                            .to_string(),
                    ));
                }
                return;
            };
            let Some(selected_residual) =
                self.selected_try_associated_type(&selected, try_trait, protocol.residual_id)
            else {
                if self.try_strict && self.strict {
                    errors.push(ResolveError::new(
                        "Cannot use '?' because the Try language-item residual type is unavailable"
                            .to_string(),
                    ));
                }
                return;
            };

            if try_type_contains_unresolved(&selected_output)
                || try_type_contains_unresolved(&selected_residual)
            {
                if self.try_strict && self.strict {
                    errors.push(ResolveError::new(
                        "Cannot use '?' because its Try output or residual type is unresolved or generic"
                            .to_string(),
                    ));
                }
                return;
            }

            {
                let mut engine = self.engine.borrow_mut();
                if let Err(error) = engine.unify(output_ty, &selected_output) {
                    if self.try_strict && self.strict {
                        errors.push(ResolveError::new(format!(
                            "Cannot use '?' because its output type could not be resolved: {error}"
                        )));
                    }
                    return;
                }
                if let Err(error) = engine.unify(residual_ty, &selected_residual) {
                    if self.try_strict && self.strict {
                        errors.push(ResolveError::new(format!(
                            "Cannot use '?' because its residual type could not be resolved: {error}"
                        )));
                    }
                    return;
                }
            }

            let method_substitution = selected
                .function
                .as_ref()
                .map(|function| {
                    self.infer_try_method_substitution(&selected, &carrier_ty, function, &[])
                })
                .unwrap_or_default();
            let method = selected.target_with_substitution(&method_substitution, |ty| {
                self.engine.borrow().resolve(ty)
            });
            *branch_method = Some(method);
            *branch_self_receiver = selected
                .function
                .as_ref()
                .and_then(|function| function.self_receiver);
        }

        if from_residual_target.is_none() {
            let resolved_return_ty = self.engine.borrow().resolve(return_ty);
            if try_type_head_is_unresolved(&resolved_return_ty) {
                if self.try_strict && self.strict {
                    errors.push(ResolveError::new(
                        "Cannot use '?' because its enclosing return type is unresolved or generic"
                            .to_string(),
                    ));
                }
                return;
            }
            let resolved_residual_ty = self.engine.borrow().resolve(residual_ty);
            let return_owner = HirExpr {
                ty: resolved_return_ty.clone(),
                kind: HirExprKind::Unit,
                span: expr.span.clone(),
            };
            let Some(from_residual_trait) = self.traits.get(&protocol.from_residual_trait_id)
            else {
                if self.try_strict && self.strict {
                    errors.push(ResolveError::new(
                        "Cannot use '?' because the FromResidual language-item trait is unavailable"
                            .to_string(),
                    ));
                }
                return;
            };
            let residual_arg = HirExpr {
                ty: resolved_residual_ty.clone(),
                kind: HirExprKind::Unit,
                span: expr.span.clone(),
            };
            let selected = match SelectionService::new(
                self.traits,
                self.impls,
                self.sized_trait_id,
                None,
                &self.bounds,
            )
            .with_effective_trait_methods(self.effective_trait_methods)
            .select_static_trait_member(
                &return_owner,
                &resolved_return_ty,
                from_residual_trait.id,
                std::slice::from_ref(&resolved_residual_ty),
                protocol.from_residual_method_id,
            ) {
                Ok(selected) => selected,
                Err(error) => {
                    let message = match error {
                        crate::selection::SelectionDiagnostic::NoImplementation { .. } => {
                            format!(
                                "Cannot use '?' because {resolved_return_ty} does not implement FromResidual {resolved_residual_ty}"
                            )
                        }
                        _ => error.message(),
                    };
                    if self.try_strict && self.strict {
                        errors.push(ResolveError::new(message));
                    }
                    return;
                }
            };
            if let Some(function) = selected.function.as_ref() {
                if function.is_unsafe && !self.unsafe_context.get() {
                    if self.try_strict && self.strict {
                        errors.push(ResolveError::new(format!(
                            "Call to unsafe function '{}' requires an unsafe block",
                            function.name
                        )));
                    }
                    return;
                }
            }
            let method_substitution = selected
                .function
                .as_ref()
                .map(|function| {
                    self.infer_try_method_substitution(
                        &selected,
                        &resolved_return_ty,
                        function,
                        &[residual_arg],
                    )
                })
                .unwrap_or_default();
            let method = selected.target_with_substitution(&method_substitution, |ty| {
                self.engine.borrow().resolve(ty)
            });
            *from_residual_target = Some(HirCallTarget::StaticMethod(HirStaticMethodTarget {
                owner_ty: resolved_return_ty,
                method,
            }));
        }

        let engine = self.engine.borrow();
        *output_ty = engine.resolve(output_ty);
        *residual_ty = engine.resolve(residual_ty);
        *return_ty = engine.resolve(return_ty);
        expr.ty = engine.resolve(&expr.ty);
    }

    fn selected_try_associated_type(
        &self,
        selected: &crate::selection::SelectedMethod,
        trait_def: &crate::hir::HirTrait,
        assoc_type_id: crate::ids::AssocTypeId,
    ) -> Option<Type> {
        trait_def
            .associated_types
            .iter()
            .find(|associated| associated.id == assoc_type_id)?;
        let ty = selected
            .associated_types
            .iter()
            .find(|associated| associated.id == assoc_type_id)
            .map(|associated| associated.ty.clone())?;
        Some(self.engine.borrow().resolve(&ty))
    }

    fn infer_try_method_substitution(
        &self,
        selected: &crate::selection::SelectedMethod,
        receiver_ty: &Type,
        function: &HirFunction,
        args: &[HirExpr],
    ) -> HashMap<crate::types::GenericParamId, Type> {
        let inferable = function
            .generic_params
            .iter()
            .map(|param| param.id)
            .collect::<HashSet<_>>();
        let mut engine = self.engine.borrow_mut();
        let mut substitution = function
            .generic_params
            .iter()
            .map(|param| (param.id, engine.fresh_type_var_of_kind(param.kind.clone())))
            .collect::<HashMap<_, _>>();
        if function.is_method {
            if let Some(self_param) = function.params.first() {
                let expected = self_param.ty.substitute_generics(&substitution);
                let _ = engine.unify(&expected, receiver_ty);
                crate::selection::infer_generic_subst_from_types(
                    &self_param.ty,
                    receiver_ty,
                    &mut substitution,
                );
            }
        }
        let param_start = usize::from(function.is_method);
        for (param, arg) in function.params[param_start..].iter().zip(args) {
            let actual = engine.resolve(&arg.ty);
            crate::selection::infer_generic_subst_from_types(&param.ty, &actual, &mut substitution);
            let expected = param.ty.substitute_generics(&substitution);
            let _ = engine.unify(&expected, &arg.ty);
        }
        substitution.retain(|param, _| inferable.contains(param));
        for ty in substitution.values_mut() {
            *ty = engine.resolve(ty);
        }
        for (param, ty) in &selected.owner_substitution {
            substitution.insert(*param, ty.clone());
        }
        substitution
    }

    #[allow(dead_code)]
    fn fresh_local_id(&mut self) -> HirLocalId {
        let id = HirLocalId(self.next_local_id);
        self.next_local_id = self.next_local_id.saturating_add(1);
        id
    }

    #[allow(dead_code)]
    fn lambda_function_type(
        &self,
        params: Vec<Type>,
        ret: Type,
        safety: FunctionSafety,
        captures: &[HirClosureCapture],
    ) -> Type {
        let captures = captures
            .iter()
            .map(|capture| FunctionCapture {
                kind: match capture.kind {
                    HirClosureCaptureKind::SharedBorrow => CaptureKind::SharedBorrow,
                    HirClosureCaptureKind::MutableBorrow => CaptureKind::MutableBorrow,
                    HirClosureCaptureKind::Move => CaptureKind::Move,
                },
                ty: capture.ty.clone(),
            })
            .collect::<Vec<_>>();
        Type::function_with_metadata(
            params,
            ret,
            safety,
            CallableKind::from_captures(&captures),
            captures,
        )
    }

    #[allow(dead_code)]
    fn collect_method_value_captures(
        &self,
        expr: &HirExpr,
        receiver_mode: Option<crate::types::ReceiverMode>,
        captures: &mut Vec<HirClosureCapture>,
        seen: &mut HashSet<HirLocalId>,
    ) {
        match &expr.kind {
            HirExprKind::ResolvedVar(HirVarRef {
                target: HirVarTarget::Local(local_id),
                ..
            }) => {
                let Some((name, ty, mutable)) = self.local_bindings.get(local_id) else {
                    return;
                };
                if !seen.insert(*local_id) {
                    return;
                }
                let kind = match receiver_mode {
                    Some(crate::types::ReceiverMode::Move) => HirClosureCaptureKind::Move,
                    Some(crate::types::ReceiverMode::Mut) => HirClosureCaptureKind::MutableBorrow,
                    Some(crate::types::ReceiverMode::Shared) | None => {
                        HirClosureCaptureKind::SharedBorrow
                    }
                };
                captures.push(HirClosureCapture {
                    name: name.clone(),
                    local_id: *local_id,
                    kind,
                    mutable: *mutable,
                    ty: ty.clone(),
                });
            }
            HirExprKind::FieldAccess(base, _, _)
            | HirExprKind::TupleIndex(base, _)
            | HirExprKind::Deref(base)
            | HirExprKind::Ref(_, base)
            | HirExprKind::Cast(base, _) => {
                self.collect_method_value_captures(base, receiver_mode, captures, seen)
            }
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn materialize_method_value(
        &mut self,
        expr: &mut HirExpr,
        receiver: HirExpr,
        method_name: String,
        selected: SelectedMethod,
        errors: &mut Vec<ResolveError>,
    ) {
        let Some(function) = selected.function.clone() else {
            errors.push(ResolveError::new(format!(
                "selected method '{}' has no executable body",
                method_name
            )));
            return;
        };
        let mut substitution = selected.owner_substitution.clone();
        for param in &function.generic_params {
            substitution.entry(param.id).or_insert_with(|| {
                self.engine
                    .borrow_mut()
                    .fresh_type_var_of_kind(param.kind.clone())
            });
        }
        let mut lambda_params = Vec::new();
        for param in &selected.substituted_params {
            let mut param = param.clone();
            param.local_id = self.fresh_local_id();
            param.ty = param.ty.substitute_generics(&substitution);
            lambda_params.push(param);
        }
        let call_args = lambda_params
            .iter()
            .map(|param| HirExpr {
                ty: param.ty.clone(),
                kind: HirExprKind::ResolvedVar(HirVarRef {
                    name: param.name.clone(),
                    target: HirVarTarget::Local(param.local_id),
                }),
                span: expr.span.clone(),
            })
            .collect::<Vec<_>>();
        let call_ret = self.resolved_type(&selected.return_type.substitute_generics(&substitution));
        self.record_selected_bounds(&selected, &substitution, "deferred method value");
        let target = selected.target_with_substitution(&substitution, |ty| self.resolved_type(ty));
        let adjusted_receiver =
            self.apply_receiver_adjustment(selected.receiver.clone(), selected.receiver_adjustment);
        let call = HirExpr {
            ty: call_ret.clone(),
            kind: HirExprKind::MethodCall(
                Box::new(adjusted_receiver),
                method_name,
                call_args,
                function.self_receiver,
                Some(target),
            ),
            span: expr.span.clone(),
        };
        let body = HirBlock {
            ty: call_ret.clone(),
            stmts: vec![HirStmt::Expr(call)],
        };
        let mut captures = Vec::new();
        self.collect_method_value_captures(
            &receiver,
            function.self_receiver,
            &mut captures,
            &mut HashSet::new(),
        );
        let params = lambda_params.iter().map(|param| param.ty.clone()).collect();
        expr.ty = self.lambda_function_type(
            params,
            call_ret.clone(),
            FunctionSafety::from_is_unsafe(function.is_unsafe),
            &captures,
        );
        expr.kind = HirExprKind::Lambda {
            params: lambda_params,
            body,
            captures,
        };
    }

    fn materialize_field_access(&mut self, expr: &mut HirExpr, errors: &mut Vec<ResolveError>) {
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
            if !contains_inference_type(receiver_ty) {
                errors.push(ResolveError::new(format!(
                    "Unknown field '{}' on type '{}'",
                    field_name, receiver_ty
                )));
            }
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
            errors.push(ResolveError::new(format!(
                "Unknown field '{}' on struct '{}'",
                field_name, structure.name
            )));
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

fn contains_inference_type(ty: &Type) -> bool {
    crate::type_services::visit::type_any(ty, |nested: &Type| {
        matches!(nested, Type::TypeVar(_) | Type::Generic(_) | Type::Error)
    })
}

fn try_type_contains_unresolved(ty: &Type) -> bool {
    crate::type_services::visit::type_any(ty, |nested| {
        matches!(
            nested,
            Type::TypeVar(_)
                | Type::Generic(_)
                | Type::Projection { .. }
                | Type::Apply { .. }
                | Type::Constructor { .. }
                | Type::Lambda { .. }
                | Type::BoundVar { .. }
                | Type::Error
        )
    })
}

fn try_carrier_has_unknown_head(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeVar(_)
            | Type::Generic(_)
            | Type::Projection { .. }
            | Type::Apply { .. }
            | Type::Constructor { .. }
            | Type::Lambda { .. }
            | Type::BoundVar { .. }
            | Type::Error
    ) || try_type_contains_unresolved(ty)
}

fn try_type_head_is_unresolved(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeVar(_)
            | Type::Generic(_)
            | Type::Projection { .. }
            | Type::Apply { .. }
            | Type::Constructor { .. }
            | Type::Lambda { .. }
            | Type::BoundVar { .. }
            | Type::Error
    )
}
