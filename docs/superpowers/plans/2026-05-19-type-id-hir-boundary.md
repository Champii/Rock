# TypeId HIR Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a resolved-HIR `TypeId` sidecar backed by `TypeContext`, proving selected HIR phase-boundary types can be addressed by interned type identity while structural `Type` remains the compatibility and product-artifact representation.

**Architecture:** Introduce `hir::type_ids` as a focused sidecar module with stable `HirTypeLocation` keys and `HirTypeIds` lookup storage. Build the sidecar by walking finalized `HirProgram` values after inference, intern every selected structural `Type` through `TypeContext`, and attach both the context and sidecar to `ResolvedHirProgram`. Product artifacts continue cloning and serializing structural HIR only.

**Tech Stack:** Rust 2021, existing `rock-lib` HIR/inference/product modules, `TypeContext`, `TypeId`, `cargo test -p rock-lib`, `cargo fmt --all`, `git diff --check`.

---

## File Structure

- Create: `lib/src/hir/type_ids.rs`
  - Owns `HirTypeLocation`, `HirTypeIds`, and HIR type collection helpers.
- Modify: `lib/src/hir/mod.rs`
  - Registers and re-exports the HIR type-ID sidecar module.
- Modify: `lib/src/infer/mod.rs`
  - Adds `TypeContext` and `HirTypeIds` to `ResolvedHirProgram`, provides sidecar accessors, and constructs the sidecar after finalization.
- Modify: `lib/src/products.rs`
  - Updates test `ResolvedHirProgram` construction through the new constructor and adds product compatibility coverage proving artifacts remain structural.
- Modify after implementation proof: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Marks Roadmap Task 11 complete for the HIR sidecar slice only.
- Modify after implementation proof: `docs/superpowers/plans/master-audit-checklist.md`
  - Updates audit evidence and keeps downstream structural-to-`TypeId` migration as remaining work.

## Task 1: Add HIR Type-ID Sidecar Shapes

**Files:**
- Create: `lib/src/hir/type_ids.rs`
- Modify: `lib/src/hir/mod.rs`

- [ ] **Step 1: Register the module and write the failing sidecar API test**

In `lib/src/hir/mod.rs`, add this module declaration near the top of the file after the crate imports:

```rust
mod type_ids;

pub use type_ids::{HirTypeIds, HirTypeLocation};
```

Create `lib/src/hir/type_ids.rs` with this test-only starting content:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CrateId, DefId, LocalDefId};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn hir_type_ids_record_and_read_locations() {
        let function = def_id(1);
        let location = HirTypeLocation::FunctionReturn { function };
        let id = crate::ids::TypeId(7);
        let mut ids = HirTypeIds::new();

        assert!(ids.is_empty());
        ids.insert(location.clone(), id);

        assert_eq!(ids.get(&location), Some(id));
        assert_eq!(ids.len(), 1);
        assert!(!ids.is_empty());
        assert_eq!(ids.iter().collect::<Vec<_>>(), vec![(&location, id)]);
    }
}
```

- [ ] **Step 2: Run the sidecar API test to verify it fails**

Run: `cargo test -p rock-lib hir_type_ids_record_and_read_locations`

Expected: FAIL with unresolved `HirTypeLocation` and `HirTypeIds` in `lib/src/hir/type_ids.rs`.

- [ ] **Step 3: Implement the sidecar location and storage API**

Insert this implementation above the `#[cfg(test)]` module in `lib/src/hir/type_ids.rs`:

```rust
use std::collections::HashMap;

use crate::ids::{AssocTypeId, DefId, FieldId, TypeId, VariantId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HirTypeLocation {
    FunctionReturn { function: DefId },
    FunctionParam { function: DefId, index: usize },
    ExternReturn { extern_id: DefId },
    ExternParam { extern_id: DefId, index: usize },
    StructField { owner: DefId, field: FieldId },
    EnumVariantNamedField {
        owner: DefId,
        variant: VariantId,
        field: FieldId,
    },
    EnumVariantPositionalField {
        owner: DefId,
        variant: VariantId,
        index: usize,
    },
    TraitSignatureReturn { trait_id: DefId, signature: DefId },
    TraitSignatureParam {
        trait_id: DefId,
        signature: DefId,
        index: usize,
    },
    ImplReceiverArg { impl_id: DefId, index: usize },
    ImplTraitArg { impl_id: DefId, index: usize },
    AssociatedTypeDef {
        impl_id: DefId,
        assoc_type: AssocTypeId,
    },
    Block { owner: DefId, path: Vec<usize> },
    LetStmt {
        owner: DefId,
        path: Vec<usize>,
        name: String,
    },
    Expr { owner: DefId, path: Vec<usize> },
    ClosureCapture {
        owner: DefId,
        path: Vec<usize>,
        name: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct HirTypeIds {
    ids: HashMap<HirTypeLocation, TypeId>,
}

impl HirTypeIds {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, location: HirTypeLocation, id: TypeId) {
        if let Some(existing) = self.ids.insert(location.clone(), id) {
            debug_assert_eq!(
                existing, id,
                "duplicate HIR type location with different TypeId: {:?}",
                location
            );
        }
    }

    pub fn get(&self, location: &HirTypeLocation) -> Option<TypeId> {
        self.ids.get(location).copied()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&HirTypeLocation, TypeId)> {
        self.ids.iter().map(|(location, id)| (location, *id))
    }
}
```

- [ ] **Step 4: Run the sidecar API test to verify it passes**

Run: `cargo test -p rock-lib hir_type_ids_record_and_read_locations`

Expected: PASS.

- [ ] **Step 5: Commit the sidecar API**

```bash
git add lib/src/hir/mod.rs lib/src/hir/type_ids.rs
git commit -m "add hir type id sidecar shapes"
```

## Task 2: Collect Top-Level HIR Type Locations

**Files:**
- Modify: `lib/src/hir/type_ids.rs`
- Modify: `lib/src/hir/mod.rs`

- [ ] **Step 1: Add failing top-level collection tests**

In `lib/src/hir/mod.rs`, update the sidecar re-export to include the collector:

```rust
pub use type_ids::{collect_hir_type_ids, HirTypeIds, HirTypeLocation};
```

Expected during the red step: this re-export remains unresolved until Step 3 adds `collect_hir_type_ids`.

Inside the existing `#[cfg(test)] mod tests` block in `lib/src/hir/type_ids.rs`, extend the imports and add this test:

```rust
    use std::collections::HashMap;

    use crate::hir::{
        HirAssociatedTypeDef, HirBlock, HirEnum, HirExtern, HirField, HirFunction, HirFunctionSig,
        HirImpl, HirImplOwner, HirParam, HirProgram, HirStruct, HirTrait, HirVariant,
        HirVariantFields,
    };
    use crate::ids::{AssocTypeId, FieldId, VariantId};
    use crate::type_context::TypeContext;
    use crate::types::{AssociatedTypeKey, GenericParamId, Type};

    fn generic(owner: DefId, index: u32) -> GenericParamId {
        GenericParamId { owner, index }
    }

    fn empty_body(ty: Type) -> HirBlock {
        HirBlock { stmts: Vec::new(), ty }
    }

    fn function(id: DefId, name: &str, param_ty: Type, ret_type: Type) -> HirFunction {
        HirFunction {
            id,
            name: name.to_string(),
            qualified_name: None,
            generic_params: Vec::new(),
            generic_param_ids: Vec::new(),
            generic_bounds: HashMap::new(),
            params: vec![HirParam {
                name: "value".to_string(),
                ty: param_ty,
                mutable: false,
                is_ref: false,
            }],
            ret_type: ret_type.clone(),
            body: empty_body(ret_type),
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        }
    }

    #[test]
    fn hir_type_ids_collect_top_level_type_locations() {
        let function_id = def_id(1);
        let extern_id = def_id(2);
        let struct_id = def_id(3);
        let other_struct_id = def_id(4);
        let enum_id = def_id(5);
        let trait_id = def_id(6);
        let signature_id = def_id(7);
        let impl_id = def_id(8);
        let assoc_id = AssocTypeId(0);
        let generic_param = generic(impl_id, 0);
        let projection = Type::Projection {
            ty: Box::new(Type::Generic(generic_param)),
            trait_id,
            assoc_type: AssociatedTypeKey {
                owner: trait_id,
                assoc_type_id: assoc_id,
            },
            trait_args: vec![Type::I64],
        };

        let program = HirProgram::from_parts(
            HashMap::from([(
                "id".to_string(),
                function(function_id, "id", Type::I64, Type::I64),
            )]),
            HashMap::from([
                (
                    "Box".to_string(),
                    HirStruct {
                        id: struct_id,
                        name: "Box".to_string(),
                        generic_params: Vec::new(),
                        fields: vec![HirField {
                            id: FieldId(0),
                            name: "value".to_string(),
                            ty: Type::Struct {
                                id: struct_id,
                                args: Vec::new(),
                            },
                            public: false,
                        }],
                    },
                ),
                (
                    "OtherBox".to_string(),
                    HirStruct {
                        id: other_struct_id,
                        name: "OtherBox".to_string(),
                        generic_params: Vec::new(),
                        fields: vec![HirField {
                            id: FieldId(0),
                            name: "value".to_string(),
                            ty: Type::Struct {
                                id: other_struct_id,
                                args: Vec::new(),
                            },
                            public: false,
                        }],
                    },
                ),
            ]),
            HashMap::from([(
                "Maybe".to_string(),
                HirEnum {
                    id: enum_id,
                    name: "Maybe".to_string(),
                    generic_params: Vec::new(),
                    variants: vec![
                        HirVariant {
                            id: VariantId(0),
                            name: "Named".to_string(),
                            fields: HirVariantFields::Named(vec![HirField {
                                id: FieldId(0),
                                name: "payload".to_string(),
                                ty: Type::Bool,
                                public: false,
                            }]),
                        },
                        HirVariant {
                            id: VariantId(1),
                            name: "Tuple".to_string(),
                            fields: HirVariantFields::Positional(vec![Type::I64]),
                        },
                    ],
                },
            )]),
            HashMap::from([(
                "Iterable".to_string(),
                HirTrait {
                    id: trait_id,
                    name: "Iterable".to_string(),
                    generic_params: Vec::new(),
                    associated_types: Vec::new(),
                    methods: HashMap::new(),
                    signatures: HashMap::from([(
                        "next".to_string(),
                        HirFunctionSig {
                            id: signature_id,
                            name: "next".to_string(),
                            generic_params: Vec::new(),
                            generic_param_ids: Vec::new(),
                            params: vec![Type::I64],
                            ret: Type::Bool,
                            generic_bounds: HashMap::new(),
                            self_receiver: None,
                        },
                    )]),
                },
            )]),
            vec![HirImpl {
                id: impl_id,
                owner: HirImplOwner::Named("Box".to_string()),
                type_name: "Box".to_string(),
                type_generics: vec!["T".to_string()],
                receiver_arg_types: vec![Type::Generic(generic_param)],
                trait_name: Some("Iterable".to_string()),
                trait_id: Some(trait_id),
                trait_generics: Vec::new(),
                trait_arg_types: vec![Type::I64],
                associated_types: vec![HirAssociatedTypeDef {
                    id: assoc_id,
                    name: "Item".to_string(),
                    ty: projection.clone(),
                }],
                bounds: Vec::new(),
                methods: HashMap::new(),
            }],
            vec![HirExtern {
                id: extern_id,
                name: "puts".to_string(),
                params: vec![Type::Pointer(Box::new(Type::U8))],
                ret: Type::I32,
                variadic: false,
            }],
        );

        let mut context = TypeContext::new();
        let ids = collect_hir_type_ids(&program, &mut context);

        let function_ret = ids
            .get(&HirTypeLocation::FunctionReturn {
                function: function_id,
            })
            .unwrap();
        let function_param = ids
            .get(&HirTypeLocation::FunctionParam {
                function: function_id,
                index: 0,
            })
            .unwrap();
        assert_eq!(function_ret, function_param);
        assert_eq!(context.type_for(function_ret), Type::I64);
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::ExternParam {
                    extern_id,
                    index: 0,
                })
                .unwrap()
            ),
            Type::Pointer(Box::new(Type::U8))
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::ExternReturn { extern_id })
                    .unwrap()
            ),
            Type::I32
        );
        let first_nominal = ids
            .get(&HirTypeLocation::StructField {
                owner: struct_id,
                field: FieldId(0),
            })
            .unwrap();
        let second_nominal = ids
            .get(&HirTypeLocation::StructField {
                owner: other_struct_id,
                field: FieldId(0),
            })
            .unwrap();
        assert_ne!(first_nominal, second_nominal);
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::EnumVariantNamedField {
                    owner: enum_id,
                    variant: VariantId(0),
                    field: FieldId(0),
                })
                .unwrap()
            ),
            Type::Bool
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::EnumVariantPositionalField {
                    owner: enum_id,
                    variant: VariantId(1),
                    index: 0,
                })
                .unwrap()
            ),
            Type::I64
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::TraitSignatureReturn {
                    trait_id,
                    signature: signature_id,
                })
                .unwrap()
            ),
            Type::Bool
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::TraitSignatureParam {
                    trait_id,
                    signature: signature_id,
                    index: 0,
                })
                .unwrap()
            ),
            Type::I64
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::ImplReceiverArg { impl_id, index: 0 })
                    .unwrap()
            ),
            Type::Generic(generic_param)
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::ImplTraitArg { impl_id, index: 0 })
                    .unwrap()
            ),
            Type::I64
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::AssociatedTypeDef {
                    impl_id,
                    assoc_type: assoc_id,
                })
                .unwrap()
            ),
            projection
        );
        assert!(!ids.is_empty());
    }
```

- [ ] **Step 2: Run the top-level collection test to verify it fails**

Run: `cargo test -p rock-lib hir_type_ids_collect_top_level_type_locations`

Expected: FAIL with unresolved `collect_hir_type_ids` in `lib/src/hir/mod.rs` and `lib/src/hir/type_ids.rs`.

- [ ] **Step 3: Implement top-level HIR type collection**

Add these imports near the top of `lib/src/hir/type_ids.rs`:

```rust
use crate::hir::{
    HirEnum, HirExtern, HirFunction, HirImpl, HirProgram, HirStruct, HirTrait, HirVariantFields,
};
use crate::type_context::TypeContext;
use crate::types::Type;
```

Insert this implementation below the `impl HirTypeIds` block:

```rust
pub fn collect_hir_type_ids(program: &HirProgram, context: &mut TypeContext) -> HirTypeIds {
    let mut ids = HirTypeIds::new();

    for function in program.functions.values() {
        collect_function_signature(function, context, &mut ids);
    }

    for ext in program.externs.values() {
        collect_extern_signature(ext, context, &mut ids);
    }

    for strukt in program.structs.values() {
        collect_struct_fields(strukt, context, &mut ids);
    }

    for enm in program.enums.values() {
        collect_enum_fields(enm, context, &mut ids);
    }

    for trait_def in program.traits.values() {
        collect_trait_signatures(trait_def, context, &mut ids);
        for method in trait_def.methods.values() {
            collect_function_signature(method, context, &mut ids);
        }
    }

    for imp in program.impls.values() {
        collect_impl_types(imp, context, &mut ids);
        for method in imp.methods.values() {
            collect_function_signature(method, context, &mut ids);
        }
    }

    ids
}

fn record_type(
    context: &mut TypeContext,
    ids: &mut HirTypeIds,
    location: HirTypeLocation,
    ty: &Type,
) {
    let id = context.intern_type(ty);
    ids.insert(location, id);
}

fn collect_function_signature(
    function: &HirFunction,
    context: &mut TypeContext,
    ids: &mut HirTypeIds,
) {
    record_type(
        context,
        ids,
        HirTypeLocation::FunctionReturn {
            function: function.id,
        },
        &function.ret_type,
    );
    for (index, param) in function.params.iter().enumerate() {
        record_type(
            context,
            ids,
            HirTypeLocation::FunctionParam {
                function: function.id,
                index,
            },
            &param.ty,
        );
    }
}

fn collect_extern_signature(ext: &HirExtern, context: &mut TypeContext, ids: &mut HirTypeIds) {
    record_type(
        context,
        ids,
        HirTypeLocation::ExternReturn { extern_id: ext.id },
        &ext.ret,
    );
    for (index, param) in ext.params.iter().enumerate() {
        record_type(
            context,
            ids,
            HirTypeLocation::ExternParam {
                extern_id: ext.id,
                index,
            },
            param,
        );
    }
}

fn collect_struct_fields(strukt: &HirStruct, context: &mut TypeContext, ids: &mut HirTypeIds) {
    for field in &strukt.fields {
        record_type(
            context,
            ids,
            HirTypeLocation::StructField {
                owner: strukt.id,
                field: field.id,
            },
            &field.ty,
        );
    }
}

fn collect_enum_fields(enm: &HirEnum, context: &mut TypeContext, ids: &mut HirTypeIds) {
    for variant in &enm.variants {
        match &variant.fields {
            HirVariantFields::Named(fields) => {
                for field in fields {
                    record_type(
                        context,
                        ids,
                        HirTypeLocation::EnumVariantNamedField {
                            owner: enm.id,
                            variant: variant.id,
                            field: field.id,
                        },
                        &field.ty,
                    );
                }
            }
            HirVariantFields::Positional(types) => {
                for (index, ty) in types.iter().enumerate() {
                    record_type(
                        context,
                        ids,
                        HirTypeLocation::EnumVariantPositionalField {
                            owner: enm.id,
                            variant: variant.id,
                            index,
                        },
                        ty,
                    );
                }
            }
            HirVariantFields::Unit => {}
        }
    }
}

fn collect_trait_signatures(trait_def: &HirTrait, context: &mut TypeContext, ids: &mut HirTypeIds) {
    for signature in trait_def.signatures.values() {
        record_type(
            context,
            ids,
            HirTypeLocation::TraitSignatureReturn {
                trait_id: trait_def.id,
                signature: signature.id,
            },
            &signature.ret,
        );
        for (index, param) in signature.params.iter().enumerate() {
            record_type(
                context,
                ids,
                HirTypeLocation::TraitSignatureParam {
                    trait_id: trait_def.id,
                    signature: signature.id,
                    index,
                },
                param,
            );
        }
    }
}

fn collect_impl_types(imp: &HirImpl, context: &mut TypeContext, ids: &mut HirTypeIds) {
    for (index, ty) in imp.receiver_arg_types.iter().enumerate() {
        record_type(
            context,
            ids,
            HirTypeLocation::ImplReceiverArg {
                impl_id: imp.id,
                index,
            },
            ty,
        );
    }

    for (index, ty) in imp.trait_arg_types.iter().enumerate() {
        record_type(
            context,
            ids,
            HirTypeLocation::ImplTraitArg {
                impl_id: imp.id,
                index,
            },
            ty,
        );
    }

    for assoc in &imp.associated_types {
        record_type(
            context,
            ids,
            HirTypeLocation::AssociatedTypeDef {
                impl_id: imp.id,
                assoc_type: assoc.id,
            },
            &assoc.ty,
        );
    }
}
```

- [ ] **Step 4: Run the top-level collection tests**

Run: `cargo test -p rock-lib hir_type_ids_collect_top_level_type_locations`

Expected: PASS.

- [ ] **Step 5: Commit top-level collection**

```bash
git add lib/src/hir/mod.rs lib/src/hir/type_ids.rs
git commit -m "collect top level hir type ids"
```

## Task 3: Collect Body Type Locations

**Files:**
- Modify: `lib/src/hir/type_ids.rs`

- [ ] **Step 1: Add failing body collection test**

Inside the existing tests module in `lib/src/hir/type_ids.rs`, add this test:

```rust
    #[test]
    fn hir_type_ids_collect_body_type_locations() {
        let function_id = def_id(20);
        let capture_ty = Type::Reference {
            mutable: false,
            inner: Box::new(Type::I64),
        };
        let lambda_ty = Type::Function(vec![Type::I64], Box::new(Type::I64));
        let function = HirFunction {
            id: function_id,
            name: "main".to_string(),
            qualified_name: None,
            generic_params: Vec::new(),
            generic_param_ids: Vec::new(),
            generic_bounds: HashMap::new(),
            params: Vec::new(),
            ret_type: Type::Unit,
            body: HirBlock {
                stmts: vec![
                    crate::hir::HirStmt::Let {
                        name: "x".to_string(),
                        ty: Type::I64,
                        value: crate::hir::HirExpr {
                            kind: crate::hir::HirExprKind::IntLiteral(1),
                            ty: Type::I64,
                            span: Default::default(),
                        },
                        mutable: false,
                    },
                    crate::hir::HirStmt::Expr(crate::hir::HirExpr {
                        kind: crate::hir::HirExprKind::Lambda {
                            params: vec![HirParam {
                                name: "value".to_string(),
                                ty: Type::I64,
                                mutable: false,
                                is_ref: false,
                            }],
                            body: HirBlock {
                                stmts: vec![crate::hir::HirStmt::Expr(crate::hir::HirExpr {
                                    kind: crate::hir::HirExprKind::Var("value".to_string()),
                                    ty: Type::I64,
                                    span: Default::default(),
                                })],
                                ty: Type::I64,
                            },
                            captures: vec![crate::hir::HirClosureCapture {
                                name: "x".to_string(),
                                kind: crate::hir::HirClosureCaptureKind::SharedBorrow,
                                ty: capture_ty.clone(),
                            }],
                        },
                        ty: lambda_ty.clone(),
                        span: Default::default(),
                    }),
                ],
                ty: Type::Unit,
            },
            is_curried: false,
            is_method: false,
            self_receiver: None,
            is_unsafe: false,
        };
        let program = HirProgram::from_parts(
            HashMap::from([("main".to_string(), function)]),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
        );

        let mut context = TypeContext::new();
        let ids = collect_hir_type_ids(&program, &mut context);

        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::Block {
                    owner: function_id,
                    path: vec![],
                })
                .unwrap()
            ),
            Type::Unit
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::LetStmt {
                    owner: function_id,
                    path: vec![0],
                    name: "x".to_string(),
                })
                .unwrap()
            ),
            Type::I64
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::Expr {
                    owner: function_id,
                    path: vec![0, 0],
                })
                .unwrap()
            ),
            Type::I64
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::Expr {
                    owner: function_id,
                    path: vec![1, 0],
                })
                .unwrap()
            ),
            lambda_ty
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::ClosureCapture {
                    owner: function_id,
                    path: vec![1, 0],
                    name: "x".to_string(),
                })
                .unwrap()
            ),
            capture_ty
        );
        assert_eq!(
            context.type_for(
                ids.get(&HirTypeLocation::Block {
                    owner: function_id,
                    path: vec![1, 0, 1],
                })
                .unwrap()
            ),
            Type::I64
        );
    }
```

- [ ] **Step 2: Run the body collection test to verify it fails**

Run: `cargo test -p rock-lib hir_type_ids_collect_body_type_locations`

Expected: FAIL because `collect_hir_type_ids` does not record `Block`, `LetStmt`, `Expr`, or `ClosureCapture` locations yet.

- [ ] **Step 3: Implement body traversal**

Extend the `use crate::hir::{ ... }` import in `lib/src/hir/type_ids.rs` to include these types:

```rust
    HirBlock, HirExpr, HirExprKind, HirStmt,
```

Update `collect_hir_type_ids` so function, trait default, and impl method bodies are collected after their signatures:

```rust
    for function in program.functions.values() {
        collect_function_signature(function, context, &mut ids);
        collect_block(function.id, &function.body, Vec::new(), context, &mut ids);
    }
```

Inside the trait-method and impl-method loops, add the same `collect_block(method.id, &method.body, Vec::new(), context, &mut ids);` call immediately after `collect_function_signature(method, context, &mut ids);`.

Insert these traversal helpers below `collect_impl_types`:

```rust
fn child_path(path: &[usize], segment: usize) -> Vec<usize> {
    let mut child = path.to_vec();
    child.push(segment);
    child
}

fn collect_block(
    owner: DefId,
    block: &HirBlock,
    path: Vec<usize>,
    context: &mut TypeContext,
    ids: &mut HirTypeIds,
) {
    record_type(
        context,
        ids,
        HirTypeLocation::Block {
            owner,
            path: path.clone(),
        },
        &block.ty,
    );

    for (index, stmt) in block.stmts.iter().enumerate() {
        let stmt_path = child_path(&path, index);
        collect_stmt(owner, stmt, stmt_path, context, ids);
    }
}

fn collect_stmt(
    owner: DefId,
    stmt: &HirStmt,
    path: Vec<usize>,
    context: &mut TypeContext,
    ids: &mut HirTypeIds,
) {
    match stmt {
        HirStmt::Let { name, ty, value, .. } => {
            record_type(
                context,
                ids,
                HirTypeLocation::LetStmt {
                    owner,
                    path: path.clone(),
                    name: name.clone(),
                },
                ty,
            );
            collect_expr(owner, value, child_path(&path, 0), context, ids);
        }
        HirStmt::Expr(expr) => collect_expr(owner, expr, child_path(&path, 0), context, ids),
        HirStmt::Return(Some(expr)) => {
            collect_expr(owner, expr, child_path(&path, 0), context, ids)
        }
        HirStmt::Break(Some(expr)) => collect_expr(owner, expr, child_path(&path, 0), context, ids),
        HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => {}
    }
}

fn collect_expr(
    owner: DefId,
    expr: &HirExpr,
    path: Vec<usize>,
    context: &mut TypeContext,
    ids: &mut HirTypeIds,
) {
    record_type(
        context,
        ids,
        HirTypeLocation::Expr {
            owner,
            path: path.clone(),
        },
        &expr.ty,
    );

    match &expr.kind {
        HirExprKind::ArrayLiteral(elems) | HirExprKind::TupleLiteral(elems) => {
            for (index, elem) in elems.iter().enumerate() {
                collect_expr(owner, elem, child_path(&path, index), context, ids);
            }
        }
        HirExprKind::FieldAccess(inner, _, _)
        | HirExprKind::TupleIndex(inner, _)
        | HirExprKind::UnaryOp(_, inner)
        | HirExprKind::Ref(_, inner)
        | HirExprKind::Deref(inner)
        | HirExprKind::Cast(inner, _) => {
            collect_expr(owner, inner, child_path(&path, 0), context, ids);
        }
        HirExprKind::Index(receiver, index) | HirExprKind::BinOp(_, receiver, index) => {
            collect_expr(owner, receiver, child_path(&path, 0), context, ids);
            collect_expr(owner, index, child_path(&path, 1), context, ids);
        }
        HirExprKind::Call(function, args) => {
            collect_expr(owner, function, child_path(&path, 0), context, ids);
            for (index, arg) in args.iter().enumerate() {
                collect_expr(owner, arg, child_path(&path, index + 1), context, ids);
            }
        }
        HirExprKind::MethodCall(receiver, _, args, _, _) => {
            collect_expr(owner, receiver, child_path(&path, 0), context, ids);
            for (index, arg) in args.iter().enumerate() {
                collect_expr(owner, arg, child_path(&path, index + 1), context, ids);
            }
        }
        HirExprKind::StructLiteral(_, _, fields) => {
            for (index, field) in fields.iter().enumerate() {
                collect_expr(owner, &field.value, child_path(&path, index), context, ids);
            }
        }
        HirExprKind::EnumVariant(_, _, args, _) | HirExprKind::Intrinsic { args, .. } => {
            for (index, arg) in args.iter().enumerate() {
                collect_expr(owner, arg, child_path(&path, index), context, ids);
            }
        }
        HirExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr(owner, condition, child_path(&path, 0), context, ids);
            collect_block(owner, then_branch, child_path(&path, 1), context, ids);
            if let Some(else_branch) = else_branch {
                collect_block(owner, else_branch, child_path(&path, 2), context, ids);
            }
        }
        HirExprKind::Match { scrutinee, arms } => {
            collect_expr(owner, scrutinee, child_path(&path, 0), context, ids);
            for (index, arm) in arms.iter().enumerate() {
                if let Some(guard) = &arm.guard {
                    collect_expr(owner, guard, child_path(&path, 1 + index * 2), context, ids);
                }
                collect_block(owner, &arm.body, child_path(&path, 2 + index * 2), context, ids);
            }
        }
        HirExprKind::While { condition, body } => {
            collect_expr(owner, condition, child_path(&path, 0), context, ids);
            collect_block(owner, body, child_path(&path, 1), context, ids);
        }
        HirExprKind::For { iter, body, .. } => {
            collect_expr(owner, iter, child_path(&path, 0), context, ids);
            collect_block(owner, body, child_path(&path, 1), context, ids);
        }
        HirExprKind::Loop(body) | HirExprKind::Block(body) => {
            collect_block(owner, body, child_path(&path, 0), context, ids);
        }
        HirExprKind::Lambda { body, captures, .. } => {
            for capture in captures {
                record_type(
                    context,
                    ids,
                    HirTypeLocation::ClosureCapture {
                        owner,
                        path: path.clone(),
                        name: capture.name.clone(),
                    },
                    &capture.ty,
                );
            }
            collect_block(owner, body, child_path(&path, 1), context, ids);
        }
        HirExprKind::Assign(lhs, rhs) | HirExprKind::Range(lhs, rhs) => {
            collect_expr(owner, lhs, child_path(&path, 0), context, ids);
            collect_expr(owner, rhs, child_path(&path, 1), context, ids);
        }
        HirExprKind::IntLiteral(_)
        | HirExprKind::FloatLiteral(_)
        | HirExprKind::BoolLiteral(_)
        | HirExprKind::StringLiteral(_)
        | HirExprKind::CharLiteral(_)
        | HirExprKind::Unit
        | HirExprKind::Var(_)
        | HirExprKind::ResolvedVar(_) => {}
    }
}
```

- [ ] **Step 4: Run body and top-level collection tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib hir_type_ids_collect_body_type_locations
cargo test -p rock-lib hir_type_ids_collect_top_level_type_locations
```

Expected: PASS for each command.

- [ ] **Step 5: Commit body collection**

```bash
git add lib/src/hir/type_ids.rs
git commit -m "collect hir body type ids"
```

## Task 4: Attach TypeContext And TypeIds To Resolved HIR

**Files:**
- Modify: `lib/src/infer/mod.rs`
- Modify: `lib/src/products.rs`

- [ ] **Step 1: Add failing resolved-HIR sidecar test**

In `lib/src/infer/mod.rs`, extend the test imports to include `HirTypeLocation`:

```rust
    use crate::hir::{
        HirBlock, HirExpr, HirExprKind, HirImpl, HirImplOwner, HirMethodCallTarget, HirParam,
        HirStmt, HirTypeLocation,
    };
```

Add this test to the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn finalize_builds_resolved_hir_type_context_and_sidecar() {
        let function_id = DefId::new(CrateId(0), LocalDefId(1));
        let mut partial = PartialHir {
            functions: HashMap::from([(
                "identity".to_string(),
                HirFunction {
                    id: function_id,
                    name: "identity".to_string(),
                    qualified_name: None,
                    generic_params: Vec::new(),
                    generic_param_ids: Vec::new(),
                    generic_bounds: HashMap::new(),
                    params: vec![HirParam {
                        name: "value".to_string(),
                        ty: Type::I64,
                        mutable: false,
                        is_ref: false,
                    }],
                    ret_type: Type::I64,
                    body: HirBlock {
                        stmts: vec![HirStmt::Expr(HirExpr {
                            kind: HirExprKind::Var("value".to_string()),
                            ty: Type::I64,
                            span: Default::default(),
                        })],
                        ty: Type::I64,
                    },
                    is_curried: false,
                    is_method: false,
                    self_receiver: None,
                    is_unsafe: false,
                },
            )]),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            impls: vec![],
            externs: vec![],
            engine: InferenceEngine::new(),
            function_type_vars: HashMap::new(),
            import_aliases: HashMap::new(),
            loaded_module_paths: vec![],
            constraint_store: ConstraintStore::default(),
            resolver: ResolverTables::default(),
            current_def_ids: BTreeSet::from([function_id]),
            root_crate_id: crate::ids::CrateId(0),
            local_def_ids: crate::ids::IdGen::<crate::ids::LocalDefId>::new(),
        };
        partial.local_def_ids.fresh();
        partial.local_def_ids.fresh();

        let resolved = finalize(partial).unwrap();
        let ret_id = resolved
            .type_id_at(&HirTypeLocation::FunctionReturn {
                function: function_id,
            })
            .unwrap();
        let param_id = resolved
            .type_id_at(&HirTypeLocation::FunctionParam {
                function: function_id,
                index: 0,
            })
            .unwrap();
        let expr_id = resolved
            .type_id_at(&HirTypeLocation::Expr {
                owner: function_id,
                path: vec![0, 0],
            })
            .unwrap();

        assert_eq!(ret_id, param_id);
        assert_eq!(param_id, expr_id);
        assert_eq!(resolved.type_at(ret_id), Type::I64);
    }
```

- [ ] **Step 2: Run the resolved-HIR test to verify it fails**

Run: `cargo test -p rock-lib finalize_builds_resolved_hir_type_context_and_sidecar`

Expected: FAIL with unresolved `ResolvedHirProgram::type_id_at` and `ResolvedHirProgram::type_at`.

- [ ] **Step 3: Add sidecar fields and accessors to `ResolvedHirProgram`**

In `lib/src/infer/mod.rs`, extend imports:

```rust
use crate::hir::{
    collect_hir_type_ids, HirBlock, HirEnum, HirExpr, HirExprKind, HirExtern, HirFunction,
    HirImpl, HirPattern, HirProgram, HirStmt, HirStruct, HirTrait, HirTypeIds, HirTypeLocation,
};
use crate::ids::{CrateId, DefId, IdGen, LocalDefId, TypeId, TypeVarId};
use crate::type_context::TypeContext;
```

Replace the `ResolvedHirProgram` definition with:

```rust
#[derive(Debug)]
pub struct ResolvedHirProgram {
    pub program: HirProgram,
    pub resolver: ResolverTables,
    pub current_def_ids: BTreeSet<DefId>,
    pub root_crate_id: crate::ids::CrateId,
    pub local_def_ids: IdGen<LocalDefId>,
    pub type_context: TypeContext,
    pub type_ids: HirTypeIds,
}

impl ResolvedHirProgram {
    pub fn new(
        program: HirProgram,
        resolver: ResolverTables,
        current_def_ids: BTreeSet<DefId>,
        root_crate_id: crate::ids::CrateId,
        local_def_ids: IdGen<LocalDefId>,
    ) -> Self {
        let mut type_context = TypeContext::new();
        let type_ids = collect_hir_type_ids(&program, &mut type_context);
        Self {
            program,
            resolver,
            current_def_ids,
            root_crate_id,
            local_def_ids,
            type_context,
            type_ids,
        }
    }

    pub fn type_id_at(&self, location: &HirTypeLocation) -> Option<TypeId> {
        self.type_ids.get(location)
    }

    pub fn type_at(&self, id: TypeId) -> Type {
        self.type_context.type_for(id)
    }
}
```

- [ ] **Step 4: Construct resolved HIR through the new constructor**

In both `finalize_lenient` and `finalize`, replace the direct `Ok(ResolvedHirProgram { ... })` construction with this shape:

```rust
    let program = HirProgram::from_parts_with_canonical_names(
        hir.functions,
        hir.structs,
        hir.enums,
        hir.traits,
        hir.impls,
        hir.externs,
        &canonical_names_by_id,
    );
    Ok(ResolvedHirProgram::new(
        program,
        hir.resolver,
        hir.current_def_ids,
        hir.root_crate_id,
        hir.local_def_ids,
    ))
```

- [ ] **Step 5: Update test constructors in `lib/src/products.rs`**

In `lib/src/products.rs`, update each test-only `ResolvedHirProgram { ... }` literal inside `#[cfg(test)] mod tests` to use `ResolvedHirProgram::new(program, resolver, current_def_ids, root_crate_id, local_def_ids)`.

For example, replace the literal in `resolved_hir_for_products` with:

```rust
        ResolvedHirProgram::new(
            HirProgram::from_parts(functions, structs, enums, traits, impls, externs),
            ResolverTables::default(),
            (0..8)
                .map(|local| DefId::new(CrateId(0), LocalDefId(local)))
                .collect(),
            CrateId(0),
            local_def_ids_after(8),
        )
```

Apply the same constructor pattern to the remaining `ResolvedHirProgram` literals in `lib/src/products.rs` tests. Preserve the existing `program`, `resolver`, `current_def_ids`, `root_crate_id`, and `local_def_ids` values for each test.

- [ ] **Step 6: Run focused resolved-HIR and product smoke tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib finalize_builds_resolved_hir_type_context_and_sidecar
cargo test -p rock-lib compiler_products_key_metadata_by_product_def_id
```

Expected: PASS for each command.

- [ ] **Step 7: Commit resolved-HIR attachment**

```bash
git add lib/src/infer/mod.rs lib/src/products.rs
git commit -m "attach type ids to resolved hir"
```

## Task 5: Add Product Artifact Compatibility Coverage

**Files:**
- Modify: `lib/src/products.rs`

- [ ] **Step 1: Add failing product compatibility test**

In `lib/src/products.rs`, add this test inside `#[cfg(test)] mod tests` near the other artifact roundtrip tests:

```rust
    #[test]
    fn compiler_products_do_not_serialize_resolved_hir_type_id_sidecar() {
        let hir = resolved_hir_for_products();
        let function_id = DefId::new(CrateId(0), LocalDefId(0));
        let ret_id = hir
            .type_id_at(&crate::hir::HirTypeLocation::FunctionReturn {
                function: function_id,
            })
            .unwrap();

        assert_eq!(hir.type_at(ret_id), Type::I64);

        let products = CompilerProducts::from_resolved_hir(
            ProductCrateIdentity::local("demo".to_string()),
            &hir,
            Vec::new(),
            BTreeMap::new(),
            ProductSourceFingerprint::default(),
            ProductLinkData::default(),
        );
        let bytes = products.to_artifact_bytes().unwrap();
        let artifact: ProductArtifact = bincode::deserialize(&bytes).unwrap();
        let roundtrip = CompilerProducts::from_artifact_bytes(&bytes).unwrap();
        let product_id = ProductDefId::from(function_id);

        assert_eq!(artifact.format_version, PRODUCT_ARTIFACT_FORMAT_VERSION);
        assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 18);
        assert_eq!(roundtrip.metadata.functions[&product_id].ret_type, Type::I64);
        assert_eq!(roundtrip.metadata.functions[&product_id].params[0].ty, Type::I64);
    }
```

- [ ] **Step 2: Run the product compatibility test to verify it passes**

Run: `cargo test -p rock-lib compiler_products_do_not_serialize_resolved_hir_type_id_sidecar`

Expected: PASS. This test should pass because Task 4 already attached the sidecar to `ResolvedHirProgram` without copying it into product structs.

- [ ] **Step 3: Run artifact regression tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib compiler_products_write_and_read_product_artifact_bytes
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract
cargo test -p rock-lib compiler_products_roundtrip_preserves_product_def_ids
```

Expected: PASS for each command.

- [ ] **Step 4: Commit product compatibility coverage**

```bash
git add lib/src/products.rs
git commit -m "cover type id sidecar product compatibility"
```

## Task 6: Update Roadmap And Audit Trackers

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Run proof tests before tracker edits**

Run these commands one at a time:

```bash
cargo test -p rock-lib hir_type_ids_
cargo test -p rock-lib finalize_builds_resolved_hir_type_context_and_sidecar
cargo test -p rock-lib compiler_products_do_not_serialize_resolved_hir_type_id_sidecar
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract
```

Expected: PASS for each command.

- [ ] **Step 2: Update Roadmap Task 11 status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, insert this baseline bullet after the existing Task 10 bullet:

```markdown
- Roadmap Task 11 has landed for the first `TypeId` phase-boundary slice: resolved HIR now owns a `TypeContext` plus explicit HIR type-ID sidecar while structural `Type` remains the compatibility and product-artifact representation for downstream phases.
```

In the Task 11 section, add this status line immediately after the heading:

```markdown
**Status:** Complete for the resolved-HIR `TypeId` sidecar slice in `docs/superpowers/plans/2026-05-19-type-id-hir-boundary.md`; downstream mono, MIR, codegen, and artifact schema migration remain future work.
```

In the focused-plan list near the bottom, add this bullet after `Type Services`:

```markdown
7. `TypeId HIR Boundary`: complete in `docs/superpowers/plans/2026-05-19-type-id-hir-boundary.md`; covers Task 11's first narrow boundary by attaching `TypeContext` and HIR type-ID sidecar data to `ResolvedHirProgram` while preserving structural `Type` compatibility for products and downstream phases.
```

Replace:

```markdown
The first six focused implementation plans are complete. Later work can continue with `TypeId` phase-boundary migration, selection-service, instance, MIR, or codegen boundaries.
```

with:

```markdown
The first seven focused implementation plans are complete. Later work can continue with downstream `TypeId` consumer migration, selection-service, instance, MIR, or codegen boundaries.
```

- [ ] **Step 3: Update audit tracker evidence and remaining work**

Before editing `docs/superpowers/plans/master-audit-checklist.md`, run `git rev-parse --short HEAD` and use that command output as the checked implementation commit marker. For example, if the command prints `abc1234`, replace the existing marker line with:

```markdown
Checked against implementation commit: `abc1234`
```

Replace the `Type Context And Semantic Types` summary row with:

```markdown
| Type Context And Semantic Types | In progress | `lib/src/type_context/mod.rs`, `lib/src/hir/type_ids.rs`, `lib/src/infer/mod.rs`, `lib/src/type_services/*`, `lib/src/types/mod.rs`, `lib/src/type_lowering.rs`, `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md` | Interned `Ty` / `TypeId` scaffolding, explicit type services, and a resolved-HIR type-ID sidecar are in place, but downstream consumers and product artifacts still use structural `Type` |
```

In the `## 3. Type Context And Semantic Types` evidence list, add this bullet after the `type_services` evidence bullet:

```markdown
- `lib/src/hir/type_ids.rs` and `lib/src/infer/mod.rs` attach a `TypeContext` and HIR type-ID sidecar to finalized `ResolvedHirProgram` values, giving selected HIR type-carrying locations explicit `TypeId` identity without changing structural product serialization.
```

In the `Done:` list, add:

```markdown
- [x] Attached a resolved-HIR `TypeId` sidecar backed by `TypeContext` for selected function, extern, struct, enum, trait, impl, block, statement, expression, and closure-capture type locations.
```

In the `Still to do:` list, replace:

```markdown
- [ ] Migrate selected HIR, inference, and artifact-facing phase boundaries from structural `Type` to context-owned `TypeId` after type fact services are stable.
```

with:

```markdown
- [ ] Migrate downstream mono, MIR, codegen, and future artifact-schema consumers from structural `Type` to context-owned `TypeId` where the resolved-HIR sidecar now provides stable identity.
```

- [ ] **Step 4: Run documentation diff checks**

Run: `git diff --check`

Expected: PASS with no whitespace errors.

- [ ] **Step 5: Commit tracker updates**

```bash
git add docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/master-audit-checklist.md
git commit -m "update audit trackers for type id hir boundary"
```

## Task 7: Final Verification

**Files:**
- Verify: formatting, focused tests, product regressions, full `rock-lib` suite, and clean worktree.

- [ ] **Step 1: Format Rust changes**

Run: `cargo fmt --all`

Expected: command exits successfully.

- [ ] **Step 2: Verify formatting is stable**

Run: `cargo fmt --all --check`

Expected: PASS.

- [ ] **Step 3: Run focused HIR type-ID tests**

Run these commands one at a time:

```bash
cargo test -p rock-lib hir_type_ids_record_and_read_locations
cargo test -p rock-lib hir_type_ids_collect_top_level_type_locations
cargo test -p rock-lib hir_type_ids_collect_body_type_locations
cargo test -p rock-lib finalize_builds_resolved_hir_type_context_and_sidecar
```

Expected: PASS for each command.

- [ ] **Step 4: Run identity, inference, and artifact regressions**

Run these commands one at a time:

```bash
cargo test -p rock-lib type_context_
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids
cargo test -p rock-lib substitute_uses_typed_type_var_id_keys
cargo test -p rock-lib compiler_products_do_not_serialize_resolved_hir_type_id_sidecar
cargo test -p rock-lib compiler_products_write_and_read_product_artifact_bytes
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
```

Expected: PASS for each command.

- [ ] **Step 5: Run the full documented library suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 6: Verify the final diff has no whitespace errors**

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 7: Verify worktree state**

Run: `git status --short`

Expected: clean.

## Self-Review Notes

- Spec coverage: Tasks 1-3 implement the HIR sidecar and selected HIR type collection; Task 4 attaches `TypeContext` / `HirTypeIds` to finalized `ResolvedHirProgram`; Task 5 proves products remain structural; Task 6 updates roadmap/audit docs; Task 7 verifies focused and full behavior.
- Placeholder scan: this plan uses concrete file paths, APIs, code snippets, test commands, and commit messages.
- Type consistency: `HirTypeLocation`, `HirTypeIds`, `collect_hir_type_ids`, `ResolvedHirProgram::new`, `type_id_at`, and `type_at` names are consistent across all tasks.
