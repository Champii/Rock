# Portable Product Artifact Type Table Design

## Goal

Finish the Type Context And Semantic Types audit track by replacing product artifact inline structural type serialization with a portable artifact-local type table. The compiler continues to use context-owned `TypeId` for in-memory semantic identity, while product artifacts serialize portable type-table references instead of raw producer `TypeId`s or repeated inline `Type` payloads.

## Background

Roadmap Task 11 completed compiler-owned `TypeId` phase boundaries after HIR finalization: mono instance identity and substitutions, MIR type-bearing fields, borrowck/agreement queries, and codegen type/layout APIs now use the owning `TypeContext`. The remaining Type Context audit gap is artifact-schema work: product artifacts still intentionally use structural `Type` as the serialized compatibility payload.

`TypeId` is session-local and must not be serialized directly. Product artifacts need a portable type identity layer that can be validated, remapped, and interned into the consumer compiler session when loaded.

## Non-Goals

- Do not serialize producer `TypeId` values.
- Do not preserve backward compatibility with format 22 artifacts; format 22 artifacts must be rejected.
- Do not migrate all in-memory `CompilerProducts` fields to `TypeId` in this task.
- Do not redesign HIR product bodies or remove HIR-shaped artifact bodies.
- Do not add sysroot or stdlib discovery behavior.

## Artifact Format

Bump `PRODUCT_ARTIFACT_FORMAT_VERSION` from `22` to `23`, including the shared sysroot contract.

The v23 artifact payload must be a serialized DTO rather than raw `CompilerProducts`:

```rust
pub struct ProductArtifact {
    pub format_version: u32,
    pub type_table: ProductTypeTable,
    pub products: SerializedCompilerProducts,
}
```

`CompilerProducts::to_artifact_bytes` is responsible for converting in-memory `CompilerProducts` into this DTO. `CompilerProducts::from_artifact_bytes` is responsible for validating and decoding the DTO back into in-memory `CompilerProducts` for existing loaders.

Unsupported format versions must still be rejected before full artifact deserialization.

## Type Table

Add an artifact-local type ID:

```rust
pub struct ProductTypeId(pub u32);
```

Add a type table that stores portable type rows. A row mirrors `crate::types::Type`, but recursive children reference `ProductTypeId`:

```rust
pub enum ProductTypeRow {
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
```

The exact row names can differ, but the schema must preserve every semantic field from `Type` without embedding child `Type` values inline.

`ProductAssociatedTypeKey` must carry a product-local owner plus the existing associated type ID:

```rust
pub struct ProductAssociatedTypeKey {
    pub owner: ProductDefId,
    pub assoc_type_id: crate::ids::AssocTypeId,
}
```

Generic parameters must also use product-local owners:

```rust
pub struct ProductGenericParamId {
    pub owner: ProductDefId,
    pub index: u32,
}
```

Serialized type rows must not contain raw producer `DefId`s. This includes nominal type owners, generic parameter owners, projection trait IDs, and associated type owners.

`Type::TypeVar` must not appear in finalized product artifacts. The encoder must reject it with a clear error if encountered rather than serializing inference-session-local state.

The type table must deduplicate equal rows while encoding. Deduplication is a storage/schema property, not a semantic guarantee across artifacts.

## Serialized Product DTOs

The serialized product DTO must mirror `CompilerProducts`, `ProductMetadata`, `ProductBodies`, and HIR product payloads, but replace every serialized `Type` field with `ProductTypeId`.

Coverage must include:

- `HirFunction.ret_type`.
- `HirParam.ty`.
- `HirClosureCapture.ty`.
- `HirMethodCallTarget.trait_args`.
- `HirField.ty`.
- `HirVariantFields::Positional` payloads.
- `HirFunctionSig.params` and `ret`.
- `HirImpl.receiver_arg_types`.
- `HirImpl.trait_arg_types`.
- `HirImpl.bounds` type arguments.
- `HirAssociatedTypeDef.ty`.
- `HirExtern.params` and `ret`.
- `HirBlock.ty`.
- `HirStmt::Let.ty`.
- `HirExpr.ty`.
- `HirExprKind::Cast` target type.
- `HirPattern::Struct` type arguments.
- Any nested functions, methods, trait defaults, generic impls, and product bodies that contain the same HIR structures.

The in-memory `CompilerProducts` type may remain structural for this task. The artifact boundary is responsible for table encoding and decoding.

## Encoding Flow

`CompilerProducts::to_artifact_bytes` must:

1. Create a type table encoder with an empty row list and row-to-ID map.
2. Walk `CompilerProducts` and convert each `Type` into a `ProductTypeId`.
3. Convert nominal `DefId` ownership inside serialized type rows to `ProductDefId` when the ID refers to product artifact definitions.
4. Build a `ProductArtifact { format_version: 23, type_table, products }` DTO.
5. Serialize the DTO with `bincode`.

If a type contains a `DefId` that cannot be represented in the product identity table, encoding must fail with a clear error rather than serializing an unstable raw producer ID.

## Decoding Flow

`CompilerProducts::from_artifact_bytes` must:

1. Read the artifact format version first.
2. Reject every version other than `23` before full artifact deserialization.
3. Deserialize the v23 artifact DTO.
4. Validate the type table.
5. Decode every `ProductTypeId` reference back into structural `Type` for the in-memory `CompilerProducts` value.
6. Reconstruct nominal `DefId` values from product IDs using the existing product identity mapping conventions.

The existing `crate_artifact/load.rs` path then continues to validate/remap product identities and re-intern loaded types into the consumer `TypeContext`. This task must not bypass that consumer-context interning path.

## Validation And Errors

The decoder must reject:

- Product type IDs outside the table.
- Recursive type-table cycles that cannot produce finite `Type` values.
- Product type rows with unmapped product definition IDs.
- Product associated type keys whose owner cannot be decoded.
- Unsupported artifact format versions.

Cycle detection must report a structured string error through the existing artifact decode `Result<Self, String>` path.

## Testing Strategy

Use TDD for each behavior change.

Required tests:

- Format version contract updates to `23` in `rock_shared` and `rock-lib`.
- Format 22 artifacts are rejected.
- Unsupported format rejection still happens before full artifact deserialization.
- A product artifact with repeated equal types deduplicates them into one type-table row.
- Function metadata and body types roundtrip through the v23 type table.
- Struct field types roundtrip through the v23 type table.
- Enum variant payload types roundtrip through the v23 type table.
- Trait signature/default method and impl receiver/trait/associated types roundtrip through the v23 type table.
- Expression, cast, block, let, lambda capture, method target trait args, and struct pattern type args roundtrip through the v23 type table.
- Loaded product artifact types are re-interned into a supplied consumer `TypeContext` after decode.
- Invalid table references and cycles are rejected.

Verification commands:

```bash
cargo test -p rock-lib product_artifact
cargo test -p rock-lib crate_artifact::load
cargo test -p rock-lib type_context
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

## Documentation Completion

After implementation, review, and full verification:

- Mark `Type Context And Semantic Types` complete in `docs/superpowers/plans/master-audit-checklist.md`.
- Update the ordered roadmap Task 11 row/notes to remove the future artifact-schema caveat or mark it complete.
- Record final verification evidence in the implementation plan.

## Risks

- HIR product payloads contain many nested `Type` locations. Missing one would preserve inline structural serialization in part of the artifact schema.
- Product/generic owner conversion is subtle. The implementation plan must explicitly audit every `DefId`-bearing type component before writing conversion code.
- Type table decoding must avoid recursive expansion loops and report invalid artifact data clearly.
- The v23 schema intentionally breaks artifact compatibility. Existing `.rkca` artifacts must be rebuilt.
