# Authoritative HIR ID-Keyed Storage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make current-crate HIR ownership ID-first by allocating semantic IDs during collection, storing core HIR definitions by `DefId`, and carrying canonical IDs on the first high-value HIR references.

**Architecture:** Build on the existing `HirDefinitionIndexes` and child-ID tables, but invert ownership so `HirProgram` stores functions, structs, enums, traits, impls, and externs by canonical ID. Keep source names in side tables and compatibility accessors while migrating downstream consumers in phase order. Limit HIR reference migration to resolved top-level calls, struct literals/patterns, enum variants/patterns, and method locations.

**Tech Stack:** Rust 2021, `rock-lib`, `DefId`, `FieldId`, `VariantId`, `AssocTypeId`, `HirProgram`, `HirDefinitionIndexes`, collection resolver tables, inference finalization, monomorphization, MIR builder, codegen, product artifacts, `cargo test -p rock-lib`.

---

## Source Documents

- Spec: `docs/superpowers/specs/2026-05-17-authoritative-hir-id-keyed-storage-design.md`
- Roadmap: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Prior HIR index plan: `docs/superpowers/plans/2026-05-10-hir-id-keyed-definition-indexes.md`
- Tracker: `docs/superpowers/plans/master-audit-checklist.md`

## Execution Notes

- Do not touch untracked `.sisyphus/`.
- Do not commit unless the user explicitly asks for a commit.
- Run tests serially.
- Prefer the smallest focused test command before broader verification.
- Keep temporary compatibility accessors narrow and ID-backed.

## File Structure

- Modify `lib/src/hir/mod.rs`: add ID-owned `HirProgram` storage, `HirNameTables`, `HirProgramOrder`, ID-backed method locations, resolved var refs, resolved struct/enum pattern shapes, constructors, accessors, and unit tests.
- Modify `lib/src/collect/context.rs`: remove supported-path use of `fresh_provisional_def_id` or restrict it to test-only/unreachable legacy fixtures after declaration builders accept canonical IDs.
- Modify `lib/src/collect/headers.rs`: make struct, enum, trait, impl, extern, function signature, and function header builders accept canonical IDs from collection or the owning declaration context.
- Modify `lib/src/collect/mod.rs`: thread canonical IDs from `ItemIndex`/`IndexingIds` into declaration building, remove post-declaration current-crate repair paths, keep artifact/external remap behavior, and add collection tests.
- Modify `lib/src/infer/mod.rs`: construct `HirProgram` through ID-owned constructors and remove finalization-time missing method ID allocation for supported current-crate methods.
- Modify `lib/src/lower/paths.rs`: emit resolved top-level function/extern references, struct literal targets, and enum variant targets.
- Modify `lib/src/lower/control_flow/pattern.rs`: emit resolved struct and enum pattern targets.
- Modify traversal/substitution files that match HIR shapes: `lib/src/lower/types_helpers/type_vars.rs`, `lib/src/lower/traits/conformance.rs`, `lib/src/infer/mod.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/specialize.rs`, `lib/src/codegen/**`, `lib/src/mir/builder/**`, and `lib/src/products.rs`.
- Modify `lib/tests/integration.rs`: add user-visible same-name and artifact-backed regression tests when unit coverage cannot observe runtime behavior.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: update completed and remaining checklist items after verification.

---

### Task 1: Add Current-Crate ID Authority Tests

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Reference: `lib/src/collect/context.rs`
- Reference: `lib/src/collect/headers.rs`

- [ ] **Step 1: Add collection assertion helpers**

Inside `#[cfg(test)] mod tests` in `lib/src/collect/mod.rs`, add these helpers near the existing test helpers:

```rust
fn is_invalid_current_crate_id(id: DefId) -> bool {
    id.crate_id == CrateId(u32::MAX)
}

fn assert_no_invalid_current_crate_ids(decls: &Declarations) {
    for (name, function) in &decls.functions {
        assert!(
            !is_invalid_current_crate_id(function.id),
            "function {name} kept invalid current-crate id {:?}",
            function.id
        );
        for generic_id in &function.generic_param_ids {
            assert!(
                !is_invalid_current_crate_id(generic_id.owner),
                "function {name} generic {:?} kept invalid owner",
                generic_id
            );
        }
    }

    for (name, sig) in &decls.function_sigs {
        assert!(
            !is_invalid_current_crate_id(sig.id),
            "signature {name} kept invalid current-crate id {:?}",
            sig.id
        );
        for generic_id in &sig.generic_param_ids {
            assert!(
                !is_invalid_current_crate_id(generic_id.owner),
                "signature {name} generic {:?} kept invalid owner",
                generic_id
            );
        }
    }

    for (name, strukt) in &decls.structs {
        assert!(
            !is_invalid_current_crate_id(strukt.id),
            "struct {name} kept invalid current-crate id {:?}",
            strukt.id
        );
    }

    for (name, enum_) in &decls.enums {
        assert!(
            !is_invalid_current_crate_id(enum_.id),
            "enum {name} kept invalid current-crate id {:?}",
            enum_.id
        );
    }

    for (name, trait_def) in &decls.traits {
        assert!(
            !is_invalid_current_crate_id(trait_def.id),
            "trait {name} kept invalid current-crate id {:?}",
            trait_def.id
        );
        for (method_name, method) in &trait_def.methods {
            assert!(
                !is_invalid_current_crate_id(method.id),
                "trait method {name}.{method_name} kept invalid id {:?}",
                method.id
            );
        }
        for (sig_name, sig) in &trait_def.signatures {
            assert!(
                !is_invalid_current_crate_id(sig.id),
                "trait signature {name}.{sig_name} kept invalid id {:?}",
                sig.id
            );
        }
    }

    for imp in &decls.impls {
        assert!(
            !is_invalid_current_crate_id(imp.id),
            "impl for {} kept invalid id {:?}",
            imp.type_name,
            imp.id
        );
        for (method_name, method) in &imp.methods {
            assert!(
                !is_invalid_current_crate_id(method.id),
                "impl method {}.{} kept invalid id {:?}",
                imp.type_name,
                method_name,
                method.id
            );
        }
    }

    for ext in &decls.externs {
        assert!(
            !is_invalid_current_crate_id(ext.id),
            "extern {} kept invalid id {:?}",
            ext.name,
            ext.id
        );
    }
}
```

- [ ] **Step 2: Add a failing test for supported current-crate declaration IDs**

Add this test in `lib/src/collect/mod.rs`:

```rust
#[test]
fn collect_assigns_supported_current_crate_ids_before_lowering() {
    let program = Program {
        module: Module {
            name: None,
            top_levels: vec![
                TopLevel::FunctionSig(function_sig(
                    "identity",
                    ParseType::Function(vec![ParseType::Type(type_inner("I64"))]),
                )),
                TopLevel::FunctionDecl(single_param_function_decl("identity", "value")),
                TopLevel::StructDecl(StructDecl {
                    name: type_inner("Box"),
                    fields: vec![],
                    exported: false,
                }),
                TopLevel::EnumDecl(crate::ast::EnumDecl {
                    name: type_inner("Maybe"),
                    variants: vec![crate::ast::EnumVariant {
                        name: ident("None"),
                        fields: crate::ast::NamedFieldsOrTypesList::TypesList(vec![]),
                    }],
                    exported: false,
                }),
                TopLevel::TraitDecl(crate::ast::TraitDecl {
                    name: type_inner("Show"),
                    associated_types: vec![],
                    methods: HashMap::from([(ident("show"), function_decl("show"))]),
                    signatures: HashMap::new(),
                    exported: false,
                }),
                TopLevel::Impl(trait_impl("Box", "Show", "show")),
                TopLevel::Extern(function_sig(
                    "puts",
                    ParseType::Function(vec![ParseType::Type(type_inner("I32"))]),
                )),
            ],
            is_inline: false,
            filepath: None,
        },
    };

    let decls = collect(&program, &CrateContext::new(), false, Some("test"))
        .expect("collect assigns supported current-crate ids");

    assert_no_invalid_current_crate_ids(&decls);
}
```

- [ ] **Step 3: Run the failing collection test**

Run: `cargo test -p rock-lib collect_assigns_supported_current_crate_ids_before_lowering -- --exact --nocapture`

Expected: FAIL because at least one supported current-crate declaration still uses `CrateId(u32::MAX)` or a zero extern ID that was not allocated by collection.

- [ ] **Step 4: Thread canonical IDs into declaration builders**

In `lib/src/collect/headers.rs`, make these exact first-line transformations so collection can pass canonical IDs:

```rust
// Before
pub(crate) fn build_struct(context: &mut CollectContext, sd: &ast::StructDecl) -> HirStruct

// After
pub(crate) fn build_struct_with_id(
    context: &mut CollectContext,
    sd: &ast::StructDecl,
    id: DefId,
) -> HirStruct
```

```rust
// Before
let id = context.fresh_provisional_def_id();
let (prev_owner, prev_params) = context.push_generic_context(id, generic_params.clone());

// After
let (prev_owner, prev_params) = context.push_generic_context(id, generic_params.clone());
```

Repeat the same transformation for `build_enum`, `build_trait`, `build_impl`, and `build_extern_with_name`. The resulting production entry points must be named `build_enum_with_id`, `build_trait_with_id`, `build_impl_with_id`, and `build_extern_with_id`. Production collection must call the `_with_id` functions. Tests that need isolated header construction must pass explicit IDs through the new functions.

- [ ] **Step 5: Build ID maps before local declaration collection**

In `lib/src/collect/mod.rs`, move `IndexingIds::new_root`, source module indexing, and resolver construction before `collector.collect_local_declarations(&program.module)`. Add a small ID environment passed into `LocalCollector`:

```rust
#[derive(Debug, Clone, Default)]
pub(crate) struct CollectedIdEnvironment {
    pub item_ids_by_path: HashMap<String, DefId>,
    pub impl_ids_by_path: HashMap<String, DefId>,
}
```

Populate `item_ids_by_path` from `resolver.item_paths`. Populate `impl_ids_by_path` from `ItemRecord` entries with `ItemKind::Impl`, using a stable key built from module path plus impl ordinal. Use that environment when collecting local declarations so builders receive IDs before HIR headers exist.

- [ ] **Step 6: Remove supported-path post-declaration fresh ID repair**

Delete or narrow `assign_fresh_ids_to_unresolved_named_items` so it no longer repairs supported current-crate declarations after local collection. Replace calls that previously depended on it with a validation pass:

```rust
fn validate_supported_current_crate_ids(decls: &Declarations) -> Result<(), Vec<ResolveError>> {
    fn push_missing(errors: &mut Vec<ResolveError>, kind: &str, name: &str, id: DefId) {
        if id.crate_id == CrateId(u32::MAX) {
            errors.push(ResolveError::new(format!(
                "missing canonical {kind} identity for {name}"
            )));
        }
    }

    let mut errors = Vec::new();

    for (name, function) in &decls.functions {
        push_missing(&mut errors, "function", name, function.id);
        for generic_id in &function.generic_param_ids {
            push_missing(&mut errors, "function generic owner", name, generic_id.owner);
        }
    }
    for (name, sig) in &decls.function_sigs {
        push_missing(&mut errors, "function signature", name, sig.id);
        for generic_id in &sig.generic_param_ids {
            push_missing(&mut errors, "signature generic owner", name, generic_id.owner);
        }
    }
    for (name, structure) in &decls.structs {
        push_missing(&mut errors, "struct", name, structure.id);
    }
    for (name, enumeration) in &decls.enums {
        push_missing(&mut errors, "enum", name, enumeration.id);
    }
    for (name, trait_def) in &decls.traits {
        push_missing(&mut errors, "trait", name, trait_def.id);
        for (method_name, method) in &trait_def.methods {
            push_missing(
                &mut errors,
                "trait method",
                &format!("{name}.{method_name}"),
                method.id,
            );
        }
        for (signature_name, signature) in &trait_def.signatures {
            push_missing(
                &mut errors,
                "trait signature",
                &format!("{name}.{signature_name}"),
                signature.id,
            );
        }
    }
    for imp in &decls.impls {
        push_missing(&mut errors, "impl", &imp.type_name, imp.id);
        for (method_name, method) in &imp.methods {
            push_missing(
                &mut errors,
                "impl method",
                &format!("{}.{}", imp.type_name, method_name),
                method.id,
            );
        }
    }
    for ext in &decls.externs {
        push_missing(&mut errors, "extern", &ext.name, ext.id);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

Call `validate_supported_current_crate_ids(&decls)?` before returning `Declarations`.

- [ ] **Step 7: Run the collection test again**

Run: `cargo test -p rock-lib collect_assigns_supported_current_crate_ids_before_lowering -- --exact --nocapture`

Expected: PASS.

---

### Task 2: Make `HirProgram` Own Definitions By `DefId`

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: direct `HirProgram::from_parts` test callers in `lib/src/**`

- [ ] **Step 1: Add failing HIR storage tests**

In `lib/src/hir/mod.rs`, add these tests inside the existing test module:

```rust
#[test]
fn hir_program_owns_functions_by_def_id_and_names_are_views() {
    let canonical_id = def_id(10);
    let alias_id = canonical_id;
    let functions = HashMap::from([
        ("main".to_string(), test_function(canonical_id, "main")),
        ("alias_main".to_string(), test_function(alias_id, "main")),
    ]);
    let canonical_names = HashMap::from([(canonical_id, "main".to_string())]);

    let program = HirProgram::from_parts_with_canonical_names(
        functions,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
        &canonical_names,
    );

    assert_eq!(program.functions.len(), 1);
    assert!(program.functions.contains_key(&canonical_id));
    assert_eq!(program.names.functions_by_name["main"], canonical_id);
    assert_eq!(program.names.functions_by_name["alias_main"], canonical_id);
    assert_eq!(program.function_by_name("alias_main").unwrap().0, canonical_id);
}

#[test]
#[should_panic(expected = "duplicate HIR function DefId")]
fn hir_program_rejects_different_function_bodies_for_one_def_id() {
    let id = def_id(11);
    let mut first = test_function(id, "first");
    first.ret_type = Type::I64;
    let mut second = test_function(id, "second");
    second.ret_type = Type::Bool;

    let _ = HirProgram::from_parts(
        HashMap::from([
            ("first".to_string(), first),
            ("second".to_string(), second),
        ]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
}
```

- [ ] **Step 2: Run the failing HIR storage test**

Run: `cargo test -p rock-lib hir_program_owns_functions_by_def_id_and_names_are_views -- --exact --nocapture`

Expected: FAIL because `program.functions` is still keyed by `String` and `HirNameTables` does not exist.

- [ ] **Step 3: Add ID-owned HIR storage types**

In `lib/src/hir/mod.rs`, replace the current `HirProgram` fields with this shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HirNameTables {
    pub functions_by_name: HashMap<String, DefId>,
    pub structs_by_name: HashMap<String, DefId>,
    pub enums_by_name: HashMap<String, DefId>,
    pub traits_by_name: HashMap<String, DefId>,
    pub externs_by_name: HashMap<String, DefId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HirProgramOrder {
    pub impls: Vec<DefId>,
    pub externs: Vec<DefId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub functions: HashMap<DefId, HirFunction>,
    pub structs: HashMap<DefId, HirStruct>,
    pub enums: HashMap<DefId, HirEnum>,
    pub traits: HashMap<DefId, HirTrait>,
    pub impls: HashMap<DefId, HirImpl>,
    pub externs: HashMap<DefId, HirExtern>,
    #[serde(default)]
    pub names: HirNameTables,
    #[serde(default)]
    pub order: HirProgramOrder,
    #[serde(skip, default)]
    pub indexes: HirDefinitionIndexes,
}
```

- [ ] **Step 4: Replace constructors with ID-owning constructors**

Keep the existing constructor signatures so callers can migrate incrementally:

```rust
impl HirProgram {
    pub fn from_parts(
        functions: HashMap<String, HirFunction>,
        structs: HashMap<String, HirStruct>,
        enums: HashMap<String, HirEnum>,
        traits: HashMap<String, HirTrait>,
        impls: Vec<HirImpl>,
        externs: Vec<HirExtern>,
    ) -> Self {
        Self::from_parts_with_canonical_names(
            functions,
            structs,
            enums,
            traits,
            impls,
            externs,
            &HashMap::new(),
        )
    }

    pub fn from_parts_with_canonical_names(
        functions: HashMap<String, HirFunction>,
        structs: HashMap<String, HirStruct>,
        enums: HashMap<String, HirEnum>,
        traits: HashMap<String, HirTrait>,
        impls: Vec<HirImpl>,
        externs: Vec<HirExtern>,
        canonical_names_by_id: &HashMap<DefId, String>,
    ) -> Self {
        let (functions, functions_by_name) = move_named_definitions(
            functions,
            canonical_names_by_id,
            "function",
            |function| function.id,
            definitions_are_same_function,
        );
        let (structs, structs_by_name) = move_named_definitions(
            structs,
            canonical_names_by_id,
            "struct",
            |structure| structure.id,
            definitions_are_same_struct,
        );
        let (enums, enums_by_name) = move_named_definitions(
            enums,
            canonical_names_by_id,
            "enum",
            |enumeration| enumeration.id,
            definitions_are_same_enum,
        );
        let (traits, traits_by_name) = move_named_definitions(
            traits,
            canonical_names_by_id,
            "trait",
            |trait_def| trait_def.id,
            definitions_are_same_trait,
        );
        let (impls, impl_order) = move_indexed_definitions(impls, "impl", |imp| imp.id);
        let (externs, extern_order, externs_by_name) = move_extern_definitions(externs);

        let mut program = Self {
            functions,
            structs,
            enums,
            traits,
            impls,
            externs,
            names: HirNameTables {
                functions_by_name,
                structs_by_name,
                enums_by_name,
                traits_by_name,
                externs_by_name,
            },
            order: HirProgramOrder {
                impls: impl_order,
                externs: extern_order,
            },
            indexes: HirDefinitionIndexes::default(),
        };
        program.rebuild_indexes_with_canonical_names(canonical_names_by_id);
        program
    }
}
```

Add these helper functions in the same file:

```rust
fn move_named_definitions<T, IdOf, Same>(
    entries: HashMap<String, T>,
    canonical_names_by_id: &HashMap<DefId, String>,
    kind: &str,
    id_of: IdOf,
    same_definition: Same,
) -> (HashMap<DefId, T>, HashMap<String, DefId>)
where
    T: Clone + std::fmt::Debug,
    IdOf: Fn(&T) -> DefId,
    Same: Fn(&T, &T) -> bool,
{
    let mut grouped: HashMap<DefId, Vec<(String, T)>> = HashMap::new();
    let mut names_by_name = HashMap::new();

    for (name, item) in entries {
        let id = id_of(&item);
        names_by_name.insert(name.clone(), id);
        grouped.entry(id).or_default().push((name, item));
    }

    let mut output = HashMap::new();
    for (id, mut candidates) in grouped {
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let selected_index = canonical_names_by_id
            .get(&id)
            .and_then(|canonical_name| {
                candidates
                    .iter()
                    .position(|(name, _)| name == canonical_name)
            })
            .unwrap_or(0);
        let selected = candidates[selected_index].1.clone();

        for (_, candidate) in &candidates {
            assert!(
                same_definition(&selected, candidate),
                "duplicate HIR {kind} DefId {:?}: {:?}",
                id,
                candidates
            );
        }

        assert!(
            output.insert(id, selected).is_none(),
            "duplicate HIR {kind} DefId: {:?}",
            id
        );
    }

    (output, names_by_name)
}

fn move_indexed_definitions<T, IdOf>(
    entries: Vec<T>,
    kind: &str,
    id_of: IdOf,
) -> (HashMap<DefId, T>, Vec<DefId>)
where
    IdOf: Fn(&T) -> DefId,
{
    let mut output = HashMap::new();
    let mut order = Vec::new();
    for item in entries {
        let id = id_of(&item);
        assert!(
            output.insert(id, item).is_none(),
            "duplicate HIR {kind} DefId: {:?}",
            id
        );
        order.push(id);
    }
    (output, order)
}

fn move_extern_definitions(
    entries: Vec<HirExtern>,
) -> (HashMap<DefId, HirExtern>, Vec<DefId>, HashMap<String, DefId>) {
    let mut output = HashMap::new();
    let mut order = Vec::new();
    let mut names = HashMap::new();
    for ext in entries {
        let id = ext.id;
        names.insert(ext.name.clone(), id);
        assert!(
            output.insert(id, ext).is_none(),
            "duplicate HIR extern DefId: {:?}",
            id
        );
        order.push(id);
    }
    (output, order, names)
}

fn definitions_are_same_function(left: &HirFunction, right: &HirFunction) -> bool {
    format!("{:?}", left) == format!("{:?}", right)
}

fn definitions_are_same_struct(left: &HirStruct, right: &HirStruct) -> bool {
    format!("{:?}", left) == format!("{:?}", right)
}

fn definitions_are_same_enum(left: &HirEnum, right: &HirEnum) -> bool {
    format!("{:?}", left) == format!("{:?}", right)
}

fn definitions_are_same_trait(left: &HirTrait, right: &HirTrait) -> bool {
    format!("{:?}", left) == format!("{:?}", right)
}
```

- [ ] **Step 5: Add ID and name accessors**

In `impl HirProgram`, add these methods and update existing ID iterators to use ID-owned maps:

```rust
pub fn function_by_name(&self, name: &str) -> Option<(DefId, &HirFunction)> {
    let id = *self.names.functions_by_name.get(name)?;
    self.functions.get(&id).map(|function| (id, function))
}

pub fn struct_by_name(&self, name: &str) -> Option<(DefId, &HirStruct)> {
    let id = *self.names.structs_by_name.get(name)?;
    self.structs.get(&id).map(|structure| (id, structure))
}

pub fn enum_by_name(&self, name: &str) -> Option<(DefId, &HirEnum)> {
    let id = *self.names.enums_by_name.get(name)?;
    self.enums.get(&id).map(|enumeration| (id, enumeration))
}

pub fn trait_by_name(&self, name: &str) -> Option<(DefId, &HirTrait)> {
    let id = *self.names.traits_by_name.get(name)?;
    self.traits.get(&id).map(|trait_def| (id, trait_def))
}

pub fn impls_in_order(&self) -> impl Iterator<Item = (DefId, &HirImpl)> {
    self.order
        .impls
        .iter()
        .filter_map(|id| self.impls.get(id).map(|imp| (*id, imp)))
}

pub fn externs_in_order(&self) -> impl Iterator<Item = (DefId, &HirExtern)> {
    self.order
        .externs
        .iter()
        .filter_map(|id| self.externs.get(id).map(|ext| (*id, ext)))
}
```

- [ ] **Step 6: Update index rebuilding**

Change `HirDefinitionIndexes::from_parts` to accept ID-owned maps and the name tables. Method, field, variant, and associated-type indexes must be built from ID-owned definitions. `methods_by_id` will still be name-bearing until Task 4.

- [ ] **Step 7: Run focused HIR storage tests**

Run: `cargo test -p rock-lib hir_program_owns_functions_by_def_id_and_names_are_views -- --exact --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib hir_program_rejects_different_function_bodies_for_one_def_id -- --exact --nocapture`

Expected: PASS.

---

### Task 3: Migrate Core Consumers To HIR Accessors

**Files:**
- Modify: `lib/src/infer/mod.rs`
- Modify: `lib/src/products.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/dce.rs`
- Modify: affected tests in those files

- [ ] **Step 1: Run a compile check to list direct field errors**

Run: `cargo test -p rock-lib hir_program_owns_functions_by_def_id_and_names_are_views -- --exact --nocapture`

Expected: FAIL with Rust compiler errors where consumers still treat `program.functions`, `program.structs`, `program.enums`, `program.traits`, `program.impls`, or `program.externs` as string-keyed maps or vectors.

- [ ] **Step 2: Migrate read-only top-level iteration**

Replace read-only loops with accessors. The loop body from the original consumer remains in place; only the iterator source and binding names change.

| Old source | New source | New bindings |
| --- | --- | --- |
| `&program.functions` | `program.functions_by_id()` | `(id, name, function)` |
| `&program.structs` | `program.structs_by_id()` | `(id, name, structure)` |
| `&program.enums` | `program.enums_by_id()` | `(id, name, enumeration)` |
| `&program.traits` | `program.traits_by_id()` | `(id, name, trait_def)` |
| `&program.impls` | `program.impls_in_order()` | `(id, imp)` |
| `&program.externs` | `program.externs_in_order()` | `(id, ext)` |

Apply this to `lib/src/products.rs`, `lib/src/codegen/mod.rs`, `lib/src/mono/process.rs`, `lib/src/mono/external.rs`, `lib/src/mir/builder/mod.rs`, and `lib/src/dce.rs`.

- [ ] **Step 3: Migrate name lookups to compatibility accessors**

Replace direct name map lookups with accessors:

```rust
let Some((function_id, function)) = program.function_by_name(name) else {
    return Err(format!("unknown function {name}"));
};
```

Use the phase's existing error type instead of `String` when the surrounding function already returns structured diagnostics. The key rule is that name lookup returns an ID-owned definition.

- [ ] **Step 4: Migrate tests that mutate `HirProgram` directly**

Replace direct test mutations such as `hir.program.functions.insert(name, function)` with reconstruction through `HirProgram::from_parts` or with new test-only helpers:

```rust
fn replace_program_functions(
    program: &mut HirProgram,
    functions: HashMap<String, HirFunction>,
) {
    let rebuilt = HirProgram::from_parts(
        functions,
        program
            .structs_by_id()
            .map(|(_, name, item)| (name.to_string(), item.clone()))
            .collect(),
        program
            .enums_by_id()
            .map(|(_, name, item)| (name.to_string(), item.clone()))
            .collect(),
        program
            .traits_by_id()
            .map(|(_, name, item)| (name.to_string(), item.clone()))
            .collect(),
        program.impls_in_order().map(|(_, item)| item.clone()).collect(),
        program.externs_in_order().map(|(_, item)| item.clone()).collect(),
    );
    *program = rebuilt;
}
```

Keep helpers local to test modules. Production code must use normal constructors and accessors.

- [ ] **Step 5: Run a broad compile check**

Run: `cargo test -p rock-lib --no-run`

Expected: PASS compile. If it fails, keep migrating direct field assumptions until the command compiles.

---

### Task 4: Make Method Locations ID-Backed

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/codegen/**`
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Add failing method-location tests**

In `lib/src/hir/mod.rs`, add:

```rust
#[test]
fn hir_method_locations_use_owner_ids_not_names_or_indexes() {
    let trait_id = def_id(20);
    let trait_method_id = def_id(21);
    let impl_id = def_id(22);
    let impl_method_id = def_id(23);

    let traits = HashMap::from([(
        "Display".to_string(),
        HirTrait {
            id: trait_id,
            name: "Display".to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods: HashMap::from([(
                "fmt".to_string(),
                test_function(trait_method_id, "fmt"),
            )]),
            signatures: HashMap::new(),
        },
    )]);
    let impls = vec![HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Widget".to_string()),
        type_name: "Widget".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: Some("Display".to_string()),
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: Vec::new(),
        methods: HashMap::from([(
            "fmt".to_string(),
            test_function(impl_method_id, "fmt"),
        )]),
    }];

    let program = HirProgram::from_parts(
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        traits,
        impls,
        Vec::new(),
    );

    assert_eq!(
        program.indexes.methods_by_id.get(&trait_method_id),
        Some(&HirMethodLocation::TraitDefault {
            trait_id,
            method_id: trait_method_id,
            method_name: "fmt".to_string(),
        })
    );
    assert_eq!(
        program.indexes.methods_by_id.get(&impl_method_id),
        Some(&HirMethodLocation::ImplMethod {
            impl_id,
            method_id: impl_method_id,
            method_name: "fmt".to_string(),
        })
    );
}
```

- [ ] **Step 2: Run the failing method-location test**

Run: `cargo test -p rock-lib hir_method_locations_use_owner_ids_not_names_or_indexes -- --exact --nocapture`

Expected: FAIL because `HirMethodLocation` still stores trait names and impl vector positions.

- [ ] **Step 3: Change `HirMethodLocation`**

In `lib/src/hir/mod.rs`, replace the enum with:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirMethodLocation {
    TraitDefault {
        trait_id: DefId,
        method_id: DefId,
        method_name: String,
    },
    ImplMethod {
        impl_id: DefId,
        method_id: DefId,
        method_name: String,
    },
}
```

Update index construction:

```rust
HirMethodLocation::ImplMethod {
    impl_id: imp.id,
    method_id: method.id,
    method_name: method_name.clone(),
}

HirMethodLocation::TraitDefault {
    trait_id: selected.trait_id,
    method_id,
    method_name: selected.method_name,
}
```

- [ ] **Step 4: Migrate method-location consumers**

Replace matches on trait names or impl indexes with ID accessors. Example pattern:

```rust
match location {
    HirMethodLocation::TraitDefault { trait_id, method_id, method_name } => {
        let (_, trait_def) = program
            .trait_by_id(*trait_id)
            .expect("method location trait id must point at a trait");
        let method = trait_def
            .methods
            .get(method_name)
            .filter(|method| method.id == *method_id)
            .expect("method location method id must point at a trait default");
        // existing behavior using method
    }
    HirMethodLocation::ImplMethod { impl_id, method_id, method_name } => {
        let (_, imp) = program
            .impl_by_id(*impl_id)
            .expect("method location impl id must point at an impl");
        let method = imp
            .methods
            .get(method_name)
            .filter(|method| method.id == *method_id)
            .expect("method location method id must point at an impl method");
        // existing behavior using method
    }
}
```

- [ ] **Step 5: Run method-location tests**

Run: `cargo test -p rock-lib hir_method_locations_use_owner_ids_not_names_or_indexes -- --exact --nocapture`

Expected: PASS.

---

### Task 5: Add Resolved HIR Reference Shapes

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: traversal/substitution matches in `lib/src/lower/**`, `lib/src/infer/mod.rs`, `lib/src/mono/**`, `lib/src/codegen/**`, `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Add HIR reference types**

In `lib/src/hir/mod.rs`, add these types near the existing HIR expression/pattern helper structs:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirVarTarget {
    Function(DefId),
    Extern(DefId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HirVarRef {
    pub name: String,
    pub target: HirVarTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HirStructPatternField {
    pub name: String,
    pub field: Option<HirFieldLocation>,
    pub pattern: HirPattern,
}
```

Change HIR variants:

```rust
pub enum HirExprKind {
    Var(String),
    ResolvedVar(HirVarRef),
    StructLiteral(String, Option<DefId>, Vec<HirStructLiteralField>),
    EnumVariant(String, String, Vec<HirExpr>, Option<HirVariantLocation>),
    // keep remaining variants unchanged
}

pub enum HirPattern {
    Struct(String, Option<DefId>, Vec<Type>, Vec<HirStructPatternField>),
    Enum(String, String, Option<HirVariantLocation>, Vec<HirPattern>),
    // keep remaining variants unchanged
}
```

`EnumVariant` already carries `HirVariantLocation`; keep that location and require `Some` for resolved variants in this slice. The new `StructLiteral` `Option<DefId>` is `Some(struct_id)` for known struct literals and `None` for error recovery.

- [ ] **Step 2: Update no-op traversal arms**

Where traversals currently match `HirExprKind::Var(_)`, change them to:

```rust
HirExprKind::Var(_) | HirExprKind::ResolvedVar(_) => {}
```

Where traversals currently match struct and enum variants, use the new pattern prefixes while keeping each file's current traversal body:

| Old pattern | New pattern | Traversed children |
| --- | --- | --- |
| `HirExprKind::StructLiteral(_, fields)` | `HirExprKind::StructLiteral(_, _, fields)` | `field.value` |
| `HirExprKind::EnumVariant(_, _, args, _)` | `HirExprKind::EnumVariant(_, _, args, _)` | `args` |
| `HirPattern::Struct(_, type_args, fields)` | `HirPattern::Struct(_, _, type_args, fields)` | `type_args` and `field.pattern` |
| `HirPattern::Enum(_, _, items)` | `HirPattern::Enum(_, _, _, items)` | `items` |

Apply these changes to `lib/src/lower/types_helpers/type_vars.rs`, `lib/src/lower/traits/conformance.rs`, `lib/src/infer/mod.rs`, `lib/src/mono/methods.rs`, `lib/src/mono/specialize.rs`, `lib/src/codegen/**`, and `lib/src/crate_artifact/load.rs`.

- [ ] **Step 3: Run a compile check for HIR shape updates**

Run: `cargo test -p rock-lib --no-run`

Expected: PASS compile. If it fails, update every pattern match named in the compiler errors to the new HIR shapes.

---

### Task 6: Lower Resolved Struct And Enum Expression Targets

**Files:**
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/hir/mod.rs` only for shared test walkers that are useful outside `lib/src/lower/paths.rs`

- [ ] **Step 1: Add expression reference tests**

In `lib/src/lower/paths.rs`, add a `#[cfg(test)] mod tests` if the file does not already have one. Add these imports and helpers before the tests:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

use crate::crate_system::CrateContext;
use crate::hir::{HirBlock, HirExpr, HirExprKind};
use crate::infer::ResolvedHirProgram;
use crate::{collect, infer, lower, macro_expansion, parser, Config};

static LOWER_PATH_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_source_to_resolved_hir_for_test(source: &str) -> ResolvedHirProgram {
    let id = LOWER_PATH_TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "rock_lower_path_hir_ref_test_{}_{}",
        std::process::id(),
        id
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let entry_file = dir.join("main.rk");
    std::fs::write(&entry_file, source).unwrap();

    let config = Config {
        entry_file,
        output_dir: dir,
        debug_print: Vec::new(),
        meta_files: Vec::new(),
        extern_artifacts: Vec::new(),
        current_crate_name: Some("test".to_string()),
        opt_level: 0,
        emit_llvm: false,
        no_link: true,
        emit_object: None,
        no_prelude: true,
        no_std: true,
        sysroot: None,
    };

    let ast = parser::parse(&config).expect("test source parses");
    let ast = macro_expansion::expand_macros(ast).expect("test macros expand");
    let crate_ctx = CrateContext::new();
    let decls = collect::collect(&ast, &crate_ctx, false, Some("test"))
        .expect("test declarations collect");
    let partial = lower::program::lower_from_declarations(
        &ast,
        decls,
        &crate_ctx,
        Some("test"),
    )
    .expect("test source lowers");
    infer::finalize(partial).expect("test HIR finalizes")
}

fn find_first_struct_literal(
    block: &HirBlock,
) -> Option<(&String, &Option<crate::ids::DefId>, &Vec<crate::hir::HirStructLiteralField>)> {
    block.stmts.iter().find_map(|stmt| match stmt {
        crate::hir::HirStmt::Expr(expr) | crate::hir::HirStmt::Return(Some(expr)) => {
            find_first_struct_literal_expr(expr)
        }
        _ => None,
    })
}

fn find_first_struct_literal_expr(
    expr: &HirExpr,
) -> Option<(&String, &Option<crate::ids::DefId>, &Vec<crate::hir::HirStructLiteralField>)> {
    match &expr.kind {
        HirExprKind::StructLiteral(name, id, fields) => Some((name, id, fields)),
        HirExprKind::Call(callee, args) => find_first_struct_literal_expr(callee)
            .or_else(|| args.iter().find_map(find_first_struct_literal_expr)),
        HirExprKind::Block(block) => find_first_struct_literal(block),
        _ => None,
    }
}

fn find_first_enum_variant(
    block: &HirBlock,
) -> Option<(&String, &String, &Vec<HirExpr>, &Option<crate::hir::HirVariantLocation>)> {
    block.stmts.iter().find_map(|stmt| match stmt {
        crate::hir::HirStmt::Expr(expr) | crate::hir::HirStmt::Return(Some(expr)) => {
            find_first_enum_variant_expr(expr)
        }
        _ => None,
    })
}

fn find_first_enum_variant_expr(
    expr: &HirExpr,
) -> Option<(&String, &String, &Vec<HirExpr>, &Option<crate::hir::HirVariantLocation>)> {
    match &expr.kind {
        HirExprKind::EnumVariant(enum_name, variant_name, args, location) => {
            Some((enum_name, variant_name, args, location))
        }
        HirExprKind::Call(callee, args) => find_first_enum_variant_expr(callee)
            .or_else(|| args.iter().find_map(find_first_enum_variant_expr)),
        HirExprKind::Block(block) => find_first_enum_variant(block),
        _ => None,
    }
}
```

Add the tests:

```rust
#[test]
fn lower_struct_literal_records_struct_id() {
    let source = r#"
struct Box { value: I64 }

fn main() -> I64 {
    let box = Box { value: 7 }
    box.value
}
"#;

    let resolved = compile_source_to_resolved_hir_for_test(source);
    let (_, main) = resolved.program.function_by_name("main").unwrap();
    let struct_id = resolved.program.names.structs_by_name["Box"];

    let literal = find_first_struct_literal(&main.body).unwrap();
    assert_eq!(literal.1, Some(struct_id));
}

#[test]
fn lower_enum_variant_records_enum_and_variant_ids() {
    let source = r#"
enum Maybe { Some(I64), None }

fn main() -> Maybe {
    Maybe::Some(7)
}
"#;

    let resolved = compile_source_to_resolved_hir_for_test(source);
    let (_, main) = resolved.program.function_by_name("main").unwrap();
    let enum_id = resolved.program.names.enums_by_name["Maybe"];

    let variant = find_first_enum_variant(&main.body).unwrap();
    let location = variant.3.as_ref().expect("enum variant must be resolved");
    assert_eq!(location.owner, enum_id);
    assert_eq!(location.name, "Some");
}
```

- [ ] **Step 2: Run the failing expression reference tests**

Run: `cargo test -p rock-lib lower_struct_literal_records_struct_id -- --exact --nocapture`

Expected: FAIL because `StructLiteral` does not carry a struct ID yet.

Run: `cargo test -p rock-lib lower_enum_variant_records_enum_and_variant_ids -- --exact --nocapture`

Expected: PASS if all enum expression paths already emit `Some(HirVariantLocation)`, or FAIL where a path still emits `None` for a known variant.

- [ ] **Step 3: Update struct literal lowering**

In `lib/src/lower/paths.rs`, change known struct literal emission from:

```rust
kind: HirExprKind::StructLiteral(type_name, fields),
```

to:

```rust
kind: HirExprKind::StructLiteral(type_name, Some(hir_struct.id), fields),
```

Change error-recovery struct literal emission to:

```rust
kind: HirExprKind::StructLiteral(type_name, None, fields),
```

- [ ] **Step 4: Normalize enum expression emission**

In `lib/src/lower/paths.rs`, every known enum variant must emit:

```rust
kind: HirExprKind::EnumVariant(
    enum_name.clone(),
    variant.name.clone(),
    args,
    Some(HirVariantLocation {
        owner: enum_info.id,
        variant_id: variant.id,
        name: variant.name.clone(),
    }),
),
```

Use the display enum name already present in the path for `enum_name`. Keep `None` only for error recovery where the enum or variant is unknown.

- [ ] **Step 5: Run expression reference tests**

Run: `cargo test -p rock-lib lower_struct_literal_records_struct_id -- --exact --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib lower_enum_variant_records_enum_and_variant_ids -- --exact --nocapture`

Expected: PASS.

---

### Task 7: Lower Resolved Struct And Enum Pattern Targets

**Files:**
- Modify: `lib/src/lower/control_flow/pattern.rs`
- Modify: traversal/substitution consumers updated in Task 5

- [ ] **Step 1: Add pattern reference tests**

In `lib/src/lower/control_flow/pattern.rs`, add a `#[cfg(test)] mod tests` if the file does not already have one. Copy the exact `compile_source_to_resolved_hir_for_test` helper from Task 6 into this test module, changing only the atomic name to `LOWER_PATTERN_TEST_COUNTER`. Add these pattern walkers:

```rust
fn find_first_struct_pattern(
    block: &HirBlock,
) -> Option<(&String, &Option<DefId>, &Vec<Type>, &Vec<HirStructPatternField>)> {
    block.stmts.iter().find_map(|stmt| match stmt {
        HirStmt::Expr(expr) | HirStmt::Return(Some(expr)) => find_first_struct_pattern_expr(expr),
        _ => None,
    })
}

fn find_first_struct_pattern_expr(
    expr: &HirExpr,
) -> Option<(&String, &Option<DefId>, &Vec<Type>, &Vec<HirStructPatternField>)> {
    match &expr.kind {
        HirExprKind::Match { arms, .. } => arms.iter().find_map(|arm| match &arm.pattern {
            HirPattern::Struct(name, id, type_args, fields) => {
                Some((name, id, type_args, fields))
            }
            _ => None,
        }),
        HirExprKind::Block(block) => find_first_struct_pattern(block),
        _ => None,
    }
}

fn find_first_enum_pattern(
    block: &HirBlock,
) -> Option<(&String, &String, &Option<HirVariantLocation>, &Vec<HirPattern>)> {
    block.stmts.iter().find_map(|stmt| match stmt {
        HirStmt::Expr(expr) | HirStmt::Return(Some(expr)) => find_first_enum_pattern_expr(expr),
        _ => None,
    })
}

fn find_first_enum_pattern_expr(
    expr: &HirExpr,
) -> Option<(&String, &String, &Option<HirVariantLocation>, &Vec<HirPattern>)> {
    match &expr.kind {
        HirExprKind::Match { arms, .. } => arms.iter().find_map(|arm| match &arm.pattern {
            HirPattern::Enum(enum_name, variant_name, location, items) => {
                Some((enum_name, variant_name, location, items))
            }
            _ => None,
        }),
        HirExprKind::Block(block) => find_first_enum_pattern(block),
        _ => None,
    }
}
```

Add these tests:

```rust
#[test]
fn lower_struct_pattern_records_struct_and_field_ids() {
    let source = r#"
struct Box { value: I64 }

fn read(box: Box) -> I64 {
    match box {
        Box { value } => value
    }
}
"#;

    let resolved = compile_source_to_resolved_hir_for_test(source);
    let (_, read) = resolved.program.function_by_name("read").unwrap();
    let struct_id = resolved.program.names.structs_by_name["Box"];

    let pattern = find_first_struct_pattern(&read.body).unwrap();
    assert_eq!(pattern.1, Some(struct_id));
    assert_eq!(pattern.3[0].field.as_ref().unwrap().owner, struct_id);
    assert_eq!(pattern.3[0].name, "value");
}

#[test]
fn lower_enum_pattern_records_enum_and_variant_ids() {
    let source = r#"
enum Maybe { Some(I64), None }

fn read(value: Maybe) -> I64 {
    match value {
        Some(x) => x,
        None => 0,
    }
}
"#;

    let resolved = compile_source_to_resolved_hir_for_test(source);
    let (_, read) = resolved.program.function_by_name("read").unwrap();
    let enum_id = resolved.program.names.enums_by_name["Maybe"];

    let pattern = find_first_enum_pattern(&read.body).unwrap();
    let location = pattern.2.as_ref().expect("enum pattern must be resolved");
    assert_eq!(location.owner, enum_id);
    assert_eq!(location.name, "Some");
}
```

- [ ] **Step 2: Run the failing pattern reference tests**

Run: `cargo test -p rock-lib lower_struct_pattern_records_struct_and_field_ids -- --exact --nocapture`

Expected: FAIL because `HirPattern::Struct` does not carry a struct ID or field locations yet.

Run: `cargo test -p rock-lib lower_enum_pattern_records_enum_and_variant_ids -- --exact --nocapture`

Expected: FAIL because `HirPattern::Enum` does not carry `HirVariantLocation` yet.

- [ ] **Step 3: Update struct pattern lowering**

In `lib/src/lower/control_flow/pattern.rs`, change known struct pattern emission to:

```rust
let struct_id = self
    .structs
    .get(&type_name)
    .map(|struct_info| struct_info.id);

let field_patterns: Vec<HirStructPatternField> = fields
    .iter()
    .map(|f| {
        let field = self
            .structs
            .get(&type_name)
            .and_then(|struct_info| {
                struct_info.fields.iter().find(|field| field.name == f.name.name).map(|field| {
                    HirFieldLocation {
                        owner: struct_info.id,
                        field_id: field.id,
                        name: field.name.clone(),
                    }
                })
            });
        let t = self
            .lower_struct_field_type(&type_name, &type_args, &f.name.name, &f.name.span)
            .unwrap_or_else(|| self.engine.fresh_type_var());
        let pattern = self.lower_pattern(&f.pattern, &t);
        HirStructPatternField {
            name: f.name.name.clone(),
            field,
            pattern,
        }
    })
    .collect();

HirPattern::Struct(type_name, struct_id, type_args, field_patterns)
```

For unknown/error recovery struct patterns, emit `HirPattern::Struct(type_name, None, vec![], vec![])`.

- [ ] **Step 4: Update enum pattern lowering**

Every known enum pattern branch in `lib/src/lower/control_flow/pattern.rs` must emit:

```rust
HirPattern::Enum(
    enum_name,
    variant_name.clone(),
    Some(HirVariantLocation {
        owner: enum_info.id,
        variant_id: variant.id,
        name: variant_name.clone(),
    }),
    patterns,
)
```

Keep `None` only in error recovery paths that preserve a pattern after an unknown enum or variant diagnostic.

- [ ] **Step 5: Run pattern reference tests**

Run: `cargo test -p rock-lib lower_struct_pattern_records_struct_and_field_ids -- --exact --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib lower_enum_pattern_records_enum_and_variant_ids -- --exact --nocapture`

Expected: PASS.

---

### Task 8: Lower Direct Top-Level Function And Extern References With IDs

**Files:**
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/codegen/**`

- [ ] **Step 1: Add resolved var tests**

In the `lib/src/lower/paths.rs` test module created in Task 6, add this walker:

```rust
fn find_first_resolved_var(block: &HirBlock) -> Option<&HirVarRef> {
    block.stmts.iter().find_map(|stmt| match stmt {
        crate::hir::HirStmt::Expr(expr) | crate::hir::HirStmt::Return(Some(expr)) => {
            find_first_resolved_var_expr(expr)
        }
        _ => None,
    })
}

fn find_first_resolved_var_expr(expr: &HirExpr) -> Option<&HirVarRef> {
    match &expr.kind {
        HirExprKind::ResolvedVar(reference) => Some(reference),
        HirExprKind::Call(callee, args) => find_first_resolved_var_expr(callee)
            .or_else(|| args.iter().find_map(find_first_resolved_var_expr)),
        HirExprKind::Block(block) => find_first_resolved_var(block),
        _ => None,
    }
}
```

Add these tests:

```rust
#[test]
fn lower_direct_function_reference_records_function_id() {
    let source = r#"
fn answer() -> I64 { 7 }

fn main() -> I64 {
    answer()
}
"#;

    let resolved = compile_source_to_resolved_hir_for_test(source);
    let answer_id = resolved.program.names.functions_by_name["answer"];
    let (_, main) = resolved.program.function_by_name("main").unwrap();

    let var = find_first_resolved_var(&main.body).unwrap();
    assert_eq!(var.name, "answer");
    assert_eq!(var.target, HirVarTarget::Function(answer_id));
}

#[test]
fn lower_direct_extern_reference_records_extern_id() {
    let source = r#"
extern puts: (I32) -> I32

fn main() -> I32 {
    puts(0)
}
"#;

    let resolved = compile_source_to_resolved_hir_for_test(source);
    let extern_id = resolved.program.names.externs_by_name["puts"];
    let (_, main) = resolved.program.function_by_name("main").unwrap();

    let var = find_first_resolved_var(&main.body).unwrap();
    assert_eq!(var.name, "puts");
    assert_eq!(var.target, HirVarTarget::Extern(extern_id));
}
```

- [ ] **Step 2: Run the failing resolved var tests**

Run: `cargo test -p rock-lib lower_direct_function_reference_records_function_id -- --exact --nocapture`

Expected: FAIL because direct function references still lower to `HirExprKind::Var(String)`.

Run: `cargo test -p rock-lib lower_direct_extern_reference_records_extern_id -- --exact --nocapture`

Expected: FAIL because direct extern references still lower to `HirExprKind::Var(String)`.

- [ ] **Step 3: Emit `ResolvedVar` for known functions and externs**

In `lib/src/lower/paths.rs`, where a path resolves to a known function, return:

```rust
kind: HirExprKind::ResolvedVar(HirVarRef {
    name: hir_name.clone(),
    target: HirVarTarget::Function(func.id),
}),
```

Where a path resolves to a known extern, return:

```rust
kind: HirExprKind::ResolvedVar(HirVarRef {
    name: extern_name.clone(),
    target: HirVarTarget::Extern(ext.id),
}),
```

Local variables, unknown names, backend symbols introduced by monomorphization, and error recovery may remain `HirExprKind::Var(String)` in this slice.

- [ ] **Step 4: Consume `ResolvedVar` in mono/codegen**

In mono and codegen expression handling, add `HirExprKind::ResolvedVar(reference)` immediately next to the existing `HirExprKind::Var(name)` arm. Reuse the existing variable-reference behavior with `let name = &reference.name;`, and use `reference.target` for any semantic function or extern lookup that already has an ID-keyed table. Any remaining name fallback must include this exact comment:

```rust
// Compatibility until roadmap Task 14 replaces backend-symbol call targets with InstanceId edges.
```

- [ ] **Step 5: Run resolved var tests**

Run: `cargo test -p rock-lib lower_direct_function_reference_records_function_id -- --exact --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib lower_direct_extern_reference_records_extern_id -- --exact --nocapture`

Expected: PASS.

---

### Task 9: Preserve Product Artifact And Dependency Behavior

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add artifact round-trip tests for ID-owned HIR metadata**

In `lib/src/products.rs`, add a focused test near existing product identity tests:

```rust
#[test]
fn product_emission_reads_id_owned_hir_definitions() {
    let function_id = DefId::new(CrateId(0), LocalDefId(0));
    let struct_id = DefId::new(CrateId(0), LocalDefId(1));
    let function = test_function(function_id, "answer", Vec::new());
    let structure = HirStruct {
        id: struct_id,
        name: "Box".to_string(),
        generic_params: Vec::new(),
        fields: Vec::new(),
    };

    let hir = ResolvedHirProgram {
        program: HirProgram::from_parts(
            HashMap::from([("answer".to_string(), function)]),
            HashMap::from([("Box".to_string(), structure)]),
            HashMap::new(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
        ),
        resolver: ResolverTables::default(),
        current_def_ids: BTreeSet::from([function_id, struct_id]),
        root_crate_id: CrateId(0),
        local_def_ids: IdGen::new(),
    };

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("test".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );

    assert!(products.metadata.functions.contains_key(&ProductDefId::from(function_id)));
    assert!(products.metadata.structs.contains_key(&ProductDefId::from(struct_id)));
}
```

- [ ] **Step 2: Run the artifact/product test**

Run: `cargo test -p rock-lib product_emission_reads_id_owned_hir_definitions -- --exact --nocapture`

Expected: PASS after Task 3 migrations; FAIL if product emission still iterates name-owned maps directly.

- [ ] **Step 3: Add an integration same-name smoke test**

In `lib/tests/integration.rs`, add:

```rust
#[test]
fn test_id_owned_hir_preserves_same_name_module_calls() {
    let output = compile_and_run(
        r#"
mod left {
    fn value() -> I64 { 11 }
}

mod right {
    fn value() -> I64 { 31 }
}

fn main() -> I64 {
    left::value() + right::value()
}
"#,
    );

    assert_eq!(output.trim(), "42");
}
```

- [ ] **Step 4: Run the integration smoke test**

Run: `cargo test -p rock-lib --test integration test_id_owned_hir_preserves_same_name_module_calls -- --exact --nocapture`

Expected: PASS.

---

### Task 10: Remove Supported-Path String Ownership Assumptions

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: all files with remaining direct name-owned HIR assumptions from search results

- [ ] **Step 1: Search for remaining direct HIR ownership assumptions**

Run: `rg "program\.(functions|structs|enums|traits|impls|externs)" lib/src lib/tests`

Expected: remaining matches are either ID-keyed map access, tests intentionally constructing HIR through `from_parts`, or code that will be migrated in this task.

- [ ] **Step 2: Replace remaining source-name semantic ownership paths**

For each production match that still treats `program.functions`, `program.structs`, `program.enums`, `program.traits`, or `program.externs` as name-owned, replace it with one of these access patterns:

```rust
program.function_by_name(name)
program.struct_by_name(name)
program.enum_by_name(name)
program.trait_by_name(name)
program.extern_by_name(name)
program.functions_by_id()
program.structs_by_id()
program.enums_by_id()
program.traits_by_id()
program.impls_in_order()
program.externs_in_order()
```

Do not add a new `HashMap<String, HirFunction>` owner to make a consumer compile.

- [ ] **Step 3: Add a guard test for name tables as views**

In `lib/src/hir/mod.rs`, add:

```rust
#[test]
fn hir_name_tables_do_not_own_semantic_bodies() {
    let id = def_id(30);
    let program = HirProgram::from_parts(
        HashMap::from([("answer".to_string(), test_function(id, "answer"))]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );

    let (by_name_id, by_name) = program.function_by_name("answer").unwrap();
    let (_, by_id) = program.function_by_id(id).unwrap();

    assert_eq!(by_name_id, id);
    assert!(std::ptr::eq(by_name, by_id));
}
```

- [ ] **Step 4: Run the guard test and compile check**

Run: `cargo test -p rock-lib hir_name_tables_do_not_own_semantic_bodies -- --exact --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib --no-run`

Expected: PASS.

---

### Task 11: Update The Audit Tracker

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` only if implementation changes the planned ordering

- [ ] **Step 1: Update completed checklist bullets**

In `docs/superpowers/plans/master-audit-checklist.md`, move or add completed bullets under `Done:` for:

```markdown
- [x] Made collection/index allocation the only authority for supported current-crate top-level, impl, method, field, variant, associated-type, and extern IDs before HIR body lowering.
- [x] Migrated `HirProgram` ownership storage from string-keyed maps to authoritative ID-keyed tables, with names retained as diagnostics/display metadata.
- [x] Converted high-value HIR semantic references for method locations, struct literals/patterns, enum variants/patterns, and direct top-level function/extern references to carry canonical IDs where lowering resolves them.
```

Keep remaining unchecked work for alias interfaces, full local/reference IDs, type context, selection service, mono instance call edges, MIR/codegen, and borrowck.

- [ ] **Step 2: Verify no checked item remains under `Still to do:`**

Run: `perl -ne 'if (/^Still to do:/) { $in=1; next } if (/^## /) { $in=0 } if ($in && /^- \[x\]/) { print "$ARGV:$.:$_"; $bad=1 } END { exit($bad ? 1 : 0) }' "docs/superpowers/plans/master-audit-checklist.md"`

Expected: no output and exit 0.

---

### Task 12: Final Verification

**Files:**
- Verify: all modified files

- [ ] **Step 1: Format check**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 2: Diff whitespace check**

Run: `git diff --check`

Expected: no output.

- [ ] **Step 3: Focused HIR and collect tests**

Run: `cargo test -p rock-lib collect_assigns_supported_current_crate_ids_before_lowering hir_program_owns_functions_by_def_id_and_names_are_views hir_method_locations_use_owner_ids_not_names_or_indexes hir_name_tables_do_not_own_semantic_bodies -- --nocapture`

Expected: PASS for all named tests.

- [ ] **Step 4: Focused reference tests**

Run: `cargo test -p rock-lib lower_struct_literal_records_struct_id lower_enum_variant_records_enum_and_variant_ids lower_struct_pattern_records_struct_and_field_ids lower_enum_pattern_records_enum_and_variant_ids lower_direct_function_reference_records_function_id lower_direct_extern_reference_records_extern_id -- --nocapture`

Expected: PASS for all named tests.

- [ ] **Step 5: Integration smoke test**

Run: `cargo test -p rock-lib --test integration test_id_owned_hir_preserves_same_name_module_calls -- --exact --nocapture`

Expected: PASS.

- [ ] **Step 6: Full library test suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 7: Final status check**

Run: `git status --short`

Expected: only intentional source, test, and docs changes are shown. `.sisyphus/` may remain untracked and must remain untouched.

---

## Completion Definition

This plan is complete when:

- Supported current-crate semantic IDs are allocated by collection/indexing before HIR body lowering.
- `HirProgram` owns core definitions by canonical ID.
- Source-name access to HIR is a view over ID-owned storage.
- Method locations, struct literals/patterns, enum variants/patterns, and direct top-level function/extern references carry canonical IDs when lowering resolves them.
- Product/artifact emission and loading work with ID-owned HIR storage.
- `master-audit-checklist.md` accurately reflects completed and remaining audit work.
- All final verification commands in Task 12 pass.
