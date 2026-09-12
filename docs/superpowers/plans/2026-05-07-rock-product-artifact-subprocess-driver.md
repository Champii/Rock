# Rock Product Artifact Subprocess Driver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `rock` build package dependencies and root executables by invoking the active dev-target `rockc`, using product artifacts for package dependencies.

**Architecture:** Keep package graph traversal and freshness orchestration in `rock`, but move compiler execution for normal packages behind a new `rock/src/rockc.rs` subprocess boundary. Dependency packages are built with `rockc --emit-artifact --emit-object --no-link` and passed onward as `--extern-product-artifact`; stdlib remains an old-format sysroot `--extern-artifact` input for this slice.

**Tech Stack:** Rust 2021, `std::process::Command`, existing `rockc` CLI flags, `rock_lib::products::CompilerProducts`, `cargo build -p rockc`, `cargo test -p rock`, `cargo test -p rockc`.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-05-07-rock-product-artifact-subprocess-driver-design.md`.

This slice builds:
- A `rockc` subprocess helper in `rock`.
- Dev-target `rockc` resolution from the current executable profile directory, with optional `ROCKC` override.
- Product artifact/object dependency emission through `rockc`.
- Root executable builds through `rockc`.
- Product-artifact freshness based on output, source, dependency artifact, and stdlib artifact mtimes.
- Tests proving package dependency artifacts are product artifacts, stdlib still uses old `--extern-artifact`, and root builds/run keep working.

This slice does not build:
- Product stdlib artifacts.
- Product loading through `--extern-artifact`.
- Removal of `CrateContext::build_artifact` or old `CrateArtifact` support from `rock_lib`.
- Registry dependency support.

## File Structure

- Create: `rock/src/rockc.rs`
- Responsibility: resolve the dev-target `rockc` path, model command arguments for tests, build dependency/root invocations, run `rockc`, and report subprocess failures with package context.
- Modify: `rock/src/main.rs`
- Responsibility: register the new `rockc` module.
- Modify: `rock/src/artifact.rs`
- Responsibility: stop building normal package dependency artifacts in-process; collect direct dependency product artifacts; run dependency `rockc` invocations; replace old artifact deserialization freshness with product-output mtime freshness.
- Modify: `rock/src/build.rs`
- Responsibility: build root executables through the `rockc` subprocess helper, passing package deps as product artifacts and stdlib as old artifact.
- Modify: `rock/src/tests/support.rs`
- Responsibility: add product artifact reader helper and small mtime helper for tests.
- Modify: `rock/src/tests/artifact.rs`
- Responsibility: update cache/freshness expectations to product artifacts and add source/object freshness regressions.
- Modify: `rock/src/tests/build.rs`
- Responsibility: update transitive build assertions to read product artifacts, not old `CrateArtifact`.
- Modify: `rock/src/tests/sysroot.rs`
- Responsibility: keep sysroot regressions passing and add one command-construction regression for old-format stdlib input if not covered in `rockc.rs` tests.

Tests in `rock` that execute `build_project` now require a dev-target `rockc` binary. Run `cargo build -p rockc` before `cargo test -p rock` during verification. The code should return a clear missing-binary error if `target/<profile>/rockc` is absent.

---

### Task 1: Add `rockc` Subprocess Command Model

**Files:**
- Create: `rock/src/rockc.rs`
- Modify: `rock/src/main.rs`
- Test: `rock/src/rockc.rs`

- [ ] **Step 1: Register the module**

Add this module declaration in `rock/src/main.rs` with the other modules:

```rust
mod rockc;
```

- [ ] **Step 2: Create the failing resolver and command-builder tests**

Create `rock/src/rockc.rs` with this test-first skeleton:

```rust
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{bundled_sysroot::STDLIB_CRATE_NAME, package::Package};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExternArtifact {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RockcInvocation {
    pub(crate) executable: PathBuf,
    pub(crate) args: Vec<OsString>,
}

impl RockcInvocation {
    fn new(executable: PathBuf) -> Self {
        Self {
            executable,
            args: Vec::new(),
        }
    }

    fn arg(mut self, value: impl Into<OsString>) -> Self {
        self.args.push(value.into());
        self
    }

    fn name_path_arg(name: &str, path: &Path) -> OsString {
        OsString::from(format!("{}={}", name, path.display()))
    }

    fn into_command(self) -> Command {
        let mut command = Command::new(self.executable);
        command.args(self.args);
        command
    }
}

pub(crate) fn resolve_rockc_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("ROCKC") {
        return Ok(PathBuf::from(path));
    }

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable path: {}", e))?;
    rockc_path_from_current_exe(&current_exe)
}

fn rockc_path_from_current_exe(current_exe: &Path) -> Result<PathBuf, String> {
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

    let executable_name = if cfg!(windows) { "rockc.exe" } else { "rockc" };
    Ok(profile_dir.join(executable_name))
}

pub(crate) fn build_dependency_artifact_invocation(
    executable: PathBuf,
    package: &Package,
    dependency_artifacts: &[ExternArtifact],
    old_artifacts: &[ExternArtifact],
) -> RockcInvocation {
    let _ = (package, dependency_artifacts, old_artifacts);
    RockcInvocation::new(executable)
}

pub(crate) fn build_root_invocation(
    executable: PathBuf,
    package: &Package,
    product_artifacts: &[ExternArtifact],
    old_artifacts: &[ExternArtifact],
) -> RockcInvocation {
    let _ = (package, product_artifacts, old_artifacts);
    RockcInvocation::new(executable)
}

pub(crate) fn run_invocation(invocation: RockcInvocation, context: &str) -> Result<(), String> {
    let executable = invocation.executable.clone();
    let status = invocation
        .into_command()
        .status()
        .map_err(|e| format!("Failed to spawn rockc for {} using {}: {}", context, executable.display(), e))?;

    if !status.success() {
        return Err(format!(
            "rockc failed for {} with status {}",
            context,
            status
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::support::{load_package, temp_test_dir, write_package};

    fn args_as_strings(invocation: &RockcInvocation) -> Vec<String> {
        invocation
            .args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn test_rockc_path_from_test_binary_uses_profile_dir() {
        let current = PathBuf::from("/workspace/target/debug/deps/rock-abc123");

        assert_eq!(
            rockc_path_from_current_exe(&current).unwrap(),
            PathBuf::from("/workspace/target/debug/rockc")
        );
    }

    #[test]
    fn test_rockc_path_from_binary_uses_same_directory() {
        let current = PathBuf::from("/workspace/target/release/rock");

        assert_eq!(
            rockc_path_from_current_exe(&current).unwrap(),
            PathBuf::from("/workspace/target/release/rockc")
        );
    }

    #[test]
    fn test_dependency_invocation_uses_product_deps_and_old_stdlib() {
        let temp_dir = temp_test_dir("rockc_dep_invocation");
        write_package(&temp_dir, "dep_b", "lib.rk", &[], "relay = x -> x\n< relay\n");
        let package = load_package(temp_dir.clone());
        let invocation = build_dependency_artifact_invocation(
            PathBuf::from("/workspace/target/debug/rockc"),
            &package,
            &[ExternArtifact {
                name: "dep_a".to_string(),
                path: PathBuf::from("/tmp/dep_a.rkca"),
            }],
            &[ExternArtifact {
                name: STDLIB_CRATE_NAME.to_string(),
                path: PathBuf::from("/tmp/stdlib.rkca"),
            }],
        );
        let args = args_as_strings(&invocation);

        assert_eq!(invocation.executable, PathBuf::from("/workspace/target/debug/rockc"));
        assert!(args.windows(2).any(|pair| pair[0] == "--crate-name" && pair[1] == "dep_b"));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--entry-file" && pair[1] == package.entry_file().to_string_lossy().as_ref()
        }));
        assert!(args.contains(&"--no-link".to_string()));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--emit-artifact" && pair[1] == package.artifact_path().to_string_lossy().as_ref()
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--emit-object" && pair[1] == package.object_path().to_string_lossy().as_ref()
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--extern-product-artifact" && pair[1] == "dep_a=/tmp/dep_a.rkca"
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--extern-artifact" && pair[1] == "stdlib=/tmp/stdlib.rkca"
        }));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_root_invocation_uses_product_deps_and_old_stdlib() {
        let temp_dir = temp_test_dir("rockc_root_invocation");
        write_package(&temp_dir, "app", "src/main.rk", &[], "main = -> 0\n");
        let package = load_package(temp_dir.clone());
        let invocation = build_root_invocation(
            PathBuf::from("/workspace/target/debug/rockc"),
            &package,
            &[ExternArtifact {
                name: "dep".to_string(),
                path: PathBuf::from("/tmp/dep.rkca"),
            }],
            &[ExternArtifact {
                name: STDLIB_CRATE_NAME.to_string(),
                path: PathBuf::from("/tmp/stdlib.rkca"),
            }],
        );
        let args = args_as_strings(&invocation);

        assert!(args.windows(2).any(|pair| {
            pair[0] == "--entry-file" && pair[1] == package.entry_file().to_string_lossy().as_ref()
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--output-dir" && pair[1] == package.build_dir().to_string_lossy().as_ref()
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--extern-product-artifact" && pair[1] == "dep=/tmp/dep.rkca"
        }));
        assert!(args.windows(2).any(|pair| {
            pair[0] == "--extern-artifact" && pair[1] == "stdlib=/tmp/stdlib.rkca"
        }));
        assert!(!args.contains(&"--emit-artifact".to_string()));
        assert!(!args.contains(&"--no-link".to_string()));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
```

- [ ] **Step 3: Run the failing command tests**

Run: `cargo test -p rock rockc::tests -- --nocapture`

Expected: FAIL because both invocation builders currently return empty argument lists.

- [ ] **Step 4: Implement command construction**

Replace the stub invocation-builder bodies in `rock/src/rockc.rs` with:

```rust
pub(crate) fn build_dependency_artifact_invocation(
    executable: PathBuf,
    package: &Package,
    dependency_artifacts: &[ExternArtifact],
    old_artifacts: &[ExternArtifact],
) -> RockcInvocation {
    let mut invocation = RockcInvocation::new(executable)
        .arg("--crate-name")
        .arg(package.manifest.crate_.name.clone())
        .arg("--entry-file")
        .arg(package.entry_file())
        .arg("--output-dir")
        .arg(package.object_dir())
        .arg("--no-link")
        .arg("--emit-object")
        .arg(package.object_path())
        .arg("--emit-artifact")
        .arg(package.artifact_path());

    if package.manifest.crate_.no_std || package.manifest.crate_.name == STDLIB_CRATE_NAME {
        invocation = invocation.arg("--no-std");
    }

    for artifact in dependency_artifacts {
        invocation = invocation
            .arg("--extern-product-artifact")
            .arg(RockcInvocation::name_path_arg(&artifact.name, &artifact.path));
    }

    for artifact in old_artifacts {
        invocation = invocation
            .arg("--extern-artifact")
            .arg(RockcInvocation::name_path_arg(&artifact.name, &artifact.path));
    }

    invocation
}

pub(crate) fn build_root_invocation(
    executable: PathBuf,
    package: &Package,
    product_artifacts: &[ExternArtifact],
    old_artifacts: &[ExternArtifact],
) -> RockcInvocation {
    let mut invocation = RockcInvocation::new(executable)
        .arg("--entry-file")
        .arg(package.entry_file())
        .arg("--output-dir")
        .arg(package.build_dir());

    if package.manifest.crate_.no_std || package.manifest.crate_.name == STDLIB_CRATE_NAME {
        invocation = invocation.arg("--no-std");
    }

    for artifact in product_artifacts {
        invocation = invocation
            .arg("--extern-product-artifact")
            .arg(RockcInvocation::name_path_arg(&artifact.name, &artifact.path));
    }

    for artifact in old_artifacts {
        invocation = invocation
            .arg("--extern-artifact")
            .arg(RockcInvocation::name_path_arg(&artifact.name, &artifact.path));
    }

    invocation
}
```

- [ ] **Step 5: Run command tests**

Run: `cargo test -p rock rockc::tests -- --nocapture`

Expected: PASS for all `rockc::tests`.

- [ ] **Step 6: Commit**

```bash
git add rock/src/main.rs rock/src/rockc.rs
git commit -m "rock: add rockc subprocess command builder"
```

---

### Task 2: Replace Dependency Artifact Freshness With Product Output Freshness

**Files:**
- Modify: `rock/src/artifact.rs`
- Test: `rock/src/tests/artifact.rs`

- [ ] **Step 1: Add failing freshness tests that do not read old artifacts**

In `rock/src/tests/artifact.rs`, update imports:

```rust
use rock_lib::products::CompilerProducts;
```

Replace old `read_artifact` checks in `test_rock_artifact_cache_reuse_and_invalidation` with product artifact checks and mtime checks:

```rust
    let dep_a_before = CompilerProducts::read_artifact_from_path(&dep_a_artifact).unwrap();
    let dep_a_export_before = dep_a_before.identity_table.export_names["answer"];
    let dep_a_mtime_before = fs::metadata(&dep_a_artifact).unwrap().modified().unwrap();
    let dep_a_object_mtime_before = fs::metadata(&dep_a_object).unwrap().modified().unwrap();
```

Replace the no-change assertions with:

```rust
    let dep_a_after_no_change = CompilerProducts::read_artifact_from_path(&dep_a_artifact).unwrap();
    let dep_a_mtime_after_no_change = fs::metadata(&dep_a_artifact).unwrap().modified().unwrap();
    let dep_a_object_mtime_after_no_change =
        fs::metadata(&dep_a_object).unwrap().modified().unwrap();
    assert_eq!(
        dep_a_export_before,
        dep_a_after_no_change.identity_table.export_names["answer"]
    );
    assert_eq!(dep_a_mtime_before, dep_a_mtime_after_no_change);
    assert_eq!(
        dep_a_object_mtime_before,
        dep_a_object_mtime_after_no_change
    );
```

Replace the after-change artifact assertion with:

```rust
    let dep_a_artifact_mtime_after_change = fs::metadata(&dep_a_artifact).unwrap().modified().unwrap();
    let dep_a_object_mtime_after_change = fs::metadata(&dep_a_object).unwrap().modified().unwrap();
    assert!(dep_a_artifact_mtime_after_change > dep_a_mtime_after_no_change);
    assert!(dep_a_object_mtime_after_change > dep_a_object_mtime_after_no_change);
```

Add this missing-artifact-output test after `test_missing_dependency_object_triggers_rebuild`:

```rust
#[test]
fn test_missing_product_artifact_triggers_rebuild() {
    let _guard = sysroot_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = temp_test_dir("missing_product_artifact_rebuild");
    let dep = temp_dir.join("dep");
    let app = temp_dir.join("app");

    write_package(&dep, "dep", "lib.rk", &[], "answer = x -> x\n< answer\n");
    write_package(
        &app,
        "app",
        "src/main.rk",
        &[("dep", dep.display().to_string())],
        "> dep::answer\n\nmain = -> answer 6\n",
    );

    let executable = build_project(&app).unwrap();
    let dep_package = load_package(dep.clone());
    let dep_artifact = dep_package.artifact_path();
    assert!(dep_artifact.exists());
    let status = ProcessCommand::new(&executable).status().unwrap();
    assert_eq!(status.code(), Some(6));

    fs::remove_file(&dep_artifact).unwrap();
    assert!(!dep_artifact.exists());

    let executable = build_project(&app).unwrap();
    assert!(dep_artifact.exists());
    let status = ProcessCommand::new(&executable).status().unwrap();
    assert_eq!(status.code(), Some(6));

    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run the failing freshness tests**

Run: `cargo build -p rockc && cargo test -p rock artifact -- --nocapture`

Expected: FAIL because current dependency artifacts are old `CrateArtifact` files and product deserialization fails.

- [ ] **Step 3: Remove old artifact deserialization from dependency freshness**

In `rock/src/artifact.rs`, remove these imports:

```rust
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

use rock_lib::{
    crate_artifact::{ArtifactSourceFingerprint, CrateArtifact, CRATE_ARTIFACT_FORMAT_VERSION},
    crate_system::CrateContext,
};
```

Replace them with:

```rust
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

use rock_lib::crate_artifact::ArtifactSourceFingerprint;
```

Leave `ArtifactSourceFingerprint`, `compute_source_fingerprint_if_available`, and `stable_hash_hex` for sysroot old-artifact freshness.

Replace `artifact_is_fresh` with this product-output freshness implementation:

```rust
fn artifact_is_fresh(
    package: &Package,
    artifact_path: &Path,
    dependency_artifacts: &[(String, PathBuf)],
    implicit_artifact_inputs: &[PathBuf],
) -> Result<bool, String> {
    let object_path = package.object_path();
    if !artifact_path.exists() || !object_path.exists() {
        return Ok(false);
    }

    let artifact_modified = file_modified(artifact_path)?;
    let object_modified = file_modified(&object_path)?;

    for source_path in collect_package_source_inputs(&package.root_dir)? {
        let source_modified = file_modified(&source_path)?;
        if source_modified > artifact_modified || source_modified > object_modified {
            return Ok(false);
        }
    }

    for (_, dependency_artifact_path) in dependency_artifacts {
        if !dependency_artifact_path.exists() {
            return Ok(false);
        }
        let dependency_modified = file_modified(dependency_artifact_path)?;
        if dependency_modified > artifact_modified || dependency_modified > object_modified {
            return Ok(false);
        }
    }

    for implicit_artifact_path in implicit_artifact_inputs {
        if !implicit_artifact_path.exists() {
            return Ok(false);
        }

        let implicit_modified = file_modified(implicit_artifact_path)?;
        if implicit_modified > artifact_modified || implicit_modified > object_modified {
            return Ok(false);
        }
    }

    Ok(true)
}

fn collect_package_source_inputs(crate_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut inputs = vec![crate_root.join("rock.toml")];
    collect_rock_sources(crate_root, crate_root, &mut inputs)?;
    inputs.sort();
    inputs.dedup();
    Ok(inputs)
}

fn collect_rock_sources(root: &Path, current: &Path, inputs: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| {
        format!(
            "Failed to read package directory {} while checking artifact cache: {}",
            current.display(),
            e
        )
    })? {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read package directory entry in {} while checking artifact cache: {}",
                current.display(),
                e
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to read file type for {} while checking artifact cache: {}",
                path.display(),
                e
            )
        })?;

        if file_type.is_dir() {
            if path == root.join(crate::package::BUILD_DIR) {
                continue;
            }
            collect_rock_sources(root, &path, inputs)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rk") {
            inputs.push(path);
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Run the freshness tests again**

Run: `cargo build -p rockc && cargo test -p rock artifact -- --nocapture`

Expected: still FAIL until dependency builds emit product artifacts in Task 3. The compile errors from unused old imports should be fixed in this step before moving on.

- [ ] **Step 5: Keep this change staged for Task 3**

Do not commit this task by itself. The product freshness code is useful only once Task 3 switches dependency builds to product artifacts. Leave the edits in the worktree and commit them with Task 3.

---

### Task 3: Build Dependency Artifacts Through `rockc`

**Files:**
- Modify: `rock/src/artifact.rs`
- Modify: `rock/src/tests/artifact.rs`
- Modify: `rock/src/tests/build.rs`
- Modify: `rock/src/tests/support.rs`
- Test: `rock/src/tests/artifact.rs`, `rock/src/tests/build.rs`

- [ ] **Step 1: Add product artifact test helper**

In `rock/src/tests/support.rs`, add this import:

```rust
use rock_lib::{crate_artifact::CrateArtifact, products::CompilerProducts};
```

Replace the existing `use rock_lib::crate_artifact::CrateArtifact;` import with the combined import above.

Add this helper below `read_artifact`:

```rust
pub(super) fn read_product_artifact(path: &Path) -> CompilerProducts {
    CompilerProducts::read_artifact_from_path(path).unwrap()
}
```

- [ ] **Step 2: Update dependency artifact tests to expect product artifacts**

In `rock/src/tests/artifact.rs`, update the support import list to include `read_product_artifact`:

```rust
use super::support::{
    assert_sysroot_stdlib_files_exist, load_package, read_product_artifact, sysroot_env_lock,
    temp_test_dir, write_package, write_package_with_options,
};
```

In `test_rock_artifact_command_builds_cache_file`, replace old artifact assertions with:

```rust
    let products = read_product_artifact(&artifact_path);
    assert_eq!(products.crate_identity.name, "dep_only");
    assert!(products.identity_table.export_names.contains_key("identity"));
```

Delete the `read_artifact(&artifact_path).source_bundle.is_none()` assertion because product artifacts do not have source bundles.

In `test_build_project_reuses_source_free_artifacts_without_dependency_sources`, replace:

```rust
    let dep_artifact = read_artifact(&dep_package.artifact_path());
    assert!(dep_artifact.source_bundle.is_none());
```

with:

```rust
    let dep_products = read_product_artifact(&dep_package.artifact_path());
    assert!(dep_products.identity_table.export_names.contains_key("answer"));
```

In `rock/src/tests/build.rs`, update imports:

```rust
use super::support::{
    load_package, read_product_artifact, sysroot_env_lock, temp_test_dir, write_package,
    write_package_with_options,
};
```

Replace old source-bundle assertions in `test_build_project_with_transitive_artifacts` with:

```rust
    assert!(
        read_product_artifact(&dep_a_package.artifact_path())
            .identity_table
            .export_names
            .contains_key("answer")
    );
    assert!(
        read_product_artifact(&dep_b_package.artifact_path())
            .identity_table
            .export_names
            .contains_key("relay")
    );
```

- [ ] **Step 3: Run tests and observe old-format failure**

Run: `cargo build -p rockc && cargo test -p rock test_rock_artifact_command_builds_cache_file -- --nocapture && cargo test -p rock test_build_project_with_transitive_artifacts -- --nocapture`

Expected: FAIL because `ensure_artifact` still writes old `CrateArtifact` files.

- [ ] **Step 4: Replace in-process dependency artifact building with `rockc` invocation**

In `rock/src/artifact.rs`, update imports from `crate` to remove `compile::compile_package_object`, `deps::crate_graph_uses_bundled_stdlib`, and `rock_lib::crate_system::CrateContext`. Add `rockc` helpers:

```rust
use crate::{
    bundled_sysroot::{ensure_sysroot_stdlib_available, package_requires_sysroot_stdlib, STDLIB_CRATE_NAME},
    deps::resolve_dependency_root,
    package::Package,
    rockc::{
        build_dependency_artifact_invocation, resolve_rockc_path, run_invocation, ExternArtifact,
    },
};
```

Replace `collect_dependency_artifacts` and remove `collect_dependency_artifacts_recursive`. The root build should receive only direct package dependency artifacts; recursive dependencies are still built by `ensure_artifact` before each direct dependency artifact is returned:

```rust
pub(crate) fn collect_dependency_artifacts(
    package: &Package,
    state: &mut ArtifactBuildState,
) -> Result<Vec<(String, PathBuf)>, String> {
    let Some(dependencies) = &package.manifest.dependencies else {
        return Ok(Vec::new());
    };

    let mut artifacts = BTreeMap::new();
    for (dep_name, dependency) in dependencies {
        let dep_root = resolve_dependency_root(&package.root_dir, dep_name, dependency)?;
        let dep_package = Package::load(dep_root.clone())?;
        let artifact_path = ensure_artifact(&dep_root, state)?;
        artifacts.insert(dep_package.manifest.crate_.name.clone(), artifact_path);
    }

    Ok(artifacts.into_iter().collect())
}
```

Replace the body of the stale-artifact branch in `ensure_artifact` with:

```rust
            if let Some(parent) = artifact_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create artifact directory {}: {}",
                        parent.display(),
                        e
                    )
                })?;
            }

            if let Some(parent) = package.object_path().parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "Failed to create object directory {}: {}",
                        parent.display(),
                        e
                    )
                })?;
            }

            let product_artifacts = dependency_artifacts
                .iter()
                .map(|(name, path)| ExternArtifact {
                    name: name.clone(),
                    path: path.clone(),
                })
                .collect::<Vec<_>>();
            let old_artifacts = implicit_artifact_inputs
                .iter()
                .map(|path| ExternArtifact {
                    name: STDLIB_CRATE_NAME.to_string(),
                    path: path.clone(),
                })
                .collect::<Vec<_>>();
            let invocation = build_dependency_artifact_invocation(
                resolve_rockc_path()?,
                &package,
                &product_artifacts,
                &old_artifacts,
            );
            run_invocation(
                invocation,
                &format!("dependency artifact for crate '{}'", package.manifest.crate_.name),
            )?;
```

Keep recursive `ensure_artifact` dependency ordering unchanged.

- [ ] **Step 5: Run dependency artifact tests**

Run: `cargo build -p rockc && cargo test -p rock test_rock_artifact_command_builds_cache_file -- --nocapture && cargo test -p rock test_build_project_with_transitive_artifacts -- --nocapture`

Expected: PASS for both tests. If this fails with a missing `rockc` error, verify `target/debug/rockc` exists before debugging compiler behavior.

- [ ] **Step 6: Run artifact cache tests**

Run: `cargo build -p rockc && cargo test -p rock artifact -- --nocapture`

Expected: PASS for artifact tests, except tests that intentionally rely on old artifact source-bundle fields should already have been updated in Step 2.

- [ ] **Step 7: Commit**

```bash
git add rock/src/artifact.rs rock/src/tests/artifact.rs rock/src/tests/build.rs rock/src/tests/support.rs
git commit -m "rock: build dependency product artifacts with rockc"
```

---

### Task 4: Build Root Executables Through `rockc`

**Files:**
- Modify: `rock/src/build.rs`
- Test: `rock/src/tests/build.rs`, `rock/src/tests/sysroot.rs`

- [ ] **Step 1: Add a root-build error test for missing `rockc` override**

In `rock/src/tests/build.rs`, add this test after `test_build_project_no_std_disables_implicit_stdlib`:

```rust
#[test]
fn test_build_project_reports_missing_rockc_override() {
    let _guard = sysroot_env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = temp_test_dir("missing_rockc_override");
    write_package(&temp_dir, "missing_rockc", "src/main.rk", &[], "main = -> 0\n");
    let previous = std::env::var_os("ROCKC");

    std::env::set_var("ROCKC", temp_dir.join("does-not-exist-rockc"));
    let error = build_project(&temp_dir).unwrap_err();

    match previous {
        Some(value) => std::env::set_var("ROCKC", value),
        None => std::env::remove_var("ROCKC"),
    }

    assert!(error.contains("Failed to spawn rockc"));
    assert!(error.contains("root executable for crate 'missing_rockc'"));

    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 2: Run the missing override test**

Run: `cargo test -p rock test_build_project_reports_missing_rockc_override -- --nocapture`

Expected: FAIL because `build_project` still calls `rock_lib::compile` in-process and ignores `ROCKC`.

- [ ] **Step 3: Replace root in-process compile with `rockc` subprocess**

In `rock/src/build.rs`, replace the imports with:

```rust
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    artifact::{collect_dependency_artifacts, ArtifactBuildState},
    bundled_sysroot::{
        ensure_sysroot_stdlib_available, package_requires_sysroot_stdlib, STDLIB_CRATE_NAME,
    },
    compile::output_executable_path,
    package::Package,
    rockc::{build_root_invocation, resolve_rockc_path, run_invocation, ExternArtifact},
};
```

Replace the `compiler_config` construction and `rock_lib::compile` call in `build_project` with:

```rust
    let product_artifacts = extern_artifacts
        .iter()
        .map(|(name, path)| ExternArtifact {
            name: name.clone(),
            path: path.clone(),
        })
        .collect::<Vec<_>>();
    let mut old_artifacts = Vec::new();
    if package_requires_sysroot_stdlib(&package) {
        let layout = ensure_sysroot_stdlib_available()?;
        old_artifacts.push(ExternArtifact {
            name: STDLIB_CRATE_NAME.to_string(),
            path: layout.stdlib_artifact,
        });
    }

    let invocation = build_root_invocation(
        resolve_rockc_path()?,
        &package,
        &product_artifacts,
        &old_artifacts,
    );
    run_invocation(
        invocation,
        &format!("root executable for crate '{}'", package.manifest.crate_.name),
    )?;
```

Also remove the earlier mutation that pushed stdlib into `extern_artifacts`; after this change, `extern_artifacts` should contain package dependency product artifacts only:

```rust
    let extern_artifacts = collect_dependency_artifacts(&package, &mut state)?;
```

- [ ] **Step 4: Run root build tests**

Run: `cargo build -p rockc && cargo test -p rock build -- --nocapture`

Expected: PASS for `rock/src/tests/build.rs` tests.

- [ ] **Step 5: Run sysroot tests**

Run: `cargo build -p rockc && cargo test -p rock sysroot -- --nocapture`

Expected: PASS. This proves root and dependency builds still pass stdlib as old-format `--extern-artifact` where needed.

- [ ] **Step 6: Commit**

```bash
git add rock/src/build.rs rock/src/tests/build.rs
git commit -m "rock: build root executables with rockc"
```

---

### Task 5: Clean Up Product Artifact Tests and Source-Free Behavior

**Files:**
- Modify: `rock/src/tests/artifact.rs`
- Modify: `rock/src/tests/build.rs`
- Modify: `rock/src/tests/support.rs`
- Test: `rock/src/tests/artifact.rs`, `rock/src/tests/build.rs`

- [ ] **Step 1: Remove stale old-artifact imports and helper usage**

In `rock/src/tests/artifact.rs`, remove any remaining `read_artifact` import from the support import list.

In `rock/src/tests/build.rs`, ensure no assertion calls `super::support::read_artifact`.

Keep `read_artifact` in `rock/src/tests/support.rs` because sysroot and old-artifact tests may still need old `CrateArtifact` support later.

- [ ] **Step 2: Add a product-deserialization assertion for source-free dependency reuse**

In `test_build_project_reuses_source_free_artifacts_without_dependency_sources`, after deleting the dependency source and rebuilding, add:

```rust
    let dep_products_after_source_delete = read_product_artifact(&dep_package.artifact_path());
    assert!(
        dep_products_after_source_delete
            .identity_table
            .export_names
            .contains_key("answer")
    );
```

This preserves the useful behavior that a prebuilt product artifact can satisfy downstream builds when dependency source files are not needed for compilation.

- [ ] **Step 3: Run all `rock` tests**

Run: `cargo build -p rockc && cargo test -p rock -- --nocapture`

Expected: PASS for all `rock` tests.

- [ ] **Step 4: Commit**

```bash
git add rock/src/tests/artifact.rs rock/src/tests/build.rs rock/src/tests/support.rs
git commit -m "rock: assert product artifact package behavior"
```

---

### Task 6: Final Verification

**Files:**
- No planned source edits.
- Test: workspace packages touched by this slice.

- [ ] **Step 1: Build dev-target `rockc`**

Run: `cargo build -p rockc`

Expected: PASS and create `target/debug/rockc`.

- [ ] **Step 2: Run `rock` tests**

Run: `cargo test -p rock -- --nocapture`

Expected: PASS for all `rock` tests.

- [ ] **Step 3: Run `rockc` tests**

Run: `cargo test -p rockc -- --nocapture`

Expected: PASS for all `rockc` tests.

- [ ] **Step 4: Run focused product/artifact regressions**

Run: `cargo test -p rock-lib products -- --nocapture`

Expected: PASS for product tests.

Run: `cargo test -p rock-lib crate_artifact -- --nocapture`

Expected: PASS for old artifact tests, proving old `--extern-artifact` support remains intact.

- [ ] **Step 5: Inspect working tree**

Run: `git status --short`

Expected: only the pre-existing untracked `docs/superpowers/plans/2026-05-07-rockc-product-artifact-emission.md` remains, unless the implementation intentionally changed additional files.

---

## Self-Review Notes

Spec coverage:
- Subprocess boundary: Task 1 creates `rock/src/rockc.rs` and command/run helpers.
- Dev-target resolver: Task 1 tests `target/<profile>/deps` and same-directory binary resolution.
- Dependency product artifacts: Task 3 invokes `rockc --emit-artifact --emit-object --no-link` and passes direct package deps through `--extern-product-artifact`.
- Root subprocess builds: Task 4 replaces `rock_lib::compile` in `build_project` with `rockc` invocation.
- Stdlib old artifact boundary: Task 1 command tests and Tasks 3-4 pass stdlib only through `--extern-artifact`.
- Freshness: Task 2 replaces old `CrateArtifact` reads with output/source/dependency/input mtime checks.
- Testing: Tasks 1-6 cover command construction, product artifact reads, cache behavior, root builds, sysroot behavior, and package-level verification.

Placeholder scan:
- The plan contains no `todo!()` implementation placeholders; the initial command builders return empty invocations only to create a concrete failing test.
- No task says to add unspecified validation or unlisted tests.

Type consistency:
- `ExternArtifact` is used consistently for both product and old artifact command inputs.
- `RockcInvocation` owns `executable` and `args` for unit testing, then converts into `std::process::Command` only at run time.
- `build_dependency_artifact_invocation` and `build_root_invocation` signatures match all task snippets.
