# HIR ID-Keyed Definition Indexes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add canonical ID-keyed indexes to resolved HIR, including top-level definitions, impls, externs, trait defaults, and impl methods.

**Architecture:** Keep existing string-keyed HIR ownership tables in place and add a derived `HirDefinitionIndexes` companion on `HirProgram`. Build indexes from final HIR containers, using resolver canonical names when available so import/export aliases point back to the canonical HIR entry instead of becoming separate index entries.

**Tech Stack:** Rust 2021, `serde`, existing `DefId`/`LocalDefId` identity types, existing collect -> lower -> infer -> mono pipeline, `cargo test -p rock-lib`.

---

## File Structure

- Modify `lib/src/hir/mod.rs`: define `HirDefinitionIndexes`, `HirMethodLocation`, `HirProgram::from_parts`, `HirProgram::from_parts_with_canonical_names`, `HirProgram::rebuild_indexes`, `HirProgram::rebuild_indexes_with_canonical_names`, and unit tests for pure index behavior.
- Modify `lib/src/infer/mod.rs`: construct resolved HIR through the new `HirProgram` helpers and pass `resolver.item_names_by_id` so aliases resolve to canonical keys.
- Modify `lib/src/collect/mod.rs`: allocate unique local `DefId` values for trait default methods and impl methods, and add collection/pipeline tests for method IDs and alias-backed HIR indexes.
- Modify `lib/src/mono/process.rs`: rebuild indexes before returning a monomorphized `HirProgram` after functions and methods have been moved and rewritten.
- Modify `lib/src/mono/external.rs`: rebuild indexes before returning from `process_with_crates` and update direct test construction.
- Modify `lib/src/products.rs`: replace direct `HirProgram` struct literals in tests with `HirProgram::from_parts` and adjust colliding method-ID fixtures to use distinct method IDs.
- Modify `lib/src/mir/builder/mod.rs`: replace direct `HirProgram` struct literals in tests/helpers with `HirProgram::from_parts`.
- Modify `lib/src/codegen/mod.rs`: update the embedded compile-check fixture to construct `HirProgram` through `HirProgram::from_parts`.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: update the checked commit and mark the HIR ID-index subtask as in progress or complete after implementation verification.

---

### Task 1: Add HIR Index Types And Pure Builder Tests

**Files:**
- Modify: `lib/src/hir/mod.rs`

- [ ] **Step 1: Write failing HIR index tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `lib/src/hir/mod.rs`. Extend the current imports to include the HIR types used below.

```rust
use std::collections::HashMap;

use crate::hir::{
    HirBlock, HirDefinitionIndexes, HirEnum, HirExpr, HirExprKind, HirField, HirFunction,
    HirImpl, HirImplOwner, HirMethodLocation, HirParam, HirProgram, HirStmt, HirStruct,
    HirTrait, HirVariant,
};
```

Add the helper functions and tests:

```rust
fn empty_body() -> HirBlock {
    HirBlock {
        stmts: vec![HirStmt::Expr(HirExpr {
            kind: HirExprKind::Unit,
            ty: Type::Unit,
            span: crate::lexer::Span::default(),
        })],
        ty: Type::Unit,
    }
}

fn test_function(id: DefId, name: &str) -> HirFunction {
    HirFunction {
        id,
        name: name.to_string(),
        qualified_name: None,
        generic_params: Vec::new(),
        generic_bounds: HashMap::new(),
        params: Vec::new(),
        ret_type: Type::Unit,
        body: empty_body(),
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    }
}

#[test]
fn hir_program_indexes_definitions_by_def_id() {
    let function_id = DefId::new(CrateId(0), LocalDefId(0));
    let struct_id = DefId::new(CrateId(0), LocalDefId(1));
    let enum_id = DefId::new(CrateId(0), LocalDefId(2));
    let trait_id = DefId::new(CrateId(0), LocalDefId(3));
    let impl_id = DefId::new(CrateId(0), LocalDefId(4));
    let extern_id = DefId::new(CrateId(0), LocalDefId(5));
    let trait_method_id = DefId::new(CrateId(0), LocalDefId(6));
    let impl_method_id = DefId::new(CrateId(0), LocalDefId(7));

    let functions = HashMap::from([(
        "answer".to_string(),
        test_function(function_id, "answer"),
    )]);
    let structs = HashMap::from([(
        "Box".to_string(),
        HirStruct {
            id: struct_id,
            name: "Box".to_string(),
            generic_params: Vec::new(),
            fields: vec![HirField {
                name: "value".to_string(),
                ty: Type::I64,
                public: true,
            }],
        },
    )]);
    let enums = HashMap::from([(
        "Maybe".to_string(),
        HirEnum {
            id: enum_id,
            name: "Maybe".to_string(),
            generic_params: Vec::new(),
            variants: vec![HirVariant {
                name: "None".to_string(),
                fields: crate::hir::HirVariantFields::Unit,
            }],
        },
    )]);
    let traits = HashMap::from([(
        "Show".to_string(),
        HirTrait {
            id: trait_id,
            name: "Show".to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods: HashMap::from([(
                "show".to_string(),
                test_function(trait_method_id, "show"),
            )]),
            signatures: HashMap::new(),
        },
    )]);
    let impls = vec![HirImpl {
        id: impl_id,
        owner: HirImplOwner::Named("Box".to_string()),
        type_name: "Box".to_string(),
        type_generics: Vec::new(),
        receiver_arg_types: Vec::new(),
        trait_name: Some("Show".to_string()),
        trait_generics: Vec::new(),
        trait_arg_types: Vec::new(),
        associated_types: Vec::new(),
        bounds: Vec::new(),
        methods: HashMap::from([(
            "show".to_string(),
            test_function(impl_method_id, "show"),
        )]),
    }];
    let externs = vec![HirExtern {
        id: extern_id,
        name: "puts".to_string(),
        params: vec![Type::Pointer(Box::new(Type::U8))],
        ret: Type::I32,
        variadic: false,
    }];

    let program = HirProgram::from_parts(functions, structs, enums, traits, impls, externs);

    assert_eq!(
        program.indexes.functions_by_id.get(&function_id).map(String::as_str),
        Some("answer")
    );
    assert_eq!(
        program.indexes.structs_by_id.get(&struct_id).map(String::as_str),
        Some("Box")
    );
    assert_eq!(
        program.indexes.enums_by_id.get(&enum_id).map(String::as_str),
        Some("Maybe")
    );
    assert_eq!(
        program.indexes.traits_by_id.get(&trait_id).map(String::as_str),
        Some("Show")
    );
    assert_eq!(program.indexes.impls_by_id.get(&impl_id), Some(&0));
    assert_eq!(program.indexes.externs_by_id.get(&extern_id), Some(&0));
    assert_eq!(
        program.indexes.methods_by_id.get(&trait_method_id),
        Some(&HirMethodLocation::TraitDefault {
            trait_name: "Show".to_string(),
            method_name: "show".to_string(),
        })
    );
    assert_eq!(
        program.indexes.methods_by_id.get(&impl_method_id),
        Some(&HirMethodLocation::ImplMethod {
            impl_index: 0,
            method_name: "show".to_string(),
        })
    );
}

#[test]
#[should_panic(expected = "duplicate HIR function DefId")]
fn hir_program_rejects_duplicate_function_ids_without_canonical_names() {
    let id = DefId::new(CrateId(0), LocalDefId(0));
    let functions = HashMap::from([
        ("first".to_string(), test_function(id, "first")),
        ("second".to_string(), test_function(id, "second")),
    ]);

    let _ = HirProgram::from_parts(
        functions,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
}

#[test]
fn hir_program_uses_canonical_names_to_select_alias_duplicates() {
    let id = DefId::new(CrateId(0), LocalDefId(0));
    let functions = HashMap::from([
        ("alias".to_string(), test_function(id, "answer")),
        ("module::answer".to_string(), test_function(id, "answer")),
    ]);
    let canonical_names = HashMap::from([(id, "module::answer".to_string())]);

    let program = HirProgram::from_parts_with_canonical_names(
        functions,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
        Vec::new(),
        &canonical_names,
    );

    assert_eq!(
        program.indexes.functions_by_id.get(&id).map(String::as_str),
        Some("module::answer")
    );
}
```

- [ ] **Step 2: Run the new HIR tests to verify failure**

Run: `cargo test -p rock-lib hir_program_indexes_definitions_by_def_id`

Expected: FAIL with compile errors for missing `HirDefinitionIndexes`, `HirMethodLocation`, and `HirProgram::from_parts`.

- [ ] **Step 3: Implement the index model and builders**

In `lib/src/hir/mod.rs`, insert the new types before `HirProgram` and add the `indexes` field with a serde default:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirMethodLocation {
    TraitDefault { trait_name: String, method_name: String },
    ImplMethod { impl_index: usize, method_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HirDefinitionIndexes {
    pub functions_by_id: HashMap<DefId, String>,
    pub structs_by_id: HashMap<DefId, String>,
    pub enums_by_id: HashMap<DefId, String>,
    pub traits_by_id: HashMap<DefId, String>,
    pub impls_by_id: HashMap<DefId, usize>,
    pub externs_by_id: HashMap<DefId, usize>,
    pub methods_by_id: HashMap<DefId, HirMethodLocation>,
}

/// A typed HIR program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub functions: HashMap<String, HirFunction>,
    pub structs: HashMap<String, HirStruct>,
    pub enums: HashMap<String, HirEnum>,
    pub traits: HashMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    #[serde(default)]
    pub indexes: HirDefinitionIndexes,
}
```

Add these impl blocks after `default_def_id()`:

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
        let indexes = HirDefinitionIndexes::from_parts(
            &functions,
            &structs,
            &enums,
            &traits,
            &impls,
            &externs,
            canonical_names_by_id,
        );

        Self {
            functions,
            structs,
            enums,
            traits,
            impls,
            externs,
            indexes,
        }
    }

    pub fn rebuild_indexes(&mut self) {
        self.rebuild_indexes_with_canonical_names(&HashMap::new());
    }

    pub fn rebuild_indexes_with_canonical_names(
        &mut self,
        canonical_names_by_id: &HashMap<DefId, String>,
    ) {
        self.indexes = HirDefinitionIndexes::from_parts(
            &self.functions,
            &self.structs,
            &self.enums,
            &self.traits,
            &self.impls,
            &self.externs,
            canonical_names_by_id,
        );
    }
}

impl HirDefinitionIndexes {
    fn from_parts(
        functions: &HashMap<String, HirFunction>,
        structs: &HashMap<String, HirStruct>,
        enums: &HashMap<String, HirEnum>,
        traits: &HashMap<String, HirTrait>,
        impls: &[HirImpl],
        externs: &[HirExtern],
        canonical_names_by_id: &HashMap<DefId, String>,
    ) -> Self {
        let mut indexes = Self::default();

        index_named_definitions(
            &mut indexes.functions_by_id,
            functions.iter().map(|(name, item)| (item.id, name.as_str())),
            canonical_names_by_id,
            "function",
        );
        index_named_definitions(
            &mut indexes.structs_by_id,
            structs.iter().map(|(name, item)| (item.id, name.as_str())),
            canonical_names_by_id,
            "struct",
        );
        index_named_definitions(
            &mut indexes.enums_by_id,
            enums.iter().map(|(name, item)| (item.id, name.as_str())),
            canonical_names_by_id,
            "enum",
        );
        index_named_definitions(
            &mut indexes.traits_by_id,
            traits.iter().map(|(name, item)| (item.id, name.as_str())),
            canonical_names_by_id,
            "trait",
        );

        for (index, imp) in impls.iter().enumerate() {
            insert_unique(&mut indexes.impls_by_id, imp.id, index, "impl");
            for (method_name, method) in &imp.methods {
                insert_unique(
                    &mut indexes.methods_by_id,
                    method.id,
                    HirMethodLocation::ImplMethod {
                        impl_index: index,
                        method_name: method_name.clone(),
                    },
                    "method",
                );
            }
        }

        for (index, ext) in externs.iter().enumerate() {
            insert_unique(&mut indexes.externs_by_id, ext.id, index, "extern");
        }

        for (trait_name, trait_def) in traits {
            for (method_name, method) in &trait_def.methods {
                insert_unique(
                    &mut indexes.methods_by_id,
                    method.id,
                    HirMethodLocation::TraitDefault {
                        trait_name: trait_name.clone(),
                        method_name: method_name.clone(),
                    },
                    "method",
                );
            }
        }

        indexes
    }
}

fn index_named_definitions<'a>(
    output: &mut HashMap<DefId, String>,
    entries: impl Iterator<Item = (DefId, &'a str)>,
    canonical_names_by_id: &HashMap<DefId, String>,
    kind: &str,
) {
    let mut grouped: HashMap<DefId, Vec<String>> = HashMap::new();
    for (id, name) in entries {
        grouped.entry(id).or_default().push(name.to_string());
    }

    for (id, mut names) in grouped {
        names.sort();
        names.dedup();

        let selected = if let Some(canonical_name) = canonical_names_by_id.get(&id) {
            if names.iter().any(|name| name == canonical_name) {
                canonical_name.clone()
            } else if names.len() == 1 {
                names[0].clone()
            } else {
                panic!(
                    "canonical HIR {} name '{}' missing for DefId {:?}; candidates: {:?}",
                    kind, canonical_name, id, names
                );
            }
        } else if names.len() == 1 {
            names[0].clone()
        } else {
            panic!("duplicate HIR {} DefId {:?}: {:?}", kind, id, names);
        };

        insert_unique(output, id, selected, kind);
    }
}

fn insert_unique<T>(map: &mut HashMap<DefId, T>, id: DefId, value: T, kind: &str) {
    assert!(
        map.insert(id, value).is_none(),
        "duplicate HIR {} DefId: {:?}",
        kind,
        id
    );
}
```

- [ ] **Step 4: Run HIR index tests to verify pass**

Run: `cargo test -p rock-lib hir_program_indexes_definitions_by_def_id`

Expected: PASS.

Run: `cargo test -p rock-lib hir_program_rejects_duplicate_function_ids_without_canonical_names`

Expected: PASS.

Run: `cargo test -p rock-lib hir_program_uses_canonical_names_to_select_alias_duplicates`

Expected: PASS.

- [ ] **Step 5: Commit Task 1**

```bash
git add lib/src/hir/mod.rs
git commit -m "hir: add ID-keyed definition indexes"
```

---

### Task 2: Route HIR Construction Through Index Builders

**Files:**
- Modify: `lib/src/infer/mod.rs`
- Modify: `lib/src/products.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Run the workspace compile check to expose direct constructor failures**

Run: `cargo test -p rock-lib finalize_preserves_resolver_tables`

Expected: FAIL with missing `indexes` field on direct `HirProgram` construction sites.

- [ ] **Step 2: Update inference finalization construction**

In `lib/src/infer/mod.rs`, replace both `program: HirProgram { ... }` blocks in `finalize_lenient` and `finalize` with `HirProgram::from_parts_with_canonical_names`. Use the resolver before moving it into `ResolvedHirProgram`:

```rust
let program = HirProgram::from_parts_with_canonical_names(
    hir.functions,
    hir.structs,
    hir.enums,
    hir.traits,
    hir.impls,
    hir.externs,
    &hir.resolver.item_names_by_id,
);

Ok(ResolvedHirProgram {
    program,
    resolver: hir.resolver,
    root_crate_id: hir.root_crate_id,
    local_def_ids: hir.local_def_ids,
})
```

Apply the same shape to `finalize_lenient`.

- [ ] **Step 3: Update direct test constructors**

Replace direct `HirProgram { functions, structs, enums, traits, impls, externs }` literals with `HirProgram::from_parts(functions, structs, enums, traits, impls, externs)` in these files:

```rust
// lib/src/products.rs
program: HirProgram::from_parts(functions, structs, enums, traits, impls, externs),

// lib/src/products.rs for empty constructors
program: HirProgram::from_parts(
    HashMap::new(),
    HashMap::new(),
    HashMap::new(),
    traits,
    Vec::new(),
    Vec::new(),
),

// lib/src/mono/external.rs
crate::hir::HirProgram::from_parts(
    HashMap::new(),
    HashMap::new(),
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![],
)
```

In `lib/src/codegen/mod.rs`, update the embedded Rust fixture string so it constructs through the helper:

```rust
program: HirProgram::from_parts(
    HashMap::new(),
    HashMap::new(),
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![],
),
```

In `lib/src/mir/builder/mod.rs`, update each test helper or direct literal to call `HirProgram::from_parts(...)` with the same six containers it already builds.

- [ ] **Step 4: Fix colliding product test method IDs**

In `lib/src/products.rs`, update `resolved_hir_with_traits_in_order` so each generated trait default method has a distinct `DefId`:

```rust
for (index, name) in names.iter().enumerate() {
    let method_id = DefId::new(CrateId(0), LocalDefId(100 + index as u32));
    let mut methods = HashMap::new();
    methods.insert(
        "show".to_string(),
        test_function(method_id, "show", Vec::new()),
    );
    traits.insert(
        (*name).to_string(),
        HirTrait {
            id: DefId::new(CrateId(0), LocalDefId(index as u32)),
            name: (*name).to_string(),
            generic_params: Vec::new(),
            associated_types: Vec::new(),
            methods,
            signatures: HashMap::new(),
        },
    );
}
```

For the test named `compiler_products_preserve_trait_defaults_with_colliding_method_ids`, change the assertion purpose to stable ordering with distinct method IDs. Rename it to `compiler_products_preserve_trait_defaults_with_distinct_method_ids` and assert both distinct IDs are present.

- [ ] **Step 5: Run focused constructor tests**

Run: `cargo test -p rock-lib finalize_preserves_resolver_tables`

Expected: PASS.

Run: `cargo test -p rock-lib compiler_products_key_metadata_by_product_def_id`

Expected: PASS.

Run: `cargo test -p rock-lib process_with_crates_records_object_backed_instances_without_re_emitting`

Expected: PASS.

- [ ] **Step 6: Commit Task 2**

```bash
git add lib/src/infer/mod.rs lib/src/products.rs lib/src/mono/external.rs lib/src/mir/builder/mod.rs lib/src/codegen/mod.rs
git commit -m "hir: construct programs with definition indexes"
```

---

### Task 3: Allocate Canonical Method DefIds During Collection

**Files:**
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Write failing collection test for method IDs**

Add this test to `lib/src/collect/mod.rs` inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn collect_assigns_unique_def_ids_to_local_trait_and_impl_methods() {
    let program = Program {
        module: Module {
            name: None,
            top_levels: vec![
                TopLevel::StructDecl(StructDecl {
                    name: type_inner("Box"),
                    fields: vec![],
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
            ],
            is_inline: false,
            filepath: None,
        },
    };

    let decls = collect(&program, &CrateContext::new(), false, Some("test"))
        .expect("collect should assign method DefIds");

    let trait_method_id = decls.traits["Show"].methods["show"].id;
    let impl_method_id = decls.impls[0].methods["show"].id;

    assert_ne!(trait_method_id, decls.traits["Show"].id);
    assert_ne!(impl_method_id, decls.impls[0].id);
    assert_ne!(trait_method_id, impl_method_id);
    assert!(!decls.resolver.item_names_by_id.contains_key(&trait_method_id));
    assert!(!decls.resolver.item_names_by_id.contains_key(&impl_method_id));
    assert_eq!(
        decls
            .methods
            .get(&("Box".to_string(), "show".to_string()))
            .map(|method| method.id),
        Some(impl_method_id)
    );
}
```

- [ ] **Step 2: Run the method-ID test to verify failure**

Run: `cargo test -p rock-lib collect_assigns_unique_def_ids_to_local_trait_and_impl_methods`

Expected: FAIL because trait and impl method IDs are still placeholder/default IDs or collide.

- [ ] **Step 3: Implement local method ID assignment**

In `lib/src/collect/mod.rs`, add `BTreeSet` tracking for non-local trait keys before local declarations are collected.

In `collect`, after prelude injection and before `dependency_impls_len`, add:

```rust
let protected_trait_keys: BTreeSet<String> = context.traits.keys().cloned().collect();
let dependency_impls_len = context.impls.len();
```

After `apply_canonical_impl_item_ids(...)`, call:

```rust
assign_canonical_method_ids(
    &mut indexing_ids,
    &protected_trait_keys,
    dependency_impls_len,
    &mut traits,
    &mut impls,
    &mut methods,
);
```

In `collect_artifact_declarations`, keep `artifact_bootstrap` unchanged, then after optional prelude injection and before `context.collect_crate_declarations(...)`, add:

```rust
let protected_trait_keys: BTreeSet<String> = context.traits.keys().cloned().collect();
let protected_impls_len = context.impls.len();
```

After `apply_canonical_impl_item_ids(...)`, call:

```rust
assign_canonical_method_ids(
    &mut indexing_ids,
    &protected_trait_keys,
    protected_impls_len,
    &mut traits,
    &mut impls,
    &mut methods,
);
```

Add this helper near the existing canonical ID helpers:

```rust
fn assign_canonical_method_ids(
    indexing_ids: &mut IndexingIds,
    protected_trait_keys: &BTreeSet<String>,
    first_local_impl_index: usize,
    traits: &mut HashMap<String, HirTrait>,
    impls: &mut [HirImpl],
    methods: &mut HashMap<(String, String), HirFunction>,
) {
    for (trait_key, trait_def) in traits.iter_mut() {
        if protected_trait_keys.contains(trait_key) {
            continue;
        }

        for method in trait_def.methods.values_mut() {
            method.id = indexing_ids.fresh_def_id();
        }
    }

    for imp in impls.iter_mut().skip(first_local_impl_index) {
        for (method_name, method) in imp.methods.iter_mut() {
            method.id = indexing_ids.fresh_def_id();
            methods.insert((imp.type_name.clone(), method_name.clone()), method.clone());
        }
    }
}
```

- [ ] **Step 4: Run the method-ID test to verify pass**

Run: `cargo test -p rock-lib collect_assigns_unique_def_ids_to_local_trait_and_impl_methods`

Expected: PASS.

- [ ] **Step 5: Commit Task 3**

```bash
git add lib/src/collect/mod.rs
git commit -m "collect: assign canonical DefIds to methods"
```

---

### Task 4: Add Pipeline Tests For Alias And Method Indexes

**Files:**
- Modify: `lib/src/collect/mod.rs`

- [ ] **Step 1: Write failing pipeline test for alias-backed indexes**

Add this test to `lib/src/collect/mod.rs` inside the existing test module:

```rust
#[test]
fn lowered_hir_indexes_import_and_export_aliases_by_canonical_def_id() {
    let temp_dir = std::env::temp_dir().join(format!(
        "rock_hir_index_aliases_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let root_path = temp_dir.join("main.rk");
    let math_path = temp_dir.join("math.rk");
    let io_path = temp_dir.join("io.rk");
    std::fs::write(&math_path, "mod io\n< io::writer_name\n< io::Writer\n").unwrap();
    std::fs::write(&io_path, "writer_name: I64\nwriter_name = -> 0\nstruct Writer\n").unwrap();

    let program = Program {
        module: Module {
            name: None,
            top_levels: vec![
                TopLevel::Mod(ident("math"), false),
                TopLevel::Import(crate::ast::Path::Ident(crate::ast::IdentifierPath {
                    path: vec![
                        crate::ast::IdentOrType::Ident(ident("math")),
                        crate::ast::IdentOrType::Ident(ident("writer_name")),
                    ],
                })),
                TopLevel::Import(crate::ast::Path::Type(crate::ast::TypePath {
                    path: vec![
                        crate::ast::IdentOrType::Ident(ident("math")),
                        crate::ast::IdentOrType::Type(crate::ast::ParseType::Type(type_inner(
                            "Writer",
                        ))),
                    ],
                })),
            ],
            is_inline: false,
            filepath: Some(root_path),
        },
    };

    let decls = collect(&program, &CrateContext::new(), false, Some("test"))
        .expect("collect should resolve aliases");
    let partial = crate::lower::program::lower_from_declarations(
        &program,
        decls,
        &CrateContext::new(),
        Some("test"),
    )
    .expect("lowering should preserve canonical alias IDs");
    let resolved = crate::infer::finalize(partial).expect("inference should finalize HIR");

    let canonical_function_id = *resolved
        .resolver
        .item_paths
        .get("test::math::io::writer_name")
        .expect("canonical function path should resolve");
    let export_function_id = *resolved
        .resolver
        .export_aliases
        .get("test::math::writer_name")
        .expect("export alias should resolve");
    let import_function_id = *resolved
        .resolver
        .import_aliases
        .get("writer_name")
        .expect("import alias should resolve");

    assert_eq!(export_function_id, canonical_function_id);
    assert_eq!(import_function_id, canonical_function_id);
    assert_eq!(
        resolved
            .program
            .indexes
            .functions_by_id
            .get(&canonical_function_id)
            .map(String::as_str),
        Some("test::math::io::writer_name")
    );

    let canonical_struct_id = *resolved
        .resolver
        .item_paths
        .get("test::math::io::Writer")
        .expect("canonical struct path should resolve");
    let export_struct_id = *resolved
        .resolver
        .export_aliases
        .get("test::math::Writer")
        .expect("export type alias should resolve");
    let import_struct_id = *resolved
        .resolver
        .import_aliases
        .get("Writer")
        .expect("import type alias should resolve");

    assert_eq!(export_struct_id, canonical_struct_id);
    assert_eq!(import_struct_id, canonical_struct_id);
    assert_eq!(
        resolved
            .program
            .indexes
            .structs_by_id
            .get(&canonical_struct_id)
            .map(String::as_str),
        Some("test::math::io::Writer")
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

- [ ] **Step 2: Write failing pipeline test for method indexes**

Add this test to the same test module:

```rust
#[test]
fn lowered_hir_indexes_trait_defaults_and_impl_methods_by_def_id() {
    let program = Program {
        module: Module {
            name: None,
            top_levels: vec![
                TopLevel::StructDecl(StructDecl {
                    name: type_inner("Box"),
                    fields: vec![],
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
            ],
            is_inline: false,
            filepath: None,
        },
    };

    let decls = collect(&program, &CrateContext::new(), false, Some("test"))
        .expect("collect should assign method IDs");
    let partial = crate::lower::program::lower_from_declarations(
        &program,
        decls,
        &CrateContext::new(),
        Some("test"),
    )
    .expect("lowering should preserve method IDs");
    let resolved = crate::infer::finalize(partial).expect("inference should finalize HIR");

    let trait_method_id = resolved.program.traits["Show"].methods["show"].id;
    let impl_method_id = resolved.program.impls[0].methods["show"].id;

    assert_eq!(
        resolved.program.indexes.methods_by_id.get(&trait_method_id),
        Some(&crate::hir::HirMethodLocation::TraitDefault {
            trait_name: "Show".to_string(),
            method_name: "show".to_string(),
        })
    );
    assert_eq!(
        resolved.program.indexes.methods_by_id.get(&impl_method_id),
        Some(&crate::hir::HirMethodLocation::ImplMethod {
            impl_index: 0,
            method_name: "show".to_string(),
        })
    );
}
```

- [ ] **Step 3: Run the new pipeline tests to verify failure or pass state**

Run: `cargo test -p rock-lib lowered_hir_indexes_import_and_export_aliases_by_canonical_def_id`

Expected before Task 1 and Task 2 code: FAIL. Expected after Tasks 1-3: PASS.

Run: `cargo test -p rock-lib lowered_hir_indexes_trait_defaults_and_impl_methods_by_def_id`

Expected before Task 3 code: FAIL. Expected after Tasks 1-3: PASS.

- [ ] **Step 4: Fix any narrow test fallout**

If the alias test shows a canonical HIR key is missing from the existing HIR maps, adjust only the index selection code in `lib/src/hir/mod.rs` to panic with the existing candidate names. Do not add alias entries to indexes. Do not change resolver semantics.

- [ ] **Step 5: Run focused pipeline tests to verify pass**

Run: `cargo test -p rock-lib lowered_hir_indexes_import_and_export_aliases_by_canonical_def_id`

Expected: PASS.

Run: `cargo test -p rock-lib lowered_hir_indexes_trait_defaults_and_impl_methods_by_def_id`

Expected: PASS.

- [ ] **Step 6: Commit Task 4**

```bash
git add lib/src/collect/mod.rs lib/src/hir/mod.rs
git commit -m "hir: index canonical aliases and methods by DefId"
```

---

### Task 5: Keep Monomorphized Program Indexes Fresh

**Files:**
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`

- [ ] **Step 1: Add index rebuild calls after mono rewrites HIR**

In `lib/src/mono/process.rs`, update the end of `Monomorphizer::process`:

```rust
program.functions = std::mem::take(&mut self.concrete_functions);
program.rebuild_indexes();

program
```

In `lib/src/mono/external.rs`, update `process_with_crates`:

```rust
pub(super) fn process_with_crates(
    &mut self,
    mut program: crate::hir::HirProgram,
    crate_ctx: &CrateContext,
) -> crate::hir::HirProgram {
    self.process_with_crates_impl(&mut program, crate_ctx);
    program.rebuild_indexes_with_canonical_names(&self.resolver.item_names_by_id);
    program
}
```

- [ ] **Step 2: Run focused mono tests**

Run: `cargo test -p rock-lib named_impl_method_origin_uses_impl_def_id_not_type_def_id`

Expected: PASS.

Run: `cargo test -p rock-lib process_with_crates_records_object_backed_instances_without_re_emitting`

Expected: PASS.

- [ ] **Step 3: Commit Task 5**

```bash
git add lib/src/mono/process.rs lib/src/mono/external.rs
git commit -m "mono: rebuild HIR definition indexes"
```

---

### Task 6: Update Audit Checklist And Run Full Verification

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Update checklist status**

In `docs/superpowers/plans/master-audit-checklist.md`, update `Checked against HEAD` to the implementation commit produced by the previous task. In section `1. Identity And Arenas`, change this item:

```markdown
- [ ] Replace string-keyed `HirProgram` maps with ID-keyed tables or equivalent canonical indexes.
```

to:

```markdown
- [x] Add canonical ID-keyed `HirProgram` indexes for top-level definitions, impls, externs, trait defaults, and impl methods while preserving string-keyed ownership maps for compatibility.
```

Keep the broader status as `In progress` because string-keyed maps still own the HIR data and semantic `Type` remains name-bearing.

- [ ] **Step 2: Run formatting check**

Run: `cargo fmt --all --check`

Expected: PASS with no formatted diff required.

If it fails, run `cargo fmt --all`, inspect the formatted diff, and rerun `cargo fmt --all --check`.

- [ ] **Step 3: Run focused tests**

Run: `cargo test -p rock-lib hir_program_indexes_definitions_by_def_id`

Expected: PASS.

Run: `cargo test -p rock-lib collect_assigns_unique_def_ids_to_local_trait_and_impl_methods`

Expected: PASS.

Run: `cargo test -p rock-lib lowered_hir_indexes_import_and_export_aliases_by_canonical_def_id`

Expected: PASS.

Run: `cargo test -p rock-lib lowered_hir_indexes_trait_defaults_and_impl_methods_by_def_id`

Expected: PASS.

- [ ] **Step 4: Run full library tests**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 5: Commit Task 6**

```bash
git add docs/superpowers/plans/master-audit-checklist.md
git commit -m "docs: update HIR index audit status"
```

---

## Self-Review Checklist

- Spec coverage: Tasks 1-2 add HIR ID indexes, Task 3 includes methods, Task 4 validates aliases, Task 5 keeps mono output fresh, and Task 6 updates the audit checklist.
- Scope control: The plan does not remove string-keyed HIR maps, migrate semantic `Type`, add field/variant/associated-type IDs, or rewrite codegen symbol naming.
- Type consistency: The plan consistently uses `DefId`, `HirDefinitionIndexes`, `HirMethodLocation`, `HirProgram::from_parts`, `HirProgram::from_parts_with_canonical_names`, `HirProgram::rebuild_indexes`, and `HirProgram::rebuild_indexes_with_canonical_names`.
- Verification: Each implementation task has focused tests, and the final task runs `cargo fmt --all --check` plus `cargo test -p rock-lib`.
