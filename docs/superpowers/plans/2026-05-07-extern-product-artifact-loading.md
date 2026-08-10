# Extern Product Artifact Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit `--extern-product-artifact name=path` dependency input path so `rockc` can consume product artifacts without changing current `--extern-artifact` behavior.

**Architecture:** Keep old `CrateArtifact` loading untouched. Add a parallel product-artifact input list in `rock_lib::Config` and `rockc`, load those artifacts only through the new flag, and adapt `CompilerProducts` into the existing `LoadedCrate` dependency shape required by collect/lower/mono. This bridge is explicit and scoped to product artifacts; there is no format sniffing or fallback behind `--extern-artifact`.

**Tech Stack:** Rust 2021, existing `serde`/`bincode` product artifacts, `clap`, `cargo test -p rock-lib`, `cargo test -p rockc`.

---

## Scope Check

This plan implements the explicit product dependency input slice from `docs/superpowers/specs/2026-05-07-extern-product-artifact-loading-design.md`.

This slice builds:
- `rock_lib::Config.extern_product_artifacts`.
- `rockc --extern-product-artifact name=path` parsing.
- duplicate crate-name rejection across old and product artifact inputs.
- explicit product artifact loading into `CrateContext`.
- a focused runtime test that emits a dependency product artifact/object, then compiles an app with `--extern-product-artifact`.

This slice does not build:
- product artifact loading through `--extern-artifact`.
- `rock` subprocess orchestration.
- removal of `CrateArtifact`, source bundles, or `CrateContext::build_artifact`.
- stdlib product artifact migration.

## File Structure

- Modify: `lib/src/lib.rs`
- Responsibility: add config field, reject duplicate extern crate names, load product artifacts before parsing current crate, include product dependencies in emitted product metadata.
- Modify: `rockc/src/main.rs`
- Responsibility: parse `--extern-product-artifact`, pass it to `rock_lib::Config`, add CLI parse tests and runtime smoke test.
- Modify: `lib/src/crate_artifact/load.rs`
- Responsibility: add explicit product-artifact loader and adapter into `LoadedCrate`.
- Test: existing test modules in touched files.

---

### Task 1: Add Explicit Product Artifact CLI and Config Surface

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `rockc/src/main.rs`
- Test: `rockc/src/main.rs`

- [ ] **Step 1: Add failing parse test for `--extern-product-artifact`**

Add this test inside `#[cfg(test)] mod tests` in `rockc/src/main.rs`:

```rust
    #[test]
    fn test_parse_extern_product_artifact() {
        let config = Config::try_parse_from([
            "rockc",
            "--entry-file",
            "main.rk",
            "--extern-product-artifact",
            "dep=build/dep.rkca",
        ])
        .unwrap();

        assert_eq!(
            config.extern_product_artifact,
            vec!["dep=build/dep.rkca".to_string()]
        );
    }
```

- [ ] **Step 2: Run the failing parse test**

Run: `cargo test -p rockc test_parse_extern_product_artifact -- --nocapture`

Expected: FAIL to compile because `extern_product_artifacts` does not exist.

- [ ] **Step 3: Add config fields**

In `lib/src/lib.rs`, add this field to `Config` next to `extern_artifacts`:

```rust
    pub extern_product_artifacts: Vec<(String, PathBuf)>,
```

Add this default value in `impl Default for Config`:

```rust
            extern_product_artifacts: vec![],
```

In `rockc/src/main.rs`, add this field to the CLI `Config` next to `extern_artifacts`:

```rust
    /// Product compiler artifact dependencies to load: name=path
    #[arg(long)]
    extern_product_artifact: Vec<String>,
```

In the `From<Config> for rock_lib::Config` conversion, add:

```rust
        let extern_product_artifacts = config
            .extern_product_artifact
            .iter()
            .map(|s| parse_name_path(s, "extern-product-artifact"))
            .collect();
```

Then pass it into `rock_lib::Config`:

```rust
            extern_product_artifacts,
```

Also update every direct `rock_lib::Config { ... }` construction in `rock` and tests to include `extern_product_artifacts: vec![]` if struct update syntax is not already used. Use `grep` for `extern_artifacts:` to find required initializers.

- [ ] **Step 4: Run parse tests**

Run: `cargo test -p rockc test_parse_extern_product_artifact -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rockc`

Expected: PASS for all `rockc` tests.

- [ ] **Step 5: Commit**

```bash
git add lib/src/lib.rs rockc/src/main.rs rock/src/build.rs rock/src/compile.rs
git commit -m "rockc: parse extern product artifacts"
```

---

### Task 2: Add Explicit Product Artifact Loader

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Add failing product loader unit test**

Add this test module to the bottom of `lib/src/crate_artifact/load.rs`:

```rust
#[cfg(test)]
mod product_tests {
    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::path::PathBuf;

    use crate::crate_system::CrateContext;
    use crate::hir::{HirBlock, HirExpr, HirExprKind, HirFunction, HirStmt};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::products::{
        CompilerProducts, ProductCrateIdentity, ProductDefId, ProductIdentityTable,
        ProductLinkData, ProductMetadata, ProductSourceFingerprint,
    };
    use crate::span::Span;
    use crate::types::Type;

    fn test_function(id: DefId, name: &str) -> HirFunction {
        HirFunction {
            id,
            name: name.to_string(),
            qualified_name: None,
            generic_params: Vec::new(),
            generic_bounds: HashMap::new(),
            params: Vec::new(),
            ret_type: Type::I64,
            body: HirBlock {
                stmts: vec![HirStmt::Expr(HirExpr {
                    kind: HirExprKind::IntLiteral(5),
                    ty: Type::I64,
                    span: Span::default(),
                })],
                ty: Type::I64,
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }

    #[test]
    fn load_product_artifact_registers_object_backed_crate() {
        let base = std::env::temp_dir().join(format!(
            "rock_product_loader_{}_{}",
            std::process::id(),
            "object_crate"
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let artifact_path = base.join("dep.rkca");
        let object_path = base.join("dep.o");
        fs::write(&object_path, []).unwrap();

        let function_id = DefId::new(CrateId(1), LocalDefId(7));
        let product_id = ProductDefId::from(function_id);
        let mut identity_table = ProductIdentityTable::default();
        identity_table.display_names.insert(product_id, "dep::answer".to_string());
        identity_table.export_names.insert("answer".to_string(), product_id);
        let mut metadata = ProductMetadata::default();
        metadata
            .functions
            .insert(product_id, test_function(function_id, "dep::answer"));
        let products = CompilerProducts {
            crate_identity: ProductCrateIdentity::local("dep".to_string()),
            identity_table,
            metadata,
            bodies: Default::default(),
            link: ProductLinkData {
                object_path: Some(object_path.clone()),
                records: Default::default(),
            },
            dependencies: Vec::new(),
            source_fingerprint: ProductSourceFingerprint::default(),
        };
        products.write_artifact_to_path(&artifact_path).unwrap();

        let mut ctx = CrateContext::new();
        ctx.load_product_artifact_from_path(artifact_path).unwrap();

        let loaded = ctx.crates.get("dep").unwrap();
        assert!(loaded.is_object_backed());
        assert_eq!(loaded.object_path, Some(object_path));
        let interface = loaded.interface.as_ref().unwrap();
        assert!(interface.functions.contains_key("dep::answer"));
        assert_eq!(
            interface.root_exports.get("answer"),
            Some(&"dep::answer".to_string())
        );

        let _ = fs::remove_dir_all(&base);
    }
}
```

- [ ] **Step 2: Run the failing loader test**

Run: `cargo test -p rock-lib load_product_artifact_registers_object_backed_crate -- --nocapture`

Expected: FAIL to compile because `load_product_artifact_from_path` does not exist.

- [ ] **Step 3: Add product loader implementation**

In `lib/src/crate_artifact/load.rs`, extend the imports:

```rust
use std::collections::{BTreeMap, HashMap};
```

Add these imports near the existing crate imports:

```rust
use crate::ast::Module;
use crate::collect::resolver::ResolverTables;
use crate::crate_system::{
    ArtifactMode, CrateConfig, CrateContext, CrateManifest, LibConfig, LoadedCrate,
};
use crate::products::{CompilerProducts, ProductDefId};
```

Keep existing imports deduplicated after adding these.

Add this method in the existing `impl CrateContext` block:

```rust
    pub fn load_product_artifact_from_path(&mut self, artifact_path: PathBuf) -> Result<(), String> {
        let products = CompilerProducts::read_artifact_from_path(&artifact_path)?;
        let loaded_crate = loaded_crate_from_products(products, artifact_path)?;
        let crate_name = loaded_crate.manifest.crate_.name.clone();
        self.crates.insert(crate_name, loaded_crate);
        Ok(())
    }
```

Add these helper functions below the `impl CrateContext` block:

```rust
fn loaded_crate_from_products(
    products: CompilerProducts,
    artifact_path: PathBuf,
) -> Result<LoadedCrate, String> {
    let crate_name = products.crate_identity.name.clone();
    let object_path = resolve_product_object_path(&products, &artifact_path)?;
    let interface = interface_from_products(&products, &crate_name);
    let resolver = resolver_from_products(&products);
    let root_dir = artifact_path
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(LoadedCrate {
        manifest: CrateManifest {
            crate_: CrateConfig {
                name: crate_name.clone(),
                version: products.crate_identity.version.clone(),
                no_std: false,
            },
            lib: LibConfig {
                path: "lib.rk".to_string(),
            },
        },
        ast: Module {
            name: None,
            top_levels: Vec::new(),
            is_inline: false,
            filepath: Some(root_dir.join("lib.rk")),
        },
        interface: Some(interface),
        resolver,
        prelude_exports: BTreeMap::new(),
        module_index: BTreeMap::new(),
        object_path: Some(object_path),
        cross_crate_hir: Some(cross_crate_hir_from_products(&products)),
        artifact_mode: ArtifactMode::Object,
        has_source_bundle: false,
        root_dir,
        module_tree: None,
        file_cache: HashMap::new(),
    })
}

fn resolve_product_object_path(
    products: &CompilerProducts,
    artifact_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let Some(object_path) = products.link.object_path.clone() else {
        return Err(format!(
            "Product artifact {} does not record an object output path",
            artifact_path.display()
        ));
    };
    let candidate = if object_path.is_absolute() {
        object_path
    } else {
        artifact_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(object_path)
    };

    if !candidate.exists() {
        return Err(format!(
            "Product artifact {} references missing object file {}",
            artifact_path.display(),
            candidate.display()
        ));
    }

    Ok(candidate.canonicalize().unwrap_or(candidate))
}

fn interface_from_products(
    products: &CompilerProducts,
    crate_name: &str,
) -> super::ArtifactCrateInterface {
    let mut root_exports = BTreeMap::new();
    for (alias, id) in &products.identity_table.export_names {
        if let Some(display_name) = product_display_name(products, *id) {
            root_exports.insert(alias.clone(), qualify_product_name(crate_name, &display_name));
        }
    }

    super::ArtifactCrateInterface {
        root_exports,
        functions: products
            .metadata
            .functions
            .iter()
            .filter_map(|(id, function)| {
                product_display_name(products, *id)
                    .map(|name| (qualify_product_name(crate_name, &name), function.clone()))
            })
            .collect(),
        structs: products
            .metadata
            .structs
            .iter()
            .filter_map(|(id, value)| {
                product_display_name(products, *id)
                    .map(|name| (qualify_product_name(crate_name, &name), value.clone()))
            })
            .collect(),
        enums: products
            .metadata
            .enums
            .iter()
            .filter_map(|(id, value)| {
                product_display_name(products, *id)
                    .map(|name| (qualify_product_name(crate_name, &name), value.clone()))
            })
            .collect(),
        traits: products
            .metadata
            .traits
            .iter()
            .filter_map(|(id, value)| {
                product_display_name(products, *id)
                    .map(|name| (qualify_product_name(crate_name, &name), value.clone()))
            })
            .collect(),
        impls: products.metadata.impls.values().cloned().collect(),
        externs: products.metadata.externs.values().cloned().collect(),
        infix_precedence: BTreeMap::new(),
    }
}

fn resolver_from_products(products: &CompilerProducts) -> ResolverTables {
    let mut resolver = ResolverTables::default();
    for (id, name) in &products.identity_table.display_names {
        if let Some(def_id) = product_def_id_to_existing_def_id(products, *id) {
            resolver.item_paths.insert(name.clone(), def_id);
            resolver.item_names_by_id.insert(def_id, name.clone());
        }
    }
    for (alias, id) in &products.identity_table.export_names {
        if let Some(def_id) = product_def_id_to_existing_def_id(products, *id) {
            resolver.export_aliases.insert(alias.clone(), def_id);
        }
    }
    resolver
}

fn cross_crate_hir_from_products(products: &CompilerProducts) -> super::ArtifactCrossCrateHir {
    super::ArtifactCrossCrateHir {
        generic_functions: products
            .bodies
            .functions
            .iter()
            .filter_map(|(id, function)| {
                product_display_name(products, *id).map(|name| (name, function.clone()))
            })
            .collect(),
        traits_with_defaults: products
            .metadata
            .traits
            .iter()
            .filter_map(|(id, trait_def)| {
                if trait_def.methods.is_empty() {
                    None
                } else {
                    product_display_name(products, *id).map(|name| (name, trait_def.clone()))
                }
            })
            .collect(),
        generic_impls: products.bodies.generic_impls.values().cloned().collect(),
    }
}

fn product_display_name(products: &CompilerProducts, id: ProductDefId) -> Option<String> {
    products.identity_table.display_names.get(&id).cloned()
}

fn qualify_product_name(crate_name: &str, name: &str) -> String {
    if name.contains("::") {
        name.to_string()
    } else {
        format!("{}::{}", crate_name, name)
    }
}

fn product_def_id_to_existing_def_id(
    products: &CompilerProducts,
    id: ProductDefId,
) -> Option<crate::ids::DefId> {
    products
        .metadata
        .functions
        .get(&id)
        .map(|value| value.id)
        .or_else(|| products.metadata.structs.get(&id).map(|value| value.id))
        .or_else(|| products.metadata.enums.get(&id).map(|value| value.id))
        .or_else(|| products.metadata.traits.get(&id).map(|value| value.id))
        .or_else(|| products.metadata.externs.get(&id).map(|value| value.id))
        .or_else(|| products.metadata.impls.get(&id).map(|value| value.id))
}
```

After inserting the helpers, run `cargo fmt --all` if rustfmt reports import ordering or wrapping issues.

- [ ] **Step 4: Run product loader test**

Run: `cargo test -p rock-lib load_product_artifact_registers_object_backed_crate -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for product tests.

- [ ] **Step 5: Commit**

```bash
git add lib/src/crate_artifact/load.rs
git commit -m "artifact: load product artifacts explicitly"
```

---

### Task 3: Wire Product Artifacts Into Compilation

**Files:**
- Modify: `lib/src/lib.rs`
- Test: `lib/src/lib.rs`

- [ ] **Step 1: Add failing duplicate-name test**

Add this test in `#[cfg(test)] mod tests` in `lib/src/lib.rs`:

```rust
    #[test]
    fn compile_rejects_duplicate_extern_artifact_names() {
        let err = crate::compile(&Config {
            extern_artifacts: vec![("dep".to_string(), "old.rkca".into())],
            extern_product_artifacts: vec![("dep".to_string(), "product.rkca".into())],
            ..Config::default()
        })
        .unwrap_err();

        let rendered = format!("{:?}", err);
        assert!(rendered.contains("Duplicate external crate artifact 'dep'"));
    }
```

- [ ] **Step 2: Run the failing duplicate-name test**

Run: `cargo test -p rock-lib compile_rejects_duplicate_extern_artifact_names -- --nocapture`

Expected: FAIL because duplicates are not checked yet.

- [ ] **Step 3: Load product artifacts and reject duplicates**

In `lib/src/lib.rs`, add a helper near `compile_impl`:

```rust
fn validate_extern_artifact_names(config: &Config) -> Result<(), Diagnostics> {
    let mut old_names = std::collections::BTreeSet::new();
    for (name, _) in &config.extern_artifacts {
        old_names.insert(name.clone());
    }

    for (name, _) in &config.extern_product_artifacts {
        if old_names.contains(name) {
            let mut diagnostics = Diagnostics::default();
            diagnostics.push(diagnostic::Diagnostic::new(
                format!("Duplicate external crate artifact '{}'", name),
                Span::default(),
            ));
            return Err(diagnostics);
        }
    }

    Ok(())
}
```

At the start of `compile_impl`, immediately after creating `ctx`, call:

```rust
    validate_extern_artifact_names(config)?;
```

After the existing loop that loads `config.extern_artifacts`, add:

```rust
    for (name, path) in &config.extern_product_artifacts {
        if let Err(e) = ctx.load_product_artifact_from_path(path.clone()) {
            let mut diagnostics = Diagnostics::default();
            diagnostics.push(diagnostic::Diagnostic::new(
                format!(
                    "Failed to load external product artifact '{}' from {}: {}",
                    name,
                    path.display(),
                    e
                ),
                Span::default(),
            ));
            return Err(diagnostics);
        }
    }
```

Update `product_dependencies_from_config` so emitted product artifacts record both old and product artifact dependencies:

```rust
fn product_dependencies_from_config(config: &Config) -> Vec<ProductDependencyIdentity> {
    config
        .extern_artifacts
        .iter()
        .chain(config.extern_product_artifacts.iter())
        .map(|(name, path)| ProductDependencyIdentity {
            name: name.clone(),
            artifact_path: path.clone(),
        })
        .collect()
}
```

- [ ] **Step 4: Run focused tests**

Run: `cargo test -p rock-lib compile_rejects_duplicate_extern_artifact_names -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib load_product_artifact_registers_object_backed_crate -- --nocapture`

Expected: PASS with 1 test passing.

- [ ] **Step 5: Commit**

```bash
git add lib/src/lib.rs
git commit -m "compiler: load explicit product artifact dependencies"
```

---

### Task 4: Prove `rockc` Consumes Product Artifacts

**Files:**
- Modify: `rockc/src/main.rs`
- Test: `rockc/src/main.rs`

- [ ] **Step 1: Add failing runtime smoke test**

Add this test inside `#[cfg(test)] mod tests` in `rockc/src/main.rs`:

```rust
    #[test]
    fn test_run_config_consumes_product_artifact_dependency() {
        let base = std::env::temp_dir().join(format!(
            "rockc_product_dependency_{}_{}",
            std::process::id(),
            "smoke"
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let dep_dir = base.join("dep");
        let app_dir = base.join("app");
        let dep_build = dep_dir.join("build");
        let app_build = app_dir.join("build");
        std::fs::create_dir_all(&dep_build).unwrap();
        std::fs::create_dir_all(&app_build).unwrap();
        let dep_entry = dep_dir.join("lib.rk");
        let app_entry = app_dir.join("main.rk");
        let dep_artifact = dep_build.join("dep.rkca");
        let dep_object = dep_build.join("dep.o");
        std::fs::write(&dep_entry, "answer = ->\n    5\n< answer\n").unwrap();
        std::fs::write(&app_entry, "> dep::answer\n\nmain = ->\n    answer!\n").unwrap();

        let dep_config = Config::try_parse_from([
            "rockc",
            "--entry-file",
            dep_entry.to_str().unwrap(),
            "--output-dir",
            dep_build.to_str().unwrap(),
            "--no-std",
            "--no-prelude",
            "--no-link",
            "--crate-name",
            "dep",
            "--emit-artifact",
            dep_artifact.to_str().unwrap(),
            "--emit-object",
            dep_object.to_str().unwrap(),
        ])
        .unwrap();
        run_config(dep_config).unwrap();

        let app_config = Config::try_parse_from([
            "rockc",
            "--entry-file",
            app_entry.to_str().unwrap(),
            "--output-dir",
            app_build.to_str().unwrap(),
            "--no-std",
            "--no-prelude",
            "--extern-product-artifact",
            &format!("dep={}", dep_artifact.display()),
        ])
        .unwrap();
        run_config(app_config).unwrap();

        let executable = app_build.join("main");
        assert!(executable.exists());
        let status = std::process::Command::new(&executable).status().unwrap();
        assert_eq!(status.code(), Some(5));

        let _ = std::fs::remove_dir_all(&base);
    }
```

- [ ] **Step 2: Run the failing smoke test**

Run: `cargo test -p rockc test_run_config_consumes_product_artifact_dependency -- --nocapture`

Expected: FAIL until product adapter behavior is sufficient for downstream compile/link.

- [ ] **Step 3: Fix only product-loading gaps exposed by the smoke test**

If the smoke test fails because product metadata lacks a fact that should be present in product artifacts, make the smallest change in `lib/src/products.rs` or `lib/src/crate_artifact/load.rs` to expose/derive that fact.

Allowed fixes in this step:
- add missing root export alias derivation from `ProductIdentityTable.export_names`
- add missing resolver entries derived from product display names and cloned HIR IDs
- normalize unqualified product names to `crate::name` only inside the product loader
- attach the product object path so linker sees the dependency object

Not allowed in this step:
- changing `--extern-artifact` behavior
- loading dependency source as a fallback
- changing `rock` orchestration
- adding stdlib product loading behavior

- [ ] **Step 4: Run smoke and regression tests**

Run: `cargo test -p rockc test_run_config_consumes_product_artifact_dependency -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rockc`

Expected: PASS for all `rockc` tests.

Run: `cargo test -p rock-lib crate_artifact -- --nocapture`

Expected: PASS for existing old artifact tests.

- [ ] **Step 5: Commit**

```bash
git add rockc/src/main.rs lib/src/products.rs lib/src/crate_artifact/load.rs
git commit -m "rockc: consume explicit product artifacts"
```

---

### Task 5: Final Verification

**Files:**
- Modify: none unless previous task exposed a required doc correction.
- Test: workspace commands.

- [ ] **Step 1: Run focused product and artifact tests**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for product tests.

Run: `cargo test -p rock-lib crate_artifact -- --nocapture`

Expected: PASS for old artifact tests.

Run: `cargo test -p rockc`

Expected: PASS for all `rockc` tests.

- [ ] **Step 2: Run broader library verification**

Run: `cargo test -p rock-lib`

Expected: PASS for the full `rock-lib` test suite.

If any cargo command hits stale-artifact linker errors after a long-timeout run, run `cargo clean` and retry the same command. Do not switch target directories.

- [ ] **Step 3: Confirm old and product flags remain separate**

Run: `cargo test -p rockc test_parse_extern_product_artifact -- --nocapture`

Expected: PASS and parse only the new product flag.

Run: `cargo test -p rock-lib compile_rejects_duplicate_extern_artifact_names -- --nocapture`

Expected: PASS with duplicate old/product names rejected.

- [ ] **Step 4: Commit docs only if plan/spec changed during implementation**

If this plan or spec needed corrections during implementation, commit only those docs:

```bash
git add docs/superpowers/plans/2026-05-07-extern-product-artifact-loading.md docs/superpowers/specs/2026-05-07-extern-product-artifact-loading-design.md
git commit -m "docs: update extern product artifact loading plan"
```

If no doc corrections were needed, do not create a documentation commit.
