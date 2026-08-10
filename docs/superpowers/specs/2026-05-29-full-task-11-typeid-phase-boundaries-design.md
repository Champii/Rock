# Full Task 11 TypeId Phase Boundary Migration Design

## Status

Approved design for implementation planning.

## Context

Roadmap Task 11 is partial overall. The first slice attached a `TypeContext` and `HirTypeIds` sidecar to `ResolvedHirProgram`, but downstream mono, MIR, codegen, and product-facing paths still consume structural `Type` values as semantic inputs.

The next task is the full Task 11 migration. The implementation should move compiler-owned phase boundaries to context-owned `TypeId` identity while preserving explicit structural compatibility at parser/lowering construction, diagnostics/display, and current product artifact serialization/loading.

## Goals

- Make `TypeId + TypeContext` the semantic type carrier after HIR finalization.
- Replace structural `Type` semantic storage in mono instance identity, MIR locals/returns/casts/callables, borrowck shape queries, and codegen type/layout/ABI APIs.
- Keep structural `Type` only at explicit compatibility boundaries: parsing/lowering construction, diagnostics/display, and product artifact write/read until a dedicated portable artifact type-table schema is designed.
- Ensure every `TypeId` is used with its owning `TypeContext`; raw IDs must not cross crate-session or artifact boundaries.
- Update roadmap and audit docs only after full verification and final review approval.

## Non-Goals

- Do not design a new product artifact type-table schema in this task.
- Do not persist raw producer `TypeId`s in product artifacts.
- Do not remove structural `Type` from parser/type-lowering construction if doing so only increases churn without improving phase-boundary identity.
- Do not rely on type display strings as semantic keys.

## Architecture

`TypeContext` remains the owner of interned `Ty` nodes and `TypeId` allocation. A `TypeId` is meaningful only with the context that produced it.

After inference/finalization, compiler-owned downstream phases should use `TypeId` as semantic identity. Structural `Type` becomes a compatibility representation reconstructed through the owning context only when a boundary still requires it.

The internal migration is big-bang in scope: HIR finalization, mono, MIR, borrowck, and codegen should be converted as one implementation plan with ordered checkpoints and reviews. Product artifacts remain an explicit structural compatibility boundary during this task, because existing artifact contracts deliberately avoid serializing the resolved-HIR type sidecar.

## Components

### Type Context And Type View

Add a lightweight type view used by downstream phases. It should expose read-only context operations and controlled interning:

- `ty(TypeId) -> &Ty`
- `type_for(TypeId) -> Type` for compatibility edges
- `intern_type(&Type) -> TypeId` for parser/lower/product-loaded structural values
- helpers that delegate shape/fact/layout/projection queries to type services

The view keeps passes from calling raw context internals everywhere and makes ownership explicit in APIs.

### HIR Finalization

Finalized HIR must have TypeId coverage for every downstream-needed type field, including:

- function, extern, trait signature, and impl method params/returns
- generic bound type arguments
- struct fields and enum variant payloads
- impl receiver args, trait args, bounds, and associated type values
- blocks, let statements, expressions, casts, closures, and captures
- method-call trait args
- struct-pattern type args

Missing required TypeIds should become structured finalization errors with the `HirTypeLocation` and owning `DefId`, not debug-only assertions.

### Monomorphization

Mono should use TypeId identity for semantic keys and substitutions:

- `InstanceKey.substitution: Vec<TypeId>`
- `InstanceRecord.substitution: Vec<TypeId>`
- method instance maps keyed by TypeIds rather than `Vec<Type>`
- generic substitution maps from `GenericParamId` to `TypeId`

Substitution rewrites should intern new `Ty` nodes into the same context and return `TypeId`s. Structural substitution helpers may remain only as adapters for explicit compatibility boundaries.

### MIR And Borrowck

MIR should carry `TypeId` for type-bearing runtime forms:

- function return type
- local declarations
- cast targets
- callable trait args and receiver/type-arg metadata
- aggregate/call metadata that currently owns structural types

Borrowck, MIR agreement, and MIR builder shape queries should inspect types through the type view or type services, not by matching structural `Type` fields stored in MIR.

### Codegen

Codegen should accept MIR plus the shared type context. LLVM type lowering, ABI signatures, layout, projections, default values, closure metadata, wrapper keys, and trait/impl matching should use TypeId-aware APIs.

Structural `Type` reconstruction is allowed only at narrow adapters where an existing helper cannot reasonably be migrated in the same step, and those adapters must be classified as compatibility boundaries.

### Product Artifacts

Current product artifacts remain structural compatibility payloads during this task.

Product emission should convert TypeId-backed HIR/body metadata to validated structural `Type` values at the artifact boundary. Product loading should validate/remap structural types as today, then intern them into the consumer context before exposing them to internal mono/MIR/codegen paths.

No producer `TypeId` should be serialized or reused by the consumer. Any future artifact TypeId persistence must be a dedicated schema task with a format bump, portable type table, remapping, and validation plan.

## Data Flow

1. Parsing/lowering/inference may construct structural `Type` values locally.
2. HIR finalization interns finalized structural types into one `TypeContext` and produces TypeId-complete HIR metadata.
3. Mono consumes HIR plus the type context and uses `TypeId` for instance identity and substitution.
4. MIR builder consumes mono output plus the same context and stores TypeIds in MIR type-bearing fields.
5. Borrowck and MIR agreement query type shape through the context/services.
6. Codegen consumes MIR plus type context and lowers layout/ABI through TypeId-aware APIs.
7. Product emission/loading converts at the structural compatibility boundary and re-interns loaded types into the consumer context.

## Error Handling

- Missing required TypeIds are finalization errors with location and owner context.
- Context mismatch should be guarded by API design: TypeId values are not accepted without a context/view parameter.
- Artifact-loaded structural types must be validated and re-interned before internal use.
- Unsupported compatibility adapters must fail loudly instead of silently comparing display strings.

## Compatibility Rules

- Structural `Type` remains valid for parse/type-lowering construction and diagnostics.
- Structural `Type` remains the current product artifact representation.
- Internal semantic identity after finalization is TypeId-based.
- Display strings are metadata for diagnostics/backend symbols only.
- Product tests should assert intentional structural compatibility, not accidental loss of TypeId semantics.

## Testing Strategy

- TypeContext tests for interned substitution, context-owned equality, projection/generic/nominal identity, and structural roundtrip.
- HIR finalization tests proving all downstream-needed type fields have TypeIds.
- Mono tests proving instance keys and substitutions use TypeId identity, including same-name nominal types, projections, generic substitutions, static methods, and artifact-backed generics.
- MIR/borrowck tests proving locals, returns, casts, and callables carry TypeIds and shape queries go through the context.
- Codegen tests for TypeId-driven layout, ABI, default values, projections, closures, enum matches, arrays/slices, casts, and artifact-backed compilation.
- Product tests proving product payloads remain structural, loaded structural types are re-interned into the consumer context, and no raw producer TypeId crosses artifact boundaries.

## Completion Criteria

- `cargo test -p rock-lib` passes.
- `cargo fmt --all --check` passes.
- `git diff --check` passes.
- Architecture audit finds no semantic structural `Type` ownership in mono/MIR/codegen internals except explicit compatibility conversion boundaries.
- Final code review approves the full Task 11 migration.
- Roadmap Task 11 and the master audit checklist are updated only after final verification and review approval.
