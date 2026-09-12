# Rockc CLI Dependency Surface Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the last source-backed dependency escape hatches from `rockc` and `rock_lib::Config`, leaving `--extern-artifact` as the only dependency input on the compiler CLI.

**Architecture:** `rock` remains the source-to-artifact driver for dependencies, while `rockc` becomes a single-crate compiler frontend that only consumes prebuilt artifacts. The compiler config should no longer carry source dependency path plumbing (`crate_paths`), and the CLI should no longer expose `--extern-crate` or `--crate-path`.

**Tech Stack:** Rust 2021, `clap`, `rock-lib`, `cargo test -p rockc`, `cargo test -p rock-lib`.

---

## File Map

- Modify: `rockc/src/main.rs`
  Remove `--extern-crate` and `--crate-path`, keep `--extern-artifact`, and update parser tests.
- Modify: `lib/src/lib.rs`
  Remove `crate_paths` from `Config` and delete the source-crate preload loop in `compile`.
- Modify: `rock/src/build.rs`
  Stop populating `crate_paths` when constructing `rock_lib::Config`.
- Modify: `rock/src/compile.rs`
  Stop populating `crate_paths` when constructing `rock_lib::Config`.
- Modify: `rockup/src/dev.rs`
  Stop populating `crate_paths` when constructing `rock_lib::Config`.
- Modify: `lib/tests/integration.rs`
  Update the test config initializer to match the reduced `Config` shape.
- Modify: `lib/src/crate_artifact/tests.rs`
  Update local `Config` initializers used by artifact tests.
- Modify: `lib/src/lower/program.rs`
  Update local `Config` initializers used by lowering tests.
- Modify: `lib/src/lower/crates/bodies.rs`
  Update local `Config` initializers used by body-lowering tests.
- Modify: `lib/src/collect/context.rs`
  Update local `Config` initializers used by collection tests.

---

## Task 1: Trim `rockc` CLI flags

**Files:**
- Modify: `rockc/src/main.rs`

- [ ] **Step 1: Remove the dead dependency flags from the parser**

Delete the `extern_crate` and `crate_path` fields from `Config`, keep `extern_artifact`, and keep the `into_compiler_config` mapping only for `extern_artifacts`:

```rust
#[derive(Parser, Debug)]
#[command(version, about = "Rock compiler", long_about = None)]
pub struct Config {
    #[arg(long)]
    entry_file: Option<PathBuf>,
    #[arg(long, default_value = "build")]
    output_dir: PathBuf,
    #[arg(long, default_value = None)]
    debug_print: Option<String>,
    #[arg(long, value_enum)]
    print: Option<PrintValue>,
    #[arg(short = 'O', long, default_value = "0")]
    opt_level: u8,
    #[arg(long)]
    emit_llvm: bool,
    #[arg(long)]
    no_link: bool,
    #[arg(value_parser = parse_meta_files)]
    meta_files: Vec<(String, PathBuf)>,
    #[arg(long)]
    no_prelude: bool,
    #[arg(long)]
    no_std: bool,
    #[arg(long)]
    sysroot: Option<PathBuf>,
    #[arg(long)]
    extern_artifact: Vec<String>,
}
```

In `into_compiler_config`, map `extern_artifact` into `rock_lib::Config.extern_artifacts`. Leave the soon-to-be-removed config fields alone in this task only if the compiler still needs them to build; Task 2 deletes those fields and removes the leftover assignments.

- [ ] **Step 2: Add CLI parsing coverage for the new surface**

Extend `rockc/src/main.rs` tests with one parse test that accepts `--extern-artifact` and one rejection test that confirms removed flags no longer parse:

```rust
#[test]
fn test_extern_artifact_parses() {
    let config = Config::try_parse_from([
        "rockc",
        "--entry-file",
        "main.rk",
        "--extern-artifact",
        "dep=/tmp/dep.rkca",
    ])
    .unwrap();

    assert_eq!(config.extern_artifact, vec!["dep=/tmp/dep.rkca".to_string()]);
}

#[test]
fn test_removed_dependency_flags_are_rejected() {
    assert!(Config::try_parse_from([
        "rockc",
        "--entry-file",
        "main.rk",
        "--extern-crate",
        "dep=/tmp/dep.rk",
    ])
    .is_err());

    assert!(Config::try_parse_from([
        "rockc",
        "--entry-file",
        "main.rk",
        "--crate-path",
        "/tmp/dep",
    ])
    .is_err());
}
```

Run:
```bash
cargo test -p rockc test_extern_artifact_parses -- --exact
cargo test -p rockc test_removed_dependency_flags_are_rejected -- --exact
```

Expected: tests pass after the parser fields are removed.

---

## Task 2: Remove `crate_paths` from compiler config

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `rock/src/build.rs`
- Modify: `rock/src/compile.rs`
- Modify: `rockup/src/dev.rs`
- Modify: `lib/tests/integration.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Remove `crate_paths` from `rock_lib::Config` and `compile`**

Delete the `crate_paths` field from `Config` and delete this preload block from `compile`:

```rust
for crate_path in &config.crate_paths {
    if let Err(e) = ctx.load_crate_from_dir(crate_path.clone()) {
        let mut diagnostics = Diagnostics::default();
        diagnostics.push(diagnostic::Diagnostic::new(
            format!("Failed to load crate from {}: {}", crate_path.display(), e),
            Span::default(),
        ));
        return Err(diagnostics);
    }
}
```

- [ ] **Step 2: Remove every `crate_paths: vec![]` initializer at compile sites**

Update the config literals in `rock/src/build.rs`, `rock/src/compile.rs`, and `rockup/src/dev.rs` to stop setting `crate_paths`.

- [ ] **Step 3: Update library tests and internal test fixtures**

Remove `crate_paths: vec![]` from the `Config` literals in `lib/tests/integration.rs`, `lib/src/crate_artifact/tests.rs`, `lib/src/lower/program.rs`, `lib/src/lower/crates/bodies.rs`, and `lib/src/collect/context.rs`.

Run: `cargo test -p rock-lib`

Expected: the crate compiles and the existing test suite passes with the smaller config shape.

---

## Task 3: Verify the new dependency surface

**Files:**
- Modify: none, verification only

- [ ] **Step 1: Run the compiler and CLI test slices that exercise the new surface**

Run:
```bash
cargo test -p rockc
cargo test -p rock-lib crate_artifact::tests::test_compile_with_interface_only_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_with_stdlib_artifact -- --exact
```

Expected: all pass, and no code path still depends on `--extern-crate` or `crate_paths`.

---

## Task 4: Remove `extern_crates` from compiler config

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `rockc/src/main.rs`
- Modify: `rock/src/build.rs`
- Modify: `rock/src/compile.rs`
- Modify: `rockup/src/dev.rs`
- Modify: `lib/tests/integration.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Verify the remaining source-backed config escape hatch with a red compile check**

Remove one `extern_crates: vec![]` initializer from a test-local `rock_lib::Config` literal, such as the first fixture in `lib/src/crate_artifact/tests.rs`.

Run:
```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_with_interface_only_artifact -- --exact
```

Expected: FAIL to compile with a missing `extern_crates` field, proving the public config still requires the source-backed dependency slot.

- [ ] **Step 2: Remove source-backed external loading from `rock_lib::Config` and `compile`**

Delete the `extern_crates` field from `rock_lib::Config`, delete the `for (name, path) in &config.extern_crates` loop in `compile`, and update `has_explicit_stdlib_override` to inspect only `config.extern_artifacts`:

```rust
fn has_explicit_stdlib_override(config: &Config) -> bool {
    config
        .extern_artifacts
        .iter()
        .any(|(name, _)| name == sysroot::STDLIB_CRATE_NAME)
}
```

- [ ] **Step 3: Remove every now-invalid `extern_crates` initializer**

Remove `extern_crates: vec![]` from config literals in `rockc/src/main.rs`, `rock/src/build.rs`, `rock/src/compile.rs`, `rockup/src/dev.rs`, `lib/src/crate_artifact/tests.rs`, `lib/src/lower/program.rs`, `lib/src/lower/crates/bodies.rs`, and `lib/src/collect/context.rs`.

- [ ] **Step 4: Rewrite integration test stdlib input to artifact-backed loading**

In `lib/tests/integration.rs`, stop constructing `Config` with `extern_crates: vec![("stdlib", stdlib_path())]`. Build or locate a stdlib artifact for the test run and pass it through `extern_artifacts: vec![("stdlib", artifact_path)]` so broad integration coverage no longer masks source-backed dependency loading.

- [ ] **Step 5: Verify the reduced config surface**

Run:
```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_with_interface_only_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_with_stdlib_artifact -- --exact
cargo test -p rock-lib
cargo test -p rockc
```

Expected: all pass, and searching Rust sources for `extern_crates` finds no matches.

---

## Task 5: Remove implicit sysroot stdlib loading from `compile`

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add a red test for explicit stdlib artifacts**

Change `lib/src/crate_artifact/tests.rs::test_compile_with_sysroot_stdlib` so it expects a stdlib-using compile with `sysroot: Some(...)` but no `extern_artifacts` to fail. Rename it to `test_compile_requires_explicit_stdlib_artifact_even_with_sysroot`.

Run:
```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_requires_explicit_stdlib_artifact_even_with_sysroot -- --exact
```

Expected before implementation: FAIL because `compile` still implicitly loads the sysroot stdlib artifact.

- [ ] **Step 2: Remove implicit stdlib loading from `compile`**

Delete the `load_bundled_stdlib(&mut ctx, config)?;` call from `compile`, and delete the now-unused `load_bundled_stdlib` and `has_explicit_stdlib_override` helpers from `lib/src/lib.rs`. Keep artifact loading through `config.extern_artifacts` unchanged.

- [ ] **Step 3: Keep only explicit stdlib artifact tests**

Update sysroot-related tests so they no longer assert implicit loading. Keep coverage that explicit `extern_artifacts: [("stdlib", path)]` works even when `sysroot` is set, because `sysroot` may still be useful for CLI/toolchain paths and artifact root tests.

- [ ] **Step 4: Update local agent docs**

Replace removed `--extern-crate stdlib=stdlib` examples in `AGENTS.md` and `CLAUDE.md` with explicit `--extern-artifact stdlib=build/stdlib.rkca` examples.

- [ ] **Step 5: Verify**

Run:
```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_requires_explicit_stdlib_artifact_even_with_sysroot -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_with_stdlib_artifact -- --exact
cargo test -p rock-lib
cargo test -p rockc
```

Expected: all pass, and direct compile no longer loads stdlib unless it is listed in `extern_artifacts`.

---

## Task 6: Make `rock` pass bundled stdlib explicitly

**Files:**
- Modify: `rock/src/build.rs`
- Modify: `rock/src/compile.rs`

- [ ] **Step 1: Reproduce driver regression**

Run:
```bash
cargo test -p rock test_build_project_uses_sysroot_stdlib -- --nocapture
```

Expected before implementation: FAIL because `rock` ensures the sysroot stdlib artifact exists but does not pass it through `rock_lib::Config.extern_artifacts` after implicit compiler loading is removed.

- [ ] **Step 2: Pass stdlib artifact for root package builds**

In `rock/src/build.rs`, when `package_requires_sysroot_stdlib(&package)` is true, call `ensure_sysroot_stdlib_available()` and append `("stdlib", layout.stdlib_artifact)` to the root package `extern_artifacts` passed to `rock_lib::Config`.

- [ ] **Step 3: Pass stdlib artifact for dependency object builds**

In `rock/src/compile.rs::compile_package_object`, when `package_requires_sysroot_stdlib(package)` is true, call `ensure_sysroot_stdlib_available()` and append `("stdlib", layout.stdlib_artifact)` to the object compile `extern_artifacts` passed to `rock_lib::Config`.

- [ ] **Step 4: Verify driver behavior**

Run:
```bash
cargo test -p rock test_build_project_uses_sysroot_stdlib -- --nocapture
cargo test -p rock
```

Expected: sysroot stdlib build tests pass with explicit artifact plumbing through `rock`.
