# Type Lowering Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract parsed type syntax conversion into one shared type-lowering boundary used by both `Lowerer` and `CollectContext` while keeping the existing structural `Type` model.

**Architecture:** Add a small `TypeLowerer` service with a `TypeLoweringContext` adapter trait. `Lowerer` and `CollectContext` keep their public `lower_parse_type` wrappers, but those wrappers delegate to the shared service. This preserves current behavior and leaves the `Ty` / type-context migration for Roadmap Task 9.

**Tech Stack:** Rust 2021, existing `rock-lib` modules, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## File Structure

- Create: `lib/src/type_lowering.rs`
  - Owns shared parsed-type to `Type` conversion.
  - Defines `TypeLoweringContext` and `TypeLowerer`.
  - Contains focused unit tests for the context adapter contract and service behavior.
- Modify: `lib/src/lib.rs`
  - Registers `type_lowering` as a crate-internal module.
- Modify: `lib/src/lower/types.rs`
  - Implements `TypeLoweringContext` for `Lowerer`.
  - Keeps `Lowerer::lower_parse_type` and `Lowerer::lower_parse_type_inner` as delegating wrappers.
  - Removes the duplicated conversion implementation from `Lowerer`.
- Modify: `lib/src/collect/context.rs`
  - Implements `TypeLoweringContext` for `CollectContext`.
  - Keeps `CollectContext::lower_parse_type` and `CollectContext::lower_parse_type_inner` as delegating wrappers.
  - Removes the duplicated conversion implementation from `CollectContext`.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Marks Task 8 complete for the extracted type-lowering boundary.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Updates the Type Context And Semantic Types audit row/section for completed parsed-type lowering extraction.

## Task 1: Add Shared Type-Lowering Service

**Files:**
- Create: `lib/src/type_lowering.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Add the module declaration and failing service tests**

In `lib/src/lib.rs`, add the module declaration near the existing compiler modules:

```rust
pub(crate) mod type_lowering;
```

Create `lib/src/type_lowering.rs` with this initial test-first skeleton:

```rust
use crate::ast;
use crate::hir::{HirEnum, HirStruct, HirTrait};
use crate::types::Type;

pub(crate) trait TypeLoweringContext {
    fn push_type_error(&mut self, message: String);
    fn current_module_prefix(&self) -> Option<String>;
    fn current_trait_name(&self) -> Option<String>;
    fn lookup_struct_type(&self, name: &str) -> Option<HirStruct>;
    fn lookup_enum_type(&self, name: &str) -> Option<HirEnum>;
    fn lookup_trait_type(&self, name: &str) -> Option<HirTrait>;
    fn generic_type_for_name(&mut self, name: &str) -> Type;
}

pub(crate) struct TypeLowerer;

impl TypeLowerer {
    pub(crate) fn lower_parse_type<C: TypeLoweringContext + ?Sized>(
        _context: &mut C,
        _parse_type: &ast::ParseType,
    ) -> Type {
        Type::Error
    }

    pub(crate) fn lower_parse_type_inner<C: TypeLoweringContext + ?Sized>(
        _context: &mut C,
        _inner: &ast::ParseTypeInner,
    ) -> Type {
        Type::Error
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    use crate::ast::{ParseType, ParseTypeInner};
    use crate::hir::{HirAssociatedTypeDecl, HirTrait};
    use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId};
    use crate::lexer::Span;
    use crate::types::{AssociatedTypeKey, GenericParamId};

    #[derive(Default)]
    struct TestTypeContext {
        errors: Vec<String>,
        traits: HashMap<String, HirTrait>,
        current_trait: Option<String>,
        generic_owner: Option<DefId>,
        generic_params: Vec<String>,
    }

    impl TypeLoweringContext for TestTypeContext {
        fn push_type_error(&mut self, message: String) {
            self.errors.push(message);
        }

        fn current_module_prefix(&self) -> Option<String> {
            None
        }

        fn current_trait_name(&self) -> Option<String> {
            self.current_trait.clone()
        }

        fn lookup_struct_type(&self, _name: &str) -> Option<HirStruct> {
            None
        }

        fn lookup_enum_type(&self, _name: &str) -> Option<HirEnum> {
            None
        }

        fn lookup_trait_type(&self, name: &str) -> Option<HirTrait> {
            self.traits.get(name).cloned()
        }

        fn generic_type_for_name(&mut self, name: &str) -> Type {
            let Some(owner) = self.generic_owner else {
                self.push_type_error(format!("unknown type '{}'", name));
                return Type::Error;
            };
            let Some(index) = self
                .generic_params
                .iter()
                .position(|param| param == name)
                .or_else(|| {
                    let mut chars = name.chars();
                    let is_implicit_generic = matches!(chars.next(), Some(ch) if ch.is_ascii_uppercase())
                        && chars.next().is_none();
                    if is_implicit_generic {
                        self.generic_params.push(name.to_string());
                        Some(self.generic_params.len() - 1)
                    } else {
                        None
                    }
                })
            else {
                self.push_type_error(format!("unknown type '{}'", name));
                return Type::Error;
            };

            Type::Generic(GenericParamId {
                owner,
                index: index as u32,
            })
        }
    }

    fn named_type(name: &str) -> ParseType {
        ParseType::Type(ParseTypeInner {
            name: name.to_string(),
            generics: vec![],
            span: Span::default(),
        })
    }

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn type_lowerer_reports_bare_slice_error() {
        let mut context = TestTypeContext::default();

        let ty = TypeLowerer::lower_parse_type(
            &mut context,
            &ParseType::Slice(Box::new(named_type("I64"))),
        );

        assert_eq!(ty, Type::Error);
        assert!(context
            .errors
            .iter()
            .any(|message| message.contains("bare slice type [T]")));
    }

    #[test]
    fn type_lowerer_lowers_associated_projection_with_trait_identity() {
        let trait_id = def_id(10);
        let assoc_type_id = AssocTypeId(2);
        let mut context = TestTypeContext {
            current_trait: Some("Iterator".to_string()),
            generic_owner: Some(trait_id),
            generic_params: vec!["Self".to_string()],
            ..TestTypeContext::default()
        };
        context.traits.insert(
            "Iterator".to_string(),
            HirTrait {
                id: trait_id,
                name: "Iterator".to_string(),
                generic_params: vec!["Self".to_string()],
                associated_types: vec![HirAssociatedTypeDecl {
                    id: assoc_type_id,
                    name: "Item".to_string(),
                }],
                methods: HashMap::new(),
                signatures: HashMap::new(),
            },
        );

        let ty = TypeLowerer::lower_parse_type(
            &mut context,
            &ParseType::Associated {
                base: ParseTypeInner {
                    name: "Self".to_string(),
                    generics: vec![],
                    span: Span::default(),
                },
                member: ParseTypeInner {
                    name: "Item".to_string(),
                    generics: vec![],
                    span: Span::default(),
                },
            },
        );

        assert_eq!(
            ty,
            Type::Projection {
                ty: Box::new(Type::Generic(GenericParamId {
                    owner: trait_id,
                    index: 0,
                })),
                trait_id,
                assoc_type: AssociatedTypeKey {
                    owner: trait_id,
                    assoc_type_id,
                },
                trait_args: vec![Type::Generic(GenericParamId {
                    owner: trait_id,
                    index: 0,
                })],
            }
        );
        assert!(context.errors.is_empty());
    }

    #[test]
    fn type_lowerer_creates_implicit_generic_in_context() {
        let owner = def_id(20);
        let mut context = TestTypeContext {
            generic_owner: Some(owner),
            ..TestTypeContext::default()
        };

        let ty = TypeLowerer::lower_parse_type(&mut context, &named_type("T"));

        assert_eq!(
            ty,
            Type::Generic(GenericParamId { owner, index: 0 })
        );
        assert_eq!(context.generic_params, vec!["T".to_string()]);
        assert!(context.errors.is_empty());
    }
}
```

- [ ] **Step 2: Run the new focused tests and verify they fail**

Run:

```bash
cargo test -p rock-lib type_lowering -- --nocapture
```

Expected: FAIL. The skeleton returns `Type::Error`, so the projection and implicit-generic tests fail.

- [ ] **Step 3: Implement the shared service**

Replace the `impl TypeLowerer` block in `lib/src/type_lowering.rs` with this implementation, and update the import list to include `AssociatedTypeKey` and `GenericParamId`:

```rust
use crate::types::{AssociatedTypeKey, GenericParamId, Type};

impl TypeLowerer {
    pub(crate) fn lower_parse_type<C: TypeLoweringContext + ?Sized>(
        context: &mut C,
        parse_type: &ast::ParseType,
    ) -> Type {
        Self::lower_parse_type_with_slice_context(context, parse_type, false)
    }

    pub(crate) fn lower_parse_type_inner<C: TypeLoweringContext + ?Sized>(
        context: &mut C,
        inner: &ast::ParseTypeInner,
    ) -> Type {
        Self::lower_parse_type_inner_with_slice_context(context, inner, false)
    }

    fn lower_parse_type_with_slice_context<C: TypeLoweringContext + ?Sized>(
        context: &mut C,
        parse_type: &ast::ParseType,
        allow_bare_slice: bool,
    ) -> Type {
        match parse_type {
            ast::ParseType::Unit => Type::Unit,
            ast::ParseType::Type(inner) => {
                Self::lower_parse_type_inner_with_slice_context(context, inner, allow_bare_slice)
            }
            ast::ParseType::Associated { base, member } => {
                let base_ty = Self::lower_parse_type_inner(context, base);
                let trait_name = if base.name == "Self" {
                    context
                        .current_trait_name()
                        .unwrap_or_else(|| base.name.clone())
                } else {
                    base.name.clone()
                };
                let Some(trait_def) = context.lookup_trait_type(&trait_name) else {
                    context.push_type_error(format!(
                        "unknown trait '{}' in associated type",
                        trait_name
                    ));
                    return Type::Error;
                };
                let Some(assoc_type) = trait_def
                    .associated_types
                    .iter()
                    .find(|assoc| assoc.name == member.name)
                    .map(|assoc| AssociatedTypeKey {
                        owner: trait_def.id,
                        assoc_type_id: assoc.id,
                    })
                else {
                    context.push_type_error(format!(
                        "trait '{}' has no associated type '{}'",
                        trait_name, member.name
                    ));
                    return Type::Error;
                };
                let trait_args = trait_def
                    .generic_params
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        Type::Generic(GenericParamId {
                            owner: trait_def.id,
                            index: index as u32,
                        })
                    })
                    .collect();

                Type::Projection {
                    ty: Box::new(base_ty),
                    trait_id: trait_def.id,
                    assoc_type,
                    trait_args,
                }
            }
            ast::ParseType::Function(types) => {
                if types.is_empty() {
                    return Type::Unit;
                }
                if types.len() == 1 {
                    return Self::lower_parse_type_with_slice_context(context, &types[0], false);
                }
                if types.len() == 2
                    && matches!(&types[0], ast::ParseType::Tuple(elems) if elems.is_empty())
                {
                    let ret = Self::lower_parse_type_with_slice_context(context, &types[1], false);
                    return Type::Function(Vec::new(), Box::new(ret));
                }
                let params: Vec<Type> = types[..types.len() - 1]
                    .iter()
                    .map(|ty| Self::lower_parse_type_with_slice_context(context, ty, false))
                    .collect();
                let ret = Self::lower_parse_type_with_slice_context(
                    context,
                    &types[types.len() - 1],
                    false,
                );
                Type::Function(params, Box::new(ret))
            }
            ast::ParseType::Slice(inner) => {
                if !allow_bare_slice {
                    context.push_type_error("bare slice type [T] must be written behind a reference, such as &[T] or &mut [T]".to_string());
                    return Type::Error;
                }
                Type::Slice(Box::new(Self::lower_parse_type_with_slice_context(
                    context, inner, false,
                )))
            }
            ast::ParseType::Array { inner, len } => {
                let inner_ty = Self::lower_parse_type_with_slice_context(context, inner, false);
                Type::Array(Box::new(inner_ty), *len)
            }
            ast::ParseType::Tuple(elems) => Type::Tuple(
                elems
                    .iter()
                    .map(|ty| Self::lower_parse_type_with_slice_context(context, ty, false))
                    .collect(),
            ),
            ast::ParseType::Reference { is_mut, pointee } => Type::Reference {
                mutable: *is_mut,
                inner: Box::new(Self::lower_parse_type_with_slice_context(
                    context, pointee, true,
                )),
            },
            ast::ParseType::Pointer(inner) => Type::Pointer(Box::new(
                Self::lower_parse_type_with_slice_context(context, inner, true),
            )),
        }
    }

    fn lower_parse_type_inner_with_slice_context<C: TypeLoweringContext + ?Sized>(
        context: &mut C,
        inner: &ast::ParseTypeInner,
        allow_bare_slice: bool,
    ) -> Type {
        let generics: Vec<Type> = inner
            .generics
            .iter()
            .map(|generic| Self::lower_parse_type(context, generic))
            .collect();

        match inner.name.as_str() {
            "I8" | "i8" => Type::I8,
            "I16" | "i16" => Type::I16,
            "I32" | "i32" | "Int" => Type::I32,
            "I64" | "i64" => Type::I64,
            "U8" | "u8" => Type::U8,
            "U16" | "u16" => Type::U16,
            "U32" | "u32" => Type::U32,
            "U64" | "u64" => Type::U64,
            "F32" | "f32" => Type::F32,
            "F64" | "f64" | "Float" => Type::F64,
            "Bool" | "bool" => Type::Bool,
            "Char" | "char" => Type::Char,
            "Unit" => Type::Unit,
            "Str" => {
                if !allow_bare_slice {
                    context.push_type_error(
                        "bare string slice type Str must be written behind a reference, such as &Str"
                            .to_string(),
                    );
                    return Type::Error;
                }
                Type::Str
            }
            name => {
                let current_module_name = context
                    .current_module_prefix()
                    .map(|prefix| format!("{}::{}", prefix, name));

                if let Some(struct_def) = current_module_name
                    .as_deref()
                    .and_then(|name| context.lookup_struct_type(name))
                    .or_else(|| context.lookup_struct_type(name))
                {
                    Type::Struct {
                        id: struct_def.id,
                        args: generics,
                    }
                } else if let Some(enum_def) = current_module_name
                    .as_deref()
                    .and_then(|name| context.lookup_enum_type(name))
                    .or_else(|| context.lookup_enum_type(name))
                {
                    Type::Enum {
                        id: enum_def.id,
                        args: generics,
                    }
                } else {
                    context.generic_type_for_name(name)
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run the focused tests and commit**

Run:

```bash
cargo test -p rock-lib type_lowering -- --nocapture
```

Expected: PASS with 3 `type_lowering` tests.

Commit:

```bash
git add lib/src/lib.rs lib/src/type_lowering.rs
git commit -m "add shared type lowering service"
```

## Task 2: Route `Lowerer` Through The Shared Service

**Files:**
- Modify: `lib/src/lower/types.rs`

- [ ] **Step 1: Add a lowerer adapter regression test**

Append this test to the existing `#[cfg(test)] mod tests` in `lib/src/lower/types.rs`:

```rust
#[test]
fn lower_parse_type_creates_implicit_generic_in_current_context() {
    let owner = def_id(60);
    let mut lowerer = Lowerer::new();
    lowerer.current_generic_owner = Some(owner);

    let ty = lowerer.lower_parse_type(&named_type("T"));

    assert_eq!(
        ty,
        Type::Generic(crate::types::GenericParamId { owner, index: 0 })
    );
    assert_eq!(lowerer.current_generic_params, vec!["T".to_string()]);
    assert!(lowerer.errors.is_empty());
}
```

- [ ] **Step 2: Run the lowerer type tests before changing the adapter**

Run:

```bash
cargo test -p rock-lib lower::types -- --nocapture
```

Expected: PASS before the refactor. This test documents behavior that must still pass after the adapter extraction.

- [ ] **Step 3: Replace `Lowerer` type-lowering logic with adapter delegation**

In `lib/src/lower/types.rs`, replace the top imports with:

```rust
use crate::ast;
use crate::hir::{HirEnum, HirStruct, HirTrait};
use crate::type_lowering::{TypeLowerer, TypeLoweringContext};
use crate::types::Type;

use crate::lower::Lowerer;
```

Replace the existing `impl Lowerer` type-lowering methods with this adapter plus wrappers:

```rust
impl TypeLoweringContext for Lowerer {
    fn push_type_error(&mut self, message: String) {
        self.push_error(message);
    }

    fn current_module_prefix(&self) -> Option<String> {
        Lowerer::current_module_prefix(self)
    }

    fn current_trait_name(&self) -> Option<String> {
        self.current_trait.clone()
    }

    fn lookup_struct_type(&self, name: &str) -> Option<HirStruct> {
        self.struct_by_resolved_name(name).cloned().or_else(|| {
            self.structs
                .iter()
                .find(|(struct_name, _)| struct_name.rsplit("::").next() == Some(name))
                .map(|(_, struct_def)| struct_def.clone())
        })
    }

    fn lookup_enum_type(&self, name: &str) -> Option<HirEnum> {
        self.enum_by_resolved_name(name).cloned().or_else(|| {
            self.enums
                .iter()
                .find(|(enum_name, _)| enum_name.rsplit("::").next() == Some(name))
                .map(|(_, enum_def)| enum_def.clone())
        })
    }

    fn lookup_trait_type(&self, name: &str) -> Option<HirTrait> {
        self.trait_by_name(name).cloned()
    }

    fn generic_type_for_name(&mut self, name: &str) -> Type {
        self.current_generic_type_for_name(name)
            .unwrap_or(Type::Error)
    }
}

impl Lowerer {
    pub(crate) fn lower_parse_type(&mut self, parse_type: &ast::ParseType) -> Type {
        TypeLowerer::lower_parse_type(self, parse_type)
    }

    pub(crate) fn lower_parse_type_inner(&mut self, inner: &ast::ParseTypeInner) -> Type {
        TypeLowerer::lower_parse_type_inner(self, inner)
    }
}
```

Remove the old `lower_parse_type_with_slice_context` and `lower_parse_type_inner_with_slice_context` methods from `lib/src/lower/types.rs`.

- [ ] **Step 4: Run lowerer-focused verification**

Run:

```bash
cargo test -p rock-lib lower::types -- --nocapture
cargo test -p rock-lib lower::tests::trait_by_name_prefers_resolver_alias_id -- --exact
```

Expected: PASS. The first command should include the new implicit-generic adapter test plus the existing nominal, slice, string, and array tests.

- [ ] **Step 5: Commit the lowerer adapter**

Commit:

```bash
git add lib/src/lower/types.rs
git commit -m "route lowerer type lowering through service"
```

## Task 3: Route `CollectContext` Through The Shared Service

**Files:**
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Add collection adapter regression tests**

Append this test module to the end of `lib/src/collect/context.rs` after the `impl CollectContext` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{ParseType, ParseTypeInner};
    use crate::ids::{CrateId, DefId, LocalDefId};
    use crate::lexer::Span;

    fn named_type(name: &str) -> ParseType {
        ParseType::Type(ParseTypeInner {
            name: name.to_string(),
            generics: vec![],
            span: Span::default(),
        })
    }

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn collect_context_lower_parse_type_reports_bare_slice_error() {
        let mut context = CollectContext::new();

        let ty = context.lower_parse_type(&ParseType::Slice(Box::new(named_type("I64"))));

        assert_eq!(ty, Type::Error);
        assert!(context
            .errors
            .iter()
            .any(|error| error.message.contains("bare slice type [T]")));
    }

    #[test]
    fn collect_context_lower_parse_type_creates_implicit_generic() {
        let owner = def_id(70);
        let mut context = CollectContext::new();
        context.current_generic_owner = Some(owner);

        let ty = context.lower_parse_type(&named_type("T"));

        assert_eq!(
            ty,
            Type::Generic(GenericParamId { owner, index: 0 })
        );
        assert_eq!(context.current_generic_params, vec!["T".to_string()]);
        assert!(context.errors.is_empty());
    }
}
```

- [ ] **Step 2: Run the collection tests before changing the adapter**

Run:

```bash
cargo test -p rock-lib collect::context -- --nocapture
```

Expected: PASS before the refactor. These tests document collection-side behavior that must survive delegation.

- [ ] **Step 3: Implement `TypeLoweringContext` for `CollectContext`**

In `lib/src/collect/context.rs`, update imports to include the shared service and remove the old `AssociatedTypeKey` import that only the duplicated implementation used:

```rust
use crate::type_lowering::{TypeLowerer, TypeLoweringContext};
use crate::types::{GenericParamId, Type};
```

Insert this trait implementation after the `LocalCollection` struct definition:

```rust
impl TypeLoweringContext for CollectContext {
    fn push_type_error(&mut self, message: String) {
        self.push_error(message);
    }

    fn current_module_prefix(&self) -> Option<String> {
        CollectContext::current_module_prefix(self)
    }

    fn current_trait_name(&self) -> Option<String> {
        self.current_trait.clone()
    }

    fn lookup_struct_type(&self, name: &str) -> Option<HirStruct> {
        self.structs.get(name).cloned().or_else(|| {
            self.structs
                .iter()
                .find(|(struct_name, _)| struct_name.rsplit("::").next() == Some(name))
                .map(|(_, struct_def)| struct_def.clone())
        })
    }

    fn lookup_enum_type(&self, name: &str) -> Option<HirEnum> {
        self.enums.get(name).cloned().or_else(|| {
            self.enums
                .iter()
                .find(|(enum_name, _)| enum_name.rsplit("::").next() == Some(name))
                .map(|(_, enum_def)| enum_def.clone())
        })
    }

    fn lookup_trait_type(&self, name: &str) -> Option<HirTrait> {
        self.trait_by_name(name).cloned()
    }

    fn generic_type_for_name(&mut self, name: &str) -> Type {
        CollectContext::generic_type_for_name(self, name)
    }
}
```

- [ ] **Step 4: Replace the duplicated collection type-lowering methods**

In the existing `impl CollectContext`, replace `lower_parse_type` and `lower_parse_type_inner` with delegating wrappers:

```rust
pub(crate) fn lower_parse_type(&mut self, parse_type: &ast::ParseType) -> Type {
    TypeLowerer::lower_parse_type(self, parse_type)
}

pub(crate) fn lower_parse_type_inner(&mut self, inner: &ast::ParseTypeInner) -> Type {
    TypeLowerer::lower_parse_type_inner(self, inner)
}
```

Remove the old `lower_parse_type_with_slice_context` and `lower_parse_type_inner_with_slice_context` methods from `lib/src/collect/context.rs`.

- [ ] **Step 5: Run collection and cross-phase verification**

Run:

```bash
cargo test -p rock-lib collect::context -- --nocapture
cargo test -p rock-lib lower::collect::traits -- --nocapture
cargo test -p rock-lib lower::collect::types -- --nocapture
cargo test -p rock-lib crate_artifact::tests::test_compile_projection_trait_identity_from_product_artifact -- --exact
```

Expected: PASS. The collection tests should include the new adapter tests; the lower collection tests cover declarations and generic/where-clause flows; the artifact projection test covers artifact-loaded projection identity.

- [ ] **Step 6: Commit the collection adapter**

Commit:

```bash
git add lib/src/collect/context.rs
git commit -m "route collection type lowering through service"
```

## Task 4: Remove Duplicate Type-Lowering Policy And Run Full Verification

**Files:**
- Modify only if needed: `lib/src/type_lowering.rs`, `lib/src/lower/types.rs`, `lib/src/collect/context.rs`

- [ ] **Step 1: Run duplicate-policy grep gates**

Run:

```bash
rg "lower_parse_type_with_slice_context|lower_parse_type_inner_with_slice_context" lib/src
rg "bare string slice type Str|bare slice type \[T\]" lib/src/lower lib/src/collect lib/src/type_lowering.rs
```

Expected: the first command has no matches. The second command should show the bare slice and bare `Str` diagnostics only in `lib/src/type_lowering.rs` plus any tests that assert the messages.

- [ ] **Step 2: Run focused semantic verification**

Run:

```bash
cargo test -p rock-lib lower::types -- --nocapture
cargo test -p rock-lib collect::context -- --nocapture
cargo test -p rock-lib lower::collect -- --nocapture
cargo test -p rock-lib crate_artifact::tests::test_compile_projection_trait_identity_from_product_artifact -- --exact
cargo test -p rock-lib crate_artifact::load::product_tests::product_artifact_remaps_projection_type_ids -- --exact
```

Expected: PASS. These commands cover lowerer wrappers, collection wrappers, declaration/header collection, artifact-loaded projections, and product projection remapping.

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

Expected: PASS. Full `rock-lib` should report all unit, integration, and doc tests passing with only the repository's existing ignored tests.

- [ ] **Step 4: Commit any verification-driven cleanup**

If Step 1 through Step 3 required code cleanup, commit only the touched implementation files:

```bash
git add lib/src/type_lowering.rs lib/src/lower/types.rs lib/src/collect/context.rs
git commit -m "clean up duplicated type lowering policy"
```

If no code changed after Task 3, do not create an empty commit.

## Task 5: Update Architecture Trackers

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Update the ordered roadmap Task 8 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the 2026-05-18 rebaseline notes with this new bullet after the Tasks 6-7 bullet:

```markdown
- Roadmap Task 8 has landed for the type-lowering ownership boundary: parsed type syntax conversion now lives behind a shared type-lowering service used by both collection and lowering while retaining the existing structural `Type` representation.
```

Update the Task 8 status line to:

```markdown
**Status:** Complete for parsed-type lowering extraction in `docs/superpowers/plans/2026-05-18-type-lowering-boundary.md`.
```

- [ ] **Step 2: Update the master audit checklist type row and section**

In `docs/superpowers/plans/master-audit-checklist.md`, update the Type Context And Semantic Types summary row to:

```markdown
| Type Context And Semantic Types | In progress | `1700c60`, `lib/src/types/mod.rs`, `lib/src/type_lowering.rs`, `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md` | Semantic identity is ID-backed and parsed type lowering is shared, but `Ty` / authoritative `TypeId` and dedicated type fact services remain deferred |
```

In section `## 3. Type Context And Semantic Types`, add this Done entry:

```markdown
- [x] Moved parsed type lowering and generic type creation policy out of `Lowerer` into a shared type-lowering boundary used by collection and lowering.
```

In the same section's Still to do list, replace this entry:

```markdown
- [ ] Move parsed type lowering and generic type creation out of `Lowerer` into a type-lowering/type-context boundary.
```

with:

```markdown
- [ ] Decide whether the shared parsed-type lowering boundary should become part of the future `Ty` / type-context service once Task 9 chooses the type representation model.
```

Update `Checked against implementation commit:` to the final implementation commit SHA for Task 8.

- [ ] **Step 3: Verify docs and commit**

Run:

```bash
git diff --check
rg "Move parsed type lowering and generic type creation out of `Lowerer`" docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
```

Expected: `git diff --check` passes. The `rg` command should have no matches for the stale unchecked wording.

Commit:

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for type lowering boundary"
```

## Task 6: Final Review And Completion Gates

**Files:**
- Review only unless a verification failure exposes a direct issue.

- [ ] **Step 1: Inspect final status and recent commits**

Run:

```bash
git status --short
git log --oneline -8
```

Expected: status is clean before final verification, and the recent commits include the type-lowering service, lowerer adapter, collection adapter, and tracker update commits.

- [ ] **Step 2: Run final verification gates**

Run:

```bash
cargo fmt --all --check
git diff --check
rg "lower_parse_type_with_slice_context|lower_parse_type_inner_with_slice_context" lib/src
rg "bare string slice type Str|bare slice type \[T\]" lib/src/lower lib/src/collect lib/src/type_lowering.rs
cargo test -p rock-lib
```

Expected:

```text
cargo fmt --all --check: PASS
git diff --check: PASS
first rg gate: no matches
second rg gate: diagnostics centralized in lib/src/type_lowering.rs plus test assertions only
cargo test -p rock-lib: PASS
```

- [ ] **Step 3: Request final code review**

Dispatch a reviewer for the complete Task 8 implementation range. The review prompt should ask for spec compliance and quality findings around:

```text
- shared type-lowering boundary ownership
- behavior preservation for Lowerer and CollectContext
- generic owner/index handling
- associated type projection identity
- bare slice and bare Str diagnostics
- resolver/import/module-local alias lookup behavior
- same-name and artifact-loaded type declarations
- accidental start of Ty/type-context migration
```

Expected: reviewer returns `APPROVED` or findings are fixed in a new commit followed by the focused and full verification commands above.

- [ ] **Step 4: Report completion with evidence**

Report the final commit range, verification results, and any reviewer findings. Do not claim completion unless Step 2 and Step 3 provide passing evidence.
