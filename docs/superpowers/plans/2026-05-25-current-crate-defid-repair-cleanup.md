# Current-Crate DefId Repair Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the lowerer-side extern `DefId(0, 0)` repair path so canonical current-crate declaration IDs are never reinterpreted as placeholders after collection.

**Architecture:** Collection already validates supported current-crate declaration IDs before lowering. This slice makes `Lowerer::from_declarations` trust collected extern IDs instead of using a hard-coded sentinel repair, and documents the remaining generated/sentinel identity follow-ups separately.

**Tech Stack:** Rust 2021, `rock-lib`, collect/lower ID pipeline, focused `cargo test -p rock-lib` unit tests, `cargo fmt --all --check`, `git diff --check`.

---

## Files

- Modify: `lib/src/lower/mod.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

---

### Task 1: Add a Regression Test for Real Extern ID Zero

**Files:**
- Modify: `lib/src/lower/mod.rs`

- [x] **Step 1: Write the failing test**

Add this test to the existing `#[cfg(test)] mod tests` in `lib/src/lower/mod.rs`, near the existing extern-ID tests:

```rust
    #[test]
    fn lowerer_from_declarations_preserves_real_local_extern_id_zero() {
        let extern_id = DefId::new(CrateId(0), LocalDefId(0));
        let collision_id = DefId::new(CrateId(0), LocalDefId(1));
        let mut resolver = ResolverTables::default();
        resolver
            .item_paths
            .insert("puts".to_string(), collision_id);

        let decls = Declarations {
            indexing_ids: IndexingIds::new_root(),
            item_index: ItemIndex::new(),
            resolver,
            current_def_ids: BTreeSet::from([extern_id]),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: Vec::new(),
            externs: vec![HirExtern {
                id: extern_id,
                name: "demo::puts".to_string(),
                params: vec![Type::Pointer(Box::new(Type::U8))],
                ret: Type::I32,
                variadic: false,
            }],
            functions: HashMap::new(),
            function_sigs: HashMap::new(),
            methods: HashMap::new(),
            function_type_vars: HashMap::new(),
            infix_precedence: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: Vec::new(),
            engine: InferenceEngine::new(),
            inject_prelude: false,
            stdlib_prelude_exports: HashMap::new(),
            stdlib_prelude_export_ids: HashMap::new(),
            module_file_cache: HashMap::new(),
            artifact_root_exports: HashMap::new(),
            artifact_root_export_ids: HashMap::new(),
            export_aliases: HashMap::new(),
            export_function_aliases: HashMap::new(),
        };

        let lowerer = Lowerer::from_declarations(decls);

        assert_eq!(lowerer.externs[0].id, extern_id);
    }
```

- [x] **Step 2: Run the focused test to verify it fails**

Run:

```bash
cargo test -p rock-lib lower::tests::lowerer_from_declarations_preserves_real_local_extern_id_zero -- --exact
```

Expected: FAIL because `resolve_extern_def_ids` rewrites `DefId(0, 0)` through the short-name resolver collision.

---

### Task 2: Remove the Extern Placeholder Repair Path

**Files:**
- Modify: `lib/src/lower/mod.rs`

- [x] **Step 1: Delete the repair call from `Lowerer::from_declarations`**

Remove these lines from `Lowerer::from_declarations`:

```rust
        let mut externs = externs;
        resolve_extern_def_ids(&mut externs, &resolver);
```

Then keep the destructured `externs` binding from `Declarations` unchanged so it is moved directly into the existing `Self` initializer field named `externs`.

- [x] **Step 2: Delete the unused repair helper**

Delete this function from `lib/src/lower/mod.rs`:

```rust
fn resolve_extern_def_ids(
    externs: &mut [HirExtern],
    resolver: &crate::collect::resolver::ResolverTables,
) {
    let temporary_id = crate::ids::DefId::new(CrateId(0), LocalDefId(0));

    for ext in externs {
        if ext.id != temporary_id {
            continue;
        }

        if let Some(def_id) = resolver.resolve_item_or_alias(&ext.name).or_else(|| {
            ext.name
                .rsplit("::")
                .next()
                .and_then(|short_name| resolver.resolve_item_or_alias(short_name))
        }) {
            ext.id = def_id;
        }
    }
}
```

- [x] **Step 3: Delete obsolete helper-unit tests**

Delete the complete `resolve_extern_def_ids_preserves_non_placeholder_dependency_ids` test from `lib/src/lower/mod.rs`, including its `#[test]` attribute and body.

Delete the complete `resolve_extern_def_ids_resolves_placeholder_local_ids` test from `lib/src/lower/mod.rs`, including its `#[test]` attribute and body.

- [x] **Step 4: Run the focused test to verify it passes**

Run:

```bash
cargo test -p rock-lib lower::tests::lowerer_from_declarations_preserves_real_local_extern_id_zero -- --exact
```

Expected: PASS.

- [x] **Step 5: Run related lowerer extern/alias tests**

Run:

```bash
cargo test -p rock-lib lower::tests::lowerer_from_declarations_extern_import_alias_lowers_to_resolved_extern -- --exact
```

Expected: PASS.

---

### Task 3: Update Roadmap and Checklist

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [x] **Step 1: Update Task 1 evidence in the master checklist**

Add one evidence bullet under `## 1. Identity And Arenas`:

```markdown
- `lib/src/lower/mod.rs` no longer repairs extern IDs by interpreting `DefId(0, 0)` as a placeholder; collected extern IDs are trusted as canonical lowering inputs.
```

- [x] **Step 2: Add one Done item in the master checklist**

Add this checkbox under `## 1. Identity And Arenas` `Done:`:

```markdown
- [x] Removed the lowerer-side extern `DefId(0, 0)` repair path so a real current-crate extern with local ID 0 cannot be rewritten through a short-name resolver collision.
```

- [x] **Step 3: Narrow the remaining generated/sentinel gap in the master checklist**

Change the existing remaining item:

```markdown
- [ ] Replace or explicitly model generated/sentinel IDs that still affect user-visible compilation paths, including auto-generated impls and legacy inference/selection fallback paths.
```

to:

```markdown
- [ ] Replace or explicitly model remaining generated/sentinel IDs that still affect user-visible compilation paths, including provisional generic owners, auto-generated impl provenance, and legacy inference fallback paths.
```

- [x] **Step 4: Update Task 1 status in the ordered roadmap**

In the Task 1 row in the `2026-05-25 Code-Verified Reconciliation` table, append this to the code evidence cell:

```markdown
; `Lowerer::from_declarations` trusts collected extern IDs instead of repairing `DefId(0, 0)` placeholders
```

In the remaining-work cell, replace:

```markdown
Generated/non-indexed paths still exist, including auto `Sized` impls, inference placeholder repairs, and legacy signature fallback IDs
```

with:

```markdown
Generated/non-indexed paths still exist, including provisional generic owners, auto `Sized` impl provenance, inference placeholder repairs, and legacy signature fallback IDs
```

---

### Task 4: Verify the Slice

**Files:**
- Test only

- [x] **Step 1: Run focused identity/lowerer tests**

Run:

```bash
cargo test -p rock-lib lower::tests::lowerer_from_declarations_preserves_real_local_extern_id_zero -- --exact
cargo test -p rock-lib lower::tests::lowerer_from_declarations_extern_import_alias_lowers_to_resolved_extern -- --exact
cargo test -p rock-lib collect::tests::generated_current_sized_impl_is_emitted_to_products -- --exact
```

Expected: all PASS.

- [x] **Step 2: Run formatting and diff hygiene**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: both commands PASS.
