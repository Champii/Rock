# Compiler Products Identity Spine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first product-backed artifact foundation: canonical product IDs and `CompilerProducts` extraction from the normal HIR pipeline, without changing existing artifact consumption.

**Architecture:** Introduce a new `lib/src/products.rs` module owned by `rock-lib`. It mirrors existing `DefId` identity into serializable product IDs, extracts ID-keyed metadata/body/link product tables from `infer::ResolvedHirProgram`, and exposes `compile_with_products` beside the existing `compile` API so tests can prove products come from the normal pipeline. Existing `.rkca` building and loading remain unchanged in this slice.

**Tech Stack:** Rust 2021, serde/bincode-compatible product types, existing `DefId`/HIR types, `cargo test -p rock-lib products`, `cargo test -p rock-lib compile_with_products`.

---

## Scope Check

The approved design covers multiple independent migration phases: compiler products, `rockc` artifact/object emission, `rock` subprocess orchestration, artifact builder removal, and source-bundle fallback removal. This plan implements only the first independently testable slice.

This slice builds:
- canonical product ID types at the artifact/product boundary
- `CompilerProducts` and supporting product sections
- extraction from finalized HIR produced by the normal pipeline
- an opt-in `compile_with_products` API for tests and future `rockc` emission work

This slice does not build:
- new `.rkca` serialization format
- `rockc --emit-artifact`
- `rock` subprocess invocation
- removal of `CrateContext::build_artifact`
- removal of `source_bundle` or `cross_crate_hir`

## File Structure

- Create: `lib/src/products.rs`
- Responsibility: product ID types, `CompilerProducts` data model, HIR product extraction, unit tests for identity and section extraction.
- Modify: `lib/src/lib.rs`
- Responsibility: publish `products`, add `CompileOutput`, add `compile_with_products`, and keep existing `compile` behavior unchanged.
- Modify: `lib/src/hir/mod.rs`
- Responsibility: add `id: DefId` to `HirExtern` so extern metadata can have a canonical product ID.
- Modify: `lib/src/collect/headers.rs`
- Responsibility: populate `HirExtern.id` during declaration collection.
- Modify: `lib/src/lower/function.rs`
- Responsibility: populate `HirExtern.id` during direct lowering fallback paths.
- Modify: `lib/src/lower/mod.rs`
- Responsibility: resolve collected extern temporary IDs to canonical resolver IDs before producing `PartialHir`.
- Modify: `lib/src/lower/collect/declarations.rs`
- Responsibility: populate `HirExtern.id` for crate/module lowering paths.

---

### Task 1: Add Product ID Types

**Files:**
- Create: `lib/src/products.rs`
- Modify: `lib/src/lib.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Add failing tests for product ID conversion and ordering**

Create `lib/src/products.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::products::{ProductCrateId, ProductDefId, ProductLocalDefId};

    #[test]
    fn product_def_id_preserves_def_id_raw_parts() {
        let def_id = DefId::new(CrateId(7), LocalDefId(42));

        let product_id = ProductDefId::from(def_id);

        assert_eq!(product_id.crate_id, ProductCrateId(7));
        assert_eq!(product_id.local_id, ProductLocalDefId(42));
    }

    #[test]
    fn product_def_id_is_usable_as_stable_btree_key() {
        let first = ProductDefId {
            crate_id: ProductCrateId(0),
            local_id: ProductLocalDefId(2),
        };
        let second = ProductDefId {
            crate_id: ProductCrateId(0),
            local_id: ProductLocalDefId(1),
        };

        let mut map = BTreeMap::new();
        map.insert(first, "second item".to_string());
        map.insert(second, "first item".to_string());

        let keys = map.keys().copied().collect::<Vec<_>>();
        assert_eq!(keys, vec![second, first]);
    }
}
```

Expose the module in `lib/src/lib.rs` near the other public modules:

```rust
pub mod products;
```

- [ ] **Step 2: Run the failing product ID tests**

Run: `cargo test -p rock-lib products::tests::product_def_id -- --nocapture`

Expected: FAIL to compile with unresolved imports for `ProductCrateId`, `ProductLocalDefId`, and `ProductDefId`.

- [ ] **Step 3: Implement the product ID types**

Replace the contents of `lib/src/products.rs` with:

```rust
use serde::{Deserialize, Serialize};

use crate::ids::{CrateId, DefId, Idx, LocalDefId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProductCrateId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProductLocalDefId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProductDefId {
    pub crate_id: ProductCrateId,
    pub local_id: ProductLocalDefId,
}

impl From<CrateId> for ProductCrateId {
    fn from(value: CrateId) -> Self {
        Self(value.raw())
    }
}

impl From<LocalDefId> for ProductLocalDefId {
    fn from(value: LocalDefId) -> Self {
        Self(value.raw())
    }
}

impl From<DefId> for ProductDefId {
    fn from(value: DefId) -> Self {
        Self {
            crate_id: ProductCrateId::from(value.crate_id),
            local_id: ProductLocalDefId::from(value.local),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::products::{ProductCrateId, ProductDefId, ProductLocalDefId};

    #[test]
    fn product_def_id_preserves_def_id_raw_parts() {
        let def_id = DefId::new(CrateId(7), LocalDefId(42));

        let product_id = ProductDefId::from(def_id);

        assert_eq!(product_id.crate_id, ProductCrateId(7));
        assert_eq!(product_id.local_id, ProductLocalDefId(42));
    }

    #[test]
    fn product_def_id_is_usable_as_stable_btree_key() {
        let first = ProductDefId {
            crate_id: ProductCrateId(0),
            local_id: ProductLocalDefId(2),
        };
        let second = ProductDefId {
            crate_id: ProductCrateId(0),
            local_id: ProductLocalDefId(1),
        };

        let mut map = BTreeMap::new();
        map.insert(first, "second item".to_string());
        map.insert(second, "first item".to_string());

        let keys = map.keys().copied().collect::<Vec<_>>();
        assert_eq!(keys, vec![second, first]);
    }
}
```

- [ ] **Step 4: Run the product ID tests**

Run: `cargo test -p rock-lib products::tests::product_def_id -- --nocapture`

Expected: PASS with 2 tests passing.

- [ ] **Step 5: Commit**

```bash
git add lib/src/lib.rs lib/src/products.rs
git commit -m "products: add canonical product IDs"
```

---

### Task 2: Add Canonical IDs to HIR Externs

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/collect/declarations.rs`
- Test: `lib/src/hir/mod.rs`

- [ ] **Step 1: Add a failing test that externs carry IDs**

Append this test to the existing `#[cfg(test)]` module in `lib/src/hir/mod.rs`. If the file has no test module, add this module at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use crate::hir::HirExtern;
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::types::Type;

    #[test]
    fn hir_extern_carries_canonical_def_id() {
        let extern_fn = HirExtern {
            id: DefId::new(CrateId(0), LocalDefId(9)),
            name: "puts".to_string(),
            params: vec![Type::Str],
            ret: Type::I32,
            variadic: false,
        };

        assert_eq!(extern_fn.id, DefId::new(CrateId(0), LocalDefId(9)));
    }
}
```

- [ ] **Step 2: Run the failing extern ID test**

Run: `cargo test -p rock-lib hir::tests::hir_extern_carries_canonical_def_id -- --nocapture`

Expected: FAIL to compile because `HirExtern` has no `id` field.

- [ ] **Step 3: Add `id` to `HirExtern`**

Update `lib/src/hir/mod.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirExtern {
    pub id: DefId,
    pub name: String,
    pub params: Vec<Type>,
    pub ret: Type,
    pub variadic: bool,
}
```

- [ ] **Step 4: Populate `HirExtern.id` in collection headers with a temporary collection ID**

In `lib/src/collect/headers.rs`, update `build_extern_with_name`:

```rust
    HirExtern {
        id: DefId::new(CrateId(0), LocalDefId(0)),
        name,
        params,
        ret,
        variadic: false,
    }
```

`CollectContext` does not own resolver tables at header-build time. This temporary ID is resolved in `Lowerer::from_declarations` before products can observe the final HIR.

If `DefId`, `CrateId`, or `LocalDefId` are not already imported in that file, add:

```rust
use crate::ids::{CrateId, DefId, LocalDefId};
```

- [ ] **Step 5: Populate `HirExtern.id` in direct lower fallback**

In `lib/src/lower/function.rs`, update `collect_extern` immediately before pushing `HirExtern`:

```rust
        let def_id = self.def_id_for_name(&[&name]);

        self.externs.push(HirExtern {
            id: def_id,
            name,
            params,
            ret,
            variadic: false,
        });
```

- [ ] **Step 6: Resolve collected extern IDs in `Lowerer::from_declarations`**

In `lib/src/lower/mod.rs`, update `from_declarations` by making `externs` mutable after destructuring:

```rust
        let mut externs = externs;
        resolve_extern_def_ids(&mut externs, &resolver);
```

Place that code immediately after the `let crate::collect::Declarations { ... } = decls;` block and before `let mut import_aliases = import_aliases;`.

Add this helper near the bottom of `lib/src/lower/mod.rs`, before `pub fn seg_name`:

```rust
fn resolve_extern_def_ids(
    externs: &mut [HirExtern],
    resolver: &crate::collect::resolver::ResolverTables,
) {
    for ext in externs {
        if let Some(def_id) = resolver
            .item_paths
            .get(&ext.name)
            .copied()
            .or_else(|| {
                ext.name
                    .rsplit("::")
                    .next()
                    .and_then(|short_name| resolver.item_paths.get(short_name).copied())
            })
        {
            ext.id = def_id;
        }
    }
}
```

- [ ] **Step 7: Populate `HirExtern.id` in crate/module declaration lowering**

In `lib/src/lower/collect/declarations.rs`, update both `self.externs.push(HirExtern { ... })` sites.

For the crate-qualified site:

```rust
                    let def_id = self.def_id_for_name(&[&qualified_name, &name]);

                    self.externs.push(HirExtern {
                        id: def_id,
                        name: qualified_name,
                        params,
                        ret,
                        variadic: false,
                    });
```

For the nested-module site:

```rust
                    let def_id = self.def_id_for_name(&[&qualified_name, &name]);

                    self.externs.push(HirExtern {
                        id: def_id,
                        name: qualified_name,
                        params,
                        ret,
                        variadic: false,
                    });
```

- [ ] **Step 8: Run the extern ID test**

Run: `cargo test -p rock-lib hir::tests::hir_extern_carries_canonical_def_id -- --nocapture`

Expected: PASS with 1 test passing.

- [ ] **Step 9: Run focused compile checks for affected modules**

Run: `cargo test -p rock-lib hir_extern_carries_canonical_def_id -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib collect`

Expected: PASS for collect tests.

Run: `cargo test -p rock-lib lower`

Expected: PASS for lower tests.

- [ ] **Step 10: Commit**

```bash
git add lib/src/hir/mod.rs lib/src/collect/headers.rs lib/src/lower/function.rs lib/src/lower/mod.rs lib/src/lower/collect/declarations.rs
git commit -m "hir: add DefId to extern declarations"
```

---

### Task 3: Add CompilerProducts Data Model and HIR Extraction

**Files:**
- Modify: `lib/src/products.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Add failing tests for metadata and body extraction**

Append these tests inside the existing `#[cfg(test)] mod tests` in `lib/src/products.rs`:

```rust
    use std::collections::HashMap;

    use crate::collect::resolver::ResolverTables;
    use crate::hir::{
        HirBlock, HirEnum, HirExpr, HirExprKind, HirExtern, HirFunction, HirImpl, HirParam,
        HirProgram, HirStruct, HirTrait,
    };
    use crate::ids::IdGen;
    use crate::infer::ResolvedHirProgram;
    use crate::lexer::Span;
    use crate::types::Type;

    fn empty_block(ty: Type) -> HirBlock {
        HirBlock { stmts: Vec::new(), ty }
    }

    fn int_body() -> HirBlock {
        HirBlock {
            stmts: vec![crate::hir::HirStmt::Expr(HirExpr {
                kind: HirExprKind::IntLiteral(1),
                ty: Type::I64,
                span: Span::default(),
            })],
            ty: Type::I64,
        }
    }

    fn test_function(id: DefId, name: &str, generic_params: Vec<String>) -> HirFunction {
        HirFunction {
            id,
            name: name.to_string(),
            qualified_name: None,
            generic_params,
            generic_bounds: HashMap::new(),
            params: vec![HirParam {
                name: "value".to_string(),
                ty: Type::I64,
                mutable: false,
                is_ref: false,
            }],
            ret_type: Type::I64,
            body: int_body(),
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }

    fn resolved_hir_for_products() -> ResolvedHirProgram {
        let plain_id = DefId::new(CrateId(0), LocalDefId(0));
        let generic_id = DefId::new(CrateId(0), LocalDefId(1));
        let struct_id = DefId::new(CrateId(0), LocalDefId(2));
        let enum_id = DefId::new(CrateId(0), LocalDefId(3));
        let trait_id = DefId::new(CrateId(0), LocalDefId(4));
        let impl_id = DefId::new(CrateId(0), LocalDefId(5));
        let extern_id = DefId::new(CrateId(0), LocalDefId(6));

        let mut functions = HashMap::new();
        functions.insert("plain".to_string(), test_function(plain_id, "plain", Vec::new()));
        functions.insert(
            "identity".to_string(),
            test_function(generic_id, "identity", vec!["T".to_string()]),
        );

        let mut structs = HashMap::new();
        structs.insert(
            "Box".to_string(),
            HirStruct {
                id: struct_id,
                name: "Box".to_string(),
                generic_params: vec!["T".to_string()],
                fields: Vec::new(),
            },
        );

        let mut enums = HashMap::new();
        enums.insert(
            "Maybe".to_string(),
            HirEnum {
                id: enum_id,
                name: "Maybe".to_string(),
                generic_params: vec!["T".to_string()],
                variants: Vec::new(),
            },
        );

        let default_method_id = DefId::new(CrateId(0), LocalDefId(7));
        let mut trait_methods = HashMap::new();
        trait_methods.insert(
            "show".to_string(),
            test_function(default_method_id, "show", Vec::new()),
        );
        let mut traits = HashMap::new();
        traits.insert(
            "Show".to_string(),
            HirTrait {
                id: trait_id,
                name: "Show".to_string(),
                generic_params: Vec::new(),
                associated_types: Vec::new(),
                methods: trait_methods,
                signatures: HashMap::new(),
            },
        );

        let impls = vec![HirImpl {
            id: impl_id,
            owner: crate::hir::HirImplOwner::Named("Box".to_string()),
            type_name: "Box".to_string(),
            type_generics: vec!["T".to_string()],
            receiver_arg_types: vec![Type::Generic("T".to_string())],
            trait_name: Some("Show".to_string()),
            trait_generics: Vec::new(),
            trait_arg_types: Vec::new(),
            associated_types: Vec::new(),
            bounds: Vec::new(),
            methods: HashMap::new(),
        }];

        let externs = vec![HirExtern {
            id: extern_id,
            name: "puts".to_string(),
            params: vec![Type::Str],
            ret: Type::I32,
            variadic: false,
        }];

        ResolvedHirProgram {
            program: HirProgram {
                functions,
                structs,
                enums,
                traits,
                impls,
                externs,
            },
            resolver: ResolverTables::default(),
            root_crate_id: CrateId(0),
            local_def_ids: IdGen::new(),
        }
    }

    #[test]
    fn compiler_products_key_metadata_by_product_def_id() {
        let hir = resolved_hir_for_products();

        let products = CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local("demo".to_string()),
            &hir,
            Vec::new(),
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        );

        let plain_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(0)));
        let extern_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(6)));

        assert!(products.metadata.functions.contains_key(&plain_id));
        assert!(products.metadata.externs.contains_key(&extern_id));
        assert_eq!(
            products.identity_table.export_names.get("plain"),
            Some(&plain_id)
        );
        assert_eq!(
            products.identity_table.display_names.get(&plain_id),
            Some(&"plain".to_string())
        );
    }

    #[test]
    fn compiler_products_select_downstream_bodies_by_product_def_id() {
        let hir = resolved_hir_for_products();

        let products = CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local("demo".to_string()),
            &hir,
            Vec::new(),
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        );

        let plain_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(0)));
        let generic_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(1)));
        let impl_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(5)));
        let default_method_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(7)));

        assert!(!products.bodies.functions.contains_key(&plain_id));
        assert!(products.bodies.functions.contains_key(&generic_id));
        assert!(products.bodies.generic_impls.contains_key(&impl_id));
        assert!(products.bodies.trait_default_methods.contains_key(&default_method_id));
    }
```

- [ ] **Step 2: Run the failing product extraction tests**

Run: `cargo test -p rock-lib products::tests::compiler_products -- --nocapture`

Expected: FAIL to compile because `CompilerProducts`, product sections, and extraction are not defined.

- [ ] **Step 3: Add product section types**

Add these imports and types near the top of `lib/src/products.rs`, below the product ID conversions:

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::hir::{HirEnum, HirExtern, HirFunction, HirImpl, HirStruct, HirTrait};
use crate::infer::ResolvedHirProgram;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerProducts {
    pub crate_identity: ProductCrateIdentity,
    pub identity_table: ProductIdentityTable,
    pub metadata: ProductMetadata,
    pub bodies: ProductBodies,
    pub link: ProductLinkData,
    pub dependencies: Vec<ProductDependencyIdentity>,
    pub source_fingerprint: ProductSourceFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub dependencies: Vec<ProductCrateIdentity>,
    pub display_names: BTreeMap<ProductDefId, String>,
    pub export_names: BTreeMap<String, ProductDefId>,
    pub backend_symbols: BTreeMap<ProductDefId, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductMetadata {
    pub functions: BTreeMap<ProductDefId, HirFunction>,
    pub structs: BTreeMap<ProductDefId, HirStruct>,
    pub enums: BTreeMap<ProductDefId, HirEnum>,
    pub traits: BTreeMap<ProductDefId, HirTrait>,
    pub impls: BTreeMap<ProductDefId, HirImpl>,
    pub externs: BTreeMap<ProductDefId, HirExtern>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductBodies {
    pub functions: BTreeMap<ProductDefId, HirFunction>,
    pub generic_impls: BTreeMap<ProductDefId, HirImpl>,
    pub trait_default_methods: BTreeMap<ProductDefId, HirFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductLinkData {
    pub object_path: Option<PathBuf>,
    pub records: BTreeMap<ProductDefId, ProductLinkRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductLinkRecord {
    pub backend_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDependencyIdentity {
    pub name: String,
    pub artifact_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductSourceFingerprint {
    pub manifest_hash: Option<String>,
    pub source_hash: Option<String>,
    pub loaded_files: Vec<PathBuf>,
}
```

- [ ] **Step 4: Implement extraction from finalized HIR**

Add this implementation in `lib/src/products.rs`:

```rust
impl CompilerProducts {
    pub fn from_resolved_hir(
        crate_identity: ProductCrateIdentity,
        hir: &ResolvedHirProgram,
        dependencies: Vec<ProductDependencyIdentity>,
        source_fingerprint: ProductSourceFingerprint,
        link: ProductLinkData,
    ) -> Self {
        let mut identity_table = ProductIdentityTable {
            local_crate: Some(ProductCrateId::from(hir.root_crate_id)),
            ..ProductIdentityTable::default()
        };
        let mut metadata = ProductMetadata::default();
        let mut bodies = ProductBodies::default();

        for (name, function) in &hir.program.functions {
            let id = ProductDefId::from(function.id);
            record_name(&mut identity_table, id, name);
            metadata.functions.insert(id, function.clone());
            if !function.generic_params.is_empty() {
                bodies.functions.insert(id, function.clone());
            }
        }

        for (name, strukt) in &hir.program.structs {
            let id = ProductDefId::from(strukt.id);
            record_name(&mut identity_table, id, name);
            metadata.structs.insert(id, strukt.clone());
        }

        for (name, enm) in &hir.program.enums {
            let id = ProductDefId::from(enm.id);
            record_name(&mut identity_table, id, name);
            metadata.enums.insert(id, enm.clone());
        }

        for (name, trt) in &hir.program.traits {
            let id = ProductDefId::from(trt.id);
            record_name(&mut identity_table, id, name);
            for (method_name, method) in &trt.methods {
                let method_id = ProductDefId::from(method.id);
                record_name(
                    &mut identity_table,
                    method_id,
                    &format!("{}::{}", name, method_name),
                );
                if !method.body.stmts.is_empty() {
                    bodies.trait_default_methods.insert(method_id, method.clone());
                }
            }
            metadata.traits.insert(id, trt.clone());
        }

        for imp in &hir.program.impls {
            let id = ProductDefId::from(imp.id);
            record_name(&mut identity_table, id, &impl_display_name(imp));
            if !imp.type_generics.is_empty() || !imp.trait_generics.is_empty() {
                bodies.generic_impls.insert(id, imp.clone());
            }
            metadata.impls.insert(id, imp.clone());
        }

        for ext in &hir.program.externs {
            let id = ProductDefId::from(ext.id);
            record_name(&mut identity_table, id, &ext.name);
            metadata.externs.insert(id, ext.clone());
        }

        Self {
            crate_identity,
            identity_table,
            metadata,
            bodies,
            link,
            dependencies,
            source_fingerprint,
        }
    }
}

fn record_name(identity_table: &mut ProductIdentityTable, id: ProductDefId, name: &str) {
    identity_table.display_names.insert(id, name.to_string());
    identity_table.export_names.insert(name.to_string(), id);
}

fn impl_display_name(imp: &HirImpl) -> String {
    match &imp.trait_name {
        Some(trait_name) => format!("{} as {}", imp.type_name, trait_name),
        None => imp.type_name.clone(),
    }
}
```

- [ ] **Step 5: Run the product extraction tests**

Run: `cargo test -p rock-lib products::tests::compiler_products -- --nocapture`

Expected: PASS with 2 tests passing.

- [ ] **Step 6: Run all product tests**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS with 4 product tests passing.

- [ ] **Step 7: Commit**

```bash
git add lib/src/products.rs
git commit -m "products: extract compiler products from HIR"
```

---

### Task 4: Add `compile_with_products` Beside Existing `compile`

**Files:**
- Modify: `lib/src/lib.rs`
- Test: `lib/src/lib.rs`

- [ ] **Step 1: Add a failing test for normal-pipeline product extraction**

Add this test module at the bottom of `lib/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{compile_with_products, Config};

    #[test]
    fn compile_with_products_extracts_ids_from_normal_pipeline() {
        let base = std::env::temp_dir().join(format!(
            "rock_compile_products_{}_{}",
            std::process::id(),
            "normal_pipeline"
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let entry_file = base.join("main.rk");
        fs::write(&entry_file, "main = ->\n    0\n").unwrap();

        let output = compile_with_products(&Config {
            entry_file,
            output_dir: base.join("build"),
            no_std: true,
            no_prelude: true,
            no_link: true,
            current_crate_name: Some("demo".to_string()),
            ..Config::default()
        })
        .unwrap();

        let products = output.products.expect("products should be emitted");
        let main_id = products
            .identity_table
            .export_names
            .get("main")
            .copied()
            .expect("main should have a product ID");

        assert!(products.metadata.functions.contains_key(&main_id));
        assert_eq!(
            products.identity_table.display_names.get(&main_id),
            Some(&"main".to_string())
        );

        let _ = fs::remove_dir_all(&base);
    }
}
```

- [ ] **Step 2: Run the failing compile-with-products test**

Run: `cargo test -p rock-lib compile_with_products_extracts_ids_from_normal_pipeline -- --nocapture`

Expected: FAIL to compile because `compile_with_products` and `CompileOutput` are not defined.

- [ ] **Step 3: Add `CompileOutput` and refactor `compile` through `compile_impl`**

In `lib/src/lib.rs`, add this import near the existing `use crate::lexer::Span;` line:

```rust
use crate::products::{
    CompilerProducts, ProductCrateIdentity, ProductDependencyIdentity, ProductLinkData,
    ProductSourceFingerprint,
};
```

Add this struct below `impl Config`:

```rust
#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub ast: Program,
    pub products: Option<CompilerProducts>,
}
```

Replace the start of the current `compile` function with these three functions:

```rust
pub fn compile(config: &Config) -> Result<Program, Diagnostics> {
    compile_impl(config, false).map(|output| output.ast)
}

pub fn compile_with_products(config: &Config) -> Result<CompileOutput, Diagnostics> {
    compile_impl(config, true)
}

fn compile_impl(config: &Config, emit_products: bool) -> Result<CompileOutput, Diagnostics> {
```

Keep the existing body of `compile` inside `compile_impl`.

- [ ] **Step 4: Extract products after inference and before MIR/mono consumes HIR**

In `lib/src/lib.rs`, immediately after the HIR debug print block and before MIR building, add:

```rust
    let products = if emit_products {
        Some(CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local(
                config
                    .current_crate_name
                    .clone()
                    .unwrap_or_else(|| module_name_from_entry(&config.entry_file)),
            ),
            &hir,
            product_dependencies_from_config(config),
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        ))
    } else {
        None
    };
```

This code uses `module_name_from_entry`, so add the helper shown in Step 5 before running tests.

- [ ] **Step 5: Add helpers for crate name and dependency product identities**

In `lib/src/lib.rs`, add these helpers below `compile_impl`:

```rust
fn module_name_from_entry(entry_file: &std::path::Path) -> String {
    entry_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main")
        .to_string()
}

fn product_dependencies_from_config(config: &Config) -> Vec<ProductDependencyIdentity> {
    config
        .extern_artifacts
        .iter()
        .map(|(name, path)| ProductDependencyIdentity {
            name: name.clone(),
            artifact_path: path.clone(),
        })
        .collect()
}
```

Then replace the existing local `module_name` calculation in `compile_impl` with:

```rust
    let module_name = module_name_from_entry(&config.entry_file);
```

Also update the `CodeGen` constructor call to borrow the new `String`:

```rust
    let mut codegen = codegen::CodeGen::new(&context, &module_name);
```

- [ ] **Step 6: Return `CompileOutput` from `compile_impl`**

At the end of `compile_impl`, replace:

```rust
    Ok(ast)
```

with:

```rust
    Ok(CompileOutput { ast, products })
```

- [ ] **Step 7: Run the compile-with-products test**

Run: `cargo test -p rock-lib compile_with_products_extracts_ids_from_normal_pipeline -- --nocapture`

Expected: PASS with 1 test passing.

- [ ] **Step 8: Run existing crate artifact compile tests to prove `compile` behavior remains compatible**

Run: `cargo test -p rock-lib crate_artifact::tests::test_compile_without_explicit_stdlib_artifact_fails_with_no_std -- --nocapture`

Expected: PASS with the existing expected compile failure behavior inside the test.

- [ ] **Step 9: Commit**

```bash
git add lib/src/lib.rs
git commit -m "compiler: expose products from normal compile path"
```

---

### Task 5: Verify Serialization and Full Library Tests

**Files:**
- Modify: `lib/src/products.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Add a product roundtrip test**

Append this test inside `#[cfg(test)] mod tests` in `lib/src/products.rs`:

```rust
    #[test]
    fn compiler_products_roundtrip_preserves_product_def_ids() {
        let hir = resolved_hir_for_products();
        let products = CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local("demo".to_string()),
            &hir,
            vec![ProductDependencyIdentity {
                name: "dep".to_string(),
                artifact_path: std::path::PathBuf::from("build/dep.rkca"),
            }],
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        );

        let bytes = bincode::serialize(&products).unwrap();
        let decoded: CompilerProducts = bincode::deserialize(&bytes).unwrap();

        let generic_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(1)));
        assert!(decoded.metadata.functions.contains_key(&generic_id));
        assert!(decoded.bodies.functions.contains_key(&generic_id));
        assert_eq!(
            decoded.identity_table.export_names.get("identity"),
            Some(&generic_id)
        );
        assert_eq!(decoded.dependencies[0].name, "dep");
    }
```

- [ ] **Step 2: Run the product roundtrip test**

Run: `cargo test -p rock-lib compiler_products_roundtrip_preserves_product_def_ids -- --nocapture`

Expected: PASS with 1 test passing.

- [ ] **Step 3: Run all product and compile-with-products tests**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for product tests.

Run: `cargo test -p rock-lib compile_with_products -- --nocapture`

Expected: PASS for the compile-with-products test.

- [ ] **Step 4: Run the library test suite**

Run: `cargo test -p rock-lib`

Expected: PASS for the full `rock-lib` test suite.

- [ ] **Step 5: Commit**

```bash
git add lib/src/products.rs
git commit -m "products: preserve product IDs through serialization"
```

---

## Completion Criteria

- `lib/src/products.rs` exists and contains canonical `ProductDefId` product identity types.
- `CompilerProducts` has `identity_table`, `metadata`, `bodies`, `link`, `dependencies`, and `source_fingerprint` sections.
- Product metadata and bodies are keyed by `ProductDefId`.
- String names appear only in secondary indexes such as `display_names`, `export_names`, and backend symbols.
- `compile(config)` still returns `Program` and existing callers do not change behavior.
- `compile_with_products(config)` returns `CompileOutput` with `Some(CompilerProducts)` extracted after inference from the normal pipeline.
- `cargo test -p rock-lib` passes.

## Next Plan After This Slice

After this plan lands, write a separate plan for `rockc` artifact/object emission using `CompilerProducts`. That plan should add CLI flags and a product-backed artifact format adapter while keeping existing artifact loading behavior available until broad integration tests pass.
