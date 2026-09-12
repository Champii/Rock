# ID-Backed Alias And Path Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete roadmap Task 4 and the strict alias/path-resolution subset of Task 5 by replacing string-to-string alias interfaces with canonical ID-backed aliases and removing lowerer-owned semantic alias maps.

**Architecture:** First make product/resolver/artifact alias persistence ID-backed only and bump product artifact format to `22`. Then remove `Lowerer` semantic alias maps by routing alias and path resolution through resolver/module-context APIs while keeping source strings only for diagnostics/display.

**Tech Stack:** Rust 2021, `rock-lib`, `ResolverTables`, `DefId`, `ProductDefId`, product artifacts via `bincode`/`serde`, focused Rust unit/integration tests, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Files And Responsibilities

- Modify `lib/src/collect/resolver.rs`: add explicit ID-backed alias categories and helper APIs for import, export, and module-local aliases.
- Modify `lib/src/products.rs`: bump product format to `22`, remove persistent string prelude export payload, and persist ID-backed alias data in `ProductIdentityTable`.
- Modify `rock-shared/src/sysroot.rs`: keep shared product artifact format at `22`.
- Modify `lib/src/crate_artifact/types.rs`: remove string-only root export interface and expose ID-backed root exports.
- Modify `lib/src/crate_system/extern_store.rs`: remove string-only prelude export metadata and expose ID-backed prelude exports.
- Modify `lib/src/crate_artifact/load.rs`: reject old/string-only alias data, remap ID-backed aliases, and construct dependency resolver/metadata from IDs.
- Modify `lib/src/collect/context.rs` and `lib/src/collect/mod.rs`: stop returning persistent string alias maps from collection; keep syntax-derived temporary maps only inside collection until resolver IDs are built.
- Modify `lib/src/lower/mod.rs`: remove lowerer-owned semantic alias fields and add narrow resolver facade methods for canonical alias display names and item lookup by ID.
- Modify `lib/src/lower/paths.rs`, `lib/src/lower/expression.rs`, `lib/src/lower/program.rs`, `lib/src/lower/crates/registration.rs`, `lib/src/lower/crates/bodies.rs`, `lib/src/lower/module_context.rs`, and `lib/src/lower/traits/conformance.rs`: replace direct string alias map lookups with resolver and ID-backed metadata calls.
- Modify `lib/src/lib.rs`: record stdlib prelude products through ID-backed helpers only.
- Modify `lib/src/mono/external.rs`: include module aliases in dependency generic-function alias propagation after string-only alias maps are removed.
- Modify docs at completion: `docs/superpowers/plans/master-audit-checklist.md`, `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, and this plan's verification notes.

---

## Task 1: Add Explicit ID-Backed Alias Tables And Format Version 22

**Files:**
- Modify: `lib/src/collect/resolver.rs`
- Modify: `lib/src/products.rs`
- Modify: `rock-shared/src/sysroot.rs`
- Test: `lib/src/collect/resolver.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Write failing resolver alias category test**

Add this test to `lib/src/collect/resolver.rs` inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn resolver_tables_resolve_distinct_alias_categories_by_id() {
    let import_id = def_id(21);
    let export_id = def_id(22);
    let module_id = def_id(23);
    let mut resolver = ResolverTables::default();

    resolver.insert_import_alias_with_name(
        "imported".to_string(),
        "dep::internal::value".to_string(),
        import_id,
    );
    resolver.insert_export_alias_with_name(
        "demo::public".to_string(),
        "demo::internal::value".to_string(),
        export_id,
    );
    resolver.insert_module_alias_with_name(
        "local".to_string(),
        "demo::module::local".to_string(),
        module_id,
    );

    assert_eq!(resolver.resolve_item_or_alias("imported"), Some(import_id));
    assert_eq!(resolver.resolve_item_or_alias("demo::public"), Some(export_id));
    assert_eq!(resolver.resolve_item_or_alias("local"), Some(module_id));
    assert_eq!(resolver.canonical_name(module_id), Some("demo::module::local"));
}
```

- [ ] **Step 2: Run failing resolver alias category test**

Run: `cargo test -p rock-lib resolver_tables_resolve_distinct_alias_categories_by_id -- --exact`

Expected: FAIL because `insert_module_alias_with_name` and `module_aliases` do not exist.

- [ ] **Step 3: Add module-local alias table to `ResolverTables`**

In `lib/src/collect/resolver.rs`, change `ResolverTables` to include `module_aliases` and update lookup:

```rust
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolverTables {
    pub module_paths: HashMap<String, ModuleId>,
    pub module_names_by_id: HashMap<ModuleId, String>,
    pub item_paths: HashMap<String, DefId>,
    pub item_names_by_id: HashMap<DefId, String>,
    pub import_aliases: HashMap<String, DefId>,
    pub export_aliases: HashMap<String, DefId>,
    pub module_aliases: HashMap<String, DefId>,
}
```

Update `resolve_item_or_alias`:

```rust
pub fn resolve_item_or_alias(&self, name: &str) -> Option<DefId> {
    self.item_paths
        .get(name)
        .copied()
        .or_else(|| self.import_aliases.get(name).copied())
        .or_else(|| self.export_aliases.get(name).copied())
        .or_else(|| self.module_aliases.get(name).copied())
}
```

Add the helper:

```rust
pub fn insert_module_alias_with_name(&mut self, alias: String, source: String, id: DefId) {
    self.module_aliases.insert(alias, id);
    self.item_names_by_id.entry(id).or_insert(source);
}
```

Add `module_aliases: HashMap::new()` to the `ResolverTables` construction in `build_resolver_tables`.

- [ ] **Step 4: Write failing product format test**

Update `product_artifact_format_version_matches_shared_contract` in `lib/src/products.rs` so it expects version `22`:

```rust
#[test]
fn product_artifact_format_version_matches_shared_contract() {
    assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 22);
    assert_eq!(
        PRODUCT_ARTIFACT_FORMAT_VERSION,
        rock_shared::sysroot::PRODUCT_ARTIFACT_FORMAT_VERSION
    );
}
```

- [ ] **Step 5: Run failing product format test**

Run: `cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact`

Expected: FAIL because compiler/shared constants are still `21`.

- [ ] **Step 6: Bump product artifact format constants**

In `lib/src/products.rs`, set:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 22;
```

In `rock-shared/src/sysroot.rs`, set:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 22;
```

- [ ] **Step 7: Run Task 1 focused tests**

Run: `cargo test -p rock-lib resolver_tables_resolve_distinct_alias_categories_by_id -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact`

Expected: PASS.

- [ ] **Step 8: Commit Task 1**

Run:

```bash
git add lib/src/collect/resolver.rs lib/src/products.rs rock-shared/src/sysroot.rs
git commit -m "add id backed alias resolver categories"
```

---

## Task 2: Remove String Prelude Export Payload From Products

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/lib.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Write failing ID-only prelude product test**

Add this test to `lib/src/products.rs` tests near `compiler_products_record_prelude_export_ids`:

```rust
#[test]
fn compiler_products_record_prelude_export_ids_without_string_payload() {
    let hir = resolved_hir_for_products();
    let mut products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let plain_def_id = DefId::new(CrateId(0), LocalDefId(0));
    let plain_product_id = ProductDefId::from(plain_def_id);

    products.record_prelude_export_ids([(
        "plain".to_string(),
        crate::crate_artifact::ArtifactExport {
            source: "plain".to_string(),
            id: plain_def_id,
        },
    )]);

    assert_eq!(
        products.identity_table.prelude_export_names.get("plain"),
        Some(&plain_product_id)
    );
    assert!(
        products.prelude_exports.is_empty(),
        "prelude exports must persist by ProductDefId, not string source"
    );
}
```

- [ ] **Step 2: Run failing ID-only prelude product test**

Run: `cargo test -p rock-lib compiler_products_record_prelude_export_ids_without_string_payload -- --exact`

Expected: FAIL because `record_prelude_export_ids` still populates `prelude_exports`.

- [ ] **Step 3: Stop serializing and recording string prelude products**

In `lib/src/products.rs`, keep the temporary field for Task 3 compile compatibility but mark it non-persistent:

```rust
#[serde(skip, default)]
pub prelude_exports: BTreeMap<String, String>,
```

Delete `record_prelude_exports`. Change `record_prelude_export_ids` to only populate `identity_table.prelude_export_names`:

```rust
pub fn record_prelude_export_ids(
    &mut self,
    exports: impl IntoIterator<Item = (String, crate::crate_artifact::ArtifactExport)>,
) {
    for (alias, export) in exports {
        if let Some(id) = self.product_def_id_for_export(&export) {
            self.identity_table.prelude_export_names.insert(alias, id);
        }
    }
}
```

Keep `CompilerProducts::from_resolved_hir` constructing `prelude_exports: BTreeMap::new()` until Task 3 removes the field. Remove tests that call `record_prelude_exports`. Replace assertions on `products.prelude_exports` with assertions on `identity_table.prelude_export_names` or `products.prelude_exports.is_empty()`.

- [ ] **Step 4: Update product emission call sites**

In `lib/src/lib.rs`, keep product recording ID-backed:

```rust
products.record_prelude_export_ids(
    decls.stdlib_prelude_export_ids
        .iter()
        .map(|(alias, export)| (alias.clone(), export.clone())),
);
```

Remove any call to `record_prelude_exports`.

- [ ] **Step 5: Run Task 2 focused tests**

Run: `cargo test -p rock-lib compiler_products_record_prelude_export_ids_without_string_payload -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib compiler_products_roundtrip_preserves_prelude_export_ids -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact`

Expected: PASS with version `22`.

- [ ] **Step 6: Commit Task 2**

Run:

```bash
git add lib/src/products.rs lib/src/lib.rs
git commit -m "persist prelude aliases by id only"
```

---

## Task 3: Load Product Prelude And Root Exports From IDs Only

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_artifact/types.rs`
- Modify: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/collect/mod.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Add old-format product rejection regression test**

In `lib/src/products.rs`, add this test near `compiler_products_reject_unknown_product_artifact_format`:

```rust
#[test]
fn compiler_products_reject_format_21_product_artifacts() {
    let hir = resolved_hir_for_products();
    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let artifact = ProductArtifact {
        format_version: 21,
        products,
    };
    let bytes = bincode::serialize(&artifact).unwrap();

    let err = CompilerProducts::from_artifact_bytes(&bytes).unwrap_err();

    assert!(
        err.contains("Unsupported product artifact format 21")
            && err.contains("expected 22"),
        "expected format-21 rejection, got {err}"
    );
}
```

- [ ] **Step 2: Run old-format product rejection regression test**

Run: `cargo test -p rock-lib compiler_products_reject_format_21_product_artifacts -- --exact`

Expected: PASS after Task 1 bumped the format to `22`; this proves format-21/string-only artifacts are rejected through format validation.

- [ ] **Step 3: Remove string-only prelude loading path and metadata**

In `lib/src/products.rs`, remove this temporary field from `CompilerProducts` and all struct literals:

```rust
#[serde(skip, default)]
pub prelude_exports: BTreeMap<String, String>,
```

In `lib/src/crate_artifact/load.rs`, change `normalize_product_prelude_exports` so it iterates only `products.identity_table.prelude_export_names` and derives display/source strings from ID metadata. Remove the loop over `products.prelude_exports`.

Use this exact return shape:

```rust
fn normalize_product_prelude_exports(
    products: &CompilerProducts,
    remap: &ProductIdentityRemap,
    interface: &super::ArtifactCrateInterface,
    crate_name: &str,
    prelude_prefix: &str,
) -> Result<BTreeMap<String, ArtifactExport>, String> {
    let mut normalized_ids = BTreeMap::new();

    for (alias, id) in &products.identity_table.prelude_export_names {
        let display_name = product_display_name(products, *id).ok_or_else(|| {
            format!(
                "Product artifact prelude export '{}' has no display name or canonical DefId",
                alias
            )
        })?;
        let def_id = product_def_id_to_prelude_export_def_id(products, remap, *id)?.ok_or_else(|| {
            format!(
                "Product artifact prelude export '{}' has no canonical prelude item ID",
                alias
            )
        })?;
        let source = qualify_product_name(crate_name, &display_name);
        normalized_ids.insert(alias.clone(), ArtifactExport { source, id: def_id });
    }

    for name in product_interface_prelude_names(interface, prelude_prefix) {
        if let Some(alias) = name.strip_prefix(&format!("{}::", prelude_prefix)) {
            if !normalized_ids.contains_key(alias) {
                let export = artifact_export_for_source(interface, &name).ok_or_else(|| {
                    format!(
                        "Product artifact prelude export '{}' references '{}' without a canonical DefId",
                        alias, name
                    )
                })?;
                normalized_ids.insert(alias.to_string(), export);
            }
        }
    }

    Ok(normalized_ids)
}
```

Change `artifact_export_for_source` to use only ID-backed interface data and remove resolver/string fallback arguments:

```rust
fn artifact_export_for_source(
    interface: &super::ArtifactCrateInterface,
    source: &str,
) -> Option<ArtifactExport> {
    interface
        .functions
        .get(source)
        .map(|function| function.id)
        .or_else(|| interface.structs.get(source).map(|value| value.id))
        .or_else(|| interface.enums.get(source).map(|value| value.id))
        .or_else(|| interface.traits.get(source).map(|value| value.id))
        .or_else(|| {
            interface
                .externs
                .iter()
                .find(|ext| ext.name == source)
                .map(|ext| ext.id)
        })
        .map(|id| ArtifactExport {
            source: source.to_string(),
            id,
        })
}
```

Adjust `extern_crate_from_products` and `ExternCrateMetadata::new` calls to pass only ID-backed prelude exports.

- [ ] **Step 4: Remove string metadata fields**

In `lib/src/crate_system/extern_store.rs`, remove:

```rust
prelude_exports: BTreeMap<String, String>,
```

Change `ExternCrateMetadata::new` to accept only:

```rust
pub(crate) fn new(
    interface: ArtifactCrateInterface,
    resolver: ResolverTables,
    prelude_export_ids: BTreeMap<String, ArtifactExport>,
) -> Self
```

Delete `prelude_exports(&self) -> &BTreeMap<String, String>`.

Update test helpers that construct extern metadata, including `add_artifact_extern_crate` in `lib/src/collect/mod.rs`, so they call:

```rust
crate::crate_system::ExternCrateMetadata::new(
    interface,
    resolver,
    prelude_export_ids,
)
```

In `lib/src/crate_artifact/types.rs`, remove:

```rust
pub root_exports: BTreeMap<String, String>,
```

Update construction in `interface_from_products` to omit `root_exports` and keep `root_export_ids`.

- [ ] **Step 5: Run Task 3 focused artifact tests**

Run: `cargo test -p rock-lib compiler_products_reject_format_21_product_artifacts -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib load_product_artifact_records_id_backed_prelude_exports -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib load_product_artifact_records_root_export_ids -- --exact`

Expected: PASS.

- [ ] **Step 6: Commit Task 3**

Run:

```bash
git add lib/src/products.rs lib/src/crate_artifact/types.rs lib/src/crate_system/extern_store.rs lib/src/crate_artifact/load.rs lib/src/collect/mod.rs
git commit -m "load artifact aliases from ids only"
```

---

## Task 4: Persist Import, Export, And Module Aliases By Product IDs

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/collect/mod.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Write failing product resolver alias round-trip test**

Add this test to `lib/src/products.rs` tests:

```rust
#[test]
fn compiler_products_persist_resolver_aliases_by_product_id() {
    let alias_id = DefId::new(CrateId(0), LocalDefId(4));
    let mut hir = resolved_hir_for_products();
    hir.resolver.insert_import_alias_with_name(
        "short".to_string(),
        "demo::plain".to_string(),
        alias_id,
    );

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );

    assert_eq!(
        products.identity_table.import_alias_names.get("short"),
        Some(&ProductDefId::from(alias_id))
    );
}
```

- [ ] **Step 2: Run failing product resolver alias test**

Run: `cargo test -p rock-lib compiler_products_persist_resolver_aliases_by_product_id -- --exact`

Expected: FAIL because `ProductIdentityTable::import_alias_names` does not exist.

- [ ] **Step 3: Add product alias identity fields**

In `lib/src/products.rs`, extend `ProductIdentityTable`:

```rust
#[serde(default)]
pub import_alias_names: BTreeMap<String, ProductDefId>,
#[serde(default)]
pub module_alias_names: BTreeMap<String, ProductDefId>,
```

Keep `export_names` as the public/root export ID map and merge resolver export aliases into it. Use `ambiguous_export_names` unchanged.

In `CompilerProducts::from_resolved_hir`, after metadata IDs and `id_remap` are finalized, remap resolver aliases:

```rust
record_product_aliases(
    &mut identity_table.import_alias_names,
    &hir.resolver.import_aliases,
    &id_remap,
);
record_product_aliases(
    &mut identity_table.export_names,
    &hir.resolver.export_aliases,
    &id_remap,
);
record_product_aliases(
    &mut identity_table.module_alias_names,
    &hir.resolver.module_aliases,
    &id_remap,
);
```

Add helper:

```rust
fn record_product_aliases(
    out: &mut BTreeMap<String, ProductDefId>,
    aliases: &HashMap<String, DefId>,
    id_remap: &BTreeMap<ProductDefId, BTreeSet<ProductDefId>>,
) {
    for (alias, id) in aliases {
        let mut product_id = ProductDefId::from(*id);
        remap_product_def_id_owner(&mut product_id, id_remap);
        out.insert(alias.clone(), product_id);
    }
}
```

- [ ] **Step 4: Write failing artifact resolver alias remap test**

Add this test to `lib/src/crate_artifact/load.rs` product tests:

```rust
#[test]
fn load_product_artifact_remaps_id_backed_import_aliases() {
    let (base, _cleanup) = temp_test_dir("id_backed_import_aliases");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();
    let mut products = product_with_function("dep", ProductCrateId(0), 1, object_path);
    products
        .identity_table
        .import_alias_names
        .insert("short".to_string(), product_def_id(0, 1));

    let artifact_path = base.join("dep.rkca");
    products.write_artifact_to_path(&artifact_path).unwrap();
    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path(artifact_path).unwrap();
    let record = ctx.extern_crate("dep").unwrap();
    let resolver = record.metadata().resolver();

    assert_eq!(
        resolver.resolve_item_or_alias("short"),
        resolver.resolve_item_or_alias("dep::answer")
    );
}
```

- [ ] **Step 5: Run failing artifact resolver alias remap test**

Run: `cargo test -p rock-lib load_product_artifact_remaps_id_backed_import_aliases -- --exact`

Expected: FAIL because artifact resolver construction ignores `import_alias_names`.

- [ ] **Step 6: Load product aliases into dependency resolver**

In `resolver_from_products` in `lib/src/crate_artifact/load.rs`, add import and module alias remapping:

```rust
for (alias, id) in &products.identity_table.import_alias_names {
    if let Some(def_id) = product_def_id_to_existing_def_id(products, remap, *id)? {
        let source = resolver
            .canonical_name(def_id)
            .ok_or_else(|| {
                format!(
                    "Product artifact import alias '{}' has no canonical display name",
                    alias
                )
            })?
            .to_string();
        resolver.insert_import_alias_with_name(alias.clone(), source, def_id);
    } else {
        return Err(format!(
            "Product artifact import alias '{}' has no canonical DefId",
            alias
        ));
    }
}

for (alias, id) in &products.identity_table.module_alias_names {
    if let Some(def_id) = product_def_id_to_existing_def_id(products, remap, *id)? {
        let source = resolver
            .canonical_name(def_id)
            .ok_or_else(|| {
                format!(
                    "Product artifact module alias '{}' has no canonical display name",
                    alias
                )
            })?
            .to_string();
        resolver.insert_module_alias_with_name(alias.clone(), source, def_id);
    } else {
        return Err(format!(
            "Product artifact module alias '{}' has no canonical DefId",
            alias
        ));
    }
}
```

- [ ] **Step 7: Run Task 4 product/artifact tests**

Run: `cargo test -p rock-lib compiler_products_persist_resolver_aliases_by_product_id -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib load_product_artifact_remaps_id_backed_import_aliases -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib product_artifact -- --nocapture`

Expected: PASS.

- [ ] **Step 8: Commit Task 4**

Run:

```bash
git add lib/src/products.rs lib/src/crate_artifact/load.rs lib/src/collect/mod.rs
git commit -m "persist resolver aliases by product id"
```

---

## Task 5: Remove Lowerer-Owned Semantic Alias Maps

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/crates/registration.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/lower/module_context.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/collect/mod.rs`
- Test: `lib/src/lower/paths.rs`
- Test: `lib/src/lower/expression.rs`

- [ ] **Step 1: Rewrite module-local alias test to use resolver IDs**

In `lib/src/lower/paths.rs`, replace `lower_module_local_alias_function_reference_records_function_id` with this resolver-backed version:

```rust
#[test]
fn module_local_alias_resolves_through_resolver_without_lowerer_string_map() {
    let mut lowerer = Lowerer::new();
    let function_id = def_id(32);
    lowerer.functions.insert(
        "demo::math::answer".to_string(),
        test_function(function_id, "answer"),
    );
    lowerer.resolver.insert_module_alias_with_name(
        "answer".to_string(),
        "demo::math::answer".to_string(),
        function_id,
    );
    lowerer.scope.define_alias(
        "answer".to_string(),
        Type::Function(Vec::new(), Box::new(Type::I64)),
        false,
    );

    let expr = lowerer.lower_identifier_path(&identifier_path(&["answer"]));

    match expr.kind {
        HirExprKind::ResolvedVar(HirVarRef { name, target: HirVarTarget::Function(id) }) => {
            assert_eq!(name, "demo::math::answer");
            assert_eq!(id, function_id);
        }
        other => panic!("expected resolver-backed module alias, got {other:?}"),
    }
}
```

Also update `lower_import_alias_function_reference_records_function_id`, `lower_nested_scope_module_local_alias_function_reference_records_function_id`, `lower_local_shadowing_module_local_alias_stays_var`, and `lower_import_alias_extern_reference_records_extern_id` to insert aliases through `lowerer.resolver.insert_import_alias_with_name` or `lowerer.resolver.insert_module_alias_with_name` instead of writing `lowerer.import_aliases` or `lowerer.module_local_aliases`.

- [ ] **Step 2: Run failing module-local resolver test**

Run: `cargo test -p rock-lib module_local_alias_resolves_through_resolver_without_lowerer_string_map -- --exact`

Expected: FAIL while `lower_identifier_path` still depends on `module_local_aliases` for canonical alias display/target resolution.

- [ ] **Step 3: Add resolver-backed display helper**

In `lib/src/lower/mod.rs`, add:

```rust
pub(crate) fn canonical_name_for_alias_or_item_lossy(&self, name: &str) -> String {
    self.canonical_name_for_alias_or_item(name)
        .unwrap_or_else(|| name.to_string())
}

pub(crate) fn function_by_def_id(&self, id: DefId) -> Option<&HirFunction> {
    self.functions.values().find(|function| function.id == id)
}

pub(crate) fn extern_by_def_id(&self, id: DefId) -> Option<&HirExtern> {
    self.externs.iter().find(|extern_| extern_.id == id)
}

pub(crate) fn top_level_var_target_by_def_id(&self, id: DefId) -> Option<HirVarTarget> {
    self.function_by_def_id(id)
        .map(|function| HirVarTarget::Function(function.id))
        .or_else(|| self.extern_by_def_id(id).map(|extern_| HirVarTarget::Extern(extern_.id)))
}
```

- [ ] **Step 4: Replace path alias map lookups**

In `lib/src/lower/paths.rs`, update `should_instantiate_scoped_identifier`:

```rust
fn should_instantiate_scoped_identifier(
    &self,
    name: &str,
    binding_scope: Option<usize>,
) -> bool {
    binding_scope == Some(0) || self.resolver.resolve_item_or_alias(name).is_some()
}
```

Replace `module_local_aliases.get(name)` / `import_aliases.get(name)` canonical-name fallback with:

```rust
let hir_name = if binding_is_alias {
    self.canonical_name_for_alias_or_item_lossy(name)
} else {
    name.clone()
};
```

When a binding is an alias, resolve the target ID first:

```rust
if binding_is_alias {
    if let Some(id) = self.resolve_item_def_id(name) {
        if let Some(target) = self.top_level_var_target_by_def_id(id) {
            let ty = self
                .function_by_def_id(id)
                .map(|func| self.instantiate_function_type(func))
                .or_else(|| {
                    self.extern_by_def_id(id)
                        .map(|ext| Type::Function(ext.params.clone(), Box::new(ext.ret.clone())))
                })
                .unwrap_or_else(|| binding.ty.clone());
            return HirExpr {
                ty,
                kind: HirExprKind::ResolvedVar(HirVarRef {
                    name: self.canonical_name_for_alias_or_item_lossy(name),
                    target,
                }),
                span,
            };
        }
    }
}
```

- [ ] **Step 5: Write failing custom operator resolver alias test**

Add this test to `lib/src/lower/expression.rs` tests:

```rust
#[test]
fn custom_operator_module_alias_lowers_through_resolver_id() {
    let mut lowerer = Lowerer::new();
    let function_id = def_id(30);
    lowerer.functions.insert(
        "demo::ops::%%".to_string(),
        test_function(function_id, "demo::ops::%%", vec![Type::I64, Type::I64], Type::I64),
    );
    lowerer.resolver.insert_module_alias_with_name(
        "%%".to_string(),
        "demo::ops::%%".to_string(),
        function_id,
    );
    lowerer.infix_precedence.insert("%%".to_string(), 5);

    let expr = Expression::BinopExpr(
        int_expr(1),
        Operator { value: "%%".to_string(), span: Span::default() },
        Box::new(Expression::UnaryExpr(int_expr(2))),
    );
    let hir = lowerer.lower_expression(&expr);

    let HirExprKind::Call(callee, _, Some(HirCallTarget::Function(id))) = hir.kind else {
        panic!("expected resolver-backed custom operator call, got {:?}", hir.kind);
    };
    assert_eq!(id, function_id);
    match callee.kind {
        HirExprKind::ResolvedVar(reference) => assert_eq!(reference.name, "demo::ops::%%"),
        other => panic!("expected resolved operator callee, got {other:?}"),
    }
}
```

- [ ] **Step 6: Run failing custom operator resolver alias test**

Run: `cargo test -p rock-lib custom_operator_module_alias_lowers_through_resolver_id -- --exact`

Expected: FAIL because custom operator lowering still checks `module_local_aliases` / `import_aliases`.

- [ ] **Step 7: Replace custom operator alias map lookups**

In `lib/src/lower/expression.rs`, replace `module_local_aliases` and `import_aliases` lookup branches with resolver ID lookup:

```rust
if func_binding.is_none() {
    if let Some(id) = self.resolve_item_def_id(op_str.as_str()) {
        if let Some(func) = self.function_by_def_id(id).cloned() {
            let hir_name = self.canonical_name_for_alias_or_item_lossy(op_str.as_str());
            func_binding = Some((
                self.instantiate_function_type(&func),
                hir_name,
                Some(HirVarTarget::Function(func.id)),
            ));
        }
    }
}
```

Keep the scope local fallback for local function-valued operators.

- [ ] **Step 8: Replace remaining lowerer alias-map consumers**

In `lib/src/lower/crates/registration.rs`, remove the `interface.root_exports` and `metadata.prelude_exports()` branches. Keep only `root_export_ids` and `prelude_export_ids`, and insert prelude aliases through the resolver helper:

```rust
self.resolver.insert_import_alias_with_name(
    short_name.clone(),
    export.source.clone(),
    export.id,
);
```

In `sync_export_alias_functions`, derive aliases from `self.resolver.export_aliases` instead of `self.export_function_aliases`:

```rust
let aliases: Vec<(String, DefId)> = self
    .resolver
    .export_aliases
    .iter()
    .map(|(alias, id)| (alias.clone(), *id))
    .collect();

for (alias, source_id) in aliases {
    let Some(source) = self.canonical_name_for_def_id(source_id).map(str::to_string) else {
        continue;
    };
    let Some(source_func) = self.functions.get(&source).cloned() else {
        continue;
    };

    if source_func.body.stmts.is_empty() {
        continue;
    }

    if let Some(alias_func) = self.functions.get_mut(&alias) {
        alias_func.generic_params = source_func.generic_params.clone();
        alias_func.generic_param_ids = source_func.generic_param_ids.clone();
        alias_func.params = source_func.params.clone();
        alias_func.ret_type = source_func.ret_type.clone();
        alias_func.body = source_func.body.clone();
        alias_func.is_method = source_func.is_method;
        alias_func.is_unsafe = source_func.is_unsafe;
        alias_func.qualified_name = source_func.qualified_name.clone();
    }
}
```

In `lib/src/lower/program.rs`, delete `artifact_root_exports` fallback from `glob_import_targets`; return targets from `artifact_root_export_ids` by mapping `ArtifactExport.source`. In `import_qualified_name`, stop writing `self.import_aliases`; if the qualified target has a resolver ID, call `insert_import_alias_with_name` and always keep the existing scope alias behavior.

In `lib/src/lower/crates/bodies.rs`, replace `self.module_local_aliases.insert(short.clone(), qualified)` with resolver insertion after resolving the target ID:

```rust
if let Some(id) = self.resolve_item_def_id(&qualified) {
    self.resolver
        .insert_module_alias_with_name(short.clone(), qualified.clone(), id);
}
```

Return `Vec<(String, Option<DefId>)>` from `inject_module_local_aliases` so `lib/src/lower/module_context.rs` can restore or remove transient `resolver.module_aliases` entries after the scoped visit:

```rust
let added_aliases = lowerer.inject_module_local_aliases(module, module_prefix);

visit(lowerer);

for (alias, previous) in added_aliases {
    if let Some(previous) = previous {
        lowerer.resolver.module_aliases.insert(alias, previous);
    } else {
        lowerer.resolver.module_aliases.remove(&alias);
    }
}
```

In `lib/src/lower/traits/conformance.rs`, replace cloned `import_aliases` lookup with `self.resolve_item_def_id` plus `self.trait_by_id`/`self.trait_by_resolved_name`, keeping the existing diagnostic behavior when no trait target exists.

In `lib/src/lower/mod.rs`, replace `resolve_owner_def_id_inner`'s `stdlib_prelude_exports` source-string recursion with `stdlib_prelude_export_ids.get(short_name).map(|export| export.id)`.

- [ ] **Step 9: Remove lowerer string alias fields and initialization**

In `lib/src/lower/mod.rs`, remove these fields from `Lowerer` and both constructors:

```rust
pub(crate) import_aliases: HashMap<String, String>,
pub(crate) stdlib_prelude_exports: HashMap<String, Option<String>>,
pub(crate) artifact_root_exports: HashMap<String, HashMap<String, String>>,
pub(crate) export_function_aliases: HashMap<String, String>,
pub(crate) module_local_aliases: HashMap<String, String>,
```

Keep `stdlib_prelude_export_ids`, `artifact_root_export_ids`, `resolver`, and `dependency_resolvers`.

- [ ] **Step 10: Run Task 5 focused tests**

Run: `cargo test -p rock-lib module_local_alias_resolves_through_resolver_without_lowerer_string_map -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib custom_operator_module_alias_lowers_through_resolver_id -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib custom_operator_import_alias_lowers_callee_to_function_id -- --exact`

Expected: PASS after migration to resolver IDs.

- [ ] **Step 11: Commit Task 5**

Run:

```bash
git add lib/src/lower/mod.rs lib/src/lower/paths.rs lib/src/lower/expression.rs lib/src/lower/program.rs lib/src/lower/crates/registration.rs lib/src/lower/crates/bodies.rs lib/src/lower/module_context.rs lib/src/lower/traits/conformance.rs lib/src/collect/mod.rs
git commit -m "resolve lowerer aliases through resolver ids"
```

---

## Task 6: Remove Collection String Alias Outputs From Phase Boundaries

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/pipeline.rs`
- Test: `lib/src/collect/mod.rs`

- [ ] **Step 1: Rewrite collection boundary audit test**

In `lib/src/collect/mod.rs`, replace the string-map assertion in `collect_records_dependency_import_alias_by_canonical_def_id` with this resolver-only assertion block:

```rust
assert!(decls.functions.contains_key("dep::io::answer"));
assert!(!decls.functions.contains_key("dep::answer"));
assert_eq!(
    decls.resolver.import_aliases.get("answer"),
    Some(&answer_id)
);
assert_eq!(
    decls.resolver.item_names_by_id.get(&answer_id),
    Some(&"dep::io::answer".to_string())
);
```

- [ ] **Step 2: Run failing collection boundary audit test**

Run: `cargo test -p rock-lib collect_records_dependency_import_alias_by_canonical_def_id -- --exact`

Expected: FAIL until the test no longer references `decls.import_aliases` and collection stops exposing the removed field.

- [ ] **Step 3: Remove alias maps from `LocalCollection` and `CollectionOutput`**

In `lib/src/collect/mod.rs`, remove these fields from `CollectionOutput`:

```rust
pub import_aliases: HashMap<String, String>,
pub stdlib_prelude_exports: HashMap<String, Option<String>>,
pub artifact_root_exports: HashMap<String, HashMap<String, String>>,
pub export_aliases: HashMap<String, String>,
pub export_function_aliases: HashMap<String, String>,
```

Keep:

```rust
pub stdlib_prelude_export_ids: HashMap<String, ArtifactExport>,
pub artifact_root_export_ids: HashMap<String, HashMap<String, ArtifactExport>>,
pub resolver: ResolverTables,
```

In `lib/src/collect/context.rs`, remove matching fields from `LocalCollection`. Keep these collection-internal fields on `CollectContext` until `finish()` builds resolver/ID outputs: `import_aliases`, `explicit_import_aliases`, `stdlib_prelude_exports`, `artifact_root_exports`, `canonical_import_aliases`, `export_aliases`, and `export_function_aliases`.

- [ ] **Step 4: Update lower pipeline construction**

Update call sites that move `CollectionOutput` into `Lowerer` so they stop assigning removed fields. In `lib/src/lower/pipeline.rs` and `lib/src/lower/program.rs`, keep only ID-backed alias assignments:

```rust
lowerer.resolver = decls.resolver.clone();
lowerer.current_def_ids = decls.current_def_ids.clone();
lowerer.stdlib_prelude_export_ids = decls.stdlib_prelude_export_ids.clone();
lowerer.artifact_root_export_ids = decls.artifact_root_export_ids.clone();
```

- [ ] **Step 5: Run Task 6 focused tests**

Run: `cargo test -p rock-lib collect_records_dependency_import_alias_by_canonical_def_id -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib collect_resolves_export_function_aliases_to_canonical_def_ids -- --exact`

Expected: PASS, proving export aliases still resolve through IDs.

- [ ] **Step 6: Commit Task 6**

Run:

```bash
git add lib/src/collect/context.rs lib/src/collect/mod.rs lib/src/lower/program.rs lib/src/lower/pipeline.rs
git commit -m "remove string alias collection outputs"
```

---

## Task 7: Final Alias Audit, Docs, Review, And Verification

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/2026-05-26-id-backed-alias-and-path-resolution.md`

- [ ] **Step 1: Run final alias audit grep**

Run:

```bash
rg "HashMap<String, String>|prelude_exports: BTreeMap<String, String>|root_exports: BTreeMap<String, String>|import_aliases: HashMap<String, String>|module_local_aliases|artifact_root_exports|export_function_aliases|record_prelude_exports|PRODUCT_ARTIFACT_FORMAT_VERSION" lib/src rock-shared/src docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
```

Expected: remaining `HashMap<String, String>` hits are unrelated display/metadata maps or collection-local syntax maps with no phase-boundary semantics; no `Lowerer` fields or product/artifact persistent alias maps remain; product artifact version hits are `22`.

- [ ] **Step 2: Update master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, under Real Collection And Name Resolution:

Add evidence:

```markdown
- `ResolverTables`, product artifacts, and artifact loading now persist import/export/prelude/module-local/root aliases through canonical IDs; string-to-string alias maps are no longer product or lowering phase-boundary contracts.
```

Move these items out of Still to do:

```markdown
- Remove lowering-time string alias compatibility maps once resolver outputs are canonical, including import aliases, module-local aliases, prelude exports, root artifact exports, and export function aliases.
- Replace collection/declaration string alias data with ID-backed alias tables as the persistent interface between collection, resolver, lowering, artifacts, and products.
```

Keep broader source/path/name resolution extraction still open for Task 5 if any non-alias lowerer path work remains.

- [ ] **Step 3: Update ordered roadmap**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update Task 4 table row to `Complete` with evidence that aliases persist by ID and product format is `22`.

Update Task 5 row to `Complete for strict alias/path-resolution subset` if lowerer semantic alias maps are removed, with remaining work noting broader path/type/module ownership extraction.

Update Task 4 section status:

```markdown
**Status:** Complete. Persistent import/export/prelude/module-local/root alias interfaces are ID-backed through resolver/product/artifact metadata, product artifact format `22` rejects old string-only alias contracts, and remaining strings are display/diagnostic metadata.
```

- [ ] **Step 4: Add final verification notes to this plan**

Append `## Final Verification` to this file with:

- exact audit grep command and classification
- exact focused alias tests run
- `cargo test -p rock-lib product_artifact`
- `cargo test -p rock-lib`
- `cargo fmt --all --check`
- `git diff --check`
- zero-test filters and replacements, if any

- [ ] **Step 5: Request code review**

Use the `requesting-code-review` skill. Ask the reviewer to verify:

- Task 4 persistent alias interfaces are ID-backed and complete.
- Product artifact format is `22` and old string-only artifacts are not accepted.
- `Lowerer` no longer owns semantic string alias maps.
- Task 5 completion claim is limited to the strict alias/path-resolution subset.
- Broader Task 5/lowerer decomposition, Task 11, Task 13, Task 21, and formatter work remain open.

- [ ] **Step 6: Run final verification commands**

Run these commands in order:

```bash
cargo test -p rock-lib alias
cargo test -p rock-lib prelude_export
cargo test -p rock-lib product_artifact
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

If a focused filter matches zero tests, replace it with the exact test names added in this implementation and record the replacement in `## Final Verification`.

- [ ] **Step 7: Commit final docs**

Run:

```bash
git add docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/2026-05-26-id-backed-alias-and-path-resolution.md
git commit -m "mark id backed alias cleanup complete"
```

---

## Completion Criteria

- Product artifact format is `22` in both `lib/src/products.rs` and `rock-shared/src/sysroot.rs`.
- Product/artifact alias persistence is ID-backed only; old string-only alias contracts are rejected.
- `Lowerer` has no semantic string alias map fields for imports, module-local aliases, artifact root exports, or export-function aliases.
- Collection/lowering/product/artifact boundaries pass aliases by resolver IDs or `ArtifactExport` IDs, not string-to-string maps.
- Final code review reports no Critical or Important findings.
- `cargo test -p rock-lib` passes.
- `cargo fmt --all --check` passes.
- `git diff --check` passes.
- Roadmap and audit docs mark Task 4 complete and keep broader Task 5/later work scoped correctly.

## Final Verification

- Audit grep command:

```bash
rg "HashMap<String, String>|prelude_exports: BTreeMap<String, String>|root_exports: BTreeMap<String, String>|import_aliases: HashMap<String, String>|module_local_aliases|artifact_root_exports|export_function_aliases|record_prelude_exports|PRODUCT_ARTIFACT_FORMAT_VERSION" lib/src rock-shared/src docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
```

- Audit grep classification: remaining `HashMap<String, String>` hits are collection-local syntax/display maps, inference metadata, monomorphization display/function-alias metadata, or transient lowerer compatibility views derived from ID-backed resolver tables; no `Lowerer` stored semantic alias fields or product/artifact persistent string alias maps remain. `PRODUCT_ARTIFACT_FORMAT_VERSION` hits are `22` in `rock-shared/src/sysroot.rs` and `lib/src/products.rs`.
- Focused alias tests run after the final review fixes in `53572b1`: `cargo test -p rock-lib alias` passed with 71 unit tests and 2 integration tests matched; `cargo test -p rock-lib prelude_export` passed with 12 unit tests matched; `cargo test -p rock-lib product_artifact` passed with 79 unit tests matched.
- Full verification run after the final review fixes in `53572b1`: `cargo test -p rock-lib` passed with 1321 unit tests, 277 integration tests, 1 test binary test, and doctests passing. Initial Task 7 verification found formatting needed in `lib/src/lower/pipeline.rs`; follow-up `cargo fmt --all` applied rustfmt-only changes, including the pre-existing formatting-only diff in `lib/src/collect/resolver.rs`; final `cargo fmt --all --check` passed; final `git diff --check` passed.
- Zero-test filters and replacements: none; each requested focused filter matched tests.
- Review request notes: a final controller review found module-local type aliases could lose precedence to root item paths in type annotations; `b071294` fixed struct, enum, and trait type lookup to check scoped module aliases before global item lookup. A follow-up review found stale artifact import/module aliases were rejected instead of dropped and dependency module aliases were omitted from external generic-function alias propagation; `53572b1` fixed both with red/green regressions. A final follow-up review should verify Task 4 persistent alias interfaces are ID-backed and complete; product artifact format is `22` and old string-only artifacts are not accepted; `Lowerer` no longer owns semantic string alias maps; Task 5 completion is limited to the strict alias/path-resolution subset; broader Task 5/lowerer decomposition, Task 11, Task 13, Task 21, and formatter work remain open.
