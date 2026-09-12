# Artifact-Only Downstream Dependencies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure every compiler invocation reads source only for the current crate and consumes every dependency exclusively through product artifact data.

**Architecture:** Add a `LoadedCrate` downstream-dependency guard that exposes artifact interface capability and rejects source-backed loaded crates. Route `collect`, `lower`, and `mono` through artifact-backed metadata/body/link capability paths, then delete or convert tests that only prove source-backed external dependency consumption. Current-crate source modules and source artifact production remain valid.

**Tech Stack:** Rust 2021, `rock-lib`, `rockc`, `rock`, product artifacts, `CrateContext`, `LoadedCrate`, `ArtifactCrateInterface`, collect/lower/infer/mono pipeline, `cargo test`.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-05-11-artifact-only-downstream-dependencies-design.md`.

The invariant for every task is:

- The current crate may be source-backed.
- Dependencies of that current crate may not be source-backed during downstream compilation.
- `rock` may still build local source path dependencies by invoking `rockc` separately with each dependency as the current crate.
- Product artifact metadata, resolver data, cross-crate HIR bodies, and object link metadata are the only dependency inputs available to compiler phases.

## File Structure

- Modify: `lib/src/crate_system/mod.rs`
  - Add a downstream artifact-interface guard on `LoadedCrate`.
  - Keep `ArtifactMode::Source` and `load_crate_from_dir` for current-crate/artifact-production tooling.
- Modify: `lib/src/crate_system/tests.rs`
  - Unit-test the guard for source-backed rejection and product-interface acceptance.
- Modify: `lib/src/collect/context.rs`
  - Stop recollecting dependency ASTs from source-backed loaded crates in normal dependency registration.
  - Register only product artifact interface data for downstream dependencies.
- Modify: `lib/src/collect/mod.rs`
  - Replace source-backed external dependency consumption tests with rejection tests.
  - Keep current-crate source-backed module tests unchanged.
- Modify: `lib/src/lower/crates/registration.rs`
  - Stop registering dependency declarations from source-backed loaded crates in legacy lower registration.
- Modify: `lib/src/lower/crates/bodies.rs`
  - Stop lowering dependency trait defaults and function bodies from dependency ASTs or file caches.
  - Keep cross-crate HIR bundle application.
- Modify: `lib/src/mono/external.rs`
  - Stop scanning dependency ASTs for generic functions/impls.
  - Keep product artifact cross-crate HIR and object-backed impl registration.
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
  - Mark downstream source-backed dependency consumption removed while keeping broader provider-boundary work in progress.

---

### Task 1: Add A Downstream Artifact-Only Guard

**Files:**
- Modify: `lib/src/crate_system/mod.rs`
- Modify: `lib/src/crate_system/tests.rs`

- [ ] **Step 1: Write failing `LoadedCrate` guard tests**

Add helpers and tests to `lib/src/crate_system/tests.rs`:

```rust
fn empty_manifest(name: &str) -> CrateManifest {
    CrateManifest {
        crate_: CrateConfig {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            no_std: false,
        },
        lib: LibConfig {
            path: "lib.rk".to_string(),
        },
        dependencies: None,
    }
}

fn empty_module() -> Module {
    Module {
        name: None,
        top_levels: Vec::new(),
        is_inline: false,
        filepath: None,
    }
}

#[test]
fn loaded_crate_rejects_source_backed_downstream_dependency() {
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

    let error = loaded
        .downstream_interface("dep", "collection")
        .expect_err("source-backed downstream dependencies should be rejected");

    assert!(error.contains("source-backed external dependency 'dep'"));
    assert!(error.contains("--extern-artifact dep=<path>"));
}

#[test]
fn loaded_crate_exposes_product_interface_for_downstream_dependency() {
    let interface = crate::crate_artifact::ArtifactCrateInterface::default();
    let loaded = LoadedCrate {
        manifest: empty_manifest("dep"),
        ast: empty_module(),
        interface: Some(interface),
        resolver: ResolverTables::default(),
        prelude_exports: BTreeMap::new(),
        object_path: None,
        cross_crate_hir: None,
        artifact_mode: ArtifactMode::InterfaceOnly,
        root_dir: PathBuf::from("/dep"),
        module_tree: None,
        file_cache: std::collections::HashMap::new(),
    };

    assert!(loaded.downstream_interface("dep", "collection").is_ok());
}
```

Also extend the imports at the top of `lib/src/crate_system/tests.rs`:

```rust
use crate::collect::resolver::ResolverTables;
```

- [ ] **Step 2: Run red tests**

Run:

```bash
cargo test -p rock-lib loaded_crate_rejects_source_backed_downstream_dependency -- --exact
cargo test -p rock-lib loaded_crate_exposes_product_interface_for_downstream_dependency -- --exact
```

Expected: FAIL to compile because `LoadedCrate::downstream_interface` does not exist.

- [ ] **Step 3: Add the guard implementation**

In `lib/src/crate_system/mod.rs`, extend the existing `impl LoadedCrate` block:

```rust
impl LoadedCrate {
    pub fn is_source_backed(&self) -> bool {
        matches!(self.artifact_mode, ArtifactMode::Source)
    }

    pub fn is_object_backed(&self) -> bool {
        matches!(self.artifact_mode, ArtifactMode::Object)
    }

    pub(crate) fn downstream_interface(
        &self,
        crate_name: &str,
        phase: &str,
    ) -> Result<&ArtifactCrateInterface, String> {
        if self.is_source_backed() {
            return Err(format!(
                "source-backed external dependency '{}' is not supported during {}; build it as a product artifact and pass it with --extern-artifact {}=<path>",
                crate_name, phase, crate_name
            ));
        }

        self.interface.as_ref().ok_or_else(|| {
            format!(
                "external dependency '{}' has no product artifact interface during {}",
                crate_name, phase
            )
        })
    }
}
```

- [ ] **Step 4: Run green tests**

Run:

```bash
cargo test -p rock-lib loaded_crate_rejects_source_backed_downstream_dependency -- --exact
cargo test -p rock-lib loaded_crate_exposes_product_interface_for_downstream_dependency -- --exact
```

Expected: PASS.

- [ ] **Step 5: Commit Task 1**

```bash
git add lib/src/crate_system/mod.rs lib/src/crate_system/tests.rs
git commit -m "crate-system: gate downstream dependencies to artifacts"
```

---

### Task 2: Stop Source-Backed Dependency Collection

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Replace the source-backed dependency bootstrap test with a rejection test**

In `lib/src/collect/mod.rs`, replace `collect_context_bootstraps_source_backed_dependency_crate` with:

```rust
#[test]
fn collect_context_rejects_source_backed_dependency_crate() {
    let mut crate_ctx = CrateContext::new();
    crate_ctx.register_crate(
        crate::crate_system::CrateManifest {
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
        PathBuf::from("/dep"),
        Module {
            name: None,
            top_levels: vec![TopLevel::StructDecl(StructDecl {
                name: type_inner("DepThing"),
                fields: vec![],
                exported: false,
            })],
            is_inline: false,
            filepath: None,
        },
    );

    let mut context = context::CollectContext::bootstrap_for_collection(false, Some("test"), None);
    context.register_crate_functions(&crate_ctx);

    assert!(!context.structs.contains_key("dep::DepThing"));
    assert!(context.errors.iter().any(|error| {
        error
            .message
            .contains("source-backed external dependency 'dep' is not supported during collection")
    }));
}
```

- [ ] **Step 2: Replace the normal `collect` source dependency test**

In `lib/src/collect/mod.rs`, replace `collect_preserves_dependency_bootstrap_while_using_local_collector` with:

```rust
#[test]
fn collect_rejects_source_backed_external_dependency_consumption() {
    let mut crate_ctx = CrateContext::new();
    crate_ctx.register_crate(
        crate::crate_system::CrateManifest {
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
        std::path::PathBuf::from("/dep"),
        Module {
            name: None,
            top_levels: vec![TopLevel::StructDecl(StructDecl {
                name: type_inner("DepThing"),
                fields: vec![],
                exported: false,
            })],
            is_inline: false,
            filepath: None,
        },
    );

    let program = program_with_struct("Point");
    let errors = match collect(&program, &crate_ctx, false, Some("test")) {
        Ok(_) => panic!("source-backed external dependency consumption should fail"),
        Err(errors) => errors,
    };

    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("source-backed external dependency 'dep' is not supported during collection")
    }));
}
```

- [ ] **Step 3: Run red collect tests**

Run:

```bash
cargo test -p rock-lib collect_context_rejects_source_backed_dependency_crate -- --exact
cargo test -p rock-lib collect_rejects_source_backed_external_dependency_consumption -- --exact
```

Expected: FAIL because source-backed dependency declarations are still collected.

- [ ] **Step 4: Route collection through `downstream_interface`**

In `lib/src/collect/context.rs`, replace the `if let Some(interface) = &loaded_crate.interface { ... }` source-backed branch in `register_loaded_crate` with this shape:

```rust
        let interface = match loaded_crate.downstream_interface(crate_name, "collection") {
            Ok(interface) => interface,
            Err(message) => {
                self.push_error(message);
                return;
            }
        };

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

        if is_stdlib && !loaded_crate.prelude_exports.is_empty() {
            self.stdlib_prelude_exports = loaded_crate
                .prelude_exports
                .iter()
                .map(|(name, source)| (name.clone(), Some(source.clone())))
                .collect();
        }
```

Keep the existing loops that register `interface.functions`, `interface.externs`, `interface.structs`, `interface.enums`, `interface.traits`, `interface.impls`, and `interface.infix_precedence`. Remove the fallthrough that copied `loaded_crate.file_cache` and called `self.collect_crate_declarations(&loaded_crate.ast, crate_name, is_stdlib)` for external dependencies.

- [ ] **Step 5: Run green collect tests and current-crate source-module guards**

Run:

```bash
cargo test -p rock-lib collect_context_rejects_source_backed_dependency_crate -- --exact
cargo test -p rock-lib collect_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib collect_indexes_loaded_source_backed_module_bodies -- --exact
cargo test -p rock-lib collect_preserves_source_backed_module_function_qualification_and_import_aliases -- --exact
```

Expected: PASS. The last two tests prove current-crate source modules are still supported.

- [ ] **Step 6: Commit Task 2**

```bash
git add lib/src/collect/context.rs lib/src/collect/mod.rs
git commit -m "collect: reject source-backed external dependencies"
```

---

### Task 3: Stop Source-Backed Dependency Lowering

**Files:**
- Modify: `lib/src/lower/crates/registration.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add a lower-from-declarations rejection test**

Add this test to the existing test module in `lib/src/collect/mod.rs`:

```rust
#[test]
fn lower_from_declarations_rejects_source_backed_external_dependency_consumption() {
    let program = program_with_struct("Point");
    let decls = collect(&program, &CrateContext::new(), false, Some("test"))
        .expect("collection without dependencies should succeed");

    let mut crate_ctx = CrateContext::new();
    crate_ctx.register_crate(
        crate::crate_system::CrateManifest {
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
        PathBuf::from("/dep"),
        Module {
            name: None,
            top_levels: vec![TopLevel::FunctionDecl(function_decl("identity"))],
            is_inline: false,
            filepath: None,
        },
    );

    let errors = match crate::lower::program::lower_from_declarations(
        &program,
        decls,
        &crate_ctx,
        Some("test"),
    ) {
        Ok(_) => panic!("source-backed external dependency lowering should fail"),
        Err(errors) => errors,
    };

    assert!(errors.iter().any(|error| {
        error
            .message
            .contains("source-backed external dependency 'dep' is not supported during lowering")
    }));
}
```

- [ ] **Step 2: Run the red lower test**

Run:

```bash
cargo test -p rock-lib lower_from_declarations_rejects_source_backed_external_dependency_consumption -- --exact
```

Expected: FAIL because lowering still reads source-backed dependency ASTs or silently accepts them.

- [ ] **Step 3: Route lower registration through the artifact interface guard**

In `lib/src/lower/crates/registration.rs`, replace the `if let Some(interface) = &loaded_crate.interface { if loaded_crate.is_source_backed() { ... } else { ... } }` block in `register_loaded_crate` with:

```rust
        let interface = match loaded_crate.downstream_interface(crate_name, "lowering") {
            Ok(interface) => interface,
            Err(message) => {
                self.push_error(message);
                return;
            }
        };
```

Keep the existing interface-registration loops after that guard. Remove the source-backed fallthrough that copies file cache and calls `self.collect_crate_declarations(&loaded_crate.ast, crate_name, is_stdlib)`.

- [ ] **Step 4: Remove source-backed body lowering branches**

In `lib/src/lower/crates/bodies.rs`, update `lower_crate_trait_bodies` to this shape:

```rust
    pub(crate) fn lower_crate_trait_bodies(&mut self, ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &ctx.crates {
            if let Err(message) = loaded_crate.downstream_interface(crate_name, "lowering") {
                self.push_error(message);
                continue;
            }

            if let Some(bundle) = &loaded_crate.cross_crate_hir {
                self.apply_cross_crate_trait_defaults(bundle);
            }
        }
    }
```

Update `lower_crate_module_bodies` to this shape:

```rust
    pub(crate) fn lower_crate_module_bodies(&mut self, ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &ctx.crates {
            if let Err(message) = loaded_crate.downstream_interface(crate_name, "lowering") {
                self.push_error(message);
                continue;
            }

            if let Some(bundle) = &loaded_crate.cross_crate_hir {
                self.apply_cross_crate_generic_bodies(bundle);
            }
        }
    }
```

Delete now-unused helpers from `lib/src/lower/crates/bodies.rs` that only lower dependency ASTs, including `lower_crate_trait_bodies_module`, `lower_crate_bodies`, `lower_crate_bodies_qualified`, and `lower_crate_function_body`, if `cargo check` reports they are unused after the branch removal.

- [ ] **Step 5: Run green lower and artifact body tests**

Run:

```bash
cargo test -p rock-lib lower_from_declarations_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_function_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_impl_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_trait_default_method_from_product_artifact -- --exact
```

Expected: PASS. The artifact tests prove cross-crate body bundles still replace dependency source lowering.

- [ ] **Step 6: Commit Task 3**

```bash
git add lib/src/lower/crates/registration.rs lib/src/lower/crates/bodies.rs lib/src/collect/mod.rs
git commit -m "lower: consume dependency bodies from artifacts only"
```

---

### Task 4: Stop Source-Backed Dependency Scanning In Mono

**Files:**
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add a monomorphization rejection test**

Add this test to the existing `#[cfg(test)] mod tests` in `lib/src/mono/external.rs`:

```rust
#[test]
#[should_panic(expected = "source-backed external dependency 'dep' is not supported during monomorphization")]
fn process_with_crates_rejects_source_backed_external_dependency_consumption() {
    let mut crate_ctx = CrateContext::new();
    crate_ctx.register_crate(
        CrateManifest {
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
        PathBuf::from("/dep"),
        Module {
            name: None,
            top_levels: vec![],
            is_inline: false,
            filepath: None,
        },
    );

    let program = crate::hir::HirProgram::from_parts(
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );

    let mut mono = Monomorphizer::new();
    let _ = mono.process_with_crates(program, &crate_ctx);
}
```

- [ ] **Step 2: Run the red mono test**

Run:

```bash
cargo test -p rock-lib process_with_crates_rejects_source_backed_external_dependency_consumption -- --exact
```

Expected: FAIL because mono currently accepts source-backed dependencies and scans dependency ASTs.

- [ ] **Step 3: Remove source-backed AST scanning from `load_external_generic_functions`**

In `lib/src/mono/external.rs`, replace the source-backed branch in `load_external_generic_functions` with a guard:

```rust
    fn load_external_generic_functions(&mut self, crate_ctx: &CrateContext) {
        for (crate_name, loaded_crate) in &crate_ctx.crates {
            if let Err(message) = loaded_crate.downstream_interface(crate_name, "monomorphization") {
                panic!("{}", message);
            }

            if loaded_crate.is_object_backed() {
                if let Some(interface) = &loaded_crate.interface {
                    for imp in &interface.impls {
                        if let Some(trait_name) = &imp.trait_name {
                            if imp.type_generics.is_empty()
                                && imp.trait_generics.is_empty()
                                && imp
                                    .methods
                                    .values()
                                    .all(|method| method.generic_params.is_empty())
                            {
                                self.trait_impls
                                    .entry(trait_name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(imp.clone());
                            }
                        }
                    }
                }
            }

            if let Some(bundle) = &loaded_crate.cross_crate_hir {
                for (qualified_name, func) in &bundle.generic_functions {
                    self.external_generic_functions
                        .insert(qualified_name.clone(), func.clone());
                }

                for imp in &bundle.generic_impls {
                    self.generic_impls
                        .insert(Self::generic_impl_key(imp), imp.clone());
                }
            }
        }
    }
```

- [ ] **Step 4: Delete source-backed mono helpers and tests**

Delete `collect_generic_functions_from_module` and `create_stub_generic_function` from `lib/src/mono/external.rs` if they become unused.

Delete the test `create_stub_generic_function_requires_canonical_def_id` because source-backed generic stub creation is no longer a supported downstream dependency path.

- [ ] **Step 5: Run green mono tests**

Run:

```bash
cargo test -p rock-lib process_with_crates_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib process_with_crates_records_object_backed_instances_without_re_emitting -- --exact
cargo test -p rock-lib test_compile_generic_function_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit Task 4**

```bash
git add lib/src/mono/external.rs
git commit -m "mono: reject source-backed external dependencies"
```

---

### Task 5: Remove Legacy Source-Backed External Coverage While Preserving Current-Crate Source Modules

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/mono/external.rs`
- Modify tests only where they describe downstream dependency source consumption.

- [ ] **Step 1: Grep for downstream source-backed branches**

Run:

```bash
rg "is_source_backed\(\)|loaded_crate\.ast|loaded_crate\.file_cache|collect_generic_functions_from_module|create_stub_generic_function" lib/src/collect lib/src/lower lib/src/mono
```

Expected before cleanup: matches may remain in deleted or modified code from Tasks 2-4.

- [ ] **Step 2: Remove or convert only external dependency source-consumption tests**

Keep tests for current-crate source modules, including:

```text
collect_indexes_loaded_source_backed_module_bodies
collect_indexes_source_backed_module_nested_inside_inline_module
collect_preserves_source_backed_module_function_qualification_and_import_aliases
collect_keeps_nested_source_backed_imports_out_of_global_aliases
lower_from_declarations_resolves_nested_source_backed_imports_inside_inline_module_bodies
lower_from_declarations_resolves_nested_imported_types_inside_inline_module_bodies
```

Remove or replace tests that assert a dependency loaded with `CrateContext::register_crate` becomes available to another crate through source-backed consumption. After Tasks 2-4, those tests should be the rejection tests added in this plan.

- [ ] **Step 3: Run the grep again**

Run:

```bash
rg "loaded_crate\.ast|loaded_crate\.file_cache|collect_generic_functions_from_module|create_stub_generic_function" lib/src/collect lib/src/lower lib/src/mono
```

Expected: no matches in downstream dependency paths. Matches in current-crate module code outside `loaded_crate.*` are acceptable.

Run:

```bash
rg "is_source_backed\(\)" lib/src/collect lib/src/lower lib/src/mono
```

Expected: no matches, or only a guard that immediately rejects source-backed dependencies without reading `loaded_crate.ast` or `loaded_crate.file_cache`.

- [ ] **Step 4: Run focused current-crate and artifact regressions**

Run:

```bash
cargo test -p rock-lib collect_indexes_loaded_source_backed_module_bodies -- --exact
cargo test -p rock-lib lower_from_declarations_resolves_nested_source_backed_imports_inside_inline_module_bodies -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_with_source_free_product_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_trait_default_method_from_product_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_run_with_stdlib_product_artifact_links -- --exact
```

Expected: PASS.

- [ ] **Step 5: Commit Task 5**

```bash
git add lib/src/collect/mod.rs lib/src/lower/crates/bodies.rs lib/src/mono/external.rs
git commit -m "tests: make artifact dependencies authoritative"
```

---

### Task 6: Update Audit Checklist And Run Final Verification

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Update checklist status**

In `docs/superpowers/plans/master-audit-checklist.md`, update the Crate And Artifact Interface Split section.

Add this checked item under `Done:`:

```markdown
- [x] Removed source-backed external dependency consumption from downstream collect/lower/mono paths; dependencies now enter compiler phases through product artifact data.
```

Keep these items in `Still to do:` because this slice does not fully split providers or remove every compatibility map:

```markdown
- [ ] Hide dependency storage mode behind provider boundaries instead of branching through lower, mono, and codegen.
- [ ] Split `LoadedCrate` into narrower metadata, interface, body-provider, and link-provider capabilities.
- [ ] Make cross-crate body access an explicit provider API.
```

If `Stop carrying mixed source/artifact state through lowering` is still present and Tasks 2-5 removed all source dependency branches from lowering, move it to `Done:` with this wording:

```markdown
- [x] Stopped carrying source-backed dependency AST/body state through lowering for downstream compilation.
```

- [ ] **Step 2: Run formatting**

Run:

```bash
cargo fmt --all --check
```

Expected: PASS. If it fails, run `cargo fmt --all`, inspect the formatted diff, and rerun `cargo fmt --all --check`.

- [ ] **Step 3: Run focused boundary tests**

Run:

```bash
cargo test -p rock-lib loaded_crate_rejects_source_backed_downstream_dependency -- --exact
cargo test -p rock-lib collect_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib lower_from_declarations_rejects_source_backed_external_dependency_consumption -- --exact
cargo test -p rock-lib process_with_crates_rejects_source_backed_external_dependency_consumption -- --exact
```

Expected: PASS.

- [ ] **Step 4: Run artifact and package-manager regressions**

Run:

```bash
cargo test -p rock-lib crate_artifact
cargo test -p rock-lib products
cargo test -p rockc
cargo test -p rock
```

Expected: PASS.

- [ ] **Step 5: Run full library tests**

Run:

```bash
cargo test -p rock-lib
```

Expected: PASS.

- [ ] **Step 6: Commit Task 6**

```bash
git add docs/superpowers/plans/master-audit-checklist.md
git commit -m "docs: mark artifact-only dependency boundary"
```

---

## Self-Review Checklist

- Spec coverage: Task 1 adds a reusable artifact-only guard; Tasks 2-4 remove source-backed external dependency consumption from collect/lower/mono; Task 5 removes legacy coverage while preserving current-crate source modules; Task 6 updates audit status and verifies artifact-backed behavior.
- Scope control: The plan does not remove current-crate source parsing, does not remove `rock` source path dependencies, does not require manual artifact prebuilds for `rock`, and does not redesign product artifacts unless implementation reveals a missing artifact capability.
- Type consistency: The plan consistently uses `LoadedCrate::downstream_interface`, `ArtifactCrateInterface`, `ArtifactMode::Source`, `CrateContext`, and product artifact cross-crate HIR names already present in the codebase.
- Verification: Each implementation task has focused tests, and the final task runs format, boundary tests, artifact/package tests, and `cargo test -p rock-lib`.
