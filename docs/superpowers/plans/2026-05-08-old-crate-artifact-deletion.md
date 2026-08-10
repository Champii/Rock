# Old CrateArtifact Deletion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the old `CrateArtifact` system after adapting useful legacy coverage to product artifacts.

**Architecture:** Add product-backed characterization tests first while old tests still exist, then delete old artifact containers/builders/loaders/helpers and old-only tests in a separate commit. Keep the product artifact adapter that converts `CompilerProducts` into `LoadedCrate`; do not add product source bundles.

**Tech Stack:** Rust 2021, Cargo workspace, `rock-lib`, `rockc`, `rock`, `rockup`, `bincode`/`serde` product artifact serialization.

---

## File Structure

- Modify: `lib/src/crate_artifact/tests.rs`
  - Add product-backed coverage for associated types, stdlib method ABI/prelude exports, extern IDs, and trait defaults.
  - Delete old `CrateArtifact` tests/helpers once product coverage exists.
- Modify: `lib/src/crate_artifact/load.rs`
  - Remove old `load_artifact_from_path` and `load_artifact_from_path_with_root`.
  - Keep product artifact loading and product-to-`LoadedCrate` conversion.
- Modify: `lib/src/crate_artifact/types.rs`
  - Remove `CrateArtifact`, old identity/fingerprint/source bundle/object output structs.
  - Keep shared interface structs still used by product artifact loading.
- Modify: `lib/src/crate_artifact/mod.rs`
  - Stop compiling/exporting old builder/helper modules.
  - Export only product-loading interface structs that remain needed by `crate_system`/product loader.
- Delete: `lib/src/crate_artifact/build.rs`
  - Old `CrateContext::build_artifact` implementation.
- Delete: `lib/src/crate_artifact/helpers.rs`
  - Old artifact-builder helper code.
- Modify: `lib/src/crate_system/mod.rs`
  - Remove imports or enum variants only if old deletion leaves them unused. Keep `ArtifactMode::Source` for normal source crates and `ArtifactMode::Object` for product-backed crates.
- Modify only if needed: `lib/src/collect/mod.rs`, `lib/src/mono/external.rs`
  - Keep existing test helpers compiling if they use `ArtifactCrateInterface`; do not add old artifact compatibility.

---

### Task 1: Add Product Metadata Coverage for Old Interface Assertions

**Files:**
- Modify: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Add product metadata tests before deleting old tests**

Insert these tests near the existing product artifact tests in `lib/src/crate_artifact/tests.rs`, after `test_source_backed_artifact_preserves_concrete_bodies`.

```rust
#[test]
fn test_product_artifact_preserves_associated_types() {
    let _guard = artifact_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = temp_test_dir("product_artifact_associated_types");
    let dep_dir = temp_dir.join("dep");
    let artifact_path = temp_dir.join("dep.rkca");

    write_crate(
        &dep_dir,
        "dep",
        "",
        r#"< trait Deref
    type Target
    @deref: () -> &Self::Target

struct Box T
    value: T

impl Deref for Box T
    type Target = T
    @deref = -> &@value
"#,
    );

    build_product_artifact(&dep_dir, "dep", &artifact_path, true, true);
    let products = CompilerProducts::read_artifact_from_path(&artifact_path).unwrap();

    let deref_trait = products
        .metadata
        .traits
        .values()
        .find(|trait_| trait_.name == "dep::Deref")
        .unwrap();
    assert_eq!(deref_trait.associated_types.len(), 1);
    assert_eq!(deref_trait.associated_types[0].name, "Target");
    assert_eq!(
        deref_trait.signatures.get("deref").unwrap().ret,
        Type::Reference {
            mutable: false,
            inner: Box::new(Type::Projection {
                ty: Box::new(Type::Generic("Self".to_string())),
                trait_name: "Deref".to_string(),
                assoc_name: "Target".to_string(),
                trait_args: vec![],
            }),
        }
    );

    let deref_impl = products
        .metadata
        .impls
        .values()
        .find(|imp| imp.trait_name.as_deref() == Some("Deref") && imp.type_name == "Box")
        .unwrap();
    assert_eq!(deref_impl.associated_types.len(), 1);
    assert_eq!(deref_impl.associated_types[0].name, "Target");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_product_artifact_interface_externs_use_resolved_def_ids() {
    let _guard = artifact_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = temp_test_dir("product_artifact_extern_def_ids");
    let dep_dir = temp_dir.join("dep");
    let artifact_path = temp_dir.join("dep.rkca");

    write_crate(
        &dep_dir,
        "dep",
        "",
        "< marker = -> 0\nextern puts: Str -> I32\n",
    );

    build_product_artifact(&dep_dir, "dep", &artifact_path, true, true);
    let products = CompilerProducts::read_artifact_from_path(&artifact_path).unwrap();

    let extern_fn = products
        .metadata
        .externs
        .values()
        .find(|ext| ext.name == "dep::puts")
        .unwrap_or_else(|| {
            panic!(
                "available externs: {:?}",
                products
                    .metadata
                    .externs
                    .values()
                    .map(|ext| ext.name.clone())
                    .collect::<Vec<_>>()
            )
        });
    let placeholder = DefId::new(CrateId(0), LocalDefId(0));
    assert_ne!(extern_fn.id, placeholder);

    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run tests and confirm product coverage is green**

Run:

```bash
cargo test -p rock-lib test_product_artifact_preserves_associated_types -- --exact --nocapture
cargo test -p rock-lib test_product_artifact_interface_externs_use_resolved_def_ids -- --exact --nocapture
```

Expected: both tests pass. If either fails, fix product metadata generation in `lib/src/products.rs` before deleting old tests.

- [ ] **Step 3: Commit product metadata coverage**

```bash
git add lib/src/crate_artifact/tests.rs lib/src/products.rs
git commit -m "tests: cover product artifact metadata"
```

---

### Task 2: Add Product Stdlib ABI and Prelude Coverage

**Files:**
- Modify: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Add product stdlib metadata assertions**

Add this test after `test_run_with_stdlib_product_artifact_links`.

```rust
#[test]
fn test_product_stdlib_artifact_preserves_string_method_abi_and_prelude_exports() {
    let _guard = artifact_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let stdlib_dir = workspace_root().join("stdlib");
    let temp_dir = temp_test_dir("product_stdlib_string_abi");
    let artifact_path = temp_dir.join("stdlib.rkca");

    build_stdlib_product_artifact(&stdlib_dir, &artifact_path);
    let products = CompilerProducts::read_artifact_from_path(&artifact_path).unwrap();

    assert_eq!(products.crate_identity.name, "stdlib");
    assert_eq!(
        products.prelude_exports.get("sqrt").map(String::as_str),
        Some("stdlib::libc::sqrt")
    );
    assert_eq!(
        products.prelude_exports.get("Deref").map(String::as_str),
        Some("stdlib::deref::Deref")
    );

    let string_impls = products
        .metadata
        .impls
        .values()
        .filter(|imp| imp.type_name == "String" && imp.trait_name.is_none())
        .collect::<Vec<_>>();
    let from_str = string_impls
        .iter()
        .find_map(|imp| imp.methods.get("from_str"))
        .unwrap();
    let concat = string_impls
        .iter()
        .find_map(|imp| imp.methods.get("concat"))
        .unwrap();

    assert_eq!(from_str.ret_type, Type::Struct("String".to_string(), vec![]));
    assert_eq!(concat.ret_type, Type::Struct("String".to_string(), vec![]));
    assert_eq!(from_str.params[0].ty, Type::Slice(Box::new(Type::U8)));
    assert_eq!(concat.params[1].ty, Type::Struct("String".to_string(), vec![]));
    assert!(products
        .metadata
        .traits
        .values()
        .any(|trait_| trait_.name == "stdlib::deref::Deref"));

    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run the focused stdlib product test**

Run:

```bash
cargo test -p rock-lib test_product_stdlib_artifact_preserves_string_method_abi_and_prelude_exports -- --exact --nocapture
```

Expected: PASS. If it fails because product names or prelude exports are missing, fix product metadata/prelude serialization in `lib/src/products.rs` or product collection code.

- [ ] **Step 3: Commit product stdlib coverage**

```bash
git add lib/src/crate_artifact/tests.rs lib/src/products.rs
git commit -m "tests: cover product stdlib metadata"
```

---

### Task 3: Add Product Trait Default Coverage

**Files:**
- Modify: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Add product tests for trait defaults**

Add these tests near the generic product artifact tests.

```rust
#[test]
fn test_product_artifact_preserves_trait_default_methods() {
    let _guard = artifact_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = temp_test_dir("product_trait_defaults");
    let dep_dir = temp_dir.join("dep");
    let artifact_path = temp_dir.join("dep.rkca");

    write_crate(
        &dep_dir,
        "dep",
        "",
        "trait Animal\n    @legs = -> 4\n< Animal\n",
    );

    build_product_artifact(&dep_dir, "dep", &artifact_path, true, true);
    let products = CompilerProducts::read_artifact_from_path(&artifact_path).unwrap();

    let animal = products
        .metadata
        .traits
        .values()
        .find(|trait_| trait_.name == "dep::Animal")
        .unwrap();
    assert!(animal.methods.contains_key("legs"));
    assert!(products
        .bodies
        .trait_default_methods
        .values()
        .any(|function| function.name == "legs" || function.name == "dep::Animal::legs"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_product_artifact_preserves_file_module_trait_default_methods() {
    let _guard = artifact_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = temp_test_dir("product_file_module_trait_defaults");
    let dep_dir = temp_dir.join("dep");
    let artifact_path = temp_dir.join("dep.rkca");

    write_crate(&dep_dir, "dep", "", "< mod animals\n");
    fs::write(
        dep_dir.join("animals.rk"),
        "< trait Animal\n    @legs = -> 4\n< Animal\n",
    )
    .unwrap();

    build_product_artifact(&dep_dir, "dep", &artifact_path, true, true);
    let products = CompilerProducts::read_artifact_from_path(&artifact_path).unwrap();

    let animal = products
        .metadata
        .traits
        .values()
        .find(|trait_| trait_.name == "dep::animals::Animal")
        .unwrap();
    assert!(animal.methods.contains_key("legs"));
    assert!(products
        .bodies
        .trait_default_methods
        .values()
        .any(|function| function.name.contains("legs")));

    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run the trait default product tests**

Run:

```bash
cargo test -p rock-lib test_product_artifact_preserves_trait_default_methods -- --exact --nocapture
cargo test -p rock-lib test_product_artifact_preserves_file_module_trait_default_methods -- --exact --nocapture
```

Expected: PASS. If a test fails because default bodies are not serialized into product artifacts, fix product body collection in `lib/src/products.rs` before proceeding.

- [ ] **Step 3: Commit product trait default coverage**

```bash
git add lib/src/crate_artifact/tests.rs lib/src/products.rs
git commit -m "tests: cover product trait defaults"
```

---

### Task 4: Delete Old-Only Tests and Helpers

**Files:**
- Modify: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Remove old-only imports**

In `lib/src/crate_artifact/tests.rs`, remove old-only imports from the top of the file.

Remove:

```rust
use crate::crate_system::{ArtifactMode, CrateContext};
use crate::infer;
use crate::lower::Lowerer;
use crate::parser;

use super::{ArtifactObjectOutput, CrateArtifact, CRATE_ARTIFACT_FORMAT_VERSION};
```

Replace with:

```rust
use crate::crate_system::CrateContext;
```

Keep these imports if tests still use them:

```rust
use crate::ids::{CrateId, DefId, LocalDefId};
use crate::products::CompilerProducts;
use crate::types::Type;
use crate::Config;
```

- [ ] **Step 2: Delete old-only helper functions**

Delete these helper functions from `lib/src/crate_artifact/tests.rs`:

```rust
fn strip_source_bundle_bodies(artifact: &mut CrateArtifact) { /* entire function */ }
fn strip_source_bundle(artifact: &mut CrateArtifact) { /* entire function */ }
fn compile_crate_object(crate_dir: &PathBuf) -> PathBuf { /* entire function */ }
fn build_object_backed_artifact(crate_dir: &PathBuf, crate_name: &str) -> (CrateArtifact, PathBuf) { /* entire function */ }
```

- [ ] **Step 3: Delete tests that only describe the old artifact system**

Remove these test functions completely:

```rust
fn test_loaded_artifact_modes_match_available_bodies() { /* entire function */ }
fn test_legacy_old_artifact_build_stdlib_artifact_exports() { /* entire function */ }
fn test_build_artifact_with_missing_root_filepath_preserves_source_backed_modules() { /* entire function */ }
fn test_build_cross_crate_hir_with_missing_root_filepath_preserves_source_backed_modules() { /* entire function */ }
fn test_build_artifact_updates_free_function_abi_after_lowering() { /* entire function */ }
fn test_build_artifact_interface_externs_use_resolved_def_ids() { /* entire function */ }
fn test_legacy_old_artifact_build_stdlib_artifact_exports_deref_trait() { /* entire function */ }
fn test_legacy_old_artifact_register_object_backed_stdlib_preserves_string_method_abi() { /* entire function */ }
fn test_legacy_old_artifact_finalize_with_object_backed_stdlib_preserves_string_method_abi() { /* entire function */ }
fn test_legacy_old_artifact_stdlib_roundtrip_binary() { /* entire function */ }
fn test_artifact_roundtrip_preserves_associated_types() { /* entire function */ }
fn test_build_artifacts_dependency_identity() { /* entire function */ }
fn test_legacy_old_artifact_glob_import_with_interface_only_artifact() { /* entire function */ }
fn test_legacy_old_artifact_compile_with_interface_only_stdlib_artifact() { /* entire function */ }
fn test_legacy_old_artifact_trait_default_hir_bundle() { /* entire function */ }
fn test_legacy_old_artifact_trait_default_file_module_hir_bundle() { /* entire function */ }
fn test_build_cross_crate_hir_preserves_dependency_trait_defaults_for_current_generic_impls() { /* entire function */ }
```

Do not delete existing product tests such as:

```rust
fn test_compile_with_stdlib_artifact() { /* keep */ }
fn test_run_with_stdlib_product_artifact_links() { /* keep */ }
fn test_source_backed_artifact_preserves_concrete_bodies() { /* keep; rename later if desired */ }
fn test_compile_with_interface_only_artifact() { /* keep; currently product-backed source-free dependency coverage */ }
fn test_compile_generic_function_from_artifact_hir_bundle() { /* keep */ }
fn test_compile_generic_function_from_file_module_artifact_hir_bundle() { /* keep */ }
fn test_compile_generic_impl_from_artifact_hir_bundle() { /* keep */ }
fn test_compile_generic_impl_from_file_module_artifact_hir_bundle() { /* keep */ }
```

- [ ] **Step 4: Rename misleading product tests**

Rename product-backed tests whose names still mention old modes:

```rust
fn test_source_backed_artifact_preserves_concrete_bodies()
```

to:

```rust
fn test_product_artifact_preserves_concrete_function_bodies()
```

Rename:

```rust
fn test_compile_with_interface_only_artifact()
```

to:

```rust
fn test_compile_with_source_free_product_artifact()
```

- [ ] **Step 5: Run tests after deleting old-only tests**

Run:

```bash
cargo test -p rock-lib crate_artifact -- --nocapture
```

Expected: PASS. The test count should shrink. Failures mentioning `CrateArtifact`, `build_artifact`, `load_artifact_from_path`, `ArtifactObjectOutput`, or `CRATE_ARTIFACT_FORMAT_VERSION` mean an old-only test/helper remains.

- [ ] **Step 6: Commit test deletion**

```bash
git add lib/src/crate_artifact/tests.rs
git commit -m "tests: remove old artifact coverage"
```

---

### Task 5: Delete Old Artifact Implementation

**Files:**
- Delete: `lib/src/crate_artifact/build.rs`
- Delete: `lib/src/crate_artifact/helpers.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_artifact/mod.rs`
- Modify: `lib/src/crate_artifact/types.rs`

- [ ] **Step 1: Remove old modules from `mod.rs`**

Edit `lib/src/crate_artifact/mod.rs` to remove old builder/helper modules and old exports.

Replace the file with:

```rust
mod load;
#[cfg(test)]
mod tests;
mod types;

pub use types::{ArtifactCrateInterface, ArtifactCrossCrateHir, ArtifactModuleSummary};
```

- [ ] **Step 2: Remove old container types from `types.rs`**

Edit `lib/src/crate_artifact/types.rs` so it contains only product-loading interface shapes:

```rust
use serde::{Deserialize, Serialize};

use crate::hir::{HirEnum, HirExtern, HirFunction, HirImpl, HirStruct, HirTrait};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactCrateInterface {
    pub root_exports: std::collections::BTreeMap<String, String>,
    pub functions: std::collections::BTreeMap<String, HirFunction>,
    pub structs: std::collections::BTreeMap<String, HirStruct>,
    pub enums: std::collections::BTreeMap<String, HirEnum>,
    pub traits: std::collections::BTreeMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    pub infix_precedence: std::collections::BTreeMap<String, u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactModuleSummary {
    pub qualified_name: String,
    pub source_path: std::path::PathBuf,
    pub inline: bool,
    pub submodules: Vec<String>,
    pub exports: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactCrossCrateHir {
    pub generic_functions: std::collections::BTreeMap<String, HirFunction>,
    pub traits_with_defaults: std::collections::BTreeMap<String, HirTrait>,
    pub generic_impls: Vec<HirImpl>,
}
```

- [ ] **Step 3: Remove old loader from `load.rs`**

In `lib/src/crate_artifact/load.rs`, remove imports used only by the old loader:

```rust
use crate::ast::Module;
use crate::crate_system::{CrateConfig, CrateManifest, LibConfig};
use super::helpers::manifest_from_artifact;
use super::{CrateArtifact, CRATE_ARTIFACT_FORMAT_VERSION};
```

Keep the imports needed by product loading. The start of the file should look like:

```rust
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::ast::Module;
use crate::collect::resolver::ResolverTables;
use crate::crate_system::{ArtifactMode, CrateConfig, CrateContext, CrateManifest, LibConfig, LoadedCrate};
use crate::products::{CompilerProducts, ProductDefId};
```

Then delete these methods from the `impl CrateContext` block:

```rust
pub fn load_artifact_from_path(&mut self, artifact_path: PathBuf) -> Result<(), String> { /* old loader */ }
pub fn load_artifact_from_path_with_root(
    &mut self,
    artifact_path: PathBuf,
    root_dir_override: Option<PathBuf>,
) -> Result<(), String> { /* old loader */ }
```

Keep:

```rust
pub fn load_product_artifact_from_path(&mut self, artifact_path: PathBuf) -> Result<(), String> { /* keep */ }
pub(crate) fn load_product_artifact_from_path_as(
    &mut self,
    expected_name: &str,
    artifact_path: PathBuf,
) -> Result<(), String> { /* keep */ }
```

Do not re-add source-bundle handling.

- [ ] **Step 4: Delete old implementation files**

Run:

```bash
git rm lib/src/crate_artifact/build.rs lib/src/crate_artifact/helpers.rs
```

- [ ] **Step 5: Compile affected crate**

Run:

```bash
cargo test -p rock-lib crate_artifact -- --nocapture
```

Expected: PASS. Fix compiler errors by removing stale imports/usages of old types only. Do not restore old builder/loader code.

- [ ] **Step 6: Commit implementation deletion**

```bash
git add lib/src/crate_artifact/mod.rs lib/src/crate_artifact/types.rs lib/src/crate_artifact/load.rs lib/src/crate_artifact/tests.rs
git add -u lib/src/crate_artifact/build.rs lib/src/crate_artifact/helpers.rs
git commit -m "lib: remove old crate artifact system"
```

---

### Task 6: Grep Audit and Final Verification

**Files:**
- Inspect only unless a grep match reveals missed old code.

- [ ] **Step 1: Grep for removed old artifact surfaces**

Use Grep or equivalent searches for:

```text
CrateArtifact
CRATE_ARTIFACT_FORMAT_VERSION
load_artifact_from_path
load_artifact_from_path_with_root
build_artifact
build_artifacts
legacy_old_artifact
ArtifactSourceBundle
ArtifactObjectOutput
ArtifactCachedModule
ArtifactCrateIdentity
ArtifactDependencyIdentity
ArtifactSourceFingerprint
```

Expected: no matches in Rust source for `CrateArtifact`, `CRATE_ARTIFACT_FORMAT_VERSION`, `ArtifactSourceBundle`, `ArtifactObjectOutput`, `ArtifactCachedModule`, `ArtifactCrateIdentity`, `ArtifactDependencyIdentity`, or `ArtifactSourceFingerprint`. Allowed Rust matches after deletion are limited to product-loading shared interface names: `ArtifactCrateInterface`, `ArtifactModuleSummary`, and `ArtifactCrossCrateHir`.

- [ ] **Step 2: Verify no source-bundle product artifact was added**

Use Grep for:

```text
source_bundle
SourceBundle
file_cache
```

Expected: no product artifact fields or product loading paths using source bundles. Existing normal source crate file caches may remain if unrelated to artifacts.

- [ ] **Step 3: Run final verification commands**

Run serialized commands:

```bash
cargo fmt --all --check
cargo test -p rockc -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock -- --nocapture
cargo build -p rockc && cargo test -p rockup -- --nocapture
cargo tree -p rock --edges normal
cargo tree -p rockup --edges normal
```

Expected:
- formatting passes
- tests pass
- `rock` and `rockup` dependency trees do not include `rock-lib`, `inkwell`, or `llvm-sys`

- [ ] **Step 4: Inspect worktree**

Run:

```bash
git status --short
```

Expected: only intentional changes for this deletion slice plus pre-existing untracked plan docs.

- [ ] **Step 5: Commit audit fixes if needed**

If grep or verification required cleanup, commit it:

```bash
git add <changed-files>
git commit -m "lib: finish old artifact cleanup"
```

If no cleanup was needed, do not create an empty commit.

---

## Self-Review Notes

Spec coverage:
- Adapt coverage first: Tasks 1-3.
- Delete old system second: Tasks 4-5.
- No source-bundle product artifacts: Tasks 5-6 explicitly forbid and audit this.
- Preserve sensible tests: Tasks 1-3 add product equivalents, Task 4 deletes old-only tests.
- Final verification and dependency boundaries: Task 6.

Placeholder scan:
- The plan contains no deferred implementation markers.
- Each code-changing step names exact files and code shape.

Type consistency:
- Product tests use existing `CompilerProducts`, `ProductMetadata`, `ProductBodies`, `HirTrait`, `HirImpl`, `Type`, and `CrateContext::load_product_artifact_from_path` names.
- Deletion tasks keep `ArtifactCrateInterface`, `ArtifactModuleSummary`, and `ArtifactCrossCrateHir`, matching current `crate_system` fields.
