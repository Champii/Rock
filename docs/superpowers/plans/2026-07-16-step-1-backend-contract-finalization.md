# Step 1 Backend Contract Finalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Track work
> in bead `new_lang2-cnb`; repository policy forbids a duplicate Markdown task
> checklist.

**Goal:** Remove the remaining MIR backend-contract staging layer and require
complete backend-contract validation in every MIR builder test without
regressing Clean-Slate Audit Steps 2–5.

**Architecture:** `MirBuilder` will allocate `MirBackendContract` first and
populate each section directly from accepted HIR, mono instances, MIR bodies,
and canonical IDs. Projection discovery will inspect accepted impl authority and
contract-owned type IDs rather than metadata rows. Agreement will have one full
validation entry point, and test helpers will create complete contract-native
fixtures.

**Tech Stack:** Rust 2021, `rock-lib`, accepted HIR, monomorphization instance
records, MIR, `MirBackendContract`, `TypeContext`, Cargo tests, rustfmt, Clippy,
beads (`bd`).

---

## File Map

- Modify `lib/src/mir/mod.rs`: delete staging row types; retain canonical MIR
  and contract payload types only.
- Modify `lib/src/mir/builder/mod.rs`: construct contract sections directly,
  resolve projections from accepted HIR impls, and build complete test fixtures.
- Modify `lib/src/mir/agreement.rs`: remove partial-validation mode and migrate
  its isolated regression to full agreement.
- Modify `lib/src/semantic_identity_audit.rs`: guard Step 1 and Step 2–5 residue.
- Modify `CLEAN_SLATE_COMPILER_AUDIT.md`: correct Step 1 evidence only after all
  gates pass.
- Reference, but do not change unless a regression requires it:
  `lib/src/mir/backend_contract.rs`, `lib/src/lib.rs`, `lib/src/products.rs`,
  `lib/src/lower/paths.rs`, and `lib/src/crate_artifact/load.rs`.

## Task 1: Add Step 1 Residue Guards

**Files:**

- Modify `lib/src/semantic_identity_audit.rs:1024-1142`

**Step 1: Add a failing source-contract test**

Add this test next to the existing backend and method-authority audits:

```rust
#[test]
fn mir_backend_contract_has_no_staging_or_partial_validation_path() {
    let mir = production_source("mir/mod.rs");
    assert_absent(
        &mir,
        &[
            "pub struct MirDropGlue",
            "pub struct MirProjectionResolution",
            "pub struct MirInstanceDeclaration",
            "pub struct MirProjectionImplMetadata",
            "pub struct MirExternDeclaration",
            "pub struct MirProductLinkCandidate",
            "pub struct MirStructLayout",
            "pub struct MirEnumLayout",
        ],
    );

    let builder = production_source("mir/builder/mod.rs");
    assert_absent(
        &builder,
        &[
            "struct_display_aliases",
            "enum_display_aliases",
            "canonical: false",
            "check_mir_runtime_agreement_for_mir_only",
        ],
    );

    let agreement = production_source("mir/agreement.rs");
    assert_absent(
        &agreement,
        &[
            "check_mir_runtime_agreement_for_mir_only",
            "validate_backend_contract: bool",
        ],
    );
}
```

The guard is intentionally limited to the exact Step 1 staging and validation
paths. It does not ban `MirEnumVariantLayout`, `MirVariantLayoutFields`, display
names used inside diagnostics, or canonical `MirArtifactExport`.

**Step 2: Verify RED**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::mir_backend_contract_has_no_staging_or_partial_validation_path -- --exact --nocapture
```

Expected: FAIL listing the staging structs, display-alias traversal, and the
MIR-only agreement entry point.

**Step 3: Record the RED evidence on the bead**

Run:

```bash
bd update new_lang2-cnb --claim --json
bd comments add new_lang2-cnb "RED: semantic_identity_audit::mir_backend_contract_has_no_staging_or_partial_validation_path fails on the remaining staging rows, alias traversal, and partial agreement path." --json
```

Expected: bead status is `in_progress` and the RED evidence is recorded.

## Task 2: Populate Non-Projection Contract Sections Directly

**Files:**

- Modify `lib/src/mir/builder/mod.rs:312-410,662-1010`
- Modify `lib/src/mir/mod.rs:116-296`
- Test `lib/src/mir/builder/mod.rs:3571-3824,4590-4992`

**Step 1: Strengthen the canonical-layout regression before production edits**

Rename
`backend_contract_records_nominal_layout_from_hir_display_helpers` to
`backend_contract_records_one_canonical_layout_without_visiting_display_aliases`
so both nominal kinds prove that aliases cannot create contract rows:

```rust
assert_eq!(contract.nominal_layouts.len(), 2);
assert!(matches!(
    contract.nominal_layouts.get(&struct_id),
    Some(crate::mir::MirNominalLayout::Struct { id, .. }) if *id == struct_id
));
assert!(matches!(
    contract.nominal_layouts.get(&enum_id),
    Some(crate::mir::MirNominalLayout::Enum { id, .. }) if *id == enum_id
));
```

This behavioral test may already pass; the Task 1 source guard supplies the RED
for alias traversal itself.

Add a second behavioral regression for the existing invented-signature
fallback:

```rust
#[test]
#[should_panic(expected = "has neither a MIR body nor a declared signature")]
fn backend_contract_rejects_instance_without_body_or_declared_signature() {
    let instance_id = InstanceId(90);
    let function_id = DefId::new(CrateId(0), LocalDefId(90));
    let mono = crate::mono::MonomorphizedProgram {
        program: empty_program(),
        instances: std::collections::BTreeMap::from([(
            instance_id,
            crate::mono::InstanceRecord {
                id: instance_id,
                origin: crate::mono::InstanceOrigin::Function(function_id),
                substitution: Vec::new(),
                symbols: crate::mono::InstanceSymbols::new("missing", "missing"),
                declared: None,
                provided_by_object: true,
                is_specialization: false,
            },
        )]),
        pre_mir_instance_bodies: crate::mono::PreMirInstanceBodies::new(),
        generated_drop_instances: Default::default(),
        type_context: TypeContext::new(),
    };
    let mut type_context = TypeContext::new();

    MirBuilder::backend_contract_for_program(
        &mono,
        &std::collections::BTreeMap::new(),
        &mut type_context,
        &MirInstanceBodies::new(),
    );
}
```

Run it before production edits. Expected: FAIL because the current staging path
invents a zero-parameter `Unit` signature instead of rejecting the malformed
instance.

**Step 2: Extract direct section-population helpers**

Replace staging-vector construction with helpers that mutate the contract:

```rust
fn populate_backend_contract_externs(
    contract: &mut super::MirBackendContract,
    program: &HirProgram,
    type_context: &mut TypeContext,
) {
    for (_, ext) in program.externs_in_order() {
        let key = super::MirCallableKey::Extern(ext.id);
        let params = ext
            .params
            .iter()
            .map(|ty| type_context.intern_type(ty))
            .collect::<Vec<_>>();
        let ret = type_context.intern_type(&ext.ret);
        let declaration = super::MirCallableDecl {
            key: key.clone(),
            source_def_id: Some(ext.id),
            kind: super::MirCallableKind::Extern {
                link_name: ext.name.clone(),
                variadic: ext.variadic,
            },
            llvm_symbol: ext.name.clone(),
            linkage: super::MirLinkage::External,
            signature: super::MirCallableSignature::from_type_ids(
                &params,
                ret,
                super::MirPassMode::Direct,
            ),
        };
        assert!(
            contract.callables.insert(key, declaration).is_none(),
            "duplicate extern callable authority for {:?}",
            ext.id,
        );
    }
}

fn populate_backend_contract_nominal_layouts(
    contract: &mut super::MirBackendContract,
    program: &HirProgram,
    type_context: &mut TypeContext,
) {
    for (id, _, structure) in program.structs_by_id() {
        let previous = contract.nominal_layouts.insert(
            id,
            super::MirNominalLayout::Struct {
                id,
                fields: structure
                    .fields
                    .iter()
                    .map(|field| {
                        (field.name.clone(), type_context.intern_type(&field.ty))
                    })
                    .collect(),
                generic_params: Self::mir_generic_param_ids(
                    id,
                    structure.generic_params.len(),
                ),
            },
        );
        assert!(previous.is_none(), "duplicate struct layout authority for {id:?}");
    }

    for (id, _, enum_def) in program.enums_by_id() {
        let previous = contract.nominal_layouts.insert(
            id,
            super::MirNominalLayout::Enum {
                id,
                variants: Self::mir_enum_variants(type_context, &enum_def.variants),
                generic_params: Self::mir_generic_param_ids(
                    id,
                    enum_def.generic_params.len(),
                ),
            },
        );
        assert!(previous.is_none(), "duplicate enum layout authority for {id:?}");
    }
}
```

Do not pass canonical/display names into either helper. `DefId` is the only
nominal key.

**Step 3: Insert instance callables and artifact exports in one authoritative traversal**

Before traversing instances, compute only the linkage decision set:

```rust
let exportable_drop_glue_symbols = program
    .instances
    .values()
    .filter(|record| {
        Self::instance_is_stdlib_drop_method(program, record)
            && record.substitution.is_empty()
            && bodies.contains_key(record.id)
            && !record.provided_by_object
    })
    .map(|record| record.symbols.backend_symbol.as_str())
    .collect::<std::collections::BTreeSet<_>>();
```

Then iterate `program.instances.values()` once. Derive `(params, ret,
is_method)` from `MirInstanceBodies` or `record.declared`, construct the
`MirCallableDecl`, insert its body mapping when local, and immediately push the
corresponding `MirArtifactExport`:

```rust
contract.artifact_exports.push(super::MirArtifactExport {
    origin_def_id: Some(Self::instance_origin_callable_def_id(&record.origin)),
    source_name: record.symbols.source_name.clone(),
    backend_symbol: record.symbols.backend_symbol.clone(),
    substitution_empty: record.substitution.is_empty(),
    has_body: bodies.contains_key(record.id),
    provided_by_object: record.provided_by_object,
    is_specialization: record.is_specialization,
    is_drop_glue: Self::instance_is_stdlib_drop_method(program, record),
});
```

Do not retain the current `Vec::new()`/`Unit` fallback for an instance with
neither a MIR body nor `record.declared`. Reject that broken mono invariant at
the source:

```rust
let (params, ret, is_method) = if let Some(body) = bodies.get(record.id) {
    (
        body.function
            .local_decls
            .iter()
            .skip(1)
            .take(body.function.arg_count)
            .map(|local| local.ty)
            .collect(),
        body.function.ret_type,
        body.is_method,
    )
} else if let Some(function) = record.declared.as_ref() {
    (
        function
            .params
            .iter()
            .map(|param| type_context.intern_type(&param.ty))
            .collect(),
        type_context.intern_type(&function.ret_type),
        function.is_method,
    )
} else {
    panic!(
        "instance {:?} has neither a MIR body nor a declared signature",
        record.id,
    );
};
```

When inserting each instance callable, assert that the canonical instance key
was not already present. Do not silently overwrite conflicting authority.

Use `record.origin` and `record.id` for identity. Do not inspect source names to
choose callable keys or linkage.

**Step 4: Insert drop glue directly**

Replace `mir_drop_glue(...) -> Vec<MirDropGlue>` with direct insertion:

```rust
fn populate_backend_contract_drop_glue(
    contract: &mut super::MirBackendContract,
    generated: &std::collections::BTreeMap<
        TypeId,
        crate::mono::GeneratedMethodInstance,
    >,
) {
    for entry in generated.values() {
        contract.drop_glue.insert(
            entry.receiver_ty,
            super::MirCallableKey::Instance(entry.instance_id),
        );
    }
}
```

Update `drop_glue_collection_consumes_generated_mono_instance` to construct a
contract, invoke this helper, and assert the map entry:

```rust
assert_eq!(
    contract.drop_glue.get(&ty_id),
    Some(&crate::mir::MirCallableKey::Instance(instance_id)),
);
```

**Step 5: Delete non-projection staging types**

Delete these definitions from `lib/src/mir/mod.rs` and remove their imports and
constructors:

```text
MirDropGlue
MirInstanceDeclaration
MirExternDeclaration
MirProductLinkCandidate
MirStructLayout
MirEnumLayout
```

Retain `MirEnumVariantLayout` and `MirVariantLayoutFields` because
`MirNominalLayout::Enum` owns them directly.

**Step 6: Run focused GREEN checks**

Run sequentially:

```bash
cargo test -p rock-lib mir::builder::tests::backend_contract_records_one_canonical_layout_without_visiting_display_aliases -- --exact --nocapture
cargo test -p rock-lib mir::builder::tests::drop_glue_collection_consumes_generated_mono_instance -- --exact --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
```

Expected: exact tests and the builder suite pass. The Task 1 source guard still
fails only on projection staging and partial agreement until Tasks 3–4 finish.

## Task 3: Resolve Projections Directly From Accepted HIR

**Files:**

- Modify `lib/src/mir/builder/mod.rs:75-109,741-756,1012-1422`
- Modify `lib/src/mir/mod.rs:128-135,231-237`
- Test `lib/src/mir/builder/mod.rs:3826-4426`

**Step 1: Preserve current projection behavior as the RED boundary**

Run the projection-focused tests before editing and record their baseline:

```bash
cargo test -p rock-lib mir::builder::tests::build_monomorphized_lowers_trait_impl_projection_contract -- --exact --nocapture
cargo test -p rock-lib mir::builder::tests::projection_outputs_use_normalized_nested_projection_keys -- --exact --nocapture
cargo test -p rock-lib mir::builder::tests::projection_outputs_emit_builtin_index_output -- --exact --nocapture
```

Expected: PASS. The Task 1 residue guard remains RED on
`MirProjectionImplMetadata` and `MirProjectionResolution`.

**Step 2: Make the projection provider consume accepted HIR directly**

Replace the metadata slice with the accepted program:

```rust
struct MirProjectionResolutionProvider<'a> {
    program: &'a HirProgram,
    builtin_index_trait_ids: &'a [DefId],
}

impl ProjectionProvider for MirProjectionResolutionProvider<'_> {
    fn resolve_projection_output(
        &self,
        base_ty: &Type,
        trait_id: DefId,
        assoc_type_id: AssocTypeId,
        trait_args: &[Type],
    ) -> Option<Type> {
        MirBuilder::resolve_projection_from_program(
            self.program,
            base_ty,
            trait_id,
            assoc_type_id,
            trait_args,
        )
    }

    fn find_projection_impl(
        &self,
        _base_ty: &Type,
        _trait_id: DefId,
        _trait_args: &[Type],
    ) -> Option<ProjectionImpl> {
        None
    }

    fn is_builtin_index_trait(&self, trait_id: DefId) -> bool {
        self.builtin_index_trait_ids.contains(&trait_id)
    }
}
```

**Step 3: Replace metadata matching with accepted-impl matching**

Implement `resolve_projection_from_program` by iterating canonical impls:

```rust
fn resolve_projection_from_program(
    program: &HirProgram,
    base: &Type,
    trait_id: DefId,
    assoc_type_id: AssocTypeId,
    trait_args: &[Type],
) -> Option<Type> {
    let mut matches = program
        .impls_in_order()
        .filter_map(|(_, imp)| {
            (imp.trait_id == Some(trait_id)).then_some(())?;
            let assoc = imp
                .associated_types
                .iter()
                .find(|assoc| assoc.id == assoc_type_id)?;
            let subst = Self::projection_impl_substitution(imp, base, trait_args)?;
            Some((imp.id, assoc.ty.substitute_generics(&subst)))
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|(impl_id, _)| *impl_id);
    matches.dedup_by_key(|(impl_id, _)| *impl_id);
    match matches.as_slice() {
        [(_, output)] => Some(output.clone()),
        _ => None,
    }
}
```

Replace `projection_metadata_base_matches` and
`projection_metadata_substitution` with one ID-based helper over `HirImpl`:

```rust
fn projection_impl_substitution(
    imp: &HirImpl,
    base: &Type,
    trait_args: &[Type],
) -> Option<HashMap<crate::types::GenericParamId, Type>> {
    if imp.trait_arg_types.len() != trait_args.len() {
        return None;
    }
    let mut subst = crate::selection::receiver_pattern_substitution(
        &imp.receiver_pattern,
        base,
    )?;
    imp.trait_arg_types
        .iter()
        .zip(trait_args)
        .all(|(expected, actual)| {
            crate::selection::type_pattern_matches(expected, actual, &mut subst)
        })
        .then_some(subst)
}
```

Preserve current reference/slice-family receiver behavior. If
`receiver_pattern_substitution` does not include the current reference
adjustment supported by `projection_metadata_base_matches`, normalize `base`
through the exact existing `Type::Reference` inner-type branch before calling
it; do not add display-name matching.

**Step 4: Populate projection outputs directly into the contract**

Replace `mir_projection_resolutions(...) -> Vec<MirProjectionResolution>` with:

```rust
fn populate_backend_contract_projection_outputs(
    contract: &mut super::MirBackendContract,
    program: &HirProgram,
    functions: &std::collections::BTreeMap<MirFunctionId, MirFunction>,
    type_context: &mut TypeContext,
    builtin_index_trait_ids: &[DefId],
) {
    let mut used_type_ids = functions
        .values()
        .flat_map(|function| {
            std::iter::once(function.ret_type)
                .chain(function.local_decls.iter().map(|local| local.ty))
        })
        .collect::<Vec<_>>();
    used_type_ids.extend(contract.callables.values().flat_map(|callable| {
        callable
            .signature
            .params
            .iter()
            .map(|param| param.semantic_ty)
            .chain(std::iter::once(callable.signature.ret.semantic_ty))
    }));
    used_type_ids.extend(contract.nominal_layouts.values().flat_map(|layout| {
        match layout {
            super::MirNominalLayout::Struct { fields, .. } => {
                fields.iter().map(|(_, ty)| *ty).collect::<Vec<_>>()
            }
            super::MirNominalLayout::Enum { variants, .. } => variants
                .iter()
                .flat_map(Self::mir_enum_variant_type_ids)
                .collect::<Vec<_>>(),
        }
    }));

    for ty in used_type_ids {
        Self::collect_projection_outputs_for_type_id(
            contract,
            program,
            ty,
            type_context,
            builtin_index_trait_ids,
        );
    }
}
```

Implement the recursive helpers as direct contract insertion:

```rust
fn collect_projection_outputs_for_type_id(
    contract: &mut super::MirBackendContract,
    program: &HirProgram,
    ty: TypeId,
    type_context: &mut TypeContext,
    builtin_index_trait_ids: &[DefId],
) {
    let ty = type_context.type_for(ty);
    Self::collect_projection_outputs_for_type(
        contract,
        program,
        &ty,
        type_context,
        builtin_index_trait_ids,
    );
}

fn collect_projection_outputs_for_type(
    contract: &mut super::MirBackendContract,
    program: &HirProgram,
    ty: &Type,
    type_context: &mut TypeContext,
    builtin_index_trait_ids: &[DefId],
) {
    match ty {
        Type::Projection {
            ty: base,
            trait_id,
            assoc_type,
            trait_args,
        } => {
            Self::collect_projection_outputs_for_type(
                contract,
                program,
                base,
                type_context,
                builtin_index_trait_ids,
            );
            for arg in trait_args {
                Self::collect_projection_outputs_for_type(
                    contract,
                    program,
                    arg,
                    type_context,
                    builtin_index_trait_ids,
                );
            }

            if assoc_type.owner != *trait_id {
                return;
            }
            let provider = MirProjectionResolutionProvider {
                program,
                builtin_index_trait_ids,
            };
            let resolved_base = ProjectionNormalizer::normalize(&provider, base);
            let resolved_trait_args = trait_args
                .iter()
                .map(|arg| ProjectionNormalizer::normalize(&provider, arg))
                .collect::<Vec<_>>();
            let output = Self::resolve_projection_from_program(
                program,
                &resolved_base,
                *trait_id,
                assoc_type.assoc_type_id,
                &resolved_trait_args,
            )
            .or_else(|| {
                (builtin_index_trait_ids.contains(trait_id)
                    && resolved_trait_args.len() == 1)
                    .then(|| {
                        TypeFacts::builtin_index_output(
                            &resolved_base,
                            &resolved_trait_args[0],
                        )
                    })
                    .flatten()
            });
            let Some(output) = output else {
                return;
            };
            let output = ProjectionNormalizer::normalize(&provider, &output);
            let key = super::MirProjectionKey {
                base: type_context.intern_type(&resolved_base),
                trait_id: *trait_id,
                assoc_type_id: assoc_type.assoc_type_id,
                trait_args: resolved_trait_args
                    .iter()
                    .map(|arg| type_context.intern_type(arg))
                    .collect(),
            };
            let output = type_context.intern_type(&output);
            if let Some(previous) = contract.projection_outputs.insert(key.clone(), output) {
                assert_eq!(
                    previous, output,
                    "conflicting projection output authority for {key:?}",
                );
            }
        }
        Type::Reference { inner, .. } | Type::Pointer(inner) | Type::Slice(inner) => {
            Self::collect_projection_outputs_for_type(
                contract,
                program,
                inner,
                type_context,
                builtin_index_trait_ids,
            );
        }
        Type::Array(inner, _) => {
            Self::collect_projection_outputs_for_type(
                contract,
                program,
                inner,
                type_context,
                builtin_index_trait_ids,
            );
        }
        Type::Tuple(elements) => {
            for element in elements {
                Self::collect_projection_outputs_for_type(
                    contract,
                    program,
                    element,
                    type_context,
                    builtin_index_trait_ids,
                );
            }
        }
        Type::Function { params, ret, .. } => {
            for param in params {
                Self::collect_projection_outputs_for_type(
                    contract,
                    program,
                    param,
                    type_context,
                    builtin_index_trait_ids,
                );
            }
            Self::collect_projection_outputs_for_type(
                contract,
                program,
                ret,
                type_context,
                builtin_index_trait_ids,
            );
        }
        Type::Struct { args, .. } | Type::Enum { args, .. } => {
            for arg in args {
                Self::collect_projection_outputs_for_type(
                    contract,
                    program,
                    arg,
                    type_context,
                    builtin_index_trait_ids,
                );
            }
        }
        _ => {}
    }
}
```

**Step 5: Populate projection trait IDs without metadata rows**

Insert directly:

```rust
for (trait_id, _, trait_def) in program.traits_by_id() {
    if !trait_def.methods.is_empty() || !trait_def.signatures.is_empty() {
        contract.projection_traits.insert(trait_id);
    }
}
contract.projection_traits.extend(
    program
        .impls_in_order()
        .filter_map(|(_, imp)| imp.trait_id),
);
contract
    .projection_traits
    .extend(builtin_index_trait_ids.iter().copied());
```

**Step 6: Delete projection staging types and imports**

Delete:

```text
MirProjectionResolution
MirProjectionImplMetadata
```

Remove `ProjectionAssociatedType` imports if they become unused. Keep
`ProjectionImpl` only if required by the `ProjectionProvider` trait signature.

**Step 7: Verify projection GREEN**

Run sequentially:

```bash
cargo test -p rock-lib mir::builder::tests::build_monomorphized_lowers_trait_impl_projection_contract -- --exact --nocapture
cargo test -p rock-lib mir::builder::tests::projection_outputs_use_normalized_nested_projection_keys -- --exact --nocapture
cargo test -p rock-lib mir::builder::tests::projection_outputs_emit_builtin_index_output -- --exact --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
```

Expected: all pass and projection output keys/values remain unchanged.

## Task 4: Require Full Agreement In Every Test

**Files:**

- Modify `lib/src/mir/agreement.rs:40-81,682-733,1470-1519`
- Modify `lib/src/mir/builder/mod.rs:4500-4587` and its helper call sites
- Test `lib/src/mir/agreement.rs:1240-1330,2300-2374`

**Step 1: Add a RED assertion for the partial mode**

In the agreement test that constructs a nominal aggregate with no corresponding
layout, call the partial entry point before deleting it and assert that the
layout error is visible:

```rust
let report = check_mir_runtime_agreement_for_mir_only(&program);
assert_eq!(report.invalid_layout_ids, 1);
```

Run:

```bash
cargo test -p rock-lib mir::agreement::tests::agreement_rejects_aggregate_with_missing_struct_layout_metadata -- --exact --nocapture
```

Expected before the fix: FAIL because the partial mode reports zero missing
layout IDs. Preserve the existing full-agreement assertion as the final GREEN
form after removing the partial call.

**Step 2: Collapse agreement to one full path**

Replace:

```rust
pub fn check_mir_runtime_agreement(program: &MirProgram) -> MirAgreementReport {
    check_mir_runtime_agreement_inner(program, true)
}
```

with the current inner body directly in `check_mir_runtime_agreement`. Delete:

```text
check_mir_runtime_agreement_for_mir_only
check_mir_runtime_agreement_inner
validate_backend_contract: bool
```

Always run:

```rust
validate_mir_backend_contract(program, &mut report);
validate_backend_contract_nominal_layout_ids(
    program,
    &backend_contract,
    &mut seen_nominal_layout_ids,
    &mut report,
);
```

Change `BackendContract::new` to take only `program`. Define
`nominal_layouts_required` from the contract sections without a validation
flag, and always run per-function nominal-layout checks.

**Step 3: Make builder fixtures contract-native**

Replace both calls to the MIR-only function with
`check_mir_runtime_agreement`. Before moving fields out of `MirBuilder`, retain
its accepted program and use the production canonical-layout helper:

```rust
let program = builder.program;
let ret_type = builder.type_id_for(&ret_type);
let mut type_context = builder.type_context.borrow().clone();
let function_id = MirFunctionId::Function(
    DefId::new(CrateId(0), LocalDefId(999)),
);
let functions = std::collections::BTreeMap::from([(
    function_id.clone(),
    MirFunction {
        id: function_id,
        name: "test".to_string(),
        basic_blocks: builder.blocks,
        local_decls: builder.locals,
        closure_captures: builder.closure_captures,
        arg_count: 0,
        ret_type,
        ownership: builder.ownership,
    },
)]);
let mut backend_contract = backend_contract;
MirBuilder::populate_backend_contract_nominal_layouts(
    &mut backend_contract,
    program,
    &mut type_context,
);
MirBuilder::populate_backend_contract_function_bodies(
    &mut backend_contract,
    &functions,
    &type_context,
);
let mir = MirProgram {
    functions,
    type_context,
    backend_contract,
};
crate::mir::agreement::check_mir_runtime_agreement(&mir)
```

Keep `assert_builder_agreement_clean` as the empty-extra-contract wrapper around
this helper; canonical nominal layouts now come from `builder.program`. Keep
`assert_builder_agreement_clean_with_contract` for tests with resolved non-local
callables. Instance-call tests must pass `instance_callable_contract(...)`, as
the existing explicit instance fixture does. Intrinsic-call tests remain covered
by `populate_backend_contract_function_bodies`, which invokes
`populate_backend_contract_intrinsic_callables`. The two existing
`assert_mir_agreement_clean` call sites must call full agreement on their
already-built `MirProgram`. Never infer callable identity from function/display
names.

**Step 4: Verify the agreement and builder suites**

Run sequentially:

```bash
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib mir::backend_contract -- --nocapture
```

Expected: all pass with no partial validation entry point. A failure caused by
a missing nominal declaration must be fixed in that fixture's `HirProgram`; a
failure caused by a resolved instance key must be fixed by passing
`instance_callable_contract`. Do not weaken agreement or restore a validation
flag.

**Step 5: Verify the original source guard is GREEN**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::mir_backend_contract_has_no_staging_or_partial_validation_path -- --exact --nocapture
```

Expected: PASS.

## Task 5: Prove Steps 2–5 Remain Closed

**Files:**

- Test `lib/src/semantic_identity_audit.rs`
- Test `lib/src/lib.rs`
- Test `lib/src/crate_artifact/load.rs`
- Test `lib/src/lower/paths.rs`
- Test `lib/tests/integration.rs`

**Step 1: Run Step 2 codegen/layout gates**

Run sequentially:

```bash
cargo test -p rock-lib codegen::types -- --nocapture
cargo test -p rock-lib codegen -- --nocapture
cargo test -p rock-lib codegen::mir_llvm -- --nocapture
```

Expected: all pass. Inspect the diff and confirm it adds no name-keyed codegen
map or substitution helper.

**Step 2: Run Step 3 product/link gates**

Run sequentially:

```bash
cargo test -p rock-lib product_link_records -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib crate_system -- --nocapture
cargo test -p rock-lib mono::external -- --nocapture
```

Expected: all pass, including explicit remap, misleading-name, missing/ambiguous
identity, and empty backend-symbol regressions.

**Step 3: Run Step 4–5 identity gates**

Run sequentially:

```bash
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib lower::paths -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib --test integration test_static_method_value_preserves_selected_authority -- --exact --nocapture
```

Expected: all pass. The semantic audit must retain the existing absence checks
for HIR backend names, downstream method rediscovery, and static authority
reconstruction.

**Step 4: Perform a direct source preservation review**

Review the final diff and verify:

```text
No struct_display_aliases or enum_display_aliases in MIR contract construction.
No ProductIdentityTable backend-symbol field or name fallback.
No HirFunction qualified_name/backend symbol field.
No static_method_target_for_callee or method display-name correlation.
No artifact schema or format-version change.
```

Any violation blocks completion even if tests pass.

## Task 6: Final Gates And Audit Closure

**Files:**

- Modify `CLEAN_SLATE_COMPILER_AUDIT.md:1217-1270`
- Modify `MEMORY.md` only if a durable Step 1 invariant is missing
- Update bead `new_lang2-cnb`

**Step 1: Run full verification**

Run sequentially:

```bash
cargo test -p rock-lib --test integration
cargo test -p rock-lib
cargo clippy -p rock-lib --all-targets
cargo fmt --all --check
git diff --check
```

Expected: every command exits zero. Existing advisory Clippy warnings are
acceptable only if no new warning is introduced by the changed code.

**Step 2: Refresh Step 1 evidence without changing Steps 2–5 claims**

Update `CLEAN_SLATE_COMPILER_AUDIT.md` Step 1 with:

- direct contract construction from accepted HIR/mono/MIR authority;
- deletion of all eight staging types;
- deletion of display-alias layout traversal;
- deletion of partial agreement validation;
- complete contract-native builder fixtures;
- exact focused and full test counts from this execution.

Do not alter Step 2–5 status except to add fresh preservation evidence if a
specific count changed.

**Step 3: Close the bead**

Run:

```bash
bd close new_lang2-cnb --reason "Removed MIR backend-contract staging rows and partial agreement validation; migrated builder fixtures to complete contracts; Steps 2-5 preservation and full gates passed" --json
```

Expected: `new_lang2-cnb` status is `closed`.

**Step 4: Report without VCS mutation**

Report changed files, direct-construction behavior, fixture migration, Step 2–5
preservation evidence, focused/full test counts, Clippy warnings, and bead
status. Do not stage, commit, amend, or push unless the user separately requests
it.
