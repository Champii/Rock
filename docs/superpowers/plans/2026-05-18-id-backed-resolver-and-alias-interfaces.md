# ID-Backed Resolver And Alias Interfaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make resolver alias tables the authoritative collection-to-lowering and product-artifact alias interface for roadmap Tasks 4-5.

**Architecture:** Add narrow ID-backed resolver APIs first, then route lowerer resolution through a small facade while preserving compatibility string views for local scope and diagnostics. Persist prelude exports by product `DefId` the same way root exports already are, leaving full removal of string compatibility maps to later roadmap tasks.

**Tech Stack:** Rust 2021, `rock-lib`, `ResolverTables`, `DefId`, `ArtifactExport`, `ProductDefId`, `Lowerer`, product artifacts, focused Cargo tests, `cargo fmt --all --check`, `cargo test -p rock-lib`.

---

## Source Documents

- Design: `docs/superpowers/specs/2026-05-18-id-backed-resolver-and-alias-interfaces-design.md`
- Roadmap: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Tracker: `docs/superpowers/plans/master-audit-checklist.md`

## Execution Notes

- Do not touch untracked `.sisyphus/`.
- Commit after each completed task.
- Run tests serially.
- Preserve string maps as compatibility views where this plan says to keep them.
- Do not start the `Ty`/type-context, instance registry, MIR/codegen, or source-loader tasks.

## File Structure

- Modify `lib/src/collect/resolver.rs`: add resolver helper APIs and unit tests.
- Modify `lib/src/lower/mod.rs`: add lowerer resolution facade, migrate owner/nominal/trait lookup, and add lowerer tests.
- Modify `lib/src/lower/types.rs`: prefer resolver ID lookup for struct/enum type lowering and add nominal alias tests.
- Modify `lib/src/lower/paths.rs`: use the lowerer facade when resolving top-level alias names for `ResolvedVar`.
- Modify `lib/src/lower/crates/registration.rs`: use resolver insert helpers for prelude aliases and keep body-sync semantics unchanged.
- Modify `lib/src/products.rs`: add ID-backed prelude export persistence and product tests.
- Modify `lib/src/lib.rs`: record stdlib prelude exports through the product helper.
- Modify `lib/src/crate_artifact/load.rs`: load prelude exports from IDs first and add remap/rejection tests.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: record the completed resolver/alias slice and keep remaining work accurate.
- Modify `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`: mark Task 4 and the Task 5 resolver-boundary subset complete if implementation reaches that state.

---

### Task 1: Add `ResolverTables` Helper APIs

**Files:**
- Modify: `lib/src/collect/resolver.rs`

- [ ] **Step 1: Add failing resolver helper tests**

Add this test module at the end of `lib/src/collect/resolver.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ids::{CrateId, DefId, LocalDefId};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn resolver_tables_resolve_items_imports_and_exports_by_id() {
        let item_id = def_id(1);
        let import_id = def_id(2);
        let export_id = def_id(3);
        let mut resolver = ResolverTables::default();
        resolver.item_paths.insert("crate::item".to_string(), item_id);
        resolver.import_aliases.insert("imported".to_string(), import_id);
        resolver.export_aliases.insert("crate::alias".to_string(), export_id);

        assert_eq!(resolver.resolve_item_or_alias("crate::item"), Some(item_id));
        assert_eq!(resolver.resolve_item_or_alias("imported"), Some(import_id));
        assert_eq!(resolver.resolve_item_or_alias("crate::alias"), Some(export_id));
        assert_eq!(resolver.resolve_item_or_alias("missing"), None);
    }

    #[test]
    fn resolver_tables_insert_aliases_with_reverse_names() {
        let import_id = def_id(10);
        let export_id = def_id(11);
        let mut resolver = ResolverTables::default();

        resolver.insert_import_alias_with_name(
            "short".to_string(),
            "dep::long".to_string(),
            import_id,
        );
        resolver.insert_export_alias_with_name(
            "crate::public".to_string(),
            "crate::private".to_string(),
            export_id,
        );

        assert_eq!(resolver.import_aliases.get("short"), Some(&import_id));
        assert_eq!(resolver.export_aliases.get("crate::public"), Some(&export_id));
        assert_eq!(resolver.canonical_name(import_id), Some("dep::long"));
        assert_eq!(resolver.canonical_name(export_id), Some("crate::private"));
    }
}
```

- [ ] **Step 2: Run the failing resolver tests**

Run: `cargo test -p rock-lib resolver_tables_ -- --nocapture`

Expected: FAIL because the helper methods do not exist.

- [ ] **Step 3: Implement the helper APIs**

Add this `impl` block after `ResolverTables` in `lib/src/collect/resolver.rs`:

```rust
impl ResolverTables {
    pub fn resolve_item_or_alias(&self, name: &str) -> Option<DefId> {
        self.item_paths
            .get(name)
            .copied()
            .or_else(|| self.import_aliases.get(name).copied())
            .or_else(|| self.export_aliases.get(name).copied())
    }

    pub fn canonical_name(&self, id: DefId) -> Option<&str> {
        self.item_names_by_id.get(&id).map(String::as_str)
    }

    pub fn insert_import_alias_with_name(&mut self, alias: String, source: String, id: DefId) {
        self.import_aliases.insert(alias, id);
        self.item_names_by_id.entry(id).or_insert(source);
    }

    pub fn insert_export_alias_with_name(&mut self, alias: String, source: String, id: DefId) {
        self.export_aliases.insert(alias, id);
        self.item_names_by_id.entry(id).or_insert(source);
    }
}
```

Replace direct chained lookups inside `build_resolver_tables` with these helpers where practical:

```rust
let resolved_import_aliases = import_aliases
    .iter()
    .filter_map(|(alias, target_path)| {
        canonical_item_paths
            .get(target_path)
            .copied()
            .map(|def_id| (alias.clone(), def_id))
    })
    .collect();
```

can remain as-is because it builds the table before the helper can be used.

- [ ] **Step 4: Run resolver tests again**

Run: `cargo test -p rock-lib resolver_tables_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add lib/src/collect/resolver.rs
git commit -m "add resolver alias helper APIs"
```

---

### Task 2: Add A Lowerer Resolution Facade

**Files:**
- Modify: `lib/src/lower/mod.rs`

- [ ] **Step 1: Add failing lowerer facade tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `lib/src/lower/mod.rs`:

```rust
#[test]
fn lowerer_resolves_current_and_dependency_aliases_through_facade() {
    let current_id = DefId::new(CrateId(0), LocalDefId(1));
    let dependency_id = DefId::new(CrateId(7), LocalDefId(2));
    let mut lowerer = Lowerer::new();
    lowerer
        .resolver
        .insert_import_alias_with_name("local_alias".to_string(), "demo::answer".to_string(), current_id);

    let mut dependency = ResolverTables::default();
    dependency.insert_export_alias_with_name(
        "dep::public".to_string(),
        "dep::internal::answer".to_string(),
        dependency_id,
    );
    lowerer
        .dependency_resolvers
        .insert("dep".to_string(), dependency);

    assert_eq!(lowerer.resolve_item_def_id("local_alias"), Some(current_id));
    assert_eq!(lowerer.resolve_item_def_id("dep::public"), Some(dependency_id));
    assert_eq!(
        lowerer.canonical_name_for_alias_or_item("dep::public").as_deref(),
        Some("dep::internal::answer")
    );
}

#[test]
fn def_id_for_name_uses_resolver_facade_for_dependencies() {
    let dependency_id = DefId::new(CrateId(9), LocalDefId(4));
    let mut lowerer = Lowerer::new();
    let mut dependency = ResolverTables::default();
    dependency.insert_import_alias_with_name(
        "answer".to_string(),
        "dep::answer".to_string(),
        dependency_id,
    );
    lowerer
        .dependency_resolvers
        .insert("dep".to_string(), dependency);

    assert_eq!(lowerer.def_id_for_name(&["answer"]), dependency_id);
}
```

- [ ] **Step 2: Run the failing facade tests**

Run: `cargo test -p rock-lib lowerer_resolves_current_and_dependency_aliases_through_facade -- --nocapture`

Run: `cargo test -p rock-lib def_id_for_name_uses_resolver_facade_for_dependencies -- --nocapture`

Expected: FAIL because the lowerer facade methods do not exist.

- [ ] **Step 3: Implement the facade methods**

Add these methods inside `impl Lowerer` in `lib/src/lower/mod.rs`, near `def_id_for_name`:

```rust
pub(crate) fn resolve_item_def_id(&self, name: &str) -> Option<DefId> {
    self.resolver.resolve_item_or_alias(name).or_else(|| {
        self.dependency_resolvers
            .values()
            .find_map(|resolver| resolver.resolve_item_or_alias(name))
    })
}

pub(crate) fn canonical_name_for_def_id(&self, id: DefId) -> Option<&str> {
    self.resolver.canonical_name(id).or_else(|| {
        self.dependency_resolvers
            .values()
            .find_map(|resolver| resolver.canonical_name(id))
    })
}

pub(crate) fn canonical_name_for_alias_or_item(&self, name: &str) -> Option<String> {
    let id = self.resolve_item_def_id(name)?;
    self.canonical_name_for_def_id(id).map(ToString::to_string)
}
```

Change `def_id_for_name` to use the facade:

```rust
pub(crate) fn def_id_for_name(&self, candidates: &[&str]) -> crate::ids::DefId {
    for candidate in candidates {
        if let Some(def_id) = self.resolve_item_def_id(candidate) {
            return def_id;
        }
    }

    panic!("missing canonical DefId for lowered item: {:?}", candidates);
}
```

Change `nominal_def_id_for_name` to use the facade:

```rust
pub(crate) fn nominal_def_id_for_name(
    &self,
    name: &str,
    fallback: crate::ids::DefId,
) -> crate::ids::DefId {
    self.resolve_item_def_id(name).unwrap_or(fallback)
}
```

Change `try_canonical_owner_path` to use `canonical_name_for_def_id`:

```rust
let def_id = candidates
    .into_iter()
    .find_map(|candidate| self.resolve_owner_def_id(&candidate))?;

self.canonical_name_for_def_id(def_id).map(ToString::to_string)
```

Change the first resolver lookup chain in `resolve_owner_def_id_inner` to:

```rust
self.resolve_item_def_id(candidate).or_else(|| {
    candidate.rsplit("::").next().and_then(|short_name| {
        self.stdlib_prelude_exports
            .get(short_name)
            .and_then(|source| source.as_deref())
            .and_then(|source| self.resolve_owner_def_id_inner(source, visited))
    })
})
```

- [ ] **Step 4: Update direct resolver lookups in existing helper functions**

In `stdlib_builtin_trait_def_id`, replace the direct lookup chain with:

```rust
self.resolve_item_def_id(&canonical_path).or_else(|| {
    self.resolver
        .import_aliases
        .get(name)
        .copied()
        .filter(|id| {
            self.resolver
                .canonical_name(*id)
                .is_some_and(|source| source == canonical_path)
        })
})
```

In `resolve_extern_def_ids`, use `resolver.resolve_item_or_alias` for the full and short names:

```rust
if let Some(def_id) = resolver.resolve_item_or_alias(&ext.name).or_else(|| {
    ext.name
        .rsplit("::")
        .next()
        .and_then(|short_name| resolver.resolve_item_or_alias(short_name))
}) {
    ext.id = def_id;
}
```

- [ ] **Step 5: Run lowerer facade tests**

Run: `cargo test -p rock-lib lowerer_resolves_current_and_dependency_aliases_through_facade -- --nocapture`

Run: `cargo test -p rock-lib def_id_for_name_uses_resolver_facade_for_dependencies -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run existing lowerer alias tests**

Run: `cargo test -p rock-lib lowerer_from_declarations_ -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add lib/src/lower/mod.rs
git commit -m "route lowerer aliases through resolver facade"
```

---

### Task 3: Prefer Resolver IDs In Type And Trait Alias Lookup

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/lower/paths.rs`

- [ ] **Step 1: Add failing nominal alias tests**

In `lib/src/lower/types.rs`, add this test inside the existing test module:

```rust
#[test]
fn lower_nominal_type_prefers_resolver_alias_id_over_suffix_match() {
    let canonical_id = def_id(40);
    let suffix_collision_id = def_id(41);
    let mut lowerer = Lowerer::new();
    lowerer.structs.insert(
        "dep::Widget".to_string(),
        HirStruct {
            id: canonical_id,
            name: "dep::Widget".to_string(),
            generic_params: Vec::new(),
            fields: Vec::new(),
        },
    );
    lowerer.structs.insert(
        "other::Widget".to_string(),
        HirStruct {
            id: suffix_collision_id,
            name: "other::Widget".to_string(),
            generic_params: Vec::new(),
            fields: Vec::new(),
        },
    );
    lowerer
        .resolver
        .insert_import_alias_with_name("Widget".to_string(), "dep::Widget".to_string(), canonical_id);

    let ty = lowerer.lower_parse_type(&named_type("Widget"));

    assert_eq!(
        ty,
        Type::Struct {
            id: canonical_id,
            args: Vec::new(),
        }
    );
}
```

In `lib/src/lower/mod.rs`, add this test inside the existing test module:

```rust
#[test]
fn trait_by_name_prefers_resolver_alias_id() {
    let canonical_id = DefId::new(CrateId(0), LocalDefId(30));
    let collision_id = DefId::new(CrateId(0), LocalDefId(31));
    let mut lowerer = Lowerer::new();
    lowerer.traits.insert(
        "dep::Show".to_string(),
        test_trait(canonical_id, "dep::Show"),
    );
    lowerer.traits.insert(
        "other::Show".to_string(),
        test_trait(collision_id, "other::Show"),
    );
    lowerer
        .resolver
        .insert_import_alias_with_name("Show".to_string(), "dep::Show".to_string(), canonical_id);

    let trait_def = lowerer.trait_by_name("Show").unwrap();

    assert_eq!(trait_def.id, canonical_id);
}
```

If `test_trait` does not exist, add this helper inside the lowerer test module:

```rust
fn test_trait(id: DefId, name: &str) -> HirTrait {
    HirTrait {
        id,
        name: name.to_string(),
        generic_params: Vec::new(),
        associated_types: Vec::new(),
        methods: HashMap::new(),
        signatures: HashMap::new(),
    }
}
```

- [ ] **Step 2: Run the failing nominal/trait tests**

Run: `cargo test -p rock-lib lower_nominal_type_prefers_resolver_alias_id_over_suffix_match -- --nocapture`

Run: `cargo test -p rock-lib trait_by_name_prefers_resolver_alias_id -- --nocapture`

Expected: FAIL if type/trait lookup still reaches suffix or string-map behavior before resolver IDs.

- [ ] **Step 3: Add lowerer ID-backed definition helpers**

Add these methods in `impl Lowerer` in `lib/src/lower/mod.rs`:

```rust
pub(crate) fn struct_by_resolved_name(&self, name: &str) -> Option<&HirStruct> {
    self.resolve_item_def_id(name)
        .and_then(|id| self.structs.values().find(|structure| structure.id == id))
        .or_else(|| self.structs.get(name))
}

pub(crate) fn enum_by_resolved_name(&self, name: &str) -> Option<&HirEnum> {
    self.resolve_item_def_id(name)
        .and_then(|id| self.enums.values().find(|enum_| enum_.id == id))
        .or_else(|| self.enums.get(name))
}

pub(crate) fn trait_by_resolved_name(&self, name: &str) -> Option<&HirTrait> {
    self.resolve_item_def_id(name)
        .and_then(|id| self.traits.values().find(|trait_def| trait_def.id == id))
        .or_else(|| self.traits.get(name))
}
```

Change `trait_by_name` to use this helper before legacy string maps:

```rust
pub(crate) fn trait_by_name(&self, name: &str) -> Option<&HirTrait> {
    self.trait_by_resolved_name(name)
        .or_else(|| {
            self.import_aliases
                .get(name)
                .and_then(|qualified| self.traits.get(qualified))
        })
        .or_else(|| {
            self.module_local_aliases
                .get(name)
                .and_then(|qualified| self.traits.get(qualified))
        })
}
```

- [ ] **Step 4: Update type lowering to use resolved helpers first**

In `lib/src/lower/types.rs`, replace the struct/enum lookup block for non-builtin names with this shape:

```rust
name => {
    let current_module_name = self
        .current_module_prefix()
        .map(|prefix| format!("{}::{}", prefix, name));

    if let Some(struct_def) = current_module_name
        .as_deref()
        .and_then(|name| self.struct_by_resolved_name(name))
        .or_else(|| self.struct_by_resolved_name(name))
        .or_else(|| {
            self.structs
                .iter()
                .find(|(struct_name, _)| struct_name.rsplit("::").next() == Some(name))
                .map(|(_, struct_def)| struct_def)
        })
    {
        Type::Struct {
            id: struct_def.id,
            args: generics,
        }
    } else if let Some(enum_def) = current_module_name
        .as_deref()
        .and_then(|name| self.enum_by_resolved_name(name))
        .or_else(|| self.enum_by_resolved_name(name))
        .or_else(|| {
            self.enums
                .iter()
                .find(|(enum_name, _)| enum_name.rsplit("::").next() == Some(name))
                .map(|(_, enum_def)| enum_def)
        })
    {
        Type::Enum {
            id: enum_def.id,
            args: generics,
        }
    } else if let Some(generic) = self.current_generic_type_for_name(name) {
        generic
    } else {
        Type::Error
    }
}
```

- [ ] **Step 5: Update expression path canonical name selection**

In `lib/src/lower/paths.rs`, replace the alias target name construction inside `lower_identifier_path` with resolver-first lookup:

```rust
let hir_name = self
    .module_local_aliases
    .get(name)
    .filter(|_| binding_is_alias)
    .cloned()
    .or_else(|| {
        if binding_is_alias {
            self.canonical_name_for_alias_or_item(name)
                .or_else(|| self.import_aliases.get(name).cloned())
        } else {
            None
        }
    })
    .unwrap_or_else(|| name.clone());
```

- [ ] **Step 6: Run nominal, trait, and existing alias tests**

Run: `cargo test -p rock-lib lower_nominal_type_prefers_resolver_alias_id_over_suffix_match -- --nocapture`

Run: `cargo test -p rock-lib trait_by_name_prefers_resolver_alias_id -- --nocapture`

Run: `cargo test -p rock-lib lowerer_from_declarations_ -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add lib/src/lower/mod.rs lib/src/lower/types.rs lib/src/lower/paths.rs
git commit -m "prefer resolver ids for lowerer alias lookup"
```

---

### Task 4: Persist Prelude Exports By Product ID

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add failing product prelude ID tests**

In `lib/src/products.rs`, add this test inside the existing test module:

```rust
#[test]
fn compiler_products_record_prelude_export_ids() {
    let hir = resolved_hir_for_products();
    let mut products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let plain_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(0)));

    products.record_prelude_exports([(
        "plain".to_string(),
        "plain".to_string(),
    )]);

    assert_eq!(products.prelude_exports.get("plain"), Some(&"plain".to_string()));
    assert_eq!(
        products.identity_table.prelude_export_names.get("plain"),
        Some(&plain_id)
    );
}

#[test]
fn compiler_products_roundtrip_preserves_prelude_export_ids() {
    let hir = resolved_hir_for_products();
    let mut products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let plain_id = ProductDefId::from(DefId::new(CrateId(0), LocalDefId(0)));
    products.record_prelude_exports([("plain".to_string(), "plain".to_string())]);

    let bytes = products.to_artifact_bytes().unwrap();
    let decoded = CompilerProducts::from_artifact_bytes(&bytes).unwrap();

    assert_eq!(
        decoded.identity_table.prelude_export_names.get("plain"),
        Some(&plain_id)
    );
}
```

- [ ] **Step 2: Run the failing product tests**

Run: `cargo test -p rock-lib compiler_products_record_prelude_export_ids -- --nocapture`

Run: `cargo test -p rock-lib compiler_products_roundtrip_preserves_prelude_export_ids -- --nocapture`

Expected: FAIL because `prelude_export_names` and `record_prelude_exports` do not exist.

- [ ] **Step 3: Add the product identity field and bump the artifact format**

In `lib/src/products.rs`, update the format version:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 18;
```

Add the field to `ProductIdentityTable`:

```rust
#[serde(default)]
pub prelude_export_names: BTreeMap<String, ProductDefId>,
```

Place it after `export_names` and before `ambiguous_export_names`.

- [ ] **Step 4: Add product prelude recording helpers**

Add these methods inside `impl CompilerProducts` in `lib/src/products.rs`:

```rust
pub fn record_prelude_exports(
    &mut self,
    exports: impl IntoIterator<Item = (String, String)>,
) {
    for (alias, source) in exports {
        self.prelude_exports.insert(alias.clone(), source.clone());
        if let Some(id) = self.product_def_id_for_source(&source) {
            self.identity_table.prelude_export_names.insert(alias, id);
        }
    }
}

fn product_def_id_for_source(&self, source: &str) -> Option<ProductDefId> {
    self.identity_table
        .display_names
        .iter()
        .find_map(|(id, name)| {
            let crate_qualified = format!("{}::{}", self.crate_identity.name, name);
            (name == source || crate_qualified == source).then_some(*id)
        })
}
```

- [ ] **Step 5: Use the helper during stdlib product emission**

In `lib/src/lib.rs`, replace:

```rust
products.prelude_exports = stdlib_prelude_exports
    .into_iter()
    .filter_map(|(name, source)| source.map(|source| (name, source)))
    .collect();
```

with:

```rust
products.record_prelude_exports(
    stdlib_prelude_exports
        .into_iter()
        .filter_map(|(name, source)| source.map(|source| (name, source))),
);
```

- [ ] **Step 6: Run product prelude tests**

Run: `cargo test -p rock-lib compiler_products_record_prelude_export_ids -- --nocapture`

Run: `cargo test -p rock-lib compiler_products_roundtrip_preserves_prelude_export_ids -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add lib/src/products.rs lib/src/lib.rs
git commit -m "persist prelude exports by product id"
```

---

### Task 5: Load Prelude Exports From Product IDs First

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Add failing artifact loader tests**

In `lib/src/crate_artifact/load.rs`, add these tests near the existing prelude export tests:

```rust
#[test]
fn load_product_artifact_records_id_backed_prelude_exports() {
    let base = std::env::temp_dir().join(format!(
        "rock_product_loader_{}_{}",
        std::process::id(),
        "id_backed_prelude_exports"
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let artifact_path = base.join("stdlib.rkca");
    let object_path = base.join("stdlib.o");
    fs::write(&object_path, []).unwrap();

    let struct_id = DefId::new(CrateId(1), LocalDefId(7));
    let product_id = ProductDefId::from(struct_id);
    let mut identity_table = ProductIdentityTable::default();
    identity_table.local_crate = Some(ProductCrateId(1));
    identity_table
        .display_names
        .insert(product_id, "stdlib::prelude::String".to_string());
    identity_table
        .prelude_export_names
        .insert("String".to_string(), product_id);

    let mut metadata = ProductMetadata::default();
    metadata.structs.insert(
        product_id,
        HirStruct {
            id: struct_id,
            name: "String".to_string(),
            generic_params: Vec::new(),
            fields: Vec::new(),
        },
    );

    let products = CompilerProducts {
        crate_identity: ProductCrateIdentity::local("stdlib".to_string()),
        identity_table,
        metadata,
        bodies: Default::default(),
        link: ProductLinkData {
            object_path: Some(object_path.clone()),
            records: Default::default(),
        },
        dependencies: Vec::new(),
        source_fingerprint: ProductSourceFingerprint::default(),
        prelude_exports: Default::default(),
        infix_precedence: Default::default(),
    };
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path(artifact_path).unwrap();

    let loaded = ctx.crates.get("stdlib").unwrap();
    let export = loaded.prelude_export_ids.get("String").unwrap();
    assert_eq!(export.source, "stdlib::prelude::String");
    assert_eq!(export.id.crate_id, CrateId(1));

    let _ = fs::remove_dir_all(base);
}

#[test]
fn load_product_artifact_rejects_prelude_export_id_without_display_name() {
    let base = std::env::temp_dir().join(format!(
        "rock_product_loader_{}_{}",
        std::process::id(),
        "bad_id_backed_prelude_export"
    ));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let artifact_path = base.join("stdlib.rkca");
    let object_path = base.join("stdlib.o");
    fs::write(&object_path, []).unwrap();

    let product_id = ProductDefId {
        crate_id: ProductCrateId(1),
        local_id: crate::products::ProductLocalDefId(7),
    };
    let mut identity_table = ProductIdentityTable::default();
    identity_table.local_crate = Some(ProductCrateId(1));
    identity_table
        .prelude_export_names
        .insert("String".to_string(), product_id);

    let products = CompilerProducts {
        crate_identity: ProductCrateIdentity::local("stdlib".to_string()),
        identity_table,
        metadata: Default::default(),
        bodies: Default::default(),
        link: ProductLinkData {
            object_path: Some(object_path.clone()),
            records: Default::default(),
        },
        dependencies: Vec::new(),
        source_fingerprint: ProductSourceFingerprint::default(),
        prelude_exports: Default::default(),
        infix_precedence: Default::default(),
    };
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    let err = ctx.load_product_artifact_from_path(artifact_path).unwrap_err();

    assert!(
        err.contains("prelude export 'String'") && err.contains("display name"),
        "unexpected error: {}",
        err
    );

    let _ = fs::remove_dir_all(base);
}
```

- [ ] **Step 2: Run the failing artifact loader tests**

Run: `cargo test -p rock-lib load_product_artifact_records_id_backed_prelude_exports -- --nocapture`

Run: `cargo test -p rock-lib load_product_artifact_rejects_prelude_export_id_without_display_name -- --nocapture`

Expected: FAIL because loader normalization ignores `prelude_export_names`.

- [ ] **Step 3: Normalize ID-backed prelude exports before string fallback**

In `loaded_crate_from_products`, change the call to `normalize_product_prelude_exports` to pass `&products`, `remap`, and `&crate_name`:

```rust
let (prelude_exports, prelude_export_ids) = normalize_product_prelude_exports(
    &products,
    remap,
    &interface,
    &resolver,
    &crate_name,
    &format!("{}::prelude", crate_name),
)?;
```

Replace `normalize_product_prelude_exports` with this signature and ID-first logic:

```rust
fn normalize_product_prelude_exports(
    products: &CompilerProducts,
    remap: &ProductIdentityRemap,
    interface: &super::ArtifactCrateInterface,
    resolver: &ResolverTables,
    crate_name: &str,
    prelude_prefix: &str,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, ArtifactExport>), String> {
    let mut normalized = BTreeMap::new();
    let mut normalized_ids = BTreeMap::new();

    for (alias, id) in &products.identity_table.prelude_export_names {
        let display_name = product_display_name(products, *id).ok_or_else(|| {
            format!(
                "Product artifact prelude export '{}' has no display name or canonical DefId",
                alias
            )
        })?;
        let def_id = product_def_id_to_existing_def_id(products, remap, *id)?.ok_or_else(|| {
            format!(
                "Product artifact prelude export '{}' has no canonical DefId",
                alias
            )
        })?;
        let source = qualify_product_name(crate_name, &display_name);
        normalized.insert(alias.clone(), source.clone());
        normalized_ids.insert(alias.clone(), ArtifactExport { source, id: def_id });
    }

    for (alias, source) in &products.prelude_exports {
        if let Some(existing) = normalized_ids.get(alias) {
            if existing.source != *source {
                return Err(format!(
                    "Product artifact prelude export '{}' has conflicting string source '{}' and ID source '{}'",
                    alias, source, existing.source
                ));
            }
            continue;
        }

        let export = artifact_export_for_source(interface, resolver, source).ok_or_else(|| {
            format!(
                "Product artifact prelude export '{}' references '{}' without a canonical DefId",
                alias, source
            )
        })?;

        normalized.insert(alias.clone(), export.source.clone());
        normalized_ids.insert(alias.clone(), export);
    }

    for name in product_interface_prelude_names(interface, prelude_prefix) {
        if let Some(alias) = name.strip_prefix(&format!("{}::", prelude_prefix)) {
            if !normalized.contains_key(alias) {
                let export = artifact_export_for_source(interface, resolver, &name).ok_or_else(|| {
                    format!(
                        "Product artifact prelude export '{}' references '{}' without a canonical DefId",
                        alias, name
                    )
                })?;
                normalized.insert(alias.to_string(), export.source.clone());
                normalized_ids.insert(alias.to_string(), export);
            }
        }
    }

    Ok((normalized, normalized_ids))
}
```

- [ ] **Step 4: Run prelude artifact tests**

Run: `cargo test -p rock-lib load_product_artifact_records_id_backed_prelude_exports -- --nocapture`

Run: `cargo test -p rock-lib load_product_artifact_rejects_prelude_export_id_without_display_name -- --nocapture`

Run: `cargo test -p rock-lib load_product_artifact_derives_missing_prelude_exports_from_prelude_items -- --nocapture`

Run: `cargo test -p rock-lib load_product_artifact_rejects_prelude_export_without_canonical_id -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add lib/src/crate_artifact/load.rs
git commit -m "load prelude exports from product ids"
```

---

### Task 6: Update Trackers And Run Final Verification

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [ ] **Step 1: Update audit tracker bullets**

In `docs/superpowers/plans/master-audit-checklist.md`, add completed bullets under `Real Collection And Name Resolution`:

```markdown
- [x] Added resolver helper APIs so lowerer and artifact code can resolve item paths, import aliases, and export aliases through one ID-backed interface.
- [x] Routed lowerer owner, nominal, trait, and top-level alias lookups through resolver IDs before falling back to compatibility string maps.
- [x] Added ID-backed product prelude export persistence and artifact loading validation, matching the existing root-export ID path.
```

Keep the remaining unchecked bullets for full source/path resolution extraction, string map removal, generated identity audit, and broader lowerer decomposition.

- [ ] **Step 2: Update roadmap status notes**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, change Task 4 status to:

```markdown
**Status:** Complete for resolver alias helper APIs, lowerer ID-first alias consumption, and product prelude ID persistence. Full removal of compatibility string maps remains tracked in the audit checklist.
```

Add this status line under Task 5:

```markdown
**Status:** Started. Owner, nominal, trait, and top-level alias lookup now use a resolver facade; full source/path resolution extraction remains future work.
```

- [ ] **Step 3: Verify tracker formatting**

Run: `perl -ne 'if (/^Still to do:/) { $in=1; next } if (/^## /) { $in=0 } if ($in && /^- \[x\]/) { print "$ARGV:$.:$_"; $bad=1 } END { exit($bad ? 1 : 0) }' "docs/superpowers/plans/master-audit-checklist.md"`

Expected: no output and exit 0.

- [ ] **Step 4: Run format check**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 5: Run whitespace check**

Run: `git diff --check`

Expected: no output.

- [ ] **Step 6: Run focused resolver/lowerer/product tests**

Run: `cargo test -p rock-lib resolver_tables_ -- --nocapture`

Run: `cargo test -p rock-lib lowerer_resolves_current_and_dependency_aliases_through_facade -- --nocapture`

Run: `cargo test -p rock-lib def_id_for_name_uses_resolver_facade_for_dependencies -- --nocapture`

Run: `cargo test -p rock-lib lower_nominal_type_prefers_resolver_alias_id_over_suffix_match -- --nocapture`

Run: `cargo test -p rock-lib trait_by_name_prefers_resolver_alias_id -- --nocapture`

Run: `cargo test -p rock-lib compiler_products_record_prelude_export_ids -- --nocapture`

Run: `cargo test -p rock-lib compiler_products_roundtrip_preserves_prelude_export_ids -- --nocapture`

Run: `cargo test -p rock-lib load_product_artifact_records_id_backed_prelude_exports -- --nocapture`

Run: `cargo test -p rock-lib load_product_artifact_rejects_prelude_export_id_without_display_name -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run full library test suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 8: Check final status**

Run: `git status --short`

Expected: only intended docs/source/test changes are shown before the final commit.

- [ ] **Step 9: Commit tracker and final verification docs**

Run:

```bash
git add docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git commit -m "update audit tracker for resolver alias slice"
```

---

## Completion Definition

This plan is complete when:

- `ResolverTables` exposes ID-backed alias resolution helpers.
- Lowerer owner, nominal, trait, and top-level alias lookups use resolver IDs before compatibility strings.
- Product artifacts persist prelude exports by product `DefId` and load them through remapped `DefId`s before string fallback.
- Compatibility string maps remain only as explicitly documented temporary views.
- Audit roadmap/tracker docs identify completed and remaining work accurately.
- Final verification commands pass.
