# Portable Product Artifact Type Table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the Type Context And Semantic Types audit track by changing product artifacts from inline structural `Type` serialization to a portable artifact-local type table.

**Architecture:** Keep in-memory `CompilerProducts` and HIR product payloads structural at the compatibility edge, but serialize artifacts through a v23 DTO with `ProductTypeId` references and a `ProductTypeTable`. Artifact loading decodes the DTO back into structural compatibility payloads, then continues through the existing product identity remap and consumer `TypeContext` interning path.

**Tech Stack:** Rust 2021, serde/bincode, existing `rock-lib` product artifacts, `rock-shared` sysroot format contract, Cargo tests.

---

## Design Spec

- Approved spec: `docs/superpowers/specs/2026-05-30-portable-product-artifact-type-table-design.md`
- Design commit: `72d24a7 design portable product artifact type table`

## File Structure

- Modify: `lib/src/products.rs`
  - Keep public `CompilerProducts` in-memory API.
  - Bump `PRODUCT_ARTIFACT_FORMAT_VERSION` to `23`.
  - Declare the new private module with `mod type_table;`.
  - Route `to_artifact_bytes` and `from_artifact_bytes` through the v23 DTO codec.
  - Update product artifact tests and add artifact-format tests.
- Create: `lib/src/products/type_table.rs`
  - Own `ProductTypeId`, `ProductGenericParamId`, `ProductAssociatedTypeKey`, `ProductTypeRow`, `ProductTypeTable`, DTO structs, encoder, decoder, and all product/HIR conversion walkers.
  - Keep this module private to `products.rs` except for `pub(super)` functions used by `CompilerProducts`.
- Modify: `rock-shared/src/sysroot.rs`
  - Bump shared `PRODUCT_ARTIFACT_FORMAT_VERSION` to `23`.
- Modify: `lib/src/crate_artifact/load.rs`
  - Add/adjust tests proving v23 decoded artifact types are re-interned into the supplied consumer `TypeContext`.
  - Avoid production changes unless v23 decoding reveals a direct integration gap.
- Modify after implementation review: `docs/superpowers/plans/master-audit-checklist.md`
  - Mark `Type Context And Semantic Types` complete.
- Modify after implementation review: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Remove the future artifact-schema caveat from Task 11 remaining work.
- Modify: this plan file
  - Append final verification notes during the final task.

---

## Task 0: Baseline And Serialization Surface Audit

**Files:**
- Read: `lib/src/products.rs`
- Read: `lib/src/hir/mod.rs`
- Read: `lib/src/types/mod.rs`
- Read: `lib/src/crate_artifact/load.rs`
- Read: `rock-shared/src/sysroot.rs`
- No code changes.

- [ ] **Step 1: Run current product/artifact baseline tests**

Run:

```bash
cargo test -p rock-lib product_artifact
cargo test -p rock-lib crate_artifact::load
cargo test -p rock-lib type_context
```

Expected: all three commands pass before this task starts changing artifact schema.

- [ ] **Step 2: Audit product-serialized type fields**

Run:

```bash
rg ": Type|Vec<Type>|Box<Type>" lib/src/hir lib/src/types.rs lib/src/types/mod.rs --glob '*.rs'
```

Expected: every `Type` field that can appear under `CompilerProducts.metadata` or `CompilerProducts.bodies` is covered by the conversion list in the approved spec:

```text
HirFunction.ret_type
HirParam.ty
HirClosureCapture.ty
HirMethodCallTarget.trait_args
HirField.ty
HirVariantFields::Positional
HirFunctionSig.params
HirFunctionSig.ret
HirImpl.receiver_arg_types
HirImpl.trait_arg_types
HirImpl.bounds.type_args
HirAssociatedTypeDef.ty
HirExtern.params
HirExtern.ret
HirBlock.ty
HirStmt::Let.ty
HirExpr.ty
HirExprKind::Cast target type
HirPattern::Struct type args
TraitBound.type_args
```

- [ ] **Step 3: Commit only if audit notes are added**

Do not create a commit for a no-change audit. If the implementer adds local audit notes to this plan, commit only this plan file:

```bash
git add docs/superpowers/plans/2026-05-30-portable-product-artifact-type-table.md
git commit -m "record artifact type table audit"
```

Expected: either no commit, or a docs-only commit.

---

## Task 1: Artifact Format Version 23 Shell

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `rock-shared/src/sysroot.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Write failing format version tests**

In `lib/src/products.rs`, update/add tests in the existing `#[cfg(test)] mod tests` section:

```rust
#[test]
fn product_artifact_format_version_matches_shared_contract() {
    assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 23);
    assert_eq!(
        PRODUCT_ARTIFACT_FORMAT_VERSION,
        rock_shared::sysroot::PRODUCT_ARTIFACT_FORMAT_VERSION
    );
}

#[test]
fn compiler_products_reject_format_22_product_artifacts() {
    let bytes = bincode::serialize(&22u32).unwrap();

    let err = CompilerProducts::from_artifact_bytes(&bytes).unwrap_err();

    assert!(
        err.contains("Unsupported product artifact format 22") && err.contains("expected 23"),
        "expected format-22 rejection, got {err}"
    );
}
```

Keep the existing unsupported-format-before-full-deserialize test and update its expected version to `23` after the implementation step.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract compiler_products_reject_format_22_product_artifacts
```

Expected: the format contract test fails because both constants are still `22`, or the command reports the old test body expecting `22`.

- [ ] **Step 3: Bump constants**

In `lib/src/products.rs`, change:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 22;
```

to:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 23;
```

In `rock-shared/src/sysroot.rs`, change:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 22;
```

to:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 23;
```

- [ ] **Step 4: Update old format test names and expected text**

Rename `compiler_products_reject_format_21_product_artifacts` to keep it as an old-version test or replace it with `compiler_products_reject_format_22_product_artifacts`. The expected error must include `expected 23`.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract
cargo test -p rock-lib compiler_products_reject_format_22_product_artifacts
cargo test -p rock-lib compiler_products_rejects_unsupported_format_before_full_deserialize
```

Expected: all three commands pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add lib/src/products.rs rock-shared/src/sysroot.rs
git commit -m "bump product artifact format to 23"
```

Expected: commit succeeds.

---

## Task 2: Product Type Table Codec

**Files:**
- Modify: `lib/src/products.rs`
- Create: `lib/src/products/type_table.rs`
- Test: `lib/src/products/type_table.rs`

- [ ] **Step 1: Add module declaration**

In `lib/src/products.rs`, add near the imports:

```rust
mod type_table;
```

- [ ] **Step 2: Write failing codec tests**

Create `lib/src/products/type_table.rs` with the tests first:

```rust
#[cfg(test)]
mod tests {
    use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId, TypeVarId};
    use crate::products::{ProductCrateId, ProductDefId, ProductLocalDefId};
    use crate::types::{AssociatedTypeKey, GenericParamId, Type};

    use super::{decode_type_for_test, encode_types_for_test, ProductTypeId};

    fn def(local: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(local))
    }

    #[test]
    fn product_type_table_deduplicates_equal_types() {
        let ty = Type::Struct {
            id: def(2),
            args: vec![Type::I64],
        };
        let (table, ids) = encode_types_for_test(&[ty.clone(), ty]).unwrap();

        assert_eq!(ids, vec![ProductTypeId(1), ProductTypeId(1)]);
        assert_eq!(table.rows.len(), 2, "struct row plus i64 row should be stored once each");
    }

    #[test]
    fn product_type_table_roundtrips_nested_projection_type() {
        let ty = Type::Projection {
            ty: Box::new(Type::Struct {
                id: def(2),
                args: vec![Type::Generic(GenericParamId {
                    owner: def(1),
                    index: 0,
                })],
            }),
            trait_id: def(3),
            assoc_type: AssociatedTypeKey {
                owner: def(3),
                assoc_type_id: AssocTypeId(0),
            },
            trait_args: vec![Type::Reference {
                mutable: false,
                inner: Box::new(Type::I64),
            }],
        };
        let (table, ids) = encode_types_for_test(&[ty.clone()]).unwrap();
        let decoded = decode_type_for_test(&table, ids[0]).unwrap();

        assert_eq!(decoded, ty);
    }

    #[test]
    fn product_type_table_rejects_type_vars() {
        let err = encode_types_for_test(&[Type::TypeVar(TypeVarId(0))]).unwrap_err();

        assert!(err.contains("TypeVar"), "unexpected error: {err}");
    }

    #[test]
    fn product_type_table_rejects_invalid_type_id() {
        let (table, _) = encode_types_for_test(&[Type::I64]).unwrap();
        let err = decode_type_for_test(&table, ProductTypeId(99)).unwrap_err();

        assert!(err.contains("unknown product type id 99"), "unexpected error: {err}");
    }

    #[test]
    fn product_type_table_decodes_product_def_ids_as_def_ids() {
        let ty = Type::Enum {
            id: DefId::new(CrateId(7), LocalDefId(9)),
            args: Vec::new(),
        };
        let (table, ids) = encode_types_for_test(&[ty.clone()]).unwrap();
        let row = &table.rows[ids[0].0 as usize];

        assert!(format!("{row:?}").contains("ProductCrateId(7)"));
        assert_eq!(decode_type_for_test(&table, ids[0]).unwrap(), ty);
    }
}
```

These tests intentionally reference helper functions and types that do not exist yet.

- [ ] **Step 3: Run codec tests to verify they fail**

Run:

```bash
cargo test -p rock-lib product_type_table_
```

Expected: compile failure for missing `ProductTypeId`, `encode_types_for_test`, and `decode_type_for_test`.

- [ ] **Step 4: Implement codec types and helpers**

In `lib/src/products/type_table.rs`, add the production code above the tests:

```rust
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId};
use crate::products::{ProductCrateId, ProductDefId, ProductLocalDefId};
use crate::types::{AssociatedTypeKey, GenericParamId, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct ProductTypeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct ProductGenericParamId {
    pub owner: ProductDefId,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct ProductAssociatedTypeKey {
    pub owner: ProductDefId,
    pub assoc_type_id: AssocTypeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) enum ProductTypeRow {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Str,
    Char,
    Unit,
    Never,
    Slice(ProductTypeId),
    Array(ProductTypeId, usize),
    Tuple(Vec<ProductTypeId>),
    Function(Vec<ProductTypeId>, ProductTypeId),
    Struct { id: ProductDefId, args: Vec<ProductTypeId> },
    Enum { id: ProductDefId, args: Vec<ProductTypeId> },
    Reference { mutable: bool, inner: ProductTypeId },
    Pointer(ProductTypeId),
    Generic(ProductGenericParamId),
    Projection {
        ty: ProductTypeId,
        trait_id: ProductDefId,
        assoc_type: ProductAssociatedTypeKey,
        trait_args: Vec<ProductTypeId>,
    },
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct ProductTypeTable {
    pub rows: Vec<ProductTypeRow>,
}

#[derive(Debug, Default)]
pub(super) struct ProductTypeEncoder {
    table: ProductTypeTable,
    ids: HashMap<ProductTypeRow, ProductTypeId>,
}

impl ProductTypeEncoder {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn finish(self) -> ProductTypeTable {
        self.table
    }

    pub(super) fn encode_type(&mut self, ty: &Type) -> Result<ProductTypeId, String> {
        let row = self.encode_row(ty)?;
        if let Some(id) = self.ids.get(&row).copied() {
            return Ok(id);
        }
        let id = ProductTypeId(self.table.rows.len() as u32);
        self.table.rows.push(row.clone());
        self.ids.insert(row, id);
        Ok(id)
    }

    fn encode_row(&mut self, ty: &Type) -> Result<ProductTypeRow, String> {
        Ok(match ty {
            Type::I8 => ProductTypeRow::I8,
            Type::I16 => ProductTypeRow::I16,
            Type::I32 => ProductTypeRow::I32,
            Type::I64 => ProductTypeRow::I64,
            Type::U8 => ProductTypeRow::U8,
            Type::U16 => ProductTypeRow::U16,
            Type::U32 => ProductTypeRow::U32,
            Type::U64 => ProductTypeRow::U64,
            Type::F32 => ProductTypeRow::F32,
            Type::F64 => ProductTypeRow::F64,
            Type::Bool => ProductTypeRow::Bool,
            Type::Str => ProductTypeRow::Str,
            Type::Char => ProductTypeRow::Char,
            Type::Unit => ProductTypeRow::Unit,
            Type::Never => ProductTypeRow::Never,
            Type::Slice(inner) => ProductTypeRow::Slice(self.encode_type(inner)?),
            Type::Array(inner, len) => ProductTypeRow::Array(self.encode_type(inner)?, *len),
            Type::Tuple(elems) => ProductTypeRow::Tuple(self.encode_types(elems)?),
            Type::Function(args, ret) => {
                ProductTypeRow::Function(self.encode_types(args)?, self.encode_type(ret)?)
            }
            Type::Struct { id, args } => ProductTypeRow::Struct {
                id: product_def_id(*id),
                args: self.encode_types(args)?,
            },
            Type::Enum { id, args } => ProductTypeRow::Enum {
                id: product_def_id(*id),
                args: self.encode_types(args)?,
            },
            Type::Reference { mutable, inner } => ProductTypeRow::Reference {
                mutable: *mutable,
                inner: self.encode_type(inner)?,
            },
            Type::Pointer(inner) => ProductTypeRow::Pointer(self.encode_type(inner)?),
            Type::TypeVar(_) => {
                return Err("product artifacts cannot serialize TypeVar types".to_string());
            }
            Type::Generic(param) => ProductTypeRow::Generic(ProductGenericParamId {
                owner: product_def_id(param.owner),
                index: param.index,
            }),
            Type::Projection {
                ty,
                trait_id,
                assoc_type,
                trait_args,
            } => ProductTypeRow::Projection {
                ty: self.encode_type(ty)?,
                trait_id: product_def_id(*trait_id),
                assoc_type: ProductAssociatedTypeKey {
                    owner: product_def_id(assoc_type.owner),
                    assoc_type_id: assoc_type.assoc_type_id,
                },
                trait_args: self.encode_types(trait_args)?,
            },
            Type::Error => ProductTypeRow::Error,
        })
    }

    pub(super) fn encode_types(&mut self, types: &[Type]) -> Result<Vec<ProductTypeId>, String> {
        types.iter().map(|ty| self.encode_type(ty)).collect()
    }
}

pub(super) struct ProductTypeDecoder<'a> {
    table: &'a ProductTypeTable,
    stack: Vec<ProductTypeId>,
}

impl<'a> ProductTypeDecoder<'a> {
    pub(super) fn new(table: &'a ProductTypeTable) -> Self {
        Self { table, stack: Vec::new() }
    }

    pub(super) fn decode_type(&mut self, id: ProductTypeId) -> Result<Type, String> {
        let row = self
            .table
            .rows
            .get(id.0 as usize)
            .ok_or_else(|| format!("unknown product type id {}", id.0))?;
        if self.stack.contains(&id) {
            return Err(format!("recursive product type id {}", id.0));
        }
        self.stack.push(id);
        let decoded = self.decode_row(row);
        self.stack.pop();
        decoded
    }

    fn decode_row(&mut self, row: &ProductTypeRow) -> Result<Type, String> {
        Ok(match row {
            ProductTypeRow::I8 => Type::I8,
            ProductTypeRow::I16 => Type::I16,
            ProductTypeRow::I32 => Type::I32,
            ProductTypeRow::I64 => Type::I64,
            ProductTypeRow::U8 => Type::U8,
            ProductTypeRow::U16 => Type::U16,
            ProductTypeRow::U32 => Type::U32,
            ProductTypeRow::U64 => Type::U64,
            ProductTypeRow::F32 => Type::F32,
            ProductTypeRow::F64 => Type::F64,
            ProductTypeRow::Bool => Type::Bool,
            ProductTypeRow::Str => Type::Str,
            ProductTypeRow::Char => Type::Char,
            ProductTypeRow::Unit => Type::Unit,
            ProductTypeRow::Never => Type::Never,
            ProductTypeRow::Slice(inner) => Type::Slice(Box::new(self.decode_type(*inner)?)),
            ProductTypeRow::Array(inner, len) => {
                Type::Array(Box::new(self.decode_type(*inner)?), *len)
            }
            ProductTypeRow::Tuple(elems) => Type::Tuple(self.decode_types(elems)?),
            ProductTypeRow::Function(args, ret) => {
                Type::Function(self.decode_types(args)?, Box::new(self.decode_type(*ret)?))
            }
            ProductTypeRow::Struct { id, args } => Type::Struct {
                id: def_id(*id),
                args: self.decode_types(args)?,
            },
            ProductTypeRow::Enum { id, args } => Type::Enum {
                id: def_id(*id),
                args: self.decode_types(args)?,
            },
            ProductTypeRow::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.decode_type(*inner)?),
            },
            ProductTypeRow::Pointer(inner) => Type::Pointer(Box::new(self.decode_type(*inner)?)),
            ProductTypeRow::Generic(param) => Type::Generic(GenericParamId {
                owner: def_id(param.owner),
                index: param.index,
            }),
            ProductTypeRow::Projection {
                ty,
                trait_id,
                assoc_type,
                trait_args,
            } => Type::Projection {
                ty: Box::new(self.decode_type(*ty)?),
                trait_id: def_id(*trait_id),
                assoc_type: AssociatedTypeKey {
                    owner: def_id(assoc_type.owner),
                    assoc_type_id: assoc_type.assoc_type_id,
                },
                trait_args: self.decode_types(trait_args)?,
            },
            ProductTypeRow::Error => Type::Error,
        })
    }

    pub(super) fn decode_types(&mut self, ids: &[ProductTypeId]) -> Result<Vec<Type>, String> {
        ids.iter().map(|id| self.decode_type(*id)).collect()
    }
}

fn product_def_id(id: DefId) -> ProductDefId {
    ProductDefId {
        crate_id: ProductCrateId(id.crate_id.raw()),
        local_id: ProductLocalDefId(id.local.raw()),
    }
}

fn def_id(id: ProductDefId) -> DefId {
    DefId::new(CrateId(id.crate_id.0), LocalDefId(id.local_id.0))
}

#[cfg(test)]
pub(super) fn encode_types_for_test(
    types: &[Type],
) -> Result<(ProductTypeTable, Vec<ProductTypeId>), String> {
    let mut encoder = ProductTypeEncoder::new();
    let ids = encoder.encode_types(types)?;
    Ok((encoder.finish(), ids))
}

#[cfg(test)]
pub(super) fn decode_type_for_test(
    table: &ProductTypeTable,
    id: ProductTypeId,
) -> Result<Type, String> {
    ProductTypeDecoder::new(table).decode_type(id)
}
```

- [ ] **Step 5: Run codec tests**

Run:

```bash
cargo test -p rock-lib product_type_table_
```

Expected: all codec tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add lib/src/products.rs lib/src/products/type_table.rs
git commit -m "add portable product type table codec"
```

Expected: commit succeeds.

---

## Task 3: Serialized Product DTO Shell

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/products/type_table.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/products/type_table.rs`

- [ ] **Step 1: Write failing empty artifact shell test**

In `lib/src/products/type_table.rs`, add a test that uses the public artifact API and checks roundtrip still returns in-memory `CompilerProducts`:

```rust
#[test]
fn serialized_product_artifact_roundtrips_empty_products() {
    let products = crate::products::CompilerProducts {
        crate_identity: crate::products::ProductCrateIdentity::local("empty".to_string()),
        identity_table: crate::products::ProductIdentityTable::default(),
        metadata: crate::products::ProductMetadata::default(),
        bodies: crate::products::ProductBodies::default(),
        link: crate::products::ProductLinkData::default(),
        dependencies: Vec::new(),
        source_fingerprint: crate::products::ProductSourceFingerprint::default(),
        infix_precedence: std::collections::BTreeMap::new(),
        proc_macros: Vec::new(),
    };
    let bytes = products.to_artifact_bytes().unwrap();
    let roundtrip = crate::products::CompilerProducts::from_artifact_bytes(&bytes).unwrap();

    assert_eq!(roundtrip.crate_identity.name, "empty");
}
```

- [ ] **Step 2: Run shell tests to verify they fail**

Run:

```bash
cargo test -p rock-lib serialized_product_artifact_roundtrips_empty_products
```

Expected: compile failure for missing `SerializedProductArtifact` and DTO conversion.

- [ ] **Step 3: Add DTO shell structs and top-level conversions**

In `lib/src/products/type_table.rs`, add the serialized artifact shell above tests:

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::products::{
    CompilerProducts, ProductBodies, ProductCrateIdentity, ProductDependencyIdentity,
    ProductIdentityTable, ProductLinkData, ProductMetadata, ProductSourceFingerprint,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedProductArtifact {
    pub format_version: u32,
    pub type_table: ProductTypeTable,
    pub products: SerializedCompilerProducts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedCompilerProducts {
    pub crate_identity: ProductCrateIdentity,
    pub identity_table: ProductIdentityTable,
    pub metadata: SerializedProductMetadata,
    pub bodies: SerializedProductBodies,
    pub link: ProductLinkData,
    pub dependencies: Vec<ProductDependencyIdentity>,
    pub source_fingerprint: ProductSourceFingerprint,
    pub infix_precedence: BTreeMap<String, u8>,
    pub proc_macros: Vec<crate::macro_expansion::proc_macro::ProcMacroArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct SerializedProductMetadata {
    pub functions: BTreeMap<crate::products::ProductDefId, SerializedHirFunction>,
    pub structs: BTreeMap<crate::products::ProductDefId, SerializedHirStruct>,
    pub enums: BTreeMap<crate::products::ProductDefId, SerializedHirEnum>,
    pub traits: BTreeMap<crate::products::ProductDefId, SerializedHirTrait>,
    pub impls: BTreeMap<crate::products::ProductDefId, SerializedHirImpl>,
    pub externs: BTreeMap<crate::products::ProductDefId, SerializedHirExtern>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct SerializedProductBodies {
    pub functions: BTreeMap<crate::products::ProductDefId, SerializedHirFunction>,
    pub generic_impls: BTreeMap<crate::products::ProductDefId, SerializedHirImpl>,
    pub trait_default_methods: BTreeMap<crate::products::ProductDefId, SerializedHirFunction>,
}

pub(super) fn products_to_artifact(
    products: &CompilerProducts,
    format_version: u32,
) -> Result<SerializedProductArtifact, String> {
    let mut encoder = ProductTypeEncoder::new();
    let serialized_products = SerializedCompilerProducts::encode(products, &mut encoder)?;
    Ok(SerializedProductArtifact {
        format_version,
        type_table: encoder.finish(),
        products: serialized_products,
    })
}

pub(super) fn artifact_to_products(
    artifact: SerializedProductArtifact,
) -> Result<CompilerProducts, String> {
    let mut decoder = ProductTypeDecoder::new(&artifact.type_table);
    artifact.products.decode(&mut decoder)
}
```

Temporarily define serialized HIR aliases as wrappers that will be fully expanded in the next tasks:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirFunction(crate::hir::HirFunction);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirStruct(crate::hir::HirStruct);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirEnum(crate::hir::HirEnum);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirTrait(crate::hir::HirTrait);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirImpl(crate::hir::HirImpl);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirExtern(crate::hir::HirExtern);
```

The temporary wrappers keep the shell compiling for empty products only. Task 4 removes these raw-HIR wrappers before product artifacts are considered schema-complete. Do not add or commit non-empty artifact tests until Task 4, because this shell is not intended to pass non-empty product serialization yet.

- [ ] **Step 4: Add top-level encode/decode implementations**

In `lib/src/products/type_table.rs`, add:

```rust
impl SerializedCompilerProducts {
    fn encode(
        products: &CompilerProducts,
        encoder: &mut ProductTypeEncoder,
    ) -> Result<Self, String> {
        Ok(Self {
            crate_identity: products.crate_identity.clone(),
            identity_table: products.identity_table.clone(),
            metadata: SerializedProductMetadata::encode(&products.metadata, encoder)?,
            bodies: SerializedProductBodies::encode(&products.bodies, encoder)?,
            link: products.link.clone(),
            dependencies: products.dependencies.clone(),
            source_fingerprint: products.source_fingerprint.clone(),
            infix_precedence: products.infix_precedence.clone(),
            proc_macros: products.proc_macros.clone(),
        })
    }

    fn decode(self, decoder: &mut ProductTypeDecoder<'_>) -> Result<CompilerProducts, String> {
        Ok(CompilerProducts {
            crate_identity: self.crate_identity,
            identity_table: self.identity_table,
            metadata: self.metadata.decode(decoder)?,
            bodies: self.bodies.decode(decoder)?,
            link: self.link,
            dependencies: self.dependencies,
            source_fingerprint: self.source_fingerprint,
            infix_precedence: self.infix_precedence,
            proc_macros: self.proc_macros,
        })
    }
}
```

Add matching metadata/body conversion implementations that map the BTreeMap values through `SerializedHir*::encode` and `decode` methods. For the temporary wrappers, `encode` returns `Ok(Self(value.clone()))`, and `decode` returns `Ok(self.0)`.

- [ ] **Step 5: Wire product artifact APIs**

In `lib/src/products.rs`, remove the old raw artifact container:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductArtifact {
    pub format_version: u32,
    pub products: CompilerProducts,
}
```

The serialized artifact container now lives in `type_table::SerializedProductArtifact` and stores `SerializedCompilerProducts` plus `ProductTypeTable`.

In `lib/src/products.rs`, change `to_artifact_bytes` to:

```rust
pub fn to_artifact_bytes(&self) -> Result<Vec<u8>, String> {
    let artifact = type_table::products_to_artifact(self, PRODUCT_ARTIFACT_FORMAT_VERSION)?;

    bincode::serialize(&artifact)
        .map_err(|err| format!("Failed to serialize product artifact: {}", err))
}
```

Change `from_artifact_bytes` full deserialize to:

```rust
let artifact: type_table::SerializedProductArtifact = bincode::deserialize(bytes)
    .map_err(|err| format!("Failed to deserialize product artifact: {}", err))?;

type_table::artifact_to_products(artifact)
```

Leave the first `u32` format-version read unchanged.

Update tests that previously constructed `ProductArtifact { format_version, products }` for unsupported versions to serialize only the version prefix:

```rust
let bytes = bincode::serialize(&(PRODUCT_ARTIFACT_FORMAT_VERSION + 1)).unwrap();
let err = CompilerProducts::from_artifact_bytes(&bytes).unwrap_err();
assert!(err.contains("Unsupported product artifact format"));
```

- [ ] **Step 6: Run shell tests**

Run:

```bash
cargo test -p rock-lib serialized_product_artifact_roundtrips_empty_products
```

Expected: PASS.

- [ ] **Step 7: Commit shell**

Run:

```bash
git add lib/src/products.rs lib/src/products/type_table.rs
git commit -m "add product artifact type table shell"
```

Expected: commit succeeds with focused tests passing.

---

## Task 4: Convert Top-Level HIR Product Types To ProductTypeId

**Files:**
- Modify: `lib/src/products/type_table.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/products/type_table.rs`

- [ ] **Step 1: Add focused declaration roundtrip tests**

In `lib/src/products.rs`, add tests next to existing product artifact tests:

```rust
#[test]
fn compiler_products_v23_artifact_contains_type_table() {
    let hir = resolved_hir_for_products();
    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let bytes = products.to_artifact_bytes().unwrap();

    assert!(super::type_table::artifact_type_table_len_for_test(&bytes).unwrap() > 0);
}
```

Add declaration roundtrip tests in the same `products.rs` test module:

```rust
#[test]
fn serialized_product_artifact_roundtrips_function_signature_types() {
    let resolved = resolved_hir_for_products();
    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &resolved,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let roundtrip = CompilerProducts::from_artifact_bytes(
        &products.to_artifact_bytes().unwrap(),
    )
    .unwrap();
    let function = roundtrip
        .metadata
        .functions
        .values()
        .find(|function| function.name == "identity")
        .unwrap();

    assert_eq!(function.params[0].ty, Type::I64);
    assert_eq!(function.ret_type, Type::I64);
}

#[test]
fn serialized_product_artifact_roundtrips_struct_enum_trait_impl_and_extern_types() {
    let resolved = resolved_hir_for_products();
    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &resolved,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let roundtrip = CompilerProducts::from_artifact_bytes(
        &products.to_artifact_bytes().unwrap(),
    )
    .unwrap();

    assert!(roundtrip.metadata.structs.values().any(|strukt| strukt.name == "Box"));
    assert!(roundtrip.metadata.enums.values().any(|enm| enm.name == "Maybe"));
    assert!(roundtrip.metadata.traits.values().any(|trait_def| trait_def.name == "Show"));
    assert!(roundtrip.metadata.impls.values().any(|imp| !imp.receiver_arg_types.is_empty()));
    assert!(roundtrip.metadata.externs.values().any(|ext| ext.ret == crate::types::Type::I32));
}
```

- [ ] **Step 2: Run declaration tests to verify failure**

Run:

```bash
cargo test -p rock-lib serialized_product_artifact_roundtrips_function_signature_types serialized_product_artifact_roundtrips_struct_enum_trait_impl_and_extern_types compiler_products_v23_artifact_contains_type_table
```

Expected: `compiler_products_v23_artifact_contains_type_table` still fails because wrappers do not populate the table.

- [ ] **Step 3: Replace raw wrapper structs for declaration-level HIR types**

In `lib/src/products/type_table.rs`, replace raw wrappers for function/param/field/struct/enum/trait/signature/impl/assoc/extern declarations with DTOs containing `ProductTypeId` where `Type` appears:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirFunction {
    pub id: crate::ids::DefId,
    pub name: String,
    pub qualified_name: Option<String>,
    pub generic_params: Vec<String>,
    pub generic_param_ids: Vec<ProductGenericParamId>,
    pub generic_bounds: Vec<(ProductGenericParamId, Vec<SerializedTraitBound>)>,
    pub params: Vec<SerializedHirParam>,
    pub ret_type: ProductTypeId,
    pub body: SerializedHirBlock,
    pub is_curried: bool,
    pub is_method: bool,
    pub self_receiver: Option<crate::ast::SelfReceiverMode>,
    pub is_unsafe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirParam {
    pub name: String,
    pub local_id: crate::ids::HirLocalId,
    pub ty: ProductTypeId,
    pub mutable: bool,
    pub is_ref: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirField {
    pub id: crate::ids::FieldId,
    pub name: String,
    pub ty: ProductTypeId,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirStruct {
    pub id: crate::ids::DefId,
    pub name: String,
    pub generic_params: Vec<String>,
    pub fields: Vec<SerializedHirField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirEnum {
    pub id: crate::ids::DefId,
    pub name: String,
    pub generic_params: Vec<String>,
    pub variants: Vec<SerializedHirVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirVariant {
    pub id: crate::ids::VariantId,
    pub name: String,
    pub fields: SerializedHirVariantFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum SerializedHirVariantFields {
    Named(Vec<SerializedHirField>),
    Positional(Vec<ProductTypeId>),
    Unit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirTrait {
    pub id: crate::ids::DefId,
    pub name: String,
    pub generic_params: Vec<String>,
    pub associated_types: Vec<crate::hir::HirAssociatedTypeDecl>,
    pub methods: std::collections::HashMap<String, SerializedHirFunction>,
    pub signatures: std::collections::HashMap<String, SerializedHirFunctionSig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirFunctionSig {
    pub id: crate::ids::DefId,
    pub name: String,
    pub generic_params: Vec<String>,
    pub generic_param_ids: Vec<ProductGenericParamId>,
    pub params: Vec<ProductTypeId>,
    pub ret: ProductTypeId,
    pub generic_bounds: Vec<(ProductGenericParamId, Vec<SerializedTraitBound>)>,
    pub self_receiver: Option<crate::ast::SelfReceiverMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirImpl {
    pub id: crate::ids::DefId,
    pub owner: crate::hir::HirImplOwner,
    pub type_name: String,
    pub type_generics: Vec<String>,
    pub receiver_arg_types: Vec<ProductTypeId>,
    pub trait_name: Option<String>,
    pub trait_id: Option<crate::ids::DefId>,
    pub trait_generics: Vec<String>,
    pub trait_arg_types: Vec<ProductTypeId>,
    pub associated_types: Vec<SerializedHirAssociatedTypeDef>,
    pub bounds: Vec<SerializedTraitBound>,
    pub methods: std::collections::HashMap<String, SerializedHirFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirAssociatedTypeDef {
    pub id: crate::ids::AssocTypeId,
    pub name: String,
    pub ty: ProductTypeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedTraitBound {
    pub trait_id: ProductDefId,
    pub type_args: Vec<ProductTypeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirExtern {
    pub id: crate::ids::DefId,
    pub name: String,
    pub params: Vec<ProductTypeId>,
    pub ret: ProductTypeId,
    pub variadic: bool,
}
```

Use `ProductGenericParamId` and `ProductDefId` inside serialized type-related DTO fields so no raw producer `DefId` is serialized inside type rows, generic IDs, or trait bounds. Top-level HIR item IDs can remain `DefId` in the serialized DTO for this task because current product artifact remapping already validates and remaps product IDs; Task 5 validates the type payloads themselves use table refs.

- [ ] **Step 4: Implement declaration encode/decode walkers**

In `lib/src/products/type_table.rs`, implement `encode` and `decode` methods for each DTO from Step 3.

Use these helper functions:

```rust
fn encode_generic_param_id(id: crate::types::GenericParamId) -> ProductGenericParamId {
    ProductGenericParamId {
        owner: product_def_id(id.owner),
        index: id.index,
    }
}

fn decode_generic_param_id(id: ProductGenericParamId) -> crate::types::GenericParamId {
    crate::types::GenericParamId {
        owner: def_id(id.owner),
        index: id.index,
    }
}

fn encode_generic_bounds(
    bounds: &crate::hir::HirGenericBounds,
    encoder: &mut ProductTypeEncoder,
) -> Result<Vec<(ProductGenericParamId, Vec<SerializedTraitBound>)>, String> {
    bounds
        .iter()
        .map(|(generic, bounds)| {
            Ok((
                encode_generic_param_id(*generic),
                bounds
                    .iter()
                    .map(|bound| SerializedTraitBound::encode(bound, encoder))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect()
}

fn decode_generic_bounds(
    bounds: Vec<(ProductGenericParamId, Vec<SerializedTraitBound>)>,
    decoder: &mut ProductTypeDecoder<'_>,
) -> Result<crate::hir::HirGenericBounds, String> {
    bounds
        .into_iter()
        .map(|(generic, bounds)| {
            Ok((
                decode_generic_param_id(generic),
                bounds
                    .into_iter()
                    .map(|bound| bound.decode(decoder))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect()
}
```

Every method must call `encoder.encode_type` for `Type` fields and `decoder.decode_type` for `ProductTypeId` fields.

- [ ] **Step 5: Run declaration tests**

Run:

```bash
cargo test -p rock-lib compiler_products_v23_artifact_contains_type_table
cargo test -p rock-lib serialized_product_artifact_roundtrips_function_signature_types
cargo test -p rock-lib serialized_product_artifact_roundtrips_struct_enum_trait_impl_and_extern_types
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add lib/src/products/type_table.rs lib/src/products.rs
git commit -m "serialize product declarations through type table"
```

Expected: commit succeeds.

---

## Task 5: Convert Product Function Bodies, Expressions, And Patterns

**Files:**
- Modify: `lib/src/products/type_table.rs`
- Test: `lib/src/products.rs`
- Test: `lib/src/products/type_table.rs`

- [ ] **Step 1: Write failing nested body coverage tests**

In `lib/src/products.rs`, add a test that mutates the product fixture to include all nested type-bearing body forms:

```rust
#[test]
fn compiler_products_type_table_roundtrips_nested_body_type_locations() {
    let mut resolved = resolved_hir_for_products();
    let function = resolved
        .program
        .functions
        .get_mut("identity")
        .expect("identity fixture exists");
    function.body = crate::hir::HirBlock {
        stmts: vec![
            crate::hir::HirStmt::Let {
                name: "x".to_string(),
                local_id: crate::ids::HirLocalId(1),
                ty: Type::I64,
                value: crate::hir::HirExpr {
                    kind: crate::hir::HirExprKind::Cast(
                        Box::new(crate::hir::HirExpr {
                            kind: crate::hir::HirExprKind::IntLiteral(7),
                            ty: Type::I64,
                            span: crate::span::Span::default(),
                        }),
                        Type::I32,
                    ),
                    ty: Type::I32,
                    span: crate::span::Span::default(),
                },
                mutable: false,
            },
            crate::hir::HirStmt::Expr(crate::hir::HirExpr {
                kind: crate::hir::HirExprKind::Lambda {
                    params: vec![crate::hir::HirParam {
                        name: "arg".to_string(),
                        local_id: crate::ids::HirLocalId(2),
                        ty: Type::I64,
                        mutable: false,
                        is_ref: false,
                    }],
                    body: crate::hir::HirBlock {
                        stmts: Vec::new(),
                        ty: Type::I64,
                    },
                    captures: vec![crate::hir::HirClosureCapture {
                        name: "cap".to_string(),
                        local_id: crate::ids::HirLocalId(3),
                        kind: crate::hir::HirClosureCaptureKind::Move,
                        ty: Type::I64,
                    }],
                },
                ty: Type::Function(vec![Type::I64], Box::new(Type::I64)),
                span: crate::span::Span::default(),
            }),
        ],
        ty: Type::I64,
    };
    resolved.type_context = crate::type_context::TypeContext::new();
    resolved.type_ids = crate::hir::collect_hir_type_ids(&resolved.program, &mut resolved.type_context);

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &resolved,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let roundtrip = CompilerProducts::from_artifact_bytes(&products.to_artifact_bytes().unwrap()).unwrap();
    let function = roundtrip
        .metadata
        .functions
        .values()
        .find(|function| function.name == "identity")
        .unwrap();

    assert_eq!(function.body.ty, Type::I64);
    match &function.body.stmts[0] {
        crate::hir::HirStmt::Let { ty, value, .. } => {
            assert_eq!(*ty, Type::I64);
            assert_eq!(value.ty, Type::I32);
            match &value.kind {
                crate::hir::HirExprKind::Cast(_, cast_ty) => assert_eq!(*cast_ty, Type::I32),
                other => panic!("expected cast expression, got {other:?}"),
            }
        }
        other => panic!("expected let statement, got {other:?}"),
    }
}
```

Add a second pattern coverage test:

```rust
#[test]
fn compiler_products_type_table_roundtrips_struct_pattern_type_args() {
    let mut resolved = resolved_hir_for_products();
    let function = resolved
        .program
        .functions
        .get_mut("identity")
        .expect("identity fixture exists");
    function.body = crate::hir::HirBlock {
        stmts: vec![crate::hir::HirStmt::Expr(crate::hir::HirExpr {
            kind: crate::hir::HirExprKind::Match {
                scrutinee: Box::new(crate::hir::HirExpr {
                    kind: crate::hir::HirExprKind::Var("box_value".to_string()),
                    ty: Type::Struct {
                        id: crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(2)),
                        args: vec![Type::I64],
                    },
                    span: crate::span::Span::default(),
                }),
                arms: vec![crate::hir::HirMatchArm {
                    pattern: crate::hir::HirPattern::Struct(
                        "Box".to_string(),
                        Some(crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(2))),
                        vec![Type::I64],
                        Vec::new(),
                    ),
                    guard: None,
                    body: crate::hir::HirBlock {
                        stmts: Vec::new(),
                        ty: Type::I64,
                    },
                }],
            },
            ty: Type::I64,
            span: crate::span::Span::default(),
        })],
        ty: Type::I64,
    };
    resolved.type_context = crate::type_context::TypeContext::new();
    resolved.type_ids = crate::hir::collect_hir_type_ids(&resolved.program, &mut resolved.type_context);

    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &resolved,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let roundtrip = CompilerProducts::from_artifact_bytes(&products.to_artifact_bytes().unwrap()).unwrap();
    let function = roundtrip
        .metadata
        .functions
        .values()
        .find(|function| function.name == "identity")
        .unwrap();

    match &function.body.stmts[0] {
        crate::hir::HirStmt::Expr(expr) => match &expr.kind {
            crate::hir::HirExprKind::Match { arms, .. } => match &arms[0].pattern {
                crate::hir::HirPattern::Struct(_, _, args, _) => assert_eq!(args, &vec![Type::I64]),
                other => panic!("expected struct pattern, got {other:?}"),
            },
            other => panic!("expected match expression, got {other:?}"),
        },
        other => panic!("expected expression statement, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run nested tests to verify failure**

Run:

```bash
cargo test -p rock-lib compiler_products_type_table_roundtrips_nested_body_type_locations compiler_products_type_table_roundtrips_struct_pattern_type_args
```

Expected: fail because body/pattern DTO conversion is not complete.

- [ ] **Step 3: Add serialized body, statement, expression, pattern DTOs**

In `lib/src/products/type_table.rs`, add DTOs for the remaining HIR structures:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirClosureCapture {
    pub name: String,
    pub local_id: crate::ids::HirLocalId,
    pub kind: crate::hir::HirClosureCaptureKind,
    pub ty: ProductTypeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirBlock {
    pub stmts: Vec<SerializedHirStmt>,
    pub ty: ProductTypeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum SerializedHirStmt {
    Let {
        name: String,
        local_id: crate::ids::HirLocalId,
        ty: ProductTypeId,
        value: SerializedHirExpr,
        mutable: bool,
    },
    Expr(SerializedHirExpr),
    Return(Option<SerializedHirExpr>),
    Break(Option<SerializedHirExpr>),
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirExpr {
    pub kind: SerializedHirExprKind,
    pub ty: ProductTypeId,
    pub span: crate::span::Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum SerializedHirExprKind {
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    CharLiteral(char),
    ArrayLiteral(Vec<SerializedHirExpr>),
    TupleLiteral(Vec<SerializedHirExpr>),
    Unit,
    Var(String),
    ResolvedVar(crate::hir::HirVarRef),
    FieldAccess(Box<SerializedHirExpr>, String, Option<crate::hir::HirFieldLocation>),
    TupleIndex(Box<SerializedHirExpr>, u32),
    Index(Box<SerializedHirExpr>, Box<SerializedHirExpr>),
    BinOp(crate::hir::BinOp, Box<SerializedHirExpr>, Box<SerializedHirExpr>),
    UnaryOp(crate::hir::UnaryOp, Box<SerializedHirExpr>),
    Call(Box<SerializedHirExpr>, Vec<SerializedHirExpr>, Option<crate::hir::HirCallTarget>),
    MethodCall(
        Box<SerializedHirExpr>,
        String,
        Vec<SerializedHirExpr>,
        Option<crate::ast::SelfReceiverMode>,
        Option<SerializedHirMethodCallTarget>,
    ),
    StructLiteral(String, Option<crate::ids::DefId>, Vec<SerializedHirStructLiteralField>),
    EnumVariant(String, String, Vec<SerializedHirExpr>, Option<crate::hir::HirVariantLocation>),
    If {
        condition: Box<SerializedHirExpr>,
        then_branch: SerializedHirBlock,
        else_branch: Option<SerializedHirBlock>,
    },
    Match {
        scrutinee: Box<SerializedHirExpr>,
        arms: Vec<SerializedHirMatchArm>,
    },
    While {
        condition: Box<SerializedHirExpr>,
        body: SerializedHirBlock,
    },
    For {
        var: String,
        local_id: crate::ids::HirLocalId,
        iter: Box<SerializedHirExpr>,
        body: SerializedHirBlock,
    },
    Loop(SerializedHirBlock),
    Block(SerializedHirBlock),
    Lambda {
        params: Vec<SerializedHirParam>,
        body: SerializedHirBlock,
        captures: Vec<SerializedHirClosureCapture>,
    },
    Ref(bool, Box<SerializedHirExpr>),
    Deref(Box<SerializedHirExpr>),
    Cast(Box<SerializedHirExpr>, ProductTypeId),
    Assign(Box<SerializedHirExpr>, Box<SerializedHirExpr>),
    Range(Box<SerializedHirExpr>, Box<SerializedHirExpr>),
    Intrinsic { name: String, args: Vec<SerializedHirExpr> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirMethodCallTarget {
    pub impl_id: Option<crate::ids::DefId>,
    pub trait_id: Option<crate::ids::DefId>,
    pub trait_args: Vec<ProductTypeId>,
    pub method_id: crate::ids::DefId,
    pub from_index_operator: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirStructLiteralField {
    pub name: String,
    pub value: SerializedHirExpr,
    pub field: Option<crate::hir::HirFieldLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirMatchArm {
    pub pattern: SerializedHirPattern,
    pub guard: Option<SerializedHirExpr>,
    pub body: SerializedHirBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SerializedHirStructPatternField {
    pub name: String,
    pub field: Option<crate::hir::HirFieldLocation>,
    pub pattern: SerializedHirPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum SerializedHirPattern {
    Wildcard,
    Binding {
        name: String,
        local_id: crate::ids::HirLocalId,
        mutable: bool,
    },
    Literal(crate::hir::HirLiteralPattern),
    Tuple(Vec<SerializedHirPattern>),
    Struct(String, Option<crate::ids::DefId>, Vec<ProductTypeId>, Vec<SerializedHirStructPatternField>),
    Enum(String, String, Option<crate::hir::HirVariantLocation>, Vec<SerializedHirPattern>),
    Or(Vec<SerializedHirPattern>),
}
```

The location types above are defined in `lib/src/hir/mod.rs`; keep the serialized fields isomorphic to the current HIR variants.

- [ ] **Step 4: Implement body/pattern encode/decode walkers**

For every DTO from Step 3, implement `encode` and `decode`. The required rules are:

```text
HirBlock.ty -> ProductTypeId
HirStmt::Let.ty -> ProductTypeId
HirExpr.ty -> ProductTypeId
HirExprKind::Cast target -> ProductTypeId
HirExprKind::MethodCall target.trait_args -> Vec<ProductTypeId>
HirExprKind::Lambda params/captures/body -> serialized nested DTOs
HirPattern::Struct type args -> Vec<ProductTypeId>
All nested HirExpr, HirBlock, HirStmt, HirPattern values recurse through DTO conversion.
All non-type identity sidecars are copied exactly.
```

The code must use match arms that mirror `HirExprKind`, `HirStmt`, and `HirPattern` one-for-one. For example:

```rust
impl SerializedHirExpr {
    fn encode(
        expr: &crate::hir::HirExpr,
        encoder: &mut ProductTypeEncoder,
    ) -> Result<Self, String> {
        Ok(Self {
            kind: SerializedHirExprKind::encode(&expr.kind, encoder)?,
            ty: encoder.encode_type(&expr.ty)?,
            span: expr.span,
        })
    }

    fn decode(self, decoder: &mut ProductTypeDecoder<'_>) -> Result<crate::hir::HirExpr, String> {
        Ok(crate::hir::HirExpr {
            kind: self.kind.decode(decoder)?,
            ty: decoder.decode_type(self.ty)?,
            span: self.span,
        })
    }
}
```

- [ ] **Step 5: Run nested body tests**

Run:

```bash
cargo test -p rock-lib compiler_products_type_table_roundtrips_nested_body_type_locations
cargo test -p rock-lib compiler_products_type_table_roundtrips_struct_pattern_type_args
cargo test -p rock-lib compiler_products_v23_artifact_contains_type_table
```

Expected: all tests pass.

- [ ] **Step 6: Run product artifact suite**

Run:

```bash
cargo test -p rock-lib product_artifact
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add lib/src/products.rs lib/src/products/type_table.rs
git commit -m "serialize product bodies through type table"
```

Expected: commit succeeds.

---

## Task 6: Remove Raw HIR Type Serialization From Artifact DTOs

**Files:**
- Modify: `lib/src/products/type_table.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Add schema guard test**

In `lib/src/products.rs`, add:

```rust
#[test]
fn product_artifact_schema_does_not_serialize_compiler_products_directly() {
    let hir = resolved_hir_for_products();
    let products = CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("demo".to_string()),
        &hir,
        Vec::new(),
        BTreeMap::new(),
        ProductSourceFingerprint::default(),
        ProductLinkData::default(),
    );
    let bytes = products.to_artifact_bytes().unwrap();

    let direct_products_decode = bincode::deserialize::<CompilerProducts>(&bytes);

    assert!(
        direct_products_decode.is_err(),
        "v23 artifacts must not be raw CompilerProducts payloads"
    );
    assert!(super::type_table::artifact_type_table_len_for_test(&bytes).unwrap() > 0);
}
```

- [ ] **Step 2: Run schema guard test**

Run:

```bash
cargo test -p rock-lib product_artifact_schema_does_not_serialize_compiler_products_directly
```

Expected: PASS after Tasks 3-5. A failure means `to_artifact_bytes` is still serializing raw `CompilerProducts` or is not populating the type table.

- [ ] **Step 3: Search for temporary raw wrappers**

Run:

```bash
rg "struct SerializedHir.*\(crate::hir|HirFunction\)|HirStruct\)|HirEnum\)|HirTrait\)|HirImpl\)|HirExtern\)" lib/src/products/type_table.rs
```

Expected: no output.

- [ ] **Step 4: Search for serialized DTO Type fields**

Run:

```bash
rg ": Type|Vec<Type>|Box<Type>|crate::types::Type" lib/src/products/type_table.rs
```

Expected: matches only in codec function signatures and tests that operate on in-memory `Type`, not in `Serialized*` DTO field definitions.

- [ ] **Step 5: Commit if cleanup changes were made**

If Step 3 or Step 4 required code cleanup, run:

```bash
git add lib/src/products/type_table.rs lib/src/products.rs
git commit -m "remove raw type payloads from artifact dto"
```

Expected: commit succeeds when cleanup changes were required. When Step 3 and Step 4 already produce the expected output, do not create a commit for this task.

---

## Task 7: Invalid Type Table Validation

**Files:**
- Modify: `lib/src/products/type_table.rs`
- Test: `lib/src/products/type_table.rs`

- [ ] **Step 1: Add invalid reference and cycle tests**

In `lib/src/products/type_table.rs`, add:

```rust
#[test]
fn product_type_table_rejects_recursive_type_rows() {
    let table = ProductTypeTable {
        rows: vec![ProductTypeRow::Slice(ProductTypeId(0))],
    };
    let err = decode_type_for_test(&table, ProductTypeId(0)).unwrap_err();

    assert!(err.contains("recursive product type id 0"), "unexpected error: {err}");
}

#[test]
fn product_type_table_rejects_invalid_nested_type_ids() {
    let table = ProductTypeTable {
        rows: vec![ProductTypeRow::Tuple(vec![ProductTypeId(4)])],
    };
    let err = decode_type_for_test(&table, ProductTypeId(0)).unwrap_err();

    assert!(err.contains("unknown product type id 4"), "unexpected error: {err}");
}
```

- [ ] **Step 2: Run validation tests to verify behavior**

Run:

```bash
cargo test -p rock-lib product_type_table_rejects_recursive_type_rows product_type_table_rejects_invalid_nested_type_ids
```

Expected: both tests pass. When a test fails, fix only `ProductTypeDecoder::decode_type` and rerun this command.

- [ ] **Step 3: Run validation suite**

Run:

```bash
cargo test -p rock-lib product_type_table_rejects
```

Expected: all validation tests pass.

- [ ] **Step 4: Commit**

Run:

```bash
git add lib/src/products/type_table.rs
git commit -m "validate product artifact type tables"
```

Expected: commit succeeds.

---

## Task 8: Artifact Loader Consumer TypeContext Integration

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Strengthen loader reinterning test for v23 type table**

In `lib/src/products.rs`, expose a test-only helper:

```rust
#[cfg(test)]
pub(crate) fn artifact_type_table_len_for_test(bytes: &[u8]) -> Result<usize, String> {
    type_table::artifact_type_table_len_for_test(bytes)
}
```

In `lib/src/products/type_table.rs`, add the helper implementation:

```rust
#[cfg(test)]
pub(super) fn artifact_type_table_len_for_test(bytes: &[u8]) -> Result<usize, String> {
    let artifact: SerializedProductArtifact = bincode::deserialize(bytes)
        .map_err(|err| format!("failed to decode artifact for type-table test: {err}"))?;
    Ok(artifact.type_table.rows.len())
}
```

In `lib/src/crate_artifact/load.rs`, update `load_product_artifact_reinterns_structural_types_for_consumer_context` to assert the artifact has a non-empty v23 type table before loading:

```rust
let bytes = std::fs::read(&artifact_path).unwrap();
assert!(crate::products::artifact_type_table_len_for_test(&bytes).unwrap() > 0);
```

- [ ] **Step 2: Run loader test**

Run:

```bash
cargo test -p rock-lib load_product_artifact_reinterns_structural_types_for_consumer_context
```

Expected: PASS. The existing loader path must re-intern decoded structural types into the supplied consumer context.

- [ ] **Step 3: Run named loader test**

Run:

```bash
cargo test -p rock-lib load_product_artifact_as_reinterns_structural_types_for_consumer_context
```

Expected: PASS.

- [ ] **Step 4: Commit if tests or helpers changed**

Run:

```bash
git add lib/src/crate_artifact/load.rs lib/src/products.rs lib/src/products/type_table.rs
git commit -m "verify artifact type table loader interning"
```

Expected: commit succeeds when files changed. When the loader tests already include the type-table assertion from an earlier task, do not create a commit for this task.

---

## Task 9: Product And Artifact Regression Suites

**Files:**
- Modify: files with failures only.
- No docs updates yet.

- [ ] **Step 1: Run product artifact suite**

Run:

```bash
cargo test -p rock-lib product_artifact
```

Expected: PASS.

- [ ] **Step 2: Run crate artifact loader suite**

Run:

```bash
cargo test -p rock-lib crate_artifact::load
```

Expected: PASS.

- [ ] **Step 3: Run source-free artifact integration tests**

Run:

```bash
cargo test -p rock-lib crate_artifact::tests::test_compile_with_source_free_product_artifact
cargo test -p rock-lib crate_artifact::tests::test_compile_generic_function_from_artifact_hir_bundle
cargo test -p rock-lib crate_artifact::tests::test_product_stdlib_artifact_preserves_string_method_abi_and_prelude_exports
```

Expected: all three commands pass.

- [ ] **Step 4: Fix any regression with TDD**

For each failure, write a focused failing test that reproduces the specific missing conversion or invalid decode behavior, run it red, make the minimal converter/decoder fix, and rerun the focused command from Steps 1-3.

- [ ] **Step 5: Commit if fixes were needed**

Run:

```bash
git add lib/src/products.rs lib/src/products/type_table.rs lib/src/crate_artifact/load.rs rock-shared/src/sysroot.rs
git commit -m "stabilize v23 product artifact regressions"
```

Expected: commit succeeds when files changed. When all regression commands pass without code changes, do not create a commit for this task.

---

## Task 10: Final Full Verification And Docs Closure

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Modify: `docs/superpowers/plans/2026-05-30-portable-product-artifact-type-table.md`

- [ ] **Step 1: Run full verification**

Run:

```bash
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Expected: PASS. Record exact test counts in this plan under `Final Verification Notes`.

- [ ] **Step 2: Run final artifact schema audit**

Run:

```bash
rg "ProductArtifact \{|CompilerProducts \{|: Type|Vec<Type>|Box<Type>|crate::types::Type" lib/src/products.rs lib/src/products/type_table.rs --glob '*.rs'
```

Expected: `ProductArtifact` serialization routes through `SerializedProductArtifact`; serialized DTO structs in `type_table.rs` contain `ProductTypeId` fields rather than structural `Type` fields. Matches for `Type` are allowed only in in-memory product APIs, encoder/decoder functions, and tests.

- [ ] **Step 3: Request final code review**

Use the `requesting-code-review` skill. Review scope:

```text
Portable product artifact type table and final Type Context/Semantic Types closure.
Verify artifact format v23 serializes product type payloads through ProductTypeTable/ProductTypeId, rejects v22 artifacts, does not serialize producer TypeId or raw producer DefIds inside type rows, preserves product artifact loading/remapping, and re-interns loaded types into consumer TypeContext.
```

Expected: reviewer returns APPROVED or findings.

- [ ] **Step 4: Fix review findings before docs**

If the reviewer requests changes, apply TDD fixes one finding at a time. Rerun focused tests for each fix, then rerun full verification from Step 1 and request re-review.

Expected: final review approval.

- [ ] **Step 5: Update master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, update the summary row for `Type Context And Semantic Types` from:

```markdown
| Type Context And Semantic Types | In progress | `lib/src/type_context/mod.rs`, `lib/src/hir/type_ids.rs`, `lib/src/infer/mod.rs`, `lib/src/mono/*`, `lib/src/mir/*`, `lib/src/codegen/*`, `lib/src/crate_artifact/load.rs`, `lib/src/products.rs` | `TypeContext`/`TypeId` now carries semantic type identity through finalized HIR, mono, MIR, borrowck/agreement, and codegen; product artifacts intentionally remain a structural compatibility boundary pending any future portable artifact type table |
```

to:

```markdown
| Type Context And Semantic Types | Complete | `lib/src/type_context/mod.rs`, `lib/src/hir/type_ids.rs`, `lib/src/infer/mod.rs`, `lib/src/mono/*`, `lib/src/mir/*`, `lib/src/codegen/*`, `lib/src/products/type_table.rs`, `lib/src/crate_artifact/load.rs`, `lib/src/products.rs` | `TypeContext`/`TypeId` carries semantic type identity through finalized HIR, mono, MIR, borrowck/agreement, and codegen; product artifacts serialize types through a portable artifact-local type table and re-intern loaded types into the consumer context |
```

In section `## 3. Type Context And Semantic Types`, change:

```markdown
Status: `In progress`
```

to:

```markdown
Status: `Complete`
```

Add this Done bullet after the Task 11 migration bullet:

```markdown
- [x] Replaced product artifact inline structural type serialization with a portable artifact-local type table; v23 artifacts store `ProductTypeId` references, reject old format 22 artifacts, and re-intern decoded types into the consumer `TypeContext` during load.
```

Replace the Still to do bullet:

```markdown
- Future artifact-schema work may add a portable serialized type table if product artifacts need to carry compact interned type data.
```

with:

```markdown
- No remaining Type Context And Semantic Types audit items.
```

- [ ] **Step 6: Update ordered roadmap**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update Task 11 reconciliation row remaining-work text from:

```markdown
Future artifact-schema work may add a portable serialized type table, but no remaining compiler-owned mono/MIR/codegen semantic type storage depends on structural `Type`
```

to:

```markdown
No remaining Type Context/Semantic Types work; future artifact evolution should be driven by explicit dependency capability needs, not type identity gaps
```

Update the Task 11 status paragraph from:

```markdown
Product artifacts intentionally remain an explicit structural compatibility boundary until a future portable artifact type-table schema is designed.
```

to:

```markdown
Product artifacts now serialize type payloads through a portable artifact-local type table and re-intern decoded types into the consumer context during artifact loading.
```

Update Work Not Reopened text from:

```markdown
- Full downstream `TypeId` phase-boundary migration for compiler-owned mono/MIR/borrowck/codegen boundaries is complete under Task 11; future artifact-schema work may still introduce a portable serialized type table without changing the current structural product compatibility boundary.
```

to:

```markdown
- Full downstream `TypeId` phase-boundary migration and product artifact type-table serialization are complete for the Type Context/Semantic Types track.
```

- [ ] **Step 7: Update final verification notes in this plan**

Append under `Final Verification Notes`:

```text
cargo test -p rock-lib: PASS with counts recorded during Task 10
cargo fmt --all --check: PASS
git diff --check: PASS
Final artifact schema audit: PASS with allowed matches classified
Final code review: APPROVED
```

Replace the summary line with exact counts from Step 1 output before committing.

- [ ] **Step 8: Run docs checks**

Run:

```bash
git diff -- docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/2026-05-30-portable-product-artifact-type-table.md
git diff --check
```

Expected: docs wording marks Type Context complete without claiming producer `TypeId` serialization; whitespace check passes.

- [ ] **Step 9: Commit docs closure**

Run:

```bash
git add docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md docs/superpowers/plans/2026-05-30-portable-product-artifact-type-table.md
git commit -m "mark type context semantics complete"
```

Expected: commit succeeds.

- [ ] **Step 10: Final clean status**

Run:

```bash
git status --short
```

Expected: no output.

---

## Final Verification Notes

Task 10 final command outputs:

```text
cargo test -p rock-lib: PASS (unit 1381 passed, 0 failed, 1 ignored; integration 277 passed; parser integration 1 passed; doctests 1 passed, 1 ignored)
cargo fmt --all --check: PASS
git diff --check: PASS
Final artifact schema audit: PASS (71 broad matches; allowed matches are in-memory product APIs/remap helpers/tests, SerializedProductArtifact shell construction, in-memory CompilerProducts reconstruction, codec helpers, and tests; no raw Type fields remain in Serialized* DTO definitions)
Final code review: APPROVED
```

---

## Plan Self-Review

- Spec coverage: Tasks 1-2 cover format version and portable type-table codec. Tasks 3-6 cover v23 DTO serialization and removal of inline structural type payloads. Task 7 covers invalid table references and cycles. Task 8 covers consumer `TypeContext` interning. Task 9 covers product/artifact regression suites. Task 10 covers final verification, review, and docs closure.
- Red-flag scan: no incomplete markers, unresolved edge handling, or open-ended implementation steps remain. The plan has one explicitly temporary wrapper in Task 3, and Task 4 removes it before schema completion.
- Type consistency: `ProductTypeId`, `ProductTypeRow`, `ProductTypeTable`, `ProductGenericParamId`, `ProductAssociatedTypeKey`, `SerializedProductArtifact`, `SerializedCompilerProducts`, `ProductTypeEncoder`, and `ProductTypeDecoder` are named consistently across tasks.
