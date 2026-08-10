# Adapted Canonical Identity Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the canonical identity slice from the audit by eliminating remaining fresh or synthetic `DefId` creation for named items in the supported `collect -> lower_from_declarations -> mono` pipeline.

**Architecture:** The repo already has canonical resolver tables, reverse canonical lookup maps, artifact persistence of resolver data, and `DefId` fields on major HIR headers. This plan starts from that current state: first make collected HIR headers use resolver-owned canonical IDs, then make lower and mono fail fast when supported named items are missing canonical identity, then replace mono's builtin-slice fake owner `DefId` with structural instance-owner identity. Legacy string maps remain only for domains not migrated by this slice.

**Tech Stack:** Rust 2021, `rock-lib`, `collect`, `lower`, `infer`, `mono`, focused unit tests, artifact regressions, `cargo fmt --all`, `cargo test -p rock-lib`.

---

## Current Repo State From Audit Review

The following items from the high-level audit and prior canonical identity plans are already present in the clean tree:

- `lib/src/ids.rs` defines `CrateId`, `ModuleId`, `LocalDefId`, `DefId`, and `IdGen`.
- `lib/src/collect/item_index.rs` allocates canonical module and definition IDs during item indexing.
- `lib/src/collect/resolver.rs` defines:
  - `ResolverTables::module_paths: HashMap<String, ModuleId>`
  - `ResolverTables::module_names_by_id: HashMap<ModuleId, String>`
  - `ResolverTables::item_paths: HashMap<String, DefId>`
  - `ResolverTables::item_names_by_id: HashMap<DefId, String>`
  - `ResolverTables::import_aliases: HashMap<String, DefId>`
  - `ResolverTables::export_aliases: HashMap<String, DefId>`
- `lib/src/collect/mod.rs` builds `item_index` and `resolver` in both `collect(...)` and `collect_artifact_declarations(...)`.
- `lib/src/hir/mod.rs` stores `DefId` on `HirFunction`, `HirStruct`, `HirEnum`, `HirTrait`, and `HirImpl`.
- Artifact resolver persistence is in place according to `docs/superpowers/plans/master-audit-checklist.md`.

The following audit items are still missing in the clean tree and are in scope for this plan:

- `lib/src/collect/mod.rs` still returns collected named-item headers with pre-resolver IDs instead of rewriting them from canonical resolver tables.
- `lib/src/lower/mod.rs::Lowerer::def_id_for_name` still allocates and inserts a fresh fallback `DefId` when lookup fails.
- `lib/src/lower/bodies.rs` rebuilds signature-backed impl method headers without preserving the existing collected method ID.
- `lib/src/mono/mod.rs::Monomorphizer::def_id_for_name` still allocates and inserts a fresh fallback `DefId` when lookup fails.
- `lib/src/mono/mod.rs` still has `builtin_slice_owner_def_id()` and `synthetic_owner_def_id(...)` fake owner IDs for `HirImplOwner::BuiltinSlice`. This is not about reviving the deprecated source-level `Array T` impl sugar. Some old tests still use `type_name: "Array"` as a generic slice-impl stand-in; when touching them in this slice, use the current borrowed-slice spelling (`"&[T]"`) so tests stay aligned with the source rule that bare `[T]` must live behind a reference.
- `lib/src/mono/registry.rs::InstanceOrigin::ImplMethod` still stores `owner: DefId` directly rather than a structural `InstanceImplOwner`.

## File Map

- Modify: `lib/src/collect/mod.rs`
  - Add collect regression for canonical HIR header IDs.
  - Add helpers that rewrite collected `HirStruct`, `HirEnum`, `HirTrait`, and `HirFunction` IDs from `ResolverTables`.
  - Call those helpers in both `collect(...)` and `collect_artifact_declarations(...)` after `build_resolver_tables(...)`.
- Modify: `lib/src/lower/mod.rs`
  - Add strict missing-identity regression.
  - Replace fresh-ID fallback in `Lowerer::def_id_for_name` with canonical-only lookup across current and dependency resolver tables.
- Modify: `lib/src/lower/bodies.rs`
  - Preserve `existing_func.id` when signature-backed impl method headers are rebuilt for body lowering, without resolving the bare method name as a standalone item.
- Modify: `lib/src/mono/mod.rs`
  - Add strict mono missing-identity regression.
  - Replace fresh-ID fallback in `Monomorphizer::def_id_for_name` with `resolve_def_id(...)`.
  - Replace fake builtin slice owner IDs with `InstanceImplOwner`.
- Modify: `lib/src/mono/external.rs`
  - Add regression proving generic external stubs require canonical IDs.
- Modify: `lib/src/mono/registry.rs`
  - Add `InstanceImplOwner` and update `InstanceOrigin::ImplMethod`.
  - Update registry unit tests.
- Modify only if compiler requires it: `lib/src/mono/methods.rs`, `lib/tests/integration.rs`
  - Update imports or expected `InstanceOrigin::ImplMethod` construction after registry type change.

## Scope Guard

- Do not redesign `Type` or replace string-bearing semantic types in this slice.
- Do not migrate `HirProgram` maps to ID-keyed tables in this slice.
- Do not assign stable first-class IDs to fields, variants, methods, or associated items beyond preserving existing method IDs.
- Do not change parser/module IO behavior.
- Keep `HirImpl.id` canonicalization out of scope because `ItemIndex` still does not uniquely index every impl item.
- Temporary lowering-only stubs in `lib/src/lower/control_flow/secondary.rs` and `lib/src/lower/traits/defaults.rs` are out of scope.

---

### Task 0: Establish Clean Baseline And Existing Test Names

**Files:**
- Verify only.

- [ ] **Step 1: Confirm no tracked implementation changes are present**

Run:

```bash
git status --short
```

Expected: no tracked `M` entries from implementation work. Pre-existing untracked docs may remain:

```text
?? docs/superpowers/plans/2026-05-04-canonical-defid-hardening.md
?? docs/superpowers/plans/master-audit-checklist.md
```

- [ ] **Step 2: Confirm the old WIP stash exists before execution**

Run:

```bash
git stash list | head -5
```

Expected: includes the safety stash named `wip canonical identity aborted pre-plan`. Do not apply it during this plan unless manually salvaging snippets.

- [ ] **Step 3: Run baseline compile for the package**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact
```

Expected: PASS. This proves the current tree compiles and the already-done reverse canonical tables still work.

---

### Task 1: Canonicalize Collected Named-Item HIR IDs

**Files:**
- Modify: `lib/src/collect/mod.rs`
- Test: `lib/src/collect/mod.rs`

- [ ] **Step 1: Add the failing collect regression**

Add this test inside `#[cfg(test)] mod tests` in `lib/src/collect/mod.rs`, near `collect_builds_reverse_canonical_tables_for_current_crate_items`:

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
                TopLevel::EnumDecl(crate::ast::EnumDecl {
                    name: type_inner("Color"),
                    variants: vec![],
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
        decls.enums["Color"].id,
        *decls.resolver.item_paths.get("Color").unwrap()
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

- [ ] **Step 2: Run the test to verify the red state**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_assigns_canonical_def_ids_to_named_function_and_type_headers -- --exact
```

Expected: FAIL because collected HIR headers are not yet rewritten from `ResolverTables`.

- [ ] **Step 3: Add canonical header rewrite helpers**

Add these helpers above `collect(...)` in `lib/src/collect/mod.rs`:

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

- [ ] **Step 4: Apply the rewrite in `collect(...)`**

Immediately after `let resolver = build_resolver_tables(...)` in `collect(...)`, add:

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

Use those rewritten variables in the returned `Declarations`.

- [ ] **Step 5: Apply the rewrite in `collect_artifact_declarations(...)`**

Immediately after `let resolver = build_resolver_tables(...)` in `collect_artifact_declarations(...)`, add the same block:

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

Use those rewritten variables in the returned `ArtifactDeclarations`.

- [ ] **Step 6: Run focused collect regressions**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_assigns_canonical_def_ids_to_named_function_and_type_headers -- --exact
cargo test -p rock-lib collect::tests::collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 1**

Run:

```bash
git add lib/src/collect/mod.rs
git commit -m "collect: assign canonical DefIds to named HIR headers"
```

---

### Task 2: Make Lowering Strict About Canonical Named-Item IDs

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/bodies.rs`
- Test: `lib/src/lower/mod.rs`

- [ ] **Step 1: Add the missing-identity regression**

Add this test inside `#[cfg(test)] mod tests` in `lib/src/lower/mod.rs`, next to `lower_uses_canonical_current_crate_import_aliases`:

```rust
#[test]
#[should_panic(expected = "missing canonical DefId for lowered item")]
fn def_id_for_name_panics_without_canonical_resolver_entry() {
    let lowerer = Lowerer::new();
    let _ = lowerer.def_id_for_name(&["missing::item"]);
}
```

- [ ] **Step 2: Run the test to verify the red state**

Run:

```bash
cargo test -p rock-lib lower::tests::def_id_for_name_panics_without_canonical_resolver_entry -- --exact
```

Expected: FAIL because `Lowerer::def_id_for_name` still allocates a fresh `DefId`.

- [ ] **Step 3: Replace lower fresh-ID fallback with strict canonical lookup**

Replace `Lowerer::def_id_for_name` in `lib/src/lower/mod.rs` with:

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

- [ ] **Step 4: Preserve existing impl method IDs during body rebuild**

In `lib/src/lower/function.rs`, split `lower_function_decl_header_with_sig(...)` so the normal standalone-function path still resolves the canonical `DefId`, while impl method body rebuilding can pass the already-collected method ID directly. Keep the existing signature-backed header construction body, but move it into `lower_function_decl_header_with_sig_and_id(...)`. In the returned `HirFunction`, change the ID field from `id: self.def_id_for_name(&[&fd.name.name]),` to `id,`.

Add a wrapper named `lower_function_decl_header_with_sig(...)` that computes `let id = self.def_id_for_name(&[&fd.name.name]);` and delegates to `self.lower_function_decl_header_with_sig_and_id(fd, sig, id)`. Rename the current body to `lower_function_decl_header_with_sig_and_id(&mut self, fd: &ast::FunctionDecl, sig: &HirFunctionSig, id: crate::ids::DefId) -> HirFunction`, then use `id,` in the returned `HirFunction`.

In `lib/src/lower/bodies.rs`, replace the signature-backed rebuild block inside `lower_impl_bodies(...)` with:

```rust
let mut func = if let Some((_, sig)) = imp
    .signatures
    .iter()
    .find(|(ident, _)| ident.name == method_name)
{
    let hir_sig = self.lower_function_sig(sig);
    self.lower_function_decl_header_with_sig_and_id(fd, &hir_sig, existing_func.id)
} else {
    existing_func.clone()
};
func.qualified_name = existing_func.qualified_name.clone();
```

- [ ] **Step 5: Run focused lower regressions**

Run:

```bash
cargo test -p rock-lib lower::tests::def_id_for_name_panics_without_canonical_resolver_entry -- --exact
cargo test -p rock-lib lower::tests::lower_uses_canonical_current_crate_import_aliases -- --exact
cargo test -p rock-lib collect::tests::lower_from_declarations_resolves_nested_source_backed_imports_inside_inline_module_bodies -- --exact
cargo test -p rock-lib collect::tests::lower_from_declarations_resolves_nested_imported_types_inside_inline_module_bodies -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit Task 2**

Run:

```bash
git add lib/src/lower/mod.rs lib/src/lower/bodies.rs
git commit -m "lower: require canonical DefIds for named items"
```

---

### Task 3: Make Mono Function Identity Strictly Canonical

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/external.rs`
- Test: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/external.rs`

- [ ] **Step 1: Add strict mono resolver regression**

Add this test inside `#[cfg(test)] mod tests` in `lib/src/mono/mod.rs`, next to `resolve_def_id_uses_resolver_aliases`:

```rust
#[test]
#[should_panic(expected = "missing canonical DefId for instance origin")]
fn resolve_def_id_panics_when_canonical_identity_is_missing() {
    let mono = Monomorphizer::new();
    let _ = mono.resolve_def_id(&["missing::item".to_string()]);
}
```

- [ ] **Step 2: Add external generic stub regression**

Add this test inside `#[cfg(test)] mod tests` in `lib/src/mono/external.rs`:

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

- [ ] **Step 3: Run the tests to verify red state**

Run:

```bash
cargo test -p rock-lib mono::tests::resolve_def_id_panics_when_canonical_identity_is_missing -- --exact
cargo test -p rock-lib mono::external::tests::create_stub_generic_function_requires_canonical_def_id -- --exact
```

Expected: first test PASS because `resolve_def_id` is already strict; second test FAIL because `create_stub_generic_function` still calls fallback-allocating `def_id_for_name`.

- [ ] **Step 4: Remove mono fresh-ID fallback**

Replace `Monomorphizer::def_id_for_name` in `lib/src/mono/mod.rs` with:

```rust
fn def_id_for_name(&self, candidates: &[&str]) -> DefId {
    let owned: Vec<String> = candidates.iter().map(|s| s.to_string()).collect();
    self.resolve_def_id(&owned)
}
```

Do not change `create_stub_generic_function(...)`; it should keep calling `self.def_id_for_name(&[qualified_name])`, which is now strict.

- [ ] **Step 5: Run focused mono regressions**

Run:

```bash
cargo test -p rock-lib mono::tests::resolve_def_id_uses_resolver_aliases -- --exact
cargo test -p rock-lib mono::tests::resolve_def_id_panics_when_canonical_identity_is_missing -- --exact
cargo test -p rock-lib mono::external::tests::create_stub_generic_function_requires_canonical_def_id -- --exact
cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact
cargo test -p rock-lib mono::external::tests::process_with_crates_records_object_backed_instances_without_re_emitting -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit Task 3**

Run:

```bash
git add lib/src/mono/mod.rs lib/src/mono/external.rs
git commit -m "mono: require canonical DefIds for named functions"
```

---

### Task 4: Replace Builtin Slice Fake Owner IDs With Structural Instance Identity

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/mod.rs`
- Modify as needed: `lib/src/mono/methods.rs`
- Modify as needed: `lib/src/mono/external.rs`
- Test: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/registry.rs`
- Test: `lib/src/mono/methods.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Replace the old builtin-slice owner regression**

In `lib/src/mono/mod.rs`, replace `builtin_slice_owner_identity_does_not_depend_on_type_name` with:

```rust
#[test]
fn builtin_slice_owner_identity_is_structural() {
    let mono = Monomorphizer::new();
    let imp = HirImpl {
        id: DefId::new(CrateId(0), LocalDefId(0)),
        owner: HirImplOwner::BuiltinSlice,
        type_name: "&[T]".to_string(),
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

Also update the test module imports to include `InstanceImplOwner` and `InstanceOrigin` if they are not already in scope:

```rust
use super::{InstanceImplOwner, InstanceOrigin, Monomorphizer};
```

- [ ] **Step 2: Run the new structural-owner test to verify red state**

Run:

```bash
cargo test -p rock-lib mono::tests::builtin_slice_owner_identity_is_structural -- --exact
```

Expected: FAIL to compile because `InstanceImplOwner` does not exist yet.

- [ ] **Step 3: Add `InstanceImplOwner` to the registry**

In `lib/src/mono/registry.rs`, replace the current `InstanceOrigin` definition with:

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

- [ ] **Step 4: Export `InstanceImplOwner` from mono**

In `lib/src/mono/mod.rs`, update the registry re-export to include `InstanceImplOwner`:

```rust
pub use registry::{
    InstanceId, InstanceImplOwner, InstanceKey, InstanceOrigin, InstanceRecord, InstanceRegistry,
    MonomorphizedProgram,
};
```

- [ ] **Step 5: Replace mono fake owner helpers with structural owner helper**

In `lib/src/mono/mod.rs`, delete these helpers:

```rust
pub(super) fn builtin_slice_owner_def_id() -> DefId
fn impl_owner_def_id(&self, imp: &HirImpl, crate_name: Option<&str>) -> DefId
fn synthetic_owner_def_id(name: &str) -> DefId
```

Add this replacement helper:

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
```

Update `method_instance_origin(...)` to use it:

```rust
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

- [ ] **Step 6: Remove now-unused synthetic-ID imports**

In `lib/src/mono/mod.rs`, remove imports that are only used by `synthetic_owner_def_id(...)`, for example:

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
```

Keep `CrateId` in the main module only if production code still uses it. If it is only test code, import it inside the test module.

- [ ] **Step 7: Update registry tests to wrap named owners**

In `lib/src/mono/registry.rs`, update `instance_registry_distinguishes_impl_methods_by_name` so all impl method origins use `InstanceImplOwner::Named(owner)`:

```rust
let map_key = InstanceKey::new(
    InstanceOrigin::ImplMethod {
        owner: InstanceImplOwner::Named(owner),
        method: "map".to_string(),
    },
    vec![Type::I64],
);
let println_key = InstanceKey::new(
    InstanceOrigin::ImplMethod {
        owner: InstanceImplOwner::Named(owner),
        method: "println".to_string(),
    },
    vec![Type::I64],
);
```

Also update each `InstanceRecord { origin: InstanceOrigin::ImplMethod { ... } }` in that test the same way.

- [ ] **Step 8: Update any remaining `InstanceOrigin::ImplMethod` constructors**

Search:

```bash
rg "InstanceOrigin::ImplMethod" lib/src lib/tests
```

For any named impl owner, change:

```rust
InstanceOrigin::ImplMethod { owner, method }
```

to:

```rust
InstanceOrigin::ImplMethod {
    owner: InstanceImplOwner::Named(owner),
    method,
}
```

For builtin slice owners, use:

```rust
InstanceOrigin::ImplMethod {
    owner: InstanceImplOwner::BuiltinSlice,
    method,
}
```

- [ ] **Step 9: Run focused structural-owner regressions**

Run:

```bash
cargo test -p rock-lib mono::tests::builtin_slice_owner_identity_is_structural -- --exact
cargo test -p rock-lib mono::registry::tests::instance_registry_distinguishes_impl_methods_by_name -- --exact
cargo test -p rock-lib mono::methods::tests::test_collect_impls_keeps_object_backed_concrete_trait_impls_available_for_dispatch -- --exact
cargo test -p rock-lib mono::methods::tests::test_monomorphize_trait_method_call_preserves_borrowed_slice_self_type -- --exact
cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact
cargo test -p rock-lib --test integration test_explicit_generic_slice_impl_method_as_value_falls_back_from_u8_slice -- --exact
```

Expected: PASS.

- [ ] **Step 10: Commit Task 4**

Run:

```bash
git add lib/src/mono/registry.rs lib/src/mono/mod.rs lib/src/mono/methods.rs lib/src/mono/external.rs lib/tests/integration.rs
git commit -m "mono: use structural builtin impl owner identity"
```

If some listed files were not modified, omit them from `git add`.

---

### Task 5: Cross-Crate, Prelude, And Artifact Confidence Pass

**Files:**
- Verify only unless a listed test fails.

- [ ] **Step 1: Run existing canonical resolver identity regressions**

Run:

```bash
cargo test -p rock-lib collect::tests::collect_builds_reverse_canonical_tables_for_current_crate_items -- --exact
cargo test -p rock-lib lower::tests::lower_uses_canonical_current_crate_import_aliases -- --exact
```

Expected: PASS.

- [ ] **Step 2: Discover whether cross-crate canonical identity tests already exist under renamed test names**

Run:

```bash
cargo test -p rock-lib collect_records_dependency_crate_identity_canonically -- --list
cargo test -p rock-lib collect_records_stdlib_prelude_identity_canonically -- --list
cargo test -p rock-lib collect_records_artifact_module_identity_canonically -- --list
```

Expected in current clean tree before this plan: likely zero listed tests. If zero, do not add those tests in this hardening slice unless Task 1-4 behavior requires it; the already-reviewed audit checklist says resolver persistence exists, but this plan is scoped to remaining fallback/synthetic `DefId` removal.

- [ ] **Step 3: Run source-free product artifact regressions**

Run:

```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_with_source_free_product_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_root_glob_import_from_source_free_product_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_with_stdlib_artifact -- --exact
cargo test -p rock-lib crate_artifact::tests::test_product_stdlib_artifact_preserves_string_method_abi_and_prelude_exports -- --exact
```

Expected: PASS.

- [ ] **Step 4: Run cross-crate generic/default product artifact regressions**

Run:

```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_function_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_impl_from_artifact_hir_bundle -- --exact
cargo test -p rock-lib crate_artifact::tests::test_compile_trait_default_method_from_product_artifact -- --exact
```

Expected: PASS.

- [ ] **Step 5: Run highest-signal mono integration regressions**

Run:

```bash
cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_generic_struct_pattern_types -- --exact
cargo test -p rock-lib --test integration test_selfless_trait_default_stays_associated_function_when_injected -- --exact
```

Expected: PASS.

---

### Task 6: Final Verification And Audit Checklist Update

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Verify: full package.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --all
```

Expected: exits successfully.

- [ ] **Step 2: Run full package suite**

Run:

```bash
cargo test -p rock-lib
```

Expected: PASS.

If Cargo reports stale or inconsistent incremental artifacts, run:

```bash
cargo clean -p rock-lib
cargo test -p rock-lib
```

Expected after retry: PASS.

- [ ] **Step 3: Update audit checklist for completed items**

In `docs/superpowers/plans/master-audit-checklist.md`, update section `## 1. Identity And Arenas` and section `## 2. Real Collection And Name Resolution` to reflect that this slice has removed the remaining named-item fallback creation in lower and mono and replaced builtin slice synthetic owner identity.

Change these lines under `Still to do` only if the full test suite passes:

```markdown
- [ ] Eliminate remaining fallback or synthetic `DefId` creation in later phases.
```

If no other known synthetic/fallback `DefId` creation remains, change to:

```markdown
- [x] Eliminate remaining fallback or synthetic `DefId` creation in later phases for the supported collect -> lower_from_declarations -> mono named-item pipeline.
```

Change these lines under `## 2. Real Collection And Name Resolution`:

```markdown
- [ ] Remove `Lowerer::def_id_for_name` fallback insertion behavior in `lib/src/lower/mod.rs`.
- [ ] Remove `Monomorphizer::def_id_for_name` fallback insertion behavior in `lib/src/mono/mod.rs`.
- [ ] Treat missing canonical identity as a real error instead of creating names on the fly.
```

to:

```markdown
- [x] Remove `Lowerer::def_id_for_name` fallback insertion behavior in `lib/src/lower/mod.rs`.
- [x] Remove `Monomorphizer::def_id_for_name` fallback insertion behavior in `lib/src/mono/mod.rs`.
- [x] Treat missing canonical identity as a real error in the supported collect -> lower_from_declarations -> mono named-item pipeline instead of creating names on the fly.
```

Leave this item unchecked unless a separate cross-crate/prelude canonical resolution expansion is also implemented:

```markdown
- [ ] Make current-crate, dependency, and prelude resolution always flow through canonical IDs.
```

- [ ] **Step 4: Commit final docs update if changed**

Run:

```bash
git add docs/superpowers/plans/master-audit-checklist.md
git commit -m "docs: update canonical identity audit checklist"
```

Skip this commit if the checklist was intentionally left unchanged.

- [ ] **Step 5: Inspect final status**

Run:

```bash
git status --short
```

Expected: only intentional committed changes are absent from status; unrelated pre-existing untracked files may remain if they were not committed.

---

## Self-Review

- Spec coverage: This plan covers the audit requirements to stop treating canonical identity as advisory in the supported named-item pipeline, remove lower/mono fallback creation, and remove mono's synthetic builtin-slice owner identity. It explicitly leaves larger future tracks, such as ID-keyed `HirProgram` maps and semantic `Ty` redesign, out of scope. It is updated for the current product-artifact tests and borrowed-slice/`Str` source rules.
- Placeholder scan: No `TBD`, `TODO`, or unspecified tests are required. Every code-changing step includes concrete snippets or exact search-and-replace guidance.
- Type consistency: `InstanceImplOwner` is introduced in `mono/registry.rs`, re-exported from `mono/mod.rs`, and then used consistently by `InstanceOrigin::ImplMethod` constructors.
