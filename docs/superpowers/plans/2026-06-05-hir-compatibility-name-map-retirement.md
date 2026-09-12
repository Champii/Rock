# HIR Compatibility Name Map Retirement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retire remaining direct semantic reads from string-keyed HIR ownership maps in migrated mono, MIR, DCE, and codegen consumers while preserving explicit display/diagnostic compatibility paths.

**Architecture:** Keep `HirProgram` ID-owned storage and derived indexes as the semantic authority. Add small display/alias helper APIs on `HirProgram`, then migrate downstream consumers away from inline `program.names.*_by_name` iteration or lookup. Remaining name-table reads should live behind helpers whose names state display/compatibility intent, or in source-name/unresolved/test setup paths.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId` indexes, monomorphization, MIR builder metadata, instance-reachability DCE, LLVM codegen metadata, focused Cargo tests, `cargo fmt --all --check`, `git diff --check`, `cargo test -p rock-lib`.

---

## Approved Spec

- `docs/superpowers/specs/2026-06-05-hir-compatibility-name-map-retirement-design.md`

## Execution Constraints

- Use `superpowers:using-git-worktrees` at execution time before source edits unless the active workspace is already isolated.
- Do not inspect or modify `.sisyphus/`.
- Do not stage, commit, amend, push, or otherwise mutate VCS state unless the current user prompt explicitly asks for it. Ignore the commit step examples in older plans for this repository.
- Preserve compatibility name tables in this slice; do not delete `HirNameTables` fields.
- Keep source/display names in diagnostics, artifact compatibility, alias metadata, and unresolved-name recovery paths.

## File Structure

- Modify `lib/src/hir/mod.rs`: add display/alias helper APIs by canonical `DefId`, helper-level tests, and documentation comments for the compatibility boundary.
- Modify `lib/src/mir/builder/mod.rs`: use HIR helper APIs for backend metadata aliases, projection impl owner aliases, and receiver method lookup names.
- Modify `lib/src/dce.rs`: use HIR helper APIs for receiver method lookup names in instance reachability.
- Modify `lib/src/codegen/mod.rs`: use HIR helper APIs for nominal layout alias metadata registration.
- Modify `lib/src/mono/process.rs`: use HIR helper APIs for monomorphizer function-alias registration.
- Modify `lib/src/mono/external.rs` only if the same helper-driven alias registration needs external-process coverage.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: update Identity/Arenas and Monomorphization Instances notes after verification.
- Modify `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`: update the rebaseline/reconciliation text after verification without claiming all string metadata is gone.

## Task 1: Add Explicit HIR Display/Alias Helpers

**Files:**
- Modify: `lib/src/hir/mod.rs`

- [ ] **Step 1: Add failing helper tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `lib/src/hir/mod.rs`, near the existing `hir_program_owns_functions_by_def_id_and_names_are_views` test:

```rust
#[test]
fn hir_program_lists_display_aliases_by_def_id() {
    let function_id = def_id(40);
    let struct_id = def_id(41);
    let enum_id = def_id(42);
    let canonical_names = HashMap::from([
        (function_id, "main".to_string()),
        (struct_id, "module::Foo".to_string()),
        (enum_id, "module::Choice".to_string()),
    ]);

    let program = HirProgram::from_parts_with_canonical_names(
        HashMap::from([
            ("main".to_string(), test_function(function_id, "main")),
            ("alias_main".to_string(), test_function(function_id, "main")),
        ]),
        HashMap::from([
            ("module::Foo".to_string(), empty_struct(struct_id, "Foo")),
            ("AliasFoo".to_string(), empty_struct(struct_id, "Foo")),
        ]),
        HashMap::from([
            ("module::Choice".to_string(), empty_enum(enum_id, "Choice")),
            ("AliasChoice".to_string(), empty_enum(enum_id, "Choice")),
        ]),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
        &canonical_names,
    );

    assert_eq!(
        program.function_display_aliases(function_id),
        vec!["main", "alias_main"]
    );
    assert_eq!(
        program.struct_display_aliases(struct_id),
        vec!["module::Foo", "AliasFoo"]
    );
    assert_eq!(
        program.enum_display_aliases(enum_id),
        vec!["module::Choice", "AliasChoice"]
    );
    assert_eq!(
        program.nominal_display_aliases(struct_id),
        vec!["module::Foo", "AliasFoo"]
    );
    assert_eq!(
        program.nominal_display_aliases(enum_id),
        vec!["module::Choice", "AliasChoice"]
    );
}

#[test]
fn hir_program_resolves_nominal_display_aliases_only_to_existing_owners() {
    let struct_id = def_id(43);
    let enum_id = def_id(44);
    let stale_id = def_id(99);
    let mut program = HirProgram::from_parts(
        HashMap::new(),
        HashMap::from([("Foo".to_string(), empty_struct(struct_id, "Foo"))]),
        HashMap::from([("Choice".to_string(), empty_enum(enum_id, "Choice"))]),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    program
        .names
        .structs_by_name
        .insert("AliasFoo".to_string(), struct_id);
    program
        .names
        .enums_by_name
        .insert("AliasChoice".to_string(), enum_id);
    program
        .names
        .structs_by_name
        .insert("Stale".to_string(), stale_id);

    assert_eq!(
        program.nominal_owner_id_for_display_alias("AliasFoo"),
        Some(struct_id)
    );
    assert_eq!(
        program.nominal_owner_id_for_display_alias("AliasChoice"),
        Some(enum_id)
    );
    assert_eq!(program.nominal_owner_id_for_display_alias("Stale"), None);
    assert!(program.nominal_display_aliases(stale_id).is_empty());
}
```

- [ ] **Step 2: Run the red helper tests**

Run:

```bash
cargo test -p rock-lib hir_program_lists_display_aliases_by_def_id -- --exact --nocapture
cargo test -p rock-lib hir_program_resolves_nominal_display_aliases_only_to_existing_owners -- --exact --nocapture
```

Expected: both fail to compile because `function_display_aliases`, `struct_display_aliases`, `enum_display_aliases`, `nominal_display_aliases`, and `nominal_owner_id_for_display_alias` do not exist.

- [ ] **Step 3: Add helper APIs on `HirProgram`**

In the `impl HirProgram` block in `lib/src/hir/mod.rs`, after the existing ID-keyed iterator methods, add:

```rust
    /// Display/compatibility aliases for a canonical function owner.
    ///
    /// Semantic consumers should use `function_by_id` when they already have a
    /// `DefId`; this helper is only for display, artifact compatibility, and
    /// alias metadata derived from a known owner.
    pub fn function_display_aliases(&self, id: DefId) -> Vec<&str> {
        display_aliases_for_id(
            &self.names.functions_by_name,
            id,
            self.function_by_id(id).map(|(name, _)| name),
        )
    }

    /// Display/compatibility aliases for a canonical struct owner.
    pub fn struct_display_aliases(&self, id: DefId) -> Vec<&str> {
        display_aliases_for_id(
            &self.names.structs_by_name,
            id,
            self.struct_by_id(id).map(|(name, _)| name),
        )
    }

    /// Display/compatibility aliases for a canonical enum owner.
    pub fn enum_display_aliases(&self, id: DefId) -> Vec<&str> {
        display_aliases_for_id(
            &self.names.enums_by_name,
            id,
            self.enum_by_id(id).map(|(name, _)| name),
        )
    }

    /// Display/compatibility aliases for a canonical nominal owner.
    pub fn nominal_display_aliases(&self, id: DefId) -> Vec<&str> {
        if self.structs.contains_key(&id) {
            self.struct_display_aliases(id)
        } else if self.enums.contains_key(&id) {
            self.enum_display_aliases(id)
        } else {
            Vec::new()
        }
    }

    /// Resolve a display alias to an existing nominal owner.
    ///
    /// This is a compatibility helper for HIR metadata that still stores a
    /// display owner string. Prefer `Type::Struct { id, .. }`,
    /// `Type::Enum { id, .. }`, or explicit owner sidecars when available.
    pub fn nominal_owner_id_for_display_alias(&self, name: &str) -> Option<DefId> {
        self.names
            .structs_by_name
            .get(name)
            .copied()
            .filter(|id| self.structs.contains_key(id))
            .or_else(|| {
                self.names
                    .enums_by_name
                    .get(name)
                    .copied()
                    .filter(|id| self.enums.contains_key(id))
            })
    }
```

Add this private helper below the `impl HirProgram` block and above `hir_function_is_codegen_concrete`:

```rust
fn display_aliases_for_id<'a>(
    names_by_name: &'a HashMap<String, DefId>,
    owner_id: DefId,
    primary_name: Option<&'a str>,
) -> Vec<&'a str> {
    let mut names = primary_name.into_iter().collect::<Vec<_>>();
    let mut aliases = names_by_name
        .iter()
        .filter_map(|(name, id)| (*id == owner_id).then_some(name.as_str()))
        .collect::<Vec<_>>();
    aliases.sort();

    for alias in aliases {
        if !names.contains(&alias) {
            names.push(alias);
        }
    }

    names
}
```

- [ ] **Step 4: Verify helper tests**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib hir_program_lists_display_aliases_by_def_id -- --exact --nocapture
cargo test -p rock-lib hir_program_resolves_nominal_display_aliases_only_to_existing_owners -- --exact --nocapture
```

Expected: all commands pass.

## Task 2: Migrate MIR Builder Name-Map Consumers

**Files:**
- Modify: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add MIR alias metadata regression**

Add this test inside `#[cfg(test)] mod tests` in `lib/src/mir/builder/mod.rs`, near `build_monomorphized_backend_metadata_type_ids_use_final_mir_context`:

```rust
#[test]
fn backend_metadata_records_nominal_aliases_from_hir_display_helpers() {
    let struct_id = DefId::new(CrateId(0), LocalDefId(0));
    let mut program = program_with_struct(vec!["value"]);
    program
        .names
        .structs_by_name
        .insert("AliasFoo".to_string(), struct_id);
    program.rebuild_indexes();

    let mut type_context = crate::type_context::TypeContext::new();
    let _ = crate::hir::collect_hir_type_ids(&program, &mut type_context);
    let mono = crate::mono::MonomorphizedProgram {
        program,
        instances: std::collections::BTreeMap::new(),
        type_context,
    };
    let functions = std::collections::BTreeMap::new();
    let mut metadata_type_context = crate::type_context::TypeContext::new();

    let metadata =
        MirBuilder::backend_metadata_for_program(&mono, &functions, &mut metadata_type_context);

    assert!(metadata
        .structs
        .iter()
        .any(|layout| layout.id == struct_id && layout.name == "Foo" && layout.canonical));
    assert!(metadata
        .structs
        .iter()
        .any(|layout| layout.id == struct_id && layout.name == "AliasFoo" && !layout.canonical));
}
```

- [ ] **Step 2: Run current MIR direct-name-map audit**

Run:

```bash
rg "program(\.program)?\.names\.(structs|enums|functions|traits)_by_name" lib/src/mir/builder/mod.rs
```

Expected before the migration: matches remain in backend metadata, projection impl metadata, callable lookup, and method lookup alias code. Keep the output for comparison after Step 4.

- [ ] **Step 3: Replace backend layout alias loops**

In `MirBuilder::backend_metadata_for_program`, replace the `for (name, _id) in &program.program.names.structs_by_name` loop with helper-driven alias enumeration:

```rust
        for (id, canonical_name, s) in program.program.structs_by_id() {
            for alias in program.program.struct_display_aliases(id) {
                if alias == canonical_name {
                    continue;
                }
                structs.push(super::MirStructLayout {
                    id,
                    name: alias.to_string(),
                    generic_params: s.generic_params.clone(),
                    fields: s
                        .fields
                        .iter()
                        .map(|field| (field.name.clone(), type_context.intern_type(&field.ty)))
                        .collect(),
                    canonical: false,
                });
            }
        }
```

Replace the enum alias loop the same way:

```rust
        for (id, canonical_name, e) in program.program.enums_by_id() {
            for alias in program.program.enum_display_aliases(id) {
                if alias == canonical_name {
                    continue;
                }
                enums.push(super::MirEnumLayout {
                    id,
                    name: alias.to_string(),
                    generic_params: e.generic_params.clone(),
                    variants: Self::mir_enum_variants(&mut *type_context, &e.variants),
                    canonical: false,
                });
            }
        }
```

Do not remove the existing canonical `structs_by_id()` and `enums_by_id()` loops.

- [ ] **Step 4: Replace projection and method lookup name-map reads**

In `mir_projection_impl_metadata`, replace the direct struct/enum map lookup with the explicit helper:

```rust
            owner_id: match &imp.owner {
                HirImplOwner::Named(owner) => program.nominal_owner_id_for_display_alias(owner),
                HirImplOwner::BuiltinSlice => None,
            },
```

In `type_names_for_method_lookup`, replace the struct and enum branches with helper-driven aliases:

```rust
            Type::Struct { id, .. } => {
                let mut names = self
                    .program
                    .struct_display_aliases(*id)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if names.is_empty() {
                    names.push(recv_ty.to_string());
                }
                names
            }
            Type::Enum { id, .. } => {
                let mut names = self
                    .program
                    .enum_display_aliases(*id)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if names.is_empty() {
                    names.push(recv_ty.to_string());
                }
                names
            }
```

Then delete the now-unused private `nominal_type_names_for_method` helper from `MirBuilder`.

Leave `callable_for_named_value` on its existing source-name compatibility path in this task. If the final audit still reports it, classify it in a comment as unresolved-name/local compatibility rather than converting it to ID lookup without a caller-side target.

- [ ] **Step 5: Verify MIR migration**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib backend_metadata_records_nominal_aliases_from_hir_display_helpers -- --exact --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
rg "program(\.program)?\.names\.(structs|enums)_by_name" lib/src/mir/builder/mod.rs
```

Expected: tests pass. The final `rg` should have no `structs_by_name` or `enums_by_name` matches in production code. Test setup matches under `#[cfg(test)]` are allowed.

## Task 3: Migrate DCE Method Lookup Aliases

**Files:**
- Modify: `lib/src/dce.rs`

- [ ] **Step 1: Add DCE alias helper regression**

In the `#[cfg(test)] mod tests` in `lib/src/dce.rs`, update the imports to include `HirField`:

```rust
    use crate::hir::{
        HirBlock, HirCallTarget, HirExpr, HirExprKind, HirField, HirFunction, HirImpl,
        HirImplOwner, HirMethodCallTarget, HirProgram, HirStmt, HirVarRef, HirVarTarget,
    };
```

Add this test near the other reachability alias tests:

```rust
#[test]
fn dce_method_lookup_uses_nominal_display_aliases_by_owner_id() {
    let struct_id = DefId::new(CrateId(0), LocalDefId(210));
    let mut program = HirProgram::from_parts(
        HashMap::new(),
        HashMap::from([(
            "module::Foo".to_string(),
            crate::hir::HirStruct {
                id: struct_id,
                name: "Foo".to_string(),
                generic_params: Vec::new(),
                fields: vec![HirField {
                    id: crate::ids::FieldId(0),
                    name: "value".to_string(),
                    ty: Type::I64,
                    public: true,
                }],
            },
        )]),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    program
        .names
        .structs_by_name
        .insert("AliasFoo".to_string(), struct_id);
    program.rebuild_indexes_with_canonical_names(&HashMap::from([(
        struct_id,
        "module::Foo".to_string(),
    )]));

    let names = super::type_names_for_method_lookup(
        &program,
        &Type::Struct {
            id: struct_id,
            args: Vec::new(),
        },
    );

    assert_eq!(names, vec!["module::Foo".to_string(), "AliasFoo".to_string()]);
}
```

- [ ] **Step 2: Run current DCE direct-name-map audit**

Run:

```bash
rg "program\.names\.(structs|enums)_by_name|owners_by_name" lib/src/dce.rs
```

Expected before migration: matches remain in `type_names_for_method_lookup` and `nominal_type_names_for_method`.

- [ ] **Step 3: Replace DCE alias lookup with HIR helpers**

In `type_names_for_method_lookup`, replace the struct and enum branches with:

```rust
        Type::Struct { id, .. } => {
            let mut names = program
                .struct_display_aliases(*id)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            if names.is_empty() {
                names.push(recv_ty.to_string());
            }
            names
        }
        Type::Enum { id, .. } => {
            let mut names = program
                .enum_display_aliases(*id)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            if names.is_empty() {
                names.push(recv_ty.to_string());
            }
            names
        }
```

Delete the now-unused private `nominal_type_names_for_method` function from `lib/src/dce.rs`.

- [ ] **Step 4: Verify DCE migration**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib dce_method_lookup_uses_nominal_display_aliases_by_owner_id -- --exact --nocapture
cargo test -p rock-lib dce -- --nocapture
rg "program\.names\.(structs|enums)_by_name|owners_by_name" lib/src/dce.rs
```

Expected: tests pass. The final `rg` should have no production direct nominal name-map reads in `lib/src/dce.rs`; test setup writes are allowed.

## Task 4: Migrate Codegen And Mono Display Compatibility Consumers

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs` only if focused tests show external processing still needs a local adjustment

- [ ] **Step 1: Add codegen alias metadata regression**

In the `#[cfg(test)] mod tests` in `lib/src/codegen/mod.rs`, add this test near the existing nominal layout tests:

```rust
#[test]
fn register_nominal_layouts_records_display_aliases_by_owner_id() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "test");
    let struct_id = DefId::new(CrateId(0), LocalDefId(310));
    let mut program = HirProgram::from_parts(
        HashMap::new(),
        HashMap::from([(
            "module::Point".to_string(),
            HirStruct {
                id: struct_id,
                name: "Point".to_string(),
                generic_params: Vec::new(),
                fields: vec![HirField {
                    id: FieldId(0),
                    name: "x".to_string(),
                    ty: Type::I64,
                    public: true,
                }],
            },
        )]),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    program
        .names
        .structs_by_name
        .insert("PointAlias".to_string(), struct_id);
    program.rebuild_indexes_with_canonical_names(&HashMap::from([(
        struct_id,
        "module::Point".to_string(),
    )]));

    codegen.register_nominal_layouts(&program);

    assert!(codegen.struct_info.contains_key("module::Point"));
    assert!(codegen.struct_info.contains_key("PointAlias"));
    assert_eq!(codegen.struct_generic_owners["PointAlias"], struct_id);
}
```

Use already imported names if this test module has them; otherwise add the minimal missing imports from `crate::hir`, `crate::ids`, `crate::types`, and `std::collections::HashMap`.

- [ ] **Step 2: Add mono alias helper regression**

In `lib/src/mono/process.rs`, add this test near the existing `generic_function_by_id_ignores_string_keyed_compat_maps` test:

```rust
#[test]
fn register_function_aliases_uses_hir_display_aliases_by_owner_id() {
    let mut mono = Monomorphizer::new();
    let id = DefId::new(CrateId(0), LocalDefId(981));
    let program = crate::hir::HirProgram::from_parts_with_canonical_names(
        HashMap::from([
            ("canonical".to_string(), generic_identity_for_process_test(id, "canonical")),
            ("alias".to_string(), generic_identity_for_process_test(id, "canonical")),
        ]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
        &HashMap::from([(id, "canonical".to_string())]),
    );

    mono.register_function_aliases(&program);

    assert_eq!(
        mono.function_aliases.get("alias"),
        Some(&"canonical".to_string())
    );
    assert!(!mono.function_aliases.contains_key("canonical"));
}
```

- [ ] **Step 3: Run current codegen/mono direct-name-map audit**

Run:

```bash
rg "program\.names\.(structs|enums|functions|traits)_by_name" lib/src/codegen/mod.rs lib/src/mono/process.rs
```

Expected before migration: matches remain in `register_nominal_layouts` and `register_function_aliases`.

- [ ] **Step 4: Replace codegen layout alias loops**

In `CodeGen::register_nominal_layouts`, replace the struct alias loop with:

```rust
        for (id, canonical_name, s) in program.structs_by_id() {
            for alias in program.struct_display_aliases(id) {
                if alias == canonical_name || self.struct_info.contains_key(alias) {
                    continue;
                }
                let mut fields = Vec::with_capacity(s.fields.len());
                for field in &s.fields {
                    let field_ty = self.intern_structural_type(&field.ty);
                    fields.push((field.name.clone(), field_ty));
                }
                self.struct_info.insert(alias.to_string(), fields);
                self.struct_generic_params
                    .insert(alias.to_string(), s.generic_params.clone());
                self.struct_generic_owners.insert(alias.to_string(), s.id);
            }
        }
```

Replace the enum alias loop with:

```rust
        for (id, canonical_name, e) in program.enums_by_id() {
            for alias in program.enum_display_aliases(id) {
                if alias == canonical_name || self.enum_info.contains_key(alias) {
                    continue;
                }
                let variants: Vec<CodegenEnumVariantLayout> = e
                    .variants
                    .iter()
                    .map(|v| CodegenEnumVariantLayout {
                        name: v.name.clone(),
                        fields: self.codegen_variant_fields_from_hir(&v.fields),
                    })
                    .collect();
                self.enum_info.insert(alias.to_string(), variants);
                self.enum_generic_params
                    .insert(alias.to_string(), e.generic_params.clone());
                self.enum_generic_owners.insert(alias.to_string(), e.id);
            }
        }
```

Do not remove the canonical `structs_by_id()` and `enums_by_id()` loops.

- [ ] **Step 5: Replace mono alias registration**

In `Monomorphizer::register_function_aliases` in `lib/src/mono/process.rs`, replace the direct `program.names.functions_by_name` iteration with:

```rust
    pub(super) fn register_function_aliases(&mut self, program: &crate::hir::HirProgram) {
        for (id, canonical_name, _) in program.functions_by_id() {
            for alias in program.function_display_aliases(id) {
                if alias != canonical_name {
                    self.function_aliases
                        .insert(alias.to_string(), canonical_name.to_string());
                }
            }
        }
    }
```

If `lib/src/mono/external.rs` tests fail because object-backed or dependency alias registration relied on filtered direct map behavior, keep the same helper as the single alias source by adjusting test fixtures or resolver setup, not by reintroducing inline name-map iteration.

- [ ] **Step 6: Verify codegen and mono migration**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib register_nominal_layouts_records_display_aliases_by_owner_id -- --exact --nocapture
cargo test -p rock-lib register_function_aliases_uses_hir_display_aliases_by_owner_id -- --exact --nocapture
cargo test -p rock-lib codegen::tests -- --nocapture
cargo test -p rock-lib mono::process -- --nocapture
cargo test -p rock-lib mono::external -- --nocapture
rg "program\.names\.(structs|enums|functions|traits)_by_name" lib/src/codegen/mod.rs lib/src/mono/process.rs
```

Expected: tests pass. The final `rg` should not report production direct name-map reads in the changed codegen or mono process paths.

## Task 5: Classify Remaining Name-Table Reads And Update Architecture Docs

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify source comments only if the final audit finds a production compatibility path whose intent is unclear

- [ ] **Step 1: Run the remaining-read audit**

Run:

```bash
rg "\.(functions|structs|enums|traits)_by_name|function_by_name|struct_by_name|enum_by_name|trait_by_name" lib/src
```

Expected: remaining hits are one of these categories:

- HIR helper implementations in `lib/src/hir/mod.rs`.
- HIR tests and setup code.
- Lowerer/collector lexical or unresolved-name compatibility paths.
- Explicit source-name entrypoint lookup such as `main` if no canonical entry ID exists at that boundary.
- Artifact/display/alias compatibility code whose function or helper name states compatibility/display intent.

If a production hit in a migrated consumer still directly reads `program.names.*_by_name`, either migrate it through the new helper APIs or add a narrow comment explaining why it is an unresolved-name compatibility path.

- [ ] **Step 2: Update master audit checklist wording**

In `docs/superpowers/plans/master-audit-checklist.md`, update the Identity And Arenas summary row from:

```markdown
| Identity And Arenas | In progress | `lib/src/hir/mod.rs`, `lib/src/collect/*`, `lib/src/lower/paths.rs`, `lib/src/lower/control_flow/pattern.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/products.rs` | Current-crate ID allocation, ID-owned `HirProgram` storage, indexed artifact declaration collection, and HIR local/reference/call sidecars have landed, but compatibility string maps and future non-ID-keyed work remain |
```

to:

```markdown
| Identity And Arenas | In progress | `lib/src/hir/mod.rs`, `lib/src/collect/*`, `lib/src/lower/paths.rs`, `lib/src/lower/control_flow/pattern.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/products.rs`, `lib/src/mono/*`, `lib/src/mir/builder/mod.rs`, `lib/src/dce.rs`, `lib/src/codegen/mod.rs` | Current-crate ID allocation, ID-owned `HirProgram` storage, indexed artifact declaration collection, HIR local/reference/call sidecars, and migrated compatibility-name-map consumers have landed; remaining strings are explicit display/diagnostic/artifact/unresolved-name compatibility or future non-ID-keyed work |
```

In the same file, update the Monomorphization Instances summary row phrase:

```markdown
string-keyed mono semantic lookup cleanup is complete where canonical IDs are available
```

to:

```markdown
string-keyed mono semantic lookup cleanup is complete where canonical IDs are available, and function alias registration now uses explicit HIR display-alias helpers
```

Update the Identity And Arenas `Still to do` item from:

```markdown
- [ ] Retire compatibility string ownership maps under Tasks 4-5 after all migrated consumers use canonical ID-keyed tables directly.
```

to:

```markdown
- [ ] Continue auditing compatibility string ownership maps only at explicit display, diagnostics, artifact, unresolved-name, or legacy test/setup boundaries; migrated mono, MIR, DCE, and codegen consumers should not read them inline as semantic owner sources.
```

- [ ] **Step 3: Update ordered roadmap wording**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, add this line after the 2026-05-25 Code-Verified Reconciliation heading or in the nearest current rebaseline notes section:

```markdown
- 2026-06-05 HIR compatibility-name-map cleanup retired inline semantic reads from migrated mono, MIR, DCE, and codegen consumers by routing display aliases through explicit `HirProgram` helper APIs; remaining string maps are compatibility/display/unresolved-name metadata, not semantic owner authority for those paths.
```

In the task table row for Task 2 or Task 15, replace any wording that implies migrated consumers still directly use string maps with wording that says the remaining strings are explicit compatibility metadata.

- [ ] **Step 4: Verify docs wording**

Re-read the edited ranges:

```bash
rg -n "compatibility string ownership maps|compatibility-name-map|function alias registration|migrated mono" docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
```

Expected: wording is narrow and does not claim all string metadata or all `HirNameTables` fields are removed.

## Task 6: Final Verification And Review

**Files:**
- No planned source edits. Fix only files named by failing diagnostics or tests.

- [ ] **Step 1: Run final direct-read audit for migrated paths**

Run:

```bash
rg "program(\.program)?\.names\.(functions|structs|enums|traits)_by_name" lib/src/mir/builder/mod.rs lib/src/codegen/mod.rs lib/src/dce.rs lib/src/mono/process.rs
```

Expected: no production direct semantic reads. If test setup writes still match, confirm each is inside `#[cfg(test)]` and not part of production behavior.

- [ ] **Step 2: Run final focused tests**

Run:

```bash
cargo test -p rock-lib hir_program_lists_display_aliases_by_def_id -- --exact --nocapture
cargo test -p rock-lib hir_program_resolves_nominal_display_aliases_only_to_existing_owners -- --exact --nocapture
cargo test -p rock-lib backend_metadata_records_nominal_aliases_from_hir_display_helpers -- --exact --nocapture
cargo test -p rock-lib dce_method_lookup_uses_nominal_display_aliases_by_owner_id -- --exact --nocapture
cargo test -p rock-lib register_nominal_layouts_records_display_aliases_by_owner_id -- --exact --nocapture
cargo test -p rock-lib register_function_aliases_uses_hir_display_aliases_by_owner_id -- --exact --nocapture
cargo test -p rock-lib mono::process -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib dce -- --nocapture
cargo test -p rock-lib codegen::tests -- --nocapture
```

Expected: all focused tests pass.

- [ ] **Step 3: Run final quality gates**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

Expected: all pass.

- [ ] **Step 4: Request final reviews**

Request two reviews before claiming completion:

- Spec compliance review against `docs/superpowers/specs/2026-06-05-hir-compatibility-name-map-retirement-design.md`.
- Code quality review focused on HIR helper boundaries, canonical-ID use, display/alias compatibility naming, same-name regressions, and doc wording accuracy.

Expected: both reviews pass, or all Critical/Important findings are fixed and re-reviewed.

## Self-Review Notes

- Spec coverage: Task 1 adds explicit helper APIs for display/alias enumeration by canonical owner ID. Tasks 2-4 migrate MIR builder, DCE, codegen, and mono consumers away from inline direct name-map reads. Task 5 classifies remaining name-table reads and updates docs. Task 6 verifies focused and full quality gates.
- Non-goals preserved: the plan does not delete `HirNameTables`, does not redesign artifact schemas, and leaves source/lexical/unresolved compatibility paths intact.
- Placeholder scan: no planned code step uses placeholder markers or unspecified test content.
- Type consistency: helper names are consistent across tasks: `function_display_aliases`, `struct_display_aliases`, `enum_display_aliases`, `nominal_display_aliases`, and `nominal_owner_id_for_display_alias`.
