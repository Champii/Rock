# TypeId HIR Boundary Design

## Context

Roadmap Task 11 starts the phase-boundary migration from structural `Type` values to context-owned `TypeId` identity. Task 9 added `Ty`, `TypeContext`, and structural conversion helpers. Task 10 moved semantic type facts, display formatting, projection normalization, and layout-facing shape checks behind explicit services.

This slice should use those foundations without attempting a whole-compiler rewrite. HIR, inference, product artifacts, mono, MIR, and codegen still carry many structural `Type` fields. Replacing all of them at once would create a large, high-risk migration and force artifact-schema decisions that this slice should avoid. The safer first Task 11 slice is to add authoritative `TypeId` identity at the resolved HIR boundary as a sidecar while preserving existing structural fields as compatibility and serialization data.

## Goal

Attach a `TypeContext` and stable HIR type-ID sidecar to `ResolvedHirProgram` after inference finalization, proving selected type-carrying HIR boundaries can be addressed by `TypeId` without changing product artifact serialization or downstream structural consumers yet.

## Non-Goals

- Do not remove structural `Type` fields from HIR in this slice.
- Do not migrate inference unification or substitution storage from `Type` to `TypeId`.
- Do not persist `TypeId` in current product artifacts.
- Do not change the product artifact format version solely for this work.
- Do not move mono, MIR, or codegen to consume `TypeId` directly yet.
- Do not add compiler-owned stdlib loading or other unrelated crate-system behavior.

## Design

### Resolved HIR Type Context

`ResolvedHirProgram` should gain a `TypeContext` field that owns interned canonical `Ty` nodes for selected finalized HIR types. This context is constructed after `infer::finalize` resolves and generalizes types, so the sidecar does not need to model unresolved inference state beyond any remaining explicit `Type::TypeVar` values already present in finalized HIR tests.

The structural `program: HirProgram` field remains unchanged. Existing consumers can continue reading `HirExpr.ty`, `HirParam.ty`, `HirFunction.ret_type`, and other `Type` fields while new code and tests can prove stable `TypeId` identity through the sidecar.

### HIR Type-ID Sidecar

Add a focused HIR type identity module at `lib/src/hir/type_ids.rs` that defines location keys and collection results for type-bearing HIR nodes. The sidecar should be explicit and stable enough for tests, but it should not require every possible HIR `Type` occurrence in the first slice.

Suggested data model:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HirTypeLocation {
    FunctionReturn { function: DefId },
    FunctionParam { function: DefId, index: usize },
    ExternReturn { extern_id: DefId },
    ExternParam { extern_id: DefId, index: usize },
    StructField { owner: DefId, field: FieldId },
    EnumVariantNamedField { owner: DefId, variant: VariantId, field: FieldId },
    EnumVariantPositionalField { owner: DefId, variant: VariantId, index: usize },
    TraitSignatureReturn { trait_id: DefId, signature: DefId },
    TraitSignatureParam { trait_id: DefId, signature: DefId, index: usize },
    ImplReceiverArg { impl_id: DefId, index: usize },
    ImplTraitArg { impl_id: DefId, index: usize },
    AssociatedTypeDef { impl_id: DefId, assoc_type: AssocTypeId },
    Block { owner: DefId, path: Vec<usize> },
    LetStmt { owner: DefId, path: Vec<usize>, name: String },
    Expr { owner: DefId, path: Vec<usize> },
    ClosureCapture { owner: DefId, path: Vec<usize>, name: String },
}

#[derive(Debug, Clone, Default)]
pub struct HirTypeIds {
    ids: HashMap<HirTypeLocation, TypeId>,
}
```

The body path should be a deterministic pre-order path within the owning function, trait-default method, or impl method body. A path segment is the child index selected while walking the HIR tree; the root block uses an empty path. Top-level signature locations use existing semantic IDs (`DefId`, `FieldId`, `VariantId`, `AssocTypeId`) where available, and body paths are only for locations that do not already have a stable semantic ID.

### Collection Boundary

Add a collector that walks a finalized `HirProgram`, interns each selected structural `Type` into a `TypeContext`, and records the returned `TypeId` at the corresponding `HirTypeLocation`.

Collection should include these stable HIR boundaries:

- Function parameter types and return types.
- Extern parameter types and return types.
- Struct field types.
- Enum named field types keyed by `FieldId` and enum positional field types keyed by position.
- Trait signature parameter and return types.
- Impl receiver argument types and trait argument types.
- Impl associated type definition types.
- Function, trait-default, and impl-method body block types.
- Let statement declared types.
- Expression types.
- Closure capture types.

The collector should recurse through HIR bodies using the existing HIR tree shape and should intern the exact finalized structural `Type` values already stored on those nodes. It should not normalize projections or alter types during collection; `TypeContext::intern_type` is the only conversion step.

### Access Pattern

`ResolvedHirProgram` should expose simple read APIs for sidecar usage:

```rust
impl ResolvedHirProgram {
    pub fn type_id_at(&self, location: &HirTypeLocation) -> Option<TypeId>;
    pub fn type_at(&self, id: TypeId) -> Type;
}
```

The accessors make the ownership boundary explicit: `TypeId` is meaningful only with the `TypeContext` stored on the same `ResolvedHirProgram`. This avoids treating `TypeId` as a globally portable value or persisting it into products.

### Product Artifact Compatibility

`CompilerProducts` should continue serializing structural HIR and structural `Type` values. The new `TypeContext` and `HirTypeIds` sidecar should not be copied into `CompilerProducts`, `ProductMetadata`, `ProductBodies`, `ArtifactCrateInterface`, or `ArtifactCrossCrateHir`.

Add tests that emit/read product artifacts and verify:

- Structural type serialization still roundtrips.
- No product-facing struct gains a serialized `TypeId` field in this slice.
- Product artifact format version does not change because of the sidecar.

### Roadmap And Audit Updates

After implementation proof, update the ordered roadmap and master audit checklist to mark Task 11 complete for the HIR sidecar slice only. The docs should explicitly say that structural `Type` fields remain in HIR as compatibility data and that later work must migrate direct consumers in mono, MIR, codegen, and any future artifact schema.

## Testing Strategy

Use TDD for each implementation task.

Focused tests should prove:

- Equal finalized HIR types share one `TypeId` in the resolved program context.
- Different nominal `DefId`s and projection identities intern to different `TypeId`s.
- A function return/parameter `TypeId` roundtrips through `ResolvedHirProgram::type_at` to the original structural `Type`.
- Body expression and let-statement types are collected deterministically.
- Extern, struct field, enum field, trait signature, impl receiver/trait arg, and associated type definition locations are collected.
- Product artifact emission and loading continue to use structural `Type` without serialized `TypeId` sidecars.

Regression tests should include:

- Existing `type_context` tests.
- Existing type identity tests.
- Inference substitution/generalization tests.
- Product artifact roundtrip/remapping tests.
- Focused integration tests for associated type projections and indexing behavior.
- Full `cargo test -p rock-lib`.

## Acceptance Criteria

- `ResolvedHirProgram` owns a `TypeContext` and a HIR type-ID sidecar built after inference finalization.
- Selected HIR type-carrying locations can be queried for `TypeId` through explicit APIs.
- Sidecar `TypeId`s roundtrip through the resolved program context to the existing structural `Type` values.
- Product artifacts remain structural and do not persist `TypeId` sidecars.
- Roadmap/audit docs describe Task 11 as complete only for the HIR boundary slice and preserve remaining downstream migration work.
- Focused tests and the full documented `rock-lib` suite pass.
