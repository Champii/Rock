# Product Link Records Backend Symbol Source Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete `CLEAN_SLATE_COMPILER_AUDIT.md` Step 3 by making product link records the only backend symbol source and marking the whole step complete after verification.

**Architecture:** Delete serialized identity-table backend symbols and keep backend symbols only in `ProductLinkData.records` on disk and `ExternCrateLink.backend_symbols` after artifact load. Object-backed concrete callables must have explicit link records; no production path may synthesize backend symbols from `qualified_name`, display names, interface names, impl type names, trait names, or method names.

**Tech Stack:** Rust 2021, `rock-lib`, product artifacts, `serde`/`bincode`, MIR artifact exports, `ProductDefId`, `DefId`, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Scope Guard

This plan implements the entire clean-slate audit Step 3. Do not stop after removing one storage map. The implementation is not complete until `CLEAN_SLATE_COMPILER_AUDIT.md` Step 3 is marked complete with validation evidence.

Do not touch `.sisyphus/`. Do not stage or commit unless the current user prompt explicitly asks for it. The worktree currently has an unrelated `MEMORY.md` change; leave it untouched unless the user explicitly asks otherwise.

## File Map

- Modify `lib/src/products.rs`: remove `ProductIdentityTable.backend_symbols`, remove `backend_symbols_from_link`, stop mirroring link records into identity metadata, update product tests, and bump product artifact format for the identity-table schema removal.
- Modify `lib/src/lib.rs`: stop writing product link records into identity metadata, delete exported-backend-symbol source-name fallback, update attach-product-link tests.
- Modify `lib/src/crate_artifact/load.rs`: load backend symbols only from `ProductLinkData.records`, add object-backed missing-link-record validation, update artifact load tests.
- Modify `lib/src/crate_system/extern_store.rs`: make object-provided predicates require link record presence, remove imported symbol fallback to `qualified_name`/formatted names.
- Modify `lib/src/crate_system/tests.rs`: add focused tests for link-record-only imported symbols and provider predicates.
- Modify `lib/src/mono/external.rs`: remove downstream fallback after `imported_impl_method_symbol`.
- Modify `lib/src/semantic_identity_audit.rs`: add or extend audit forbiddance for removed identity-table backend-symbol and fallback patterns.
- Modify `lib/src/products/type_table.rs` only if removing `ProductIdentityTable.backend_symbols` requires fixture updates in type-table tests.
- Modify `rock-shared/src/sysroot.rs`: bump shared `PRODUCT_ARTIFACT_FORMAT_VERSION`.
- Modify `CLEAN_SLATE_COMPILER_AUDIT.md`: mark Step 3 complete only after all verification passes.

## Task 1: Establish Red Tests For Product Link-Only Storage

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Rename and rewrite the product storage test**

In `lib/src/products.rs`, replace `compiler_products_record_backend_symbols_in_identity_table` with this test:

```rust
#[test]
fn compiler_products_preserve_backend_symbols_in_link_records() {
    let hir = resolved_hir_for_products();
    let plain_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(0)));
    let mut records = BTreeMap::new();
    records.insert(
        plain_id,
        ProductLinkRecord {
            backend_symbol: "rock_main".to_string(),
        },
    );

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData {
            object_path: None,
            records,
        },
    );

    assert_eq!(
        products
            .link
            .records
            .get(&plain_id)
            .map(|record| &record.backend_symbol),
        Some(&"rock_main".to_string())
    );
}
```

Do not assert on `products.identity_table.backend_symbols`; that field will be deleted later in the plan.

- [ ] **Step 2: Update real-local-id-zero link record preservation test**

In `lib/src/products.rs`, rename `compiler_products_preserve_backend_symbols_for_real_local_id_zero` to `compiler_products_preserve_link_records_for_real_local_id_zero`.

Replace the final identity-table assertion:

```rust
assert_eq!(
    products.identity_table.backend_symbols.get(&impl_id),
    Some(&"impl_box_show".to_string())
);
```

with no identity-table assertion. Keep this existing link-record assertion:

```rust
assert_eq!(
    products
        .link
        .records
        .get(&impl_id)
        .map(|record| &record.backend_symbol),
    Some(&"impl_box_show".to_string())
);
```

- [ ] **Step 3: Update ambiguous link ID test**

In `lib/src/products.rs`, rename `compiler_products_drop_backend_symbols_for_ambiguous_link_ids` to `compiler_products_drop_link_records_for_ambiguous_link_ids`.

Delete this assertion because the identity-table field will be removed:

```rust
assert!(!products
    .identity_table
    .backend_symbols
    .values()
    .any(|backend_symbol| backend_symbol == &symbol));
```

Keep this link-record assertion:

```rust
assert!(!products
    .link
    .records
    .values()
    .any(|record| record.backend_symbol == symbol));
```

- [ ] **Step 4: Update attach-product tests that check identity mirroring**

In `lib/src/lib.rs`, remove identity-table backend-symbol assertions from these tests:

- `product_link_records_ignore_body_only_function_rows`
- `product_link_records_ignore_body_only_trait_default_rows`
- `product_link_records_filter_non_drop_glue_specialization_artifact_exports`

For each test, keep the existing `products.link.records` assertion and delete the `products.identity_table.backend_symbols` assertion block.

In `product_link_records_export_drop_glue_specialization_artifact_exports`, delete this assertion:

```rust
assert_eq!(
    products.identity_table.backend_symbols.get(&product_id),
    Some(&"Box_drop".to_string())
);
```

Keep the `products.link.records` assertion.

- [ ] **Step 5: Add a red test proving source/display names do not attach link records by backend-symbol fallback**

Add this test near the other `attach_product_link_records` tests in `lib/src/lib.rs`:

```rust
#[test]
fn product_link_records_do_not_match_backend_symbols_from_display_names() {
    let function_id = DefId::new(CrateId(0), LocalDefId(20));
    let product_id = ProductDefId::from(function_id);
    let function = link_test_function(function_id, "answer");
    let mut products = Some(link_test_products(vec![("answer", function)]));
    let export = crate::mir::MirArtifactExport {
        origin_def_id: Some(DefId::new(CrateId(0), LocalDefId(99))),
        source_name: "not_answer".to_string(),
        backend_symbol: "answer".to_string(),
        substitution_empty: true,
        has_body: true,
        provided_by_object: false,
        is_specialization: false,
        is_drop_glue: false,
    };

    attach_product_link_records(&mut products, &[export], &HashMap::new(), Some("app"));

    let products = products.expect("products should remain present");
    assert!(!products.link.records.contains_key(&product_id));
}
```

This test should fail before deleting `product_id_for_exported_backend_symbol`, because the old code can reconstruct a match from `qualified_name`/display metadata.

- [ ] **Step 6: Run red product/link tests**

Run:

```bash
cargo test -p rock-lib products::tests::compiler_products_preserve_backend_symbols_in_link_records -- --exact --nocapture
cargo test -p rock-lib products::tests::compiler_products_preserve_link_records_for_real_local_id_zero -- --exact --nocapture
cargo test -p rock-lib products::tests::compiler_products_drop_link_records_for_ambiguous_link_ids -- --exact --nocapture
cargo test -p rock-lib tests::product_link_records_do_not_match_backend_symbols_from_display_names -- --exact --nocapture
```

Expected before implementation: the rewritten tests that no longer reference deleted fields may pass, but `product_link_records_do_not_match_backend_symbols_from_display_names` should FAIL because the fallback still attaches a record.

## Task 2: Establish Red Tests For Artifact Link Record Validation

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Replace identity-backend-symbol fallback test with missing function link-record test**

In `lib/src/crate_artifact/load.rs`, replace `load_product_artifact_rejects_identity_backend_symbol_for_undeclared_callable_id` with this test:

```rust
#[test]
fn load_product_artifact_rejects_object_function_missing_link_record() {
    let (base, _cleanup) = temp_test_dir("object_function_missing_link_record");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();
    let mut products = product_with_function("dep", ProductCrateId(0), 0, object_path);
    products.link.records.clear();
    let artifact_path = base.join("dep.rkca");
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    let err = ctx
        .load_product_artifact_from_path(artifact_path)
        .expect_err("object-backed concrete functions require link records");

    assert!(
        err.contains("missing link record")
            && err.contains("backend symbol")
            && err.contains("function"),
        "unexpected error: {err}"
    );
}
```

- [ ] **Step 2: Add a missing impl-method link-record test**

Add this test near the artifact link-record validation tests in `lib/src/crate_artifact/load.rs`:

```rust
#[test]
fn load_product_artifact_rejects_object_impl_method_missing_link_record() {
    let (base, _cleanup) = temp_test_dir("object_impl_method_missing_link_record");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();
    let artifact_path = base.join("dep.rkca");

    let impl_id = DefId::new(CrateId(0), LocalDefId(7));
    let method_id = DefId::new(CrateId(0), LocalDefId(8));
    let product_impl_id = ProductDefId::from(impl_id);
    let product_method_id = ProductDefId::from(method_id);
    let mut method = test_function(method_id, "make");
    method.is_method = true;
    let imp = HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Box".to_string()),
        type_name: "Box".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: None,
        trait_id: None,
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: HashMap::new(),
        methods: HashMap::from([("make".to_string(), method)]),
    };

    let mut identity_table = ProductIdentityTable::default();
    identity_table.local_crate = Some(ProductCrateId(0));
    identity_table
        .display_names
        .insert(product_impl_id, "Box".to_string());
    identity_table
        .display_names
        .insert(product_method_id, "Box::make".to_string());
    let mut interface = ProductInterface::default();
    interface
        .impls
        .insert(product_impl_id, ProductImplInterface::from(&imp));
    let products = CompilerProducts {
        crate_identity: ProductCrateIdentity::local("dep".to_string()),
        identity_table,
        interface,
        bodies: ProductBodies::default(),
        link: ProductLinkData {
            object_path: Some(object_path),
            records: BTreeMap::new(),
        },
        dependencies: Vec::new(),
        source_fingerprint: ProductSourceFingerprint::default(),
        infix_precedence: Default::default(),
        proc_macros: Vec::new(),
    };
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    let err = ctx
        .load_product_artifact_from_path(artifact_path)
        .expect_err("object-backed concrete impl methods require link records");

    assert!(
        err.contains("missing link record")
            && err.contains("backend symbol")
            && err.contains("impl")
            && err.contains("method 'make'"),
        "unexpected error: {err}"
    );
}
```

- [ ] **Step 3: Add a generic function exemption test**

Add this test near the new missing-link tests in `lib/src/crate_artifact/load.rs`:

```rust
#[test]
fn load_product_artifact_allows_generic_function_without_link_record() {
    let (base, _cleanup) = temp_test_dir("generic_function_without_link_record");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();
    let artifact_path = base.join("dep.rkca");
    let product_id = product_def_id(0, 0);
    let function_id = product_hir_def_id(product_id);
    let mut function = test_function(function_id, "dep::id");
    function.generic_params.push("T".to_string());
    function.generic_param_ids.push(crate::types::GenericParamId {
        owner: function_id,
        index: 0,
    });

    let mut identity_table = ProductIdentityTable::default();
    identity_table.local_crate = Some(ProductCrateId(0));
    identity_table
        .display_names
        .insert(product_id, "dep::id".to_string());
    let mut interface = ProductInterface::default();
    interface
        .functions
        .insert(product_id, ProductFunctionInterface::from(&function));
    let products = CompilerProducts {
        crate_identity: ProductCrateIdentity::local("dep".to_string()),
        identity_table,
        interface,
        bodies: ProductBodies::default(),
        link: ProductLinkData {
            object_path: Some(object_path),
            records: BTreeMap::new(),
        },
        dependencies: Vec::new(),
        source_fingerprint: ProductSourceFingerprint::default(),
        infix_precedence: Default::default(),
        proc_macros: Vec::new(),
    };
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path(artifact_path).unwrap();
}
```

- [ ] **Step 4: Update object-backed crate fixture test to include an explicit link record**

In `load_product_artifact_registers_object_backed_crate`, replace the empty records map:

```rust
records: Default::default(),
```

with:

```rust
records: BTreeMap::from([(
    product_id,
    ProductLinkRecord {
        backend_symbol: "dep_answer".to_string(),
    },
)]),
```

Add this assertion after the object path assertion:

```rust
assert_eq!(dep.link().backend_symbol(function_id), Some("dep_answer"));
```

- [ ] **Step 5: Update method-like root export fixture to avoid missing-link failure**

In `load_product_artifact_qualifies_method_like_root_export_sources`, the trait default method is body-backed metadata, not an object-provided impl method. To keep this test about root export qualification, set `link.object_path` to `None`:

```rust
link: ProductLinkData {
    object_path: None,
    records: Default::default(),
},
```

- [ ] **Step 6: Run red artifact tests**

Run:

```bash
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_object_function_missing_link_record -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_object_impl_method_missing_link_record -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_allows_generic_function_without_link_record -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_registers_object_backed_crate -- --exact --nocapture
```

Expected before implementation: the two missing-link tests should FAIL because loading currently accepts object-backed artifacts without those records. The generic exemption should pass or continue to pass after implementation.

## Task 3: Establish Red Tests For External Store Link-Only Symbols

**Files:**
- Modify: `lib/src/crate_system/tests.rs`

- [ ] **Step 1: Add imported function symbol fallback test**

Add this test near `extern_crate_link_makes_object_path_required_for_object_backed_dependencies` in `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn extern_crate_ref_imported_function_symbol_requires_link_record() {
    let function_id = DefId::new(CrateId(9), LocalDefId(1));
    let function = hir_function_with_id(function_id, "answer", Some("dep::answer"), &[]);
    let mut interface = ArtifactCrateInterface::default();
    interface.insert_function("dep::answer".to_string(), function.clone());
    let metadata = ExternCrateMetadata::new(interface, ResolverTables::default(), BTreeMap::new());
    let record = ExternCrateRecord::new(
        CrateId(9),
        "dep".to_string(),
        metadata,
        ExternCrateBodies::default(),
        ExternCrateLink::object(PathBuf::from("/dep/dep.o"), BTreeMap::new()),
    );
    let mut store = ExternCrateStore::default();
    store.insert(record).unwrap();
    let dep = store.by_name("dep").unwrap();

    assert_eq!(dep.imported_function_symbol("dep::answer", &function), None);
}
```

This should fail before implementation because the current code falls back to `qualified_name`.

- [ ] **Step 2: Add imported impl method fallback test**

Add this test near the previous new test in `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn extern_crate_ref_imported_impl_method_symbol_requires_link_record() {
    let impl_id = DefId::new(CrateId(9), LocalDefId(2));
    let method_id = DefId::new(CrateId(9), LocalDefId(3));
    let mut method = hir_function_with_id(method_id, "make", Some("dep::Box::make"), &[]);
    method.is_method = true;
    let imp = HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Box".to_string()),
        type_name: "Box".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: None,
        trait_id: None,
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: HashMap::new(),
        methods: HashMap::from([("make".to_string(), method.clone())]),
    };
    let mut interface = ArtifactCrateInterface::default();
    interface.insert_impl(imp.clone());
    let metadata = ExternCrateMetadata::new(interface, ResolverTables::default(), BTreeMap::new());
    let record = ExternCrateRecord::new(
        CrateId(9),
        "dep".to_string(),
        metadata,
        ExternCrateBodies::default(),
        ExternCrateLink::object(PathBuf::from("/dep/dep.o"), BTreeMap::new()),
    );
    let mut store = ExternCrateStore::default();
    store.insert(record).unwrap();
    let dep = store.by_name("dep").unwrap();

    assert!(!dep.provides_impl_method_body(&imp, &method));
    assert_eq!(dep.imported_impl_method_symbol(&imp, "make", &method), None);
}
```

This should fail before implementation because provider predicates currently do not require link-record presence.

- [ ] **Step 3: Add positive link-record test**

Add this test near the two red tests in `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn extern_crate_ref_imported_symbols_use_explicit_link_records() {
    let function_id = DefId::new(CrateId(9), LocalDefId(1));
    let function = hir_function_with_id(function_id, "answer", Some("dep::answer"), &[]);
    let mut interface = ArtifactCrateInterface::default();
    interface.insert_function("dep::answer".to_string(), function.clone());
    let metadata = ExternCrateMetadata::new(interface, ResolverTables::default(), BTreeMap::new());
    let record = ExternCrateRecord::new(
        CrateId(9),
        "dep".to_string(),
        metadata,
        ExternCrateBodies::default(),
        ExternCrateLink::object(
            PathBuf::from("/dep/dep.o"),
            BTreeMap::from([(function_id, "dep_answer".to_string())]),
        ),
    );
    let mut store = ExternCrateStore::default();
    store.insert(record).unwrap();
    let dep = store.by_name("dep").unwrap();

    assert_eq!(
        dep.imported_function_symbol("dep::answer", &function),
        Some("dep_answer".to_string())
    );
}
```

- [ ] **Step 4: Run red external-store tests**

Run:

```bash
cargo test -p rock-lib crate_system::tests::extern_crate_ref_imported_function_symbol_requires_link_record -- --exact --nocapture
cargo test -p rock-lib crate_system::tests::extern_crate_ref_imported_impl_method_symbol_requires_link_record -- --exact --nocapture
cargo test -p rock-lib crate_system::tests::extern_crate_ref_imported_symbols_use_explicit_link_records -- --exact --nocapture
```

Expected before implementation: the two missing-link tests should FAIL; the explicit-link test should pass.

## Task 4: Remove Product Identity Backend Symbol Storage

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `rock-shared/src/sysroot.rs`

- [ ] **Step 1: Delete the identity-table field**

In `lib/src/products.rs`, remove this field from `ProductIdentityTable`:

```rust
pub backend_symbols: BTreeMap<ProductDefId, String>,
```

Keep all other identity fields unchanged.

- [ ] **Step 2: Stop populating identity backend symbols from remapped link data**

In `CompilerProducts::from_resolved_hir`, replace:

```rust
let link = remap_link_data(link, &id_remap);
identity_table.backend_symbols = backend_symbols_from_link(&link, &id_remap);
```

with:

```rust
let link = remap_link_data(link, &id_remap);
```

- [ ] **Step 3: Delete the mirror helper**

In `lib/src/products.rs`, delete the entire `backend_symbols_from_link` function:

```rust
fn backend_symbols_from_link(
    link: &ProductLinkData,
    id_remap: &BTreeMap<ProductDefId, BTreeSet<ProductDefId>>,
) -> BTreeMap<ProductDefId, String> {
    link.records
        .iter()
        .filter_map(|(id, record)| {
            // Ambiguous remaps cannot safely claim a single product identity.
            if id_remap.get(id).is_some_and(|ids| ids.len() > 1) {
                None
            } else {
                Some((*id, record.backend_symbol.clone()))
            }
        })
        .collect()
}
```

Do not delete `remap_link_data`; it remains the link-record remapping authority.

- [ ] **Step 4: Stop attach path from mirroring backend symbols into identity metadata**

In `lib/src/lib.rs`, replace this block in `attach_product_link_records`:

```rust
let backend_symbol = symbol_overrides
    .get(&def_id)
    .cloned()
    .unwrap_or_else(|| candidate.backend_symbol.clone());
products.link.records.insert(
    product_id,
    ProductLinkRecord {
        backend_symbol: backend_symbol.clone(),
    },
);
products
    .identity_table
    .backend_symbols
    .insert(product_id, backend_symbol);
```

with:

```rust
let backend_symbol = symbol_overrides
    .get(&def_id)
    .cloned()
    .unwrap_or_else(|| candidate.backend_symbol.clone());
products.link.records.insert(
    product_id,
    ProductLinkRecord {
        backend_symbol,
    },
);
```

- [ ] **Step 5: Remove exported backend-symbol fallback matching**

In `product_link_id_for_candidate` in `lib/src/lib.rs`, delete these lines:

```rust
let exported_symbol = exported_product_symbol(&candidate.backend_symbol, current_crate_name);
product_id_for_exported_backend_symbol(products, &exported_symbol, current_crate_name)
```

Replace them with:

```rust
None
```

Then delete the entire `product_id_for_exported_backend_symbol` function and the `exported_product_symbol` helper below it.

- [ ] **Step 6: Remove artifact loader identity-table backend-symbol fallback**

Deleting `ProductIdentityTable.backend_symbols` must remove all source references in the same compile step. In `backend_symbols_from_products` in `lib/src/crate_artifact/load.rs`, delete this loop:

```rust
for (id, symbol) in &products.identity_table.backend_symbols {
    validate_product_backend_symbol_id(products, *id, "identity backend symbol")?;
    backend_symbols
        .entry(remap.def_id(*id)?)
        .or_insert_with(|| symbol.clone());
}
```

The function should keep loading `products.link.records` only:

```rust
fn backend_symbols_from_products(
    products: &CompilerProducts,
    remap: &ProductIdentityRemap,
) -> Result<BTreeMap<DefId, String>, String> {
    let mut backend_symbols = BTreeMap::new();

    for (id, record) in &products.link.records {
        validate_product_backend_symbol_id(products, *id, "link record")?;
        backend_symbols.insert(remap.def_id(*id)?, record.backend_symbol.clone());
    }

    Ok(backend_symbols)
}
```

- [ ] **Step 7: Bump product artifact format for the schema removal**

Removing the serialized `ProductIdentityTable.backend_symbols` field changes the product artifact schema, so Task 4 owns the format bump.

In `lib/src/products.rs`, change:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 31;
```

to:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 32;
```

In `rock-shared/src/sysroot.rs`, change:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 31;
```

to:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 32;
```

In `lib/src/products.rs`, update `product_artifact_format_version_matches_shared_contract`:

```rust
assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 32);
```

Keep the shared-contract equality assertion unchanged.

- [ ] **Step 8: Run product/link tests**

Run:

```bash
cargo test -p rock-lib products::tests::compiler_products_preserve_backend_symbols_in_link_records -- --exact --nocapture
cargo test -p rock-lib products::tests::compiler_products_preserve_link_records_for_real_local_id_zero -- --exact --nocapture
cargo test -p rock-lib products::tests::compiler_products_drop_link_records_for_ambiguous_link_ids -- --exact --nocapture
cargo test -p rock-lib tests::product_link_records_do_not_match_backend_symbols_from_display_names -- --exact --nocapture
cargo test -p rock-lib product_link_records -- --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_link_record_for_undeclared_callable_id -- --exact --nocapture
```

Expected after implementation: all listed tests PASS.

## Task 5: Add Missing Object Link Record Validation

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Add product-interface concrete helper functions**

Add these helpers after `product_id_has_interface_callable_binding` in `lib/src/crate_artifact/load.rs`:

```rust
fn product_function_requires_downstream_specialization(function: &ProductFunctionInterface) -> bool {
    !function.generic_params.is_empty() || !function.generic_param_ids.is_empty()
}

fn product_function_is_codegen_concrete(function: &ProductFunctionInterface) -> bool {
    function.params.iter().all(product_type_is_codegen_concrete)
        && product_type_is_codegen_concrete(&function.ret_type)
}

fn product_function_requires_object_link_record(function: &ProductFunctionInterface) -> bool {
    !product_function_requires_downstream_specialization(function)
        && product_function_is_codegen_concrete(function)
}

fn product_impl_method_requires_object_link_record(
    impl_def: &ProductImplInterface,
    method: &ProductFunctionInterface,
) -> bool {
    product_impl_link_shape_is_codegen_concrete(impl_def)
        && product_function_requires_object_link_record(method)
}

fn product_impl_link_shape_is_codegen_concrete(impl_def: &ProductImplInterface) -> bool {
    impl_def.type_generics.is_empty()
        && impl_def
            .receiver_arg_types
            .iter()
            .all(product_type_is_codegen_concrete)
        && impl_def
            .trait_arg_types
            .iter()
            .all(product_type_is_codegen_concrete)
        && impl_def
            .associated_types
            .iter()
            .all(|associated| product_type_is_codegen_concrete(&associated.ty))
}

fn product_type_is_codegen_concrete(ty: &Type) -> bool {
    match ty {
        Type::TypeVar(_) | Type::Generic(_) | Type::Projection { .. } => false,
        Type::Slice(inner) | Type::Pointer(inner) => product_type_is_codegen_concrete(inner),
        Type::Array(inner, _) => product_type_is_codegen_concrete(inner),
        Type::Reference { inner, .. } => product_type_is_codegen_concrete(inner),
        Type::Function { params, ret, .. } => {
            params.iter().all(product_type_is_codegen_concrete)
                && product_type_is_codegen_concrete(ret)
        }
        Type::Tuple(items) => items.iter().all(product_type_is_codegen_concrete),
        Type::Struct { args, .. } | Type::Enum { args, .. } => {
            args.iter().all(product_type_is_codegen_concrete)
        }
        Type::Error
        | Type::Unit
        | Type::Never
        | Type::I8
        | Type::I16
        | Type::Bool
        | Type::I64
        | Type::I32
        | Type::U64
        | Type::U16
        | Type::U32
        | Type::U8
        | Type::F32
        | Type::F64
        | Type::Str
        | Type::Char => true,
    }
}
```

If `Type` has additional variants in the current source, mirror the concrete/non-concrete rules from `hir_type_is_codegen_concrete` in `lib/src/hir/mod.rs` rather than defaulting to permissive string behavior.

- [ ] **Step 2: Add object link-record validation entry point**

Add this function near `backend_symbols_from_products` in `lib/src/crate_artifact/load.rs`:

```rust
fn validate_object_link_records(products: &CompilerProducts) -> Result<(), String> {
    if products.link.object_path.is_none() {
        return Ok(());
    }

    for (id, function) in &products.interface.functions {
        if product_function_requires_object_link_record(function)
            && !products.link.records.contains_key(id)
        {
            return Err(format!(
                "Product artifact object-backed function '{}' ID {}::{} is missing link record backend symbol",
                function.name, id.crate_id.0, id.local_id.0
            ));
        }
    }

    for (impl_id, impl_def) in &products.interface.impls {
        for (method_name, method) in &impl_def.methods {
            let method_id = ProductDefId::from(method.id);
            if product_impl_method_requires_object_link_record(impl_def, method)
                && !products.link.records.contains_key(&method_id)
            {
                return Err(format!(
                    "Product artifact object-backed impl {} method '{}' ID {}::{} is missing link record backend symbol",
                    impl_id.local_id.0, method_name, method_id.crate_id.0, method_id.local_id.0
                ));
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Call object link-record validation during product artifact load**

In `extern_record_from_products` in `lib/src/crate_artifact/load.rs`, after:

```rust
validate_product_interface_rows(products)?;
```

insert:

```rust
validate_object_link_records(products)?;
```

The sequence should be:

```rust
validate_product_interface_rows(products)?;
validate_object_link_records(products)?;
let backend_symbols = backend_symbols_from_products(products, remap)?;
let link = if products.link.object_path.is_some() {
    ExternCrateLink::object(
        resolve_product_object_path(products, artifact_path)?,
        backend_symbols,
    )
} else {
    ExternCrateLink::metadata_only(backend_symbols)
};
```

- [ ] **Step 4: Run artifact validation tests**

Run:

```bash
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_link_record_for_undeclared_callable_id -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_object_function_missing_link_record -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_rejects_object_impl_method_missing_link_record -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_allows_generic_function_without_link_record -- --exact --nocapture
cargo test -p rock-lib crate_artifact::load::product_tests::load_product_artifact_registers_object_backed_crate -- --exact --nocapture
```

Expected after implementation: all listed tests PASS.

## Task 6: Remove External Symbol Fallbacks And Align Provider Predicates

**Files:**
- Modify: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add link-record presence helper**

In `impl ExternCrateLink` in `lib/src/crate_system/extern_store.rs`, add this helper after `backend_symbol`:

```rust
fn has_backend_symbol(&self, id: DefId) -> bool {
    self.backend_symbols.contains_key(&id)
}
```

- [ ] **Step 2: Make provider predicates require link records**

Replace `concrete_impl_body_is_object_provided` with:

```rust
pub(crate) fn concrete_impl_body_is_object_provided(&self, imp: &HirImpl) -> bool {
    self.is_object_backed()
        && impl_link_shape_is_codegen_concrete(imp)
        && imp.methods.values().all(|method| {
            function_is_object_provided_candidate(method) && self.has_backend_symbol(method.id)
        })
}
```

Replace `impl_method_body_is_object_provided` with:

```rust
pub(crate) fn impl_method_body_is_object_provided(
    &self,
    imp: &HirImpl,
    method: &HirFunction,
) -> bool {
    self.is_object_backed()
        && impl_link_shape_is_codegen_concrete(imp)
        && function_is_object_provided_candidate(method)
        && self.has_backend_symbol(method.id)
}
```

Add `impl_link_shape_is_codegen_concrete` and `type_is_codegen_concrete` helpers so concrete trait arguments such as `impl Add I64 for I64` are object-provided when link records exist, while unresolved `Generic`, `TypeVar`, and `Projection` shapes are not.

- [ ] **Step 3: Remove imported function fallback**

In `ExternCrateRef::imported_function_symbol`, replace:

```rust
Some(
    self.record
        .link
        .backend_symbol(func.id)
        .map(str::to_string)
        .or_else(|| func.qualified_name.clone())
        .unwrap_or_else(|| interface_name.to_string()),
)
```

with:

```rust
self.record.link.backend_symbol(func.id).map(str::to_string)
```

Keep the existing early return guard.

- [ ] **Step 4: Remove imported impl method fallback**

In `ExternCrateRef::imported_impl_method_symbol`, replace:

```rust
Some(
    self.record
        .link
        .backend_symbol(method.id)
        .map(str::to_string)
        .or_else(|| method.qualified_name.clone())
        .unwrap_or_else(|| format!("{}_{}", imp.type_name, method_name)),
)
```

with:

```rust
let _ = method_name;
self.record.link.backend_symbol(method.id).map(str::to_string)
```

Keep the existing `provides_impl_method_body` guard.

- [ ] **Step 5: Remove mono fallback after imported impl method lookup**

In `record_imported_impl` in `lib/src/mono/external.rs`, replace:

```rust
let backend_symbol = dep
    .imported_impl_method_symbol(imp, method_name, method)
    .or_else(|| {
        method
            .is_method
            .then(|| method.qualified_name.clone())
            .flatten()
    })
    .unwrap_or_else(|| format!("{}_{}", imp.type_name, method_name));
```

with:

```rust
let Some(backend_symbol) = dep.imported_impl_method_symbol(imp, method_name, method) else {
    continue;
};
```

- [ ] **Step 6: Run external-store and mono-focused tests**

Run:

```bash
cargo test -p rock-lib crate_system::tests::extern_crate_ref_imported_function_symbol_requires_link_record -- --exact --nocapture
cargo test -p rock-lib crate_system::tests::extern_crate_ref_imported_impl_method_symbol_requires_link_record -- --exact --nocapture
cargo test -p rock-lib crate_system::tests::extern_crate_ref_imported_symbols_use_explicit_link_records -- --exact --nocapture
cargo test -p rock-lib mono::external -- --nocapture
```

Expected after implementation: all listed tests PASS.

## Task 7: Verify Product Artifact Format Version

**Files:**
- Verify: `lib/src/products.rs`
- Verify: `rock-shared/src/sysroot.rs`

- [ ] **Step 1: Verify the Task 4 format bump is present**

Task 4 owns the schema-removal format bump to `32`. If Task 4 has already been implemented, this task is verification-only; do not bump again.

Confirm `lib/src/products.rs` contains:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 32;
```

Confirm `rock-shared/src/sysroot.rs` contains:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 32;
```

Confirm `lib/src/products.rs` has `product_artifact_format_version_matches_shared_contract` asserting:

```rust
assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 32);
```

- [ ] **Step 2: Run format-version tests**

Run:

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact --nocapture
cargo test -p rock-lib product_artifact_rejects_unsupported_format_version -- --nocapture
```

Expected after implementation: all listed tests PASS.

## Task 8: Add Regression Audit And Remove Residue

**Files:**
- Modify: `lib/src/semantic_identity_audit.rs`
- Modify: any source file still reported by the residue scans.

- [ ] **Step 1: Add semantic identity audit coverage for Step 3 residues**

In `lib/src/semantic_identity_audit.rs`, add this test near other product/artifact/backend boundary tests:

```rust
#[test]
fn product_backend_symbols_are_link_record_owned() {
    assert_production_file_absent(
        "products.rs",
        &[
            "pub backend_symbols: BTreeMap<ProductDefId, String>",
            "identity_table.backend_symbols",
            "backend_symbols_from_link",
        ],
    );
    assert_production_file_absent(
        "lib.rs",
        &[
            "identity_table.backend_symbols",
            "product_id_for_exported_backend_symbol",
            "exported_product_symbol",
        ],
    );
    assert_production_file_absent(
        "crate_artifact/load.rs",
        &["identity_table.backend_symbols", "identity backend symbol"],
    );
}

#[test]
fn external_object_symbols_do_not_fall_back_to_source_names() {
    assert_production_file_absent(
        "crate_system/extern_store.rs",
        &[
            ".or_else(|| func.qualified_name.clone())",
            ".or_else(|| method.qualified_name.clone())",
            "unwrap_or_else(|| interface_name.to_string())",
            "unwrap_or_else(|| format!(\"{}_{}\", imp.type_name, method_name))",
        ],
    );
    assert_production_file_absent(
        "mono/external.rs",
        &[
            "method.qualified_name.clone()",
            "unwrap_or_else(|| format!(\"{}_{}\", imp.type_name, method_name))",
        ],
    );
}
```

If exact string literals need escaping to compile, preserve the same searched substrings with valid Rust string syntax.

- [ ] **Step 2: Run audit tests**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::product_backend_symbols_are_link_record_owned -- --exact --nocapture
cargo test -p rock-lib semantic_identity_audit::external_object_symbols_do_not_fall_back_to_source_names -- --exact --nocapture
```

Expected after implementation: both tests PASS.

- [ ] **Step 3: Run manual residue scans**

Run:

```bash
if /home/linuxbrew/.linuxbrew/bin/rg -n "identity_table\.backend_symbols|ProductIdentityTable[^\n]*backend_symbols|backend_symbols_from_link|product_id_for_exported_backend_symbol|exported_product_symbol" lib/src --glob '!semantic_identity_audit.rs'; then exit 1; else test "$?" -eq 1; fi
```

Expected after implementation: no output and exit 0.

Run:

```bash
if /home/linuxbrew/.linuxbrew/bin/rg -n "qualified_name\.clone\(\)|format!\(\"\{\}_\{\}\", imp\.type_name, method_name\)" lib/src/crate_system/extern_store.rs lib/src/mono/external.rs; then exit 1; else test "$?" -eq 1; fi
```

Expected after implementation: no output and exit 0.

## Task 9: Run Full Focused Verification

**Files:**
- No code edits expected.

- [ ] **Step 1: Run product and artifact test groups**

Run:

```bash
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib crate_system -- --nocapture
```

Expected after implementation: all tests PASS.

- [ ] **Step 2: Run downstream affected test groups**

Run:

```bash
cargo test -p rock-lib mono::external -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected after implementation: all tests PASS.

- [ ] **Step 3: Run integration suite**

Run:

```bash
cargo test -p rock-lib --test integration > /tmp/rock-lib-integration-step3.log 2>&1
```

Expected after implementation: exit 0. Inspect `/tmp/rock-lib-integration-step3.log` and confirm the final summary reports all integration tests passed.

- [ ] **Step 4: Run formatting and diff checks**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected after implementation: both commands produce no output and exit 0.

## Task 10: Mark Clean-Slate Audit Step 3 Complete

**Files:**
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md`

- [ ] **Step 1: Add Step 3 completion status after verification**

Only after Tasks 1-9 pass, update `CLEAN_SLATE_COMPILER_AUDIT.md` under `### Step 3: Make Product Link Records The Only Backend Symbol Source`.

Insert this status block immediately after the Step 3 heading:

```markdown
Status: Complete as of 2026-07-05. `ProductIdentityTable` no longer stores
backend symbols. Product artifact backend symbols are serialized only through
`ProductLinkData.records` and loaded into `ExternCrateLink.backend_symbols`.
Object-backed concrete callable artifacts now require explicit link records, and
production dependency symbol lookup no longer falls back to `qualified_name`,
display names, interface names, impl type names, trait names, or method names.
The product artifact format version was bumped for the schema change.

Validation evidence:

- `cargo test -p rock-lib products -- --nocapture`
- `cargo test -p rock-lib crate_artifact -- --nocapture`
- `cargo test -p rock-lib crate_system -- --nocapture`
- `cargo test -p rock-lib mono::external -- --nocapture`
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`
- `cargo test -p rock-lib --test integration`
- `cargo fmt --all --check`
- `git diff --check`
- Source residue scans found no production matches for removed identity-table
  backend-symbol storage or source-name backend-symbol fallbacks.
```

- [ ] **Step 2: Re-run final documentation checks**

Run:

```bash
git diff --check
/home/linuxbrew/.linuxbrew/bin/rg -n "Status: Complete as of 2026-07-05" CLEAN_SLATE_COMPILER_AUDIT.md
```

Expected after implementation: `git diff --check` exits 0; the `rg` command prints the Step 3 status line.

- [ ] **Step 3: Inspect final worktree summary**

Run:

```bash
git status --short
git diff --stat
```

Expected after implementation: only intended Step 3 files are modified, plus the approved spec/plan docs if they were not already committed. `MEMORY.md` may still be present as an unrelated unstaged user change; do not stage or modify it unless the user asks.

## Final Verification Checklist

Before claiming Step 3 is complete, confirm all of these are true from fresh command output:

- `ProductIdentityTable.backend_symbols` is absent from Rust source.
- `backend_symbols_from_link`, `product_id_for_exported_backend_symbol`, and `exported_product_symbol` are absent from Rust source.
- `ExternCrateRef` and mono external import logic do not synthesize backend symbols from `qualified_name` or `format!("{}_{}", ...)`.
- Object-backed concrete functions and impl methods without link records fail artifact load.
- Generic/downstream-specialized callables do not require link records.
- Artifact format version is `32` in both `lib/src/products.rs` and `rock-shared/src/sysroot.rs`.
- `CLEAN_SLATE_COMPILER_AUDIT.md` Step 3 has the completion status block.
- All verification commands in Task 9 and Task 10 pass.
