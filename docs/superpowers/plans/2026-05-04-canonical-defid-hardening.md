# Canonical DefId Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the supported `collect -> lower_from_declarations -> mono` pipeline use only canonical identity for named items, while replacing mono's builtin-slice fake owner `DefId` with structural identity.

**Architecture:** First canonicalize collect-produced HIR headers from `item_index` and resolver tables before `Declarations` leaves `collect`. Then make lowering preserve those collected IDs and stop minting new named-item IDs in supported paths. Finally, tighten monomorphization so named functions and impl owners resolve only from canonical tables, and builtin slice impls use an explicit structural owner in `InstanceOrigin` instead of a hashed synthetic `DefId`.

**Tech Stack:** Rust 2021, `rock-lib`, current collect/lower/infer/mono pipeline, focused unit tests, artifact tests, and `cargo test -p rock-lib`.

---

## File Map

- `lib/src/collect/mod.rs`: build resolver tables, then rewrite collected named-item HIR headers to canonical `DefId`s before returning `Declarations` and `ArtifactDeclarations`.
- `lib/src/lower/mod.rs`: replace fresh-ID fallback in `def_id_for_name` with strict canonical lookup.
- `lib/src/lower/bodies.rs`: preserve already-collected IDs when rebuilding signature-backed impl method headers during body lowering.
- `lib/src/mono/mod.rs`: require canonical lookup for function origins and named impl owners; delete fresh-ID and synthetic-owner fallback behavior.
- `lib/src/mono/external.rs`: require canonical IDs for external generic stubs.
- `lib/src/mono/registry.rs`: represent builtin impl owners structurally inside `InstanceOrigin`.
- `lib/src/collect/mod.rs`, `lib/src/lower/mod.rs`, `lib/src/mono/mod.rs`, `lib/src/mono/external.rs`, `lib/src/mono/methods.rs`, `lib/src/crate_artifact/tests.rs`, `lib/tests/integration.rs`: focused regressions for canonical identity, artifact-backed loading, and slice-backed dispatch.

## Scope Guard

- This slice hardens the supported pipeline used by `lib/src/lib.rs`: `collect::collect` -> `lower::program::lower_from_declarations` -> `infer::finalize` -> `mono::monomorphize_with_crates`.
- `HirImpl.id` stays out of scope for now. `ItemIndex` still keys impl items only by top-level short name, so it cannot yet provide stable unique impl identities for every impl in every module.
- Temporary lowering-only stubs in `lib/src/lower/control_flow/secondary.rs` and `lib/src/lower/traits/defaults.rs` stay out of scope. They are helper values, not canonical named items.
- This slice does not redesign `Type`; it only hardens identity.

### Task 1: Canonicalize Collected Named-Item HIR IDs

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Test: `lib/src/collect/mod.rs`

- [ ] **Step 1: Write the failing collect regression**

Add this test to `lib/src/collect/mod.rs` near the other collect identity tests:

```rust
#[test]
fn collect_assigns_canonical_def_ids_to_named_function_and_type_headers() {
    let program = Program {
        module: Module {
            name: None,
            top_levels: vec![
                TopLevel::FunctionDecl(function_decl("root_fn")),
                TopLevel::StructDecl(StructDecl {
                    name: type_inner("Point"),
                    fields: vec![],
                    exported: false,
                }),
                TopLevel::TraitDecl(crate::ast::TraitDecl {
                    name: type_inner("Show"),
                    associated_types: vec![],
                    methods: HashMap::new(),
                    signatures: HashMap::new(),
                    exported: false,
                }),
                inline_module(
                    "math",
                    vec![TopLevel::FunctionDecl(function_decl("vector_len"))],
                ),
            ],
            is_inline: false,
            filepath: None,
        },
    };

    let decls = collect(&program, &CrateContext::new(), false, Some("test"))
        .expect("collect should assign canonical ids to named headers");

    assert_eq!(
        decls.functions["root_fn"].id,
        *decls.resolver.item_paths.get("root_fn").unwrap()
    );
    assert_eq!(
        decls.structs["Point"].id,
        *decls.resolver.item_paths.get("Point").unwrap()
    );
    assert_eq!(
        decls.traits["Show"].id,
        *decls.resolver.item_paths.get("Show").unwrap()
    );
    assert_eq!(
        decls.functions["math::vector_len"].id,
        *decls.resolver.item_paths.get("math::vector_len").unwrap()
    );
}
```

This test can use the existing test-module imports in `lib/src/collect/mod.rs`; no new helper module is needed.

- [ ] **Step 2: Run the new test to verify the red state**

Run: `cargo test -p rock-lib collect_assigns_canonical_def_ids_to_named_function_and_type_headers -- --exact`
Expected: FAIL because `collect` still returns placeholder `DefId`s for collected HIR headers.

- [ ] **Step 3: Rewrite collected named-item IDs after resolver construction**

Add these helpers to `lib/src/collect/mod.rs` above `collect(...)`:

```rust
fn resolve_named_hir_def_id(
    resolver: &ResolverTables,
    map_key: &str,
    fallback_name: &str,
) -> Option<crate::ids::DefId> {
    [map_key, fallback_name].into_iter().find_map(|candidate| {
        resolver
            .item_paths
            .get(candidate)
            .copied()
            .or_else(|| resolver.import_aliases.get(candidate).copied())
            .or_else(|| resolver.export_aliases.get(candidate).copied())
    })
}

fn apply_canonical_named_item_ids(
    resolver: &ResolverTables,
    structs: &mut HashMap<String, HirStruct>,
    enums: &mut HashMap<String, HirEnum>,
    traits: &mut HashMap<String, HirTrait>,
    functions: &mut HashMap<String, HirFunction>,
) {
    for (key, strukt) in structs.iter_mut() {
        if let Some(def_id) = resolve_named_hir_def_id(resolver, key, &strukt.name) {
            strukt.id = def_id;
        }
    }

    for (key, enum_) in enums.iter_mut() {
        if let Some(def_id) = resolve_named_hir_def_id(resolver, key, &enum_.name) {
            enum_.id = def_id;
        }
    }

    for (key, trait_) in traits.iter_mut() {
        if let Some(def_id) = resolve_named_hir_def_id(resolver, key, &trait_.name) {
            trait_.id = def_id;
        }
    }

    for (key, func) in functions.iter_mut() {
        if let Some(def_id) = resolve_named_hir_def_id(resolver, key, &func.name) {
            func.id = def_id;
        }
    }
}
```

Then update both `collect(...)` and `collect_artifact_declarations(...)` so they canonicalize named headers immediately after `build_resolver_tables(...)`:

```rust
let mut structs = structs;
let mut enums = enums;
let mut traits = traits;
let mut functions = functions;

apply_canonical_named_item_ids(
    &resolver,
    &mut structs,
    &mut enums,
    &mut traits,
    &mut functions,
);
```

Use those rewritten `structs`, `enums`, `traits`, and `functions` when constructing `Declarations` and `ArtifactDeclarations`.

- [ ] **Step 4: Re-run the focused collect regressions**

Run:

- `cargo test -p rock-lib collect_assigns_canonical_def_ids_to_named_function_and_type_headers -- --exact`
- `cargo test -p rock-lib collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact`
- `cargo test -p rock-lib collect_records_dependency_crate_identity_canonically -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the collect-side hardening**

```bash
git add lib/src/collect/mod.rs
git commit -m "collect: assign canonical DefIds to named HIR headers"
```

### Task 2: Make Lowering Strict About Canonical Named-Item IDs

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/bodies.rs`
- Test: `lib/src/lower/mod.rs`
- Test: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add the failing lower-side invariant test**

Add this test to `lib/src/lower/mod.rs` next to the existing `lower_uses_canonical_current_crate_import_aliases` test:

```rust
#[test]
#[should_panic(expected = "missing canonical DefId for lowered item")]
fn def_id_for_name_panics_without_canonical_resolver_entry() {
    let mut lowerer = Lowerer::new();
    let _ = lowerer.def_id_for_name(&["missing::item"]);
}
```

- [ ] **Step 2: Run the new test to verify the red state**

Run: `cargo test -p rock-lib def_id_for_name_panics_without_canonical_resolver_entry -- --exact`
Expected: FAIL because `Lowerer::def_id_for_name` still allocates a fresh `DefId` instead of panicking.

- [ ] **Step 3: Replace fresh-ID fallback with strict canonical lookup and preserve existing IDs during body rebuilding**

In `lib/src/lower/mod.rs`, change `def_id_for_name` to stop mutating resolver state and to fail fast when a canonical identity is missing:

```rust
pub(crate) fn def_id_for_name(&self, candidates: &[&str]) -> crate::ids::DefId {
    for candidate in candidates {
        if let Some(def_id) = self
            .resolver
            .item_paths
            .get(*candidate)
            .copied()
            .or_else(|| self.resolver.import_aliases.get(*candidate).copied())
            .or_else(|| self.resolver.export_aliases.get(*candidate).copied())
        {
            return def_id;
        }

        for resolver in self.dependency_resolvers.values() {
            if let Some(def_id) = resolver
                .item_paths
                .get(*candidate)
                .copied()
                .or_else(|| resolver.import_aliases.get(*candidate).copied())
                .or_else(|| resolver.export_aliases.get(*candidate).copied())
            {
                return def_id;
            }
        }
    }

    panic!("missing canonical DefId for lowered item: {:?}", candidates);
}
```

In `lib/src/lower/bodies.rs`, preserve the already-collected method ID when a signature-backed impl method header is rebuilt for body lowering:

```rust
let mut func = if let Some((_, sig)) = imp
    .signatures
    .iter()
    .find(|(ident, _)| ident.name == method_name)
{
    let hir_sig = self.lower_function_sig(sig);
    let mut rebuilt = self.lower_function_decl_header_with_sig(fd, &hir_sig);
    rebuilt.id = existing_func.id;
    rebuilt
} else {
    existing_func.clone()
};
func.qualified_name = existing_func.qualified_name.clone();
```

- [ ] **Step 4: Re-run the focused lower regressions**

Run:

- `cargo test -p rock-lib def_id_for_name_panics_without_canonical_resolver_entry -- --exact`
- `cargo test -p rock-lib lower_uses_canonical_current_crate_import_aliases -- --exact`
- `cargo test -p rock-lib lower_from_declarations_resolves_nested_source_backed_imports_inside_inline_module_bodies -- --exact`
- `cargo test -p rock-lib lower_from_declarations_resolves_nested_imported_types_inside_inline_module_bodies -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the lower-side hardening**

```bash
git add lib/src/lower/mod.rs lib/src/lower/bodies.rs
git commit -m "lower: require canonical DefIds for named items"
```

### Task 3: Make Mono Function Identity Strictly Canonical

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/external.rs`
- Test: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/external.rs`

- [ ] **Step 1: Add the failing mono regressions**

Add this test to `lib/src/mono/mod.rs` next to `resolve_def_id_uses_resolver_aliases`:

```rust
#[test]
#[should_panic(expected = "missing canonical DefId for instance origin")]
fn resolve_def_id_panics_when_canonical_identity_is_missing() {
    let mono = Monomorphizer::new();
    let _ = mono.resolve_def_id(&["missing::item".to_string()]);
}
```

Add this test to `lib/src/mono/external.rs` inside the existing test module:

```rust
#[test]
#[should_panic(expected = "missing canonical DefId for instance origin")]
fn create_stub_generic_function_requires_canonical_def_id() {
    let mut mono = Monomorphizer::new();
    let fd = crate::ast::FunctionDecl {
        name: crate::ast::Ident {
            name: "identity".to_string(),
            span: Span::default(),
        },
        lambda: crate::ast::LambdaDecl {
            parameters: vec![],
            body: crate::ast::Block { statements: vec![] },
            arrow_kind: crate::ast::LambdaArrowKind::Normal,
        },
        self_receiver: None,
        is_unsafe: false,
        exported: false,
    };

    let _ = mono.create_stub_generic_function(&fd, "dep::identity");
}
```

- [ ] **Step 2: Run the new tests to verify the red state**

Run:

- `cargo test -p rock-lib resolve_def_id_panics_when_canonical_identity_is_missing -- --exact`
- `cargo test -p rock-lib create_stub_generic_function_requires_canonical_def_id -- --exact`

Expected: the first test already passes, and the second FAILS because `create_stub_generic_function` still goes through the fresh-ID fallback helper.

- [ ] **Step 3: Remove fresh-ID fallback from mono named-item lookup**

In `lib/src/mono/mod.rs`, make `def_id_for_name` delegate to the existing strict resolver path instead of allocating new local IDs:

```rust
fn def_id_for_name(&self, candidates: &[&str]) -> DefId {
    let owned: Vec<String> = candidates.iter().map(|s| s.to_string()).collect();
    self.resolve_def_id(&owned)
}
```

Keep `resolve_def_id(...)` as the single place that produces the panic message for missing canonical identity.

In `lib/src/mono/external.rs`, leave `create_stub_generic_function(...)` calling `self.def_id_for_name(&[qualified_name])`; the helper above is now strict, so missing dependency-canonical identity becomes a real failure instead of silent fresh ID allocation.

- [ ] **Step 4: Re-run the focused mono regressions**

Run:

- `cargo test -p rock-lib mono::tests::resolve_def_id_uses_resolver_aliases -- --exact`
- `cargo test -p rock-lib mono::tests::resolve_def_id_panics_when_canonical_identity_is_missing -- --exact`
- `cargo test -p rock-lib mono::external::tests::create_stub_generic_function_requires_canonical_def_id -- --exact`
- `cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact`
- `cargo test -p rock-lib mono::external::tests::process_with_crates_records_object_backed_instances_without_re_emitting -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the mono named-function hardening**

```bash
git add lib/src/mono/mod.rs lib/src/mono/external.rs
git commit -m "mono: require canonical DefIds for named functions"
```

### Task 4: Replace Builtin Slice Fake Owner IDs With Structural Instance Identity

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/external.rs`
- Test: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/registry.rs`
- Test: `lib/src/mono/methods.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add the failing structural-owner regression**

Replace `builtin_slice_owner_identity_does_not_depend_on_type_name` in `lib/src/mono/mod.rs` with this test:

```rust
#[test]
fn builtin_slice_owner_identity_is_structural() {
    let mono = Monomorphizer::new();
    let imp = HirImpl {
        id: DefId::new(CrateId(0), LocalDefId(0)),
        owner: HirImplOwner::BuiltinSlice,
        type_name: "Array".to_string(),
        type_generics: vec!["T".to_string()],
        receiver_arg_types: vec![Type::Generic("T".to_string())],
        trait_name: Some("Show".to_string()),
        trait_generics: vec![],
        trait_arg_types: vec![],
        associated_types: vec![],
        bounds: vec![],
        methods: HashMap::new(),
    };

    assert_eq!(
        mono.method_instance_origin(&imp, None, "println"),
        InstanceOrigin::ImplMethod {
            owner: InstanceImplOwner::BuiltinSlice,
            method: "println".to_string(),
        }
    );
}
```

- [ ] **Step 2: Run the new structural-owner test to verify the red state**

Run: `cargo test -p rock-lib builtin_slice_owner_identity_is_structural -- --exact`
Expected: FAIL to compile until the new structural owner enum exists.

- [ ] **Step 3: Introduce explicit builtin owner identity in the registry and mono origin builder**

In `lib/src/mono/registry.rs`, add a structural owner enum and use it inside `InstanceOrigin`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstanceImplOwner {
    Named(DefId),
    BuiltinSlice,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstanceOrigin {
    Function(DefId),
    ImplMethod {
        owner: InstanceImplOwner,
        method: String,
    },
}
```

Update the registry tests in the same file so named impl methods now use `InstanceImplOwner::Named(owner)` instead of a raw `DefId`.

In `lib/src/mono/mod.rs`, replace the old fake-owner helpers with one structural helper:

```rust
fn impl_owner_identity(&self, imp: &HirImpl, crate_name: Option<&str>) -> InstanceImplOwner {
    match &imp.owner {
        HirImplOwner::BuiltinSlice => InstanceImplOwner::BuiltinSlice,
        HirImplOwner::Named(owner_path) => {
            let mut candidates = vec![owner_path.clone(), imp.type_name.clone()];
            if let Some(crate_name) = crate_name {
                candidates.push(format!("{}::{}", crate_name, imp.type_name));
            }
            if let Some(trait_name) = &imp.trait_name {
                candidates.push(format!("{}::{}", imp.type_name, trait_name));
                candidates.push(trait_name.clone());
            }

            InstanceImplOwner::Named(self.resolve_def_id(&candidates))
        }
    }
}

fn method_instance_origin(
    &self,
    imp: &HirImpl,
    crate_name: Option<&str>,
    method_name: &str,
) -> InstanceOrigin {
    InstanceOrigin::ImplMethod {
        owner: self.impl_owner_identity(imp, crate_name),
        method: method_name.to_string(),
    }
}
```

Delete these old helpers from `lib/src/mono/mod.rs`:

```rust
pub(super) fn builtin_slice_owner_def_id() -> DefId
fn impl_owner_def_id(&self, imp: &HirImpl, crate_name: Option<&str>) -> DefId
fn synthetic_owner_def_id(name: &str) -> DefId
```

Update any `InstanceOrigin::ImplMethod` constructors in `lib/src/mono/methods.rs` and `lib/src/mono/external.rs` tests to use `InstanceImplOwner::Named(...)` where needed.

- [ ] **Step 4: Re-run the focused structural-owner regressions**

Run:

- `cargo test -p rock-lib mono::tests::builtin_slice_owner_identity_is_structural -- --exact`
- `cargo test -p rock-lib mono::registry::tests::instance_registry_distinguishes_impl_methods_by_name -- --exact`
- `cargo test -p rock-lib mono::methods::tests::test_monomorphize_trait_method_call_prefers_concrete_slice_impl_over_generic_builtin_slice_impl -- --exact`
- `cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact`
- `cargo test -p rock-lib --test integration test_explicit_generic_slice_impl_method_as_value_falls_back_from_u8_slice -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the structural-owner hardening**

```bash
git add lib/src/mono/registry.rs lib/src/mono/mod.rs lib/src/mono/methods.rs lib/src/mono/external.rs lib/tests/integration.rs
git commit -m "mono: use structural builtin impl owner identity"
```

### Task 5: Cross-Crate And Artifact Confidence Pass

**Files:**
- Verify only unless one of these tests fails.

- [ ] **Step 1: Run the interface-only artifact regressions**

Run:

- `cargo test -p rock-lib crate_artifact::tests::test_compile_with_interface_only_artifact -- --exact`
- `cargo test -p rock-lib crate_artifact::tests::test_glob_import_with_interface_only_artifact -- --exact`
- `cargo test -p rock-lib crate_artifact::tests::test_compile_with_interface_only_stdlib_artifact -- --exact`

Expected: PASS.

- [ ] **Step 2: Run the cross-crate generic/default artifact regression**

Run: `cargo test -p rock-lib crate_artifact::tests::test_build_cross_crate_hir_preserves_dependency_trait_defaults_for_current_generic_impls -- --exact`
Expected: PASS.

- [ ] **Step 3: Run the highest-signal mono integration regressions**

Run:

- `cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact`
- `cargo test -p rock-lib --test integration test_trait_default_method_substitutes_generic_struct_pattern_types -- --exact`
- `cargo test -p rock-lib --test integration test_selfless_trait_default_stays_associated_function_when_injected -- --exact`

Expected: PASS.

### Task 6: Final Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run the full package suite**

Run: `cargo test -p rock-lib`
Expected: PASS.

- [ ] **Step 2: Inspect final status**

Run: `git status --short`
Expected: only the intended tracked changes for this slice, plus any unrelated pre-existing workspace noise.
