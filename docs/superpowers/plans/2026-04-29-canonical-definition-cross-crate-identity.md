# Canonical Definition And Cross-Crate Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make canonical `DefId`/`ModuleId` identity authoritative for the current crate, then immediately extend the same identity model across dependency crates, stdlib/prelude, and artifact-backed paths.

**Architecture:** `collect` already builds canonical current-crate path tables; this slice makes those tables usable as authoritative identity by adding reverse lookups such as `module_names: HashMap<ModuleId, String>` and `item_names: HashMap<DefId, String>` and threading the resolver tables into `Lowerer`. The follow-up phase extends the same canonical data model to loaded crates and artifact-backed modules so `lower`, `collect`, and artifact loading stop depending on separate string-only identity paths.

**Tech Stack:** Rust 2021, `rock-lib`, `collect`, `lower`, `crate_artifact`, focused unit/integration tests, `cargo fmt --all`, `cargo test -p rock-lib`

---

### Task 1: Add authoritative canonical definition lookups for the current crate

**Files:**
- Modify: `lib/src/collect/resolver.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Add the failing canonical-definition test**

Add a collect test that proves the resolver tables can answer both directions for current-crate identity:

- `module_paths["math"] == ModuleId(1)`
- `module_paths["test::io"] == ModuleId(2)`
- `item_paths["RootThing"] == DefId::new(CrateId(0), LocalDefId(0))`
- `item_paths["math::Vector"] == DefId::new(CrateId(0), LocalDefId(1))`
- `item_paths["test::io::Writer"] == DefId::new(CrateId(0), LocalDefId(2))`
- reverse lookup by `DefId` returns the canonical path string for the same item

Add a small test helper in `lib/src/collect/mod.rs` named `current_crate_canonical_fixture()` that builds the same root/inline/source-backed `Program` used in the existing current-crate collect tests. Reuse that helper in the collect and lower tests below so the fixture stays identical across steps.

```rust
#[test]
fn collect_builds_reverse_canonical_tables_for_current_crate_items() {
    let program = current_crate_canonical_fixture();
    let decls = collect(&program, &CrateContext::new(), false, Some("test")).unwrap();

    assert_eq!(decls.resolver.module_paths.get("math"), Some(&ModuleId(1)));
    assert_eq!(decls.resolver.module_paths.get("test::io"), Some(&ModuleId(2)));

    let writer_id = DefId::new(CrateId(0), LocalDefId(2));
    assert_eq!(
        decls.resolver.item_paths.get("test::io::Writer"),
        Some(&writer_id)
    );
    assert_eq!(
        decls.resolver.item_names.get(&writer_id).map(String::as_str),
        Some("test::io::Writer")
    );
}
```

- [ ] **Step 2: Run the test and confirm the red state**

Run: `cargo test -p rock-lib collect::tests::collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact`

Expected: FAIL until the reverse canonical lookup tables are added.

- [ ] **Step 3: Implement the reverse canonical lookup tables**

Add reverse-name maps to `ResolverTables` so lower can recover canonical names from `DefId`/`ModuleId` without string fallback. Build them in `build_resolver_tables(...)` from the same current-crate path walk that already populates `module_paths` and `item_paths`.

- [ ] **Step 4: Re-run the test and confirm it passes**

Run: `cargo test -p rock-lib collect::tests::collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the collect-side identity tables**

```bash
git add lib/src/collect/context.rs lib/src/collect/mod.rs lib/src/collect/resolver.rs
git commit -m "add reverse canonical identity tables"
```

### Task 2: Make lower consume canonical current-crate identity with no string fallback

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/collect/declarations.rs`
- Modify: `lib/src/lower/crates/registration.rs`

- [ ] **Step 1: Add the failing lower regression**

Add a lower-side test that clears the legacy string alias map but leaves the canonical resolver tables populated, then verifies current-crate imported names still lower correctly. The test should prove the new path does not depend on `import_aliases: HashMap<String, String>`.

```rust
#[test]
fn lower_uses_canonical_import_aliases_without_string_fallback() {
    let program = current_crate_canonical_fixture();
    let mut decls = collect(&program, &CrateContext::new(), false, Some("test")).unwrap();
    decls.import_aliases.clear();

    let lowerer = Lowerer::from_declarations(decls);
    let hir = lowerer.lower_program(&program).unwrap();

    assert!(hir.functions.contains_key("test::io::writer_name"));
    assert!(hir.functions.contains_key("writer_name"));
}
```

- [ ] **Step 2: Run the lower regression and confirm the red state**

Run: `cargo test -p rock-lib lower::paths::tests::lower_uses_canonical_import_aliases_without_string_fallback -- --exact`

Expected: FAIL until `Lowerer` is wired to canonical resolver identity.

- [ ] **Step 3: Thread `ResolverTables` through `Lowerer`**

Replace the current `resolver: _` discard in `Lowerer::from_declarations(...)` with the real resolver tables. Build any derived reverse maps once there, then use them in `lower/paths.rs` and `lower/program.rs` for current-crate name recovery.

- [ ] **Step 4: Remove the canonical-path fallback branch for migrated names**

Update the identifier and import lookup paths so migrated current-crate names resolve directly through canonical IDs and reverse canonical maps. Keep legacy string maps only for name domains that are still intentionally unmigrated in this branch.

- [ ] **Step 5: Re-run the lower regression and confirm it passes**

Run: `cargo test -p rock-lib lower::paths::tests::lower_uses_canonical_import_aliases_without_string_fallback -- --exact`

Expected: PASS.

- [ ] **Step 6: Commit the lower migration**

```bash
git add lib/src/lower/mod.rs lib/src/lower/paths.rs lib/src/lower/program.rs lib/src/lower/collect/declarations.rs lib/src/lower/crates/registration.rs
git commit -m "use canonical identity in lower"
```

### Task 3: Extend canonical identity to dependency crates, stdlib/prelude, and artifact-backed paths immediately

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/resolver.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/crate_artifact/build.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/lower/crates/registration.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/crate_system/mod.rs` if extra identity data has to be carried on loaded crates

- [ ] **Step 1: Add the failing cross-crate identity tests**

Add three focused tests, each with an exact identity assertion:

- `collect_records_dependency_crate_identity_canonically`
  - assert `decls.declarations.resolver.item_paths` contains a dependency-crate function path such as `stdlib::println`
  - assert the reverse lookup map returns the same canonical path for that `DefId`
- `collect_records_stdlib_prelude_identity_canonically`
  - assert `decls.declarations.resolver.import_aliases` contains a prelude alias such as `print`
  - assert the alias resolves to the canonical `DefId` for `stdlib::print`
- `collect_records_artifact_module_identity_canonically`
  - assert `decls.declarations.resolver.module_paths` contains a loaded artifact module path such as `stdlib::string`
  - assert the reverse module-name map returns the same canonical path for the corresponding `ModuleId`

- [ ] **Step 2: Run the cross-crate tests and confirm the red state**

Run:

- `cargo test -p rock-lib collect::tests::collect_records_dependency_crate_identity_canonically -- --exact`
- `cargo test -p rock-lib collect::tests::collect_records_stdlib_prelude_identity_canonically -- --exact`
- `cargo test -p rock-lib collect::tests::collect_records_artifact_module_identity_canonically -- --exact`

Expected: FAIL until cross-crate identity is wired into collect and lower.

- [ ] **Step 3: Populate canonical identity for loaded crates and artifact summaries**

Extend collect-side crate registration so loaded dependency crates, stdlib prelude exports, and artifact-backed module summaries all contribute canonical module/item names into the same resolver model.

- [ ] **Step 4: Consume cross-crate canonical identity in lower and artifact loading**

Update crate registration, prelude injection, and artifact module lookup so the same canonical tables cover dependency crates, stdlib/prelude, and artifact-backed module paths without a string-path fallback branch.

- [ ] **Step 5: Re-run the cross-crate tests and confirm they pass**

Run the focused dependency/stdlib/artifact tests again.

Expected: PASS.

- [ ] **Step 6: Commit the cross-crate identity expansion**

```bash
git add lib/src/collect/context.rs lib/src/collect/resolver.rs lib/src/collect/mod.rs lib/src/crate_artifact/build.rs lib/src/crate_artifact/load.rs lib/src/lower/crates/registration.rs lib/src/lower/program.rs lib/src/crate_system/mod.rs
git commit -m "extend canonical identity across crates"
```

### Task 4: Regression verification

**Files:**
- Verify only unless a test failure forces a fix.

- [ ] **Step 1: Run the focused regressions**

Run:

- `cargo test -p rock-lib collect::tests::collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact`
- `cargo test -p rock-lib lower::paths::tests::lower_uses_canonical_import_aliases_without_string_fallback -- --exact`
- the new dependency/stdlib/artifact canonical identity tests from Task 3

Expected: PASS.

- [ ] **Step 2: Run formatting and the full package suite**

Run:

- `cargo fmt --all`
- `cargo test -p rock-lib`

If Cargo reports stale or incremental linker artifacts, run `cargo clean -p rock-lib` and retry the same failing command.

- [ ] **Step 3: Inspect final status**

Run: `git status --short`

Expected: only the intended tracked changes for this slice, plus any unrelated pre-existing untracked files.
