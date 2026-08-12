//! Type inference finalization — converts PartialHir (with type variables)
//! into a fully-typed HirProgram.

mod authority;
pub mod constraints;
mod engine;
mod finalize;
mod generalize;
mod helpers;
pub mod solve;
mod type_vars;

pub use constraints::ConstraintStore;
pub use engine::InferenceEngine;
pub use generalize::{generalize_single_function, generalize_single_function_with_exclusions};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use crate::collect::resolver::ResolverTables;
use crate::hir::{
    collect_hir_type_ids, AcceptedHirProgram, HirEnum, HirExtern, HirFunction, HirImpl,
    HirLanguageItems, HirNameTables, HirProgram, HirStruct, HirTrait, HirTypeAlias, HirTypeIds,
    HirTypeLocation,
};
use crate::ids::{CrateId, DefId, IdGen, Idx, LocalDefId, TypeId, TypeVarId};
use crate::lower::ResolveError;
use crate::type_context::TypeContext;
use crate::type_services::normalize::TypeNormalizationEnv;
use crate::types::Type;

#[derive(Debug)]
pub struct ResolvedHirProgram {
    pub program: AcceptedHirProgram,
    pub resolver: ResolverTables,
    pub current_def_ids: BTreeSet<DefId>,
    pub root_crate_id: crate::ids::CrateId,
    pub local_def_ids: IdGen<LocalDefId>,
    pub type_context: TypeContext,
    pub type_ids: HirTypeIds,
    pub(crate) normalization_env: TypeNormalizationEnv,
}

impl ResolvedHirProgram {
    #[cfg(test)]
    pub fn new(
        program: HirProgram,
        resolver: ResolverTables,
        current_def_ids: BTreeSet<DefId>,
        root_crate_id: crate::ids::CrateId,
        local_def_ids: IdGen<LocalDefId>,
    ) -> Self {
        let program = AcceptedHirProgram::try_from(program)
            .expect("ResolvedHirProgram requires validated method authority");
        Self::from_accepted(
            program,
            resolver,
            current_def_ids,
            root_crate_id,
            local_def_ids,
        )
    }

    pub fn from_accepted(
        program: AcceptedHirProgram,
        resolver: ResolverTables,
        current_def_ids: BTreeSet<DefId>,
        root_crate_id: crate::ids::CrateId,
        local_def_ids: IdGen<LocalDefId>,
    ) -> Self {
        Self::from_accepted_with_normalization_env(
            program,
            resolver,
            current_def_ids,
            root_crate_id,
            local_def_ids,
            TypeNormalizationEnv::new(),
        )
    }

    fn from_accepted_with_normalization_env(
        program: AcceptedHirProgram,
        resolver: ResolverTables,
        current_def_ids: BTreeSet<DefId>,
        root_crate_id: crate::ids::CrateId,
        local_def_ids: IdGen<LocalDefId>,
        normalization_env: TypeNormalizationEnv,
    ) -> Self {
        let mut type_context = TypeContext::with_normalization_env(normalization_env.clone());
        let type_ids = collect_hir_type_ids(program.program(), &mut type_context);
        Self {
            program,
            resolver,
            current_def_ids,
            root_crate_id,
            local_def_ids,
            type_context,
            type_ids,
            normalization_env,
        }
    }

    pub fn type_id_at(&self, location: &HirTypeLocation) -> Option<TypeId> {
        self.type_ids.get(location)
    }

    pub fn require_type_id_at(&self, location: HirTypeLocation) -> Result<TypeId, ResolveError> {
        self.type_id_at(&location).ok_or_else(|| {
            ResolveError::new(format!("missing finalized HIR TypeId at {:?}", location))
        })
    }

    pub fn type_at(&self, id: TypeId) -> Type {
        self.type_context.type_for(id)
    }
}

/// HIR with type variables still present — output of the lower stage.
pub struct PartialHir {
    pub functions: HashMap<DefId, HirFunction>,
    pub structs: HashMap<DefId, HirStruct>,
    pub enums: HashMap<DefId, HirEnum>,
    pub traits: HashMap<DefId, HirTrait>,
    pub impls: HashMap<DefId, HirImpl>,
    pub externs: HashMap<DefId, HirExtern>,
    pub type_aliases: HashMap<DefId, HirTypeAlias>,
    pub engine: InferenceEngine,
    pub function_type_vars: HashMap<DefId, HashSet<TypeVarId>>,
    pub import_aliases: HashMap<String, String>,
    pub loaded_module_paths: Vec<(String, PathBuf)>,
    pub constraint_store: ConstraintStore,
    pub resolver: ResolverTables,
    pub current_def_ids: BTreeSet<DefId>,
    pub root_crate_id: crate::ids::CrateId,
    pub local_def_ids: IdGen<LocalDefId>,
    pub language_items: HirLanguageItems,
    pub imported_effective_trait_methods: HashMap<(DefId, DefId), DefId>,
}

impl PartialHir {
    fn validate_item_ids(&self) -> Result<(), Vec<ResolveError>> {
        let mut mismatches = Vec::new();
        validate_item_ids(
            &self.functions,
            |function| function.id,
            "function declaration",
            &mut mismatches,
        );
        validate_item_ids(
            &self.structs,
            |structure| structure.id,
            "struct declaration",
            &mut mismatches,
        );
        validate_item_ids(
            &self.enums,
            |enumeration| enumeration.id,
            "enum declaration",
            &mut mismatches,
        );
        validate_item_ids(
            &self.traits,
            |trait_def| trait_def.id,
            "trait declaration",
            &mut mismatches,
        );
        validate_item_ids(
            &self.impls,
            |imp| imp.id,
            "impl declaration",
            &mut mismatches,
        );
        validate_item_ids(
            &self.externs,
            |extern_| extern_.id,
            "extern declaration",
            &mut mismatches,
        );
        validate_item_ids(
            &self.type_aliases,
            |alias| alias.id,
            "type alias declaration",
            &mut mismatches,
        );

        if mismatches.is_empty() {
            Ok(())
        } else {
            mismatches.sort_by(|left, right| {
                left.key
                    .cmp(&right.key)
                    .then_with(|| left.category.cmp(right.category))
            });
            Err(mismatches
                .into_iter()
                .map(|mismatch| {
                    ResolveError::new(format!(
                        "{} ID mismatch: key {:?}, payload {:?}",
                        mismatch.category, mismatch.key, mismatch.payload_id
                    ))
                })
                .collect())
        }
    }
}

struct ItemIdMismatch {
    key: DefId,
    payload_id: DefId,
    category: &'static str,
}

fn validate_item_ids<T>(
    items: &HashMap<DefId, T>,
    id_of: impl Fn(&T) -> DefId,
    category: &'static str,
    mismatches: &mut Vec<ItemIdMismatch>,
) {
    for (key, item) in items {
        let payload_id = id_of(item);
        if *key != payload_id {
            mismatches.push(ItemIdMismatch {
                key: *key,
                payload_id,
                category,
            });
        }
    }
}

fn validate_method_ids(hir: &PartialHir) -> Result<(), Vec<ResolveError>> {
    let mut errors = Vec::new();
    let mut seen = HashMap::<DefId, String>::new();

    let mut record = |id: DefId, owner: String, errors: &mut Vec<ResolveError>| {
        if id.crate_id == CrateId(u32::MAX) {
            errors.push(ResolveError::new(format!(
                "provisional method identity is not allowed for {owner}"
            )));
            return;
        }

        if let Some(previous_owner) = seen.get(&id) {
            if previous_owner != &owner {
                errors.push(ResolveError::new(format!(
                    "duplicate method identity {:?} used by {} and {}",
                    id, previous_owner, owner
                )));
            }
        } else {
            seen.insert(id, owner);
        }
    };

    let mut trait_ids = hir.traits.keys().copied().collect::<Vec<_>>();
    trait_ids.sort();
    for id in trait_ids {
        let trait_def = &hir.traits[&id];
        let mut methods = trait_def
            .methods
            .iter()
            .map(|(name, method)| (method.id, name))
            .collect::<Vec<_>>();
        methods.sort_by_key(|(id, _)| *id);
        for (_, method_name) in methods {
            let method = &trait_def.methods[method_name];
            record(
                method.id,
                format!("trait {}.{}", trait_def.id.local.raw(), method_name),
                &mut errors,
            );
        }
        let mut signatures = trait_def
            .signatures
            .iter()
            .map(|(name, signature)| (signature.id, name))
            .collect::<Vec<_>>();
        signatures.sort_by_key(|(id, _)| *id);
        for (_, signature_name) in signatures {
            let signature = &trait_def.signatures[signature_name];
            record(
                signature.id,
                format!("trait {}.{}", trait_def.id.local.raw(), signature_name),
                &mut errors,
            );
        }
    }

    let mut impl_ids = hir.impls.keys().copied().collect::<Vec<_>>();
    impl_ids.sort();
    for id in impl_ids {
        let imp = &hir.impls[&id];
        let mut methods = imp
            .methods
            .iter()
            .map(|(name, method)| (method.id, name))
            .collect::<Vec<_>>();
        methods.sort_by_key(|(id, _)| *id);
        for (_, method_name) in methods {
            let method = &imp.methods[method_name];
            record(
                method.id,
                format!("impl {}.{}", imp.id.local.raw(), method_name),
                &mut errors,
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn canonical_names_for_hir(hir: &PartialHir) -> HashMap<DefId, String> {
    hir.resolver.item_names_by_id.clone()
}

fn hir_name_tables(
    resolver: &ResolverTables,
    functions: &HashMap<DefId, HirFunction>,
    structs: &HashMap<DefId, HirStruct>,
    enums: &HashMap<DefId, HirEnum>,
    traits: &HashMap<DefId, HirTrait>,
    externs: &HashMap<DefId, HirExtern>,
    type_aliases: &HashMap<DefId, HirTypeAlias>,
) -> HirNameTables {
    fn names_for<T>(resolver: &ResolverTables, ids: &HashMap<DefId, T>) -> HashMap<String, DefId> {
        let mut names = HashMap::new();
        for (name, id) in &resolver.item_paths {
            if ids.contains_key(id) {
                names.insert(name.clone(), *id);
            }
        }
        for (id, name) in &resolver.item_names_by_id {
            if ids.contains_key(id) {
                names.entry(name.clone()).or_insert(*id);
            }
        }
        for aliases in [
            &resolver.import_aliases,
            &resolver.export_aliases,
            &resolver.module_aliases,
        ] {
            for (name, id) in aliases {
                if ids.contains_key(id) {
                    names.entry(name.clone()).or_insert(*id);
                }
            }
        }
        names
    }

    HirNameTables {
        functions_by_name: names_for(resolver, functions),
        structs_by_name: names_for(resolver, structs),
        enums_by_name: names_for(resolver, enums),
        traits_by_name: names_for(resolver, traits),
        externs_by_name: names_for(resolver, externs),
        type_aliases_by_name: names_for(resolver, type_aliases),
    }
}

fn hir_program_from_partial(
    hir: PartialHir,
    canonical_names_by_id: &HashMap<DefId, String>,
) -> (
    HirProgram,
    ResolverTables,
    BTreeSet<DefId>,
    crate::ids::CrateId,
    IdGen<LocalDefId>,
) {
    let PartialHir {
        functions,
        structs,
        enums,
        traits,
        impls,
        externs,
        type_aliases,
        engine: _,
        function_type_vars: _,
        import_aliases: _,
        loaded_module_paths: _,
        constraint_store: _,
        resolver,
        current_def_ids,
        root_crate_id,
        local_def_ids,
        language_items,
        imported_effective_trait_methods,
    } = hir;

    let names = hir_name_tables(
        &resolver,
        &functions,
        &structs,
        &enums,
        &traits,
        &externs,
        &type_aliases,
    );

    let mut program = HirProgram::from_id_parts_with_names_and_canonical_names(
        functions,
        structs,
        enums,
        traits,
        impls,
        externs,
        names,
        language_items,
        canonical_names_by_id,
    );
    program.type_aliases = type_aliases;
    program
        .indexes
        .effective_trait_methods
        .extend(imported_effective_trait_methods);

    (
        program,
        resolver,
        current_def_ids,
        root_crate_id,
        local_def_ids,
    )
}

/// Finalize types: apply inference substitutions and generalize.
/// Consumes PartialHir and produces a fully-typed HirProgram.
pub fn finalize(mut hir: PartialHir) -> Result<ResolvedHirProgram, Vec<ResolveError>> {
    hir.validate_item_ids()?;
    validate_method_ids(&hir)?;
    // Phase 4: solve accumulated constraints; trait violations on concrete types are hard errors.
    solve_pending_constraints(&mut hir)?;

    // Phase 4.5: apply numeric defaults only where literal constraints provide evidence.
    hir.engine.apply_numeric_defaults(&hir.constraint_store);

    // First materialize obligations whose rigid heads are already known.  This
    // resolves inferred callees before propagating their solved schemes into
    // call-site instances, while leaving genuinely deferred Try obligations
    // for the fixed-point pass below.
    authority::materialize_pending_authorities(&mut hir, false, false)?;
    authority::propagate_function_instances(&mut hir)?;

    solve_pending_constraints(&mut hir)?;
    hir.engine.apply_numeric_defaults(&hir.constraint_store);

    authority::materialize_pending_authorities(&mut hir, false, true)?;

    // Phase 5: generalize free type variables into generic parameters
    generalize::generalize_all_functions(&mut hir);

    // Phase 6: finalize — replace all remaining TypeVars
    let errors = finalize::apply_finalization(&mut hir);
    if !errors.is_empty() {
        return Err(errors);
    }
    authority::materialize_pending_authorities(&mut hir, true, true)?;

    let canonical_names_by_id = canonical_names_for_hir(&hir);
    let normalization_env = hir.engine.normalization_env();
    let (program, resolver, current_def_ids, root_crate_id, local_def_ids) =
        hir_program_from_partial(hir, &canonical_names_by_id);
    let authority_errors = program.validate_method_authorities();
    if !authority_errors.is_empty() {
        return Err(authority_errors
            .into_iter()
            .map(ResolveError::new)
            .collect());
    }
    let program = AcceptedHirProgram::try_from(program).map_err(|errors| {
        errors
            .into_iter()
            .map(ResolveError::new)
            .collect::<Vec<_>>()
    })?;
    Ok(ResolvedHirProgram::from_accepted_with_normalization_env(
        program,
        resolver,
        current_def_ids,
        root_crate_id,
        local_def_ids,
        normalization_env,
    ))
}

fn solve_pending_constraints(hir: &mut PartialHir) -> Result<(), Vec<ResolveError>> {
    let solve_result = solve::solve_constraints(
        &mut hir.engine,
        &hir.constraint_store,
        &hir.impls,
        &hir.structs,
        &hir.enums,
        &hir.traits,
        &hir.language_items,
    );
    for warning in &solve_result.warnings {
        eprintln!("{}", warning);
    }
    if solve_result.errors.is_empty() {
        Ok(())
    } else {
        Err(solve_result
            .errors
            .into_iter()
            .map(ResolveError::new)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hir::{
        HirBlock, HirExpr, HirExprKind, HirImpl, HirImplOwner, HirImplReceiverPattern, HirParam,
        HirStmt, HirStruct, HirTrait, HirTypeLocation,
    };
    use crate::types::Type;

    fn empty_function(id: DefId, name: &str) -> HirFunction {
        HirFunction {
            id,
            name: name.to_string(),
            generic_params: Vec::new(),
            generic_bounds: HashMap::new().into(),
            params: Vec::new(),
            ret_type: Type::Unit,
            body: HirBlock {
                stmts: Vec::new(),
                ty: Type::Unit,
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }

    fn partial_hir_with_traits(traits: HashMap<DefId, HirTrait>) -> PartialHir {
        PartialHir {
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits,
            impls: HashMap::new(),
            externs: HashMap::new(),
            type_aliases: HashMap::new(),
            engine: InferenceEngine::new(),
            function_type_vars: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver: ResolverTables::default(),
            current_def_ids: BTreeSet::new(),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
            language_items: HirLanguageItems::default(),
            imported_effective_trait_methods: HashMap::new(),
        }
    }

    #[test]
    fn strict_materialization_rejects_targetless_renamed_method_call() {
        let trait_id = DefId::new(CrateId(0), LocalDefId(10));
        let member_id = DefId::new(CrateId(0), LocalDefId(11));
        let impl_id = DefId::new(CrateId(0), LocalDefId(12));
        let method_id = DefId::new(CrateId(0), LocalDefId(13));
        let receiver_id = DefId::new(CrateId(0), LocalDefId(14));

        let receiver_ty = Type::Struct {
            id: receiver_id,
            args: Vec::new(),
        };
        let method = HirFunction {
            id: method_id,
            name: "legacy_name".to_string(),
            generic_params: Vec::new(),
            generic_bounds: HashMap::new().into(),
            params: vec![HirParam {
                name: "self".to_string(),
                local_id: crate::ids::HirLocalId(0),
                ty: receiver_ty.clone(),
                mutable: false,
                is_ref: false,
            }],
            ret_type: Type::Unit,
            body: HirBlock {
                stmts: Vec::new(),
                ty: Type::Unit,
            },
            is_curried: false,
            is_method: true,
            self_receiver: Some(crate::types::ReceiverMode::Move),
            is_unsafe: false,
        };
        let trait_def = HirTrait {
            target: None,
            predicates: Vec::new(),
            id: trait_id,
            name: "RenamedProtocol".to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods: HashMap::from([("source_name".to_string(), {
                let mut signature = method.clone();
                signature.id = member_id;
                signature.name = "source_name".to_string();
                signature
            })]),
            signatures: HashMap::new(),
        };
        let imp = HirImpl {
            id: impl_id,
            owner: HirImplOwner::Named("Owner".to_string()),
            type_name: "Owner".to_string(),
            type_generics: Vec::new(),
            receiver_pattern: HirImplReceiverPattern::Exact(receiver_ty.clone()),
            trait_name: Some("RenamedProtocol".to_string()),
            trait_id: Some(trait_id),
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: HashMap::new().into(),
            methods: HashMap::from([("legacy_name".to_string(), method)]),
        };
        let mut function = empty_function(DefId::new(CrateId(0), LocalDefId(15)), "caller");
        function.body.stmts.push(HirStmt::Expr(HirExpr {
            kind: HirExprKind::MethodCall(
                Box::new(HirExpr {
                    kind: HirExprKind::Var("owner".to_string()),
                    ty: receiver_ty.clone(),
                    span: Default::default(),
                }),
                "source_name".to_string(),
                Vec::new(),
                Some(crate::types::ReceiverMode::Move),
                None,
            ),
            ty: Type::Unit,
            span: Default::default(),
        }));

        let mut partial = partial_hir_with_traits(HashMap::from([(trait_id, trait_def)]));
        partial.functions.insert(function.id, function);
        partial.impls.insert(impl_id, imp);
        partial
            .imported_effective_trait_methods
            .insert((impl_id, member_id), method_id);

        let errors = authority::materialize_pending_authorities(&mut partial, true, true)
            .expect_err("strict inference must reject targetless method dispatch");
        assert!(errors.iter().any(|error| {
            error
                .message
                .contains("accepted HIR method call has no selected authority")
        }));
    }

    #[test]
    fn finalize_rejects_duplicate_impl_method_ids_instead_of_repairing_them() {
        let duplicate_method_id = DefId::new(CrateId(0), LocalDefId(8));
        let mut partial = partial_hir_with_traits(HashMap::new());
        for (impl_id, type_name) in [
            (DefId::new(CrateId(0), LocalDefId(20)), "First"),
            (DefId::new(CrateId(0), LocalDefId(21)), "Second"),
        ] {
            partial.impls.insert(
                impl_id,
                HirImpl {
                    id: impl_id,
                    owner: HirImplOwner::Named(type_name.to_string()),
                    type_name: type_name.to_string(),
                    type_generics: Vec::new(),
                    receiver_pattern: Vec::new().into(),
                    trait_name: None,
                    trait_id: None,
                    trait_generics: Vec::new(),
                    trait_arg_types: Vec::new(),
                    associated_types: Vec::new(),
                    bounds: std::collections::HashMap::new().into(),
                    methods: HashMap::from([(
                        "id".to_string(),
                        empty_function(duplicate_method_id, "id"),
                    )]),
                },
            );
        }

        let errors = match finalize(partial) {
            Ok(_) => panic!("duplicate method IDs should not be repaired during inference"),
            Err(errors) => errors,
        };

        assert!(errors
            .iter()
            .any(|error| error.message.contains("duplicate method identity")));
    }

    #[test]
    fn finalization_exempts_only_resolver_root_entrypoint_from_generalization() {
        let main_id = DefId::new(CrateId(0), LocalDefId(60));
        let nested_id = DefId::new(CrateId(0), LocalDefId(61));
        let mut engine = InferenceEngine::new();
        let Type::TypeVar(main_var) = engine.fresh_type_var_at(Default::default()) else {
            panic!("fresh inference variable should be a TypeVar");
        };
        let Type::TypeVar(nested_var) = engine.fresh_type_var_at(Default::default()) else {
            panic!("fresh inference variable should be a TypeVar");
        };
        let mut main = empty_function(main_id, "main");
        main.ret_type = Type::TypeVar(main_var);
        main.body.ty = Type::TypeVar(main_var);
        let mut nested = empty_function(nested_id, "main");
        nested.ret_type = Type::TypeVar(nested_var);
        nested.body.ty = Type::TypeVar(nested_var);
        let mut resolver = ResolverTables::default();
        resolver.item_paths.insert("main".to_string(), main_id);
        let mut partial = PartialHir {
            functions: HashMap::from([(main_id, main), (nested_id, nested)]),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            externs: HashMap::new(),
            type_aliases: HashMap::new(),
            engine,
            function_type_vars: HashMap::from([
                (main_id, HashSet::from([main_var])),
                (nested_id, HashSet::from([nested_var])),
            ]),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver,
            current_def_ids: BTreeSet::from([main_id, nested_id]),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
            language_items: HirLanguageItems::default(),
            imported_effective_trait_methods: HashMap::new(),
        };

        generalize::generalize_all_functions(&mut partial);

        assert!(partial.functions[&main_id].generic_params.is_empty());
        assert_eq!(
            partial.functions[&nested_id]
                .generic_params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>(),
            vec!["T"]
        );
        assert!(finalize(partial).is_err());
    }

    #[test]
    fn finalization_item_id_errors_are_sorted() {
        fn partial(reverse_functions: bool) -> PartialHir {
            let first_key = DefId::new(CrateId(0), LocalDefId(1));
            let second_key = DefId::new(CrateId(0), LocalDefId(2));
            let mut functions = HashMap::new();
            let entries = [
                (
                    first_key,
                    empty_function(DefId::new(CrateId(0), LocalDefId(11)), "first"),
                ),
                (
                    second_key,
                    empty_function(DefId::new(CrateId(0), LocalDefId(12)), "second"),
                ),
            ];
            for (key, function) in if reverse_functions {
                entries.into_iter().rev().collect::<Vec<_>>()
            } else {
                entries.into_iter().collect()
            } {
                functions.insert(key, function);
            }
            let structure = HirStruct {
                id: DefId::new(CrateId(0), LocalDefId(13)),
                name: "structure".to_string(),
                generic_params: Vec::new(),
                fields: Vec::new(),
            };

            PartialHir {
                functions,
                structs: HashMap::from([(first_key, structure)]),
                enums: HashMap::new(),
                traits: HashMap::new(),
                impls: HashMap::new(),
                externs: HashMap::new(),
                type_aliases: HashMap::new(),
                engine: InferenceEngine::new(),
                function_type_vars: HashMap::new(),
                import_aliases: HashMap::new(),
                loaded_module_paths: vec![],
                constraint_store: ConstraintStore::default(),
                resolver: ResolverTables::default(),
                current_def_ids: BTreeSet::new(),
                root_crate_id: crate::ids::CrateId(0),
                local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
                language_items: HirLanguageItems::default(),
                imported_effective_trait_methods: HashMap::new(),
            }
        }

        let strict = |reverse| {
            finalize(partial(reverse))
                .expect_err("strict finalization should reject item ID mismatches")
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
        };
        let expected = vec![
            "function declaration ID mismatch: key DefId { crate_id: CrateId(0), local: LocalDefId(1) }, payload DefId { crate_id: CrateId(0), local: LocalDefId(11) }".to_string(),
            "struct declaration ID mismatch: key DefId { crate_id: CrateId(0), local: LocalDefId(1) }, payload DefId { crate_id: CrateId(0), local: LocalDefId(13) }".to_string(),
            "function declaration ID mismatch: key DefId { crate_id: CrateId(0), local: LocalDefId(2) }, payload DefId { crate_id: CrateId(0), local: LocalDefId(12) }".to_string(),
        ];

        assert_eq!(strict(false), expected);
        assert_eq!(strict(true), expected);
    }

    #[test]
    fn finalize_reports_impl_method_identity_errors_in_def_id_order() {
        fn diagnostics(reverse_insertion: bool) -> Vec<String> {
            let duplicate_method_id = DefId::new(CrateId(0), LocalDefId(8));
            let first_impl_id = DefId::new(CrateId(0), LocalDefId(20));
            let second_impl_id = DefId::new(CrateId(0), LocalDefId(21));
            let mut partial = partial_hir_with_traits(HashMap::new());
            let impls = [
                (
                    first_impl_id,
                    HirImpl {
                        id: first_impl_id,
                        owner: HirImplOwner::Named("First".to_string()),
                        type_name: "First".to_string(),
                        type_generics: Vec::new(),
                        receiver_pattern: Vec::new().into(),
                        trait_name: None,
                        trait_id: None,
                        trait_generics: Vec::new(),
                        trait_arg_types: Vec::new(),
                        associated_types: Vec::new(),
                        bounds: HashMap::new().into(),
                        methods: HashMap::from([
                            (
                                "later".to_string(),
                                empty_function(duplicate_method_id, "later"),
                            ),
                            (
                                "earlier".to_string(),
                                empty_function(DefId::new(CrateId(0), LocalDefId(7)), "earlier"),
                            ),
                        ]),
                    },
                ),
                (
                    second_impl_id,
                    HirImpl {
                        id: second_impl_id,
                        owner: HirImplOwner::Named("Second".to_string()),
                        type_name: "Second".to_string(),
                        type_generics: Vec::new(),
                        receiver_pattern: Vec::new().into(),
                        trait_name: None,
                        trait_id: None,
                        trait_generics: Vec::new(),
                        trait_arg_types: Vec::new(),
                        associated_types: Vec::new(),
                        bounds: HashMap::new().into(),
                        methods: HashMap::from([(
                            "duplicate".to_string(),
                            empty_function(duplicate_method_id, "duplicate"),
                        )]),
                    },
                ),
            ];
            for (id, imp) in if reverse_insertion {
                impls.into_iter().rev().collect::<Vec<_>>()
            } else {
                impls.into_iter().collect()
            } {
                partial.impls.insert(id, imp);
            }

            finalize(partial)
                .expect_err("duplicate method identity should be rejected")
                .into_iter()
                .map(|error| error.message)
                .collect()
        }

        assert_eq!(diagnostics(false), diagnostics(true));
    }

    #[test]
    fn finalize_preserves_resolver_tables() {
        let partial = PartialHir {
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            externs: HashMap::new(),
            type_aliases: HashMap::new(),
            engine: InferenceEngine::new(),
            function_type_vars: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver: ResolverTables::default(),
            current_def_ids: BTreeSet::new(),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
            language_items: HirLanguageItems::default(),
            imported_effective_trait_methods: HashMap::new(),
        };

        let resolved = finalize(partial).unwrap();

        assert!(resolved.resolver.module_paths.is_empty());
        assert!(resolved.program.functions.is_empty());
    }

    #[test]
    fn resolved_hir_reports_missing_required_type_id_location() {
        let resolved = finalize(partial_hir_with_traits(HashMap::new())).unwrap();
        let missing = HirTypeLocation::FunctionReturn {
            function: DefId::new(CrateId(0), LocalDefId(999)),
        };

        let err = resolved.require_type_id_at(missing).unwrap_err();

        assert!(err.message.contains("missing finalized HIR TypeId"));
    }

    #[test]
    fn finalize_builds_resolved_hir_type_context_and_sidecar() {
        let function_id = DefId::new(CrateId(0), LocalDefId(1));
        let mut partial = PartialHir {
            functions: HashMap::from([(
                function_id,
                HirFunction {
                    id: function_id,
                    name: "identity".to_string(),
                    generic_params: Vec::new(),
                    generic_bounds: HashMap::new().into(),
                    params: vec![HirParam {
                        name: "value".to_string(),
                        local_id: crate::ids::HirLocalId(0),
                        ty: Type::I64,
                        mutable: false,
                        is_ref: false,
                    }],
                    ret_type: Type::I64,
                    body: HirBlock {
                        stmts: vec![HirStmt::Expr(HirExpr {
                            kind: HirExprKind::Var("value".to_string()),
                            ty: Type::I64,
                            span: Default::default(),
                        })],
                        ty: Type::I64,
                    },
                    is_curried: false,
                    is_method: false,
                    self_receiver: None,
                    is_unsafe: false,
                },
            )]),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            externs: HashMap::new(),
            type_aliases: HashMap::new(),
            engine: InferenceEngine::new(),
            function_type_vars: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver: ResolverTables::default(),
            current_def_ids: BTreeSet::from([function_id]),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
            language_items: HirLanguageItems::default(),
            imported_effective_trait_methods: HashMap::new(),
        };
        partial.local_def_ids.fresh();
        partial.local_def_ids.fresh();

        let resolved = finalize(partial).unwrap();
        let ret_id = resolved
            .type_id_at(&HirTypeLocation::FunctionReturn {
                function: function_id,
            })
            .unwrap();
        let param_id = resolved
            .type_id_at(&HirTypeLocation::FunctionParam {
                function: function_id,
                index: 0,
            })
            .unwrap();
        let expr_id = resolved
            .type_id_at(&HirTypeLocation::Expr {
                owner: function_id,
                path: vec![0, 0],
            })
            .unwrap();

        assert_eq!(ret_id, param_id);
        assert_eq!(param_id, expr_id);
        assert_eq!(resolved.type_at(ret_id), Type::I64);
    }

    #[test]
    fn finalize_preserves_id_keyed_function_payload_and_resolver_aliases() {
        let id = DefId::new(CrateId(0), LocalDefId(50));
        let mut resolver = ResolverTables::default();
        resolver
            .item_names_by_id
            .insert(id, "package::canonical_answer".to_string());
        resolver
            .item_paths
            .insert("package::canonical_answer".to_string(), id);
        resolver
            .import_aliases
            .insert("imported_answer".to_string(), id);

        let partial = PartialHir {
            functions: HashMap::from([(id, empty_function(id, "payload_display"))]),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            externs: HashMap::new(),
            type_aliases: HashMap::new(),
            engine: InferenceEngine::new(),
            function_type_vars: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver,
            current_def_ids: BTreeSet::new(),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
            language_items: HirLanguageItems::default(),
            imported_effective_trait_methods: HashMap::new(),
        };

        let resolved = finalize(partial).expect("ID-keyed payload should finalize");

        assert_eq!(resolved.program.functions[&id].name, "payload_display");
        assert_eq!(
            resolved.program.names.functions_by_name["imported_answer"],
            id
        );
        assert_eq!(
            resolved.program.function_by_id(id).map(|(name, _)| name),
            Some("package::canonical_answer")
        );
    }

    #[test]
    fn finalize_reports_function_key_payload_id_mismatch() {
        let key = DefId::new(CrateId(0), LocalDefId(51));
        let payload = DefId::new(CrateId(0), LocalDefId(52));
        let partial = PartialHir {
            functions: HashMap::from([(key, empty_function(payload, "payload_display"))]),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            externs: HashMap::new(),
            type_aliases: HashMap::new(),
            engine: InferenceEngine::new(),
            function_type_vars: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver: ResolverTables::default(),
            current_def_ids: BTreeSet::new(),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
            language_items: HirLanguageItems::default(),
            imported_effective_trait_methods: HashMap::new(),
        };

        let errors = finalize(partial).expect_err("mismatched function IDs should be rejected");

        assert!(errors
            .iter()
            .any(|error| error.message.contains("function declaration ID mismatch")));
    }
}
