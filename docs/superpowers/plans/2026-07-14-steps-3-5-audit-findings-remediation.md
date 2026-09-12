# Steps 3 And 5 Audit Findings Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the remaining name-based product link attachment and static-method authority reconstruction paths, and reject empty artifact backend symbols.

**Architecture:** Reuse the `id_remap` already built by `CompilerProducts::from_resolved_hir`, retaining it only as producer-side compiler state until MIR artifact exports are attached. Qualified static resolution will return exact impl/method/trait authority and canonical generic IDs so lowering can instantiate `HirStaticMethodTarget` without rescanning by names. Product artifact serialization remains unchanged.

**Tech Stack:** Rust 2021, `rock-lib`, HIR lowering and selection authority, product artifacts, MIR backend contracts, `serde`/`bincode`, Cargo tests.

---

## File Map

- Modify `docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md`: record the newly discovered static-method regression as F65 before fixing it.
- Modify `lib/src/products.rs`: name the existing product ID remap type and expose the same remap to the compiler construction path without serializing it.
- Modify `lib/src/lib.rs`: retain the producer remap, make link attachment fallible, and remove export/display-name matching.
- Modify `lib/src/crate_artifact/load.rs`: reject empty backend symbols while validating product link records.
- Modify `lib/src/lower/resolution.rs`: return exact qualified static-method authority instead of only a raw method `DefId`.
- Modify `lib/src/lower/paths.rs`: instantiate static-method values from the resolved ID-keyed authority.
- Modify `lib/src/lower/control_flow/secondary.rs`: delete `static_method_target_for_callee` and its name-based generic/trait reconstruction.
- Modify `lib/src/semantic_identity_audit.rs`: prevent the removed product and static-method fallback patterns from returning.
- Modify `CLEAN_SLATE_COMPILER_AUDIT.md`: refresh Step 3 and Step 5 completion evidence after verification.

Do not modify artifact format constants. Do not add a remap field to `CompilerProducts`, `ProductIdentityTable`, `ProductLinkData`, or serialized artifact rows. Do not commit, stage, or push unless the user explicitly requests VCS changes.

### Task 1: Record The Task 5 Regression

**Files:**
- Modify: `docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md`

- [ ] **Step 1: Add F65 to the summary table**

Add this row immediately after F64:

```markdown
| F65 | open | Qualified static-method path resolution returns only a raw method `DefId`; lowering then reconstructs impl/trait authority and generic bindings through method and generic display names. |
```

- [ ] **Step 2: Add the detailed F65 section**

Append this section after F64 and before final gate evidence:

```markdown
### F65. Qualified static-method authority is reconstructed from names

`resolve_static_method_path` selects a canonical owner/impl pair but returns a
`LowerResolvedValue` containing only `HirVarTarget::Function(method_id)`.
`static_method_target_for_callee` later rescans impls, resolves the selected
trait member by method name, and correlates impl-owned and method-owned generic
parameters by display name. This contradicts F36/F57 and permits names to
participate after canonical selection.

Relevant code:

- `lib/src/lower/resolution.rs`, `resolve_static_method_path`
- `lib/src/lower/paths.rs`, qualified static-method value lowering
- `lib/src/lower/control_flow/secondary.rs`, `static_method_target_for_callee`

Closure requirements:

- Qualified static resolution returns exact impl, method, optional trait/member,
  receiver-pattern, and generic-parameter authority.
- Lowering instantiates the exact authority without an impl rescan, trait-member
  name lookup, or generic display-name matching.
- `static_method_target_for_callee` is deleted.
- Focused lowering, semantic identity audit, and complete `rock-lib` suites pass.
```

- [ ] **Step 3: Verify the ledger records F65 as open**

Run: `rg -n "F65|static_method_target_for_callee" docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md`

Expected: one summary row and one detailed F65 section, both describing the open regression.

### Task 2: Reuse The Existing Product ID Remap For Link Attachment

**Files:**
- Modify: `lib/src/products.rs:587-927,2350-2407`
- Modify: `lib/src/lib.rs:244-266,311-316,695-779`
- Test: `lib/src/lib.rs:782-1203`

- [ ] **Step 1: Write failing explicit-remap attachment tests**

In `lib/src/lib.rs` tests, change `link_test_products` to return both products and the existing remap from a compiler-only constructor. Add these tests:

```rust
#[test]
fn product_link_records_use_explicit_remap_not_display_names() {
    let original_id = DefId::new(CrateId(0), LocalDefId(40));
    let non_callable_id = ProductDefId::from(original_id);
    let callable_id = ProductDefId {
        crate_id: ProductCrateId(0),
        local_id: crate::products::ProductLocalDefId(41),
    };
    let method = link_test_function(
        DefId::new(CrateId(0), LocalDefId(callable_id.local_id.0)),
        "actual_method",
    );
    let mut products = link_test_products(vec![]).0;
    products.identity_table.display_names.insert(
        non_callable_id,
        "misleading".to_string(),
    );
    products.interface.functions.insert(
        callable_id,
        crate::products::ProductFunctionInterface::from(&method),
    );
    let remap = BTreeMap::from([(
        ProductDefId::from(original_id),
        BTreeSet::from([non_callable_id, callable_id]),
    )]);
    let candidate = crate::mir::MirArtifactExport {
        origin_def_id: Some(original_id),
        source_name: "misleading".to_string(),
        backend_symbol: "explicit_symbol".to_string(),
        substitution_empty: true,
        has_body: true,
        provided_by_object: false,
        is_specialization: false,
        is_drop_glue: false,
    };

    let mut products = Some(products);
    attach_product_link_records(
        &mut products,
        Some(&remap),
        &[candidate],
        &HashMap::new(),
    )
    .expect("one remapped callable should attach");
    let products = products.expect("products remain available");
    assert_eq!(
        products
            .link
            .records
            .get(&callable_id)
            .map(|record| record.backend_symbol.as_str()),
        Some("explicit_symbol")
    );
    assert!(!products.link.records.contains_key(&non_callable_id));
}

#[test]
fn product_link_records_reject_missing_explicit_remap() {
    let function_id = DefId::new(CrateId(0), LocalDefId(50));
    let mut products = Some(link_test_products(vec![(
        "answer",
        link_test_function(function_id, "answer"),
    )]).0);
    let candidate = crate::mir::MirArtifactExport {
        origin_def_id: Some(function_id),
        source_name: "answer".to_string(),
        backend_symbol: "answer_symbol".to_string(),
        substitution_empty: true,
        has_body: true,
        provided_by_object: false,
        is_specialization: false,
        is_drop_glue: false,
    };

    let error = attach_product_link_records(
        &mut products,
        Some(&BTreeMap::new()),
        &[candidate],
        &HashMap::new(),
    )
    .expect_err("missing producer identity must fail");

    assert!(error.contains("no explicit product callable identity"));
}

#[test]
fn product_link_records_reject_ambiguous_callable_remap() {
    let original_id = DefId::new(CrateId(0), LocalDefId(60));
    let first_id = ProductDefId::from(original_id);
    let second_id = ProductDefId {
        crate_id: ProductCrateId(0),
        local_id: crate::products::ProductLocalDefId(61),
    };
    let mut products = link_test_products(vec![(
        "first",
        link_test_function(original_id, "first"),
    )]).0;
    let second = link_test_function(
        DefId::new(CrateId(0), LocalDefId(second_id.local_id.0)),
        "second",
    );
    products.interface.functions.insert(
        second_id,
        crate::products::ProductFunctionInterface::from(&second),
    );
    let remap = BTreeMap::from([(
        ProductDefId::from(original_id),
        BTreeSet::from([first_id, second_id]),
    )]);
    let candidate = crate::mir::MirArtifactExport {
        origin_def_id: Some(original_id),
        source_name: "first".to_string(),
        backend_symbol: "ambiguous_symbol".to_string(),
        substitution_empty: true,
        has_body: true,
        provided_by_object: false,
        is_specialization: false,
        is_drop_glue: false,
    };
    let mut products = Some(products);

    let error = attach_product_link_records(
        &mut products,
        Some(&remap),
        &[candidate],
        &HashMap::new(),
    )
    .expect_err("multiple callable identities must fail");

    assert!(error.contains("ambiguous product callable identities"));
}
```

Do not inspect export or display names to determine the expected ID.

- [ ] **Step 2: Run the new tests to verify RED**

Run:

```bash
cargo test -p rock-lib tests::product_link_records_use_explicit_remap_not_display_names -- --exact --nocapture
cargo test -p rock-lib tests::product_link_records_reject_missing_explicit_remap -- --exact --nocapture
cargo test -p rock-lib tests::product_link_records_reject_ambiguous_callable_remap -- --exact --nocapture
```

Expected: compilation fails because no compiler-only remap-returning constructor exists and `attach_product_link_records` is not fallible.

- [ ] **Step 3: Name and return the existing remap without serializing it**

In `lib/src/products.rs`, add:

```rust
pub(crate) type ProductIdRemap = BTreeMap<ProductDefId, BTreeSet<ProductDefId>>;
```

Change internal signatures that currently spell
`BTreeMap<ProductDefId, BTreeSet<ProductDefId>>` for `id_remap` to use
`ProductIdRemap`.

Rename the existing body-bearing constructor to the compiler-visible
`from_resolved_hir_with_remap`, leave its statements from crate-identity setup
through `remap_link_data` unchanged, and replace only its final `Self` return
with this exact tail:

```rust
pub(crate) fn from_resolved_hir_with_remap(
    crate_identity: ProductCrateIdentity,
    hir: &ResolvedHirProgram,
    dependencies: Vec<ProductDependencyIdentity>,
    dependency_crate_identities: BTreeMap<ProductCrateId, ProductCrateIdentity>,
    source_fingerprint: ProductSourceFingerprint,
    link: ProductLinkData,
) -> (Self, ProductIdRemap) {
    let products = Self {
        crate_identity,
        identity_table,
        interface,
        bodies,
        link,
        dependencies,
        source_fingerprint,
        infix_precedence: BTreeMap::new(),
        proc_macros: Vec::new(),
    };
    (products, id_remap)
}
```

Keep the public constructor as the artifact-facing convenience API:

```rust
pub fn from_resolved_hir(
    crate_identity: ProductCrateIdentity,
    hir: &ResolvedHirProgram,
    dependencies: Vec<ProductDependencyIdentity>,
    dependency_crate_identities: BTreeMap<ProductCrateId, ProductCrateIdentity>,
    source_fingerprint: ProductSourceFingerprint,
    link: ProductLinkData,
) -> Self {
    Self::from_resolved_hir_with_remap(
        crate_identity,
        hir,
        dependencies,
        dependency_crate_identities,
        source_fingerprint,
        link,
    )
    .0
}
```

This is the same map already produced by construction. Do not recompute it and do not add it to serialized data.

- [ ] **Step 4: Retain the remap in the compiler driver**

In `lib/src/lib.rs`, construct `(products, product_id_remap)` together:

```rust
let (mut products, product_id_remap) = if emit_products {
    let source_fingerprint =
        product_source_fingerprint(config, &source_db, &fingerprint_source_files);
    let (mut products, id_remap) = CompilerProducts::from_resolved_hir_with_remap(
        ProductCrateIdentity::local(current_crate_name.clone()),
        &hir,
        product_dependencies_from_config(config),
        product_dependency_crate_identities_from_context(crate_ctx),
        source_fingerprint,
        ProductLinkData::default(),
    );
    if config.current_crate_name.as_deref() == Some("stdlib") {
        products.record_prelude_export_ids(
            loaded_prelude_export_ids
                .iter()
                .map(|(alias, export)| (alias.clone(), export.clone())),
        );
    }
    products.infix_precedence = infix_precedence.into_iter().collect();
    (Some(products), Some(id_remap))
} else {
    (None, None)
};
```

Do not put `product_id_remap` into `CompilerProducts`.

- [ ] **Step 5: Make link attachment use only the remap**

Change the helper signature to:

```rust
fn attach_product_link_records(
    products: &mut Option<CompilerProducts>,
    product_id_remap: Option<&crate::products::ProductIdRemap>,
    candidates: &[crate::mir::MirArtifactExport],
    symbol_overrides: &std::collections::HashMap<crate::ids::DefId, String>,
) -> Result<(), String>
```

Replace `product_link_id_for_candidate` with:

```rust
fn product_link_id_for_candidate(
    products: &CompilerProducts,
    product_id_remap: &crate::products::ProductIdRemap,
    def_id: crate::ids::DefId,
) -> Result<ProductDefId, String> {
    let requested = ProductDefId::from(def_id);
    let callable_ids = product_id_remap
        .get(&requested)
        .into_iter()
        .flatten()
        .copied()
        .filter(|id| product_id_is_callable(products, *id))
        .collect::<Vec<_>>();

    match callable_ids.as_slice() {
        [id] => Ok(*id),
        [] => Err(format!(
            "MIR artifact export {:?} has no explicit product callable identity",
            def_id
        )),
        ids => Err(format!(
            "MIR artifact export {:?} has ambiguous product callable identities {:?}",
            def_id, ids
        )),
    }
}
```

After the existing candidate eligibility filters, require `origin_def_id` and the remap:

```rust
let def_id = candidate.origin_def_id.ok_or_else(|| {
    "MIR artifact export has no canonical origin DefId".to_string()
})?;
let remap = product_id_remap.ok_or_else(|| {
    "product link attachment has no producer identity remap".to_string()
})?;
let product_id = product_link_id_for_candidate(products, remap, def_id)?;
```

Delete all reads of `candidate.source_name`, `identity_table.export_names`, and
`identity_table.display_names` from link attachment. Return `Ok(())` after the
loop.

- [ ] **Step 6: Propagate attachment failure as a compiler diagnostic**

At the compile call site:

```rust
attach_product_link_records(
    &mut products,
    product_id_remap.as_ref(),
    &mir_program.backend_contract.artifact_exports,
    codegen.mir_contract_symbol_overrides(),
)
.map_err(|message| {
    Diagnostics::from(vec![diagnostic::Diagnostic::new(message, Span::default())])
})?;
```

Update all unit-test calls to pass the remap and unwrap or assert the returned error.

- [ ] **Step 7: Run focused product-link tests to verify GREEN**

Run: `cargo test -p rock-lib product_link_records -- --nocapture`

Expected: all link-record tests pass, including explicit remap and missing-remap failures.

### Task 3: Reject Empty Product Backend Symbols

**Files:**
- Modify: `lib/src/crate_artifact/load.rs:1545-1589`
- Test: `lib/src/crate_artifact/load.rs` product tests near the missing-link-record tests

- [ ] **Step 1: Write the failing artifact validation test**

Add:

```rust
#[test]
fn load_product_artifact_rejects_empty_link_record_backend_symbol() {
    let (base, _cleanup) = temp_test_dir("empty_link_record_backend_symbol");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();
    let artifact_path = base.join("dep.rkca");
    let mut products = product_with_function("dep", ProductCrateId(0), 0, object_path);
    let function_id = *products
        .interface
        .functions
        .keys()
        .next()
        .expect("fixture function");
    products.link.records.insert(
        function_id,
        ProductLinkRecord {
            backend_symbol: String::new(),
        },
    );
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    let error = ctx
        .load_product_artifact_from_path(artifact_path)
        .expect_err("empty backend symbols must fail during artifact loading");

    assert!(error.contains("empty backend symbol"));
    assert!(error.contains(&function_id.local_id.0.to_string()));
}
```

Use the existing `product_with_function` fixture helper shown above; do not introduce a second fixture constructor.

- [ ] **Step 2: Run the test to verify RED**

Run: `cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_empty_link_record_backend_symbol -- --exact --nocapture`

Expected: FAIL because empty strings currently pass `backend_symbols_from_products`.

- [ ] **Step 3: Validate every link-record symbol before remapping**

In `backend_symbols_from_products`, add:

```rust
for (id, record) in &products.link.records {
    validate_product_backend_symbol_id(products, *id, "link record")?;
    if record.backend_symbol.is_empty() {
        return Err(format!(
            "Product artifact link record for ID {}::{} has an empty backend symbol",
            id.crate_id.0, id.local_id.0
        ));
    }
    backend_symbols.insert(remap.def_id(*id)?, record.backend_symbol.clone());
}
```

Do not trim or otherwise rewrite symbols; reject only the empty string.

- [ ] **Step 4: Run focused artifact tests to verify GREEN**

Run:

```bash
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_empty_link_record_backend_symbol -- --exact --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
```

Expected: the new test and all artifact tests pass.

### Task 4: Resolve Qualified Static Methods With Exact Authority

**Files:**
- Modify: `lib/src/lower/resolution.rs:1-38,213-315`
- Modify: `lib/src/lower/paths.rs:1-180,747-792`
- Modify: `lib/src/lower/control_flow/secondary.rs:1-141`
- Test: `lib/src/lower/paths.rs:1353-1388,1769-2018`

- [ ] **Step 1: Write the source-level regression guard first**

Extend `lib/src/semantic_identity_audit.rs` with:

```rust
#[test]
fn qualified_static_method_authority_is_not_reconstructed_from_names() {
    assert_production_file_absent(
        "lower/control_flow/secondary.rs",
        &[
            "fn static_method_target_for_callee(",
            ".position(|name| name == owner_name)",
            ".methods.get(method_name)",
            ".signatures.get(method_name)",
        ],
    );
}
```

Keep the check narrowly scoped to the removed helper; ordinary source-level method-name resolution remains valid inside collection/resolution and selection.

- [ ] **Step 2: Add a behavioral static-authority regression**

Add a `lower::paths` test that creates a generic struct impl where authority is canonical by ID and display generic names deliberately differ:

```rust
#[test]
fn qualified_static_method_uses_canonical_generic_ids_not_display_names() {
    let mut lowerer = Lowerer::new();
    let owner_id = def_id(200);
    let impl_id = def_id(201);
    let method_id = def_id(202);
    let owner_param = GenericParamId {
        owner: impl_id,
        index: 0,
    };
    lowerer.items.structs.insert(
        "Box".to_string(),
        HirStruct {
            id: owner_id,
            name: "Box".to_string(),
            generic_params: vec!["StructDisplay".to_string()],
            fields: Vec::new(),
        },
    );
    register_item_path(&mut lowerer, "Box", owner_id);
    let mut method = test_function(method_id, "make");
    method.generic_params = vec!["MethodDisplayDoesNotMatch".to_string()];
    method.generic_param_ids = vec![owner_param];
    let local_id = lowerer.fresh_local_id();
    method.params = vec![HirParam {
        name: "value".to_string(),
        local_id,
        ty: Type::Generic(owner_param),
        mutable: false,
        is_ref: false,
    }];
    method.ret_type = Type::Struct {
        id: owner_id,
        args: vec![Type::Generic(owner_param)],
    };
    lowerer.items.impls.push(HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Box".to_string()),
        type_name: "Box".to_string(),
        type_generics: vec!["ImplDisplay".to_string()],
        receiver_pattern: HirImplReceiverPattern::Exact(Type::Struct {
            id: owner_id,
            args: vec![Type::Generic(owner_param)],
        }),
        trait_name: None,
        trait_id: None,
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: HashMap::new(),
        methods: HashMap::from([("make".to_string(), method)]),
    });

    let expr = lowerer.lower_identifier_path(&identifier_path(&["Box", "make"]));
    let (_, target) = static_method_value_parts(&expr);

    assert_eq!(target.method.impl_id(), Some(impl_id));
    assert_eq!(target.method.method_id(), Some(method_id));
    assert_eq!(target.method.owner_substitution[0].param, owner_param);
}
```

Add this second regression with a trait-backed impl whose `trait_id` has no
matching trait definition:

```rust
#[test]
fn qualified_static_method_with_incomplete_trait_authority_is_an_error() {
    let mut lowerer = Lowerer::new();
    let owner_id = def_id(210);
    let impl_id = def_id(211);
    let method_id = def_id(212);
    let missing_trait_id = def_id(213);
    lowerer.items.structs.insert(
        "Box".to_string(),
        HirStruct {
            id: owner_id,
            name: "Box".to_string(),
            generic_params: Vec::new(),
            fields: Vec::new(),
        },
    );
    register_item_path(&mut lowerer, "Box", owner_id);
    lowerer.items.impls.push(HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Box".to_string()),
        type_name: "Box".to_string(),
        type_generics: Vec::new(),
        receiver_pattern: HirImplReceiverPattern::Exact(Type::Struct {
            id: owner_id,
            args: Vec::new(),
        }),
        trait_name: Some("MissingTrait".to_string()),
        trait_id: Some(missing_trait_id),
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: HashMap::new(),
        methods: HashMap::from([(
            "make".to_string(),
            test_function(method_id, "make"),
        )]),
    });

    let expr = lowerer.lower_identifier_path(&identifier_path(&["Box", "make"]));

    assert_eq!(expr.ty, Type::Error);
    assert!(!matches!(
        expr.kind,
        HirExprKind::ResolvedVar(HirVarRef {
            target: HirVarTarget::Function(id),
            ..
        }) if id == method_id
    ));
    assert!(lowerer
        .diagnostics
        .errors()
        .iter()
        .any(|error| error.message.contains("references unknown trait")));
}
```

- [ ] **Step 3: Run both tests to verify RED**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::qualified_static_method_authority_is_not_reconstructed_from_names -- --exact --nocapture
cargo test -p rock-lib lower::paths::tests::qualified_static_method_uses_canonical_generic_ids_not_display_names -- --exact --nocapture
```

Expected: the audit test fails because `static_method_target_for_callee` exists.
The behavioral test passes before the refactor and establishes that changing
display generic names cannot change the selected IDs; it must remain green
after the helper is removed.

- [ ] **Step 4: Introduce a typed resolved static-method record**

In `lib/src/lower/resolution.rs`, add:

```rust
#[derive(Debug, Clone)]
pub(crate) struct LowerResolvedStaticMethod {
    pub(crate) value: LowerResolvedValue,
    pub(crate) receiver_pattern: HirImplReceiverPattern,
    pub(crate) owner_generic_params: Vec<crate::types::GenericParamId>,
    pub(crate) method_generic_params: Vec<crate::types::GenericParamId>,
    pub(crate) method_target: crate::hir::HirMethodCallTarget,
}
```

Change `resolve_static_method_path` to return
`Result<Option<LowerResolvedStaticMethod>, String>`. Return `Ok(None)` when the
path has no canonical nominal owner or no static candidate. Return a descriptive
`Err` when multiple impls match or when a selected trait-backed impl lacks its
exact trait/member authority. Construct authority while `(imp, method)` are
still available:

```rust
let (imp, method) = match matches.as_slice() {
    [] => return Ok(None),
    [candidate] => *candidate,
    candidates => {
        return Err(format!(
            "qualified static method '{}::{}' has ambiguous impl candidates {:?}",
            structure.name,
            method_name,
            candidates.iter().map(|(imp, _)| imp.id).collect::<Vec<_>>(),
        ));
    }
};
let selected_trait = match imp.trait_id {
    Some(trait_id) => {
        let trait_def = self
            .lowerer
            .items
            .traits
            .values()
            .find(|trait_def| trait_def.id == trait_id)
            .ok_or_else(|| {
                format!("selected static impl {:?} references unknown trait {:?}", imp.id, trait_id)
            })?;
        let member_id = trait_def
            .methods
            .get(method_name)
            .map(|member| member.id)
            .or_else(|| trait_def.signatures.get(method_name).map(|member| member.id))
            .ok_or_else(|| {
                format!(
                    "selected static impl {:?} has no trait member authority for '{}'",
                    imp.id, method_name
                )
            })?;
        Some(crate::hir::HirSelectedTraitMember {
            trait_id,
            member_id,
            trait_args: imp.trait_arg_types.clone(),
        })
    }
    None => None,
};
let owner_generic_params = (0..imp.type_generics.len())
    .map(|index| crate::types::GenericParamId {
        owner: imp.id,
        index: index as u32,
    })
    .collect();
let method_generic_params = method
    .generic_param_ids
    .iter()
    .copied()
    .filter(|param| param.owner == method.id)
    .collect();
let owner_name = self.preferred_nominal_name(structure.id);

Ok(Some(LowerResolvedStaticMethod {
    value: self.resolved_method_value(&owner_name, method_name, method),
    receiver_pattern: imp.receiver_pattern.clone(),
    owner_generic_params,
    method_generic_params,
    method_target: crate::hir::HirMethodCallTarget::impl_method(
        imp.id,
        method.id,
        selected_trait,
    ),
}))
```

Method-name lookup in this function is source resolution while the exact
canonical trait and impl are in hand. No later phase may repeat it.

- [ ] **Step 5: Instantiate the typed authority using only generic IDs**

Move the type-substitution portion of `static_method_target_for_callee` into a
new `Lowerer` helper in `lower/paths.rs`. Add `use crate::lexer::Span;` to that
module's imports:

```rust
fn instantiate_resolved_static_method(
    &mut self,
    resolved: crate::lower::resolution::LowerResolvedStaticMethod,
    span: Span,
) -> Option<(HirExpr, HirStaticMethodTarget)> {
    let callee_ty = self.instantiate_resolved_value_type(&resolved.value);
    let mut substitution = HashMap::new();
    crate::selection::infer_generic_subst_from_types(
        &resolved.value.ty,
        &callee_ty,
        &mut substitution,
    );

    let mut method = resolved.method_target;
    method.owner_substitution = resolved
        .owner_generic_params
        .iter()
        .copied()
        .map(|param| {
            substitution
                .get(&param)
                .cloned()
                .map(|ty| HirTypeBinding { param, ty })
        })
        .collect::<Option<Vec<_>>>()?;
    method.method_substitution = resolved
        .method_generic_params
        .iter()
        .copied()
        .map(|param| {
            substitution
                .get(&param)
                .cloned()
                .map(|ty| HirTypeBinding { param, ty })
        })
        .collect::<Option<Vec<_>>>()?;

    for ty in method.trait_args_mut().into_iter().flatten() {
        *ty = ty.substitute_generics(&substitution);
    }
    let owner_substitution = method
        .owner_substitution
        .iter()
        .map(|binding| (binding.param, binding.ty.clone()))
        .collect::<HashMap<_, _>>();
    let owner_ty = match resolved.receiver_pattern {
        HirImplReceiverPattern::Exact(ty) => ty.substitute_generics(&owner_substitution),
        HirImplReceiverPattern::SliceFamily { element } => {
            Type::Slice(Box::new(element.substitute_generics(&owner_substitution)))
        }
    };
    let callee = HirExpr {
        ty: callee_ty,
        kind: HirExprKind::ResolvedVar(HirVarRef {
            name: resolved.value.name,
            target: resolved.value.target?,
        }),
        span,
    };

    Some((callee, HirStaticMethodTarget { owner_ty, method }))
}
```

Use the existing `HirMethodCallTarget::trait_args_mut` method shown above. Do
not add a string-keyed helper.

- [ ] **Step 6: Use the authority record in path lowering**

Replace the current `resolve_static_method_path` block in `lower_identifier_path` with:

```rust
match crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_static_method_path(&names)
{
    Ok(Some(resolved)) => {
        if let Some((callee, target)) =
            self.instantiate_resolved_static_method(resolved, span.clone())
        {
            return self.static_method_value_lambda(callee, target);
        }
        self.diagnostics.push_with_span(
            "selected static method has incomplete canonical authority".to_string(),
            span,
        );
        return self.error_expression();
    }
    Ok(None) => {}
    Err(message) => {
        self.diagnostics.push_with_span(message, span);
        return self.error_expression();
    }
}
```

Delete the raw-function fallback for a recognized static method path.

- [ ] **Step 7: Delete the old reconstruction helper**

Delete `Lowerer::static_method_target_for_callee` from
`lib/src/lower/control_flow/secondary.rs`. Let `cargo fmt` and the compiler
identify imports that became unused; retain imports still used by the remaining
secondary-expression lowering code.

- [ ] **Step 8: Run focused static-method tests to verify GREEN**

Run:

```bash
cargo test -p rock-lib lower::paths -- --nocapture
cargo test -p rock-lib semantic_identity_audit::qualified_static_method_authority_is_not_reconstructed_from_names -- --exact --nocapture
cargo test -p rock-lib --test integration test_static_method_value_preserves_selected_authority -- --exact --nocapture
```

Expected: all focused tests pass.

### Task 5: Refresh Audit Evidence And Run Final Gates

**Files:**
- Modify: `docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md`
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md`

- [ ] **Step 1: Close F65 with concrete evidence**

Change the F65 summary row from `open` to `closed`. Replace its closure requirements with closure evidence naming the final symbols and tests actually used. Do not alter F1-F64.

- [ ] **Step 2: Refresh Step 3 and Step 5 completion text**

In `CLEAN_SLATE_COMPILER_AUDIT.md`:

- add that link attachment reuses the producer's existing ID remap and fails on missing/ambiguous callable identity;
- add that artifact loading rejects empty backend symbols;
- add that qualified static values carry exact authority directly from resolution and no longer invoke `static_method_target_for_callee`;
- record the actual verification commands and counts from this execution.

Do not change the status of Steps 1, 2, or 4.

- [ ] **Step 3: Run focused regression suites**

Run sequentially:

```bash
cargo test -p rock-lib product_link_records -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib lower::paths -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected: all tests pass with zero failures.

- [ ] **Step 4: Run the complete library suite once**

Run: `cargo test -p rock-lib > /tmp/rock-lib-steps-3-5-remediation.log 2>&1`

Expected: exit status 0. Inspect the saved log once for the final passed/failed counts; do not rerun the full suite merely to recover output.

- [ ] **Step 5: Run lint, formatting, and whitespace gates**

Run sequentially:

```bash
cargo clippy -p rock-lib --all-targets
cargo fmt --all --check
git diff --check
```

Expected: all commands exit 0 with no errors.

- [ ] **Step 6: Confirm no forbidden residue remains**

Run `rg -n "static_method_target_for_callee" lib/src/lower` and confirm there are
no matches. Then inspect `product_link_id_for_candidate` and
`attach_product_link_records` in `lib/src/lib.rs` and confirm neither function
contains `candidate.source_name`, `identity_table.export_names`, or
`identity_table.display_names`. The semantic audit may intentionally contain the
deleted helper name as a forbidden-pattern assertion.

- [ ] **Step 7: Review the final diff without changing VCS state**

Run:

```bash
git status --short
git diff --stat
git diff -- docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md CLEAN_SLATE_COMPILER_AUDIT.md lib/src/products.rs lib/src/lib.rs lib/src/crate_artifact/load.rs lib/src/lower/resolution.rs lib/src/lower/paths.rs lib/src/lower/control_flow/secondary.rs lib/src/semantic_identity_audit.rs
```

Expected: only the intended source, tests, design/plan, ledger, and audit files are changed. Leave unrelated user files untouched.
