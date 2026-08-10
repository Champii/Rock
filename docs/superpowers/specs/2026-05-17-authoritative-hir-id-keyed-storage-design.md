# Authoritative HIR ID-Keyed Storage Design

## Context

The architecture roadmap identifies the first focused slice as `Authoritative HIR ID-Keyed Storage`, covering the first three ordered tasks enough to make HIR ownership ID-first:

- Make current-crate ID allocation single-source.
- Make HIR ID-keyed storage authoritative.
- Convert the first practical HIR semantic references to IDs.

The compiler already has important groundwork:

- `lib/src/ids.rs` defines `DefId`, child IDs, and typed allocation helpers.
- `lib/src/collect/item_index.rs` allocates root-module and item IDs through `IndexingIds`.
- `lib/src/hir/mod.rs` has `HirDefinitionIndexes`, ID-keyed accessors, and child-location tables.
- `lib/src/types/mod.rs` now stores semantic type identity with canonical IDs for nominal types, generics, type vars, trait bounds, and projections.

The remaining problem is ownership. `HirProgram` still stores core definitions in `HashMap<String, ...>` tables, then derives ID indexes that point back to those names. Several HIR references still carry only source strings, so later phases must reconstruct semantic targets from names even when canonical IDs are already known.

## Goals

- Make collection/index allocation the only authority for supported current-crate top-level, method, and child definition IDs.
- Remove supported-path provisional `CrateId(u32::MAX)` and post-declaration ID repair behavior before body lowering sees declarations.
- Change `HirProgram` so canonical `DefId`s own functions, structs, enums, traits, impls, and externs.
- Keep source/display names as metadata on definitions or in name tables, not as semantic ownership keys.
- Preserve compatibility accessors for unmigrated consumers, but make those accessors views over ID-owned storage.
- Migrate high-value HIR semantic references that already have canonical targets: method locations, struct literal targets, enum variant expressions/patterns, and direct top-level function/extern references where lowering resolves an ID.
- Keep existing user-visible diagnostics source-name based.

## Non-Goals

- Do not implement the full ID-backed alias interface plan. Import/export/prelude aliases may still be represented by existing resolver structures until the next focused plan.
- Do not migrate every local variable binding or capture to a new local-ID system in this slice.
- Do not introduce `Ty`, type interning, or an authoritative `TypeId` context.
- Do not move trait/method selection into a shared service yet.
- Do not move codegen to MIR or replace monomorphized HIR bodies in this slice.
- Do not preserve deprecated storage paths after their consumers are migrated, unless they are required for artifact decoding or a direct compatibility accessor.

## Proposed Architecture

### 1. ID Allocation Authority

`IndexingIds` and collection-owned declaration builders must assign all current-crate semantic IDs before lowering constructs final HIR declarations. The supported path must not create semantic IDs from lower, infer, mono, codegen, product emission, or artifact writing.

The implementation must audit all supported current-crate ID sources and classify each one as:

- `Authoritative`: allocated by collection/indexing for a real current-crate declaration.
- `Metadata`: a child ID such as field, variant, associated type, or method identity derived from the owning declaration.
- `Test fixture`: explicit IDs created inside tests.
- `External/product remap`: IDs loaded from artifacts and remapped into the consumer session.
- `Invalid fixture`: deliberate impossible IDs used in negative tests only.

Any remaining supported-path `CrateId(u32::MAX)` owner must become an error or be replaced by an already-known owner ID before final HIR construction. Tests may keep explicit impossible IDs only when the test name and assertions make the invalid fixture intentional.

### 2. Authoritative HIR Storage

`HirProgram` must store owned top-level definitions by canonical ID. A concrete shape may use direct fields or a nested storage struct, but semantic ownership must be ID-keyed:

```rust
pub struct HirProgram {
    pub functions: HashMap<DefId, HirFunction>,
    pub structs: HashMap<DefId, HirStruct>,
    pub enums: HashMap<DefId, HirEnum>,
    pub traits: HashMap<DefId, HirTrait>,
    pub impls: HashMap<DefId, HirImpl>,
    pub externs: HashMap<DefId, HirExtern>,
    pub names: HirNameTables,
    pub indexes: HirDefinitionIndexes,
}
```

`HirNameTables` must provide source/display lookup without owning semantic bodies:

```rust
pub struct HirNameTables {
    pub functions_by_name: HashMap<String, DefId>,
    pub structs_by_name: HashMap<String, DefId>,
    pub enums_by_name: HashMap<String, DefId>,
    pub traits_by_name: HashMap<String, DefId>,
    pub externs_by_name: HashMap<String, DefId>,
}
```

The exact field names can be adjusted to minimize churn, but the invariant is fixed: there is one owned HIR body per canonical ID, and any name lookup returns the ID of that body.

`HirDefinitionIndexes` may either be collapsed into the ID-owned maps or kept as derived child/location tables. It must no longer be the only bridge from ID to name-owned HIR storage.

### 3. Compatibility Accessors

Unmigrated code may keep source-name lookup through methods such as `function_by_name`, `struct_by_name`, and `trait_by_name`. Those methods must resolve names through `HirNameTables`, then return the ID-owned definition.

Compatibility accessors are allowed during this slice when they prevent a high-risk all-at-once rewrite. They must not introduce a second owner for the same function, type, trait, impl, extern, method, or child definition.

### 4. First HIR Reference Migrations

This slice must migrate references that are already naturally ID-backed or have a clear resolved target during lowering:

- `HirMethodLocation` must identify trait defaults and impl methods by owner ID plus method ID or by direct method `DefId`, not by trait name or impl vector position.
- Struct literal and struct pattern HIR must carry the resolved struct `DefId` and field IDs while preserving source field names for diagnostics.
- Enum variant expressions and enum pattern HIR must carry the enum `DefId` and variant ID while preserving source enum/variant names for diagnostics.
- Direct top-level function and extern references must carry a resolved target ID when lowering can distinguish them from locals.

Local variables, closure captures, and pattern bindings may remain source-name based for this slice unless the implementation already has a narrow local-reference mechanism. They must not block ID-owned HIR storage.

## Data Flow

1. Parser and source/module handling produce AST exactly as today.
2. Collection indexes modules and declarations, assigning canonical current-crate IDs for declarations and child identities needed by this slice.
3. Header/declaration builders receive IDs from collection rather than allocating or repairing them after the fact.
4. Lowering builds HIR definitions with canonical IDs and records source names as metadata.
5. Inference finalization constructs `HirProgram` through ID-owned constructors.
6. Mono, MIR, codegen, products, and artifact writing migrate to ID iterators and compatibility accessors over ID-owned storage.
7. Existing resolver alias tables remain the source of alias-to-ID truth until the next alias-interface plan replaces their persistent shape.

## Invariants

- A supported current-crate HIR definition must have a canonical `DefId` before it enters `HirProgram`.
- No supported current-crate semantic owner may be stored under only a source string.
- Two different HIR owners may not share one `DefId` within the same table.
- One semantic owner may have multiple source aliases, but all aliases must map to the same canonical ID.
- Names and backend symbols remain display/output metadata, not semantic identity.
- ID-owned storage must round-trip through product artifact emission/loading without leaking producer-local crate numbers.

## Error Handling

Missing canonical identity in the supported collect -> lower -> infer path must be reported as a compiler diagnostic or hard internal invariant failure before downstream phases run. It must not be repaired by inserting a fresh ID in a later phase.

Duplicate IDs in HIR construction must fail loudly. In tests, a panic with a specific duplicate-ID message is acceptable. In production paths, prefer structured diagnostics when the duplicate is caused by source declarations or artifact input.

## Testing Strategy

Add focused tests before implementation:

- Collection/index tests proving current-crate top-level declarations, impls, trait defaults, impl methods, fields, variants, and associated types receive canonical IDs before lowering.
- Negative tests proving supported current-crate lowering does not produce `CrateId(u32::MAX)` owners in declarations, generic params, trait bounds, or HIR reference targets.
- HIR construction tests proving ID-owned storage rejects duplicates and name aliases resolve to one canonical body.
- Same-name module tests for functions, structs, enums, traits, methods, fields, and variants.
- HIR reference tests for struct literals, enum expressions, enum patterns, and direct top-level calls.
- Artifact/product tests proving ID-owned HIR metadata emits and reloads with the expected remapped IDs.
- Focused mono/MIR/codegen smoke tests proving migrated consumers no longer require string-owned HIR maps.

Verification must start with the smallest relevant focused tests, then run `cargo fmt --all --check`, `git diff --check`, and `cargo test -p rock-lib`.

## Risks

- `HirProgram` field type changes will touch many consumers. The plan must sequence migrations by phase and keep temporary accessors narrow.
- Some string lookups are still legitimate local-scope or diagnostic operations. The implementation must not force local IDs into this slice just to remove every string from HIR.
- Existing artifact serialization may depend on skipped or defaulted HIR fields. Any storage-shape changes must preserve product artifact behavior or explicitly migrate serialization.
- Trait default methods currently have ownership and injection behavior that overlaps with selection-service future work. This slice must make their identity authoritative without changing selection semantics.

## Completion Criteria

This slice is complete when:

- Supported current-crate semantic IDs are allocated by collection/indexing before HIR body lowering.
- `HirProgram` owns core definitions by canonical ID, not by source string.
- Source-name access to HIR is a compatibility view over ID-owned storage.
- High-value HIR semantic references listed above carry canonical IDs where lowering already knows them.
- Existing integration tests and new identity/reference tests pass through `cargo test -p rock-lib`.
- `docs/superpowers/plans/master-audit-checklist.md` reflects the completed tasks and remaining alias/type/selection work accurately.
