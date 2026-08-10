# Stdlib Product Artifact Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make product artifacts the single public artifact format, move stdlib sysroot packaging to `rockup`, and remove `rock_lib` from `rock` and `rockup` dependencies.

**Architecture:** Add a lightweight shared contract crate for manifest/sysroot/tooling contracts, then collapse `rockc --extern-artifact` onto product artifacts. `rockup` provisions product stdlib sysroots by shelling out to `rockc`; `rock` consumes sysroots and only auto-refreshes the implicit workspace dev sysroot by invoking dev-target `rockup`.

**Tech Stack:** Rust 2021, Cargo workspace crates, `clap`, `toml`, optional `serde`, `std::process::Command`, existing `rockc --emit-artifact --emit-object`, `cargo test -p rock-shared`, `cargo test -p rock`, `cargo test -p rockup`, `cargo test -p rockc`, focused `rock-lib` tests.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-05-08-stdlib-product-artifact-migration-design.md`.

This slice is large but cohesive because the public artifact format, stdlib packaging owner, and `rock_lib` dependency removal are mutually dependent. Splitting the work would leave one of these broken states:
- `rockup` still linked to `rock_lib` for stdlib packaging.
- `rock` still expecting old stdlib artifacts.
- `rockc --extern-artifact` still meaning old artifacts while sysroot contains product artifacts.

Parallelization points:
- Task 1 is the shared prerequisite.
- After Task 1, Task 2 (`rockc`/`rock_lib` CLI collapse) and Task 3 (`rockup` packaging) can be dispatched to separate subagents if they coordinate only through the committed `rock-shared` API.
- Task 4 (`rock` migration) depends on Tasks 2 and 3.
- Tasks 5 and 6 are integration cleanup and verification and should run sequentially.

## File Structure

- Create: `rock-shared/Cargo.toml`
- Responsibility: lightweight shared crate with feature-gated manifest, sysroot, filesystem, and process helpers.
- Create: `rock-shared/src/lib.rs`
- Responsibility: feature-gated module exports.
- Create: `rock-shared/src/manifest.rs`
- Responsibility: `rock.toml` structs and parser currently in `rock_lib::crate_system`.
- Create: `rock-shared/src/sysroot.rs`
- Responsibility: sysroot constants, layout, resolution, and target triple contract currently duplicated in `rock_lib` and `rockup`.
- Create: `rock-shared/src/fs.rs`
- Responsibility: package source discovery and mtime/fingerprint helpers used by `rock`/`rockup` freshness checks.
- Create: `rock-shared/src/process.rs`
- Responsibility: dev-target sibling executable resolution shared by `rock` and `rockup`.
- Modify: `Cargo.toml`
- Responsibility: add `rock-shared` workspace member.
- Modify: `lib/Cargo.toml`
- Responsibility: depend on `rock-shared` manifest/sysroot/serde features, remove direct `toml` if unused after migration.
- Modify: `lib/src/crate_system/mod.rs`, `lib/src/crate_system/manifest.rs`, `lib/src/crate_system/tests.rs`, `lib/src/sysroot.rs`
- Responsibility: re-export shared manifest/sysroot contracts and keep compatibility wrappers where needed.
- Modify: `lib/src/lib.rs`, `rockc/src/main.rs`, `lib/src/crate_artifact/load.rs`, relevant tests
- Responsibility: make `--extern-artifact` product-only and remove `extern_product_artifacts`.
- Modify: `rockup/Cargo.toml`, `rockup/src/dev.rs`, `rockup/src/cli.rs`, `rockup/src/layout.rs`, `rockup/src/constants.rs`, `rockup/src/tests/*`
- Responsibility: remove `rock-lib` dependency, invoke `rockc` for product stdlib packaging, use shared contracts.
- Modify: `rock/Cargo.toml`, `rock/src/package.rs`, `rock/src/deps.rs`, `rock/src/bundled_sysroot.rs`, `rock/src/artifact.rs`, `rock/src/build.rs`, `rock/src/rockc.rs`, `rock/src/compile.rs`, `rock/src/tests/*`
- Responsibility: remove `rock-lib` dependency, consume product stdlib artifacts, auto-refresh dev workspace sysroot via `rockup` only.

---

### Task 1: Add Shared Manifest and Sysroot Contract Crate

**Files:**
- Create: `rock-shared/Cargo.toml`
- Create: `rock-shared/src/lib.rs`
- Create: `rock-shared/src/manifest.rs`
- Create: `rock-shared/src/sysroot.rs`
- Create: `rock-shared/src/fs.rs`
- Create: `rock-shared/src/process.rs`
- Modify: `Cargo.toml`
- Modify: `lib/Cargo.toml`
- Modify: `lib/src/crate_system/mod.rs`
- Modify: `lib/src/crate_system/manifest.rs`
- Modify: `lib/src/crate_system/tests.rs`
- Modify: `lib/src/sysroot.rs`
- Test: `rock-shared/src/manifest.rs`, `rock-shared/src/sysroot.rs`, `rock-shared/src/process.rs`

- [ ] **Step 1: Add failing shared crate tests**

Create `rock-shared/Cargo.toml`:

```toml
[package]
name = "rock-shared"
version = "0.1.0"
edition = "2021"

[features]
default = ["manifest", "sysroot"]
manifest = ["dep:toml"]
sysroot = []
fs = []
process = []
serde = ["dep:serde"]

[dependencies]
toml = { workspace = true, optional = true }
serde = { version = "1.0", features = ["derive"], optional = true }
```

Create `rock-shared/src/lib.rs`:

```rust
#[cfg(feature = "manifest")]
pub mod manifest;

#[cfg(feature = "sysroot")]
pub mod sysroot;

#[cfg(feature = "fs")]
pub mod fs;

#[cfg(feature = "process")]
pub mod process;
```

Create `rock-shared/src/manifest.rs` by moving the manifest structs from `lib/src/crate_system/mod.rs` and the parser body from `lib/src/crate_system/manifest.rs`. The public API must be free functions, not `CrateContext` methods:

```rust
use std::{collections::BTreeMap, path::Path};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub path: Option<String>,
    pub version: Option<String>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateManifest {
    pub crate_: CrateConfig,
    pub lib: LibConfig,
    pub dependencies: Option<BTreeMap<String, Dependency>>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateConfig {
    pub name: String,
    pub version: String,
    pub no_std: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibConfig {
    pub path: String,
}

pub fn load_manifest(manifest_path: &Path) -> Result<CrateManifest, String> {
    let content = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read rock.toml: {}", e))?;

    parse_rock_toml(&content)
}

pub fn parse_rock_toml(content: &str) -> Result<CrateManifest, String> {
    let parsed: toml::Value = content
        .parse()
        .map_err(|e| format!("Failed to parse rock.toml: {}", e))?;

    let crate_table = parsed
        .get("crate")
        .ok_or("rock.toml missing [crate] section")?
        .as_table()
        .ok_or("[crate] section must be a table")?;

    let name = crate_table
        .get("name")
        .ok_or("rock.toml missing crate.name")?
        .as_str()
        .ok_or("crate.name must be a string")?
        .to_string();

    let version = crate_table
        .get("version")
        .ok_or("rock.toml missing crate.version")?
        .as_str()
        .ok_or("crate.version must be a string")?
        .to_string();

    let no_std = crate_table
        .get("no_std")
        .map(|value| value.as_bool().ok_or("crate.no_std must be a boolean"))
        .transpose()?
        .unwrap_or(false);

    let lib_table = parsed
        .get("lib")
        .ok_or("rock.toml missing [lib] section")?
        .as_table()
        .ok_or("[lib] section must be a table")?;

    let lib_path = lib_table
        .get("path")
        .ok_or("rock.toml missing lib.path")?
        .as_str()
        .ok_or("lib.path must be a string")?
        .to_string();

    let dependencies = if let Some(deps_table) = parsed.get("dependencies") {
        let deps = deps_table.as_table().ok_or("[dependencies] must be a table")?;
        let mut dep_map = BTreeMap::new();
        for (dep_name, dep_value) in deps {
            let dep = dep_value
                .as_table()
                .ok_or_else(|| format!("Dependency '{}' must be a table", dep_name))?;
            let path = dep.get("path").and_then(|v| v.as_str()).map(str::to_string);
            let version = dep
                .get("version")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if path.is_none() && version.is_none() {
                return Err(format!(
                    "Dependency '{}' must have at least 'path' or 'version'",
                    dep_name
                ));
            }
            dep_map.insert(dep_name.clone(), Dependency { path, version });
        }
        Some(dep_map)
    } else {
        None
    };

    Ok(CrateManifest {
        crate_: CrateConfig {
            name,
            version,
            no_std,
        },
        lib: LibConfig { path: lib_path },
        dependencies,
    })
}
```

Add tests in `rock-shared/src/manifest.rs` by moving the current tests from `lib/src/crate_system/tests.rs` and changing calls from `CrateContext::parse_rock_toml` to `parse_rock_toml`.

Create `rock-shared/src/sysroot.rs` by moving the constants/types/functions from `lib/src/sysroot.rs`. Keep the public names exactly the same so compatibility re-exports are simple:

```rust
use std::{env, path::{Path, PathBuf}};

pub const ROCK_SYSROOT_ENV: &str = "ROCK_SYSROOT";
pub const CARGO_TARGET_DIR_ENV: &str = "CARGO_TARGET_DIR";
pub const STDLIB_CRATE_NAME: &str = "stdlib";
pub const STDLIB_ARTIFACT_FILE_NAME: &str = "stdlib.rkca";
pub const STDLIB_OBJECT_FILE_NAME: &str = "stdlib.o";
pub const TOOLCHAIN_MANIFEST_FILE_NAME: &str = "manifest.json";
pub const COMPONENTS_MANIFEST_FILE_NAME: &str = "components.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysrootSource {
    Cli,
    Env,
    CargoTargetDir,
    CurrentDirTarget,
    ExecutableRelative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysrootResolution {
    pub path: PathBuf,
    pub source: SysrootSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysrootLayout {
    pub sysroot: PathBuf,
    pub target_triple: String,
    pub target_libdir: PathBuf,
    pub stdlib_artifact: PathBuf,
    pub stdlib_object: PathBuf,
    pub manifest_path: PathBuf,
    pub components_path: PathBuf,
}
```

Move the existing implementations for `SysrootResolution::is_explicit`, `SysrootResolution::into_layout`, `SysrootLayout::new`, `resolve_sysroot`, `resolve_sysroot_from`, `sysroot_from_executable`, `host_target_triple`, `host_target_vendor`, `host_target_env`, and the sysroot tests from `lib/src/sysroot.rs`.

Create `rock-shared/src/process.rs` with the dev-target executable resolver currently embedded in `rock/src/rockc.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn dev_target_binary_from_current_exe(binary_name: &str) -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable path: {}", e))?;
    dev_target_binary_from_exe(&current_exe, binary_name)
}

pub fn dev_target_binary_from_exe(current_exe: &Path, binary_name: &str) -> Result<PathBuf, String> {
    let profile_dir = if current_exe
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        == Some("deps")
    {
        current_exe
            .parent()
            .and_then(|deps| deps.parent())
            .ok_or_else(|| format!("Failed to resolve Cargo profile directory from {}", current_exe.display()))?
            .to_path_buf()
    } else {
        current_exe
            .parent()
            .ok_or_else(|| format!("Failed to resolve executable directory from {}", current_exe.display()))?
            .to_path_buf()
    };

    let executable_name = if cfg!(windows) {
        format!("{}.exe", binary_name)
    } else {
        binary_name.to_string()
    };
    Ok(profile_dir.join(executable_name))
}
```

Add `rock-shared/src/process.rs` tests equivalent to current `rockc_path_from_current_exe` tests, using `dev_target_binary_from_exe(&PathBuf::from("/workspace/target/debug/deps/rock-abc123"), "rockc")` and expecting `/workspace/target/debug/rockc`.

Create `rock-shared/src/fs.rs` with source input helpers currently needed by `rock/src/artifact.rs` freshness checks:

```rust
use std::{fs, path::{Path, PathBuf}, time::SystemTime};

pub fn file_modified(path: &Path) -> Result<SystemTime, String> {
    fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?
        .modified()
        .map_err(|e| format!("Failed to read modified time for {}: {}", path.display(), e))
}

pub fn collect_package_source_inputs(crate_root: &Path, build_dir_name: &str) -> Result<Vec<PathBuf>, String> {
    let mut inputs = vec![crate_root.join("rock.toml")];
    collect_rock_sources(crate_root, crate_root, build_dir_name, &mut inputs)?;
    inputs.sort();
    inputs.dedup();
    Ok(inputs)
}

fn collect_rock_sources(
    root: &Path,
    current: &Path,
    build_dir_name: &str,
    inputs: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| {
        format!("Failed to read package directory {} while checking artifact cache: {}", current.display(), e)
    })? {
        let entry = entry.map_err(|e| {
            format!("Failed to read package directory entry in {} while checking artifact cache: {}", current.display(), e)
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!("Failed to read file type for {} while checking artifact cache: {}", path.display(), e)
        })?;

        if file_type.is_dir() {
            if path == root.join(build_dir_name) {
                continue;
            }
            collect_rock_sources(root, &path, build_dir_name, inputs)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rk") {
            inputs.push(path);
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Wire the shared crate into the workspace**

In root `Cargo.toml`, add `rock-shared` to workspace members:

```toml
members = [
  "rockc",
  "rock",
  "lib",
  "rockup",
  "rock-shared",
]
```

In `lib/Cargo.toml`, add:

```toml
rock-shared = { path = "../rock-shared", features = ["manifest", "sysroot", "serde"] }
```

Remove `toml = { workspace = true }` from `lib/Cargo.toml` after the manifest parser is moved and `cargo test -p rock-lib` confirms no direct `toml` usage remains.

- [ ] **Step 3: Re-export shared manifest contracts from `rock-lib`**

In `lib/src/crate_system/mod.rs`, remove the local definitions of `Dependency`, `CrateManifest`, `CrateConfig`, and `LibConfig`; replace them with:

```rust
pub use rock_shared::manifest::{CrateConfig, CrateManifest, Dependency, LibConfig};
```

Keep the rest of `LoadedCrate`, `CrateContext`, `ArtifactMode`, `ModuleTree`, and `ModuleNode` in `rock-lib`.

In `lib/src/crate_system/manifest.rs`, replace the parser implementation with compatibility wrappers:

```rust
use std::path::Path;

use super::{CrateContext, CrateManifest};

impl CrateContext {
    pub fn load_manifest(manifest_path: &Path) -> Result<CrateManifest, String> {
        rock_shared::manifest::load_manifest(manifest_path)
    }

    pub fn parse_rock_toml(content: &str) -> Result<CrateManifest, String> {
        rock_shared::manifest::parse_rock_toml(content)
    }
}
```

In `lib/src/crate_system/tests.rs`, keep the existing tests for compatibility wrappers or move them to `rock-shared` and leave one smoke test:

```rust
#[test]
fn test_crate_context_parse_rock_toml_delegates_to_shared_parser() {
    let content = r#"
[crate]
name = "test_crate"
version = "0.1.0"

[lib]
path = "src/lib.rk"
"#;

    let manifest = crate::crate_system::CrateContext::parse_rock_toml(content).unwrap();
    assert_eq!(manifest.crate_.name, "test_crate");
    assert_eq!(manifest.lib.path, "src/lib.rk");
}
```

- [ ] **Step 4: Re-export shared sysroot contract from `rock-lib`**

Replace the body of `lib/src/sysroot.rs` with a compatibility re-export and keep tests in `rock-shared`:

```rust
pub use rock_shared::sysroot::*;
```

If any `rock-lib` tests rely on private helpers from the old module, move those tests to `rock-shared/src/sysroot.rs` and update calls to the shared public API.

- [ ] **Step 5: Run shared-crate focused tests**

Run: `cargo test -p rock-shared --features manifest,sysroot,fs,process -- --nocapture`

Expected: PASS with manifest parser tests, sysroot layout/resolution tests, and process resolver tests.

Run: `cargo test -p rock-lib crate_system -- --nocapture`

Expected: PASS, proving compatibility wrappers still work.

Run: `cargo test -p rock-lib sysroot -- --nocapture`

Expected: PASS or 0 filtered tests if sysroot tests moved to `rock-shared`.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rock-shared lib/Cargo.toml lib/src/crate_system/mod.rs lib/src/crate_system/manifest.rs lib/src/crate_system/tests.rs lib/src/sysroot.rs
git commit -m "shared: add manifest and sysroot contracts"
```

---

### Task 2: Collapse `rockc` and `rock-lib` Artifact Inputs

**Parallelizable:** Can run in parallel with Task 3 after Task 1 is committed if Task 3 avoids touching `rockc/src/main.rs` and `lib/src/lib.rs`.

**Files:**
- Modify: `rockc/src/main.rs`
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/tests/integration.rs`
- Test: `rockc/src/main.rs`, `lib/src/lib.rs`

- [ ] **Step 1: Add failing CLI collapse tests**

In `rockc/src/main.rs`, replace `test_extern_product_artifact_parses` with:

```rust
#[test]
fn test_extern_product_artifact_is_rejected() {
    let error = Config::try_parse_from([
        "rockc",
        "--entry-file",
        "main.rk",
        "--extern-product-artifact",
        "dep=build/dep.rkca",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("unexpected argument"));
}
```

Update `test_run_config_consumes_product_artifact_dependency` and `test_run_config_consumes_product_artifact_generic_dependency` so the app config uses `--extern-artifact`:

```rust
let extern_artifact = format!("dep={}", dep_artifact.display());
let app_config = Config::try_parse_from([
    "rockc",
    "--entry-file",
    app_entry.to_str().unwrap(),
    "--output-dir",
    app_build.to_str().unwrap(),
    "--no-std",
    "--no-prelude",
    "--extern-artifact",
    extern_artifact.as_str(),
])
.unwrap();
```

In `lib/src/lib.rs`, replace the duplicate cross-list test with a duplicate single-list test:

```rust
#[test]
fn compile_rejects_duplicate_extern_artifact_names() {
    let config = Config {
        entry_file: PathBuf::from("main.rk"),
        extern_artifacts: vec![
            ("dep".to_string(), "first.rkca".into()),
            ("dep".to_string(), "second.rkca".into()),
        ],
        ..Config::default()
    };

    let error = validate_extern_artifact_names(&config).unwrap_err();
    assert!(error.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("Duplicate external artifact crate name 'dep'")
    }));
}
```

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rockc test_extern_product_artifact_is_rejected -- --nocapture`

Expected: FAIL because `--extern-product-artifact` still parses.

Run: `cargo test -p rock-lib compile_rejects_duplicate_extern_artifact_names -- --nocapture`

Expected: FAIL to compile or fail behaviorally because `extern_product_artifacts` still exists and duplicate validation still compares two lists.

- [ ] **Step 3: Collapse `rockc` CLI parsing**

In `rockc/src/main.rs`, remove this field from `Config`:

```rust
#[arg(long)]
extern_product_artifact: Vec<String>,
```

In `Config::into_compiler_config`, remove the `extern_product_artifacts` local and pass only `extern_artifacts` into `rock_lib::Config`:

```rust
let extern_artifacts = config
    .extern_artifact
    .iter()
    .map(|s| parse_name_path(s, "extern-artifact"))
    .collect();
```

Remove `extern_product_artifacts` from the `rock_lib::Config` initializer.

- [ ] **Step 4: Collapse `rock_lib::Config` artifact loading**

In `lib/src/lib.rs`, remove the `extern_product_artifacts` field from `Config`.

In `compile_impl`, replace both artifact loading loops with a single product-loader loop:

```rust
for (name, path) in &config.extern_artifacts {
    if let Err(e) = ctx.load_product_artifact_from_path(path.clone()) {
        let mut diagnostics = Diagnostics::default();
        diagnostics.push(diagnostic::Diagnostic::new(
            format!(
                "Failed to load external artifact '{}' from {}: {}",
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

Update `validate_extern_artifact_names` to check one list:

```rust
fn validate_extern_artifact_names(config: &Config) -> Result<(), Diagnostics> {
    let mut seen = std::collections::BTreeSet::new();
    for (name, _) in &config.extern_artifacts {
        if !seen.insert(name.clone()) {
            let mut diagnostics = Diagnostics::default();
            diagnostics.push(diagnostic::Diagnostic::new(
                format!("Duplicate external artifact crate name '{}'", name),
                Span::default(),
            ));
            return Err(diagnostics);
        }
    }

    Ok(())
}
```

Update `product_dependencies_from_config` to use only `extern_artifacts`:

```rust
fn product_dependencies_from_config(config: &Config) -> Vec<ProductDependencyIdentity> {
    config
        .extern_artifacts
        .iter()
        .map(|(name, artifact_path)| ProductDependencyIdentity {
            name: name.clone(),
            artifact_path: artifact_path.clone(),
        })
        .collect()
}
```

Remove all `extern_product_artifacts: vec![]` initializers across the workspace. Current known files with initializers are:

- `rockup/src/dev.rs`
- `rock/src/compile.rs`
- `lib/tests/integration.rs`
- `lib/src/lower/program.rs`
- `lib/src/lower/crates/bodies.rs`
- `lib/src/crate_artifact/tests.rs`
- `lib/src/collect/context.rs`

Run: `git grep -n "extern_product_artifacts:" -- '*.rs'`

Expected after editing: only the removed `Config` field location or no matches remain before moving to the next step.

- [ ] **Step 5: Keep old loader internal only**

In `lib/src/crate_artifact/load.rs`, keep `load_artifact_from_path` and `load_artifact_from_path_with_root` for legacy tests, but add a short comment above `load_artifact_from_path`:

```rust
// Legacy old-format artifact loader. The public `rockc --extern-artifact`
// path uses `load_product_artifact_from_path`; this remains only for tests
// and pending old-artifact deletion work.
```

- [ ] **Step 6: Run focused green tests**

Run: `cargo test -p rockc -- --nocapture`

Expected: PASS; product dependency runtime tests now use `--extern-artifact`.

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib compile_rejects_duplicate_extern_artifact_names -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add rockc/src/main.rs lib/src/lib.rs lib/src/crate_artifact/load.rs lib/tests/integration.rs lib/src/lower/program.rs lib/src/lower/crates/bodies.rs lib/src/crate_artifact/tests.rs lib/src/collect/context.rs rock/src/compile.rs rockup/src/dev.rs
git commit -m "rockc: make extern artifacts product artifacts"
```

Before running the `git add` command, use `git status --short` to ensure only expected files are present. Do not add the pre-existing untracked `docs/superpowers/plans/2026-05-07-rockc-product-artifact-emission.md`.

---

### Task 3: Move Stdlib Product Packaging to `rockup`

**Parallelizable:** Can run in parallel with Task 2 after Task 1 if the subagent uses the current temporary flag in tests and expects a final integration update. Prefer sequential execution if using a single worktree.

**Files:**
- Modify: `rockup/Cargo.toml`
- Modify: `rockup/src/dev.rs`
- Modify: `rockup/src/cli.rs`
- Modify: `rockup/src/layout.rs`
- Modify: `rockup/src/constants.rs`
- Modify: `rockup/src/tests/dev.rs`
- Modify: `rockup/src/tests/cli.rs`
- Test: `rockup/src/tests/dev.rs`, `rockup/src/tests/cli.rs`

- [ ] **Step 1: Add failing `rockup` CLI and packaging tests**

In `rockup/src/cli.rs`, extend `DevStdlibCommand::Package` with an explicit `rockc` override:

```rust
Package {
    #[arg(long)]
    path: PathBuf,
    #[arg(long)]
    sysroot: PathBuf,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    copy_source: bool,
    #[arg(long)]
    rockc: Option<PathBuf>,
}
```

Update the call site in `run` to pass `rockc.as_deref()`:

```rust
let packaged = package_dev_stdlib(
    &path,
    &sysroot,
    target,
    copy_source,
    rockc.as_deref(),
)?;
```

In `rockup/src/tests/cli.rs`, update `test_cli_parses_dev_stdlib_package_command` to include `--rockc /tmp/rockc` and assert it:

```rust
"--rockc",
"/tmp/rockc",
```

Pattern match field list:

```rust
DevStdlibCommand::Package {
    path,
    sysroot,
    target,
    copy_source,
    rockc,
}
```

Assertions:

```rust
assert_eq!(rockc.as_deref(), Some(std::path::Path::new("/tmp/rockc")));
```

In `rockup/src/tests/dev.rs`, add a command-builder test by extracting command construction into a helper in Step 3. The test should initially fail because no helper exists:

```rust
#[test]
fn test_stdlib_package_invocation_uses_rockc_product_outputs() {
    let temp_dir = temp_test_dir("stdlib_package_invocation");
    let sysroot = temp_dir.join("dev-sysroot");
    let stdlib = workspace_stdlib_root();
    let target = host_target_triple();
    let layout = crate::layout::ToolchainLayout::new(sysroot.clone());

    let invocation = crate::dev::build_stdlib_package_invocation(
        PathBuf::from("/tmp/rockc"),
        &stdlib,
        &layout,
    )
    .unwrap();
    let args = invocation.args_as_strings();

    assert_eq!(invocation.executable, PathBuf::from("/tmp/rockc"));
    assert!(args.windows(2).any(|pair| pair[0] == "--crate-name" && pair[1] == "stdlib"));
    assert!(args.contains(&"--no-std".to_string()));
    assert!(args.contains(&"--no-prelude".to_string()));
    assert!(args.contains(&"--no-link".to_string()));
    assert!(args.windows(2).any(|pair| pair[0] == "--emit-object" && pair[1] == layout.stdlib_object.to_string_lossy().as_ref()));
    assert!(args.windows(2).any(|pair| pair[0] == "--emit-artifact" && pair[1] == layout.stdlib_artifact.to_string_lossy().as_ref()));

    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run red rockup tests**

Run: `cargo test -p rockup test_cli_parses_dev_stdlib_package_command -- --nocapture`

Expected: FAIL until `rockc` override is added to the CLI struct and match pattern.

Run: `cargo test -p rockup test_stdlib_package_invocation_uses_rockc_product_outputs -- --nocapture`

Expected: FAIL because `build_stdlib_package_invocation` does not exist.

- [ ] **Step 3: Remove `rock_lib` dependency from `rockup` and add command model**

In `rockup/Cargo.toml`, replace:

```toml
rock-lib = { path = "../lib" }
```

with:

```toml
rock-shared = { path = "../rock-shared", features = ["manifest", "sysroot", "process"] }
```

In `rockup/src/layout.rs`, replace local target triple logic with `rock_shared::sysroot::host_target_triple`:

```rust
pub(crate) use rock_shared::sysroot::host_target_triple;
```

Keep `ToolchainLayout` but change constants to shared constants in `rockup/src/constants.rs`:

```rust
pub(crate) const STDLIB_ARTIFACT_NAME: &str = rock_shared::sysroot::STDLIB_ARTIFACT_FILE_NAME;
pub(crate) const STDLIB_OBJECT_NAME: &str = rock_shared::sysroot::STDLIB_OBJECT_FILE_NAME;
pub(crate) const TOOLCHAIN_MANIFEST_NAME: &str = rock_shared::sysroot::TOOLCHAIN_MANIFEST_FILE_NAME;
pub(crate) const COMPONENTS_MANIFEST_NAME: &str = rock_shared::sysroot::COMPONENTS_MANIFEST_FILE_NAME;
```

In `rockup/src/dev.rs`, remove all `rock_lib` imports. Add a small invocation type:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RockcInvocation {
    pub(crate) executable: PathBuf,
    pub(crate) args: Vec<std::ffi::OsString>,
}

impl RockcInvocation {
    pub(crate) fn args_as_strings(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn into_command(self) -> std::process::Command {
        let mut command = std::process::Command::new(self.executable);
        command.args(self.args);
        command
    }
}
```

Add resolver:

```rust
fn resolve_rockc_path(override_path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os("ROCKC") {
        return Ok(PathBuf::from(path));
    }
    rock_shared::process::dev_target_binary_from_current_exe("rockc")
}
```

Add command builder:

```rust
pub(crate) fn build_stdlib_package_invocation(
    executable: PathBuf,
    stdlib_root: &Path,
    layout: &crate::layout::ToolchainLayout,
) -> Result<RockcInvocation, String> {
    let manifest = rock_shared::manifest::load_manifest(&stdlib_root.join("rock.toml"))?;
    if manifest.crate_.name != rock_shared::sysroot::STDLIB_CRATE_NAME {
        return Err(format!(
            "Expected stdlib crate at {}, found '{}' instead",
            stdlib_root.display(),
            manifest.crate_.name
        ));
    }

    Ok(RockcInvocation {
        executable,
        args: vec![
            "--crate-name".into(),
            rock_shared::sysroot::STDLIB_CRATE_NAME.into(),
            "--entry-file".into(),
            stdlib_root.join(&manifest.lib.path).into_os_string(),
            "--output-dir".into(),
            layout.target_component_dir.clone().into_os_string(),
            "--no-std".into(),
            "--no-prelude".into(),
            "--no-link".into(),
            "--emit-object".into(),
            layout.stdlib_object.clone().into_os_string(),
            "--emit-artifact".into(),
            layout.stdlib_artifact.clone().into_os_string(),
        ],
    })
}
```

- [ ] **Step 4: Reimplement `package_dev_stdlib` through `rockc`**

Change signature in `rockup/src/dev.rs`:

```rust
pub(crate) fn package_dev_stdlib(
    stdlib_root: &Path,
    sysroot_root: &Path,
    target_triple: Option<String>,
    copy_source: bool,
    rockc: Option<&Path>,
) -> Result<PathBuf, String> {
```

Use `rock_shared::manifest::load_manifest`, `rock_shared::sysroot::STDLIB_CRATE_NAME`, and `crate::layout::ToolchainLayout`. Replace the in-process compile and old artifact build with:

```rust
let executable = resolve_rockc_path(rockc)?;
let invocation = build_stdlib_package_invocation(executable, &stdlib_root, &layout)?;
let status = invocation
    .into_command()
    .status()
    .map_err(|e| format!("Failed to spawn rockc for dev stdlib packaging: {}", e))?;
if !status.success() {
    return Err(format!(
        "rockc failed for dev stdlib packaging with status {}",
        status
    ));
}
```

Then call `write_sysroot_metadata(&layout, &manifest.crate_.version)?;` and copy source as before.

Update `write_sysroot_metadata` to use product format version. Add a shared constant in `rock-shared/src/sysroot.rs`:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 1;
```

Then use:

```rust
format!(
    "    \"artifact_format_version\": {}\n",
    rock_shared::sysroot::PRODUCT_ARTIFACT_FORMAT_VERSION
),
```

- [ ] **Step 5: Run rockup green tests**

Run: `cargo build -p rockc && cargo test -p rockup -- --nocapture`

Expected: PASS, and `test_package_dev_stdlib_writes_sysroot_layout` creates `stdlib.rkca`, `stdlib.o`, metadata, and source component without linking `rock_lib`.

Run: `cargo tree -p rockup --no-dev`

Expected: output does not contain `rock-lib`, `inkwell`, or `llvm-sys`.

- [ ] **Step 6: Commit**

```bash
git add rockup/Cargo.toml rockup/src/dev.rs rockup/src/cli.rs rockup/src/layout.rs rockup/src/constants.rs rockup/src/tests/dev.rs rockup/src/tests/cli.rs rock-shared/src/sysroot.rs
git commit -m "rockup: package stdlib through rockc"
```

---

### Task 4: Remove `rock_lib` From `rock` and Use Product Stdlib Sysroots

**Files:**
- Modify: `rock/Cargo.toml`
- Modify: `rock/src/package.rs`
- Modify: `rock/src/deps.rs`
- Modify: `rock/src/bundled_sysroot.rs`
- Modify: `rock/src/artifact.rs`
- Modify: `rock/src/build.rs`
- Modify: `rock/src/rockc.rs`
- Modify: `rock/src/compile.rs`
- Modify: `rock/src/tests/support.rs`
- Modify: `rock/src/tests/artifact.rs`
- Modify: `rock/src/tests/build.rs`
- Modify: `rock/src/tests/sysroot.rs`
- Test: `rock/src/tests/*`, `rock/src/rockc.rs`

- [ ] **Step 1: Add failing tests for single artifact flag and no `rock_lib` dependency**

In `rock/src/rockc.rs` tests, replace the two command tests with single-artifact expectations. For dependency invocation:

```rust
#[test]
fn test_dependency_invocation_uses_extern_artifact_for_all_deps() {
    let temp_dir = temp_test_dir("rockc_dep_invocation");
    write_package(&temp_dir, "dep_b", "lib.rk", &[], "relay = x -> x\n< relay\n");
    let package = load_package(temp_dir.clone());
    let invocation = build_dependency_artifact_invocation(
        PathBuf::from("/workspace/target/debug/rockc"),
        &package,
        &[
            ExternArtifact { name: "dep_a".to_string(), path: PathBuf::from("/tmp/dep_a.rkca") },
            ExternArtifact { name: STDLIB_CRATE_NAME.to_string(), path: PathBuf::from("/tmp/stdlib.rkca") },
        ],
    );
    let args = args_as_strings(&invocation);

    assert!(args.windows(2).any(|pair| pair[0] == "--extern-artifact" && pair[1] == "dep_a=/tmp/dep_a.rkca"));
    assert!(args.windows(2).any(|pair| pair[0] == "--extern-artifact" && pair[1] == "stdlib=/tmp/stdlib.rkca"));
    assert!(!args.contains(&"--extern-product-artifact".to_string()));

    let _ = std::fs::remove_dir_all(temp_dir);
}
```

For root invocation, use the same single slice and assert no `--extern-product-artifact`.

Add a dependency check test in `rock/src/tests/build.rs`:

```rust
#[test]
fn test_rock_cargo_toml_does_not_depend_on_rock_lib() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .unwrap();
    assert!(!manifest.contains("rock-lib"));
}
```

- [ ] **Step 2: Run red tests**

Run: `cargo test -p rock test_dependency_invocation_uses_extern_artifact_for_all_deps -- --nocapture`

Expected: FAIL until `rock/src/rockc.rs` command builders are simplified.

Run: `cargo test -p rock test_rock_cargo_toml_does_not_depend_on_rock_lib -- --nocapture`

Expected: FAIL because `rock/Cargo.toml` still depends on `rock-lib`.

- [ ] **Step 3: Remove `rock_lib` dependency from `rock`**

In `rock/Cargo.toml`, replace:

```toml
rock-lib = { path = "../lib" }
```

with:

```toml
rock-shared = { path = "../rock-shared", features = ["manifest", "sysroot", "fs", "process"] }
```

In `rock/src/package.rs`, replace `rock_lib::crate_system::{CrateContext, CrateManifest}` with:

```rust
use rock_shared::manifest::{self, CrateManifest};
```

Replace `CrateContext::load_manifest(&manifest_path)?` with:

```rust
let manifest = manifest::load_manifest(&manifest_path)?;
```

In `rock/src/deps.rs`, replace `rock_lib::crate_system::{CrateContext, Dependency}` with:

```rust
use rock_shared::manifest::Dependency;
```

Remove `load_package_graph_into_context`; it exists only for old in-process stdlib packaging and should be unused after this slice.

- [ ] **Step 4: Simplify `rock` subprocess command builders**

In `rock/src/rockc.rs`, change signatures:

```rust
pub(crate) fn build_dependency_artifact_invocation(
    executable: PathBuf,
    package: &Package,
    dependency_artifacts: &[ExternArtifact],
) -> RockcInvocation
```

```rust
pub(crate) fn build_root_invocation(
    executable: PathBuf,
    package: &Package,
    artifacts: &[ExternArtifact],
) -> RockcInvocation
```

In both builders, emit every artifact as:

```rust
for artifact in dependency_artifacts {
    invocation = invocation
        .arg("--extern-artifact")
        .arg(RockcInvocation::name_path_arg(&artifact.name, &artifact.path));
}
```

Remove all `--extern-product-artifact` emission.

Use shared process helper in `resolve_rockc_path`:

```rust
rock_shared::process::dev_target_binary_from_current_exe("rockc")
```

- [ ] **Step 5: Replace `rock` sysroot packaging with dev-only `rockup` invocation**

In `rock/src/bundled_sysroot.rs`, remove `CrateArtifact`, `CrateContext`, `ArtifactObjectOutput`, `compile_package_object`, and old `package_sysroot_stdlib` logic.

Use shared sysroot imports:

```rust
use rock_shared::sysroot::{self, SysrootLayout, SysrootResolution, SysrootSource, STDLIB_CRATE_NAME};
```

Add dev-target `rockup` resolver:

```rust
fn resolve_rockup_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("ROCKUP") {
        return Ok(PathBuf::from(path));
    }
    rock_shared::process::dev_target_binary_from_current_exe("rockup")
}
```

Replace implicit rebuild call with:

```rust
fn package_sysroot_stdlib_with_rockup(stdlib_root: &Path, layout: &SysrootLayout) -> Result<(), String> {
    let rockup = resolve_rockup_path()?;
    let status = std::process::Command::new(&rockup)
        .args([
            "dev",
            "stdlib",
            "package",
            "--path",
        ])
        .arg(stdlib_root)
        .arg("--sysroot")
        .arg(&layout.sysroot)
        .arg("--target")
        .arg(&layout.target_triple)
        .arg("--copy-source")
        .status()
        .map_err(|e| format!("Failed to spawn rockup for dev sysroot stdlib packaging: {}", e))?;

    if !status.success() {
        return Err(format!(
            "rockup failed for dev sysroot stdlib packaging with status {}",
            status
        ));
    }

    Ok(())
}
```

Constrain auto-rebuild to dev workspace sysroots:

```rust
fn can_auto_rebuild_stdlib(resolution: &SysrootResolution, workspace_stdlib_root: Option<&Path>) -> bool {
    matches!(resolution.source, SysrootSource::CurrentDirTarget)
        && workspace_stdlib_root
            .map(|root| root.join("rock.toml").exists())
            .unwrap_or(false)
}
```

In `ensure_sysroot_stdlib_available`, if explicit sysroot is invalid, keep returning the existing explicit sysroot error. If non-explicit and `can_auto_rebuild_stdlib` is true, invoke `package_sysroot_stdlib_with_rockup`.

Freshness should use metadata and mtimes instead of product deserialization. Reuse `rock_shared::fs::collect_package_source_inputs` and `file_modified` for workspace stdlib source comparisons.

- [ ] **Step 6: Remove old stdlib branch from dependency artifacts**

In `rock/src/artifact.rs`, remove `build_old_stdlib_artifact` and related imports. Build every package dependency with `build_dependency_artifact_invocation(resolve_rockc_path()?, &package, &artifacts)`.

Create one artifact list:

```rust
let mut artifacts = dependency_artifacts
    .iter()
    .map(|(name, path)| ExternArtifact {
        name: name.clone(),
        path: path.clone(),
    })
    .collect::<Vec<_>>();
artifacts.extend(implicit_artifact_inputs.iter().map(|path| ExternArtifact {
    name: STDLIB_CRATE_NAME.to_string(),
    path: path.clone(),
}));
```

Use `rock_shared::fs::collect_package_source_inputs` and `rock_shared::fs::file_modified` in `artifact_is_fresh`, then remove local duplicate helper functions.

- [ ] **Step 7: Unify root build artifact passing**

In `rock/src/build.rs`, remove product/old split. Create one `Vec<ExternArtifact>` from `extern_artifacts`, append sysroot stdlib product artifact when needed, and call:

```rust
let invocation = build_root_invocation(resolve_rockc_path()?, &package, &artifacts);
```

In `rock/src/compile.rs`, delete `compile_package_object` if it is unused after Step 6. Keep `output_executable_path`.

- [ ] **Step 8: Update `rock` tests to avoid product deserialization through `rock_lib`**

In `rock/src/tests/support.rs`, remove `use rock_lib::products::CompilerProducts` and `read_product_artifact`.

Replace product deserialization assertions in `rock/src/tests/artifact.rs`, `rock/src/tests/build.rs`, and `rock/src/tests/sysroot.rs` with black-box behavior assertions:
- artifact path exists
- object path exists
- dependent executable runs with expected exit code/output
- sysroot metadata files exist
- mtime changes when source changes

Keep no-std tests that assert `Unknown variable: max` is surfaced through the subprocess error context.

- [ ] **Step 9: Run rock green tests**

Run: `cargo build -p rockc && cargo build -p rockup && cargo test -p rock -- --nocapture`

Expected: PASS. The `rock` crate should compile without `rock-lib` and should auto-refresh workspace stdlib through dev-target `rockup` when needed.

Run: `cargo tree -p rock --no-dev`

Expected: output does not contain `rock-lib`, `inkwell`, or `llvm-sys`.

- [ ] **Step 10: Commit**

```bash
git add rock/Cargo.toml rock/src/package.rs rock/src/deps.rs rock/src/bundled_sysroot.rs rock/src/artifact.rs rock/src/build.rs rock/src/rockc.rs rock/src/compile.rs rock/src/tests
git commit -m "rock: consume product sysroots without rock-lib"
```

---

### Task 5: Rewrite Legacy Stdlib Artifact Tests and Integration Helpers

**Files:**
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/tests/integration.rs`
- Modify: `rockc/src/main.rs`
- Test: `rock-lib` focused tests, `rockc` tests

- [ ] **Step 1: Remove old stdlib artifact helper from integration tests**

In `lib/tests/integration.rs`, replace old stdlib artifact helper logic with a product stdlib helper that shells out to `rockc --emit-artifact --emit-object --no-link`. Use the same command shape as `rockup` dev stdlib packaging.

Remove any `extern_product_artifacts` initializer and pass stdlib product artifact through `extern_artifacts`.

- [ ] **Step 2: Rewrite old stdlib `CrateArtifact` tests that represent public behavior**

In `lib/src/crate_artifact/tests.rs`, inspect these tests:
- `test_compile_with_stdlib_artifact`
- `test_run_with_stdlib_artifact_stdlib_method_delegation`
- `test_compile_requires_explicit_stdlib_artifact_even_with_sysroot`
- `test_explicit_stdlib_artifact_works_when_sysroot_is_set`
- `test_compile_without_explicit_stdlib_artifact_fails_with_no_std`
- `test_compile_with_interface_only_stdlib_artifact`

For tests that assert public CLI/product behavior, rewrite them to build product stdlib artifacts and pass them through `extern_artifacts`.

For tests that only assert legacy old `CrateArtifact` internals, keep them if they still pass and rename them to include `legacy_old_artifact`, for example:

```rust
fn test_legacy_old_artifact_compile_with_interface_only_stdlib_artifact() {
```

Do not leave tests whose names imply stdlib public behavior but still use old `CrateArtifact` bytes.

- [ ] **Step 3: Ensure `rockc` tests use only `--extern-artifact`**

In `rockc/src/main.rs`, grep for `extern-product-artifact`. The only remaining reference should be the rejection test name/string.

Run: `cargo test -p rockc -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Run focused old/product tests**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib crate_artifact -- --nocapture`

Expected: PASS. Remaining old artifact tests should be intentionally legacy/internal only.

- [ ] **Step 5: Commit**

```bash
git add lib/src/crate_artifact/tests.rs lib/tests/integration.rs rockc/src/main.rs
git commit -m "tests: move stdlib artifact coverage to products"
```

---

### Task 6: Final Verification and Dependency Audit

**Files:**
- No planned source edits.
- Test: full touched workspace packages and dependency graph.

- [ ] **Step 1: Verify formatting**

Run: `cargo fmt -p rock-shared --check && cargo fmt -p rock --check && cargo fmt -p rockup --check && cargo fmt -p rockc --check && cargo fmt -p rock-lib --check`

Expected: PASS. If this fails, run the same commands without `--check` package-by-package, then rerun the check command.

- [ ] **Step 2: Build dev tools**

Run: `cargo build -p rockc && cargo build -p rockup && cargo build -p rock`

Expected: PASS.

- [ ] **Step 3: Run shared and tool tests**

Run: `cargo test -p rock-shared --features manifest,sysroot,fs,process -- --nocapture`

Expected: PASS.

Run: `cargo test -p rockup -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock -- --nocapture`

Expected: PASS.

Run: `cargo test -p rockc -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Run focused compiler tests**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib crate_artifact -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Audit dependency graph**

Run: `cargo tree -p rock --no-dev`

Expected: output does not contain `rock-lib`, `inkwell`, or `llvm-sys`.

Run: `cargo tree -p rockup --no-dev`

Expected: output does not contain `rock-lib`, `inkwell`, or `llvm-sys`.

Run: `cargo tree -p rock-lib --no-dev`

Expected: output contains `rock-shared` and still contains compiler dependencies such as `inkwell`.

- [ ] **Step 6: Grep removed temporary surfaces**

Run: use Grep for `extern_product_artifacts`, `--extern-product-artifact`, `extern-product-artifact`, and `build_old_stdlib_artifact`.

Expected:
- no `extern_product_artifacts` field or initializer remains
- no `--extern-product-artifact` emission remains
- only a rejection test may mention `extern-product-artifact`
- no `build_old_stdlib_artifact` remains

- [ ] **Step 7: Inspect working tree**

Run: `git status --short`

Expected: only the pre-existing untracked `docs/superpowers/plans/2026-05-07-rockc-product-artifact-emission.md` and this new plan file if it has not been committed separately.

---

## Self-Review Notes

Spec coverage:
- Product artifacts as public format: Tasks 2, 4, and 5.
- Remove `--extern-product-artifact`: Task 2 and Task 6 grep audit.
- `rockup` owns stdlib packaging: Task 3.
- Dev-only `rock` auto-refresh via `rockup`: Task 4.
- Remove `rock_lib` from `rock` and `rockup`: Tasks 3, 4, and Task 6 dependency audit.
- Shared feature-gated contract crate: Task 1.
- Preserve no-std/no-prelude: Task 4 keeps existing tests and command flags.
- Keep old `CrateArtifact` internal only: Task 5.

Placeholder scan:
- No section uses vague deferment language.
- Code snippets name exact files, functions, commands, and expected test outcomes.

Type consistency:
- Shared crate name is consistently `rock-shared`.
- Manifest types stay `Dependency`, `CrateManifest`, `CrateConfig`, and `LibConfig`.
- Sysroot constants keep existing public names.
- Product artifact input field remains `extern_artifacts` after the CLI collapse.
