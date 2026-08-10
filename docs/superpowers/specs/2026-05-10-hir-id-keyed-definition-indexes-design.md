# HIR ID-Keyed Definition Indexes Design

## Context

The canonical identity hardening slice closed the supported collect -> lower -> mono gaps where later phases could synthesize or confuse `DefId` values. The remaining identity problem is that in-memory HIR is still primarily addressed by strings:

- `HirProgram` stores functions, structs, enums, and traits in `HashMap<String, ...>` tables.
- `HirProgram` stores impls and externs in vectors, requiring positional or name scans.
- Trait default methods and impl methods live in string-keyed method maps.
- `ResolverTables` already map canonical item paths, import aliases, and export aliases to canonical `DefId` values.
- Product metadata is already persisted mostly by canonical product IDs.

This slice bridges those states by adding canonical ID-keyed indexes to HIR while preserving existing name-keyed tables for compatibility.

## Goals

- Make `HirProgram` expose canonical ID-keyed lookup indexes for top-level definitions.
- Include method/default-method bodies in the ID indexes using their existing `HirFunction.id` values.
- Keep existing string-keyed maps during this slice to avoid a high-risk rewrite of lowering, inference, monomorphization, and codegen.
- Ensure import/export aliases point at the same indexed HIR definition as the canonical name.
- Update the master audit checklist status for this slice against the current commit when implementation lands.

## Non-Goals

- Do not remove the existing `HashMap<String, ...>` HIR tables yet.
- Do not introduce new first-class IDs for fields, variants, associated types, or trait signatures.
- Do not migrate semantic `Type` from name-bearing variants to ID-bearing `Ty` values.
- Do not rewrite monomorphization or codegen symbol naming.
- Do not change resolver semantics beyond validating that HIR indexes agree with existing resolver tables.

## Proposed Model

Add an ID-indexed companion structure in `lib/src/hir/mod.rs`:

```rust
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
```

The string values point back into the existing canonical name-keyed tables. The `usize` values point back into `impls` and `externs`. This avoids duplicating HIR bodies and keeps one owner for mutable state.

Add a method-location enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirMethodLocation {
    TraitDefault { trait_name: String, method_name: String },
    ImplMethod { impl_index: usize, method_name: String },
}
```

`trait_name` is the existing key into `HirProgram.traits`. `impl_index` is the current vector position for the owning impl. A future child-ID slice can replace this with canonical owner/method IDs without changing the basic index concept.

Extend `HirProgram`:

```rust
pub struct HirProgram {
    pub functions: HashMap<String, HirFunction>,
    pub structs: HashMap<String, HirStruct>,
    pub enums: HashMap<String, HirEnum>,
    pub traits: HashMap<String, HirTrait>,
    pub impls: Vec<HirImpl>,
    pub externs: Vec<HirExtern>,
    pub indexes: HirDefinitionIndexes,
}
```

Construct `HirProgram` through a new helper named `HirProgram::from_parts(...)`. The helper builds indexes once from the final HIR containers and is the construction path for `infer::finalize`, `infer::finalize_lenient`, and tests that construct `HirProgram` directly.

## Data Flow

Collection and lowering continue producing the existing HIR containers. Inference finalization consumes `PartialHir`, finalizes types, then calls `HirProgram::from_parts(...)` to attach ID indexes to the resolved HIR program.

Alias resolution remains owned by `ResolverTables`. The HIR indexes do not add alias entries. Instead, alias tests resolve through `resolver.import_aliases` or `resolver.export_aliases` to a canonical `DefId`, then use the HIR ID index to find the canonical HIR definition.

Products and artifact loading can keep their current ID-keyed metadata representation. They may later switch to using `HirProgram.indexes`, but this slice only needs to avoid regressing product identity behavior.

## Validation Rules

Index construction should be deterministic and strict enough to catch identity regressions:

- Every top-level HIR function, struct, enum, and trait with a `DefId` gets one ID index entry.
- Every impl and extern with a `DefId` gets one ID index entry.
- Every trait default method and impl method with a `DefId` gets one method-location entry.
- Duplicate IDs inside the same index should panic in tests or debug assertions rather than silently overwrite entries.
- Alias names should not create additional HIR index entries; aliases must resolve to the canonical definition ID.

Index construction must not special-case the all-zero `DefId`; it can be a valid canonical ID in the current crate. The supported lowered pipeline must resolve extern IDs before `HirProgram::from_parts(...)`, and tests that construct multiple externs directly must assign distinct IDs.

## Testing

Add focused unit tests near HIR or inference construction for pure index behavior:

- `HirProgram::from_parts` indexes functions, structs, enums, traits, impls, externs, trait defaults, and impl methods by `DefId`.
- Duplicate IDs in one index are rejected.
- Method locations point to the expected owner and method name without cloning bodies.

Add pipeline-level tests in the existing collect/lower/infer test area:

- A local import alias resolves through `ResolverTables.import_aliases` to the same `DefId` as the canonical function, and `program.indexes.functions_by_id` points to the canonical function key.
- An export alias resolves through `ResolverTables.export_aliases` to the same `DefId` as the canonical function or type, and the HIR index points to the canonical key.
- A type import/export alias for a struct resolves to the canonical struct `DefId`, and `program.indexes.structs_by_id` points to the canonical struct key.
- Trait default methods and impl methods are present in `methods_by_id` with the expected `HirMethodLocation`.

Run the smallest relevant tests first, then `cargo test -p rock-lib` after implementation.

## Risks

- Preserving string maps means this is not the final architecture. It is an incremental identity spine, not the endpoint.
- Method locations are still partly string/position based because child method identity is not fully designed yet.
- Tests must distinguish canonical names from aliases so the new indexes do not accidentally normalize display names incorrectly.
- Serialization changes to `HirProgram` can affect artifacts or snapshots if any callers serialize whole HIR programs.

## Future Work

- Replace name-keyed HIR maps with ID-keyed ownership tables once consumers have migrated.
- Add canonical child IDs for methods, associated types, fields, and enum variants if the audit confirms they need first-class identity.
- Migrate semantic `Type` to ID-bearing `Ty` values after HIR definitions are reliably ID-addressable.
- Move monomorphization method identity from strings to canonical method or owner-child IDs.
