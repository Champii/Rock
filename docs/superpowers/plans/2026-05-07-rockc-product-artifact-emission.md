# Rockc Product Artifact Emission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teach `rockc` to emit product-backed artifacts and explicit object files from the normal compiler pipeline.

**Architecture:** Keep the existing `CrateArtifact` loader/builder untouched. Add a product-artifact serialization wrapper around `CompilerProducts`, add explicit object output support to `rock_lib::Config`, and make `rockc --emit-artifact` call `compile_with_products` then write product artifact bytes. Product artifacts are emitted and round-tripped by the new product API only; downstream `--extern-artifact` consumption is not changed in this slice.

**Tech Stack:** Rust 2021, `serde`/`bincode`, `clap`, existing `rock-lib` codegen, `cargo test -p rock-lib`, `cargo test -p rockc`.

---

## Scope Check

This is Phase 2 from the design: `rockc` artifact/object emission using compiler products. It intentionally does not implement `rock` subprocess orchestration, product-backed downstream artifact consumption, removal of `CrateContext::build_artifact`, or removal of source bundles.

This slice builds:
- Product artifact serialization/deserialization API for `CompilerProducts`.
- Explicit object output path support in `rock_lib::Config`.
- Product link object path population after object emission.
- `rockc --emit-artifact <path>` and `rockc --emit-object <path>`.
- Tests proving one source crate can emit a product artifact and object file from one normal compile.

This slice does not build:
- Loading product artifacts through current `--extern-artifact` dependency compilation.
- Replacing `.rkca` `CrateArtifact` fields.
- Removing `source_bundle`, `cross_crate_hir`, or `CrateContext::build_artifact`.
- Backend symbol harvesting from codegen internals beyond object path recording.

## File Structure

- Modify: `lib/src/products.rs`
- Responsibility: product artifact wrapper, format version, read/write helpers, serialization tests.
- Modify: `lib/src/lib.rs`
- Responsibility: `Config.emit_object`, explicit object output path handling, attach product link object path.
- Modify: `rockc/src/main.rs`
- Responsibility: parse `--emit-artifact`, `--emit-object`, optional `--crate-name`; call `compile_with_products` when artifact output is requested; write product artifact bytes.
- Modify: existing Rust test modules in the same files.
- Responsibility: focused unit/integration-style coverage without changing downstream artifact loading.

---

### Task 1: Add Product Artifact Serialization API

**Files:**
- Modify: `lib/src/products.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Add failing tests for product artifact roundtrip and format rejection**

Add these tests inside the existing `#[cfg(test)] mod tests` in `lib/src/products.rs`:

Also extend the existing `use crate::products::{ ... }` list in that test module with:

```rust
        ProductArtifact, PRODUCT_ARTIFACT_FORMAT_VERSION,
```

```rust
    #[test]
    fn compiler_products_write_and_read_product_artifact_bytes() {
        let hir = resolved_hir_for_products();
        let products = CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local("demo".to_string()),
            &hir,
            vec![ProductDependencyIdentity {
                name: "dep".to_string(),
                artifact_path: PathBuf::from("build/dep.rkca"),
            }],
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        );

        let bytes = products.to_artifact_bytes().unwrap();
        let decoded = CompilerProducts::from_artifact_bytes(&bytes).unwrap();
        let identity_id = decoded.identity_table.export_names["identity"];

        assert!(decoded.metadata.functions.contains_key(&identity_id));
        assert!(decoded.bodies.functions.contains_key(&identity_id));
        assert_eq!(decoded.dependencies[0].name, "dep");
        assert_eq!(decoded.dependencies[0].artifact_path, PathBuf::from("build/dep.rkca"));
    }

    #[test]
    fn compiler_products_reject_unknown_product_artifact_format() {
        let hir = resolved_hir_for_products();
        let products = CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local("demo".to_string()),
            &hir,
            Vec::new(),
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        );
        let artifact = ProductArtifact {
            format_version: PRODUCT_ARTIFACT_FORMAT_VERSION + 1,
            products,
        };
        let bytes = bincode::serialize(&artifact).unwrap();

        let err = CompilerProducts::from_artifact_bytes(&bytes).unwrap_err();

        assert!(err.contains("Unsupported product artifact format"));
    }
```

- [ ] **Step 2: Run the first failing product artifact test**

Run: `cargo test -p rock-lib compiler_products_write_and_read_product_artifact_bytes -- --nocapture`

Expected: FAIL to compile because `to_artifact_bytes`, `from_artifact_bytes`, `ProductArtifact`, and `PRODUCT_ARTIFACT_FORMAT_VERSION` do not exist.

- [ ] **Step 3: Run the second failing product artifact test**

Run: `cargo test -p rock-lib compiler_products_reject_unknown_product_artifact_format -- --nocapture`

Expected: FAIL for the same missing API.

- [ ] **Step 4: Add product artifact wrapper and byte helpers**

In `lib/src/products.rs`, add near the product type definitions:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductArtifact {
    pub format_version: u32,
    pub products: CompilerProducts,
}
```

Add these methods to the existing `impl CompilerProducts` block:

```rust
    pub fn to_artifact_bytes(&self) -> Result<Vec<u8>, String> {
        let artifact = ProductArtifact {
            format_version: PRODUCT_ARTIFACT_FORMAT_VERSION,
            products: self.clone(),
        };

        bincode::serialize(&artifact)
            .map_err(|err| format!("Failed to serialize product artifact: {}", err))
    }

    pub fn from_artifact_bytes(bytes: &[u8]) -> Result<Self, String> {
        let artifact: ProductArtifact = bincode::deserialize(bytes)
            .map_err(|err| format!("Failed to deserialize product artifact: {}", err))?;

        if artifact.format_version != PRODUCT_ARTIFACT_FORMAT_VERSION {
            return Err(format!(
                "Unsupported product artifact format {} (expected {})",
                artifact.format_version, PRODUCT_ARTIFACT_FORMAT_VERSION
            ));
        }

        Ok(artifact.products)
    }
```

- [ ] **Step 5: Add path read/write helpers**

Add these methods to the same `impl CompilerProducts` block:

```rust
    pub fn write_artifact_to_path(&self, path: &std::path::Path) -> Result<(), String> {
        let bytes = self.to_artifact_bytes()?;
        std::fs::write(path, bytes)
            .map_err(|err| format!("Failed to write product artifact {}: {}", path.display(), err))
    }

    pub fn read_artifact_from_path(path: &std::path::Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|err| format!("Failed to read product artifact {}: {}", path.display(), err))?;
        Self::from_artifact_bytes(&bytes)
            .map_err(|err| format!("Failed to deserialize product artifact {}: {}", path.display(), err))
    }
```

- [ ] **Step 6: Run product artifact tests**

Run: `cargo test -p rock-lib compiler_products_write_and_read_product_artifact_bytes -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib compiler_products_reject_unknown_product_artifact_format -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for all product tests.

If cargo hits stale-artifact linker errors, run `cargo clean` and retry the same command. Do not switch target directories.

- [ ] **Step 7: Commit**

```bash
git add lib/src/products.rs
git commit -m "products: add product artifact serialization"
```

---

### Task 2: Attach Explicit Object Output to Compiler Products

**Files:**
- Modify: `lib/src/lib.rs`
- Test: `lib/src/lib.rs`

- [ ] **Step 1: Add a failing test for explicit object output path in products**

Add this test inside the existing `#[cfg(test)] mod tests` in `lib/src/lib.rs`:

```rust
    #[test]
    fn compile_with_products_records_explicit_object_output_path() {
        let base = std::env::temp_dir().join(format!(
            "rock_compile_products_{}_{}",
            std::process::id(),
            "object_output"
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let entry_file = base.join("main.rk");
        let object_path = base.join("custom-main.o");
        fs::write(&entry_file, "main = ->\n    0\n").unwrap();

        let output = compile_with_products(&Config {
            entry_file,
            output_dir: base.join("build"),
            no_std: true,
            no_prelude: true,
            no_link: true,
            emit_object: Some(object_path.clone()),
            current_crate_name: Some("demo".to_string()),
            ..Config::default()
        })
        .unwrap();

        let products = output.products.expect("products should be emitted");
        assert!(object_path.exists());
        assert_eq!(products.link.object_path, Some(object_path));

        let _ = fs::remove_dir_all(&base);
    }
```

- [ ] **Step 2: Run the failing object output test**

Run: `cargo test -p rock-lib compile_with_products_records_explicit_object_output_path -- --nocapture`

Expected: FAIL to compile because `Config` has no `emit_object` field.

- [ ] **Step 3: Add `emit_object` to compiler config**

In `lib/src/lib.rs`, add this field to `Config` after `no_link`:

```rust
    pub emit_object: Option<PathBuf>,
```

Update every explicit `rock_lib::Config` initializer that does not use `..Config::default()` by adding:

```rust
            emit_object: None,
```

Search with:

Run: `cargo test -p rock-lib compile_with_products_records_explicit_object_output_path -- --nocapture`

Expected: FAIL until every explicit initializer is updated, then fail because object output behavior is not implemented.

- [ ] **Step 4: Write helper to attach product object path**

Add this helper in `lib/src/lib.rs` below `product_dependencies_from_config`:

```rust
fn attach_product_object_path(products: &mut Option<CompilerProducts>, object_path: &std::path::Path) {
    if let Some(products) = products.as_mut() {
        products.link.object_path = Some(object_path.to_path_buf());
    }
}
```

- [ ] **Step 5: Use explicit object path during no-link object emission**

In `compile_impl`, change the `if config.no_link { ... }` object path calculation to:

```rust
    if config.no_link {
        let obj_path = config
            .emit_object
            .clone()
            .unwrap_or_else(|| config.output_dir.join(format!("{}.o", module_name)));
        if let Some(parent) = obj_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = codegen.write_object(&obj_path, opt) {
            let diagnostics = Diagnostics::from(vec![e]);
            return Err(diagnostics);
        }
        attach_product_object_path(&mut products, &obj_path);
    } else {
```

This preserves old `--no-link` output when `emit_object` is `None` and records explicit object output when requested.

- [ ] **Step 6: Run object output and focused compatibility tests**

Run: `cargo test -p rock-lib compile_with_products_records_explicit_object_output_path -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib compile_with_products_extracts_ids_from_normal_pipeline -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rock-lib crate_artifact::tests::test_compile_without_explicit_stdlib_artifact_fails_with_no_std -- --nocapture`

Expected: PASS with 1 test passing.

If cargo hits stale-artifact linker errors, run `cargo clean` and retry the same command. Do not switch target directories.

- [ ] **Step 7: Commit**

```bash
git add lib/src/lib.rs lib/src/collect/context.rs lib/src/lower/program.rs lib/src/lower/crates/bodies.rs lib/src/crate_artifact/tests.rs lib/tests/integration.rs rock/src/build.rs rock/src/compile.rs rockup/src/dev.rs rockc/src/main.rs
git commit -m "compiler: record explicit object product output"
```

Only include files that actually needed `Config` initializer updates.

---

### Task 3: Add `rockc` Product Artifact and Object Flags

**Files:**
- Modify: `rockc/src/main.rs`
- Test: `rockc/src/main.rs`

- [ ] **Step 1: Add failing CLI parse tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `rockc/src/main.rs`:

```rust
    #[test]
    fn test_emit_artifact_and_object_parse() {
        let config = Config::try_parse_from([
            "rockc",
            "--entry-file",
            "main.rk",
            "--emit-artifact",
            "build/main.rkca",
            "--emit-object",
            "build/main.o",
            "--crate-name",
            "demo",
        ])
        .unwrap();

        assert_eq!(config.emit_artifact, Some(PathBuf::from("build/main.rkca")));
        assert_eq!(config.emit_object, Some(PathBuf::from("build/main.o")));
        assert_eq!(config.crate_name, Some("demo".to_string()));
    }
```

- [ ] **Step 2: Run the failing parse test**

Run: `cargo test -p rockc test_emit_artifact_and_object_parse -- --nocapture`

Expected: FAIL to compile because `Config` has no `emit_artifact`, `emit_object`, or `crate_name` fields.

- [ ] **Step 3: Add CLI fields**

In `rockc/src/main.rs`, add fields to `Config`:

```rust
    /// Crate name for product identity and codegen symbol qualification
    #[arg(long)]
    crate_name: Option<String>,
    /// Emit product-backed compiler artifact to this path
    #[arg(long)]
    emit_artifact: Option<PathBuf>,
    /// Emit object file to this exact path
    #[arg(long)]
    emit_object: Option<PathBuf>,
```

Update `into_compiler_config`:

```rust
            current_crate_name: config.crate_name,
            emit_object: config.emit_object,
```

- [ ] **Step 4: Refactor `run` through `run_config`**

Replace `run` with:

```rust
fn run() -> Result<(), Diagnostics> {
    run_config(Config::parse())
}

fn run_config(config: Config) -> Result<(), Diagnostics> {
    if let Some(output) = print_request_output(&config)? {
        println!("{}", output);
        return Ok(());
    }

    let emit_artifact = config.emit_artifact.clone();
    let compiler_config = config.into_compiler_config()?;

    if let Some(path) = emit_artifact {
        let output = rock_lib::compile_with_products(&compiler_config)?;
        let products = output.products.ok_or_else(|| {
            diagnostics_from_message("Compiler did not produce product data for artifact emission")
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                diagnostics_from_message(format!(
                    "Failed to create artifact directory {}: {}",
                    parent.display(),
                    err
                ))
            })?;
        }
        products.write_artifact_to_path(&path).map_err(diagnostics_from_message)?;
    } else {
        rock_lib::compile(&compiler_config)?;
    }

    Ok(())
}
```

- [ ] **Step 5: Run parse tests**

Run: `cargo test -p rockc test_emit_artifact_and_object_parse -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rockc test_removed_dependency_flags_are_rejected -- --nocapture`

Expected: PASS with 1 test passing.

- [ ] **Step 6: Commit**

```bash
git add rockc/src/main.rs
git commit -m "rockc: parse product artifact outputs"
```

---

### Task 4: Verify `rockc` Emits Product Artifact and Object Files

**Files:**
- Modify: `rockc/src/main.rs`
- Test: `rockc/src/main.rs`

- [ ] **Step 1: Add failing `run_config` emission test**

Add this test inside `#[cfg(test)] mod tests` in `rockc/src/main.rs`:

Also update the test module import to include `run_config`:

```rust
    use super::{print_request_output, run_config, Config};
```

```rust
    #[test]
    fn test_run_config_emits_product_artifact_and_object() {
        let base = std::env::temp_dir().join(format!(
            "rockc_product_artifact_{}_{}",
            std::process::id(),
            "emit"
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let entry_file = base.join("main.rk");
        let artifact_path = base.join("build").join("demo.rkca");
        let object_path = base.join("build").join("demo.o");
        std::fs::write(&entry_file, "main = ->\n    0\n").unwrap();

        let config = Config::try_parse_from([
            "rockc",
            "--entry-file",
            entry_file.to_str().unwrap(),
            "--output-dir",
            base.join("build").to_str().unwrap(),
            "--no-std",
            "--no-prelude",
            "--no-link",
            "--crate-name",
            "demo",
            "--emit-artifact",
            artifact_path.to_str().unwrap(),
            "--emit-object",
            object_path.to_str().unwrap(),
        ])
        .unwrap();

        run_config(config).unwrap();

        assert!(artifact_path.exists());
        assert!(object_path.exists());
        let products = rock_lib::products::CompilerProducts::read_artifact_from_path(&artifact_path)
            .unwrap();
        let main_id = products.identity_table.export_names["main"];
        assert!(products.metadata.functions.contains_key(&main_id));
        assert_eq!(products.link.object_path, Some(object_path));

        let _ = std::fs::remove_dir_all(&base);
    }
```

- [ ] **Step 2: Run the emission test**

Run: `cargo test -p rockc test_run_config_emits_product_artifact_and_object -- --nocapture`

Expected: PASS with 1 test passing.

- [ ] **Step 3: Run rockc emission and unit tests**

Run: `cargo test -p rockc test_run_config_emits_product_artifact_and_object -- --nocapture`

Expected: PASS with 1 test passing.

Run: `cargo test -p rockc`

Expected: PASS for all `rockc` tests.

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for all product tests.

If cargo hits stale-artifact linker errors, run `cargo clean` and retry the same command. Do not switch target directories.

- [ ] **Step 4: Commit**

```bash
git add rockc/src/main.rs
git commit -m "rockc: emit product artifacts"
```

---

### Task 5: Final Verification and Documentation Touch-Up

**Files:**
- Modify: `docs/superpowers/plans/2026-05-07-rockc-product-artifact-emission.md` if command names changed during implementation.
- Test: workspace commands.

- [ ] **Step 1: Run focused final tests**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for all product tests.

Run: `cargo test -p rock-lib compile_with_products -- --nocapture`

Expected: PASS for compile-with-products tests.

Run: `cargo test -p rockc`

Expected: PASS for all `rockc` tests.

- [ ] **Step 2: Run full library tests**

Run: `cargo test -p rock-lib`

Expected: PASS for the full `rock-lib` test suite.

If any command hits stale-artifact linker errors, run `cargo clean` and retry the same command. Do not switch target directories.

- [ ] **Step 3: Confirm no downstream artifact behavior was changed**

Run: `cargo test -p rock-lib crate_artifact -- --nocapture`

Expected: PASS for existing crate artifact tests.

- [ ] **Step 4: Commit any plan updates only if implementation changed command names**

If the final implementation kept `--emit-artifact`, `--emit-object`, and `--crate-name`, do not edit or commit this plan. If command names changed, update this plan's command names and commit:

```bash
git add docs/superpowers/plans/2026-05-07-rockc-product-artifact-emission.md
git commit -m "docs: update rockc product artifact plan"
```

---

## Completion Criteria

- `CompilerProducts` can be serialized/deserialized through product artifact helpers.
- `rock_lib::Config.emit_object` supports an explicit object output path.
- `compile_with_products` records `products.link.object_path` when an object is emitted through `emit_object`/`no_link`.
- `rockc --emit-artifact <path>` writes product-backed artifact bytes from `compile_with_products`.
- `rockc --emit-object <path> --no-link` writes an object at the exact requested path.
- Existing `rock_lib::compile` callers still compile as before.
- Existing `CrateArtifact` loading/building tests still pass.
- `cargo test -p rock-lib` and `cargo test -p rockc` pass.

## Next Plan After This Slice

After this lands, write Plan 3 for product-backed artifact loading behind the existing `--extern-artifact` boundary. That plan should add a loader that can detect product artifacts, expose metadata/body/link provider capabilities, and keep old `CrateArtifact` loading available until product-backed stdlib and broad integration tests pass.
