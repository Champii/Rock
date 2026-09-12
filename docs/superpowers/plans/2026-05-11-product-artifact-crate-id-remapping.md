# Product Artifact Crate ID Remapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decode product-artifact-local crate IDs through a consumer-session crate identity map so loaded artifacts expose collision-free canonical `DefId`s.

**Architecture:** Product artifacts keep producer-local `ProductCrateId` values on disk. `CrateContext` owns a session map from stable `ProductCrateIdentity` to consumer `CrateId`, and artifact loading builds a per-artifact `ProductIdentityRemap` before constructing `LoadedCrate` metadata, resolver tables, and cross-crate HIR. Backend object symbols are preserved as ABI names and are not rewritten by identity remapping.

**Tech Stack:** Rust 2021, `rock-lib`, `rock-shared`, product artifacts, `CrateContext`, `CompilerProducts`, `ResolverTables`, focused `cargo test -p rock-lib` commands.

---

## Spec And Scope

Read first:

- `docs/superpowers/specs/2026-05-11-canonical-identity-completion-design.md`
- Phase 1 only: `Product Artifact CrateId Remapping`

This plan does not implement later canonical identity phases. The next phases remain in the spec:

- canonical dependency/prelude/artifact resolution
- authoritative ID-keyed HIR consumers
- first-class child definition identity
- monomorphization instance identity cleanup
- semantic type identity

## File Map

- Modify: `lib/src/products.rs`
  - Make `ProductCrateIdentity` usable as a session identity-map key.
  - Change `ProductIdentityTable.dependencies` from an ordered list without IDs to a map from producer-local `ProductCrateId` to `ProductCrateIdentity`.
  - Bump `PRODUCT_ARTIFACT_FORMAT_VERSION` because the serialized product identity table changes shape.
  - Update product tests that inspect dependency identity metadata.
- Modify: `rock-shared/src/sysroot.rs`
  - Bump the shared `PRODUCT_ARTIFACT_FORMAT_VERSION` to match `rock-lib`.
- Modify: `lib/src/crate_system/mod.rs`
  - Add the consumer-session product crate identity table and next dependency crate counter to `CrateContext`.
- Modify: `lib/src/crate_system/context.rs`
  - Initialize and maintain the consumer-session product crate identity map.
- Modify: `lib/src/crate_artifact/load.rs`
  - Build `ProductIdentityRemap` during artifact load.
  - Remap product IDs before constructing loaded interface, resolver, and cross-crate HIR data.
  - Add artifact-loader regressions for crate ID collision, shared transitive identity reuse, nested HIR remapping, and unresolved product crate IDs.
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
  - Mark this artifact identity sub-slice as done after tests pass.

---

### Task 1: Product Identity Table Schema

**Files:**
- Modify: `lib/src/products.rs:43-90`
- Modify: `lib/src/products.rs:187-201`
- Modify: `lib/src/products.rs:1067-1091`
- Modify: `lib/src/products.rs:1172-1178`
- Modify: `rock-shared/src/sysroot.rs:20`

- [ ] **Step 1: Add the failing dependency crate-ID metadata test**

In `lib/src/products.rs`, replace `compiler_products_record_dependency_identities_in_identity_table` with:

```rust
#[test]
fn compiler_products_record_dependency_identities_by_product_crate_id() {
    let hir = resolved_hir_for_products();
    let stdlib = ProductDependencyIdentity {
        name: "stdlib".to_string(),
        artifact_path: PathBuf::from("build/stdlib.rkca"),
    };
    let math = ProductDependencyIdentity {
        name: "math".to_string(),
        artifact_path: PathBuf::from("build/math.rkca"),
    };

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        vec![stdlib, math],
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );

    assert_eq!(products.dependencies.len(), 2);
    assert_eq!(products.identity_table.local_crate, Some(ProductCrateId(0)));
    assert_eq!(
        products
            .identity_table
            .dependencies
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![ProductCrateId(1), ProductCrateId(2)]
    );
    assert_eq!(
        products.identity_table.dependencies[&ProductCrateId(1)].name,
        "stdlib"
    );
    assert_eq!(
        products.identity_table.dependencies[&ProductCrateId(2)].name,
        "math"
    );
}
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run:

```bash
cargo test -p rock-lib compiler_products_record_dependency_identities_by_product_crate_id -- --exact
```

Expected: FAIL to compile because `identity_table.dependencies` is still `Vec<ProductCrateIdentity>` and has no `keys()` method.

- [ ] **Step 3: Make product crate identities map-addressable**

In `lib/src/products.rs`, replace the `PRODUCT_ARTIFACT_FORMAT_VERSION`, `ProductCrateIdentity`, and `ProductIdentityTable` definitions with:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProductCrateIdentity {
    pub name: String,
    pub version: String,
    pub target_triple: Option<String>,
    pub format_version: u32,
}

impl ProductCrateIdentity {
    pub fn local(name: String) -> Self {
        Self {
            name,
            version: "0.1.0".to_string(),
            target_triple: None,
            format_version: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductIdentityTable {
    pub local_crate: Option<ProductCrateId>,
    pub dependencies: BTreeMap<ProductCrateId, ProductCrateIdentity>,
    pub display_names: BTreeMap<ProductDefId, String>,
    pub export_names: BTreeMap<String, ProductDefId>,
    pub backend_symbols: BTreeMap<ProductDefId, String>,
}
```

In `rock-shared/src/sysroot.rs`, change the shared format version to:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 5;
```

- [ ] **Step 4: Fill dependency identities by producer-local crate ID**

Add this helper near `record_name` in `lib/src/products.rs`:

```rust
fn dependency_crate_identities(
    dependencies: &[ProductDependencyIdentity],
    local_crate: ProductCrateId,
) -> BTreeMap<ProductCrateId, ProductCrateIdentity> {
    let mut identities = BTreeMap::new();
    let mut next_raw = 0u32;

    for dependency in dependencies {
        while ProductCrateId(next_raw) == local_crate
            || identities.contains_key(&ProductCrateId(next_raw))
        {
            next_raw = next_raw
                .checked_add(1)
                .expect("product dependency crate ID generator exhausted u32 ID space");
        }

        identities.insert(
            ProductCrateId(next_raw),
            ProductCrateIdentity::local(dependency.name.clone()),
        );
        next_raw = next_raw
            .checked_add(1)
            .expect("product dependency crate ID generator exhausted u32 ID space");
    }

    identities
}
```

In `CompilerProducts::from_resolved_hir`, replace the `identity_table` initializer with:

```rust
let local_crate = ProductCrateId::from(hir.root_crate_id);
let mut identity_table = ProductIdentityTable {
    local_crate: Some(local_crate),
    dependencies: dependency_crate_identities(&dependencies, local_crate),
    ..ProductIdentityTable::default()
};
```

- [ ] **Step 5: Update format-version and dependency metadata tests**

In `product_artifact_format_version_matches_shared_contract`, update the literal assertion:

```rust
assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 5);
```

Keep tests that inspect `products.dependencies[0]` unchanged; `CompilerProducts.dependencies` remains the external artifact path list.

- [ ] **Step 6: Run focused product schema tests**

Run:

```bash
cargo test -p rock-lib compiler_products_record_dependency_identities_by_product_crate_id -- --exact
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --exact
cargo test -p rock-lib compiler_products_write_and_read_product_artifact_bytes -- --exact
```

Expected: PASS for all three commands.

- [ ] **Step 7: Commit product schema change**

```bash
git add lib/src/products.rs rock-shared/src/sysroot.rs
git commit -m "products: record dependency crate identities by id"
```

---

### Task 2: Artifact Loader Red Tests

**Files:**
- Modify: `lib/src/crate_artifact/load.rs:1-10`
- Modify: `lib/src/crate_artifact/load.rs:378-868`

- [ ] **Step 1: Extend product-loader test imports**

In `lib/src/crate_artifact/load.rs`, update the test module's collection import:

```rust
use std::collections::{BTreeMap, HashMap};
```

In `lib/src/crate_artifact/load.rs`, update the test imports from `crate::products` to include the product ID and body/link helpers used by these tests:

```rust
use crate::products::{
    CompilerProducts, ProductBodies, ProductCrateId, ProductCrateIdentity, ProductDefId,
    ProductIdentityTable, ProductLinkData, ProductLinkRecord, ProductLocalDefId,
    ProductMetadata, ProductSourceFingerprint,
};
```

- [ ] **Step 2: Add product-artifact fixture helpers**

Add these helpers after `test_function` in `lib/src/crate_artifact/load.rs`:

```rust
fn product_with_function(
    crate_name: &str,
    product_crate_id: ProductCrateId,
    local_id: u32,
    object_path: PathBuf,
) -> CompilerProducts {
    let product_id = ProductDefId {
        crate_id: product_crate_id,
        local_id: ProductLocalDefId(local_id),
    };
    let function_id = DefId::new(CrateId(product_crate_id.0), LocalDefId(local_id));
    let function_name = format!("{}::answer", crate_name);
    let function = test_function(function_id, &function_name);

    let mut identity_table = ProductIdentityTable::default();
    identity_table.local_crate = Some(product_crate_id);
    identity_table
        .display_names
        .insert(product_id, function_name.clone());
    identity_table
        .export_names
        .insert("answer".to_string(), product_id);

    let mut metadata = ProductMetadata::default();
    metadata.functions.insert(product_id, function.clone());

    let mut bodies = ProductBodies::default();
    bodies.functions.insert(product_id, function);

    let mut link_records = BTreeMap::new();
    link_records.insert(
        product_id,
        ProductLinkRecord {
            backend_symbol: format!("{}_answer", crate_name),
        },
    );

    CompilerProducts {
        crate_identity: ProductCrateIdentity::local(crate_name.to_string()),
        identity_table,
        metadata,
        bodies,
        link: ProductLinkData {
            object_path: Some(object_path),
            records: link_records,
        },
        dependencies: Vec::new(),
        source_fingerprint: ProductSourceFingerprint::default(),
        prelude_exports: Default::default(),
        infix_precedence: Default::default(),
    }
}

fn write_product_fixture(base: &std::path::Path, crate_name: &str) -> PathBuf {
    let artifact_path = base.join(format!("{}.rkca", crate_name));
    let object_path = base.join(format!("{}.o", crate_name));
    fs::write(&object_path, []).unwrap();

    let products = product_with_function(crate_name, ProductCrateId(0), 0, object_path);
    products.write_artifact_to_path(&artifact_path).unwrap();

    artifact_path
}
```

- [ ] **Step 3: Add the same-local-ID collision regression**

Add this test after the helper functions:

```rust
#[test]
fn load_product_artifacts_remap_same_local_crate_ids_to_distinct_consumer_crates() {
    let base = std::env::temp_dir().join(format!(
        "rock_product_loader_{}_{}",
        std::process::id(),
        "distinct_consumer_crates"
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let a_artifact = write_product_fixture(&base, "a");
    let b_artifact = write_product_fixture(&base, "b");

    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path(a_artifact).unwrap();
    ctx.load_product_artifact_from_path(b_artifact).unwrap();

    let a_id = ctx.crates["a"].resolver.item_paths["a::answer"];
    let b_id = ctx.crates["b"].resolver.item_paths["b::answer"];

    assert_ne!(a_id.crate_id, b_id.crate_id);
    assert_eq!(a_id.local, LocalDefId(0));
    assert_eq!(b_id.local, LocalDefId(0));

    let _ = fs::remove_dir_all(base);
}
```

- [ ] **Step 4: Add the shared transitive dependency identity regression**

Add this test after the same-local-ID test:

```rust
#[test]
fn product_crate_id_remap_reuses_shared_transitive_dependency_identity() {
    let shared = ProductCrateIdentity::local("shared".to_string());
    let mut first = product_with_function(
        "first",
        ProductCrateId(0),
        0,
        PathBuf::from("first.o"),
    );
    first
        .identity_table
        .dependencies
        .insert(ProductCrateId(1), shared.clone());

    let mut second = product_with_function(
        "second",
        ProductCrateId(0),
        0,
        PathBuf::from("second.o"),
    );
    second
        .identity_table
        .dependencies
        .insert(ProductCrateId(1), shared);

    let mut ctx = CrateContext::new();
    let first_remap = super::ProductIdentityRemap::from_products(&mut ctx, &first).unwrap();
    let second_remap = super::ProductIdentityRemap::from_products(&mut ctx, &second).unwrap();

    assert_ne!(
        first_remap.crate_ids[&ProductCrateId(0)],
        second_remap.crate_ids[&ProductCrateId(0)]
    );
    assert_eq!(
        first_remap.crate_ids[&ProductCrateId(1)],
        second_remap.crate_ids[&ProductCrateId(1)]
    );
}
```

- [ ] **Step 5: Add the generic body ID remapping regression**

Add this test after the shared transitive test:

```rust
#[test]
fn load_product_artifact_remaps_generic_body_ids_with_interface_ids() {
    let base = std::env::temp_dir().join(format!(
        "rock_product_loader_{}_{}",
        std::process::id(),
        "generic_body_ids"
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let artifact_path = write_product_fixture(&base, "dep");

    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path(artifact_path).unwrap();

    let loaded = ctx.crates.get("dep").unwrap();
    let interface_id = loaded.interface.as_ref().unwrap().functions["dep::answer"].id;
    let body_id = loaded
        .cross_crate_hir
        .as_ref()
        .unwrap()
        .generic_functions["dep::answer"]
        .id;

    assert_eq!(interface_id, body_id);
    assert_ne!(body_id.crate_id, CrateId(0));

    let _ = fs::remove_dir_all(base);
}
```

- [ ] **Step 6: Run the red loader tests**

Run:

```bash
cargo test -p rock-lib load_product_artifacts_remap_same_local_crate_ids_to_distinct_consumer_crates -- --exact
cargo test -p rock-lib product_crate_id_remap_reuses_shared_transitive_dependency_identity -- --exact
cargo test -p rock-lib load_product_artifact_remaps_generic_body_ids_with_interface_ids -- --exact
```

Expected:

- First command FAILS because both loaded artifacts still decode `ProductCrateId(0)` as `CrateId(0)`.
- Second command FAILS to compile because `ProductIdentityRemap` does not exist yet.
- Third command FAILS because generic body IDs still use producer-local crate IDs.

Do not change implementation or commit in this task. Continue directly to Task 3 and Task 4 so the branch is not left with committed failing tests.

---

### Task 3: Consumer Session Crate Identity Map

**Files:**
- Modify: `lib/src/crate_system/mod.rs:10-18`
- Modify: `lib/src/crate_system/mod.rs:180-185`
- Modify: `lib/src/crate_system/context.rs:24-31`
- Modify: `lib/src/crate_artifact/load.rs:1-10`
- Modify: `lib/src/crate_artifact/load.rs:12-45`

- [ ] **Step 1: Add `CrateContext` identity-map storage**

In `lib/src/crate_system/mod.rs`, add this import:

```rust
use crate::products::ProductCrateIdentity;
```

Replace `CrateContext` with:

```rust
#[derive(Debug)]
pub struct CrateContext {
    /// Map of crate name to its loaded crate data
    pub crates: BTreeMap<String, LoadedCrate>,
    pub(crate) product_crate_ids: BTreeMap<ProductCrateIdentity, crate::ids::CrateId>,
    next_product_crate_id: u32,
}
```

- [ ] **Step 2: Initialize and allocate consumer crate IDs**

In `lib/src/crate_system/context.rs`, update `CrateContext::new` to:

```rust
pub fn new() -> Self {
    Self {
        crates: BTreeMap::new(),
        product_crate_ids: BTreeMap::new(),
        next_product_crate_id: 1,
    }
}
```

Add this method inside `impl CrateContext` after `new`:

```rust
pub(crate) fn consumer_crate_id_for_product_identity(
    &mut self,
    identity: &crate::products::ProductCrateIdentity,
) -> crate::ids::CrateId {
    if let Some(crate_id) = self.product_crate_ids.get(identity).copied() {
        return crate_id;
    }

    let crate_id = crate::ids::CrateId(self.next_product_crate_id);
    self.next_product_crate_id = self
        .next_product_crate_id
        .checked_add(1)
        .expect("consumer product crate ID generator exhausted u32 ID space");
    self.product_crate_ids.insert(identity.clone(), crate_id);
    crate_id
}
```

- [ ] **Step 3: Add `ProductIdentityRemap`**

In `lib/src/crate_artifact/load.rs`, update imports:

```rust
use crate::products::{CompilerProducts, ProductCrateId, ProductDefId};
```

Add this struct and implementation above `impl CrateContext`:

```rust
#[derive(Debug, Clone)]
struct ProductIdentityRemap {
    crate_ids: BTreeMap<ProductCrateId, CrateId>,
}

impl ProductIdentityRemap {
    fn from_products(ctx: &mut CrateContext, products: &CompilerProducts) -> Result<Self, String> {
        let local_crate = products.identity_table.local_crate.ok_or_else(|| {
            format!(
                "Product artifact for crate '{}' has no local product crate ID",
                products.crate_identity.name
            )
        })?;

        let mut crate_ids = BTreeMap::new();
        crate_ids.insert(
            local_crate,
            ctx.consumer_crate_id_for_product_identity(&products.crate_identity),
        );

        for (product_crate_id, identity) in &products.identity_table.dependencies {
            if *product_crate_id == local_crate {
                return Err(format!(
                    "Product artifact for crate '{}' maps dependency '{}' to local product crate ID {}",
                    products.crate_identity.name,
                    identity.name,
                    product_crate_id.0
                ));
            }

            crate_ids.insert(
                *product_crate_id,
                ctx.consumer_crate_id_for_product_identity(identity),
            );
        }

        Ok(Self { crate_ids })
    }

    fn def_id(&self, id: ProductDefId) -> Result<DefId, String> {
        let Some(crate_id) = self.crate_ids.get(&id.crate_id).copied() else {
            return Err(format!(
                "Product artifact references unmapped product crate ID {}",
                id.crate_id.0
            ));
        };

        Ok(DefId::new(crate_id, LocalDefId(id.local_id.0)))
    }
}
```

- [ ] **Step 4: Build the remap during artifact load**

In both `load_product_artifact_from_path` and `load_product_artifact_from_path_as`, insert remap construction before `loaded_crate_from_products`:

```rust
let remap = ProductIdentityRemap::from_products(self, &products)?;
let loaded_crate = loaded_crate_from_products(products, artifact_path, &remap)?;
```

Change the `loaded_crate_from_products` signature to:

```rust
fn loaded_crate_from_products(
    products: CompilerProducts,
    artifact_path: PathBuf,
    remap: &ProductIdentityRemap,
) -> Result<LoadedCrate, String> {
```

- [ ] **Step 5: Run the shared transitive remap test**

Run:

```bash
cargo test -p rock-lib product_crate_id_remap_reuses_shared_transitive_dependency_identity -- --exact
```

Expected: PASS. The other loader tests still fail until artifact metadata and HIR construction use the remap.

- [ ] **Step 6: Continue without committing**

Do not commit yet. The session crate identity map is only partially wired at this point, and the Task 2 loader tests are intentionally not all green until Task 4 applies the remap to loaded artifact data.

---

### Task 4: Apply Remapping To Loaded Artifact Data

**Files:**
- Modify: `lib/src/crate_artifact/load.rs:48-376`
- Modify: `lib/src/crate_artifact/load.rs:420-868`

- [ ] **Step 1: Make artifact conversion functions return `Result`**

Update calls inside `loaded_crate_from_products`:

```rust
let interface = interface_from_products(&products, &crate_name, remap)?;
let resolver = resolver_from_products(&products, remap)?;
let cross_crate_hir = cross_crate_hir_from_products(&products, &crate_name, remap)?;
```

Use `cross_crate_hir` in the `LoadedCrate` initializer:

```rust
cross_crate_hir: Some(cross_crate_hir),
```

- [ ] **Step 2: Add HIR remap helpers**

Add these helpers above `interface_from_products`:

```rust
fn remap_function_id(
    mut function: crate::hir::HirFunction,
    id: ProductDefId,
    remap: &ProductIdentityRemap,
) -> Result<crate::hir::HirFunction, String> {
    function.id = remap.def_id(id)?;
    Ok(function)
}

fn remap_struct_id(
    mut value: crate::hir::HirStruct,
    id: ProductDefId,
    remap: &ProductIdentityRemap,
) -> Result<crate::hir::HirStruct, String> {
    value.id = remap.def_id(id)?;
    Ok(value)
}

fn remap_enum_id(
    mut value: crate::hir::HirEnum,
    id: ProductDefId,
    remap: &ProductIdentityRemap,
) -> Result<crate::hir::HirEnum, String> {
    value.id = remap.def_id(id)?;
    Ok(value)
}

fn remap_trait_ids(
    mut value: crate::hir::HirTrait,
    id: ProductDefId,
    remap: &ProductIdentityRemap,
) -> Result<crate::hir::HirTrait, String> {
    value.id = remap.def_id(id)?;
    for method in value.methods.values_mut() {
        method.id = remap.def_id(ProductDefId::from(method.id))?;
    }
    Ok(value)
}

fn remap_impl_ids(
    mut value: crate::hir::HirImpl,
    id: ProductDefId,
    remap: &ProductIdentityRemap,
) -> Result<crate::hir::HirImpl, String> {
    value.id = remap.def_id(id)?;
    for method in value.methods.values_mut() {
        method.id = remap.def_id(ProductDefId::from(method.id))?;
    }
    Ok(value)
}

fn remap_extern_id(
    mut value: crate::hir::HirExtern,
    id: ProductDefId,
    remap: &ProductIdentityRemap,
) -> Result<crate::hir::HirExtern, String> {
    value.id = remap.def_id(id)?;
    Ok(value)
}
```

- [ ] **Step 3: Remap interface metadata**

Replace `interface_from_products` with this shape:

```rust
fn interface_from_products(
    products: &CompilerProducts,
    crate_name: &str,
    remap: &ProductIdentityRemap,
) -> Result<super::ArtifactCrateInterface, String> {
    let mut root_exports = BTreeMap::new();
    for (alias, id) in &products.identity_table.export_names {
        if let Some(display_name) = product_display_name(products, *id) {
            root_exports.insert(alias.clone(), qualify_product_name(crate_name, &display_name));
        }
    }

    let functions = products
        .metadata
        .functions
        .iter()
        .filter_map(|(id, function)| {
            product_display_name(products, *id)
                .map(|name| (*id, name, function.clone()))
        })
        .map(|(id, name, function)| {
            remap_function_id(function, id, remap)
                .map(|function| (qualify_product_name(crate_name, &name), function))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let structs = products
        .metadata
        .structs
        .iter()
        .filter_map(|(id, value)| product_display_name(products, *id).map(|name| (*id, name, value.clone())))
        .map(|(id, name, value)| {
            remap_struct_id(value, id, remap)
                .map(|value| (qualify_product_name(crate_name, &name), value))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let enums = products
        .metadata
        .enums
        .iter()
        .filter_map(|(id, value)| product_display_name(products, *id).map(|name| (*id, name, value.clone())))
        .map(|(id, name, value)| {
            remap_enum_id(value, id, remap)
                .map(|value| (qualify_product_name(crate_name, &name), value))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let traits = products
        .metadata
        .traits
        .iter()
        .filter_map(|(id, value)| product_display_name(products, *id).map(|name| (*id, name, value.clone())))
        .map(|(id, name, value)| {
            remap_trait_ids(value, id, remap)
                .map(|value| (qualify_product_name(crate_name, &name), value))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let impls = products
        .metadata
        .impls
        .iter()
        .map(|(id, value)| remap_impl_ids(value.clone(), *id, remap))
        .collect::<Result<Vec<_>, _>>()?;

    let externs = products
        .metadata
        .externs
        .iter()
        .map(|(id, value)| remap_extern_id(value.clone(), *id, remap))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(super::ArtifactCrateInterface {
        root_exports,
        functions,
        structs,
        enums,
        traits,
        impls,
        externs,
        infix_precedence: products.infix_precedence.clone(),
    })
}
```

- [ ] **Step 4: Remap resolver IDs**

Replace `resolver_from_products` and `product_def_id_to_existing_def_id` with:

```rust
fn resolver_from_products(
    products: &CompilerProducts,
    remap: &ProductIdentityRemap,
) -> Result<ResolverTables, String> {
    let mut resolver = ResolverTables::default();
    for (id, name) in &products.identity_table.display_names {
        if let Some(def_id) = product_def_id_to_existing_def_id(products, remap, *id)? {
            resolver.item_paths.insert(name.clone(), def_id);
            resolver.item_names_by_id.insert(def_id, name.clone());
        }
    }
    for (alias, id) in &products.identity_table.export_names {
        if let Some(def_id) = product_def_id_to_existing_def_id(products, remap, *id)? {
            resolver.export_aliases.insert(alias.clone(), def_id);
        }
    }

    Ok(resolver)
}

fn product_def_id_to_existing_def_id(
    products: &CompilerProducts,
    remap: &ProductIdentityRemap,
    id: ProductDefId,
) -> Result<Option<DefId>, String> {
    if products.metadata.functions.contains_key(&id)
        || products.metadata.structs.contains_key(&id)
        || products.metadata.enums.contains_key(&id)
        || products.metadata.traits.contains_key(&id)
        || products.metadata.externs.contains_key(&id)
    {
        Ok(Some(remap.def_id(id)?))
    } else {
        Ok(None)
    }
}
```

Delete `product_def_id_to_def_id` from `lib/src/crate_artifact/load.rs`; product ID decoding now goes through `ProductIdentityRemap::def_id`.

- [ ] **Step 5: Remap cross-crate HIR bodies**

Replace `cross_crate_hir_from_products` with:

```rust
fn cross_crate_hir_from_products(
    products: &CompilerProducts,
    crate_name: &str,
    remap: &ProductIdentityRemap,
) -> Result<super::ArtifactCrossCrateHir, String> {
    let generic_functions = products
        .bodies
        .functions
        .iter()
        .filter_map(|(id, function)| {
            product_display_name(products, *id)
                .map(|name| (*id, name, function.clone()))
        })
        .map(|(id, name, function)| {
            remap_function_id(function, id, remap)
                .map(|function| (qualify_product_name(crate_name, &name), function))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let traits_with_defaults = products
        .metadata
        .traits
        .iter()
        .filter_map(|(id, trait_def)| {
            if trait_def.methods.is_empty() {
                None
            } else {
                product_display_name(products, *id)
                    .map(|name| (*id, name, trait_def.clone()))
            }
        })
        .map(|(id, name, trait_def)| {
            remap_trait_ids(trait_def, id, remap)
                .map(|trait_def| (qualify_product_name(crate_name, &name), trait_def))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let generic_impls = products
        .bodies
        .generic_impls
        .iter()
        .map(|(id, imp)| remap_impl_ids(imp.clone(), *id, remap))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(super::ArtifactCrossCrateHir {
        generic_functions,
        traits_with_defaults,
        generic_impls,
    })
}
```

- [ ] **Step 6: Run remapping tests**

Run:

```bash
cargo test -p rock-lib load_product_artifacts_remap_same_local_crate_ids_to_distinct_consumer_crates -- --exact
cargo test -p rock-lib product_crate_id_remap_reuses_shared_transitive_dependency_identity -- --exact
cargo test -p rock-lib load_product_artifact_remaps_generic_body_ids_with_interface_ids -- --exact
cargo test -p rock-lib resolver_from_products_ignores_impl_display_names -- --exact
cargo test -p rock-lib load_product_artifact_qualifies_generic_body_names -- --exact
```

Expected: PASS for all five commands.

- [ ] **Step 7: Commit loader tests and loaded artifact remapping**

```bash
git add lib/src/crate_system/mod.rs lib/src/crate_system/context.rs lib/src/crate_artifact/load.rs
git commit -m "crate-artifact: remap product ids on load"
```

---

### Task 5: Artifact Error Case, Audit Update, And Verification

**Files:**
- Modify: `lib/src/crate_artifact/load.rs:420-868`
- Modify: `docs/superpowers/plans/master-audit-checklist.md:196-206`

- [ ] **Step 1: Add unresolved product crate ID regression**

Add this test near the other product loader remapping tests:

```rust
#[test]
fn load_product_artifact_rejects_unmapped_product_crate_id() {
    let base = std::env::temp_dir().join(format!(
        "rock_product_loader_{}_{}",
        std::process::id(),
        "unmapped_product_crate"
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let artifact_path = base.join("dep.rkca");
    let object_path = base.join("dep.o");
    fs::write(&object_path, []).unwrap();

    let mut products = product_with_function("dep", ProductCrateId(0), 0, object_path);
    let foreign_id = ProductDefId {
        crate_id: ProductCrateId(9),
        local_id: ProductLocalDefId(0),
    };
    products
        .identity_table
        .display_names
        .insert(foreign_id, "foreign::answer".to_string());
    products
        .metadata
        .functions
        .insert(foreign_id, test_function(DefId::new(CrateId(9), LocalDefId(0)), "foreign::answer"));
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    let err = ctx.load_product_artifact_from_path(artifact_path).unwrap_err();

    assert!(err.contains("unmapped product crate ID 9"));

    let _ = fs::remove_dir_all(base);
}
```

- [ ] **Step 2: Run the unresolved-ID test**

Run:

```bash
cargo test -p rock-lib load_product_artifact_rejects_unmapped_product_crate_id -- --exact
```

Expected: PASS.

- [ ] **Step 3: Update the master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, under `## 8. Crate And Artifact Interface Split` `Done:`, add this checked item after the provider-capability lines:

```markdown
- [x] Decode product artifact crate IDs through a consumer-session crate identity map so artifacts do not leak producer-local crate numbers.
```

Do not mark later canonical identity phases done in this task.

- [ ] **Step 4: Run focused artifact remapping verification**

Run:

```bash
cargo test -p rock-lib load_product_artifacts_remap_same_local_crate_ids_to_distinct_consumer_crates -- --exact
cargo test -p rock-lib product_crate_id_remap_reuses_shared_transitive_dependency_identity -- --exact
cargo test -p rock-lib load_product_artifact_remaps_generic_body_ids_with_interface_ids -- --exact
cargo test -p rock-lib load_product_artifact_rejects_unmapped_product_crate_id -- --exact
cargo test -p rock-lib compiler_products_write_and_read_product_artifact_bytes -- --exact
```

Expected: PASS for all five commands.

- [ ] **Step 5: Run final Phase 1 verification**

Run:

```bash
cargo fmt --all --check
cargo test -p rock-lib
```

Expected: PASS for both commands.

- [ ] **Step 6: Commit audit and final remapping regression**

```bash
git add lib/src/crate_artifact/load.rs docs/superpowers/plans/master-audit-checklist.md
git commit -m "test: reject unmapped product crate ids"
```

---

## Plan Self-Review Notes

- Spec coverage: This plan implements Phase 1 from `2026-05-11-canonical-identity-completion-design.md` and deliberately does not claim later phases.
- Artifact-local crate IDs: Task 3 decodes producer `ProductCrateId` through a `ProductCrateIdentity -> CrateId` session table.
- Shared transitive identity: Task 2 and Task 3 include a regression that the same transitive `ProductCrateIdentity` reuses one consumer `CrateId`.
- Distinct producer-local IDs: Task 2 and Task 4 include a regression that two artifacts with `ProductCrateId(0), local 0` do not collide.
- Nested IDs: Task 4 remaps function, struct, enum, trait, impl, method, extern, resolver, and cross-crate HIR IDs.
- Backend symbols: The plan remaps semantic IDs only and leaves `ProductLinkRecord.backend_symbol` unchanged.
- Later work: The remaining canonical identity completion phases stay in the approved spec and need separate implementation plans after Phase 1 lands.
