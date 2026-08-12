//! Phase 4 constraint solver: discharge accumulated constraints after lowering.
//!
//! This runs between generalization and finalization. It:
//!   - Verifies trait bounds: if a type variable resolved to a concrete type, checks
//!     that the type has an impl for the required trait.
//!   - Checks integer/float literal constraints: if a TypeVar from a literal still
//!     has no concrete type, emits an ambiguity warning.
//!
//! Trait bound violations on concrete types are hard errors.

use std::collections::HashMap;

use crate::hir::{HirEnum, HirImpl, HirImplReceiverPattern, HirStruct, HirVariantFields};
use crate::ids::DefId;
use crate::ids::{Idx, TypeVarId};
use crate::infer::constraints::{Constraint, ConstraintStore};
use crate::infer::InferenceEngine;
use crate::language_items::LanguageItems;
use crate::type_services::facts::TypeFacts;
use crate::type_services::normalize::TypeNormalizer;
use crate::types::{CallableKind, CaptureKind, GenericParamId, TraitBound, Type};

#[derive(Debug, Clone, Copy, Default)]
struct BuiltinTraitIds {
    sized: Option<DefId>,
    fn_once: Option<DefId>,
    fn_mut: Option<DefId>,
    fn_trait: Option<DefId>,
    send: Option<DefId>,
    sync: Option<DefId>,
}

impl BuiltinTraitIds {
    fn from_language_items(items: &LanguageItems<DefId>) -> Self {
        Self {
            sized: items.sized.as_ref().map(|items| items.trait_id),
            fn_once: items.fn_once.as_ref().map(|items| items.trait_id),
            fn_mut: items.fn_mut.as_ref().map(|items| items.trait_id),
            fn_trait: items.fn_trait.as_ref().map(|items| items.trait_id),
            send: items.send.as_ref().map(|items| items.trait_id),
            sync: items.sync.as_ref().map(|items| items.trait_id),
        }
    }
}

/// Result of constraint solving.
pub struct SolveResult {
    /// Warnings (non-fatal: ambiguous literals that were defaulted).
    pub warnings: Vec<String>,
    /// Hard errors: concrete type violates a required trait bound.
    pub errors: Vec<String>,
    /// TypeVar ids that remain free but have at least one trait bound attached.
    pub generic_bounds: HashMap<TypeVarId, Vec<TraitBound>>,
}

/// Solve all constraints in `store`.
///
/// `impls` is the list of all known trait implementations.
/// Trait violations on concrete types are always hard errors.
pub fn solve_constraints(
    engine: &mut InferenceEngine,
    store: &ConstraintStore,
    impls: &HashMap<DefId, HirImpl>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
    traits: &HashMap<DefId, crate::hir::HirTrait>,
    language_items: &LanguageItems<DefId>,
) -> SolveResult {
    let builtin_traits = BuiltinTraitIds::from_language_items(language_items);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut generic_bounds: HashMap<TypeVarId, Vec<TraitBound>> = HashMap::new();

    for constraint in &store.constraints {
        let Constraint::Equality {
            left,
            right,
            context,
            ..
        } = constraint
        else {
            continue;
        };
        let resolved_left = engine.resolve(left);
        let resolved_right = engine.resolve(right);
        if let Err(error) = engine.unify(&resolved_left, &resolved_right) {
            if contains_unresolved_type(&resolved_left) || contains_unresolved_type(&resolved_right)
            {
                continue;
            }
            errors.push(format!("{context}: {error}"));
        }
    }

    // Callable bounds relate a callable's argument and return types to generic
    // parameters. Establish those equalities before validating any bounds so
    // constraint iteration order cannot leave `Ret` unconstrained.
    for constraint in &store.constraints {
        let Constraint::Trait { ty, bound, .. } = constraint else {
            continue;
        };
        let is_callable = builtin_traits.fn_once == Some(bound.trait_id)
            || builtin_traits.fn_mut == Some(bound.trait_id)
            || builtin_traits.fn_trait == Some(bound.trait_id);
        if !is_callable || bound.type_args.len() != 2 {
            continue;
        }
        let resolved = engine.resolve(ty);
        if let Type::Function { params, ret, .. } = resolved {
            let args = match params.as_slice() {
                [] => Type::Unit,
                [param] => param.clone(),
                params => Type::Tuple(params.to_vec()),
            };
            let _ = engine.unify(&bound.type_args[0], &args);
            if !try_unify_bound_constructor_application(
                engine,
                store,
                impls,
                &bound.type_args[1],
                &ret,
            ) {
                let _ = engine.unify(&bound.type_args[1], &ret);
            }
        } else {
            infer_explicit_impl_trait_args(engine, &resolved, bound, impls, structs, enums);
        }
    }

    for constraint in &store.constraints {
        match constraint {
            Constraint::Trait {
                ty,
                bound,
                span: _,
                context,
            } => {
                let normalization_env = engine.normalization_env();
                let resolved_ty = engine.resolve(ty);
                let resolved = TypeNormalizer::new(&normalization_env)
                    .normalize(&resolved_ty)
                    .unwrap_or(resolved_ty);
                if !matches!(resolved, Type::TypeVar(_) | Type::Generic(_))
                    && contains_unresolved_type(&resolved)
                {
                    continue;
                }
                let resolved_bound = TraitBound {
                    trait_id: bound.trait_id,
                    type_args: bound
                        .type_args
                        .iter()
                        .map(|arg| {
                            let resolved = engine.resolve(arg);
                            TypeNormalizer::new(&normalization_env)
                                .normalize(&resolved)
                                .unwrap_or(resolved)
                        })
                        .collect(),
                };
                match &resolved {
                    Type::TypeVar(id) => {
                        // Still a free variable — will become a Generic param.
                        generic_bounds.entry(*id).or_default().extend(
                            trait_bounds_with_supertraits(&resolved, &resolved_bound, traits),
                        );
                    }
                    Type::Generic(_) => {
                        // Already a generic parameter — bound satisfied at mono time.
                    }
                    concrete_ty => {
                        if context == "try operator"
                            && language_items
                                .try_protocol
                                .as_ref()
                                .is_some_and(|protocol| bound.trait_id == protocol.try_trait_id)
                            && crate::type_services::visit::type_any(concrete_ty, |nested| {
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
                        {
                            continue;
                        }
                        if !impl_exists_for(
                            concrete_ty,
                            resolved_bound.trait_id,
                            &resolved_bound.type_args,
                            impls,
                            structs,
                            enums,
                            builtin_traits,
                        ) {
                            if context == "try operator"
                                && language_items
                                    .try_protocol
                                    .as_ref()
                                    .is_some_and(|protocol| bound.trait_id == protocol.try_trait_id)
                            {
                                errors.push(format!(
                                    "Cannot use '?' on non-carrier type {concrete_ty}"
                                ));
                                continue;
                            }
                            let msg = format!(
                                "type `{}` does not implement trait `trait#{}::{}` (required by {})",
                                concrete_ty,
                                resolved_bound.trait_id.crate_id.0,
                                resolved_bound.trait_id.local.0,
                                context
                            );
                            errors.push(msg);
                        }
                    }
                }
            }

            // Integer literal constraints: unresolved literals default later, but a
            // resolved literal must still be an integer type.
            Constraint::IntLiteral { var, span: _ } => {
                let resolved = engine.resolve(&Type::TypeVar(*var));
                match &resolved {
                    Type::TypeVar(_) => {}
                    Type::Generic(_) | Type::Projection { .. } => {}
                    ty if TypeFacts::is_integer(ty) => {}
                    other => {
                        errors.push(format!(
                            "integer literal resolved to non-integer type `{}`",
                            other
                        ));
                    }
                }
            }

            Constraint::FloatLiteral { var, span: _ } => {
                let resolved = engine.resolve(&Type::TypeVar(*var));
                match &resolved {
                    Type::TypeVar(_) => {
                        warnings.push(format!(
                            "warning: type of float literal is ambiguous (TypeVar {}); \
                             add a type annotation such as `: F64`",
                            var.raw()
                        ));
                    }
                    ty if TypeFacts::is_float(ty) => {}
                    other => {
                        warnings.push(format!(
                            "warning: float literal resolved to non-float type `{}`",
                            other
                        ));
                    }
                }
            }
            Constraint::Equality { .. } => {}
        }
    }

    SolveResult {
        warnings,
        errors,
        generic_bounds,
    }
}

fn contains_unresolved_type(ty: &Type) -> bool {
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

fn try_unify_bound_constructor_application(
    engine: &mut InferenceEngine,
    store: &ConstraintStore,
    impls: &HashMap<DefId, HirImpl>,
    pattern: &Type,
    actual: &Type,
) -> bool {
    let resolved_pattern = engine.resolve(pattern);
    let Type::Apply { constructor, args } = &resolved_pattern else {
        return false;
    };
    let Type::TypeVar(constructor_var) = constructor.as_ref() else {
        return false;
    };

    let bounded_traits = store
        .constraints
        .iter()
        .filter_map(|constraint| {
            let Constraint::Trait { ty, bound, .. } = constraint else {
                return None;
            };
            matches!(engine.resolve(ty), Type::TypeVar(id) if id == *constructor_var)
                .then_some(bound.trait_id)
        })
        .collect::<Vec<_>>();
    if bounded_traits.is_empty() {
        return false;
    }

    let mut candidates: Vec<(Type, InferenceEngine)> = Vec::new();
    for imp in impls.values() {
        if !imp
            .trait_id
            .is_some_and(|trait_id| bounded_traits.contains(&trait_id))
        {
            continue;
        }
        let receiver = match &imp.receiver_pattern {
            HirImplReceiverPattern::Constructor(receiver)
            | HirImplReceiverPattern::Exact(receiver) => receiver,
            HirImplReceiverPattern::SliceFamily { .. } => continue,
        };
        let mut probe = engine.clone_for_probe();
        let impl_substitution = imp
            .type_generics
            .iter()
            .map(|param| (param.id, probe.fresh_type_var_of_kind(param.kind.clone())))
            .collect::<HashMap<_, _>>();
        let candidate = receiver.substitute_generics(&impl_substitution);
        let applied = Type::Apply {
            constructor: Box::new(candidate.clone()),
            args: args.clone(),
        };
        if probe.unify(&applied, actual).is_err() {
            continue;
        }
        let resolved_candidate = probe.resolve(&candidate);
        let normalization_env = probe.normalization_env();
        let resolved_candidate = TypeNormalizer::new(&normalization_env)
            .normalize(&resolved_candidate)
            .unwrap_or(resolved_candidate);
        if probe
            .unify(&Type::TypeVar(*constructor_var), &resolved_candidate)
            .is_err()
        {
            continue;
        }
        if candidates
            .iter()
            .all(|(existing, _)| existing != &resolved_candidate)
        {
            candidates.push((resolved_candidate, probe));
        }
    }

    if candidates.len() != 1 {
        return false;
    }
    let (_, selected) = candidates.pop().expect("one bounded constructor candidate");
    *engine = selected;
    true
}

fn trait_bounds_with_supertraits(
    subject: &Type,
    initial: &TraitBound,
    traits: &HashMap<DefId, crate::hir::HirTrait>,
) -> Vec<TraitBound> {
    fn expand(
        subject: &Type,
        bound: TraitBound,
        traits: &HashMap<DefId, crate::hir::HirTrait>,
        visiting: &mut std::collections::HashSet<crate::types::Predicate>,
        output: &mut Vec<TraitBound>,
    ) {
        let key = crate::types::Predicate::Trait {
            subject: subject.clone(),
            trait_id: bound.trait_id,
            args: bound.type_args.clone(),
        };
        if !visiting.insert(key.clone()) {
            return;
        }
        if !output.contains(&bound) {
            output.push(bound.clone());
        }
        if let Some(trait_def) = traits.get(&bound.trait_id) {
            let mut subst = crate::selection::generic_substitution_for_owner(
                &trait_def.generic_params,
                &bound.type_args,
            );
            let target_id = trait_def.target.as_ref().map(|target| target.id).unwrap_or(
                crate::types::GenericParamId {
                    owner: trait_def.id,
                    index: trait_def.generic_params.len() as u32,
                },
            );
            subst.insert(target_id, subject.clone());
            for predicate in &trait_def.predicates {
                let crate::types::Predicate::Trait {
                    subject: implied_subject,
                    trait_id,
                    args,
                } = predicate.substitute_generics(&subst);
                if implied_subject == *subject {
                    expand(
                        subject,
                        TraitBound {
                            trait_id,
                            type_args: args,
                        },
                        traits,
                        visiting,
                        output,
                    );
                }
            }
        }
        visiting.remove(&key);
    }

    let mut output = Vec::new();
    expand(
        subject,
        initial.clone(),
        traits,
        &mut std::collections::HashSet::new(),
        &mut output,
    );
    output
}

/// Check whether `ty` has an impl of the canonical trait ID in the given impl list.
///
/// Matches on the `type_name` field of `HirImpl`. Generic impls (with
/// `type_generics` non-empty, e.g. `impl<T> Show for Vec<T>`) are treated as
/// matching any type of the right shape.
fn impl_exists_for(
    ty: &Type,
    trait_id: DefId,
    trait_args: &[Type],
    impls: &HashMap<DefId, HirImpl>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
    builtin_traits: BuiltinTraitIds,
) -> bool {
    if builtin_traits.sized == Some(trait_id) {
        return type_is_sized(ty);
    }

    if let Some(satisfied) = callable_trait_satisfied(ty, trait_id, trait_args, builtin_traits) {
        return satisfied;
    }

    if explicit_impl_exists_for(
        ty,
        trait_id,
        trait_args,
        impls,
        structs,
        enums,
        builtin_traits,
    ) {
        return true;
    }

    auto_trait_satisfied(
        ty,
        trait_id,
        impls,
        structs,
        enums,
        builtin_traits,
        &mut std::collections::HashSet::new(),
    )
}

fn explicit_impl_exists_for(
    ty: &Type,
    trait_id: DefId,
    trait_args: &[Type],
    impls: &HashMap<DefId, HirImpl>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
    builtin_traits: BuiltinTraitIds,
) -> bool {
    let marker_provider_crate =
        if builtin_traits.send == Some(trait_id) || builtin_traits.sync == Some(trait_id) {
            Some(trait_id.crate_id)
        } else {
            None
        };
    let mut impl_ids = impls.keys().copied().collect::<Vec<_>>();
    impl_ids.sort();
    impl_ids.into_iter().any(|id| {
        if marker_provider_crate.is_some_and(|provider| id.crate_id != provider) {
            return false;
        }
        let Some(imp) = impls.get(&id) else {
            return false;
        };
        if imp.trait_id != Some(trait_id) || !impl_receiver_owner_matches(ty, imp, structs, enums) {
            return false;
        }

        let Some(mut subst) =
            crate::selection::receiver_pattern_substitution(&imp.receiver_pattern, ty)
        else {
            return false;
        };

        impl_trait_args_match(&imp.trait_arg_types, trait_args, &mut subst)
            && impl_bounds_satisfied(imp, &subst, impls, structs, enums, builtin_traits)
    })
}

fn infer_explicit_impl_trait_args(
    engine: &mut InferenceEngine,
    ty: &Type,
    bound: &TraitBound,
    impls: &HashMap<DefId, HirImpl>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
) {
    let mut impl_ids = impls.keys().copied().collect::<Vec<_>>();
    impl_ids.sort();
    for id in impl_ids {
        let Some(imp) = impls.get(&id) else {
            continue;
        };
        if imp.trait_id != Some(bound.trait_id)
            || imp.trait_arg_types.len() != bound.type_args.len()
            || !impl_receiver_owner_matches(ty, imp, structs, enums)
        {
            continue;
        }
        let Some(subst) =
            crate::selection::receiver_pattern_substitution(&imp.receiver_pattern, ty)
        else {
            continue;
        };
        let mut probe = engine.clone_for_probe();
        let matches = imp
            .trait_arg_types
            .iter()
            .zip(&bound.type_args)
            .all(|(expected, actual)| {
                probe
                    .unify(actual, &expected.substitute_generics(&subst))
                    .is_ok()
            });
        if matches {
            *engine = probe;
            return;
        }
    }
}

fn impl_bounds_satisfied(
    imp: &HirImpl,
    subst: &HashMap<crate::types::GenericParamId, Type>,
    impls: &HashMap<DefId, HirImpl>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
    builtin_traits: BuiltinTraitIds,
) -> bool {
    imp.bounds.iter().all(|(generic_param, bounds)| {
        let Some(ty) = subst.get(generic_param) else {
            return false;
        };

        bounds.iter().all(|bound| {
            let type_args: Vec<Type> = bound
                .type_args
                .iter()
                .map(|arg| arg.substitute_generics(subst))
                .collect();
            impl_exists_for(
                ty,
                bound.trait_id,
                &type_args,
                impls,
                structs,
                enums,
                builtin_traits,
            )
        })
    })
}

fn callable_trait_satisfied(
    ty: &Type,
    trait_id: DefId,
    trait_args: &[Type],
    builtin_traits: BuiltinTraitIds,
) -> Option<bool> {
    let required_kind = if builtin_traits.fn_trait == Some(trait_id) {
        CallableKind::Fn
    } else if builtin_traits.fn_mut == Some(trait_id) {
        CallableKind::FnMut
    } else if builtin_traits.fn_once == Some(trait_id) {
        CallableKind::FnOnce
    } else {
        return None;
    };
    let Type::Function {
        params,
        ret,
        callable_kind,
        ..
    } = ty
    else {
        return None;
    };
    if trait_args.len() != 2 || *callable_kind > required_kind {
        return Some(false);
    }
    let args = match params.as_slice() {
        [] => Type::Unit,
        [param] => param.clone(),
        params => Type::Tuple(params.to_vec()),
    };
    let args_match = trait_args[0] == args
        || matches!((&trait_args[0], &args), (Type::Tuple(elements), Type::Unit) if elements.is_empty());
    Some(args_match && trait_args[1] == **ret)
}

fn auto_trait_satisfied(
    ty: &Type,
    trait_id: DefId,
    impls: &HashMap<DefId, HirImpl>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
    builtin_traits: BuiltinTraitIds,
    visiting: &mut std::collections::HashSet<Type>,
) -> bool {
    let is_send = builtin_traits.send == Some(trait_id);
    let is_sync = builtin_traits.sync == Some(trait_id);
    if !is_send && !is_sync {
        return false;
    }
    if !visiting.insert(ty.clone()) {
        return true;
    }

    let nested = |nested: &Type, visiting: &mut std::collections::HashSet<Type>| {
        explicit_impl_exists_for(nested, trait_id, &[], impls, structs, enums, builtin_traits)
            || auto_trait_satisfied(
                nested,
                trait_id,
                impls,
                structs,
                enums,
                builtin_traits,
                visiting,
            )
    };

    let satisfied = match ty {
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::F32
        | Type::F64
        | Type::Bool
        | Type::Char
        | Type::Unit
        | Type::Never
        | Type::Str => true,
        Type::Array(inner, _) | Type::Slice(inner) => nested(inner, visiting),
        Type::Tuple(elements) => elements.iter().all(|element| nested(element, visiting)),
        Type::Reference { .. } if is_send => false,
        Type::Reference { inner, .. } => {
            let required = builtin_traits.sync;
            required.is_some_and(|required| {
                explicit_impl_exists_for(
                    inner,
                    required,
                    &[],
                    impls,
                    structs,
                    enums,
                    builtin_traits,
                ) || auto_trait_satisfied(
                    inner,
                    required,
                    impls,
                    structs,
                    enums,
                    builtin_traits,
                    visiting,
                )
            })
        }
        Type::Pointer(_) => false,
        Type::Function { captures, .. } if is_send => captures
            .iter()
            .all(|capture| capture.kind == CaptureKind::Move && nested(&capture.ty, visiting)),
        Type::Function { captures, .. } => captures.iter().all(|capture| {
            let required = match capture.kind {
                CaptureKind::SharedBorrow => builtin_traits.sync,
                CaptureKind::MutableBorrow | CaptureKind::Move => builtin_traits.sync,
            };
            required.is_some_and(|required| {
                explicit_impl_exists_for(
                    &capture.ty,
                    required,
                    &[],
                    impls,
                    structs,
                    enums,
                    builtin_traits,
                ) || auto_trait_satisfied(
                    &capture.ty,
                    required,
                    impls,
                    structs,
                    enums,
                    builtin_traits,
                    visiting,
                )
            })
        }),
        Type::Struct { id, args } => structs.get(id).is_some_and(|structure| {
            let subst = args
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, ty)| {
                    (
                        GenericParamId {
                            owner: *id,
                            index: index as u32,
                        },
                        ty,
                    )
                })
                .collect();
            structure
                .fields
                .iter()
                .all(|field| nested(&field.ty.substitute_generics(&subst), visiting))
        }),
        Type::Enum { id, args } => enums.get(id).is_some_and(|enumeration| {
            let subst = args
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, ty)| {
                    (
                        GenericParamId {
                            owner: *id,
                            index: index as u32,
                        },
                        ty,
                    )
                })
                .collect();
            enumeration
                .variants
                .iter()
                .all(|variant| match &variant.fields {
                    HirVariantFields::Unit => true,
                    HirVariantFields::Positional(fields) => fields
                        .iter()
                        .all(|field| nested(&field.substitute_generics(&subst), visiting)),
                    HirVariantFields::Named(fields) => fields
                        .iter()
                        .all(|field| nested(&field.ty.substitute_generics(&subst), visiting)),
                })
        }),
        Type::Generic(_)
        | Type::Projection { .. }
        | Type::TypeVar(_)
        | Type::Constructor { .. }
        | Type::Apply { .. }
        | Type::Lambda { .. }
        | Type::BoundVar { .. }
        | Type::Error => false,
    };
    visiting.remove(ty);
    satisfied
}

fn impl_trait_args_match(
    expected: &[Type],
    actual: &[Type],
    subst: &mut HashMap<crate::types::GenericParamId, Type>,
) -> bool {
    if expected.len() != actual.len() {
        return false;
    }

    expected
        .iter()
        .zip(actual.iter())
        .all(|(expected, actual)| {
            matches!((expected, actual), (Type::Tuple(items), Type::Unit) if items.is_empty())
                || matches!((expected, actual), (Type::Unit, Type::Tuple(items)) if items.is_empty())
                || crate::selection::type_pattern_matches(expected, actual, subst)
        })
}

fn type_is_sized(ty: &Type) -> bool {
    match ty {
        Type::Slice(_) | Type::Str => false,
        Type::Array(inner, _) => type_is_sized(inner),
        Type::Tuple(elems) => elems.iter().all(type_is_sized),
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::F32
        | Type::F64
        | Type::Bool
        | Type::Char
        | Type::Unit
        | Type::Never
        | Type::Struct { .. }
        | Type::Enum { .. }
        | Type::Reference { .. }
        | Type::Pointer(_)
        | Type::Function { .. } => true,
        Type::Generic(_)
        | Type::Projection { .. }
        | Type::TypeVar(_)
        | Type::Constructor { .. }
        | Type::Apply { .. }
        | Type::Lambda { .. }
        | Type::BoundVar { .. }
        | Type::Error => false,
    }
}

fn impl_receiver_owner_matches(
    ty: &Type,
    imp: &HirImpl,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
) -> bool {
    if let Some(id) = constructor_owner_id(ty) {
        return (structs.contains_key(&id) || enums.contains_key(&id))
            && receiver_owner_id(imp) == Some(id);
    }
    match ty {
        Type::Struct { id, .. } => structs.contains_key(id) && receiver_owner_id(imp) == Some(*id),
        Type::Enum { id, .. } => enums.contains_key(id) && receiver_owner_id(imp) == Some(*id),
        _ => {
            let ty_name = type_base_name(ty);
            imp.type_name == ty_name
                || (!imp.type_generics.is_empty() && type_shape_matches(&imp.type_name, &ty_name))
        }
    }
}

fn constructor_owner_id(ty: &Type) -> Option<DefId> {
    match ty {
        Type::Constructor { id, .. } | Type::Struct { id, .. } | Type::Enum { id, .. } => Some(*id),
        Type::Apply { constructor, .. } => constructor_owner_id(constructor),
        Type::Lambda { body, .. } => constructor_owner_id(body),
        _ => None,
    }
}

fn receiver_owner_id(imp: &HirImpl) -> Option<DefId> {
    match &imp.receiver_pattern {
        crate::hir::HirImplReceiverPattern::Exact(Type::Struct { id, .. })
        | crate::hir::HirImplReceiverPattern::Exact(Type::Enum { id, .. }) => Some(*id),
        crate::hir::HirImplReceiverPattern::Constructor(ty) => constructor_owner_id(ty),
        crate::hir::HirImplReceiverPattern::Exact(_)
        | crate::hir::HirImplReceiverPattern::SliceFamily { .. } => None,
    }
}

fn fixed_array_len_from_type_name(type_name: &str) -> Option<usize> {
    let inner = type_name.strip_prefix('[')?.strip_suffix(']')?;
    let (_, len) = inner.split_once(';')?;
    len.trim().parse().ok()
}

fn type_shape_matches(candidate: &str, lookup: &str) -> bool {
    if candidate == lookup {
        return true;
    }

    match (
        fixed_array_len_from_type_name(candidate),
        fixed_array_len_from_type_name(lookup),
    ) {
        (Some(candidate_len), Some(lookup_len)) => candidate_len == lookup_len,
        _ => candidate == "Array" && lookup == "Array",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hir::HirImplOwner;
    use crate::ids::{CrateId, LocalDefId, TypeVarId};
    use crate::lexer::Span;
    use crate::types::TraitBound;

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    fn test_impl(type_name: &str, owner_id: DefId, trait_id: DefId) -> HirImpl {
        HirImpl {
            id: def_id(100),
            owner: HirImplOwner::Named(type_name.to_string()),
            type_name: type_name.to_string(),
            type_generics: Vec::new(),
            receiver_pattern: crate::hir::HirImplReceiverPattern::Exact(Type::Struct {
                id: owner_id,
                args: Vec::new(),
            }),
            trait_name: Some("Show".to_string()),
            trait_id: Some(trait_id),
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: std::collections::HashMap::new().into(),
            methods: HashMap::new(),
        }
    }

    #[test]
    fn solve_constraints_returns_generic_bounds_by_typed_type_var_id() {
        let mut engine = InferenceEngine::new();
        let mut store = ConstraintStore::new();
        store.add_trait(
            Type::TypeVar(TypeVarId(0)),
            TraitBound {
                trait_id: crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(1)),
                type_args: Vec::new(),
            },
            Span::default(),
            "test",
        );

        let result = solve_constraints(
            &mut engine,
            &store,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &LanguageItems::default(),
        );

        assert_eq!(
            result.generic_bounds.get(&TypeVarId(0)),
            Some(&vec![TraitBound {
                trait_id: crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(1)),
                type_args: Vec::new(),
            }])
        );
    }

    #[test]
    fn concrete_trait_bound_matches_struct_impl_by_def_id() {
        let trait_id = def_id(1);
        let struct_id = def_id(2);
        let mut structs = HashMap::new();
        structs.insert(
            struct_id,
            HirStruct {
                id: struct_id,
                name: "Widget".to_string(),
                generic_params: Vec::new(),
                fields: Vec::new(),
            },
        );

        assert!(impl_exists_for(
            &Type::Struct {
                id: struct_id,
                args: Vec::new(),
            },
            trait_id,
            &[],
            &HashMap::from([(def_id(3), test_impl("Widget", struct_id, trait_id))]),
            &structs,
            &HashMap::new(),
            BuiltinTraitIds::default(),
        ));
    }

    #[test]
    fn constructor_trait_selection_solver_matches_constructor_impl_by_def_id() {
        let trait_id = def_id(20);
        let enum_id = def_id(21);
        let constructor = Type::Constructor {
            id: enum_id,
            flavor: crate::types::NominalTypeKind::Enum,
        };
        let mut imp = test_impl("Maybe", enum_id, trait_id);
        imp.receiver_pattern = crate::hir::HirImplReceiverPattern::Constructor(constructor.clone());
        let impls = HashMap::from([(imp.id, imp)]);
        let enums = HashMap::from([(
            enum_id,
            HirEnum {
                id: enum_id,
                name: "Maybe".to_string(),
                generic_params: vec![crate::types::GenericParamDecl::type_param(
                    crate::types::GenericParamId {
                        owner: enum_id,
                        index: 0,
                    },
                    "T",
                )],
                variants: Vec::new(),
            },
        )]);
        let mut store = ConstraintStore::new();
        store.add_trait(
            constructor,
            TraitBound {
                trait_id,
                type_args: Vec::new(),
            },
            Span::default(),
            "constructor bound",
        );

        let result = solve_constraints(
            &mut InferenceEngine::new(),
            &store,
            &impls,
            &HashMap::new(),
            &enums,
            &HashMap::new(),
            &LanguageItems::default(),
        );

        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn recursive_supertrait_obligations_terminate_and_are_memoized() {
        let first = def_id(30);
        let second = def_id(31);
        let trait_def = |id: DefId, name: &str, parent: DefId| crate::hir::HirTrait {
            id,
            name: name.to_string(),
            generic_params: Vec::new(),
            target: None,
            predicates: vec![crate::types::Predicate::Trait {
                subject: Type::Generic(crate::types::GenericParamId {
                    owner: id,
                    index: 0,
                }),
                trait_id: parent,
                args: Vec::new(),
            }],
            associated_types: Vec::new(),
            methods: HashMap::new(),
            signatures: HashMap::new(),
        };
        let traits = HashMap::from([
            (first, trait_def(first, "First", second)),
            (second, trait_def(second, "Second", first)),
        ]);
        let mut engine = InferenceEngine::new();
        let variable = engine.fresh_type_var();
        let Type::TypeVar(variable_id) = variable.clone() else {
            unreachable!();
        };
        let mut store = ConstraintStore::new();
        store.add_trait(
            variable,
            TraitBound {
                trait_id: first,
                type_args: Vec::new(),
            },
            Span::default(),
            "recursive test",
        );

        let result = solve_constraints(
            &mut engine,
            &store,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &traits,
            &LanguageItems::default(),
        );

        let bounds = &result.generic_bounds[&variable_id];
        assert_eq!(bounds.len(), 2);
        assert!(bounds.iter().any(|bound| bound.trait_id == first));
        assert!(bounds.iter().any(|bound| bound.trait_id == second));
    }

    #[test]
    fn same_named_traits_do_not_satisfy_each_other_by_name() {
        let required_trait_id = def_id(1);
        let wrong_trait_id = def_id(2);
        let struct_id = def_id(3);
        let mut structs = HashMap::new();
        structs.insert(
            struct_id,
            HirStruct {
                id: struct_id,
                name: "Widget".to_string(),
                generic_params: Vec::new(),
                fields: Vec::new(),
            },
        );

        assert!(!impl_exists_for(
            &Type::Struct {
                id: struct_id,
                args: Vec::new(),
            },
            required_trait_id,
            &[],
            &HashMap::from([(def_id(4), test_impl("Widget", struct_id, wrong_trait_id))]),
            &structs,
            &HashMap::new(),
            BuiltinTraitIds::default(),
        ));
    }
}

/// Return the base name of a type for impl lookup (e.g. "I64", "Vec", "Point").
fn type_base_name(ty: &Type) -> String {
    match ty {
        Type::I8 => "I8".into(),
        Type::I16 => "I16".into(),
        Type::I32 => "I32".into(),
        Type::I64 => "I64".into(),
        Type::U8 => "U8".into(),
        Type::U16 => "U16".into(),
        Type::U32 => "U32".into(),
        Type::U64 => "U64".into(),
        Type::F32 => "F32".into(),
        Type::F64 => "F64".into(),
        Type::Bool => "Bool".into(),
        Type::Str => "Str".into(),
        Type::Char => "Char".into(),
        Type::Slice(_) => "Array".into(),
        Type::Array(_, _) => ty.to_string(),
        Type::Struct { .. } | Type::Enum { .. } => ty.to_string(),
        _ => ty.to_string(),
    }
}
