# Dependency Provider Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hide dependency storage-mode details behind explicit metadata, body, and link provider capabilities before resuming canonical identity work.

**Architecture:** Keep `LoadedCrate` as the internal storage object for this slice, but add provider views that expose downstream dependency capabilities without requiring compiler phases to inspect `ArtifactMode`, `cross_crate_hir`, `object_path`, or `interface` directly. Route collect, lower, mono, and codegen setup through those provider APIs, preserving the existing artifact-only rejection path and current-crate source artifact production.

**Tech Stack:** Rust 2021, `rock-lib`, `CrateContext`, `LoadedCrate`, product artifacts, collect/lower/mono/codegen pipeline, `cargo test`.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-05-11-dependency-provider-boundaries-design.md`.

The invariant for every task is:

- Current-crate source parsing and source artifact production stay valid.
- Downstream dependencies remain artifact-only.
- `LoadedCrate` can remain the storage struct, but downstream compiler phases should request capabilities instead of branching on storage mode or reading storage fields directly.
- Product artifact schema and `rock` transitive artifact collection are not redesigned in this slice.

## File Structure

- Modify: `lib/src/crate_system/mod.rs`
  - Add `DependencyMetadata`, `DependencyBodies`, `DependencyLink`, and provider methods on `LoadedCrate`.
  - Keep `ArtifactMode` internal to the crate system for now.
- Modify: `lib/src/crate_system/context.rs`
  - Add crate-context link input aggregation for codegen/linking.
  - Replace `get_object_paths` with provider-backed link inputs.
- Modify: `lib/src/crate_system/tests.rs`
  - Add provider unit tests for source rejection, metadata exposure, body exposure, and link exposure.
- Modify: `lib/src/collect/context.rs`
  - Register dependency declarations via metadata/link providers instead of `downstream_interface` and `is_object_backed`.
- Modify: `lib/src/collect/mod.rs`
  - Add or update tests for provider-based collection and keep existing source-backed rejection/current-crate source module tests.
- Modify: `lib/src/lower/crates/registration.rs`
  - Register dependency declarations via metadata/link providers instead of `downstream_interface` and `is_object_backed`.
- Modify: `lib/src/lower/crates/bodies.rs`
  - Apply dependency bodies via body provider instead of reading `loaded_crate.cross_crate_hir`.
- Modify: `lib/src/mono/external.rs`
  - Load object-backed functions/impls and cross-crate HIR bodies via body/link providers.
- Modify: `lib/src/mono/mod.rs`
  - Stop preloading dependency resolvers from `LoadedCrate` fields outside provider-backed mono setup.
- Modify: `lib/src/lib.rs`
  - Use provider-backed crate link inputs for codegen object crate names and link object paths.
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
  - Mark provider-boundary hiding complete if grep and tests prove downstream phases no longer branch through storage details.

---

### Task 1: Add Dependency Provider Capability Views

**Files:**
- Modify: `lib/src/crate_system/mod.rs`
- Modify: `lib/src/crate_system/tests.rs`

- [ ] **Step 1: Add failing provider tests**

Add these imports to the test module in `lib/src/crate_system/tests.rs` if they are not already present:

```rust
use std::collections::HashMap;

use crate::crate_artifact::ArtifactCrossCrateHir;
use crate::hir::{HirBlock, HirFunction};
use crate::ids::{CrateId, DefId, LocalDefId};
use crate::types::Type;
```

Add this helper above the provider tests:

```rust
fn hir_function(name: &str, qualified_name: Option<&str>, generic_params: &[&str]) -> HirFunction {
    HirFunction {
        id: DefId::new(CrateId(0), LocalDefId(1)),
        name: name.to_string(),
        qualified_name: qualified_name.map(str::to_string),
        generic_params: generic_params.iter().map(|param| param.to_string()).collect(),
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

Add these tests to `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn loaded_crate_rejects_source_backed_dependency_providers() {
    let loaded = LoadedCrate {
        manifest: empty_manifest("dep"),
        ast: empty_module(),
        interface: None,
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: None,
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::Source,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };

    let metadata_error = loaded
        .downstream_metadata("dep", "collection")
        .expect_err("source-backed metadata should be rejected");
    let bodies_error = loaded
        .downstream_bodies("dep", "lowering")
        .expect_err("source-backed bodies should be rejected");
    let link_error = loaded
        .downstream_link("dep", "codegen")
        .expect_err("source-backed links should be rejected");

    assert!(metadata_error.contains("source-backed external dependency 'dep'"));
    assert!(bodies_error.contains("source-backed external dependency 'dep'"));
    assert!(link_error.contains("source-backed external dependency 'dep'"));
}

#[test]
fn loaded_crate_metadata_provider_exposes_interface_and_prelude_exports() {
    let mut interface = crate::crate_artifact::ArtifactCrateInterface::default();
    interface
        .root_exports
        .insert("answer".to_string(), "dep::answer".to_string());
    let mut prelude_exports = BTreeMap::new();
    prelude_exports.insert("pipe".to_string(), "dep::prelude::pipe".to_string());
    let mut resolver = ResolverTables::default();
    let answer_id = DefId::new(CrateId(0), LocalDefId(7));
    resolver
        .item_paths
        .insert("dep::answer".to_string(), answer_id);

    let loaded = LoadedCrate {
        manifest: empty_manifest("dep"),
        ast: empty_module(),
        interface: Some(interface),
        resolver,
        prelude_exports,
        object_path: None,
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::InterfaceOnly,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };

    let metadata = loaded
        .downstream_metadata("dep", "collection")
        .expect("artifact metadata provider should exist");

    assert_eq!(
        metadata.interface.root_exports.get("answer"),
        Some(&"dep::answer".to_string())
    );
    assert_eq!(
        metadata.prelude_exports.get("pipe"),
        Some(&"dep::prelude::pipe".to_string())
    );
    assert_eq!(metadata.resolver.item_paths.get("dep::answer"), Some(&answer_id));
}

#[test]
fn loaded_crate_body_provider_exposes_cross_crate_hir_bundle() {
    let mut bundle = ArtifactCrossCrateHir {
        generic_functions: BTreeMap::new(),
        traits_with_defaults: BTreeMap::new(),
        generic_impls: Vec::new(),
    };
    bundle.generic_functions.insert(
        "dep::id".to_string(),
        hir_function("id", Some("dep::id"), &["T"]),
    );

    let loaded = LoadedCrate {
        manifest: empty_manifest("dep"),
        ast: empty_module(),
        interface: Some(crate::crate_artifact::ArtifactCrateInterface::default()),
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: None,
        cross_crate_hir: Some(bundle),
        artifact_mode: ArtifactMode::InterfaceOnly,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };

    let bodies = loaded
        .downstream_bodies("dep", "lowering")
        .expect("artifact body provider should exist");

    assert!(bodies.bundle().unwrap().generic_functions.contains_key("dep::id"));
}

#[test]
fn loaded_crate_link_provider_exposes_object_inputs() {
    let loaded = LoadedCrate {
        manifest: empty_manifest("dep"),
        ast: empty_module(),
        interface: Some(crate::crate_artifact::ArtifactCrateInterface::default()),
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: Some(PathBuf::from("/dep/build/dep.o")),
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::Object,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };

    let link = loaded
        .downstream_link("dep", "codegen")
        .expect("artifact link provider should exist");

    assert!(link.is_object_backed());
    assert_eq!(link.object_path(), Some(&PathBuf::from("/dep/build/dep.o")));
}
```

- [ ] **Step 2: Run provider tests red**

Run:

```bash
cargo test -p rock-lib loaded_crate_rejects_source_backed_dependency_providers -- --exact
cargo test -p rock-lib loaded_crate_metadata_provider_exposes_interface_and_prelude_exports -- --exact
cargo test -p rock-lib loaded_crate_body_provider_exposes_cross_crate_hir_bundle -- --exact
cargo test -p rock-lib loaded_crate_link_provider_exposes_object_inputs -- --exact
```

Expected: FAIL to compile because `downstream_metadata`, `downstream_bodies`, `downstream_link`, `DependencyBodies::bundle`, `DependencyLink::is_object_backed`, and `DependencyLink::object_path` are still undefined. When short `--exact` filters match 0 tests, rerun with full module paths like `crate_system::tests::loaded_crate_rejects_source_backed_dependency_providers -- --exact` and report both.

- [ ] **Step 3: Add provider structs and methods**

In `lib/src/crate_system/mod.rs`, update imports:

```rust
use crate::hir::HirImpl;
```

Add these provider structs below `LoadedCrate`:

```rust
pub(crate) struct DependencyMetadata<'a> {
    pub interface: &'a ArtifactCrateInterface,
    pub resolver: &'a ResolverTables,
    pub prelude_exports: &'a BTreeMap<String, String>,
}

pub(crate) struct DependencyBodies<'a> {
    bundle: Option<&'a ArtifactCrossCrateHir>,
}

impl<'a> DependencyBodies<'a> {
    pub(crate) fn bundle(&self) -> Option<&'a ArtifactCrossCrateHir> {
        self.bundle
    }
}

pub(crate) struct DependencyLink<'a> {
    object_backed: bool,
    object_path: Option<&'a PathBuf>,
}

impl<'a> DependencyLink<'a> {
    pub(crate) fn is_object_backed(&self) -> bool {
        self.object_backed
    }

    pub(crate) fn object_path(&self) -> Option<&'a PathBuf> {
        self.object_path
    }

    pub(crate) fn concrete_impl_body_is_object_provided(&self, imp: &HirImpl) -> bool {
        self.object_backed
            && imp.type_generics.is_empty()
            && imp.trait_generics.is_empty()
            && imp
                .methods
                .values()
                .all(|method| method.generic_params.is_empty())
    }
}
```

Extend the `impl LoadedCrate` block with:

```rust
    pub(crate) fn downstream_metadata(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<DependencyMetadata<'_>, String> {
        let interface = self.downstream_interface(crate_name, phase)?;
        Ok(DependencyMetadata {
            interface,
            resolver: &self.resolver,
            prelude_exports: &self.prelude_exports,
        })
    }

    pub(crate) fn downstream_bodies(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<DependencyBodies<'_>, String> {
        self.downstream_interface(crate_name, phase)?;
        Ok(DependencyBodies {
            bundle: self.cross_crate_hir.as_ref(),
        })
    }

    pub(crate) fn downstream_link(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<DependencyLink<'_>, String> {
        self.downstream_interface(crate_name, phase)?;
        Ok(DependencyLink {
            object_backed: self.is_object_backed(),
            object_path: self.object_path.as_ref(),
        })
    }
```

Keep `downstream_interface` in place during this task because existing phases still call it.

- [ ] **Step 4: Run provider tests green**

Run:

```bash
cargo test -p rock-lib crate_system::tests::loaded_crate_rejects_source_backed_dependency_providers -- --exact
cargo test -p rock-lib crate_system::tests::loaded_crate_metadata_provider_exposes_interface_and_prelude_exports -- --exact
cargo test -p rock-lib crate_system::tests::loaded_crate_body_provider_exposes_cross_crate_hir_bundle -- --exact
cargo test -p rock-lib crate_system::tests::loaded_crate_link_provider_exposes_object_inputs -- --exact
```

Expected: PASS.

- [ ] **Step 5: Commit Task 1**

```bash
git add lib/src/crate_system/mod.rs lib/src/crate_system/tests.rs
git commit -m "crate-system: expose dependency provider capabilities"
```

---

### Task 2: Route Collection Through Metadata And Link Providers

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add failing collect provider test**

Add this test to `lib/src/collect/mod.rs` near the existing artifact dependency registration tests:

```rust
#[test]
fn collect_registers_artifact_dependency_through_provider_capabilities() {
    let mut crate_ctx = CrateContext::new();
    let mut interface = crate::crate_artifact::ArtifactCrateInterface::default();
    interface.functions.insert(
        "dep::answer".to_string(),
        hir_function("answer"),
    );
    interface
        .root_exports
        .insert("answer".to_string(), "dep::answer".to_string());

    let loaded = LoadedCrate {
        manifest: crate::crate_system::CrateManifest {
            crate_: crate::crate_system::CrateConfig {
                name: "dep".to_string(),
                version: "0.1.0".to_string(),
                no_std: false,
            },
            lib: crate::crate_system::LibConfig {
                path: "lib.rk".to_string(),
            },
            dependencies: None,
        },
        ast: Module {
            name: None,
            top_levels: Vec::new(),
            is_inline: false,
            filepath: None,
        },
        interface: Some(interface),
        resolver: crate::collect::resolver::ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: None,
        cross_crate_hir: None,
        artifact_mode: crate::crate_system::ArtifactMode::InterfaceOnly,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };
    crate_ctx.add_crate("dep".to_string(), loaded);

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

- [ ] **Step 2: Run the collect provider test red**

Run:

```bash
cargo test -p rock-lib collect_registers_artifact_dependency_through_provider_capabilities -- --exact
```

Expected before implementation: the test may pass because behavior already works through `downstream_interface`; if it passes, proceed and use the grep gate in Step 5 as the red signal for this refactor. Record that the behavior test was already green.

- [ ] **Step 3: Route collect registration through providers**

In `lib/src/collect/context.rs`, replace the start of `register_loaded_crate` with:

```rust
        let metadata = match loaded_crate.downstream_metadata(crate_name, "collection") {
            Ok(metadata) => metadata,
            Err(message) => {
                self.push_error(message);
                return;
            }
        };
        let link = match loaded_crate.downstream_link(crate_name, "collection") {
            Ok(link) => link,
            Err(message) => {
                self.push_error(message);
                return;
            }
        };
        let interface = metadata.interface;
```

Replace prelude export access with:

```rust
        if is_stdlib && !metadata.prelude_exports.is_empty() {
            self.stdlib_prelude_exports = metadata
                .prelude_exports
                .iter()
                .map(|(name, source)| (name.clone(), Some(source.clone())))
                .collect();
        }
```

Replace the object-backed impl body computation with:

```rust
            let impl_body_is_object_provided =
                link.concrete_impl_body_is_object_provided(&registered_impl);
```

After this edit, `collect/context.rs` should not call `downstream_interface` or `is_object_backed` directly.

- [ ] **Step 4: Run collect tests green**

Run:

```bash
cargo test -p rock-lib collect_registers_artifact_dependency_through_provider_capabilities -- --exact
cargo test -p rock-lib collect::tests::collect_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib collect::tests::collect_indexes_loaded_source_backed_module_bodies -- --exact
cargo test -p rock-lib collect::tests::collect_accepts_artifact_dependency_without_registering_source_module_path -- --exact
```

Expected: PASS.

- [ ] **Step 5: Run collect grep gate**

Run:

```bash
rg "downstream_interface|loaded_crate\.is_object_backed\(\)|loaded_crate\.interface|loaded_crate\.prelude_exports" lib/src/collect/context.rs
```

Expected: no matches.

- [ ] **Step 6: Commit Task 2**

```bash
git add lib/src/collect/context.rs lib/src/collect/mod.rs
git commit -m "collect: use dependency provider capabilities"
```

---

### Task 3: Route Lowering Through Metadata And Body Providers

**Files:**
- Modify: `lib/src/lower/crates/registration.rs`
- Modify: `lib/src/lower/crates/bodies.rs`

- [ ] **Step 1: Add link provider regression test**

Update the `#[cfg(test)]` module imports in `lib/src/lower/crates/registration.rs`:

```rust
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::ast::Module;
use crate::collect::resolver::ResolverTables;
use crate::crate_artifact::ArtifactCrateInterface;
use crate::crate_system::{ArtifactMode, CrateConfig, CrateContext, CrateManifest, LibConfig};
use crate::crate_system::LoadedCrate;
use crate::hir::{HirBlock, HirFunction, HirImpl, HirImplOwner};
use crate::ids::{CrateId, DefId, LocalDefId};
use crate::lower::Lowerer;
use crate::types::Type;
```

Add this test to the same module:

```rust
#[test]
fn lower_registration_uses_link_provider_for_object_backed_impls() {
    let mut interface = ArtifactCrateInterface::default();
    let mut methods = HashMap::new();
    methods.insert(
        "show".to_string(),
        HirFunction {
            id: DefId::new(CrateId(0), LocalDefId(2)),
            name: "show".to_string(),
            qualified_name: Some("DepThing_show".to_string()),
            generic_params: Vec::new(),
            generic_bounds: HashMap::new(),
            params: Vec::new(),
            ret_type: Type::I64,
            body: HirBlock {
                stmts: Vec::new(),
                ty: Type::I64,
            },
            is_curried: false,
            is_method: true,
            self_receiver: None,
            is_unsafe: false,
        },
    );
    interface.impls.push(HirImpl {
        id: DefId::new(CrateId(0), LocalDefId(1)),
        owner: HirImplOwner::Named("DepThing".to_string()),
        type_name: "DepThing".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: None,
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: Vec::new(),
        methods,
    });

    let loaded_crate = LoadedCrate {
        manifest: CrateManifest {
            crate_: CrateConfig {
                name: "dep".to_string(),
                version: "0.1.0".to_string(),
                no_std: false,
            },
            lib: LibConfig {
                path: "lib.rk".to_string(),
            },
            dependencies: None,
        },
        ast: Module {
            name: None,
            top_levels: Vec::new(),
            is_inline: false,
            filepath: None,
        },
        interface: Some(interface),
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: Some(PathBuf::from("/dep/dep.o")),
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::Object,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };

    let mut lowerer = Lowerer::new();
    lowerer.register_loaded_crate("dep", &loaded_crate);

    let method = lowerer
        .methods
        .get(&("DepThing".to_string(), "show".to_string()))
        .expect("object-backed method should be registered");
    assert_eq!(method.qualified_name.as_deref(), Some("dep::DepThing_show"));
}
```

- [ ] **Step 2: Run lower provider regression red or already-green**

Run:

```bash
cargo test -p rock-lib lower_registration_uses_link_provider_for_object_backed_impls -- --exact
```

Expected before implementation: the test may pass because behavior already works through `is_object_backed`; if it passes, use the grep gate in Step 6 as the red signal for this refactor.

- [ ] **Step 3: Route lower registration through metadata/link providers**

In `lib/src/lower/crates/registration.rs`, replace `register_crate_resolvers` with:

```rust
    pub(crate) fn register_crate_resolvers(&mut self, ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &ctx.crates {
            let metadata = match loaded_crate.downstream_metadata(crate_name, "lowering") {
                Ok(metadata) => metadata,
                Err(message) => {
                    self.push_error_once(message);
                    continue;
                }
            };

            self.dependency_resolvers
                .insert(crate_name.clone(), metadata.resolver.clone());
        }
    }
```

In `lib/src/lower/crates/registration.rs`, replace the start of `register_loaded_crate` with:

```rust
        let metadata = match loaded_crate.downstream_metadata(crate_name, "lowering") {
            Ok(metadata) => metadata,
            Err(message) => {
                self.push_error_once(message);
                return;
            }
        };
        let link = match loaded_crate.downstream_link(crate_name, "lowering") {
            Ok(link) => link,
            Err(message) => {
                self.push_error_once(message);
                return;
            }
        };
        let interface = metadata.interface;
```

Replace stdlib prelude export access with:

```rust
        if is_stdlib && !metadata.prelude_exports.is_empty() {
            self.stdlib_prelude_exports = metadata
                .prelude_exports
                .iter()
                .map(|(name, source)| (name.clone(), Some(source.clone())))
                .collect();
        }
```

Replace the object-backed impl body computation with:

```rust
            let impl_body_is_object_provided =
                link.concrete_impl_body_is_object_provided(&registered_impl);
```

- [ ] **Step 4: Route lower body loading through body providers**

In `lib/src/lower/crates/bodies.rs`, replace `lower_crate_trait_bodies` with:

```rust
    pub(crate) fn lower_crate_trait_bodies(&mut self, ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &ctx.crates {
            let bodies = match loaded_crate.downstream_bodies(crate_name, "lowering") {
                Ok(bodies) => bodies,
                Err(message) => {
                    self.push_error_once(message);
                    continue;
                }
            };

            if let Some(bundle) = bodies.bundle() {
                self.apply_cross_crate_trait_defaults(bundle);
            }
        }
    }
```

Replace `lower_crate_module_bodies` with:

```rust
    pub(crate) fn lower_crate_module_bodies(&mut self, ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &ctx.crates {
            let bodies = match loaded_crate.downstream_bodies(crate_name, "lowering") {
                Ok(bodies) => bodies,
                Err(message) => {
                    self.push_error_once(message);
                    continue;
                }
            };

            if let Some(bundle) = bodies.bundle() {
                self.apply_cross_crate_generic_bodies(bundle);
            }
        }
    }
```

Keep the direct `use crate::crate_artifact::ArtifactCrossCrateHir;` import because `apply_cross_crate_trait_defaults` and `apply_cross_crate_generic_bodies` still accept `&ArtifactCrossCrateHir`.

- [ ] **Step 5: Run lower tests green**

Run:

```bash
cargo test -p rock-lib lower_registration_uses_link_provider_for_object_backed_impls -- --exact
cargo test -p rock-lib lower_from_declarations_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_function_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_impl_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_trait_default_method_from_product_artifact -- --exact
```

Expected: PASS.

- [ ] **Step 6: Run lower grep gate**

Run:

```bash
rg "downstream_interface|loaded_crate\.is_object_backed\(\)|loaded_crate\.interface|loaded_crate\.resolver|loaded_crate\.prelude_exports|loaded_crate\.cross_crate_hir" lib/src/lower
```

Expected: no production-code matches. Test fixture construction matches under `#[cfg(test)]` are acceptable if they are constructing `LoadedCrate` values.

- [ ] **Step 7: Commit Task 3**

```bash
git add lib/src/lower/crates/registration.rs lib/src/lower/crates/bodies.rs
git commit -m "lower: use dependency provider capabilities"
```

---

### Task 4: Route Monomorphization Through Body And Link Providers

**Files:**
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mono/mod.rs`

- [ ] **Step 1: Add mono provider regression test**

Add these helpers and this test to the `#[cfg(test)] mod tests` in `lib/src/mono/external.rs`, near the other object-backed dependency tests:

```rust
fn simple_hir_function(name: &str, qualified_name: Option<&str>, id: DefId) -> HirFunction {
    HirFunction {
        id,
        name: name.to_string(),
        qualified_name: qualified_name.map(str::to_string),
        generic_params: vec![],
        generic_bounds: HashMap::new(),
        params: vec![],
        ret_type: Type::I32,
        body: HirBlock {
            stmts: vec![],
            ty: Type::I32,
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    }
}

fn loaded_object_backed_function_crate(
    crate_name: &str,
    qualified_name: &str,
    func: HirFunction,
    object_path: PathBuf,
) -> LoadedCrate {
    let mut interface = ArtifactCrateInterface::default();
    interface
        .functions
        .insert(qualified_name.to_string(), func);

    let mut resolver = ResolverTables::default();
    let def_id = DefId::new(CrateId(0), LocalDefId(0));
    resolver
        .item_paths
        .insert(qualified_name.to_string(), def_id);
    resolver
        .item_names_by_id
        .insert(def_id, qualified_name.to_string());

    LoadedCrate {
        manifest: CrateManifest {
            crate_: CrateConfig {
                name: crate_name.to_string(),
                version: "0.1.0".to_string(),
                no_std: false,
            },
            lib: LibConfig {
                path: "lib.rk".to_string(),
            },
            dependencies: None,
        },
        ast: Module {
            name: None,
            top_levels: vec![],
            is_inline: false,
            filepath: None,
        },
        interface: Some(interface),
        resolver,
        prelude_exports: BTreeMap::new(),
        object_path: Some(object_path),
        cross_crate_hir: Some(ArtifactCrossCrateHir {
            generic_functions: BTreeMap::new(),
            traits_with_defaults: BTreeMap::new(),
            generic_impls: vec![],
        }),
        artifact_mode: ArtifactMode::Object,
        root_dir: PathBuf::new(),
        module_tree: None,
        file_cache: HashMap::new(),
    }
}

#[test]
fn process_with_crates_uses_link_provider_for_object_backed_functions() {
    let answer_id = DefId::new(CrateId(0), LocalDefId(0));
    let func = simple_hir_function("answer", Some("dep::answer"), answer_id);
    let loaded = loaded_object_backed_function_crate(
        "dep",
        "dep::answer",
        func,
        PathBuf::from("dep.o"),
    );
    let mut crate_ctx = CrateContext::new();
    crate_ctx.add_crate("dep".to_string(), loaded);

    let program = crate::hir::HirProgram::from_parts(
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );

    let mut mono = Monomorphizer::new();
    let output = wrapped_process_with_crates(&mut mono, program, &crate_ctx);

    assert!(output
        .instances
        .values()
        .any(|record| record.backend_symbol == "dep::answer" && record.provided_by_object));
}
```

- [ ] **Step 2: Run mono provider test red or already-green**

Run:

```bash
cargo test -p rock-lib process_with_crates_uses_link_provider_for_object_backed_functions -- --exact
```

Expected before implementation: the behavior may already pass through direct object-backed checks. The grep gate in Step 5 is the red signal for this refactor.

- [ ] **Step 3: Replace direct object/interface reads in mono**

In `lib/src/mono/mod.rs`, remove the eager dependency resolver preload from `monomorphize_with_crates`:

```rust
    let mut mono = Monomorphizer::new();
    mono.resolver = program.resolver;
    let program = mono.process_with_crates(program.program, crate_ctx);
```

In `process_with_crates_impl`, replace the `object_backed_impls` computation and dependency resolver assignment with provider-backed logic:

```rust
        let object_backed_impls = crate_ctx
            .crates
            .iter()
            .flat_map(|(crate_name, loaded_crate)| {
                let metadata = match loaded_crate.downstream_metadata(crate_name, "monomorphization") {
                    Ok(metadata) => metadata,
                    Err(message) => panic!("{}", message),
                };
                let link = match loaded_crate.downstream_link(crate_name, "monomorphization") {
                    Ok(link) => link,
                    Err(message) => panic!("{}", message),
                };

                metadata
                    .interface
                    .impls
                    .iter()
                    .filter(|imp| link.concrete_impl_body_is_object_provided(imp))
                    .map(Self::impl_signature_key)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        self.set_object_backed_impls(object_backed_impls);
        self.dependency_resolvers = crate_ctx
            .crates
            .iter()
            .map(|(crate_name, loaded_crate)| {
                loaded_crate
                    .downstream_metadata(crate_name, "monomorphization")
                    .unwrap_or_else(|message| panic!("{}", message))
                    .resolver
                    .clone()
            })
            .collect();
```

In `register_object_backed_functions`, replace direct mode/interface access with:

```rust
        for (crate_name, loaded_crate) in &crate_ctx.crates {
            let metadata = match loaded_crate.downstream_metadata(crate_name, "monomorphization") {
                Ok(metadata) => metadata,
                Err(message) => panic!("{}", message),
            };
            let link = match loaded_crate.downstream_link(crate_name, "monomorphization") {
                Ok(link) => link,
                Err(message) => panic!("{}", message),
            };
            if !link.is_object_backed() {
                continue;
            }

            for (name, func) in &metadata.interface.functions {
                let origin = self.function_instance_origin(name, func);
                let instance_key = crate::mono::InstanceKey::new(origin.clone(), Vec::new());
                let mut declared = func.clone();
                declared.name = name.clone();

                self.instances
                    .intern(instance_key, |id| crate::mono::InstanceRecord {
                        id,
                        origin: origin.clone(),
                        substitution: Vec::new(),
                        source_name: name.clone(),
                        backend_symbol: name.clone(),
                        declared: Some(declared.clone()),
                        body: None,
                        provided_by_object: true,
                    });
            }
        }
```

Replace `register_object_backed_instances` with:

```rust
    fn register_object_backed_instances(
        &mut self,
        program_impls: &[crate::hir::HirImpl],
        crate_ctx: &CrateContext,
    ) {
        for (crate_name, loaded_crate) in &crate_ctx.crates {
            let metadata = match loaded_crate.downstream_metadata(crate_name, "monomorphization") {
                Ok(metadata) => metadata,
                Err(message) => panic!("{}", message),
            };
            let link = match loaded_crate.downstream_link(crate_name, "monomorphization") {
                Ok(link) => link,
                Err(message) => panic!("{}", message),
            };
            if !link.is_object_backed() {
                continue;
            }

            for imp in &metadata.interface.impls {
                if link.concrete_impl_body_is_object_provided(imp) {
                    self.record_object_backed_impl(crate_name, imp);
                }
            }
        }

        for imp in program_impls {
            self.record_object_backed_impl("", imp);
        }
    }
```

Replace `load_external_generic_functions` with:

```rust
    fn load_external_generic_functions(&mut self, crate_ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &crate_ctx.crates {
            let metadata = match loaded_crate.downstream_metadata(crate_name, "monomorphization") {
                Ok(metadata) => metadata,
                Err(message) => panic!("{}", message),
            };
            let link = match loaded_crate.downstream_link(crate_name, "monomorphization") {
                Ok(link) => link,
                Err(message) => panic!("{}", message),
            };
            let bodies = match loaded_crate.downstream_bodies(crate_name, "monomorphization") {
                Ok(bodies) => bodies,
                Err(message) => panic!("{}", message),
            };

            if link.is_object_backed() {
                for imp in &metadata.interface.impls {
                    if let Some(trait_name) = &imp.trait_name {
                        if link.concrete_impl_body_is_object_provided(imp) {
                            self.trait_impls
                                .entry(trait_name.clone())
                                .or_insert_with(Vec::new)
                                .push(imp.clone());
                        }
                    }
                }
            }

            if let Some(bundle) = bodies.bundle() {
                for (qualified_name, func) in &bundle.generic_functions {
                    self.external_generic_functions
                        .insert(qualified_name.clone(), func.clone());
                }

                for imp in &bundle.generic_impls {
                    if let Some(trait_name) = &imp.trait_name {
                        self.trait_impls
                            .entry(trait_name.clone())
                            .or_insert_with(Vec::new)
                            .push(imp.clone());
                    } else {
                        self.generic_impls
                            .insert(Self::generic_impl_key(imp), imp.clone());
                    }
                }
            }
        }
    }
```

- [ ] **Step 4: Run mono tests green**

Run:

```bash
cargo test -p rock-lib process_with_crates_uses_link_provider_for_object_backed_functions -- --exact
cargo test -p rock-lib mono::external::tests::process_with_crates_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib mono::external::tests::load_external_generic_functions_keeps_generic_trait_impls_out_of_inherent_table -- --exact
cargo test -p rock-lib process_with_crates_records_object_backed_instances_without_re_emitting -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_function_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_impl_from_artifact_hir_bundle -- --exact
```

Expected: PASS.

- [ ] **Step 5: Run mono grep gate**

Run:

```bash
rg "loaded_crate\.is_object_backed\(\)|loaded_crate\.interface|loaded_crate\.resolver|loaded_crate\.cross_crate_hir|downstream_interface" lib/src/mono/external.rs lib/src/mono/mod.rs
```

Expected: no production-code matches. Test helper construction matches are acceptable.

- [ ] **Step 6: Commit Task 4**

```bash
git add lib/src/mono/external.rs lib/src/mono/mod.rs
git commit -m "mono: use dependency provider capabilities"
```

---

### Task 5: Route Codegen Setup And Linking Through Link Providers

**Files:**
- Modify: `lib/src/crate_system/context.rs`
- Modify: `lib/src/crate_system/tests.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add crate-context link input tests**

Add this test to `lib/src/crate_system/tests.rs`:

```rust
#[test]
fn crate_context_collects_dependency_link_inputs_from_provider_capabilities() {
    let mut ctx = CrateContext::new();
    let loaded = LoadedCrate {
        manifest: empty_manifest("dep"),
        ast: empty_module(),
        interface: Some(crate::crate_artifact::ArtifactCrateInterface::default()),
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: Some(PathBuf::from("/dep/dep.o")),
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::Object,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };
    ctx.add_crate("dep".to_string(), loaded);

    let inputs = ctx
        .dependency_link_inputs("codegen")
        .expect("object-backed dependency should expose link inputs");

    assert_eq!(inputs.object_crate_names, vec!["dep".to_string()]);
    assert_eq!(inputs.object_paths, vec![PathBuf::from("/dep/dep.o")]);
}
```

Add this test too:

```rust
#[test]
fn crate_context_link_inputs_reject_source_backed_dependencies() {
    let mut ctx = CrateContext::new();
    ctx.register_crate(empty_manifest("dep"), PathBuf::from("/dep"), empty_module());

    let error = ctx
        .dependency_link_inputs("codegen")
        .expect_err("source-backed dependency link inputs should be rejected");

    assert!(error.contains("source-backed external dependency 'dep'"));
}
```

- [ ] **Step 2: Run link input tests red**

Run:

```bash
cargo test -p rock-lib crate_context_collects_dependency_link_inputs_from_provider_capabilities -- --exact
cargo test -p rock-lib crate_context_link_inputs_reject_source_backed_dependencies -- --exact
```

Expected: FAIL to compile because `CrateContext::dependency_link_inputs` does not exist.

- [ ] **Step 3: Add provider-backed link input aggregation**

In `lib/src/crate_system/context.rs`, add this struct near the `impl CrateContext` block:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencyLinkInputs {
    pub object_crate_names: Vec<String>,
    pub object_paths: Vec<PathBuf>,
}
```

Add this method to `impl CrateContext`:

```rust
    pub(crate) fn dependency_link_inputs(
        &self,
        phase: &str,
    ) -> Result<DependencyLinkInputs, String> {
        let mut object_crate_names = Vec::new();
        let mut object_paths = Vec::new();

        for (crate_name, loaded_crate) in &self.crates {
            let link = loaded_crate.downstream_link(crate_name, phase)?;
            if link.is_object_backed() {
                object_crate_names.push(crate_name.clone());
                if let Some(path) = link.object_path() {
                    object_paths.push(path.clone());
                }
            }
        }

        Ok(DependencyLinkInputs {
            object_crate_names,
            object_paths,
        })
    }
```

After Step 4 updates the only caller, remove `get_object_paths` from `impl CrateContext`.

- [ ] **Step 4: Route `lib.rs` through link inputs**

In `lib/src/lib.rs`, replace object crate name and object path setup with:

```rust
    let link_inputs = match crate_ctx.dependency_link_inputs("codegen") {
        Ok(inputs) => inputs,
        Err(message) => {
            let diagnostics = Diagnostics::from(vec![diagnostic::Diagnostic::new(
                message,
                Span::default(),
            )]);
            return Err(diagnostics);
        }
    };
    codegen.set_object_crate_names(link_inputs.object_crate_names.clone());
```

Then replace:

```rust
        let crate_objects = crate_ctx.get_object_paths();
        if let Err(e) = codegen.write_executable(&exe_path, opt, &crate_objects) {
```

with:

```rust
        if let Err(e) = codegen.write_executable(&exe_path, opt, &link_inputs.object_paths) {
```

- [ ] **Step 5: Run link/codegen tests green**

Run:

```bash
cargo test -p rock-lib crate_context_collects_dependency_link_inputs_from_provider_capabilities -- --exact
cargo test -p rock-lib crate_context_link_inputs_reject_source_backed_dependencies -- --exact
cargo test -p rock-lib crate_artifact::tests::test_run_with_stdlib_product_artifact_links -- --exact
cargo test -p rock tests::build::test_build_project_with_transitive_artifacts -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run codegen setup grep gate**

Run:

```bash
rg "loaded_crate\.is_object_backed\(\)|get_object_paths\(" lib/src/lib.rs lib/src/crate_system/context.rs
```

Expected: no matches.

- [ ] **Step 7: Commit Task 5**

```bash
git add lib/src/crate_system/context.rs lib/src/crate_system/tests.rs lib/src/lib.rs
git commit -m "codegen: use dependency link providers"
```

---

### Task 6: Remove Remaining Downstream Storage-Mode Branching And Update Audit Status

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify production files only if grep finds remaining downstream storage detail reads.

- [ ] **Step 1: Run full provider grep audit**

Run:

```bash
rg "downstream_interface|loaded_crate\.is_object_backed\(\)|artifact_mode|loaded_crate\.interface|loaded_crate\.resolver|loaded_crate\.cross_crate_hir|loaded_crate\.object_path" lib/src/collect lib/src/lower lib/src/mono lib/src/lib.rs
```

Expected: no production-code matches. Matches in tests that construct `LoadedCrate` fixtures are acceptable. Matches inside `crate_system` are acceptable because it owns storage and providers.

- [ ] **Step 2: Fix any remaining production matches**

If Step 1 finds production matches in downstream phases, replace them with provider APIs:

```rust
let metadata = loaded_crate.downstream_metadata(crate_name, "<phase>")?;
let bodies = loaded_crate.downstream_bodies(crate_name, "<phase>")?;
let link = loaded_crate.downstream_link(crate_name, "<phase>")?;
```

Use the existing phase error handling style:

- `collect`: `self.push_error(message); return;`
- `lower`: `self.push_error_once(message); return;` or `continue;` in body loops
- `mono`: `panic!("{}", message)`
- `lib.rs`: convert provider error into `Diagnostics`

- [ ] **Step 3: Update audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, under `## 8. Crate And Artifact Interface Split`, add this checked item under `Done:`:

```markdown
- [x] Hid downstream dependency metadata, body, and link access behind provider capabilities instead of branching through loaded-crate storage details in collect/lower/mono/codegen setup.
- [x] Made cross-crate body access explicit through the dependency body provider API.
```

Keep this item in `Still to do:` because `LoadedCrate` still stores all concerns internally:

```markdown
- [ ] Split `LoadedCrate` into narrower metadata, interface, body-provider, and link-provider capabilities.
```

- [ ] **Step 4: Run focused provider regression tests**

Run:

```bash
cargo test -p rock-lib loaded_crate_rejects_source_backed_dependency_providers -- --exact
cargo test -p rock-lib collect_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib lower_from_declarations_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib process_with_crates_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib crate_context_collects_dependency_link_inputs_from_provider_capabilities -- --exact
```

Expected: PASS. Use full module paths if short filters match 0 tests.

- [ ] **Step 5: Run artifact and current-crate source regressions**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_indexes_loaded_source_backed_module_bodies -- --exact
cargo test -p rock-lib collect::tests::lower_from_declarations_resolves_nested_source_backed_imports_inside_inline_module_bodies -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_with_source_free_product_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_trait_default_method_from_product_artifact -- --exact
cargo test -p rock tests::artifact::test_collect_dependency_artifacts_includes_transitive_dependencies -- --exact --nocapture
cargo test -p rock tests::build::test_build_project_with_transitive_artifacts -- --exact --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run final verification**

Run:

```bash
cargo fmt --all --check
cargo test -p rock-lib crate_artifact
cargo test -p rock-lib products
cargo test -p rock
cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit Task 6**

```bash
git add docs/superpowers/plans/master-audit-checklist.md lib/src/collect lib/src/lower lib/src/mono lib/src/lib.rs lib/src/crate_system
git commit -m "docs: mark dependency provider boundary cleanup"
```

Before committing, inspect `git status --short` and do not stage unrelated files such as `stdlib/vec.rk`.

---

## Self-Review Checklist

- Spec coverage: Task 1 adds metadata/body/link provider capabilities; Tasks 2-5 route collect/lower/mono/codegen through them; Task 6 performs grep gates, updates audit status, and runs final verification.
- Scope control: The plan does not redesign product artifacts, does not remove current-crate source compilation, does not implement transitive artifact resolution in `rockc`, and does not resume canonical identity work.
- Type consistency: Provider names are consistently `DependencyMetadata`, `DependencyBodies`, `DependencyLink`, `downstream_metadata`, `downstream_bodies`, `downstream_link`, and `dependency_link_inputs`.
- Verification: The plan includes red/green tests for provider APIs, focused artifact/current-crate regressions, grep gates, and a final bare `cargo test` workspace run.
