# Complete Task 5 Lower Resolution Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish roadmap Task 5 by moving source/path/name resolution policy for body and type lowering behind a dedicated lower resolution boundary.

**Architecture:** Add `lib/src/lower/resolution.rs` as the single policy layer for lower-side value, type, trait, import, module-prefix, prelude, dependency, and artifact-root-export resolution. Keep `Lowerer` as the mutable state shell during this slice, but migrate body/type lowering call sites away from ad hoc `Lowerer` field searches and broad semantic helper chains.

**Tech Stack:** Rust 2021, `rock-lib`, existing `ResolverTables`, `Scope`, `ArtifactExport`, HIR IDs, focused unit tests, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Files And Responsibilities

- Create `lib/src/lower/resolution.rs`: read-only `LowerResolutionContext`, explicit result structs/enums, resolver precedence, canonical-name helpers, import/glob resolution helpers, owner/builtin trait resolution helpers.
- Modify `lib/src/lower/mod.rs`: declare `resolution` module; remove or shrink broad semantic helper bodies by delegating to `LowerResolutionContext`; keep non-resolution state and mutation helpers in `Lowerer`.
- Modify `lib/src/lower/types.rs`: make `TypeLoweringContext` nominal and trait lookup delegate to resolution APIs; remove suffix-based semantic lookup fallback.
- Modify `lib/src/lower/paths.rs`: make identifier, qualified path, enum variant, struct construction, and custom value lookup consume typed resolution results.
- Modify `lib/src/lower/expression.rs`: route custom-operator lookup through the value resolution API.
- Modify `lib/src/lower/program.rs`: route import/glob import target resolution through import-specific resolution APIs.
- Modify `lib/src/lower/crates/bodies.rs`: route module-local alias injection through import/module resolution APIs instead of direct resolver-map probing.
- Modify `lib/src/lower/crates/registration.rs`: route stdlib prelude collision checks through resolution APIs while leaving resolver-table registration mutation in place.
- Modify `lib/src/lower/collect/declarations.rs`: route export-source qualification/current-prefix resolution through the resolution boundary where it participates in name/path resolution.
- Modify `lib/src/lower/types_helpers/helpers.rs`: route contextual method-lookup name expansion through resolution APIs.
- Modify `lib/src/lower/traits/conformance.rs`, `lib/src/lower/bodies.rs`, and index/operator call sites that use builtin trait or owner path helpers: move semantic owner/builtin trait resolution into `resolution.rs` while preserving existing behavior.
- Modify `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`: mark Task 5 complete only after final verification/review.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: check off the broader Task 5 source/path/name resolution item only after final verification/review.
- Append final verification notes to this plan.

---

## Task 1: Add Lower Resolution Module And Canonical ID Helpers

**Files:**
- Create: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/mod.rs`
- Test: `lib/src/lower/resolution.rs`

- [ ] **Step 1: Add failing canonical/dependency/module-alias tests**

Create `lib/src/lower/resolution.rs` with only the tests below and minimal imports so it fails to compile because `LowerResolutionContext` does not exist:

```rust
#[cfg(test)]
mod tests {
    use crate::collect::resolver::ResolverTables;
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::lower::resolution::LowerResolutionContext;
    use crate::lower::Lowerer;

    fn def_id(crate_id: u32, local_id: u32) -> DefId {
        DefId::new(CrateId(crate_id), LocalDefId(local_id))
    }

    #[test]
    fn resolution_context_resolves_current_and_dependency_items() {
        let current_id = def_id(0, 10);
        let dep_id = def_id(2, 20);
        let mut lowerer = Lowerer::new();
        lowerer
            .resolver
            .item_paths
            .insert("demo::local".to_string(), current_id);
        lowerer
            .resolver
            .item_names_by_id
            .insert(current_id, "demo::local".to_string());

        let mut dep = ResolverTables::default();
        dep.item_paths.insert("dep::value".to_string(), dep_id);
        dep.item_names_by_id.insert(dep_id, "dep::value".to_string());
        lowerer.dependency_resolvers.insert("dep".to_string(), dep);

        let resolution = LowerResolutionContext::new(&lowerer);

        assert_eq!(resolution.resolve_item_id("demo::local"), Some(current_id));
        assert_eq!(resolution.resolve_item_id("dep::value"), Some(dep_id));
        assert_eq!(resolution.canonical_name(current_id), Some("demo::local"));
        assert_eq!(resolution.canonical_name(dep_id), Some("dep::value"));
    }

    #[test]
    fn resolution_context_prefers_module_alias_before_root_item_when_requested() {
        let root_id = def_id(0, 30);
        let module_id = def_id(0, 31);
        let mut lowerer = Lowerer::new();
        lowerer.resolver.item_paths.insert("Thing".to_string(), root_id);
        lowerer
            .resolver
            .item_names_by_id
            .insert(root_id, "Thing".to_string());
        lowerer.resolver.insert_module_alias_with_name(
            "Thing".to_string(),
            "demo::helper::Thing".to_string(),
            module_id,
        );

        let resolution = LowerResolutionContext::new(&lowerer);

        assert_eq!(resolution.resolve_item_id("Thing"), Some(root_id));
        assert_eq!(resolution.resolve_module_alias_or_item_id("Thing"), Some(module_id));
        assert_eq!(
            resolution.canonical_name_for_module_alias_or_item("Thing"),
            Some("demo::helper::Thing".to_string())
        );
    }
}
```

- [ ] **Step 2: Register the module and run the failing tests**

In `lib/src/lower/mod.rs`, add the module declaration near the other lower modules:

```rust
pub(crate) mod resolution;
```

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_current_and_dependency_items -- --exact
```

Expected: FAIL to compile because `LowerResolutionContext` is not defined.

- [ ] **Step 3: Implement canonical ID resolution scaffold**

Replace the temporary `resolution.rs` test-only file with this implementation plus the tests from Step 1:

```rust
use crate::ids::DefId;
use crate::lower::Lowerer;

pub(crate) struct LowerResolutionContext<'a> {
    lowerer: &'a Lowerer,
}

impl<'a> LowerResolutionContext<'a> {
    pub(crate) fn new(lowerer: &'a Lowerer) -> Self {
        Self { lowerer }
    }

    pub(crate) fn resolve_item_id(&self, name: &str) -> Option<DefId> {
        self.lowerer.resolver.resolve_item_or_alias(name).or_else(|| {
            self.lowerer
                .dependency_resolvers
                .values()
                .find_map(|resolver| resolver.resolve_item_or_alias(name))
        })
    }

    pub(crate) fn resolve_module_alias_or_item_id(&self, name: &str) -> Option<DefId> {
        self.lowerer
            .resolver
            .module_aliases
            .get(name)
            .copied()
            .or_else(|| self.resolve_item_id(name))
    }

    pub(crate) fn resolve_item_id_for_path(&self, name: &str) -> Option<DefId> {
        self.resolve_item_id(name).or_else(|| {
            self.lowerer.current_crate_name.as_ref().and_then(|crate_name| {
                self.resolve_item_id(&format!("{}::{}", crate_name, name))
            })
        })
    }

    pub(crate) fn canonical_name(&self, id: DefId) -> Option<&str> {
        self.lowerer.resolver.canonical_name(id).or_else(|| {
            self.lowerer
                .dependency_resolvers
                .values()
                .find_map(|resolver| resolver.canonical_name(id))
        })
    }

    pub(crate) fn canonical_name_for_item(&self, name: &str) -> Option<String> {
        let id = self.resolve_item_id(name)?;
        self.canonical_name(id).map(ToString::to_string)
    }

    pub(crate) fn canonical_name_for_module_alias_or_item(&self, name: &str) -> Option<String> {
        let id = self.resolve_module_alias_or_item_id(name)?;
        self.canonical_name(id).map(ToString::to_string)
    }
}
```

Keep the tests from Step 1 below the implementation.

- [ ] **Step 4: Delegate existing `Lowerer` helpers to the resolution context**

In `lib/src/lower/mod.rs`, replace the bodies of these existing methods with delegation:

```rust
pub(crate) fn resolve_item_def_id(&self, name: &str) -> Option<DefId> {
    crate::lower::resolution::LowerResolutionContext::new(self).resolve_item_id(name)
}

pub(crate) fn resolve_module_alias_or_item_def_id(&self, name: &str) -> Option<DefId> {
    crate::lower::resolution::LowerResolutionContext::new(self)
        .resolve_module_alias_or_item_id(name)
}

pub(crate) fn resolve_item_def_id_for_path(&self, name: &str) -> Option<DefId> {
    crate::lower::resolution::LowerResolutionContext::new(self).resolve_item_id_for_path(name)
}

pub(crate) fn canonical_name_for_def_id(&self, id: DefId) -> Option<&str> {
    crate::lower::resolution::LowerResolutionContext::new(self).canonical_name(id)
}

pub(crate) fn canonical_name_for_alias_or_item(&self, name: &str) -> Option<String> {
    crate::lower::resolution::LowerResolutionContext::new(self).canonical_name_for_item(name)
}

pub(crate) fn canonical_name_for_alias_or_item_lossy(&self, name: &str) -> String {
    self.canonical_name_for_alias_or_item(name)
        .unwrap_or_else(|| name.to_string())
}

pub(crate) fn canonical_name_for_module_alias_or_item_lossy(&self, name: &str) -> String {
    crate::lower::resolution::LowerResolutionContext::new(self)
        .canonical_name_for_module_alias_or_item(name)
        .unwrap_or_else(|| name.to_string())
}
```

- [ ] **Step 5: Run Task 1 tests**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_current_and_dependency_items -- --exact
cargo test -p rock-lib lower::resolution::tests::resolution_context_prefers_module_alias_before_root_item_when_requested -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit Task 1**

Run:

```bash
git add lib/src/lower/mod.rs lib/src/lower/resolution.rs
git commit -m "add lower resolution context"
```

---

## Task 2: Move Nominal Type And Trait Resolution Behind The Boundary

**Files:**
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/lower/mod.rs`
- Test: `lib/src/lower/resolution.rs`
- Test: `lib/src/lower/types.rs`

- [ ] **Step 1: Add failing nominal resolution tests**

Append these tests to `lib/src/lower/resolution.rs` tests:

```rust
use crate::hir::{HirEnum, HirStruct, HirTrait};
use std::collections::HashMap;

fn test_struct(id: DefId, name: &str) -> HirStruct {
    HirStruct {
        id,
        name: name.to_string(),
        generic_params: Vec::new(),
        fields: Vec::new(),
    }
}

fn test_enum(id: DefId, name: &str) -> HirEnum {
    HirEnum {
        id,
        name: name.to_string(),
        generic_params: Vec::new(),
        variants: Vec::new(),
    }
}

fn test_trait(id: DefId, name: &str) -> HirTrait {
    HirTrait {
        id,
        name: name.to_string(),
        generic_params: Vec::new(),
        associated_types: Vec::new(),
        methods: HashMap::new(),
        signatures: HashMap::new(),
    }
}

#[test]
fn resolution_context_resolves_nominals_by_module_alias_first() {
    let root_id = def_id(0, 40);
    let module_id = def_id(0, 41);
    let mut lowerer = Lowerer::new();
    lowerer
        .structs
        .insert("Thing".to_string(), test_struct(root_id, "Thing"));
    lowerer.structs.insert(
        "demo::helper::Thing".to_string(),
        test_struct(module_id, "demo::helper::Thing"),
    );
    lowerer.resolver.item_paths.insert("Thing".to_string(), root_id);
    lowerer
        .resolver
        .item_names_by_id
        .insert(root_id, "Thing".to_string());
    lowerer.resolver.insert_module_alias_with_name(
        "Thing".to_string(),
        "demo::helper::Thing".to_string(),
        module_id,
    );

    let resolution = LowerResolutionContext::new(&lowerer);
    let resolved = resolution.resolve_struct_type("Thing").unwrap();

    assert_eq!(resolved.id, module_id);
}

#[test]
fn resolution_context_resolves_enum_and_trait_by_id() {
    let enum_id = def_id(0, 50);
    let trait_id = def_id(0, 51);
    let mut lowerer = Lowerer::new();
    lowerer
        .enums
        .insert("demo::Choice".to_string(), test_enum(enum_id, "demo::Choice"));
    lowerer
        .traits
        .insert("demo::Show".to_string(), test_trait(trait_id, "demo::Show"));
    lowerer
        .resolver
        .item_paths
        .insert("Choice".to_string(), enum_id);
    lowerer
        .resolver
        .item_paths
        .insert("Show".to_string(), trait_id);
    lowerer
        .resolver
        .item_names_by_id
        .insert(enum_id, "demo::Choice".to_string());
    lowerer
        .resolver
        .item_names_by_id
        .insert(trait_id, "demo::Show".to_string());

    let resolution = LowerResolutionContext::new(&lowerer);

    assert_eq!(resolution.resolve_enum_type("Choice").unwrap().id, enum_id);
    assert_eq!(resolution.resolve_trait_type("Show").unwrap().id, trait_id);
}
```

- [ ] **Step 2: Run nominal tests to verify failure**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_nominals_by_module_alias_first -- --exact
```

Expected: FAIL to compile because `resolve_struct_type`, `resolve_enum_type`, and `resolve_trait_type` do not exist.

- [ ] **Step 3: Implement nominal resolution methods**

Add these methods inside `impl<'a> LowerResolutionContext<'a>`:

```rust
pub(crate) fn resolve_struct_type(&self, name: &str) -> Option<crate::hir::HirStruct> {
    self.resolve_module_alias_or_item_id(name)
        .and_then(|id| {
            self.lowerer
                .structs
                .values()
                .find(|structure| structure.id == id)
                .cloned()
        })
        .or_else(|| self.lowerer.structs.get(name).cloned())
}

pub(crate) fn resolve_enum_type(&self, name: &str) -> Option<crate::hir::HirEnum> {
    self.resolve_module_alias_or_item_id(name)
        .and_then(|id| {
            self.lowerer
                .enums
                .values()
                .find(|enum_def| enum_def.id == id)
                .cloned()
        })
        .or_else(|| self.lowerer.enums.get(name).cloned())
}

pub(crate) fn resolve_trait_type(&self, name: &str) -> Option<crate::hir::HirTrait> {
    self.resolve_module_alias_or_item_id(name)
        .and_then(|id| {
            self.lowerer
                .traits
                .values()
                .find(|trait_def| trait_def.id == id)
                .cloned()
        })
        .or_else(|| self.lowerer.traits.get(name).cloned())
}
```

- [ ] **Step 4: Update type lowering to use resolution methods without suffix fallback**

In `lib/src/lower/types.rs`, replace `lookup_struct_type`, `lookup_enum_type`, and `lookup_trait_type` implementations with:

```rust
fn lookup_struct_type(&self, name: &str) -> Option<HirStruct> {
    crate::lower::resolution::LowerResolutionContext::new(self).resolve_struct_type(name)
}

fn lookup_enum_type(&self, name: &str) -> Option<HirEnum> {
    crate::lower::resolution::LowerResolutionContext::new(self).resolve_enum_type(name)
}

fn lookup_trait_type(&self, name: &str) -> Option<HirTrait> {
    crate::lower::resolution::LowerResolutionContext::new(self).resolve_trait_type(name)
}
```

Delete `struct_by_module_alias_or_resolved_name`, `enum_by_module_alias_or_resolved_name`, and `trait_by_module_alias_or_resolved_name` from `types.rs`.

- [ ] **Step 5: Keep `Lowerer` nominal helpers as private compatibility or delete unused ones**

In `lib/src/lower/mod.rs`, replace `trait_by_name`, `struct_by_resolved_name`, `enum_by_resolved_name`, and `trait_by_resolved_name` bodies with resolution-context delegation while they still have callers:

```rust
pub(crate) fn trait_by_name(&self, name: &str) -> Option<&HirTrait> {
    let id = crate::lower::resolution::LowerResolutionContext::new(self)
        .resolve_trait_type(name)?
        .id;
    self.trait_by_id(id)
}

pub(crate) fn struct_by_resolved_name(&self, name: &str) -> Option<&HirStruct> {
    let id = crate::lower::resolution::LowerResolutionContext::new(self)
        .resolve_struct_type(name)?
        .id;
    self.structs.values().find(|structure| structure.id == id)
}

pub(crate) fn enum_by_resolved_name(&self, name: &str) -> Option<&HirEnum> {
    let id = crate::lower::resolution::LowerResolutionContext::new(self)
        .resolve_enum_type(name)?
        .id;
    self.enums.values().find(|enum_def| enum_def.id == id)
}

pub(crate) fn trait_by_resolved_name(&self, name: &str) -> Option<&HirTrait> {
    let id = crate::lower::resolution::LowerResolutionContext::new(self)
        .resolve_trait_type(name)?
        .id;
    self.trait_by_id(id)
}
```

- [ ] **Step 6: Run Task 2 tests**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_nominals_by_module_alias_first -- --exact
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_enum_and_trait_by_id -- --exact
cargo test -p rock-lib lower::types::tests::lower_struct_type_prefers_module_local_alias_over_root_type -- --exact
cargo test -p rock-lib lower::types::tests::lower_enum_type_prefers_module_local_alias_over_root_type -- --exact
cargo test -p rock-lib lower::types::tests::lookup_trait_type_prefers_module_local_alias_over_root_trait -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 2**

Run:

```bash
git add lib/src/lower/resolution.rs lib/src/lower/types.rs lib/src/lower/mod.rs
git commit -m "resolve nominal types through lower resolution context"
```

---

## Task 3: Move Single-Identifier Value Resolution Behind The Boundary

**Files:**
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/expression.rs`
- Test: `lib/src/lower/resolution.rs`
- Test: `lib/src/lower/paths.rs`
- Test: `lib/src/lower/expression.rs`

- [ ] **Step 1: Add failing value-resolution tests**

Append these tests to `lib/src/lower/resolution.rs` tests:

```rust
use crate::hir::{HirBlock, HirFunction, HirParam, HirVarTarget};
use crate::ids::HirLocalId;
use crate::types::Type;

fn test_function(id: DefId, name: &str) -> HirFunction {
    HirFunction {
        id,
        name: name.to_string(),
        qualified_name: None,
        generic_params: Vec::new(),
        generic_param_ids: Vec::new(),
        generic_bounds: HashMap::new(),
        params: Vec::<HirParam>::new(),
        ret_type: Type::I64,
        body: HirBlock {
            stmts: Vec::new(),
            ty: Type::I64,
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    }
}

#[test]
fn resolution_context_resolves_local_before_top_level_alias() {
    let function_id = def_id(0, 60);
    let mut lowerer = Lowerer::new();
    lowerer.functions.insert(
        "value".to_string(),
        test_function(function_id, "value"),
    );
    lowerer.resolver.item_paths.insert("value".to_string(), function_id);
    lowerer
        .resolver
        .item_names_by_id
        .insert(function_id, "value".to_string());
    lowerer
        .scope
        .define_local("value".to_string(), Type::Bool, true, HirLocalId(9));

    let resolution = LowerResolutionContext::new(&lowerer);
    let resolved = resolution.resolve_identifier_value("value").unwrap();

    assert_eq!(resolved.target, Some(HirVarTarget::Local(HirLocalId(9))));
    assert_eq!(resolved.name, "value");
    assert_eq!(resolved.ty, Type::Bool);
}

#[test]
fn resolution_context_resolves_alias_to_top_level_target() {
    let function_id = def_id(0, 61);
    let mut lowerer = Lowerer::new();
    lowerer.functions.insert(
        "demo::helper::value".to_string(),
        test_function(function_id, "demo::helper::value"),
    );
    lowerer.resolver.insert_module_alias_with_name(
        "value".to_string(),
        "demo::helper::value".to_string(),
        function_id,
    );
    lowerer.scope.define_alias(
        "value".to_string(),
        Type::Function(Vec::new(), Box::new(Type::I64)),
        false,
    );

    let resolution = LowerResolutionContext::new(&lowerer);
    let resolved = resolution.resolve_identifier_value("value").unwrap();

    assert_eq!(resolved.target, Some(HirVarTarget::Function(function_id)));
    assert_eq!(resolved.name, "demo::helper::value");
}
```

- [ ] **Step 2: Run value tests to verify failure**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_local_before_top_level_alias -- --exact
```

Expected: FAIL to compile because `resolve_identifier_value` and the value result type do not exist.

- [ ] **Step 3: Implement value result type and identifier value resolution**

Add this near the top of `resolution.rs`:

```rust
use crate::hir::HirVarTarget;
use crate::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LowerResolvedValue {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) target: Option<HirVarTarget>,
    pub(crate) is_alias: bool,
    pub(crate) scope_index: Option<usize>,
    pub(crate) should_instantiate: bool,
}
```

Add these methods to `LowerResolutionContext`:

```rust
pub(crate) fn resolve_identifier_value(&self, name: &str) -> Option<LowerResolvedValue> {
    if let Some(binding) = self.lowerer.scope.lookup(name).cloned() {
        if let Some(local_id) = binding.local_id {
            return Some(LowerResolvedValue {
                name: name.to_string(),
                ty: binding.ty,
                target: Some(HirVarTarget::Local(local_id)),
                is_alias: binding.is_alias,
                scope_index: self.lowerer.scope.binding_scope_index(name),
                should_instantiate: false,
            });
        }

        if binding.is_alias {
            if let Some(id) = self.resolve_module_alias_or_item_id(name) {
                if let Some(target) = self.top_level_var_target_by_id(id) {
                    return Some(LowerResolvedValue {
                        name: self
                            .canonical_name_for_module_alias_or_item(name)
                            .unwrap_or_else(|| name.to_string()),
                        ty: self.top_level_value_type(id).unwrap_or(binding.ty),
                        target: Some(target),
                        is_alias: true,
                        scope_index: self.lowerer.scope.binding_scope_index(name),
                        should_instantiate: true,
                    });
                }
            }
        }

        if self.lowerer.scope.binding_scope_index(name) == Some(0) {
            if let Some(id) = self.resolve_item_id(name) {
                if let Some(target) = self.top_level_var_target_by_id(id) {
                    return Some(LowerResolvedValue {
                        name: self
                            .canonical_name_for_item(name)
                            .unwrap_or_else(|| name.to_string()),
                        ty: self.top_level_value_type(id).unwrap_or(binding.ty),
                        target: Some(target),
                        is_alias: false,
                        scope_index: Some(0),
                        should_instantiate: true,
                    });
                }
            }
        }

        let should_instantiate = self.should_instantiate_type(&binding.ty);
        return Some(LowerResolvedValue {
            name: name.to_string(),
            ty: binding.ty,
            target: None,
            is_alias: binding.is_alias,
            scope_index: self.lowerer.scope.binding_scope_index(name),
            should_instantiate,
        });
    }

    let id = self.resolve_module_alias_or_item_id(name)?;
    let target = self.top_level_var_target_by_id(id)?;
    Some(LowerResolvedValue {
        name: self
            .canonical_name_for_module_alias_or_item(name)
            .unwrap_or_else(|| name.to_string()),
        ty: self.top_level_value_type(id)?,
        target: Some(target),
        is_alias: self.lowerer.resolver.module_aliases.contains_key(name),
        scope_index: None,
        should_instantiate: true,
    })
}

fn should_instantiate_type(&self, ty: &Type) -> bool {
    let mut generic_params = std::collections::HashSet::new();
    ty.collect_generic_params(&mut generic_params);
    !generic_params.is_empty()
}

pub(crate) fn top_level_var_target_by_id(&self, id: DefId) -> Option<HirVarTarget> {
    self.lowerer
        .functions
        .values()
        .find(|function| function.id == id)
        .map(|function| HirVarTarget::Function(function.id))
        .or_else(|| {
            self.lowerer
                .externs
                .iter()
                .find(|extern_| extern_.id == id)
                .map(|extern_| HirVarTarget::Extern(extern_.id))
        })
}

pub(crate) fn top_level_value_type(&self, id: DefId) -> Option<Type> {
    self.lowerer
        .functions
        .values()
        .find(|function| function.id == id)
        .map(|function| {
            Type::Function(
                function.params.iter().map(|param| param.ty.clone()).collect(),
                Box::new(function.ret_type.clone()),
            )
        })
        .or_else(|| {
            self.lowerer
                .externs
                .iter()
                .find(|extern_| extern_.id == id)
                .map(|extern_| Type::Function(extern_.params.clone(), Box::new(extern_.ret.clone())))
        })
}
```

- [ ] **Step 4: Delegate `top_level_var_target_by_def_id` and remove direct target lookup policy from `Lowerer`**

In `lib/src/lower/mod.rs`, replace `top_level_var_target_by_def_id` with:

```rust
pub(crate) fn top_level_var_target_by_def_id(&self, id: DefId) -> Option<HirVarTarget> {
    crate::lower::resolution::LowerResolutionContext::new(self).top_level_var_target_by_id(id)
}
```

- [ ] **Step 5: Migrate the single-identifier branch in `lower_identifier_path`**

In `lib/src/lower/paths.rs`, in the `if path.path.len() == 1` branch, keep the existing enum-variant/type-name/unknown diagnostics below the function check, but replace the first binding/function alias block with this pattern:

```rust
if let Some(resolved) = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_identifier_value(name)
{
    let ty = if resolved.should_instantiate {
        self.instantiate_generics(resolved.ty.clone())
    } else {
        resolved.ty.clone()
    };
    let kind = if let Some(target) = resolved.target {
        HirExprKind::ResolvedVar(HirVarRef {
            name: resolved.name,
            target,
        })
    } else {
        HirExprKind::Var(resolved.name)
    };
    return HirExpr {
        ty,
        kind,
        span,
    };
}
```

Do not delete enum variant and unknown-variable diagnostics in this task.

- [ ] **Step 6: Migrate custom operator lookup to the value resolution API**

In `lib/src/lower/expression.rs`, replace the custom-operator `func_binding` discovery block under `if trait_name.is_empty()` with:

```rust
let func_binding = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_identifier_value(op_str.as_str())
    .map(|resolved| {
        let func_ty = if resolved.should_instantiate {
            self.instantiate_generics(resolved.ty.clone())
        } else {
            resolved.ty.clone()
        };
        (func_ty, resolved.name, resolved.target)
    });
```

Keep the existing `if let Some((func_ty, hir_name, target)) = func_binding` block unchanged. This preserves local custom operators, module-local operator aliases, and unresolved custom-operator fallback behavior while removing direct resolver/function/scope search from expression lowering.

- [ ] **Step 7: Run Task 3 tests**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_local_before_top_level_alias -- --exact
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_alias_to_top_level_target -- --exact
cargo test -p rock-lib lower::paths::tests::lower_local_shadowing_module_local_alias_stays_var -- --exact
cargo test -p rock-lib lower::paths::tests::module_local_alias_resolves_through_resolver_without_lowerer_string_map -- --exact
cargo test -p rock-lib lower::expression::tests::custom_operator_module_alias_lowers_through_resolver_id -- --exact
cargo test -p rock-lib lower::expression::tests::custom_operator_scope_local_lowers_callee_to_local_call_target -- --exact
```

Expected: PASS.

- [ ] **Step 8: Commit Task 3**

Run:

```bash
git add lib/src/lower/resolution.rs lib/src/lower/mod.rs lib/src/lower/paths.rs lib/src/lower/expression.rs
git commit -m "resolve identifier values through lower resolution context"
```

---

## Task 4: Move Qualified Value, Variant, And Constructor Resolution Behind The Boundary

**Files:**
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/paths.rs`
- Test: `lib/src/lower/resolution.rs`
- Test: `lib/src/lower/paths.rs`

- [ ] **Step 1: Add failing qualified path tests**

Append this test to `lib/src/lower/resolution.rs` tests:

```rust
#[test]
fn resolution_context_resolves_qualified_function_after_first_segment_alias() {
    let function_id = def_id(0, 70);
    let mut lowerer = Lowerer::new();
    lowerer.functions.insert(
        "demo::helper::answer".to_string(),
        test_function(function_id, "demo::helper::answer"),
    );
    lowerer
        .resolver
        .item_paths
        .insert("demo::helper::answer".to_string(), function_id);
    lowerer
        .resolver
        .item_names_by_id
        .insert(function_id, "demo::helper::answer".to_string());
    lowerer.resolver.insert_module_alias_with_name(
        "Helper".to_string(),
        "demo::helper".to_string(),
        def_id(0, 71),
    );

    let resolution = LowerResolutionContext::new(&lowerer);
    let resolved = resolution
        .resolve_qualified_value_path(&["Helper".to_string(), "answer".to_string()])
        .unwrap();

    assert_eq!(resolved.target, Some(HirVarTarget::Function(function_id)));
    assert_eq!(resolved.name, "demo::helper::answer");
}
```

- [ ] **Step 2: Run qualified test to verify failure**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_qualified_function_after_first_segment_alias -- --exact
```

Expected: FAIL to compile because `resolve_qualified_value_path` does not exist.

- [ ] **Step 3: Implement first-segment canonicalization and qualified value resolution**

Add these methods to `LowerResolutionContext`:

```rust
pub(crate) fn canonicalize_first_path_segment(&self, segments: &[String]) -> Vec<String> {
    let Some((first, rest)) = segments.split_first() else {
        return Vec::new();
    };

    std::iter::once(
        self.canonical_name_for_module_alias_or_item(first)
            .unwrap_or_else(|| first.clone()),
    )
    .chain(rest.iter().cloned())
    .collect()
}

pub(crate) fn resolve_qualified_value_path(
    &self,
    segments: &[String],
) -> Option<LowerResolvedValue> {
    let canonical_segments = self.canonicalize_first_path_segment(segments);
    let qualified_name = canonical_segments.join("::");

    if let Some(binding) = self.lowerer.scope.lookup(&qualified_name).cloned() {
        if let Some(id) = self.resolve_item_id_for_path(&qualified_name) {
            if let Some(target) = self.top_level_var_target_by_id(id) {
                return Some(LowerResolvedValue {
                    name: self
                        .canonical_name(id)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| qualified_name.clone()),
                    ty: self.top_level_value_type(id).unwrap_or(binding.ty),
                    target: Some(target),
                    is_alias: binding.is_alias,
                    scope_index: self.lowerer.scope.binding_scope_index(&qualified_name),
                    should_instantiate: true,
                });
            }
        }

        let should_instantiate = self.should_instantiate_type(&binding.ty);
        let scope_index = self.lowerer.scope.binding_scope_index(&qualified_name);
        return Some(LowerResolvedValue {
            name: qualified_name,
            ty: binding.ty,
            target: None,
            is_alias: binding.is_alias,
            scope_index,
            should_instantiate,
        });
    }

    let id = self.resolve_item_id_for_path(&qualified_name)?;
    let target = self.top_level_var_target_by_id(id)?;
    Some(LowerResolvedValue {
        name: self
            .canonical_name(id)
            .map(ToString::to_string)
            .unwrap_or(qualified_name),
        ty: self.top_level_value_type(id)?,
        target: Some(target),
        is_alias: false,
        scope_index: None,
        should_instantiate: true,
    })
}
```

- [ ] **Step 4: Update multi-segment value path lowering to use the resolution boundary**

In `lib/src/lower/paths.rs`, replace direct first-segment canonicalization and repeated scope/function checks for `names.len() == 2` and `names.len() > 2` value paths with:

```rust
if let Some(resolved) = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_qualified_value_path(&names)
{
    let ty = if resolved.should_instantiate {
        self.instantiate_generics(resolved.ty)
    } else {
        resolved.ty
    };
    let kind = if let Some(target) = resolved.target {
        HirExprKind::ResolvedVar(HirVarRef {
            name: resolved.name,
            target,
        })
    } else {
        HirExprKind::Var(resolved.name)
    };
    return HirExpr {
        ty,
        kind,
        span,
    };
}
```

Keep enum-variant, static method, unexported-type, and unknown fallback branches in place until they move in later steps.

- [ ] **Step 5: Route instance first-segment name through resolution boundary**

In `lower_instance`, replace the local `resolved_first` construction with:

```rust
let canonical_segments =
    crate::lower::resolution::LowerResolutionContext::new(self)
        .canonicalize_first_path_segment(&segments);
let type_name = canonical_segments.join("::");
```

Use `canonical_segments[0]` where the code currently uses `resolved_first` for `Enum::Variant` checks.

- [ ] **Step 6: Run Task 4 tests**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_qualified_function_after_first_segment_alias -- --exact
cargo test -p rock-lib lower::paths::tests::module_local_struct_alias_shadows_root_struct_for_static_method -- --exact
cargo test -p rock-lib lower::paths::tests::module_local_struct_alias_shadows_root_struct_for_instance_construction -- --exact
cargo test -p rock-lib lower::paths::tests::module_local_enum_alias_shadows_root_enum_for_qualified_variant -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 4**

Run:

```bash
git add lib/src/lower/resolution.rs lib/src/lower/paths.rs
git commit -m "resolve qualified paths through lower resolution context"
```

---

## Task 5: Move Import, Glob Import, Artifact Root Export, And Module Alias Injection Resolution

**Files:**
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/lower/collect/declarations.rs`
- Test: `lib/src/lower/resolution.rs`
- Test: existing import/glob tests in `lib/src/lower/program.rs` and `lib/src/lower/crates/bodies.rs`

- [ ] **Step 1: Add failing artifact root export resolution test**

Append this test to `lib/src/lower/resolution.rs` tests:

```rust
use crate::crate_artifact::ArtifactExport;

#[test]
fn resolution_context_resolves_artifact_root_glob_targets_by_export_id() {
    let export_id = def_id(3, 5);
    let mut lowerer = Lowerer::new();
    lowerer.artifact_root_export_ids.insert(
        "dep".to_string(),
        HashMap::from([(
            "answer".to_string(),
            ArtifactExport {
                source: "dep::answer".to_string(),
                id: export_id,
            },
        )]),
    );

    let resolution = LowerResolutionContext::new(&lowerer);
    let targets = resolution.resolve_glob_import_targets(&["dep".to_string()]).unwrap();

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].short_name, "answer");
    assert_eq!(targets[0].source, "dep::answer");
    assert_eq!(targets[0].id, Some(export_id));
}
```

- [ ] **Step 2: Run artifact root test to verify failure**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_artifact_root_glob_targets_by_export_id -- --exact
```

Expected: FAIL to compile because `resolve_glob_import_targets` and `LowerImportTarget` do not exist.

- [ ] **Step 3: Implement import target data and artifact-root glob resolution**

Add this struct near `LowerResolvedValue`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LowerImportTarget {
    pub(crate) short_name: String,
    pub(crate) source: String,
    pub(crate) id: Option<DefId>,
}
```

Add this method to `LowerResolutionContext`:

```rust
pub(crate) fn resolve_glob_import_targets(
    &self,
    module_path: &[String],
) -> Option<Vec<LowerImportTarget>> {
    if module_path.len() != 1 {
        return None;
    }

    self.lowerer
        .artifact_root_export_ids
        .get(&module_path[0])
        .map(|exports| {
            exports
                .iter()
                .map(|(short_name, export)| LowerImportTarget {
                    short_name: short_name.clone(),
                    source: export.source.clone(),
                    id: Some(export.id),
                })
                .collect()
        })
}
```

- [ ] **Step 4: Move `glob_import_targets` artifact handling behind resolution**

In `lib/src/lower/program.rs`, update the artifact-root branch in `glob_import_targets` to call resolution:

```rust
if let Some(targets) = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_glob_import_targets(module_path)
{
    return Ok(targets
        .into_iter()
        .map(|target| (target.short_name, target.source))
        .collect());
}
```

In `handle_glob_import`, replace direct `artifact_root_export_ids` probing with the IDs returned by `resolve_glob_import_targets`:

```rust
let artifact_targets = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_glob_import_targets(module_path)
    .unwrap_or_default();
for target in &artifact_targets {
    if let Some(id) = target.id {
        self.resolver
            .insert_import_alias_with_name(target.short_name.clone(), target.source.clone(), id);
    }
}
```

Then continue importing each `(short_name, source)` returned by `glob_import_targets`.

- [ ] **Step 5: Add source-name qualification helper to resolution**

Move policy from `ModuleLoweringContext::qualify_export_source` and `Lowerer::qualify_export_source` into `resolution.rs`:

```rust
pub(crate) fn qualify_export_source(&self, resolved_prefix: &str, source: &str) -> String {
    let first_segment = source.split("::").next().unwrap_or(source);
    let is_absolute = self
        .lowerer
        .loaded_module_paths
        .iter()
        .any(|(name, _)| name == first_segment);

    if is_absolute {
        source.to_string()
    } else {
        format!("{}::{}", resolved_prefix, source)
    }
}

pub(crate) fn current_function_owns_import_name(
    &self,
    short_name: &str,
    imported_id: DefId,
) -> bool {
    self.lowerer.functions.get(short_name).is_some_and(|existing| {
        existing.id != imported_id
            && self.lowerer.current_def_ids.contains(&existing.id)
            && self.resolve_item_id(short_name) == Some(existing.id)
    })
}
```

Update call sites in `lower/program.rs` and `lower/collect/declarations.rs` to call `LowerResolutionContext::new(self).qualify_export_source(...)`.

In `lib/src/lower/program.rs`, update `import_qualified_name` so canonicalization and import collision checks use the resolution boundary:

```rust
let resolved_qualified_name = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_item_id_for_path(&qualified_name)
    .and_then(|id| {
        crate::lower::resolution::LowerResolutionContext::new(self)
            .canonical_name(id)
            .map(str::to_string)
    })
    .unwrap_or_else(|| qualified_name.clone());

let resolved_id = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_item_id_for_path(&resolved_qualified_name);
if resolved_id.is_some_and(|id| {
    crate::lower::resolution::LowerResolutionContext::new(self)
        .current_function_owns_import_name(&short_name, id)
}) {
    return;
}
```

Use that pattern in both the function-import branch and the scope-import branch, and delete or delegate the private `current_function_owns_name` helper so it no longer calls `self.resolver.resolve_item_or_alias` directly.

In `lib/src/lower/collect/declarations.rs`, replace export alias ID lookup with:

```rust
if let Some(id) = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_item_id(&qualified_source)
{
    self.resolver.insert_export_alias_with_name(
        qualified_export.clone(),
        qualified_source.clone(),
        id,
    );
}
```

- [ ] **Step 6: Add module-local alias insertion helper**

Add this method to `LowerResolutionContext`:

```rust
pub(crate) fn resolve_module_local_alias_target(&self, qualified: &str) -> Option<DefId> {
    self.resolve_item_id_for_path(qualified)
}

pub(crate) fn previous_module_alias_id(&self, short_name: &str) -> Option<DefId> {
    self.lowerer.resolver.module_aliases.get(short_name).copied()
}
```

In `lib/src/lower/crates/bodies.rs`, replace each direct `self.resolve_item_def_id_for_path(&qualified)` inside `inject_module_local_aliases` with:

```rust
crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_module_local_alias_target(&qualified)
```

Also replace each direct `self.resolver.module_aliases.get(&short).copied()` in that function with:

```rust
crate::lower::resolution::LowerResolutionContext::new(self)
    .previous_module_alias_id(&short)
```

- [ ] **Step 7: Run Task 5 tests**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_artifact_root_glob_targets_by_export_id -- --exact
cargo test -p rock-lib lower::program::tests::glob_import_targets_prefers_id_backed_artifact_root_exports -- --exact
cargo test -p rock-lib lower::module_context::tests::module_context_scopes_module_local_aliases_and_prefix -- --exact
cargo test -p rock-lib lower::module_context::tests::module_context_duplicate_module_alias_cleanup_restores_previous_resolver_state -- --exact
```

Expected: PASS.

- [ ] **Step 8: Commit Task 5**

Run:

```bash
git add lib/src/lower/resolution.rs lib/src/lower/program.rs lib/src/lower/crates/bodies.rs lib/src/lower/collect/declarations.rs
git commit -m "resolve imports through lower resolution context"
```

---

## Task 6: Move Builtin Trait And Owner Path Resolution Behind The Boundary

**Files:**
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/lower/bodies.rs`
- Modify: `lib/src/lower/crates/registration.rs`
- Test: `lib/src/lower/resolution.rs`
- Test: existing operator/index/trait tests

- [ ] **Step 1: Add failing builtin trait and owner resolution tests**

Append this test to `lib/src/lower/resolution.rs` tests:

```rust
#[test]
fn resolution_context_resolves_builtin_trait_and_owner_path_by_id() {
    let trait_id = def_id(0, 80);
    let owner_id = def_id(0, 81);
    let mut lowerer = Lowerer::new();
    lowerer
        .traits
        .insert("stdlib::sized::Sized".to_string(), test_trait(trait_id, "Sized"));
    lowerer
        .resolver
        .item_paths
        .insert("stdlib::sized::Sized".to_string(), trait_id);
    lowerer
        .resolver
        .item_names_by_id
        .insert(trait_id, "stdlib::sized::Sized".to_string());
    lowerer
        .resolver
        .item_paths
        .insert("demo::Box".to_string(), owner_id);
    lowerer
        .resolver
        .item_names_by_id
        .insert(owner_id, "demo::Box".to_string());

    let resolution = LowerResolutionContext::new(&lowerer);

    assert_eq!(resolution.resolve_builtin_trait("Sized").unwrap().id, trait_id);
    assert_eq!(resolution.try_canonical_owner_path("demo::Box"), Some("demo::Box".to_string()));
}
```

- [ ] **Step 2: Run builtin/owner test to verify failure**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_builtin_trait_and_owner_path_by_id -- --exact
```

Expected: FAIL to compile because `resolve_builtin_trait` and `try_canonical_owner_path` do not exist on `LowerResolutionContext`.

- [ ] **Step 3: Implement builtin trait and owner resolution**

Move the policy from `Lowerer::builtin_trait_by_name`, `stdlib_builtin_trait_def_id`, `stdlib_builtin_trait_module`, `try_canonical_owner_path`, and `resolve_owner_def_id` into `resolution.rs` as methods:

```rust
pub(crate) fn resolve_builtin_trait(&self, name: &str) -> Option<crate::hir::HirTrait> {
    let trait_id = self
        .lowerer
        .stdlib_prelude_export_ids
        .get(name)
        .map(|export| export.id)
        .or_else(|| self.stdlib_builtin_trait_id(name))?;

    self.lowerer
        .traits
        .values()
        .find(|trait_def| trait_def.id == trait_id)
        .cloned()
}

fn stdlib_builtin_trait_id(&self, name: &str) -> Option<DefId> {
    let module = Self::stdlib_builtin_trait_module(name)?;
    let canonical_path = format!("stdlib::{}::{}", module, name);

    self.resolve_item_id(&canonical_path).or_else(|| {
        self.lowerer
            .resolver
            .import_aliases
            .get(name)
            .copied()
            .filter(|id| {
                self.lowerer
                    .resolver
                    .canonical_name(*id)
                    .is_some_and(|source| source == canonical_path)
            })
    })
}

fn stdlib_builtin_trait_module(name: &str) -> Option<&'static str> {
    match name {
        "Bitwise" => Some("bitwise"),
        "Deref" => Some("deref"),
        "Eq" => Some("eq"),
        "Index" => Some("index"),
        "Neg" => Some("neg"),
        "Not" => Some("not"),
        "Num" => Some("num"),
        "Ord" => Some("ord"),
        "Sized" => Some("sized"),
        _ => None,
    }
}

pub(crate) fn try_canonical_owner_path(&self, name: &str) -> Option<String> {
    let mut candidates = vec![name.to_string()];

    if let Some(crate_name) = self.lowerer.current_crate_name.as_deref() {
        let crate_prefix = format!("{}::", crate_name);
        if let Some(stripped) = name.strip_prefix(&crate_prefix) {
            candidates.push(stripped.to_string());
        }
    }

    if let Some(short_name) = name.rsplit("::").next() {
        candidates.push(short_name.to_string());
    }

    if let Some(prefix) = self.current_module_prefix() {
        candidates.push(format!("{}::{}", prefix, name));
    }

    let def_id = candidates
        .into_iter()
        .find_map(|candidate| self.resolve_owner_id(&candidate))?;

    self.canonical_name(def_id).map(ToString::to_string)
}

pub(crate) fn resolve_owner_id(&self, candidate: &str) -> Option<DefId> {
    self.resolve_item_id(candidate).or_else(|| {
        candidate.rsplit("::").next().and_then(|short_name| {
            self.lowerer
                .stdlib_prelude_export_ids
                .get(short_name)
                .map(|export| export.id)
        })
    })
}
```

Also add this method if Task 5 has not already moved it:

```rust
pub(crate) fn current_module_prefix(&self) -> Option<String> {
    if let Some(prefix) = &self.lowerer.current_qualified_module_prefix {
        return Some(prefix.clone());
    }

    self.lowerer
        .loaded_module_paths
        .iter()
        .find(|(_, path)| path == &self.lowerer.current_module_path)
        .map(|(name, _)| name.clone())
        .or_else(|| self.lowerer.current_crate_name.clone())
}
```

Also add this prelude collision helper so stdlib prelude injection no longer probes raw resolver alias maps outside the boundary:

```rust
pub(crate) fn prelude_short_name_is_user_owned(&self, short_name: &str) -> bool {
    self.lowerer.resolver.item_paths.contains_key(short_name)
        || self.lowerer.resolver.import_aliases.contains_key(short_name)
}
```

- [ ] **Step 4: Delegate or remove `Lowerer` owner/builtin helpers**

In `lib/src/lower/mod.rs`, replace public helper bodies with delegation:

```rust
pub(crate) fn builtin_trait_by_name(&self, name: &str) -> Option<&HirTrait> {
    let id = crate::lower::resolution::LowerResolutionContext::new(self)
        .resolve_builtin_trait(name)?
        .id;
    self.trait_by_id(id)
}

pub(crate) fn canonical_owner_path(&self, name: &str) -> String {
    crate::lower::resolution::LowerResolutionContext::new(self)
        .try_canonical_owner_path(name)
        .unwrap_or_else(|| panic!("missing canonical owner path for impl type: {}", name))
}

pub(crate) fn try_canonical_owner_path(&self, name: &str) -> Option<String> {
    crate::lower::resolution::LowerResolutionContext::new(self).try_canonical_owner_path(name)
}
```

Remove `stdlib_builtin_trait_def_id`, `stdlib_builtin_trait_module`, `resolve_owner_def_id`, and `resolve_owner_def_id_inner` from `Lowerer` after call sites move to `LowerResolutionContext::resolve_owner_id`.

- [ ] **Step 5: Update direct owner ID call sites**

In `lib/src/lower/traits/conformance.rs`, replace direct `self.resolve_owner_def_id(...)` calls with:

```rust
crate::lower::resolution::LowerResolutionContext::new(self).resolve_owner_id(name)
```

or, when the code has a candidate string:

```rust
crate::lower::resolution::LowerResolutionContext::new(self).resolve_owner_id(candidate)
```

Keep the existing fallback structure unchanged.

In `lib/src/lower/crates/registration.rs`, replace the `short_name_is_user_owned` expression in `inject_stdlib_prelude` with:

```rust
let short_name_is_user_owned = crate::lower::resolution::LowerResolutionContext::new(self)
    .prelude_short_name_is_user_owned(&short_name);
```

- [ ] **Step 6: Run Task 6 tests**

Run:

```bash
cargo test -p rock-lib lower::resolution::tests::resolution_context_resolves_builtin_trait_and_owner_path_by_id -- --exact
cargo test -p rock-lib lower::traits::conformance::tests::auto_impl_sized_uses_stdlib_prelude_export_id -- --exact
cargo test -p rock-lib test_stdlib_index_trait_is_available_from_prelude -- --exact
cargo test -p rock-lib test_stdlib_deref_trait_is_available_from_prelude -- --exact
```

Expected: PASS.

- [ ] **Step 7: Commit Task 6**

Run:

```bash
git add lib/src/lower/resolution.rs lib/src/lower/mod.rs lib/src/lower/expression.rs lib/src/lower/control_flow/secondary.rs lib/src/lower/traits/conformance.rs lib/src/lower/bodies.rs lib/src/lower/crates/registration.rs
git commit -m "resolve builtin traits through lower resolution context"
```

---

## Task 7: Remove Broad Ad Hoc Resolution Access From Body/Type Lowering

**Files:**
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/crates/bodies.rs`
- Modify: `lib/src/lower/collect/declarations.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: architecture audit commands

- [ ] **Step 1: Run the architecture audit before cleanup**

Run:

```bash
rg "dependency_resolvers|resolver\.(import_aliases|export_aliases|module_aliases|resolve_item_or_alias)|canonical_name_for_alias_or_item|canonical_name_for_module_alias_or_item|resolve_module_alias_or_item_def_id|resolve_item_def_id_for_path|resolve_item_def_id|struct_by_resolved_name|enum_by_resolved_name|trait_by_resolved_name|trait_by_name|rsplit\(\"::\"\).*next" lib/src/lower --glob '*.rs'
```

Expected: FAIL audit. Matches remain outside `lib/src/lower/resolution.rs` and test code.

- [ ] **Step 2: Move direct `resolver.module_aliases` use in `paths.rs` to context helpers**

Delete `canonical_name_for_module_alias_or_name` from `paths.rs`. Replace call sites with:

```rust
crate::lower::resolution::LowerResolutionContext::new(self)
    .canonical_name_for_module_alias_or_item(name)
    .unwrap_or_else(|| name.to_string())
```

For path segments, prefer `canonicalize_first_path_segment` from Task 4.

- [ ] **Step 3: Move `should_instantiate_scoped_identifier` policy into resolution context**

Add this method to `LowerResolutionContext`:

```rust
pub(crate) fn should_instantiate_scoped_identifier(
    &self,
    name: &str,
    binding_scope: Option<usize>,
) -> bool {
    binding_scope == Some(0) || self.resolve_item_id(name).is_some()
}
```

Replace `self.should_instantiate_scoped_identifier(...)` in `paths.rs` with:

```rust
crate::lower::resolution::LowerResolutionContext::new(self)
    .should_instantiate_scoped_identifier(name, binding_scope)
```

Delete `should_instantiate_scoped_identifier` from `paths.rs`.

- [ ] **Step 4: Remove suffix-name semantic fallbacks from type lookup**

Verify `types.rs` no longer has this pattern:

```rust
.find(|(struct_name, _)| struct_name.rsplit("::").next() == Some(name))
```

If the pattern remains, delete it and keep only `LowerResolutionContext::resolve_struct_type`, `resolve_enum_type`, and `resolve_trait_type`.

- [ ] **Step 5: Restrict remaining direct scope access to local binding lifetime and injection**

Inspect all `self.scope.lookup` hits in `lib/src/lower/*.rs` and `lib/src/lower/**/*.rs`. Leave direct scope access only when the function is defining locals, checking mutability/lifetime, capturing locals, or injecting a scope alias. Move any lookup that decides source/path/name target identity into `LowerResolutionContext`.

Allowed direct scope patterns after this task:

```rust
self.scope.define(...)
self.scope.define_local(...)
self.scope.define_alias(...)
self.scope.push()
self.scope.pop()
self.scope.lookup(...) // only for local variable mutation, closure capture, or alias injection type retrieval
```

- [ ] **Step 6: Move selection, method-lookup, and conformance audit hits behind resolution APIs**

Add this enum near the other resolution result types in `lib/src/lower/resolution.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LowerNominalKind {
    Struct,
    Enum,
}
```

Add these methods to `LowerResolutionContext`:

```rust
pub(crate) fn resolver_tables_for_selection(
    &self,
) -> (
    &crate::collect::resolver::ResolverTables,
    &std::collections::HashMap<String, crate::collect::resolver::ResolverTables>,
) {
    (&self.lowerer.resolver, &self.lowerer.dependency_resolvers)
}

pub(crate) fn canonical_type_name_for_method_lookup(&self, ty: &Type) -> Option<String> {
    match ty {
        Type::Struct { id, .. } | Type::Enum { id, .. } => self
            .canonical_name(*id)
            .map(str::to_string)
            .or_else(|| {
                self.lowerer
                    .structs
                    .iter()
                    .find_map(|(name, strukt)| (strukt.id == *id).then_some(name.clone()))
            })
            .or_else(|| {
                self.lowerer
                    .enums
                    .iter()
                    .find_map(|(name, enum_)| (enum_.id == *id).then_some(name.clone()))
            })
            .or_else(|| crate::lower::Lowerer::get_type_name_for_method_lookup(ty)),
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
            Some(ty.to_string())
        }
        Type::Reference { inner, .. } => self.canonical_type_name_for_method_lookup(inner),
        _ => crate::lower::Lowerer::get_type_name_for_method_lookup(ty),
    }
}

pub(crate) fn method_lookup_type_names(&self, ty: &Type) -> Vec<String> {
    match ty {
        Type::Struct { .. } | Type::Enum { .. } => {
            let mut names: Vec<_> = self
                .canonical_type_name_for_method_lookup(ty)
                .into_iter()
                .collect();
            if let Some(short_name) = names
                .first()
                .and_then(|name| name.rsplit("::").next())
                .filter(|short_name| Some(*short_name) != names.first().map(String::as_str))
            {
                names.push(short_name.to_string());
            }
            names
        }
        Type::Slice(_) => vec!["Array".to_string()],
        Type::Array(_, _) => vec![ty.to_string(), "Array".to_string()],
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
            let mut names = vec![ty.to_string()];
            names.extend(self.method_lookup_type_names(inner));
            names
        }
        Type::Reference { inner, .. } => self.method_lookup_type_names(inner),
        _ => crate::lower::Lowerer::get_type_name_for_method_lookup(ty)
            .into_iter()
            .collect(),
    }
}

pub(crate) fn resolve_trait_id(&self, name: &str) -> Option<DefId> {
    self.resolve_trait_type(name).map(|trait_def| trait_def.id)
}

pub(crate) fn resolve_nominal_type_id(
    &self,
    type_name: &str,
    resolved_id: Option<DefId>,
    kind: LowerNominalKind,
) -> Option<DefId> {
    if let Some(id) = resolved_id {
        let matches_kind = match kind {
            LowerNominalKind::Struct => self.lowerer.structs.values().any(|strukt| strukt.id == id),
            LowerNominalKind::Enum => self.lowerer.enums.values().any(|enum_def| enum_def.id == id),
        };
        return matches_kind.then_some(id);
    }

    match kind {
        LowerNominalKind::Struct => self.resolve_struct_type(type_name).map(|strukt| strukt.id),
        LowerNominalKind::Enum => self.resolve_enum_type(type_name).map(|enum_def| enum_def.id),
    }
}
```

Update `lib/src/lower/control_flow/secondary.rs` `selection_service` to get resolver tables through the boundary:

```rust
let resolution = crate::lower::resolution::LowerResolutionContext::new(self);
let (resolver, dependency_resolvers) = resolution.resolver_tables_for_selection();
crate::selection::SelectionService::new(
    &self.traits,
    &self.impls,
    &self.methods,
    resolver,
    dependency_resolvers,
    self.current_trait.as_deref(),
    &self.current_impl_bounds,
)
```

Update `lib/src/lower/types_helpers/helpers.rs` contextual lookup methods to delegate:

```rust
pub(crate) fn get_type_name_for_method_lookup_in_context(&self, ty: &Type) -> Option<String> {
    crate::lower::resolution::LowerResolutionContext::new(self)
        .canonical_type_name_for_method_lookup(ty)
}

pub(crate) fn get_type_names_for_method_lookup_in_context(&self, ty: &Type) -> Vec<String> {
    crate::lower::resolution::LowerResolutionContext::new(self).method_lookup_type_names(ty)
}
```

Update `lib/src/lower/expression.rs` operator method lookup to consume names from the boundary. Replace the local `type_name`/`rsplit("::")` expansion:

```rust
let type_name = self.get_type_name_for_method_lookup_in_context(&resolved_ty);
if let Some(tn) = type_name.clone() {
    let mut lookup_type_names = vec![tn.clone()];
    if let Some(short_type_name) = tn.rsplit("::").next() {
        if short_type_name != tn {
            lookup_type_names.push(short_type_name.to_string());
        }
    }
```

with:

```rust
let lookup_type_names = crate::lower::resolution::LowerResolutionContext::new(self)
    .method_lookup_type_names(&resolved_ty);
if !lookup_type_names.is_empty() {
```

Keep the existing `let found_method = ...`, method-call construction, and missing-operator diagnostics inside that `if` block unchanged.

Update `lib/src/lower/traits/conformance.rs` to import `LowerNominalKind` and replace trait/nominal ID lookup with boundary calls:

```rust
use crate::lower::resolution::LowerNominalKind;
```

```rust
let id = imp
    .trait_name
    .as_deref()
    .and_then(|trait_name| {
        crate::lower::resolution::LowerResolutionContext::new(self).resolve_trait_id(trait_name)
    });
let canonical = id.and_then(|id| {
    crate::lower::resolution::LowerResolutionContext::new(self)
        .canonical_name(id)
        .map(str::to_string)
});
```

```rust
let enum_id = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_nominal_type_id(&imp.type_name, resolved_owner_id, LowerNominalKind::Enum);
let struct_id = crate::lower::resolution::LowerResolutionContext::new(self)
    .resolve_nominal_type_id(&imp.type_name, resolved_owner_id, LowerNominalKind::Struct);
```

Delete the local `nominal_id_for_type_name` helper after the replacement.

In `lib/src/lower/mod.rs`, replace internal `self.resolve_item_def_id(...)` uses inside `def_id_for_name` and `lower_struct_field_type` with direct `LowerResolutionContext::new(self).resolve_item_id(...)` calls.

- [ ] **Step 7: Classify intentional resolver mutation sites**

Leave direct resolver mutation in code that records or restores resolver state, not resolution policy. The final audit may still show these mutation-only sites:

```text
lib/src/lower/crates/registration.rs register_crate_resolvers: writes dependency resolver tables from crate metadata.
lib/src/lower/module_context.rs with_module_local_aliases: restores previous module alias IDs after a scoped module-lowering visit.
```

Do not move these writes into `LowerResolutionContext`; the context is read-only by design. Include them in the final architecture audit classification if they remain.

- [ ] **Step 8: Run architecture audit again**

Run:

```bash
rg "dependency_resolvers|resolver\.(import_aliases|export_aliases|module_aliases|resolve_item_or_alias)|canonical_name_for_alias_or_item|canonical_name_for_module_alias_or_item|resolve_module_alias_or_item_def_id|resolve_item_def_id_for_path|resolve_item_def_id|struct_by_resolved_name|enum_by_resolved_name|trait_by_resolved_name|trait_by_name|rsplit\(\"::\"\).*next" lib/src/lower --glob '*.rs'
```

Expected: PASS audit by classification. Matches are allowed only in:

```text
lib/src/lower/resolution.rs
lib/src/lower/* tests
Lowerer delegation methods that exist only for non-migrated tests and call `LowerResolutionContext`
display-only diagnostic code not used for semantic resolution
mutation-only registration/restoration sites listed in Step 7
```

Record the exact classification in this plan's final verification notes.

- [ ] **Step 9: Run focused tests**

Run:

```bash
cargo test -p rock-lib alias
cargo test -p rock-lib lower::types::tests::lower_nominal_type_prefers_resolver_alias_id_over_suffix_match -- --exact
cargo test -p rock-lib lower::paths::tests::lower_local_shadowing_module_local_alias_stays_var -- --exact
cargo test -p rock-lib lower::expression::tests::custom_operator_module_alias_shadows_root_operator_with_same_symbol -- --exact
```

Expected: PASS.

- [ ] **Step 10: Commit Task 7**

Run:

```bash
git add lib/src/lower
git commit -m "centralize lower resolution policy"
```

---

## Task 8: Final Docs, Verification, And Review

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-28-complete-task-5-lower-resolution-boundary.md`

- [x] **Step 1: Update ordered roadmap Task 5 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the Task 5 row from:

```markdown
| 5. Source/path resolution out of `Lowerer` | Complete for strict alias/path-resolution subset | Lowerer semantic alias maps are removed, alias/path lookups consume canonical resolver/product/artifact IDs, and product format `22` rejects old string-only alias contracts | Broader path/type/module ownership extraction remains; `Lowerer` still owns scope, module caches, resolver tables, dependency resolvers, prelude policy, and other decomposition state |
```

to:

```markdown
| 5. Source/path resolution out of `Lowerer` | Complete | Body/type lowering routes source/path/name lookup through `LowerResolutionContext`; lower-side value, nominal type, trait, import, prelude, dependency, module-prefix, and artifact-root-export resolution policy is centralized behind intent-specific APIs | Broader source-loader ownership and lowerer state-shell decomposition remain Task 17/18 work |
```

Also update the Task 5 detailed status paragraph from:

```markdown
**Status:** Complete for strict alias/path-resolution subset. Lowerer semantic alias maps are removed and persistent alias/path-resolution contracts are ID-backed; broader source/path/type/module ownership extraction remains open.
```

to:

```markdown
**Status:** Complete. Body/type lowering consumes source/path/name resolution through `LowerResolutionContext`; semantic aliases and canonical path/type/trait/import/prelude/dependency/artifact lookups are resolved behind lower-side intent APIs. Broader source-loader ownership and full `Lowerer` state-shell decomposition remain Task 17/18 work.
```

- [x] **Step 2: Update master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, move this item from `Still to do` to `Done` in section 2:

```markdown
- [x] Split source/path/name resolution policy out of `Lowerer`; body and type lowering consume lower resolution context results for source/path, alias, module-prefix, prelude, dependency, and top-level target resolution while direct scope lookup remains limited to local binding, capture, mutability, and alias-injection state.
```

Remove the unchecked copy from `Still to do`.

- [x] **Step 3: Run final focused checks**

Run:

```bash
cargo test -p rock-lib alias
cargo test -p rock-lib product_artifact
cargo test -p rock-lib prelude_export
```

Expected: PASS.

- [x] **Step 4: Run final full verification**

Run:

```bash
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Expected: PASS.

- [x] **Step 5: Run final architecture audit**

Run:

```bash
rg "dependency_resolvers|resolver\.(import_aliases|export_aliases|module_aliases|resolve_item_or_alias)|canonical_name_for_alias_or_item|canonical_name_for_module_alias_or_item|resolve_module_alias_or_item_def_id|resolve_item_def_id_for_path|resolve_item_def_id|struct_by_resolved_name|enum_by_resolved_name|trait_by_resolved_name|trait_by_name|rsplit\(\"::\"\).*next" lib/src/lower --glob '*.rs'
```

Expected: remaining matches are in `lib/src/lower/resolution.rs`, tests, explicit compatibility delegations that call `LowerResolutionContext` without owning policy, display-only diagnostics, or mutation-only registration/restoration sites classified in Task 7 Step 7.

- [x] **Step 6: Append final verification notes to this plan**

Append:

```markdown
## Final Verification

- Focused tests: `cargo test -p rock-lib alias`, `cargo test -p rock-lib product_artifact`, and `cargo test -p rock-lib prelude_export` passed.
- Full verification: `cargo test -p rock-lib`, `cargo fmt --all --check`, and `git diff --check` passed.
- Architecture audit command:

```bash
rg "dependency_resolvers|resolver\.(import_aliases|export_aliases|module_aliases|resolve_item_or_alias)|canonical_name_for_alias_or_item|canonical_name_for_module_alias_or_item|resolve_module_alias_or_item_def_id|resolve_item_def_id_for_path|resolve_item_def_id|struct_by_resolved_name|enum_by_resolved_name|trait_by_resolved_name|trait_by_name|rsplit\(\"::\"\).*next" lib/src/lower --glob '*.rs'
```

- Architecture audit classification: remaining matches are centralized in `lib/src/lower/resolution.rs`, tests, explicit compatibility delegations into `LowerResolutionContext`, display-only diagnostics, or mutation-only registration/restoration sites; no body/type lowering path owns source/path/name resolution policy directly.
- Final code review status: pending.
```

- [x] **Step 7: Commit docs and verification notes**

Run:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-28-complete-task-5-lower-resolution-boundary.md
git commit -m "mark task 5 lower resolution complete"
```

- [ ] **Step 8: Request final code review**

Use `requesting-code-review` with this review scope:

```text
Review complete Task 5 lower resolution boundary. Verify body/type lowering no longer owns source/path/name resolution policy; resolution is centralized in `LowerResolutionContext`; Task 17/18 work remains correctly scoped; tests and docs match the implementation.
```

- [ ] **Step 9: Address final review findings**

If the reviewer reports Critical or Important findings, fix them with TDD and repeat focused/full verification before reporting completion.

If the reviewer reports no Critical or Important findings, update this plan's final verification notes with the review result and commit that note.

## Final Verification

- Focused tests: `cargo test -p rock-lib alias` passed with 83 unit tests and 2 integration tests; `cargo test -p rock-lib product_artifact` passed with 79 unit tests; `cargo test -p rock-lib prelude_export` passed with 12 unit tests.
- Full verification: `cargo test -p rock-lib` passed with 1340 unit tests, 277 integration tests, 1 parser fixture test, and doctests reporting 1 passed / 1 ignored; `cargo fmt --all --check` passed; `git diff --check` passed.
- Architecture audit command:

```bash
rg "dependency_resolvers|resolver\.(import_aliases|export_aliases|module_aliases|resolve_item_or_alias)|canonical_name_for_alias_or_item|canonical_name_for_module_alias_or_item|resolve_module_alias_or_item_def_id|resolve_item_def_id_for_path|resolve_item_def_id|struct_by_resolved_name|enum_by_resolved_name|trait_by_resolved_name|trait_by_name|rsplit\(\"::\"\).*next" lib/src/lower --glob '*.rs'
```

- Architecture audit classification: `lib/src/lower/resolution.rs` contains centralized resolution policy/tests; `lib/src/lower/mod.rs` contains `Lowerer` state fields/constructors, explicit compatibility delegations into `LowerResolutionContext`, tests, `trait_by_name` delegation, and display-only private-field diagnostics; `lib/src/lower/module_context.rs` contains mutation-only module-alias restoration and tests; `lib/src/lower/crates/registration.rs` contains dependency resolver registration; `lib/src/lower/program.rs` contains tests; `lib/src/lower/control_flow/secondary.rs` contains selection handoff names returned by `resolver_tables_for_selection`; `lib/src/lower/traits/conformance.rs` contains test setup registration.
- Final code review status: pending.
