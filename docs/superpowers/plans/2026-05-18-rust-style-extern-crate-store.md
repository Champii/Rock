# Rust-Style Extern Crate Store Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace phase-facing mixed `LoadedCrate` dependency storage with a Rust-style split between current source input and an artifact-only extern crate store.

**Architecture:** Add `CurrentCrateSource`, `ExternCrateStore`, `ExternCrateRecord`, `ExternCrateMetadata`, `ExternCrateBodies`, and `ExternCrateLink` under `crate_system`. Product artifact loading builds artifact-only extern records keyed by session `CrateId`; collect, lower, mono, and codegen query metadata/body/link capabilities instead of `LoadedCrate`; `LoadedCrate` and `ArtifactMode` are deleted by the final cleanup task.

**Tech Stack:** Rust 2021, `rock-lib`, product artifacts, `CrateContext`, session `CrateId`, `ResolverTables`, HIR bodies, monomorphization instance registry, LLVM object linking, `cargo test`.

---

## Source Spec

This plan implements `docs/superpowers/specs/2026-05-18-rust-style-extern-crate-store-design.md`.

The implementation intentionally uses one temporary staging point: product artifact loading may dual-write the existing `LoadedCrate` map and the new `ExternCrateStore` while phase call sites are migrated. That temporary state is removed in Task 7. The final architecture must not keep `LoadedCrate` as dependency storage.

## File Structure

- Create: `lib/src/crate_system/extern_store.rs`
  - Owns `CurrentCrateSource`, `ExternCrateStore`, `ExternCrateRecord`, `ExternCrateRef`, `ExternCrateMetadata`, `ExternCrateBodies`, `ExternCrateLink`, `ExternCrateLinkage`, and `DependencyLinkInputs`.
  - Keeps artifact dependency records AST-free by construction.
- Modify: `lib/src/crate_system/mod.rs`
  - Exports the new crate-system records.
  - Keeps `LoadedCrate` and `ArtifactMode` only until Task 7.
- Modify: `lib/src/crate_system/context.rs`
  - Adds `source_crates` and `extern_crates` storage to `CrateContext`.
  - Adds source and extern accessors.
  - Moves link-input aggregation onto `ExternCrateStore`.
- Modify: `lib/src/crate_system/tests.rs`
  - Adds store/source/link capability tests and later removes `LoadedCrate` tests.
- Modify: `lib/src/crate_artifact/load.rs`
  - Builds `ExternCrateRecord` values from product artifacts after validation/remapping.
  - Migrates dependency-definition validators from `ctx.crates` to `ctx.extern_crates()`.
  - Updates artifact-load tests to inspect extern capabilities.
- Modify: `lib/src/crate_artifact/tests.rs`
  - Updates direct `ctx.crates` assertions to extern-store capability assertions.
- Modify: `lib/src/collect/context.rs`
  - Replaces `register_loaded_crate` with metadata/link capability registration.
- Modify: `lib/src/collect/mod.rs`
  - Uses `CurrentCrateSource` for source artifact declaration collection.
  - Iterates `ctx.extern_crates()` for dependency registration.
  - Updates collect tests away from `LoadedCrate` construction.
- Modify: `lib/src/lower/crates/registration.rs`
  - Registers dependency resolvers and declarations from `ExternCrateMetadata` and `ExternCrateLink`.
- Modify: `lib/src/lower/crates/bodies.rs`
  - Imports generic functions, generic impls, and trait defaults through `ExternCrateBodies` accessors.
- Modify: `lib/src/lower/program.rs`
  - Uses `ctx.has_extern_crate("stdlib")` for prelude injection decisions.
- Modify: `lib/src/mono/external.rs`
  - Iterates `ctx.extern_crates()` and consumes body/link capabilities.
  - Updates mono tests away from `LoadedCrate` construction.
- Modify: `lib/src/lib.rs`
  - Keeps codegen link setup on `crate_ctx.dependency_link_inputs()`, backed by the extern store.
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Updates Tasks 6-7 status after implementation.
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
  - Updates the Crate And Artifact Interface Split evidence and remaining gaps.

---

### Task 1: Add Artifact-Only Extern Store Types

**Files:**
- Create: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/crate_system/mod.rs`
- Modify: `lib/src/crate_system/tests.rs`

- [ ] **Step 1: Write failing extern-store tests**

Add these imports to `lib/src/crate_system/tests.rs`:

```rust
use crate::crate_artifact::{ArtifactCrateInterface, ArtifactCrossCrateHir, ArtifactExport};
```

Add this helper below the existing `hir_function` helper:

```rust
fn hir_function_with_id(
    id: DefId,
    name: &str,
    qualified_name: Option<&str>,
    generic_params: &[&str],
) -> HirFunction {
    let generic_param_ids = generic_params
        .iter()
        .enumerate()
        .map(|(index, _)| crate::types::GenericParamId {
            owner: id,
            index: index as u32,
        })
        .collect();

    HirFunction {
        id,
        name: name.to_string(),
        qualified_name: qualified_name.map(str::to_string),
        generic_params: generic_params
            .iter()
            .map(|param| param.to_string())
            .collect(),
        generic_param_ids,
        generic_bounds: HashMap::new(),
        params: Vec::new(),
        ret_type: Type::I64,
        body: HirBlock {
            stmts: Vec::new(),
            ty: Type::I64,
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    }
}
```

Add these tests near the existing provider tests:

```rust
#[test]
fn extern_crate_store_records_artifact_capabilities_by_session_crate_id() {
    let crate_id = CrateId(42);
    let answer_id = DefId::new(crate_id, LocalDefId(7));
    let mut interface = ArtifactCrateInterface::default();
    interface.functions.insert(
        "dep::answer".to_string(),
        hir_function_with_id(answer_id, "answer", Some("dep::answer"), &[]),
    );
    interface.root_exports.insert(
        "answer".to_string(),
        "dep::answer".to_string(),
    );
    interface.root_export_ids.insert(
        "answer".to_string(),
        ArtifactExport {
            source: "dep::answer".to_string(),
            id: answer_id,
        },
    );

    let mut resolver = ResolverTables::default();
    resolver
        .item_paths
        .insert("dep::answer".to_string(), answer_id);
    resolver
        .item_names_by_id
        .insert(answer_id, "dep::answer".to_string());

    let record = ExternCrateRecord::new(
        crate_id,
        "dep".to_string(),
        ExternCrateMetadata::new(
            interface,
            resolver,
            BTreeMap::from([("pipe".to_string(), "dep::prelude::pipe".to_string())]),
            BTreeMap::new(),
        ),
        ExternCrateBodies::default(),
        ExternCrateLink::object(
            PathBuf::from("/dep/dep.o"),
            BTreeMap::from([(answer_id, "dep_answer".to_string())]),
        ),
    );
    let mut store = ExternCrateStore::default();
    store.insert(record).expect("extern record should insert");

    let dep = store.by_name("dep").expect("dep extern crate should exist");
    assert_eq!(dep.crate_id(), crate_id);
    assert!(dep.metadata().interface().functions.contains_key("dep::answer"));
    assert_eq!(
        dep.metadata().prelude_exports().get("pipe"),
        Some(&"dep::prelude::pipe".to_string())
    );
    assert_eq!(dep.link().object_path(), Some(&PathBuf::from("/dep/dep.o")));
    assert_eq!(dep.link().backend_symbol(answer_id), Some("dep_answer"));
}

#[test]
fn extern_crate_store_rejects_duplicate_names_and_ids() {
    let mut store = ExternCrateStore::default();
    let first = ExternCrateRecord::new(
        CrateId(1),
        "dep".to_string(),
        ExternCrateMetadata::empty_for_test(),
        ExternCrateBodies::default(),
        ExternCrateLink::metadata_only(BTreeMap::new()),
    );
    store.insert(first).expect("first extern crate should insert");

    let duplicate_name = ExternCrateRecord::new(
        CrateId(2),
        "dep".to_string(),
        ExternCrateMetadata::empty_for_test(),
        ExternCrateBodies::default(),
        ExternCrateLink::metadata_only(BTreeMap::new()),
    );
    let name_error = store
        .insert(duplicate_name)
        .expect_err("duplicate extern crate name should fail");
    assert!(name_error.contains("duplicate external crate name 'dep'"));

    let duplicate_id = ExternCrateRecord::new(
        CrateId(1),
        "other".to_string(),
        ExternCrateMetadata::empty_for_test(),
        ExternCrateBodies::default(),
        ExternCrateLink::metadata_only(BTreeMap::new()),
    );
    let id_error = store
        .insert(duplicate_id)
        .expect_err("duplicate extern crate id should fail");
    assert!(id_error.contains("duplicate external crate id 1"));
}

#[test]
fn extern_crate_bodies_expose_categories_without_raw_bundle_access() {
    let id = DefId::new(CrateId(3), LocalDefId(1));
    let mut bundle = ArtifactCrossCrateHir {
        generic_functions: BTreeMap::new(),
        traits_with_defaults: BTreeMap::new(),
        generic_impls: Vec::new(),
    };
    bundle.generic_functions.insert(
        "dep::id".to_string(),
        hir_function_with_id(id, "id", Some("dep::id"), &["T"]),
    );

    let bodies = ExternCrateBodies::from_cross_crate_hir(bundle);

    assert!(bodies.generic_functions().contains_key("dep::id"));
    assert!(bodies.traits_with_defaults().is_empty());
    assert!(bodies.generic_impls().is_empty());
}
```

- [ ] **Step 2: Run extern-store tests red**

Run:

```bash
cargo test -p rock-lib crate_system::tests::extern_crate_store_records_artifact_capabilities_by_session_crate_id -- --exact
cargo test -p rock-lib crate_system::tests::extern_crate_store_rejects_duplicate_names_and_ids -- --exact
cargo test -p rock-lib crate_system::tests::extern_crate_bodies_expose_categories_without_raw_bundle_access -- --exact
```

Expected: FAIL to compile because `ExternCrateStore`, `ExternCrateRecord`, `ExternCrateMetadata`, `ExternCrateBodies`, and `ExternCrateLink` do not exist yet.

- [ ] **Step 3: Add the extern store implementation**

Create `lib/src/crate_system/extern_store.rs`:

```rust
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::ast::Module;
use crate::collect::resolver::ResolverTables;
use crate::crate_artifact::{ArtifactCrateInterface, ArtifactCrossCrateHir, ArtifactExport};
use crate::hir::{HirFunction, HirImpl, HirTrait};
use crate::ids::{CrateId, DefId};

use super::{CrateManifest, ModuleTree};

#[derive(Debug)]
pub(crate) struct CurrentCrateSource {
    pub manifest: CrateManifest,
    pub root_dir: PathBuf,
    pub ast: Module,
    pub module_tree: Option<ModuleTree>,
    pub file_cache: HashMap<PathBuf, Module>,
}

impl CurrentCrateSource {
    pub(crate) fn new(manifest: CrateManifest, root_dir: PathBuf, ast: Module) -> Self {
        Self {
            manifest,
            root_dir,
            ast,
            module_tree: None,
            file_cache: HashMap::new(),
        }
    }

    pub(crate) fn root_module_path(&self) -> PathBuf {
        self.ast
            .filepath
            .clone()
            .unwrap_or_else(|| self.root_dir.join(&self.manifest.lib.path))
    }

    pub(crate) fn lib_path(&self) -> PathBuf {
        self.root_dir.join(&self.manifest.lib.path)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExternCrateMetadata {
    interface: ArtifactCrateInterface,
    resolver: ResolverTables,
    prelude_exports: BTreeMap<String, String>,
    prelude_export_ids: BTreeMap<String, ArtifactExport>,
}

impl ExternCrateMetadata {
    pub(crate) fn new(
        interface: ArtifactCrateInterface,
        resolver: ResolverTables,
        prelude_exports: BTreeMap<String, String>,
        prelude_export_ids: BTreeMap<String, ArtifactExport>,
    ) -> Self {
        Self {
            interface,
            resolver,
            prelude_exports,
            prelude_export_ids,
        }
    }

    #[cfg(test)]
    pub(crate) fn empty_for_test() -> Self {
        Self::new(
            ArtifactCrateInterface::default(),
            ResolverTables::default(),
            BTreeMap::new(),
            BTreeMap::new(),
        )
    }

    pub(crate) fn interface(&self) -> &ArtifactCrateInterface {
        &self.interface
    }

    pub(crate) fn resolver(&self) -> &ResolverTables {
        &self.resolver
    }

    pub(crate) fn prelude_exports(&self) -> &BTreeMap<String, String> {
        &self.prelude_exports
    }

    pub(crate) fn prelude_export_ids(&self) -> &BTreeMap<String, ArtifactExport> {
        &self.prelude_export_ids
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExternCrateBodies {
    generic_functions: BTreeMap<String, HirFunction>,
    traits_with_defaults: BTreeMap<String, HirTrait>,
    generic_impls: Vec<HirImpl>,
}

impl ExternCrateBodies {
    pub(crate) fn from_cross_crate_hir(bundle: ArtifactCrossCrateHir) -> Self {
        Self {
            generic_functions: bundle.generic_functions,
            traits_with_defaults: bundle.traits_with_defaults,
            generic_impls: bundle.generic_impls,
        }
    }

    pub(crate) fn generic_functions(&self) -> &BTreeMap<String, HirFunction> {
        &self.generic_functions
    }

    pub(crate) fn traits_with_defaults(&self) -> &BTreeMap<String, HirTrait> {
        &self.traits_with_defaults
    }

    pub(crate) fn generic_impls(&self) -> &[HirImpl] {
        &self.generic_impls
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.generic_functions.is_empty()
            && self.traits_with_defaults.is_empty()
            && self.generic_impls.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExternCrateLinkage {
    Object { object_path: PathBuf },
    MetadataOnly,
}

#[derive(Debug, Clone)]
pub(crate) struct ExternCrateLink {
    linkage: ExternCrateLinkage,
    backend_symbols: BTreeMap<DefId, String>,
}

impl ExternCrateLink {
    pub(crate) fn object(
        object_path: PathBuf,
        backend_symbols: BTreeMap<DefId, String>,
    ) -> Self {
        Self {
            linkage: ExternCrateLinkage::Object { object_path },
            backend_symbols,
        }
    }

    pub(crate) fn metadata_only(backend_symbols: BTreeMap<DefId, String>) -> Self {
        Self {
            linkage: ExternCrateLinkage::MetadataOnly,
            backend_symbols,
        }
    }

    pub(crate) fn is_object_backed(&self) -> bool {
        matches!(self.linkage, ExternCrateLinkage::Object { .. })
    }

    pub(crate) fn object_path(&self) -> Option<&PathBuf> {
        match &self.linkage {
            ExternCrateLinkage::Object { object_path } => Some(object_path),
            ExternCrateLinkage::MetadataOnly => None,
        }
    }

    pub(crate) fn backend_symbol(&self, id: DefId) -> Option<&str> {
        self.backend_symbols.get(&id).map(String::as_str)
    }

    pub(crate) fn concrete_impl_body_is_object_provided(&self, imp: &HirImpl) -> bool {
        self.is_object_backed()
            && imp.type_generics.is_empty()
            && imp.trait_generics.is_empty()
            && imp
                .methods
                .values()
                .all(|method| method.generic_params.is_empty())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExternCrateRecord {
    crate_id: CrateId,
    name: String,
    metadata: ExternCrateMetadata,
    bodies: ExternCrateBodies,
    link: ExternCrateLink,
}

impl ExternCrateRecord {
    pub(crate) fn new(
        crate_id: CrateId,
        name: String,
        metadata: ExternCrateMetadata,
        bodies: ExternCrateBodies,
        link: ExternCrateLink,
    ) -> Self {
        Self {
            crate_id,
            name,
            metadata,
            bodies,
            link,
        }
    }

    pub(crate) fn crate_id(&self) -> CrateId {
        self.crate_id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn metadata(&self) -> &ExternCrateMetadata {
        &self.metadata
    }

    pub(crate) fn bodies(&self) -> &ExternCrateBodies {
        &self.bodies
    }

    pub(crate) fn link(&self) -> &ExternCrateLink {
        &self.link
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExternCrateRef<'a> {
    crate_id: CrateId,
    name: &'a str,
    record: &'a ExternCrateRecord,
}

impl<'a> ExternCrateRef<'a> {
    fn new(record: &'a ExternCrateRecord) -> Self {
        Self {
            crate_id: record.crate_id(),
            name: record.name(),
            record,
        }
    }

    pub(crate) fn crate_id(&self) -> CrateId {
        self.crate_id
    }

    pub(crate) fn name(&self) -> &'a str {
        self.name
    }

    pub(crate) fn metadata(&self) -> &'a ExternCrateMetadata {
        self.record.metadata()
    }

    pub(crate) fn bodies(&self) -> &'a ExternCrateBodies {
        self.record.bodies()
    }

    pub(crate) fn link(&self) -> &'a ExternCrateLink {
        self.record.link()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencyLinkInputs {
    pub object_crate_names: Vec<String>,
    pub object_paths: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub(crate) struct ExternCrateStore {
    records: BTreeMap<CrateId, ExternCrateRecord>,
    names: BTreeMap<String, CrateId>,
}

impl ExternCrateStore {
    pub(crate) fn insert(&mut self, record: ExternCrateRecord) -> Result<(), String> {
        if self.records.contains_key(&record.crate_id()) {
            return Err(format!(
                "duplicate external crate id {} for '{}'",
                record.crate_id().0,
                record.name()
            ));
        }
        if self.names.contains_key(record.name()) {
            return Err(format!("duplicate external crate name '{}'", record.name()));
        }

        let crate_id = record.crate_id();
        let name = record.name().to_string();
        self.records.insert(crate_id, record);
        self.names.insert(name, crate_id);
        Ok(())
    }

    pub(crate) fn by_name(&self, name: &str) -> Option<ExternCrateRef<'_>> {
        self.names
            .get(name)
            .and_then(|crate_id| self.records.get(crate_id))
            .map(ExternCrateRef::new)
    }

    pub(crate) fn by_crate_id(&self, crate_id: CrateId) -> Option<ExternCrateRef<'_>> {
        self.records.get(&crate_id).map(ExternCrateRef::new)
    }

    pub(crate) fn contains_name(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = ExternCrateRef<'_>> + '_ {
        self.names
            .values()
            .filter_map(|crate_id| self.records.get(crate_id))
            .map(ExternCrateRef::new)
    }

    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn link_inputs(&self) -> DependencyLinkInputs {
        let mut object_crate_names = Vec::new();
        let mut object_paths = Vec::new();

        for dep in self.iter() {
            if let Some(path) = dep.link().object_path() {
                object_crate_names.push(dep.name().to_string());
                object_paths.push(path.clone());
            }
        }

        DependencyLinkInputs {
            object_crate_names,
            object_paths,
        }
    }
}
```

- [ ] **Step 4: Export the new types from `crate_system`**

In `lib/src/crate_system/mod.rs`, add the module:

```rust
mod extern_store;
```

Add these re-exports after `pub use rock_shared::manifest::{CrateConfig, CrateManifest, Dependency, LibConfig};`:

```rust
pub(crate) use extern_store::{
    CurrentCrateSource, DependencyLinkInputs, ExternCrateBodies, ExternCrateLink,
    ExternCrateLinkage, ExternCrateMetadata, ExternCrateRecord, ExternCrateRef,
    ExternCrateStore,
};
```

- [ ] **Step 5: Run extern-store tests green**

Run:

```bash
cargo test -p rock-lib crate_system::tests::extern_crate_store_records_artifact_capabilities_by_session_crate_id -- --exact
cargo test -p rock-lib crate_system::tests::extern_crate_store_rejects_duplicate_names_and_ids -- --exact
cargo test -p rock-lib crate_system::tests::extern_crate_bodies_expose_categories_without_raw_bundle_access -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit Task 1**

Run:

```bash
git add lib/src/crate_system/extern_store.rs lib/src/crate_system/mod.rs lib/src/crate_system/tests.rs
git commit -m "add extern crate store types"
```

---

### Task 2: Add Source/Extern Storage To `CrateContext`

**Files:**
- Modify: `lib/src/crate_system/context.rs`
- Modify: `lib/src/crate_system/mod.rs`
- Modify: `lib/src/crate_system/tests.rs`

- [ ] **Step 1: Write failing `CrateContext` storage tests**

Add these tests to `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn crate_context_registers_source_crates_outside_extern_store() {
    let mut ctx = CrateContext::new();
    ctx.register_crate(empty_manifest("dep"), PathBuf::from("/dep"), empty_module());

    assert!(ctx.has_crate("dep"));
    assert!(ctx.has_source_crate("dep"));
    assert!(!ctx.has_extern_crate("dep"));
    assert!(ctx.source_crate("dep").is_some());
    assert!(ctx.extern_crate("dep").is_none());
}

#[test]
fn crate_context_adds_artifact_records_to_extern_store() {
    let mut ctx = CrateContext::new();
    let crate_id = CrateId(5);
    ctx.add_extern_crate(ExternCrateRecord::new(
        crate_id,
        "dep".to_string(),
        ExternCrateMetadata::empty_for_test(),
        ExternCrateBodies::default(),
        ExternCrateLink::metadata_only(BTreeMap::new()),
    ))
    .expect("extern crate should insert");

    assert!(ctx.has_crate("dep"));
    assert!(!ctx.has_source_crate("dep"));
    assert!(ctx.has_extern_crate("dep"));
    assert_eq!(ctx.extern_crate("dep").unwrap().crate_id(), crate_id);
    assert_eq!(ctx.extern_crates().count(), 1);
}
```

- [ ] **Step 2: Run context storage tests red**

Run:

```bash
cargo test -p rock-lib crate_system::tests::crate_context_registers_source_crates_outside_extern_store -- --exact
cargo test -p rock-lib crate_system::tests::crate_context_adds_artifact_records_to_extern_store -- --exact
```

Expected: FAIL to compile because `CrateContext` does not have source/extern store accessors.

- [ ] **Step 3: Extend `CrateContext` fields**

In `lib/src/crate_system/mod.rs`, replace `CrateContext` with this staged shape:

```rust
#[derive(Debug)]
pub struct CrateContext {
    /// Temporary compatibility map removed in Task 7.
    pub crates: BTreeMap<String, LoadedCrate>,
    source_crates: BTreeMap<String, CurrentCrateSource>,
    extern_crates: ExternCrateStore,
    pub(crate) product_crate_ids: BTreeMap<ProductCrateIdentity, crate::ids::CrateId>,
    next_product_crate_id: u32,
}
```

- [ ] **Step 4: Initialize and expose source/extern storage**

In `lib/src/crate_system/context.rs`, update imports:

```rust
use super::{
    ArtifactMode, CrateContext, CrateManifest, CurrentCrateSource, ExternCrateRecord,
    ExternCrateRef, LoadedCrate,
};
```

Update `CrateContext::new`:

```rust
pub fn new() -> Self {
    Self {
        crates: BTreeMap::new(),
        source_crates: BTreeMap::new(),
        extern_crates: Default::default(),
        product_crate_ids: BTreeMap::new(),
        next_product_crate_id: 1,
    }
}
```

At the start of `register_crate`, create a source record and store it:

```rust
pub fn register_crate(&mut self, manifest: CrateManifest, source_dir: PathBuf, ast: Module) {
    let name = manifest.crate_.name.clone();
    self.source_crates.insert(
        name.clone(),
        CurrentCrateSource::new(manifest.clone(), source_dir.clone(), ast.clone()),
    );

    let loaded_crate = LoadedCrate {
        manifest,
        ast,
        interface: None,
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        prelude_export_ids: BTreeMap::new(),
        object_path: None,
        backend_symbols: BTreeMap::new(),
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::Source,
        root_dir: source_dir,
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };
    self.crates.insert(name, loaded_crate);
}
```

In `load_crate_from_dir`, after `file_cache` has been collected and before creating the temporary `LoadedCrate`, add:

```rust
let mut source = CurrentCrateSource::new(manifest.clone(), crate_dir.clone(), ast.clone());
source.file_cache = file_cache.clone();
self.source_crates.insert(name.clone(), source);
```

Add these methods to `impl CrateContext`:

```rust
pub(crate) fn add_extern_crate(&mut self, record: ExternCrateRecord) -> Result<(), String> {
    self.extern_crates.insert(record)
}

pub(crate) fn extern_crate(&self, name: &str) -> Option<ExternCrateRef<'_>> {
    self.extern_crates.by_name(name)
}

pub(crate) fn extern_crates(&self) -> impl Iterator<Item = ExternCrateRef<'_>> + '_ {
    self.extern_crates.iter()
}

pub(crate) fn has_extern_crate(&self, name: &str) -> bool {
    self.extern_crates.contains_name(name)
}

pub(crate) fn source_crate(&self, name: &str) -> Option<&CurrentCrateSource> {
    self.source_crates.get(name)
}

pub(crate) fn source_crate_mut(&mut self, name: &str) -> Option<&mut CurrentCrateSource> {
    self.source_crates.get_mut(name)
}

pub(crate) fn has_source_crate(&self, name: &str) -> bool {
    self.source_crates.contains_key(name)
}
```

Change `has_crate` to include both stores:

```rust
pub fn has_crate(&self, name: &str) -> bool {
    self.has_source_crate(name) || self.has_extern_crate(name) || self.crates.contains_key(name)
}
```

Change `crate_count` to count names across both stores:

```rust
pub fn crate_count(&self) -> usize {
    let mut names = std::collections::BTreeSet::new();
    names.extend(self.source_crates.keys().cloned());
    names.extend(self.extern_crates().map(|dep| dep.name().to_string()));
    names.extend(self.crates.keys().cloned());
    names.len()
}
```

Keep `add_crate` unchanged until Task 7 so existing tests and call sites compile.

- [ ] **Step 5: Route link input aggregation through the extern store and update tests**

In `lib/src/crate_system/context.rs`, replace `dependency_link_inputs` with:

```rust
pub(crate) fn dependency_link_inputs(
    &self,
    _phase: &str,
) -> Result<DependencyLinkInputs, String> {
    Ok(self.extern_crates.link_inputs())
}
```

Remove the local `DependencyLinkInputs` struct from `context.rs`; Task 1 moved it to `extern_store.rs`.

In `lib/src/crate_system/tests.rs`, replace `crate_context_collects_dependency_link_inputs_from_provider_capabilities` with:

```rust
#[test]
fn crate_context_collects_dependency_link_inputs_from_provider_capabilities() {
    let mut ctx = CrateContext::new();
    ctx.add_extern_crate(ExternCrateRecord::new(
        CrateId(9),
        "dep".to_string(),
        ExternCrateMetadata::empty_for_test(),
        ExternCrateBodies::default(),
        ExternCrateLink::object(PathBuf::from("/dep/dep.o"), BTreeMap::new()),
    ))
    .expect("extern crate should insert");

    let inputs = ctx
        .dependency_link_inputs("codegen")
        .expect("object-backed dependency should expose link inputs");

    assert_eq!(inputs.object_crate_names, vec!["dep".to_string()]);
    assert_eq!(inputs.object_paths, vec![PathBuf::from("/dep/dep.o")]);
}
```

Replace `crate_context_link_inputs_reject_object_backed_dependencies_without_object_path` with this type-level invariant test:

```rust
#[test]
fn extern_crate_link_makes_object_path_required_for_object_backed_dependencies() {
    let link = ExternCrateLink::object(PathBuf::from("/dep/dep.o"), BTreeMap::new());

    assert!(link.is_object_backed());
    assert_eq!(link.object_path(), Some(&PathBuf::from("/dep/dep.o")));
}
```

Remove `crate_context_link_inputs_reject_source_backed_dependencies`; source crates are not extern records and are covered by `crate_context_registers_source_crates_outside_extern_store`.

- [ ] **Step 6: Run context tests green**

Run:

```bash
cargo test -p rock-lib crate_system::tests::crate_context_registers_source_crates_outside_extern_store -- --exact
cargo test -p rock-lib crate_system::tests::crate_context_adds_artifact_records_to_extern_store -- --exact
cargo test -p rock-lib crate_system::tests::crate_context_collects_dependency_link_inputs_from_provider_capabilities -- --exact
cargo test -p rock-lib crate_system::tests::extern_crate_link_makes_object_path_required_for_object_backed_dependencies -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 2**

Run:

```bash
git add lib/src/crate_system/mod.rs lib/src/crate_system/context.rs lib/src/crate_system/tests.rs
git commit -m "split crate context source and extern storage"
```

---

### Task 3: Build Extern Records From Product Artifacts

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_system/tests.rs`

- [ ] **Step 1: Write failing product-load extern-store test**

Add this assertion pattern to the existing `load_product_artifact_*` test group in `lib/src/crate_artifact/load.rs` by creating a focused test near the tests that inspect `loaded.interface`:

```rust
#[test]
fn load_product_artifact_inserts_artifact_only_extern_record() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path();
    let object_path = base.join("dep.o");
    std::fs::write(&object_path, []).unwrap();
    let products = product_with_function("dep", ProductCrateId(0), 0, object_path.clone());
    let artifact_path = base.join("dep.rkca");
    products.write_artifact_to_path(&artifact_path).unwrap();

    let mut ctx = CrateContext::new();
    ctx.load_product_artifact_from_path(artifact_path).unwrap();

    let dep = ctx.extern_crate("dep").expect("extern record should be inserted");
    assert!(dep.metadata().interface().functions.contains_key("dep::answer"));
    assert!(dep.bodies().generic_functions().contains_key("dep::answer"));
    assert_eq!(dep.link().object_path(), Some(&object_path.canonicalize().unwrap()));
}
```

- [ ] **Step 2: Run product-load extern-store test red**

Run:

```bash
cargo test -p rock-lib crate_artifact::load::tests::load_product_artifact_inserts_artifact_only_extern_record -- --exact
```

Expected: FAIL because product artifact loading does not insert into `ExternCrateStore` yet.

- [ ] **Step 3: Add an extern-record constructor for product artifacts**

In `lib/src/crate_artifact/load.rs`, update the crate-system import:

```rust
use crate::crate_system::{
    ArtifactMode, CrateConfig, CrateContext, CrateManifest, ExternCrateBodies, ExternCrateLink,
    ExternCrateMetadata, ExternCrateRecord, LibConfig, LoadedCrate,
};
```

Add this function below `loaded_crate_from_products`:

```rust
fn extern_crate_from_products(
    products: &CompilerProducts,
    artifact_path: &Path,
    remap: &ProductIdentityRemap,
    ctx: &CrateContext,
) -> Result<ExternCrateRecord, String> {
    let crate_name = products.crate_identity.name.clone();
    let crate_id = remap
        .crate_ids
        .get(&remap.local_crate)
        .copied()
        .ok_or_else(|| {
            format!(
                "Product artifact for crate '{}' has no remapped local crate ID",
                crate_name
            )
        })?;
    let object_path = resolve_product_object_path(products, artifact_path)?;
    let backend_symbols = backend_symbols_from_products(products, remap)?;
    let interface = interface_from_products(products, &crate_name, remap, ctx)?;
    let resolver = resolver_from_products(products, &crate_name, remap)?;
    let cross_crate_hir = cross_crate_hir_from_products(products, &crate_name, remap, ctx)?;
    let (prelude_exports, prelude_export_ids) = normalize_product_prelude_exports(
        products,
        remap,
        &interface,
        &resolver,
        &crate_name,
        &format!("{}::prelude", crate_name),
    )?;

    Ok(ExternCrateRecord::new(
        crate_id,
        crate_name,
        ExternCrateMetadata::new(interface, resolver, prelude_exports, prelude_export_ids),
        ExternCrateBodies::from_cross_crate_hir(cross_crate_hir),
        ExternCrateLink::object(object_path, backend_symbols),
    ))
}
```

- [ ] **Step 4: Dual-write loaded product artifacts during migration**

In `load_product_artifact_from_path`, replace the body after remap creation with:

```rust
let products = CompilerProducts::read_artifact_from_path(&artifact_path)?;
let remap = ProductIdentityRemap::from_products(self, &products)?;
let extern_record = extern_crate_from_products(&products, &artifact_path, &remap, self)?;
let loaded_crate = loaded_crate_from_products(products, artifact_path, &remap, self)?;
let crate_name = loaded_crate.manifest.crate_.name.clone();
self.add_extern_crate(extern_record)?;
self.crates.insert(crate_name, loaded_crate);
```

In `load_product_artifact_from_path_as`, replace the body after the expected-name check with:

```rust
let remap = ProductIdentityRemap::from_products(self, &products)?;
let extern_record = extern_crate_from_products(&products, &artifact_path, &remap, self)?;
let loaded_crate = loaded_crate_from_products(products, artifact_path, &remap, self)?;
self.add_extern_crate(extern_record)?;
self.crates.insert(crate_name, loaded_crate);
```

This task intentionally keeps `loaded_crate_from_products` until all old call sites are migrated.

- [ ] **Step 5: Confirm link-input tests still use extern records**

Run:

```bash
rg "crate_context_collects_dependency_link_inputs_from_provider_capabilities|LoadedCrate|add_crate" lib/src/crate_system/tests.rs
```

Expected: the `crate_context_collects_dependency_link_inputs_from_provider_capabilities` test body contains `ctx.add_extern_crate(ExternCrateRecord::new(` and does not contain `LoadedCrate` or `add_crate`.

- [ ] **Step 6: Run product-load and link-input tests green**

Run:

```bash
cargo test -p rock-lib crate_artifact::load::tests::load_product_artifact_inserts_artifact_only_extern_record -- --exact
cargo test -p rock-lib crate_system::tests::crate_context_collects_dependency_link_inputs_from_provider_capabilities -- --exact
cargo test -p rock-lib crate_system::tests::extern_crate_link_makes_object_path_required_for_object_backed_dependencies -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 3**

Run:

```bash
git add lib/src/crate_artifact/load.rs lib/src/crate_system/tests.rs
git commit -m "load product artifacts into extern crate store"
```

---

### Task 4: Route Collection Through Extern Metadata And Link Capabilities

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/mod.rs` tests

- [ ] **Step 1: Write failing collect extern-store test**

In `lib/src/collect/mod.rs`, add this test near existing artifact dependency registration tests:

```rust
#[test]
fn collect_registers_artifact_dependency_from_extern_store() {
    let mut crate_ctx = CrateContext::new();
    let id = DefId::new(CrateId(7), LocalDefId(1));
    let mut interface = crate::crate_artifact::ArtifactCrateInterface::default();
    interface.functions.insert("dep::answer".to_string(), hir_function("answer"));
    interface
        .root_exports
        .insert("answer".to_string(), "dep::answer".to_string());
    let mut resolver = crate::collect::resolver::ResolverTables::default();
    resolver.item_paths.insert("dep::answer".to_string(), id);
    resolver.item_names_by_id.insert(id, "dep::answer".to_string());

    crate_ctx
        .add_extern_crate(crate::crate_system::ExternCrateRecord::new(
            CrateId(7),
            "dep".to_string(),
            crate::crate_system::ExternCrateMetadata::new(
                interface,
                resolver,
                BTreeMap::new(),
                BTreeMap::new(),
            ),
            crate::crate_system::ExternCrateBodies::default(),
            crate::crate_system::ExternCrateLink::metadata_only(BTreeMap::new()),
        ))
        .unwrap();

    let mut context = context::CollectContext::bootstrap_for_collection(false, Some("test"), None);
    context.register_crate_functions(&crate_ctx);

    assert!(context.functions.contains_key("dep::answer"));
    assert_eq!(
        context
            .artifact_root_exports
            .get("dep")
            .and_then(|exports| exports.get("answer")),
        Some(&"dep::answer".to_string())
    );
}
```

- [ ] **Step 2: Run collect extern-store test red or old-green**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_registers_artifact_dependency_from_extern_store -- --exact
```

Expected: FAIL before implementation if `register_crate_functions` still iterates only `ctx.crates`. If it passes because a previous task already routed the method, continue and use the grep gate in Step 6 as the refactor signal.

- [ ] **Step 3: Change collect registration signatures**

In `lib/src/collect/context.rs`, update imports:

```rust
use crate::crate_system::{CrateContext, ExternCrateLink, ExternCrateMetadata};
```

Replace `register_crate_functions` and `register_loaded_crate` with:

```rust
pub(crate) fn register_crate_functions(&mut self, ctx: &CrateContext) {
    for dep in ctx.extern_crates() {
        self.register_extern_crate(dep.name(), dep.metadata(), dep.link());
    }
}

pub(crate) fn register_extern_crate(
    &mut self,
    crate_name: &str,
    metadata: &ExternCrateMetadata,
    link: &ExternCrateLink,
) {
    let is_stdlib = crate_name == "stdlib";
    let interface = metadata.interface();

    if !interface.root_exports.is_empty() {
        self.artifact_root_exports.insert(
            crate_name.to_string(),
            interface
                .root_exports
                .iter()
                .map(|(name, source)| (name.clone(), source.clone()))
                .collect(),
        );
    }

    if !interface.root_export_ids.is_empty() {
        self.artifact_root_export_ids.insert(
            crate_name.to_string(),
            interface
                .root_export_ids
                .iter()
                .map(|(name, export)| (name.clone(), export.clone()))
                .collect(),
        );
    }

    if is_stdlib && !metadata.prelude_exports().is_empty() {
        self.stdlib_prelude_exports = metadata
            .prelude_exports()
            .iter()
            .map(|(name, source)| (name.clone(), Some(source.clone())))
            .collect();
    }

    if is_stdlib && !metadata.prelude_export_ids().is_empty() {
        self.stdlib_prelude_export_ids = metadata
            .prelude_export_ids()
            .iter()
            .map(|(name, export)| (name.clone(), export.clone()))
            .collect();
    }

    for (name, func) in &interface.functions {
        let param_types: Vec<Type> = func.params.iter().map(|param| param.ty.clone()).collect();
        let func_type = Type::Function(param_types, Box::new(func.ret_type.clone()));
        self.scope.define(name.clone(), func_type, false);
        self.functions.insert(name.clone(), func.clone());
    }

    for ext in &interface.externs {
        let func_type = Type::Function(ext.params.clone(), Box::new(ext.ret.clone()));
        self.scope.define(ext.name.clone(), func_type, false);
        self.externs.push(ext.clone());
    }

    for (name, strukt) in &interface.structs {
        self.structs.insert(name.clone(), strukt.clone());
    }

    for (name, enum_) in &interface.enums {
        self.enums.insert(name.clone(), enum_.clone());
    }

    for (name, trait_) in &interface.traits {
        self.traits.insert(name.clone(), trait_.clone());
    }

    for imp in &interface.impls {
        let mut registered_impl = imp.clone();
        let impl_body_is_object_provided =
            link.concrete_impl_body_is_object_provided(&registered_impl);

        for (method_name, func) in &mut registered_impl.methods {
            if impl_body_is_object_provided {
                let backend_name = func
                    .qualified_name
                    .clone()
                    .unwrap_or_else(|| format!("{}_{}", registered_impl.type_name, method_name));
                func.qualified_name = Some(format!("{}::{}", crate_name, backend_name));
            }

            self.methods.insert(
                (registered_impl.type_name.clone(), method_name.clone()),
                func.clone(),
            );
            if let crate::hir::HirImplOwner::Named(owner) = &registered_impl.owner {
                self.methods
                    .insert((owner.clone(), method_name.clone()), func.clone());
            }
        }
        self.impls.push(registered_impl);
    }

    for (name, precedence) in &interface.infix_precedence {
        self.infix_precedence.insert(name.clone(), *precedence);
    }
}
```

Remove the `LoadedCrate` import from this file.

- [ ] **Step 4: Update source artifact declaration collection**

In `lib/src/collect/mod.rs`, replace the source lookup at the start of `collect_artifact_declarations`:

```rust
let source_crate = crate_ctx
    .source_crate(current_crate_name)
    .expect("artifact declaration collection requires loaded current source crate");
let module = &source_crate.ast;
let root_module_path = source_crate.root_module_path();
let inject_prelude = inject_prelude && crate_ctx.has_extern_crate("stdlib");
```

Also replace the main collection prelude gate in `collect`:

```rust
let inject_prelude = inject_prelude && crate_ctx.has_extern_crate("stdlib");
```

Replace the file-cache loop:

```rust
for (path, cached_module) in &source_crate.file_cache {
    context
        .module_file_cache
        .entry(path.clone())
        .or_insert_with(|| cached_module.clone());
}
```

Replace the dependency registration loop:

```rust
for dep in crate_ctx.extern_crates() {
    if dep.name() == current_crate_name {
        continue;
    }

    context.register_extern_crate(dep.name(), dep.metadata(), dep.link());
}
```

- [ ] **Step 5: Update collect tests that construct artifact dependencies**

In `lib/src/collect/mod.rs`, replace test-only `LoadedCrate` artifact dependency construction with `crate_ctx.add_extern_crate` using this constructor pattern:

```rust
crate_ctx
    .add_extern_crate(crate::crate_system::ExternCrateRecord::new(
        CrateId(1),
        "dep".to_string(),
        crate::crate_system::ExternCrateMetadata::new(
            interface,
            resolver,
            BTreeMap::new(),
            BTreeMap::new(),
        ),
        crate::crate_system::ExternCrateBodies::default(),
        crate::crate_system::ExternCrateLink::metadata_only(BTreeMap::new()),
    ))
    .unwrap();
```

For tests that intentionally register source crates, keep direct calls to `crate_ctx.register_crate` and assert through `source_crate` or collection errors only when the source crate is the current crate.

- [ ] **Step 6: Run collect tests and grep gate**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_registers_artifact_dependency_from_extern_store -- --exact
cargo test -p rock-lib collect::tests::collect_accepts_artifact_dependency_without_registering_source_module_path -- --exact
cargo test -p rock-lib collect::tests::collect_indexes_loaded_source_backed_module_bodies -- --exact
rg "register_loaded_crate|LoadedCrate|ctx\.crates|crate_ctx\.crates|downstream_metadata|downstream_link|ArtifactMode" lib/src/collect/context.rs lib/src/collect/mod.rs
```

Expected: tests PASS. The `rg` command should show no matches in production code sections for `collect/context.rs` or for the dependency registration path in `collect/mod.rs`; test matches in `collect/mod.rs` must be converted before Task 7.

- [ ] **Step 7: Commit Task 4**

Run:

```bash
git add lib/src/collect/context.rs lib/src/collect/mod.rs
git commit -m "route collection through extern crate store"
```

---

### Task 5: Route Lowering Through Extern Capabilities

**Files:**
- Modify: `lib/src/lower/crates/registration.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/lower/program.rs`

- [ ] **Step 1: Write failing lower extern-store registration test**

In `lib/src/lower/crates/registration.rs`, replace the object-backed `LoadedCrate` construction in `lower_registration_uses_link_provider_for_object_backed_impls` with a `CrateContext` extern record:

```rust
let mut crate_ctx = CrateContext::new();
crate_ctx
    .add_extern_crate(crate::crate_system::ExternCrateRecord::new(
        CrateId(1),
        "dep".to_string(),
        crate::crate_system::ExternCrateMetadata::new(
            interface,
            ResolverTables::default(),
            BTreeMap::new(),
            BTreeMap::new(),
        ),
        crate::crate_system::ExternCrateBodies::default(),
        crate::crate_system::ExternCrateLink::object(PathBuf::from("/dep/dep.o"), BTreeMap::new()),
    ))
    .unwrap();

let mut lowerer = Lowerer::new();
lowerer.register_crate_functions(&crate_ctx);
```

Keep the existing final assertion:

```rust
let method = lowerer
    .methods
    .get(&("DepThing".to_string(), "show".to_string()))
    .expect("object-backed method should be registered");
assert_eq!(method.qualified_name.as_deref(), Some("dep::DepThing_show"));
```

- [ ] **Step 2: Write failing lower body capability test**

In `lib/src/lower/crates/bodies.rs`, add a `#[cfg(test)]` module with this test:

```rust
#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use crate::collect::resolver::ResolverTables;
    use crate::crate_artifact::ArtifactCrateInterface;
    use crate::crate_system::{
        CrateContext, ExternCrateBodies, ExternCrateLink, ExternCrateMetadata, ExternCrateRecord,
    };
    use crate::hir::{HirBlock, HirFunction};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::lower::Lowerer;
    use crate::types::Type;

    fn generic_function() -> HirFunction {
        let id = DefId::new(CrateId(4), LocalDefId(1));
        HirFunction {
            id,
            name: "id".to_string(),
            qualified_name: Some("dep::id".to_string()),
            generic_params: vec!["T".to_string()],
            generic_param_ids: vec![crate::types::GenericParamId { owner: id, index: 0 }],
            generic_bounds: HashMap::new(),
            params: Vec::new(),
            ret_type: Type::I64,
            body: HirBlock {
                stmts: Vec::new(),
                ty: Type::I64,
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }

    #[test]
    fn lower_imports_generic_bodies_from_extern_body_capability() {
        let bodies = ExternCrateBodies::from_cross_crate_hir(crate::crate_artifact::ArtifactCrossCrateHir {
            generic_functions: BTreeMap::from([("dep::id".to_string(), generic_function())]),
            traits_with_defaults: BTreeMap::new(),
            generic_impls: Vec::new(),
        });
        let mut ctx = CrateContext::new();
        ctx.add_extern_crate(ExternCrateRecord::new(
            CrateId(4),
            "dep".to_string(),
            ExternCrateMetadata::new(
                ArtifactCrateInterface::default(),
                ResolverTables::default(),
                BTreeMap::new(),
                BTreeMap::new(),
            ),
            bodies,
            ExternCrateLink::metadata_only(BTreeMap::new()),
        ))
        .unwrap();

        let mut lowerer = Lowerer::new();
        lowerer.lower_crate_module_bodies(&ctx);

        assert!(lowerer.functions.contains_key("dep::id"));
    }
}
```

- [ ] **Step 3: Run lower tests red or old-green**

Run:

```bash
cargo test -p rock-lib lower::crates::registration::tests::lower_registration_uses_link_provider_for_object_backed_impls -- --exact
cargo test -p rock-lib lower::crates::bodies::tests::lower_imports_generic_bodies_from_extern_body_capability -- --exact
```

Expected: FAIL before implementation if lower still only consumes `LoadedCrate`; if registration already passes through temporary compatibility, continue and use the grep gate in Step 7.

- [ ] **Step 4: Route lower registration through extern metadata/link**

In `lib/src/lower/crates/registration.rs`, update imports:

```rust
use crate::crate_system::{CrateContext, ExternCrateLink, ExternCrateMetadata};
```

Replace `register_crate_resolvers` with:

```rust
pub(crate) fn register_crate_resolvers(&mut self, ctx: &CrateContext) {
    for dep in ctx.extern_crates() {
        self.dependency_resolvers
            .insert(dep.name().to_string(), dep.metadata().resolver().clone());
    }
}
```

Replace `register_crate_functions` and `register_loaded_crate` with `register_extern_crate` using the same logic as collection, except preserve the existing lower-specific trait-impl-method behavior:

```rust
pub(crate) fn register_crate_functions(&mut self, ctx: &CrateContext) {
    self.register_crate_resolvers(ctx);

    for dep in ctx.extern_crates() {
        self.register_extern_crate(dep.name(), dep.metadata(), dep.link());
    }
}

pub(crate) fn register_extern_crate(
    &mut self,
    crate_name: &str,
    metadata: &ExternCrateMetadata,
    link: &ExternCrateLink,
) {
    let is_stdlib = crate_name == "stdlib";
    let interface = metadata.interface();

    if !interface.root_exports.is_empty() {
        self.artifact_root_exports.insert(
            crate_name.to_string(),
            interface
                .root_exports
                .iter()
                .map(|(name, source)| (name.clone(), source.clone()))
                .collect(),
        );
    }
    if !interface.root_export_ids.is_empty() {
        self.artifact_root_export_ids.insert(
            crate_name.to_string(),
            interface
                .root_export_ids
                .iter()
                .map(|(name, export)| (name.clone(), export.clone()))
                .collect(),
        );
    }

    if is_stdlib && !metadata.prelude_exports().is_empty() {
        self.stdlib_prelude_exports = metadata
            .prelude_exports()
            .iter()
            .map(|(name, source)| (name.clone(), Some(source.clone())))
            .collect();
    }
    if is_stdlib && !metadata.prelude_export_ids().is_empty() {
        self.stdlib_prelude_export_ids = metadata
            .prelude_export_ids()
            .iter()
            .map(|(name, export)| (name.clone(), export.clone()))
            .collect();
    }

    for (name, func) in &interface.functions {
        let param_types: Vec<Type> = func.params.iter().map(|param| param.ty.clone()).collect();
        let func_type = Type::Function(param_types, Box::new(func.ret_type.clone()));
        self.scope.define(name.clone(), func_type, false);
        self.functions.insert(name.clone(), func.clone());
    }

    for ext in &interface.externs {
        let func_type = Type::Function(ext.params.clone(), Box::new(ext.ret.clone()));
        self.scope.define(ext.name.clone(), func_type, false);
        self.externs.push(ext.clone());
    }

    for (name, strukt) in &interface.structs {
        self.structs.insert(name.clone(), strukt.clone());
    }
    for (name, enum_) in &interface.enums {
        self.enums.insert(name.clone(), enum_.clone());
    }
    for (name, trait_) in &interface.traits {
        self.traits.insert(name.clone(), trait_.clone());
    }

    for imp in &interface.impls {
        let mut registered_impl = imp.clone();
        let impl_body_is_object_provided =
            link.concrete_impl_body_is_object_provided(&registered_impl);

        for (method_name, func) in &mut registered_impl.methods {
            if impl_body_is_object_provided {
                let backend_name = func
                    .qualified_name
                    .clone()
                    .unwrap_or_else(|| format!("{}_{}", registered_impl.type_name, method_name));
                func.qualified_name = Some(format!("{}::{}", crate_name, backend_name));
            }

            if registered_impl.trait_name.is_none() {
                self.methods.insert(
                    (registered_impl.type_name.clone(), method_name.clone()),
                    func.clone(),
                );
                if let HirImplOwner::Named(owner) = &registered_impl.owner {
                    self.methods
                        .insert((owner.clone(), method_name.clone()), func.clone());
                }
            }
        }
        self.impls.push(registered_impl);
    }

    for (name, precedence) in &interface.infix_precedence {
        self.infix_precedence.insert(name.clone(), *precedence);
    }
}
```

- [ ] **Step 5: Route lower body imports through body accessors**

In `lib/src/lower/crates/bodies.rs`, remove the `ArtifactCrossCrateHir` import and add:

```rust
use crate::crate_system::{CrateContext, ExternCrateBodies};
```

Replace both loops over `ctx.crates` with `ctx.extern_crates()`:

```rust
for dep in ctx.extern_crates() {
    self.apply_cross_crate_trait_defaults(dep.bodies());
}
```

and:

```rust
for dep in ctx.extern_crates() {
    self.apply_cross_crate_generic_bodies(dep.bodies());
}
```

Change helper signatures and field access:

```rust
fn apply_cross_crate_trait_defaults(&mut self, bodies: &ExternCrateBodies) {
    for (trait_name, trait_def) in bodies.traits_with_defaults() {
        if let Some(existing) = self.traits.get_mut(trait_name) {
            *existing = trait_def.clone();
        } else {
            self.traits.insert(trait_name.clone(), trait_def.clone());
        }
    }
}

fn apply_cross_crate_generic_bodies(&mut self, bodies: &ExternCrateBodies) {
    for (qualified_name, func) in bodies.generic_functions() {
        let param_types: Vec<Type> = func.params.iter().map(|param| param.ty.clone()).collect();
        let func_type = Type::Function(param_types, Box::new(func.ret_type.clone()));
        self.scope.define(qualified_name.clone(), func_type, false);
        self.functions.insert(qualified_name.clone(), func.clone());
    }

    for bundle_impl in bodies.generic_impls() {
        let existing_idx = self.impls.iter().position(|imp| imp.id == bundle_impl.id);

        let target_impl = if let Some(idx) = existing_idx {
            &mut self.impls[idx]
        } else {
            self.impls.push(bundle_impl.clone());
            self.impls.last_mut().unwrap()
        };

        for (method_name, func) in &bundle_impl.methods {
            target_impl.methods.insert(method_name.clone(), func.clone());
            if target_impl.trait_name.is_none() {
                self.methods.insert(
                    (target_impl.type_name.clone(), method_name.clone()),
                    func.clone(),
                );
            }

            if !func.is_method && !target_impl.type_generics.is_empty() {
                self.functions.insert(
                    format!("{}_{}", target_impl.type_name, method_name),
                    func.clone(),
                );
            }
        }
    }
}
```

- [ ] **Step 6: Use extern stdlib for prelude decisions**

In `lib/src/lower/program.rs`, replace:

```rust
let has_stdlib = ctx.has_crate("stdlib");
```

with:

```rust
let has_stdlib = ctx.has_extern_crate("stdlib");
```

In `lower_from_declarations`, replace:

```rust
if lowerer.inject_prelude && crate_ctx.has_crate("stdlib") {
    lowerer.inject_stdlib_prelude(crate_ctx);
}
```

with:

```rust
if lowerer.inject_prelude && crate_ctx.has_extern_crate("stdlib") {
    lowerer.inject_stdlib_prelude(crate_ctx);
}
```

- [ ] **Step 7: Run lower tests and grep gate**

Run:

```bash
cargo test -p rock-lib lower::crates::registration::tests::lower_registration_uses_link_provider_for_object_backed_impls -- --exact
cargo test -p rock-lib lower::crates::registration::tests::lower_registration_does_not_register_trait_impl_methods_in_name_map -- --exact
cargo test -p rock-lib lower::crates::bodies::tests::lower_imports_generic_bodies_from_extern_body_capability -- --exact
rg "register_loaded_crate|LoadedCrate|ctx\.crates|crate_ctx\.crates|downstream_metadata|downstream_bodies|downstream_link|ArtifactCrossCrateHir|ArtifactMode" lib/src/lower/crates lib/src/lower/program.rs
```

Expected: tests PASS. The `rg` command should show no production-code matches in `lib/src/lower/crates` or `lib/src/lower/program.rs`.

- [ ] **Step 8: Commit Task 5**

Run:

```bash
git add lib/src/lower/crates/registration.rs lib/src/lower/crates/bodies.rs lib/src/lower/program.rs
git commit -m "route lowering through extern crate capabilities"
```

---

### Task 6: Route Monomorphization And Link Setup Through Extern Store

**Files:**
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_system/context.rs`

- [ ] **Step 1: Update mono tests to construct extern records**

In `lib/src/mono/external.rs`, replace the helper return types `LoadedCrate` with `ExternCrateRecord`.

Change the imports in the test module from:

```rust
use crate::crate_system::{
    ArtifactMode, CrateConfig, CrateContext, CrateManifest, LibConfig, LoadedCrate,
};
```

to:

```rust
use crate::crate_system::{
    CrateContext, ExternCrateBodies, ExternCrateLink, ExternCrateMetadata, ExternCrateRecord,
};
```

Change `loaded_object_backed_crate_with_backend_symbols` to return `ExternCrateRecord` and end with:

```rust
ExternCrateRecord::new(
    CrateId(1),
    "stdlib".to_string(),
    ExternCrateMetadata::new(
        ArtifactCrateInterface {
            impls: interface_impls,
            ..ArtifactCrateInterface::default()
        },
        resolver,
        BTreeMap::new(),
        BTreeMap::new(),
    ),
    ExternCrateBodies::from_cross_crate_hir(ArtifactCrossCrateHir {
        generic_functions: BTreeMap::new(),
        traits_with_defaults: BTreeMap::new(),
        generic_impls: cross_crate_impls,
    }),
    ExternCrateLink::object(PathBuf::from("stdlib.o"), backend_symbols),
)
```

Change `loaded_object_backed_function_crate` to return `ExternCrateRecord` and end with:

```rust
ExternCrateRecord::new(
    CrateId(1),
    crate_name.to_string(),
    ExternCrateMetadata::new(interface, resolver, BTreeMap::new(), BTreeMap::new()),
    ExternCrateBodies::from_cross_crate_hir(ArtifactCrossCrateHir {
        generic_functions: BTreeMap::new(),
        traits_with_defaults: BTreeMap::new(),
        generic_impls: vec![],
    }),
    ExternCrateLink::object(object_path, BTreeMap::new()),
)
```

Replace test insertions like:

```rust
crate_ctx.add_crate("dep".to_string(), loaded);
```

with:

```rust
crate_ctx.add_extern_crate(loaded).unwrap();
```

For `process_with_crates_uses_artifact_backend_symbols_for_object_backed_functions`, build the backend-symbol map before constructing the record:

```rust
let loaded = loaded_object_backed_function_crate_with_backend_symbols(
    "dep",
    "dep::answer",
    func,
    PathBuf::from("dep.o"),
    BTreeMap::from([(answer_id, "artifact_dep_answer".to_string())]),
);
```

Add that helper by copying `loaded_object_backed_function_crate` and passing the final `backend_symbols` into `ExternCrateLink::object`.

- [ ] **Step 2: Run mono tests red or old-green**

Run:

```bash
cargo test -p rock-lib mono::external::tests::process_with_crates_uses_link_provider_for_object_backed_functions -- --exact
cargo test -p rock-lib mono::external::tests::process_with_crates_uses_artifact_backend_symbols_for_object_backed_functions -- --exact
cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact
```

Expected: FAIL before implementation if mono still iterates `crate_ctx.crates`.

- [ ] **Step 3: Route mono dependency loops through extern records**

In `lib/src/mono/external.rs`, replace the import:

```rust
use crate::crate_system::{CrateContext, DependencyLink};
```

with:

```rust
use crate::crate_system::{CrateContext, ExternCrateLink};
```

In `process_with_crates_impl`, replace the `object_backed_impls` construction with:

```rust
let object_backed_impls = crate_ctx
    .extern_crates()
    .flat_map(|dep| {
        dep.metadata()
            .interface()
            .impls
            .iter()
            .filter(|imp| dep.link().concrete_impl_body_is_object_provided(imp))
            .map(Self::impl_signature_key)
            .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();
```

Replace dependency resolver loading with:

```rust
self.dependency_resolvers = crate_ctx
    .extern_crates()
    .map(|dep| (dep.name().to_string(), dep.metadata().resolver().clone()))
    .collect();
```

Replace every loop shaped like:

```rust
for (crate_name, loaded_crate) in &crate_ctx.crates {
```

with:

```rust
for dep in crate_ctx.extern_crates() {
    let crate_name = dep.name();
    let metadata = dep.metadata();
    let link = dep.link();
```

Then replace field access as follows:

```rust
metadata.interface.functions      -> metadata.interface().functions
metadata.interface.impls          -> metadata.interface().impls
metadata.resolver.import_aliases  -> metadata.resolver().import_aliases
metadata.resolver.export_aliases  -> metadata.resolver().export_aliases
metadata.resolver.item_names_by_id -> metadata.resolver().item_names_by_id
bodies.bundle().generic_functions -> dep.bodies().generic_functions()
bodies.bundle().generic_impls     -> dep.bodies().generic_impls()
```

Change `record_object_backed_impl` signature:

```rust
fn record_object_backed_impl(
    &mut self,
    crate_name: &str,
    imp: &crate::hir::HirImpl,
    link: Option<&ExternCrateLink>,
)
```

- [ ] **Step 4: Remove source-backed mono panic test**

Delete `process_with_crates_rejects_source_backed_external_dependency_consumption`. Source crates are no longer external dependencies, and `CrateContext::register_crate` does not insert extern records. The replacement source/external boundary test is `crate_context_registers_source_crates_outside_extern_store` from Task 2.

- [ ] **Step 5: Make link setup use infallible extern link inputs**

In `lib/src/crate_system/context.rs`, after Task 3 has made object paths required by `ExternCrateLink::object`, simplify `dependency_link_inputs`:

```rust
pub(crate) fn dependency_link_inputs(&self) -> DependencyLinkInputs {
    self.extern_crates.link_inputs()
}
```

In `lib/src/lib.rs`, replace the match around `dependency_link_inputs("codegen")` with:

```rust
let link_inputs = crate_ctx.dependency_link_inputs();
```

Remove now-unused diagnostic wrapping for dependency link inputs in that section.

- [ ] **Step 6: Run mono/link tests and grep gate**

Run:

```bash
cargo test -p rock-lib mono::external::tests::process_with_crates_uses_link_provider_for_object_backed_functions -- --exact
cargo test -p rock-lib mono::external::tests::process_with_crates_uses_artifact_backend_symbols_for_object_backed_functions -- --exact
cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact
cargo test -p rock-lib crate_system::tests::crate_context_collects_dependency_link_inputs_from_provider_capabilities -- --exact
rg "LoadedCrate|DependencyLink|ctx\.crates|crate_ctx\.crates|downstream_metadata|downstream_bodies|downstream_link|ArtifactMode" lib/src/mono/external.rs lib/src/lib.rs lib/src/crate_system/context.rs
```

Expected: tests PASS. The `rg` command should show no production-code matches in `mono/external.rs`, `lib.rs`, or `crate_system/context.rs` except the temporary `LoadedCrate` compatibility code in `context.rs` that Task 7 removes.

- [ ] **Step 7: Commit Task 6**

Run:

```bash
git add lib/src/mono/external.rs lib/src/lib.rs lib/src/crate_system/context.rs
git commit -m "route mono and linking through extern crate store"
```

---

### Task 7: Delete `LoadedCrate` And `ArtifactMode`

**Files:**
- Modify: `lib/src/crate_system/mod.rs`
- Modify: `lib/src/crate_system/context.rs`
- Modify: `lib/src/crate_system/tests.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/lower/crates/registration.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Remove compatibility storage from `CrateContext`**

In `lib/src/crate_system/mod.rs`, replace `CrateContext` with:

```rust
#[derive(Debug)]
pub struct CrateContext {
    source_crates: BTreeMap<String, CurrentCrateSource>,
    extern_crates: ExternCrateStore,
    pub(crate) product_crate_ids: BTreeMap<ProductCrateIdentity, crate::ids::CrateId>,
    next_product_crate_id: u32,
}
```

Delete the definitions for `ArtifactMode`, `LoadedCrate`, `DependencyMetadata`, `DependencyBodies`, and `DependencyLink`, and delete the full `impl LoadedCrate` block.

Remove now-unused imports from `mod.rs`:

```rust
use std::path::PathBuf;
use crate::ast::Module;
use crate::collect::resolver::ResolverTables;
use crate::crate_artifact::{ArtifactCrateInterface, ArtifactCrossCrateHir, ArtifactExport};
use crate::hir::HirImpl;
use crate::ids::DefId;
```

- [ ] **Step 2: Remove compatibility construction from context methods**

In `lib/src/crate_system/context.rs`, update imports:

```rust
use super::{CrateContext, CrateManifest, CurrentCrateSource, ExternCrateRecord, ExternCrateRef};
```

In `CrateContext::new`, remove `crates: BTreeMap::new()`.

Replace `register_crate` with:

```rust
pub fn register_crate(&mut self, manifest: CrateManifest, source_dir: PathBuf, ast: Module) {
    let name = manifest.crate_.name.clone();
    self.source_crates.insert(
        name,
        CurrentCrateSource::new(manifest, source_dir, ast),
    );
}
```

In `load_crate_from_dir`, remove the `LoadedCrate` construction and final `self.crates.insert`. After collecting `file_cache`, store only `CurrentCrateSource`:

```rust
let name = manifest.crate_.name.clone();
let mut source = CurrentCrateSource::new(manifest, crate_dir, ast);
source.file_cache = file_cache;
self.source_crates.insert(name, source);
Ok(())
```

Replace `get_crate_lib_path` with:

```rust
pub fn get_crate_lib_path(&self, name: &str) -> Option<PathBuf> {
    self.source_crate(name).map(CurrentCrateSource::lib_path)
}
```

Delete `add_crate`.

Change `has_crate` and `crate_count`:

```rust
pub fn has_crate(&self, name: &str) -> bool {
    self.has_source_crate(name) || self.has_extern_crate(name)
}

pub fn crate_count(&self) -> usize {
    let mut names = std::collections::BTreeSet::new();
    names.extend(self.source_crates.keys().cloned());
    names.extend(self.extern_crates().map(|dep| dep.name().to_string()));
    names.len()
}
```

In `load_crate_with_dependencies`, update module tree assignment:

```rust
if let Some(source_crate) = self.source_crate_mut(&crate_name) {
    match source_crate.build_module_tree() {
        Ok(tree) => {
            source_crate.module_tree = Some(tree);
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to build module tree for '{}': {}",
                crate_name, e
            );
        }
    }
}
```

Add this method to `CurrentCrateSource` in `extern_store.rs`:

```rust
pub(crate) fn build_module_tree(&self) -> Result<ModuleTree, String> {
    crate::crate_system::module_tree::build_module_tree(&self.ast, &self.root_dir)
}
```

If `build_module_tree` is private to `LoadedCrate`, move its current implementation to `module_tree.rs` as `pub(crate) fn build_module_tree(module: &Module, root_dir: &Path) -> Result<ModuleTree, String>` and call it from `CurrentCrateSource`.

In `compilation_order`, iterate `source_crates` for source graphs and `extern_crates` for standalone artifact names:

```rust
for name in self.source_crates.keys() {
    all_crates.insert(name.clone());
    in_degree.insert(name.clone(), 0);
    adj_list.insert(name.clone(), Vec::new());
}
for dep in self.extern_crates() {
    all_crates.insert(dep.name().to_string());
    in_degree.entry(dep.name().to_string()).or_insert(0);
    adj_list.entry(dep.name().to_string()).or_default();
}
for (name, source_crate) in &self.source_crates {
    if let Some(ref deps) = source_crate.manifest.dependencies {
        for dep_name in deps.keys() {
            if all_crates.contains(dep_name) {
                adj_list.get_mut(dep_name).unwrap().push(name.clone());
                *in_degree.get_mut(name).unwrap() += 1;
            }
        }
    }
}
```

- [ ] **Step 3: Remove loaded-crate product construction**

In `lib/src/crate_artifact/load.rs`, remove imports of `ArtifactMode`, `CrateConfig`, `CrateManifest`, `LibConfig`, `LoadedCrate`, and `Module`.

Delete `loaded_crate_from_products`.

In `load_product_artifact_from_path`, replace the body after remap creation with:

```rust
let extern_record = extern_crate_from_products(&products, &artifact_path, &remap, self)?;
self.add_extern_crate(extern_record)?;
```

In `load_product_artifact_from_path_as`, replace the body after remap creation with:

```rust
let extern_record = extern_crate_from_products(&products, &artifact_path, &remap, self)?;
self.add_extern_crate(extern_record)?;
```

- [ ] **Step 4: Migrate product dependency validators to extern store**

In `ProductDependencyDefinitions::from_context`, replace:

```rust
for loaded in ctx.crates.values() {
    let Some(interface) = loaded.interface.as_ref() else {
        continue;
    };
```

with:

```rust
for dep in ctx.extern_crates() {
    let interface = dep.metadata().interface();
```

Replace:

```rust
let crate_name = &loaded.manifest.crate_.name;
```

with:

```rust
let crate_name = dep.name();
```

Replace the entire `if let Some(cross_crate_hir) = loaded.cross_crate_hir.as_ref()` block with:

```rust
for (qualified_name, function) in dep.bodies().generic_functions() {
    if let Some(id) = product_id_for_def(function.id) {
        defs.functions.insert(id);
        add_callable_names(
            &mut defs.function_names,
            id,
            crate_name,
            [qualified_name.as_str(), function.name.as_str()],
        );
    }
}

for imp in dep.bodies().generic_impls() {
    record_dependency_impl(&mut defs, imp, &product_id_for_def);
}
```

In `ProductChildLocationValidator::from_products_with_context`, replace the dependency loop with:

```rust
for dep in ctx.extern_crates() {
    let interface = dep.metadata().interface();

    for struct_def in interface.structs.values() {
        let Some(owner) = product_id_for_def(struct_def.id) else {
            continue;
        };
        for field in &struct_def.fields {
            validator
                .fields_by_id
                .insert((owner, field.id), field.name.clone());
        }
    }

    for enum_def in interface.enums.values() {
        let Some(owner) = product_id_for_def(enum_def.id) else {
            continue;
        };
        for variant in &enum_def.variants {
            validator
                .variants_by_id
                .insert((owner, variant.id), variant.name.clone());
            if let crate::hir::HirVariantFields::Named(fields) = &variant.fields {
                for field in fields {
                    validator
                        .fields_by_id
                        .insert((owner, field.id), field.name.clone());
                }
            }
        }
    }
}
```

- [ ] **Step 5: Update artifact tests to inspect extern capabilities**

In `lib/src/crate_artifact/load.rs` tests and `lib/src/crate_artifact/tests.rs`, replace direct loaded-crate assertions using this mapping:

```rust
let loaded = ctx.crates.get("dep").unwrap();
let interface = loaded.interface.as_ref().unwrap();
```

becomes:

```rust
let dep = ctx.extern_crate("dep").unwrap();
let interface = dep.metadata().interface();
```

Replace:

```rust
loaded.cross_crate_hir.as_ref().unwrap().generic_functions["dep::answer"]
```

with:

```rust
dep.bodies().generic_functions()["dep::answer"]
```

Replace:

```rust
loaded.cross_crate_hir.as_ref().unwrap().generic_impls
```

with:

```rust
dep.bodies().generic_impls()
```

Replace:

```rust
loaded.object_path
```

with:

```rust
dep.link().object_path().cloned()
```

Replace:

```rust
loaded.backend_symbols.get(&function.id).map(String::as_str)
```

with:

```rust
dep.link().backend_symbol(function.id)
```

Replace:

```rust
loaded.resolver.item_paths
```

with:

```rust
dep.metadata().resolver().item_paths
```

Replace:

```rust
loaded.prelude_export_ids.get("String")
loaded.prelude_exports.get("String")
```

with:

```rust
dep.metadata().prelude_export_ids().get("String")
dep.metadata().prelude_exports().get("String")
```

- [ ] **Step 6: Update remaining unit tests away from `LoadedCrate`**

In `lib/src/crate_system/tests.rs`, delete tests that directly exercise `downstream_interface`, `downstream_metadata`, `downstream_bodies`, and `downstream_link`. Replace their coverage with the extern-store tests from Tasks 1-3.

In `lib/src/lower/crates/registration.rs`, delete `rejected_source_backed_crate_does_not_register_module_path` and `accepted_artifact_crate_does_not_register_module_path`. Source crates are not extern dependencies, and artifact crates are covered by `lower_registration_uses_link_provider_for_object_backed_impls`.

In `lib/src/collect/mod.rs`, replace remaining `LoadedCrate` test fixtures with `ExternCrateRecord` for artifact dependencies or `register_crate` for current-source tests.

- [ ] **Step 7: Run deletion grep gate**

Run:

```bash
rg "LoadedCrate|ArtifactMode|downstream_interface|downstream_metadata|downstream_bodies|downstream_link|ctx\.crates|crate_ctx\.crates|\.crates" lib/src
```

Expected: no production-code matches. Test matches are allowed only when checking unrelated terminology such as `crate_count`; there must be no `LoadedCrate`, `ArtifactMode`, or direct `CrateContext::crates` matches anywhere under `lib/src`.

- [ ] **Step 8: Run focused tests**

Run:

```bash
cargo test -p rock-lib crate_system::tests -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib collect_registers_artifact_dependency_from_extern_store -- --exact
cargo test -p rock-lib lower_registration_uses_link_provider_for_object_backed_impls -- --exact
cargo test -p rock-lib process_with_crates_uses_link_provider_for_object_backed_functions -- --exact
```

Expected: PASS.

- [ ] **Step 9: Commit Task 7**

Run:

```bash
git add lib/src/crate_system/mod.rs lib/src/crate_system/context.rs lib/src/crate_system/tests.rs lib/src/crate_artifact/load.rs lib/src/crate_artifact/tests.rs lib/src/collect/mod.rs lib/src/lower/crates/registration.rs lib/src/mono/external.rs
git commit -m "remove mixed loaded crate dependency storage"
```

---

### Task 8: Update Architecture Trackers And Final Verification

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Update roadmap Tasks 6-7**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the 2026-05-18 rebaseline note to mention this completed slice after implementation:

```markdown
- Roadmap Tasks 6-7 have landed for the dependency storage boundary: external dependencies now live in an artifact-only extern crate store keyed by session `CrateId`; current source input lives in `CurrentCrateSource`; collect/lower/mono/codegen consume metadata/body/link capabilities without `LoadedCrate` or `ArtifactMode`.
```

Update Task 6 status line:

```markdown
**Status:** Complete for artifact-backed external dependencies in `docs/superpowers/plans/2026-05-18-rust-style-extern-crate-store.md`.
```

Update Task 7 status line:

```markdown
**Status:** Complete for explicit body/link capability APIs in `docs/superpowers/plans/2026-05-18-rust-style-extern-crate-store.md`.
```

- [ ] **Step 2: Update master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, update the Crate And Artifact Interface Split summary row:

```markdown
| Crate And Artifact Interface Split | In progress | `lib/src/crate_system/extern_store.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/collect/context.rs`, `lib/src/lower/crates/*`, `lib/src/mono/external.rs` | External dependency storage is AST-free and provider-backed; remaining work is product schema evolution and future transitive artifact tooling outside this slice |
```

In section `## 8. Crate And Artifact Interface Split`, add Done entries:

```markdown
- [x] Replaced mixed `LoadedCrate` dependency storage with an artifact-only `ExternCrateStore` keyed by session `CrateId`.
- [x] Split current source input into `CurrentCrateSource`, keeping AST/module caches out of external dependency records.
- [x] Routed collect, lower, mono, and codegen/link setup through extern metadata, body, and link capabilities.
- [x] Deleted `LoadedCrate` and `ArtifactMode` as dependency-facing storage concepts.
```

Remove or reword Still-to-do entries that say `LoadedCrate` must still be split. Keep future-looking entries only if they are outside this slice, such as product schema evolution or transitive artifact discovery.

- [ ] **Step 3: Run final verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib crate_artifact
cargo test -p rock-lib
```

Expected: PASS.

- [ ] **Step 4: Run final grep gates**

Run:

```bash
rg "LoadedCrate|ArtifactMode|downstream_interface|downstream_metadata|downstream_bodies|downstream_link|ctx\.crates|crate_ctx\.crates|\.crates" lib/src
rg "ast|file_cache|module_tree|root_dir" lib/src/crate_system/extern_store.rs
```

Expected: first command has no matches. Second command may match only `CurrentCrateSource`; it must not match `ExternCrateRecord`, `ExternCrateMetadata`, `ExternCrateBodies`, `ExternCrateLink`, or `ExternCrateStore` fields.

- [ ] **Step 5: Commit Task 8**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for extern crate store"
```

---

## Final Completion Gate

Before reporting completion, run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
git status --short
```

Expected:

- `cargo fmt --all --check`: exit 0
- `git diff --check`: exit 0
- `cargo test -p rock-lib`: exit 0
- `git status --short`: clean after the final commit

Do not claim the slice is complete without fresh output from these commands.
