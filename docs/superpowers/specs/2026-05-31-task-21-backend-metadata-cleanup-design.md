# Task 21 Backend Metadata Cleanup Design

## Goal

Remove direct codegen trait/member selection and method-alias reconstruction from the active MIR backend path by giving codegen an explicit backend metadata input for selected method symbols and projection impl facts.

The immediate Task 21 slice is intentionally scoped to the trait/member/backend-symbol metadata that Task 13 left behind. The active compile path should continue emitting executable bodies from MIR, but it should no longer scan HIR impls in codegen to rediscover selected trait implementations or synthesize method aliases from source-level impl names.

## Current State

The active compile path builds a `MonomorphizedProgram`, prunes unreachable instances, builds monomorphized MIR, runs MIR borrow checking and agreement checks, then calls `CodeGen::compile_program_from_mir`.

Executable bodies are MIR-backed, but declaration preparation still reads `MonomorphizedProgram.program` for backend facts. Some of that is acceptable for later metadata slices, but trait/member lookup currently crosses the semantic boundary too far:

- `CodeGen` owns `trait_impls: HashMap<(String, Vec<TypeId>, DefId, Vec<TypeId>), HirImpl>` and registers it by cloning HIR impls.
- `CodeGen::find_trait_impl` matches receiver names, receiver type arguments, trait IDs, and trait arguments to recover selected impls.
- HIR expression codegen compatibility tests still exercise selected-method resolution through `trait_impls` and source-name aliases.
- `register_impl_method_aliases` scans `program.impls_in_order()` and inserts alias strings into `functions` from impl names, trait names, and method names.
- `resolve_mir_callable_symbol` uses `impl_method_backend_symbols` for selected MIR methods, but that table is currently populated while registering instance records and supplemented by HIR impl/member tables.
- Projection resolution for associated types calls `ProjectionProvider::find_projection_impl`, which currently delegates to `CodeGen::find_trait_impl`.

This means codegen can still re-run pieces of trait/member selection instead of consuming explicit selected backend facts.

## Non-Goals

- Do not redesign all backend metadata in one step. Layout metadata, extern declarations, products, object output, symbol export records, and link metadata may continue using existing HIR/mono inputs unless a small local change is required for this slice.
- Do not remove the test-only HIR body codegen path in this task.
- Do not change language behavior, ABI behavior, exported symbols, product artifact formats, or stdlib loading policy.
- Do not reintroduce targetless mono/lowering selection fallback or source-name method lookup as semantic authority.
- Do not claim producer `TypeId` values are serialized; backend metadata stays in-memory and uses the current compilation `TypeContext`.

## Architecture

Introduce a small explicit backend metadata surface for codegen selection facts. It should be built before declaration setup and passed into the MIR codegen preparation path.

The metadata should contain only facts that codegen needs after lowering/mono/MIR have already selected call targets:

- Exact method backend symbols keyed by selected `(impl_id, method_id)`.
- Trait member IDs keyed by `(trait_id, source_member_name)` so selected targets can be validated without scanning `HirTrait` inside codegen.
- Projection impl records or reduced projection facts keyed by receiver shape, trait ID, and concrete trait args, enough for `Type::Projection` normalization during LLVM type lowering.
- Builtin index trait IDs observed from selected targets, if the current builtin array/slice projection fallback still needs this guard.

The first implementation can keep the metadata value close to codegen, for example under `lib/src/codegen/metadata.rs`, and populate it from existing monomorphized/HIR data in a single builder function. The important boundary is that `CodeGen` consumes the prepared metadata and no longer owns HIR-scanning selection logic in its declaration path.

The builder may still read `MonomorphizedProgram.program` for this slice because broader HIR-free metadata extraction is explicitly future work. However, semantic matching should happen before `CodeGen` receives the data. `CodeGen` should perform direct map lookups by canonical IDs and fail when required metadata is absent.

## Components

### Backend Metadata Builder

Add a builder that derives selection metadata from `MonomorphizedProgram` and the executable `MirProgram`:

- Walk instance records to record exact impl-method backend symbols from `InstanceOrigin::ImplMethod { owner: Named(impl_id), method }`.
- Walk trait definitions once to collect trait method/signature member IDs.
- Walk trait impls once to collect projection impl facts and any selected method symbol facts that cannot be derived from instances.
- Walk MIR callable constants and/or retained HIR bodies only to collect selected builtin index trait IDs if the existing projection fallback still requires it.

The builder should produce a plain data object. It should not mutate LLVM declarations or `CodeGen::functions`.

### CodeGen Metadata Consumption

`CodeGen` should store the explicit metadata object or copy its maps during declaration preparation.

Replace direct trait/member registration calls in the MIR path:

- Remove `register_trait_impls` from the active MIR preparation path.
- Remove `register_trait_member_ids` from the active MIR preparation path.
- Remove `register_impl_method_aliases` from the active MIR preparation path.
- Keep instance registration and MIR function declaration authoritative for emitted callable symbols.

`resolve_mir_callable_symbol` should resolve selected methods through exact metadata first:

- `MirCallable::Method { instance: Some(id), .. }` resolves through `instance_symbols_by_id`.
- `MirCallable::Method { impl_id: Some(impl_id), method_id, .. }` resolves through the metadata method-symbol map.
- `MirCallable::Method { method_id, .. }` may resolve through `function_symbols_by_id` only for direct/static method identities that are already represented as function symbols.
- Missing selected method metadata returns `CodegenError`; it must not search HIR impl names or trait names.

### Projection Metadata

LLVM type lowering still needs to normalize `Type::Projection` values in some layout paths. Replace `CodeGen::find_trait_impl` with a metadata lookup that returns either the selected projection impl record or a reduced associated-type resolution fact.

The minimal low-risk form is to keep a `ProjectionImplMetadata` value that owns the needed associated type definitions plus substitution inputs. That avoids rewriting projection normalization in the same slice. The metadata should be keyed by canonical trait ID and interned current-compilation type IDs for receiver/trait args rather than by source type names alone.

Array and slice builtin behavior should remain explicit. If a user-defined impl exists for a fixed array or slice, projection resolution must prefer the metadata impl. Builtin fallback remains available only when selected builtin index metadata says the target is the compiler builtin path.

### Test-Only HIR Codegen Compatibility

The test-only `compile_program` HIR body path may continue to prepare compatibility metadata if existing unit tests require it. The active `compile_program_from_mir` path must use the explicit backend metadata builder and should not call HIR trait/member registration helpers.

Where practical, tests that currently mutate `codegen.trait_impls` directly should move to metadata builder or metadata injection helpers. If that is too large for this slice, keep those tests under the compatibility path and add new MIR/backend-metadata tests for the production behavior.

## Data Flow

The intended active flow is:

```text
Resolved HIR
    -> monomorphization / instance registry / DCE
    -> monomorphized MIR
    -> backend selection metadata builder
    -> CodeGen declaration setup from instances, layouts, externs, and metadata
    -> LLVM bodies from MIR
```

MIR remains the executable boundary. The metadata object is not a second executable representation; it is declaration and symbol support for canonical targets selected earlier in the pipeline.

## Error Handling

Missing metadata should fail explicitly:

- A MIR selected method with `(impl_id, method_id)` but no backend symbol metadata returns `CodegenError` naming the callable and containing MIR function when available.
- A selected trait member whose `(trait_id, name)` maps to a different member ID is rejected rather than falling back to another method with the same name.
- Projection normalization with no matching metadata leaves the projection unresolved or returns an explicit error according to the existing type-lowering helper contract; it must not scan HIR impls in `CodeGen`.
- Builtin index fallback is allowed only through explicit builtin metadata, not by matching method names such as `index` alone.

## Testing Strategy

Use TDD for implementation. Add focused failing tests before code changes for the behavior boundaries that are currently HIR-driven.

Focused tests should cover:

- `compile_program_from_mir` does not call `register_impl_method_aliases` or require source-name impl aliases for selected MIR method calls.
- Selected MIR method calls resolve through exact `(impl_id, method_id)` backend metadata and fail when that metadata is missing.
- A wrong trait member ID is rejected even when a method with the requested source name exists.
- Projection resolution prefers explicit user impl metadata over builtin array/slice fallback.
- Builtin index fallback remains available for builtin-selected array/slice indexing without treating arbitrary same-name `index` methods as builtin.

Regression verification should include existing focused and integration coverage for methods, trait dispatch, operators, indexing, arrays/slices, generics, object-backed dependencies, and product-backed examples.

## Completion Criteria

This Task 21 cleanup slice is complete when:

- The active MIR codegen path consumes an explicit backend selection metadata object for trait/member/projection facts.
- `CodeGen::prepare_mir_program_declarations` no longer scans HIR impls or traits to register trait impls, trait member IDs, or impl method aliases.
- `CodeGen::find_trait_impl` is removed from the active codegen path or replaced by metadata-only lookup.
- Selected MIR callable resolution uses canonical instance/function/impl/method IDs and fails loudly for missing metadata.
- Existing runtime behavior remains stable for method calls, trait calls, custom operators, indexing, arrays/slices, and object-backed methods.
- Focused tests, relevant integration filters, `cargo fmt --all --check`, `cargo test -p rock-lib`, and `git diff --check` pass.

## Risks

- Projection normalization currently shares HIR-shaped impl data. A reduced metadata form may expose hidden dependencies on full `HirImpl` records; use the smallest working metadata first and reduce it later if needed.
- Some legacy HIR codegen unit tests may still rely on direct mutation of `CodeGen::trait_impls`. Keep compatibility isolated from the active MIR path instead of over-expanding this task.
- Fixed-array and builtin-slice indexing has subtle fallback behavior. Tests must cover both user-defined impl preference and builtin fallback guards.
- Backend alias removal can expose places that still call by generated source names. Those should become explicit metadata or test-only compatibility, not production semantic fallback.
