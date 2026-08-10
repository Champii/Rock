# Clean-Slate Compiler Architecture Audit

This document captures the whole-codebase audit performed on the HIR-codegen debt removal PR worktree.

Source of truth for this audit:

- Worktree: `/root/new_lang2/.worktrees/hir-codegen-debt-removal`
- Branch: `hir-codegen-debt-removal`
- Existing PR context: the old direct HIR-to-LLVM backend was removed, but several old/transitional systems remain.

The purpose of this document is to preserve the detailed analysis so a future session can turn it into a formal spec and implementation plan without rediscovering the same codebase facts.

## Executive Summary

Final status as of 2026-08-02: all 13 recommended refactor steps and both
previously unassigned production findings are complete. The historical findings
below describe the baseline that motivated the work; each implemented step's
status and the final closure swipe are authoritative for the current tree.

The main problem is not one isolated leftover. It is a recurring architectural pattern:

- semantic identity is partially ID-based and partially string/name-based;
- backend facts are duplicated across MIR metadata, backend contracts, codegen maps, and product link metadata;
- HIR still carries backend-oriented names;
- downstream phases still recover meaning by display aliases or compatibility names;
- tests can bypass the real backend contract with unchecked fixture paths;
- some compiler-recognized language semantics still use hardcoded names such as `Sized`, `Array`, or `stdlib::sized::Sized`.

The clean-slate target should be a single clear pipeline:

```text
parse -> collect/resolve -> lower -> infer/select -> monomorphize -> MIR build -> MIR checks -> codegen -> products/artifacts
```

Each phase should own one set of facts and pass typed, ID-keyed data to the next phase. Names should exist at the boundaries where names are real input/output: source parsing, resolver imports/exports, diagnostics/debug output, and artifact display metadata. Names should not drive semantic lookup, backend dispatch, layout, ABI, drop glue, or cross-crate linking.

## High-Level Clean-Slate Principles

These principles should guide the spec and all implementation tasks.

1. Semantic decisions use canonical IDs.
   - Use `DefId`, `TypeId`, `InstanceId`, `AssocTypeId`, `FieldId`, `VariantId`, and semantic target structs.
   - Do not use display strings, qualified names, short names, or aliases to recover semantics after collection/resolution.

2. Each phase owns its output contract.
   - Collection owns name/import/export resolution.
   - Lowering owns AST-to-HIR conversion.
   - Inference/selection owns type constraints and selected call targets.
   - Monomorphization owns concrete instances and instance symbols.
   - MIR builder owns MIR and the backend contract.
   - Codegen consumes MIR and backend contract only.
   - Product/artifact code owns serialized interface/body/link data.

3. No dual backend authority.
   - There should be exactly one MIR backend contract.
   - There should not be a legacy sidecar feeding the real contract.
   - Codegen should not read metadata that is not part of the backend contract.

4. Strings are allowed only for source/display/boundary concerns.
   - Source names, module names, diagnostic labels, debug names, artifact display names, exported alias names, and linker symbols are strings.
   - Strings are not acceptable as method selection keys, type owner identity, nominal layout identity, projection identity, callable identity, drop-glue identity, or backend ABI identity.

5. Compiler-recognized language items must be explicit IDs.
   - If the compiler recognizes `Drop`, `Sized`, `Try`, `Index`, or any similar trait specially, that trait must be represented as a language item ID.
   - Do not identify such traits through names like `"Sized"` or `"stdlib::sized::Sized"`.

6. Artifact link records are authoritative for backend symbols.
   - Identity tables identify definitions and display/export names.
   - Link records identify object-backed symbols.
   - Missing link symbols for object-backed concrete bodies should be load-time errors, not recovered from names.

7. Diagnostic recovery must not leak into executable compiler paths.
   - `Type::Error` can exist inside diagnostic recovery while gathering errors.
   - It should not reach monomorphization, MIR, borrow checking, products, or codegen in accepted programs.

8. Test helpers should model the production contract.
   - Tests should build valid `MirBackendContract` fixtures.
   - Tests should not pass by syncing old name-keyed side tables or using unchecked MIR codegen paths except for explicitly narrow unit tests.

## Current Pipeline Observed In The PR Worktree

The current driver path in `lib/src/lib.rs` is roughly:

1. Load source graph.
2. Parse and macro-expand.
3. Collect declarations.
   - `collect::collect_with_source_graph(...)` at `lib/src/lib.rs:185-197`.
4. Lower from declarations.
   - `lower::program::lower_from_declarations(...)` at `lib/src/lib.rs:202-213`.
5. Finalize inference.
   - `infer::finalize(...)` at `lib/src/lib.rs:215-221`.
6. Attach language items for `Drop`.
   - `hir.program.language_items = ...` at `lib/src/lib.rs:226-236`.
7. Optionally create products from resolved HIR.
   - `CompilerProducts::from_resolved_hir(...)` at `lib/src/lib.rs:243-263`.
8. Monomorphize.
   - `mono::monomorphize_with_crates(...)` at `lib/src/lib.rs:267-270`.
9. Prune DCE instance bodies.
   - `dce::prune_unreachable_instances(...)` at `lib/src/lib.rs:269-271`.
10. Build MIR.
    - `MirBuilder::build_monomorphized_with_instance_bodies(...)` at `lib/src/lib.rs:273-276`.
11. Run borrow checking.
    - `BorrowChecker::run(...)` at `lib/src/lib.rs:280-282`.
12. Run MIR/codegen agreement check.
    - `mir::agreement::check_mir_runtime_agreement(...)` at `lib/src/lib.rs:283-291`.
13. Codegen from MIR.
    - `codegen.compile_program_from_mir(...)` at `lib/src/lib.rs:301-309`.
14. Attach product link records from MIR backend contract artifact exports.
    - `attach_product_link_records(...)` at `lib/src/lib.rs:310-315` and `lib/src/lib.rs:694-734`.

This is a good high-level shape, but many internals still violate the clean ownership model.

## P0 Production-Active Old/Transitional Systems

### 1. `MirBackendMetadata` Still Coexists With `MirBackendContract`

This is the clearest remaining old backend sidecar.

Current facts:

- `MirProgram` stores both `backend_metadata` and `backend_contract`.
  - `lib/src/mir/mod.rs:31-37`
- `MirBackendMetadata` still owns a broad set of backend facts.
  - `lib/src/mir/mod.rs:123-135`
- Its fields include:
  - `externs`
  - `instance_declarations`
  - `structs`
  - `enums`
  - `trait_members`
  - `projection_impls`
  - `projection_resolutions`
  - `drop_glue`
  - `builtin_index_trait_ids`
  - `product_link_candidates`
- MIR builder still creates `backend_metadata` first.
  - `lib/src/mir/builder/mod.rs:384-389`
- MIR builder then converts metadata to the contract through `from_legacy_metadata_with_type_context`.
  - `lib/src/mir/builder/mod.rs:390-399`
- `MirProgram` is then constructed with both fields.
  - `lib/src/mir/builder/mod.rs:401-406`
- The adapter is explicitly named legacy.
  - `lib/src/mir/backend_contract.rs:161-381`
- The adapter includes legacy helper functions:
  - `from_legacy_metadata`
  - `from_legacy_metadata_with_type_context`
  - `from_legacy_metadata_inner`
  - `legacy_generic_param_ids`
  - `legacy_instance_signature`
  - `legacy_receiver_pass_mode`
- The adapter has an explicit comment that metadata has no runtime-helper sidecar yet.
  - `lib/src/mir/backend_contract.rs:328-329`

Why this is production-active debt:

- It is not dead test code.
- It is the normal production MIR builder path.
- The backend contract is not built directly from authoritative MIR/mono facts.
- Agreement and codegen still read data derived from or directly in metadata.

Clean target:

- Delete `MirBackendMetadata` from `MirProgram`.
- Delete `MirBackendMetadata` if no remaining non-production owner exists.
- Delete all `from_legacy_metadata*` functions.
- MIR builder should directly create `MirBackendContract`.
- Any display names needed for diagnostics should live in a separate diagnostics/display structure, not in backend metadata.
- The contract must own:
  - callable declarations;
  - function body mapping;
  - nominal layouts keyed by `DefId`;
  - projection outputs keyed by `MirProjectionKey`;
  - drop glue keyed by `TypeId` to `MirCallableKey`;
  - runtime helper requirements;
  - artifact export candidates.

### 2. Codegen Still Receives `MirBackendMetadata` For Nominal Layout Names

Current facts:

- `CodeGen::register_mir_nominal_layouts` takes both contract and metadata.
  - `lib/src/codegen/mod.rs:159-163`
- It immediately calls `register_mir_nominal_layout_names(metadata)`.
  - `lib/src/codegen/mod.rs:164`
- `register_mir_nominal_layout_names` populates name-keyed maps from metadata layouts.
  - `lib/src/codegen/mod.rs:196-237`
- The name-keyed maps include:
  - `struct_info`
  - `struct_generic_param_ids`
  - `struct_generic_owners`
  - `struct_names_by_id`
  - `enum_info`
  - `enum_generic_param_ids`
  - `enum_generic_owners`
  - `enum_names_by_id`
- The production declaration preparation path passes both contract and metadata.
  - `lib/src/codegen/mod.rs:460-479`
  - `lib/src/codegen/mod.rs:473`

Why this is production-active debt:

- Codegen still consumes the old sidecar.
- Layout lowering is partly ID-keyed via `MirBackendContract`, but names are still populated from metadata.
- This preserves old string side tables in the active backend path.

Clean target:

- Codegen layout registration should consume `MirBackendContract` only.
- If codegen needs display names, those names should be diagnostics-only and keyed by `DefId`.
- Remove or isolate name-keyed layout maps.
- Prefer:
  - `struct_layouts_by_id: HashMap<DefId, ...>`
  - `enum_layouts_by_id: HashMap<DefId, ...>`
  - `struct_generic_param_ids_by_id: HashMap<DefId, ...>`
  - `enum_generic_param_ids_by_id: HashMap<DefId, ...>`
- Remove production use of:
  - `struct_info: HashMap<String, ...>`
  - `enum_info: HashMap<String, ...>`
  - `struct_generic_owners: HashMap<String, DefId>`
  - `enum_generic_owners: HashMap<String, DefId>`

### 3. MIR Agreement Still Treats Legacy Metadata As Contract Input

Current facts:

- The agreement checker calls metadata validation directly.
  - `lib/src/mir/agreement.rs:59-61`
- `validate_backend_metadata_type_ids` walks `program.backend_metadata`.
  - `lib/src/mir/agreement.rs:477-515`
- `BackendContract::new` derives projection trait information from `program.backend_metadata`.
  - `lib/src/mir/agreement.rs:734-779`
- It reads:
  - `program.backend_metadata.trait_members`
  - `program.backend_metadata.projection_impls`
  - `program.backend_metadata.builtin_index_trait_ids`
  - `program.backend_metadata.projection_resolutions`

Why this is production-active debt:

- Agreement checking is part of accepted compilation before codegen.
- Metadata is not merely diagnostic; it influences the agreement model.
- A stale or divergent metadata sidecar can still affect validation.

Clean target:

- Agreement checks should validate `MirProgram.functions`, `MirProgram.type_context`, and `MirBackendContract` only.
- Projection trait knowledge should come from contract data, not metadata sidecars.
- If additional projection/runtime facts are needed, add explicit fields to `MirBackendContract` rather than reading metadata.

### 4. Unresolved Callable Variants In MIR (Closed)

Status: Complete as of 2026-08-02. `MirCallable` can now contain only
`Resolved(MirCallableKey)`. The direct `Function`, `Extern`, `Instance`,
`Closure`, `Intrinsic`, and `RuntimeHelper` variants are deleted. DCE, borrow
checking, agreement, backend-contract validation, and codegen consume only the
canonical key wrapper, and the obsolete `UnresolvedCallableOperand` contract
error path is gone.

Historical facts that motivated the cleanup:

- `MirCallable` has both resolved and unresolved/pre-contract variants.
  - `lib/src/mir/identity.rs:787-805`
- Variants include:
  - `Resolved(MirCallableKey)`
  - `Function(DefId)`
  - `Extern(DefId)`
  - `Instance(InstanceId)`
  - `Method { ... }`
  - `Closure(MirClosureId)`
  - `Intrinsic(MirIntrinsicId)`
  - `RuntimeHelper(MirRuntimeHelper)`
- MIR builder can still emit `MirCallable::Method`.
  - `lib/src/mir/builder/mod.rs:2076-2184`
  - `lib/src/mir/builder/mod.rs:2294-2336`
- Codegen rejects non-`Resolved` in symbol lookup.
  - `lib/src/codegen/mod.rs:333-363`
- Backend contract validation rejects unresolved callable operands.
  - `lib/src/mir/backend_contract.rs:689-723`

Why this is production-active debt:

- It is good that checked backend code rejects unresolved operands.
- But the MIR data model still allows old unresolved forms.
- MIR construction still has paths that build those forms.
- A professional clean-slate MIR should represent the post-resolution contract directly.

Clean target:

- Split callable concepts into two stages:
  - builder-local/pre-MIR unresolved callable candidates;
  - final MIR callable operands as `MirCallableKey` only.
- Final `MirProgram` should not contain unresolved callable variants.
- If intrinsics/runtime helpers are callable, they should have `MirCallableKey::Intrinsic` or `MirCallableKey::RuntimeHelper` entries in the contract.
- Delete or make private/pre-MIR-only variants that cannot appear in final MIR.

### 5. Collect/Lower Still Maintain Parallel String-Keyed And ID-Keyed Item Systems

Current facts:

- `Declarations` explicitly documents that `ItemIndex` is the migration target but current lowering still uses legacy string-keyed declaration maps.
  - `lib/src/collect/mod.rs:35-60`
- `Declarations` carries both:
  - `indexing_ids`
  - `item_index`
  - string maps for `structs`, `enums`, `traits`, `functions`, `function_sigs`, `methods`, `function_type_vars`
- `LowerItems` stores semantic items by `String` and also stores by-ID indexes.
  - `lib/src/lower/items.rs:6-23`
- `LowerItems::rebuild_id_indexes` rebuilds ID maps by cloning values from string maps.
  - `lib/src/lower/items.rs:105-130`
- `Lowerer::def_id_for_name` resolves candidate strings to IDs and panics when missing.
  - `lib/src/lower/mod.rs:284-294`
- `PartialHir` is still name-keyed.
  - `lib/src/infer/mod.rs:76-90`

Why this is production-active debt:

- Lowering is still not consuming a purely resolved declaration model.
- The phase boundary between collect and lower remains transitional.
- IDs can be reconstructed by names after collection, which breaks the clean ownership rule.

Clean target:

- `collect` should output ID-keyed declaration records.
- `lower` should receive declarations keyed by `DefId` and module scopes keyed by `ModuleId`/`DefId`.
- Name maps remain in resolver tables only.
- `PartialHir` should become ID-keyed or be replaced by a typed HIR builder state whose semantic maps are ID-keyed.
- Eliminate `def_id_for_name` from production lowering.
- Keep names only as fields on declarations/functions for display/debug/export, not as keys for semantic lookup.

### 6. Method Selection Still Uses Legacy/String Lookup Paths

Current facts:

- `selection/legacy_lookup.rs` maps types to method lookup names.
  - `lib/src/selection/legacy_lookup.rs:4-91`
- It hardcodes names for primitives and structural types:
  - `"Array"`
  - `"I64"`
  - `"I32"`
  - `"Bool"`
  - `"Str"`
  - pointer/slice/reference string shapes
- Lower resolution builds canonical and short names for method lookup.
  - `lib/src/lower/resolution.rs:564-624`
- `SelectionService` still stores traits and methods through string-keyed maps.
  - `lib/src/selection/service.rs:18-23`
- Selection still maps primitive receiver types to string owner kinds.
  - `lib/src/selection/service.rs:1530-1590`
- Mono duplicates method type-name lookup.
  - `lib/src/mono/mod.rs:627-713`
  - `lib/src/mono/mod.rs:747-884`
- MIR builder still uses display aliases/type names for method lookup.
  - `lib/src/mir/builder/mod.rs:2580-2626`

Why this is production-active debt:

- Method dispatch semantics are still recoverable through display/type names after selection.
- Several phases duplicate method receiver matching logic.
- This undermines canonical ID ownership and makes cross-crate collisions/aliases risky.

Clean target:

- Selection must output a complete `SelectionAuthority` containing:
  - selected origin;
  - `impl_id` if applicable;
  - `trait_id` if applicable;
  - `method_id`;
  - selected `InstanceId` or enough canonical data for mono to produce one;
  - receiver adjustment;
  - trait args as type IDs or fully resolved types;
  - associated type outputs;
  - builtin/core-operation identity if the selection is compiler-owned.
- Mono and MIR should consume selected authority, not rerun lookup.
- `selection/legacy_lookup.rs` should disappear or be reduced to test-only compatibility checks during migration.

### 7. HIR Display Aliases Leak Into Production Semantics

Current facts:

- `HirNameTables` says it is for display/serialization aliases.
  - `lib/src/hir/mod.rs:80-91`
- It explicitly says production compiler phases should use ID-keyed maps, resolved targets, or `HirDefinitionIndexes`.
- Mono registers aliases from HIR display aliases.
  - `lib/src/mono/process.rs:44-52`
- MIR builder copies struct/enum display aliases into layout metadata.
  - `lib/src/mir/builder/mod.rs:707-748`
- MIR builder uses display aliases for method lookup.
  - `lib/src/mir/builder/mod.rs:2592-2626`
- DCE has display-alias lookup tests, though those appear test-only.

Why this is production-active debt:

- The HIR docs and actual use disagree.
- Display aliases are still involved in semantic downstream paths.
- This allows alias names to affect mono/MIR/codegen behavior.

Clean target:

- `HirNameTables` remains only for diagnostics, debug output, artifact compatibility, and external display.
- Add audit tests that forbid production code in mono/MIR/codegen from calling:
  - `function_display_aliases`
  - `struct_display_aliases`
  - `enum_display_aliases`
  - `nominal_display_aliases`
- If display names are needed, pass an explicit diagnostics map keyed by `DefId`.

### 8. HIR Owns Backend-Oriented Names Through `qualified_name`

Current facts:

- `HirFunction.qualified_name` comment says it stores `"TypeName_methodName"` for precise type var lookup.
  - `lib/src/hir/mod.rs:1310-1323`
- Collect has `format_impl_backend_name`.
  - `lib/src/collect/headers.rs:300-324`
- Collect writes this value into `func.qualified_name` for impl methods.
  - `lib/src/collect/headers.rs:1051-1067`
- Lower has a second similar `format_impl_backend_name`.
  - `lib/src/lower/collect/traits.rs:159-195`
- Mono uses `qualified_name` to find impl/method origin.
  - `lib/src/mono/mod.rs:175-205`
- Mono uses `qualified_name` or a formatted fallback as backend symbol input.
  - `lib/src/mono/mod.rs:310-323`
- Mono method specialization stores `qualified_name` as instance source names.
  - `lib/src/mono/methods.rs:80-111`
  - `lib/src/mono/methods.rs:533-589`
- Artifact load constructs multiple callable fallback names from `qualified_name`, method name, owner name, and short owner name.
  - `lib/src/crate_artifact/load.rs:322-364`
- Artifact interface construction inserts a canonical name into `function.qualified_name` if missing.
  - `lib/src/crate_artifact/load.rs:3017-3031`
- Product export matching uses `function.qualified_name` as the backend name.
  - `lib/src/lib.rs:819-849`
- Product serialization stores `qualified_name` in serialized HIR function and function interface rows.
  - `lib/src/products/type_table.rs:58-89`

Why this is production-active debt:

- HIR carries backend naming concerns.
- `qualified_name` is a hybrid field: display name, lookup compatibility key, artifact name, and backend symbol fallback.
- That hybrid role creates unclear ownership.

Clean target:

- Split the concept into explicit fields/owners:
  - source/display name: HIR or resolver diagnostic metadata;
  - canonical path/display path: resolver/artifact interface;
  - backend symbol: mono/backend contract/link data;
  - method identity: `DefId` and `impl_id`/`trait_id`, never a formatted string.
- Remove backend symbol construction from collect/lower.
- Generate backend symbols once from `InstanceId`/`DefId` plus crate namespace.
- Artifact loading should not recover callable identity from `qualified_name` fallbacks.

### 9. Product/Artifact Backend Symbol Ownership Is Duplicated

Current facts:

- `ProductIdentityTable` contains `backend_symbols`.
  - `lib/src/products.rs:331-346`
- `ProductLinkData` also contains backend symbols through link records.
  - `lib/src/products.rs:361-370`
- Product construction copies link records into `identity_table.backend_symbols`.
  - `lib/src/products.rs:884-885`
  - `lib/src/products.rs:2314-2329`
- Compile attaches both link records and identity-table backend symbols.
  - `lib/src/lib.rs:694-734`
- Artifact loading reads link records first, then falls back to identity-table backend symbols.
  - `lib/src/crate_artifact/load.rs:1534-1552`
- External crate imports fall back from object link symbols to `qualified_name`, then to interface names.
  - `lib/src/crate_system/extern_store.rs:401-420`
  - `lib/src/crate_system/extern_store.rs:446-463`

Why this is production-active debt:

- Backend symbol authority is split between identity and link data.
- The loader accepts both representations.
- Missing backend symbols can be silently reconstructed from display-ish names.

Clean target:

- `ProductLinkData.records` is the only serialized backend symbol authority.
- Remove `ProductIdentityTable.backend_symbols`.
- Reject artifacts that claim object-backed concrete bodies without link records.
- Remove loader fallback from identity-table backend symbols.
- Remove external-store fallback from link symbols to `qualified_name`/interface names for object-backed code.
- Bump artifact format when changing this.

### 10. Type Finalization Has Lenient Fallback Paths

Current facts:

- `finalize_lenient` still exists and defaults unresolved variables to `I64`.
  - `lib/src/infer/mod.rs:358-394`
- Strict `finalize` uses `apply_finalization`.
  - `lib/src/infer/mod.rs:398-446`
- `apply_finalization` finalizes current-unit functions strictly but external artifact functions leniently.
  - `lib/src/infer/finalize.rs:76-110`
- It finalizes impl methods leniently because it cannot distinguish own vs external artifact impls.
  - `lib/src/infer/finalize.rs:112-123`
- It finalizes trait default methods leniently.
  - `lib/src/infer/finalize.rs:125-138`
- It finalizes struct fields and externs with an `I64` default.
  - `lib/src/infer/finalize.rs:140-164`
- Fully lenient finalization remains.
  - `lib/src/infer/finalize.rs:175-247`
- The inference engine returns `Type::Error` for ambiguous type vars so compilation can continue.
  - `lib/src/infer/engine.rs:512-559`

Why this is production-active debt:

- Accepted compilation can mask unresolved types by defaulting them.
- The inability to distinguish own vs external impl methods is itself a phase ownership problem.
- Lenient fallback is appropriate for diagnostics/recovery only, not for final accepted HIR.

Clean target:

- One finalization policy for accepted compilation: all own accepted code must be fully resolved or produce diagnostics.
- External artifact interfaces should already be finalized/validated when loaded.
- Generic declarations should carry explicit generic params, not unresolved type vars that need `I64` fallback.
- Add a post-finalization invariant check that no `TypeVar` or `Type::Error` reaches monomorphization.
- Remove `finalize_lenient` unless there is a narrowly documented artifact-validation use that cannot affect accepted code.

### 11. `Type::Error` Sentinel Can Reach Later Phases

Current facts:

- Type lowering returns `Type::Error` on unknown associated types and invalid bare slices.
  - `lib/src/type_lowering.rs:45-134`
- Lower generic type lookup returns `Type::Error` when a generic name is missing.
  - `lib/src/lower/types.rs:31-34`
- Lower expressions use `Type::Error` for missing self/field type failures.
  - `lib/src/lower/expression.rs:1047-1099`
- Secondary field access can fall back to `Type::Error`.
  - `lib/src/lower/control_flow/secondary.rs:1168-1232`
- Inference engine uses `Type::Error` in strict finalization to continue after ambiguous types.
  - `lib/src/infer/engine.rs:518-559`
- Later phases still match `Type::Error`:
  - codegen: `lib/src/codegen/mod.rs:391-414`
  - products: `lib/src/products.rs:1065`, `lib/src/products.rs:2459`
  - MIR agreement: `lib/src/mir/agreement.rs:473`
  - borrowck: `lib/src/mir/borrowck/mod.rs:1242`

Why this is production-active risk:

- Some `Type::Error` use is normal for diagnostic recovery.
- But later phases accepting or ignoring it can hide missing error gates.
- A production compiler should have explicit phase barriers preventing error sentinels from reaching executable backend phases.

Clean target:

- Keep `Type::Error` as recovery-only while collecting diagnostics.
- Add a required validation barrier after lowering/inference before mono.
- Accepted `ResolvedHirProgram` should not contain `Type::Error` or `TypeVar`.
- Products should reject `Type::Error` in serialized accepted artifacts unless explicitly representing failed compilation, which this compiler does not appear to do.

### 12. Compiler-Recognized Traits Are Inconsistent: `Drop` Uses Language Items, `Sized` Uses Strings

Current facts:

- `Drop` is modeled through language items.
  - `lib/src/hir/mod.rs:101-104`
  - `lib/src/lib.rs:226-236`
  - `lib/src/mono/process.rs:132-133`
  - `lib/src/mono/external.rs:37-46`
  - `lib/src/crate_system/extern_store.rs:68-93`
- `Sized` is resolved by prelude string.
  - `lib/src/lower/resolution.rs:88-97`
- Lower auto-implements `Sized` by resolving `"Sized"`.
  - `lib/src/lower/traits/conformance.rs:800-849`
- Lower injects implicit `Sized` bounds by resolving `"Sized"`.
  - `lib/src/lower/bodies.rs:774-794`
- Selection identifies builtin `Sized` by canonical string `"stdlib::sized::Sized"`.
  - `lib/src/selection/service.rs:1317-1337`
- Inference looks up `"stdlib::sized::Sized"` directly.
  - `lib/src/infer/mod.rs:361`
  - `lib/src/infer/mod.rs:400`

Why this is production-active debt:

- `Sized` is a compiler-recognized semantic concept but is identified by strings.
- This breaks the repository rule that compiler code must not hardcode stdlib trait names for operator/trait meanings except explicitly allowed prelude injection.
- It creates inconsistent handling with `Drop`, which is closer to the correct model.

Clean target:

- Add `stdlib_sized_trait` to language items or a more general `HirLanguageItems` model.
- Artifact interface stores this language item ID.
- Extern crate metadata exposes it like `Drop`.
- Lower/infer/selection use the ID only.
- Delete string checks for `"Sized"` and `"stdlib::sized::Sized"` outside tests.

### 13. Builtin Index Selection Is A Compiler-Owned Semantic Fallback

Current facts:

- Selection tries user `Index` impls, then falls back to builtin index selection.
  - `lib/src/selection/service.rs:720-735`
- Builtin selection fabricates `SelectedOrigin::BuiltinIndex` with no function/impl/target.
  - `lib/src/selection/service.rs:737-773`
- `SelectedOrigin::BuiltinIndex` exists in selection output.
  - `lib/src/selection/types.rs:37-45`
- Builtin output is hardcoded structurally.
  - `lib/src/type_services/facts.rs:62-77`
- It supports slices, arrays, and pointers indexed by `I64`.

Why this is production-active debt or at least a required design decision:

- If indexing is supposed to be stdlib/operator-defined, this violates the rule against compiler-owned fallback meaning.
- If indexing is supposed to be core language syntax, then the current path is still unclear because it is modeled as method selection fallback.

Clean target options:

- Option A: indexing is core language semantics.
  - Lower/index selection should produce a dedicated core operation, not a fake method.
  - MIR should have explicit index/bounds semantics or an intrinsic call with typed contract.
  - Do not pretend it is a missing trait method.
- Option B: indexing is stdlib trait semantics.
  - Remove builtin fallback.
  - Require explicit loaded dependency/prelude `Index` implementations.
  - Use selected trait method IDs and instances like all other operator meanings.

The spec must decide this explicitly.

### 14. AST Syntax Types Leak Past Lowering

Current facts:

- `HirFunction.self_receiver` stores `Option<SelfReceiverMode>` imported from AST.
  - `lib/src/hir/mod.rs:1310-1323`
- Serialized product function rows store `crate::ast::SelfReceiverMode`.
  - `lib/src/products/type_table.rs:58-89`
- Mono imports and uses `ast::SelfReceiverMode`.
  - `lib/src/mono/process.rs:67-98`
- MIR selected-method metadata stores `Option<crate::ast::SelfReceiverMode>`.
  - `lib/src/mir/identity.rs:808-817`
- MIR builder writes AST receiver mode into MIR selected-method metadata.
  - `lib/src/mir/builder/mod.rs:2339-2359`

Why this is production-active layering debt:

- AST syntax should not leak into HIR/MIR/product semantic contracts.
- Receiver syntax should be lowered into a semantic receiver mode or ABI/pass mode.
- This creates unnecessary coupling between parse syntax and backend phases.

Clean target:

- Introduce a semantic receiver mode in HIR or selection, for example `HirReceiverMode` or `ReceiverPassKind`.
- Lower AST `SelfReceiverMode` into that semantic enum immediately.
- MIR should use `MirPassMode`/`MirParamAbi`/receiver adjustment metadata, not AST syntax.
- Product serialization should use semantic receiver mode if it must serialize this fact.

## Production Gaps Not Strictly Legacy But Relevant To Production Grade

These are not all old systems, but they are visible production gaps that should be tracked as part of the professional compiler cleanup.

### 1. MIR Constant Codegen Has Unsupported Constants In Generic Path

Current facts:

- `Constant::Callable(_)` and `Constant::TypeId(_)` in `compile_mir_constant` return `"MIR constant codegen is not implemented yet"`.
  - `lib/src/codegen/mir_llvm/operand.rs:74-78`
- Callable constants have a separate function path immediately after this.
  - `lib/src/codegen/mir_llvm/operand.rs:82-100`

Why it matters:

- If these constants can reach the generic constant codegen path, compilation fails late.
- If they should never reach the path, the code should state and validate that invariant.

Clean target:

- Either implement all valid MIR constants in codegen or reject invalid ones during MIR validation before codegen.
- Replace generic “not implemented yet” with a precise invariant error if unreachable.

### 2. MIR Assertion Codegen Is Partial

Current facts:

- `MirAssertKind::BoundsCheck` is implemented.
  - `lib/src/codegen/mir_llvm/assert.rs:16-19`
- Other assertions return `"MIR assertion codegen is not implemented yet"`.
  - `lib/src/codegen/mir_llvm/assert.rs:20-24`

Why it matters:

- `MirAssertKind::NonNull` appears in agreement handling as unsupported/malformed.
  - `lib/src/mir/agreement.rs:107-114`
- A production MIR should either fully define supported assertions or reject unsupported assertion kinds before codegen.

Clean target:

- Define the full assertion set.
- Implement supported assertions.
- Reject unsupported assertions during MIR validation, not during codegen.

### 3. MIR Call Cleanup/Unwind Codegen Is Not Implemented

Current facts:

- Calls with cleanup blocks error with `"MIR call cleanup codegen is not implemented yet"`.
  - `lib/src/codegen/mir_llvm/terminator.rs:170-184`

Why it matters:

- If MIR can represent cleanup/unwind edges, backend either needs to support them or a prior phase must guarantee they cannot occur.
- The current codegen path fails late.

Clean target:

- Decide whether Rock supports unwinding/cleanup in MIR now.
- If yes, implement cleanup codegen.
- If no, remove or forbid cleanup from production MIR and keep it out of terminators until supported.

### 4. `rock test` Was Exposed But Not Implemented (Closed)

Status: Complete as of 2026-08-02. The unimplemented public command was removed
from the Clap command enum and dispatch. `rock test` is now rejected during CLI
parsing, with a focused regression test.

Historical facts that motivated the cleanup:

- CLI exposes `Command::Test`.
  - `rock/src/cli.rs:49-60`
- Running it returns `"rock test is not implemented yet"`.
  - `rock/src/cli.rs:14-23`

Why it matters:

- This is not compiler pipeline debt, but it is a production-facing scaffold.
- A professional CLI should not advertise a command that is not implemented unless marked unstable/hidden.

Clean target:

- Implement `rock test`, hide it, or remove it until ready.

## Test-Only Scaffolds That Should Be Tightened

These are not production paths, but they preserve old mental models and can let tests pass without exercising the actual clean backend contract.

### 1. Unchecked MIR Codegen Test Path

Current facts:

- `compile_mir_program_unchecked_for_test` exists under `#[cfg(test)]`.
  - `lib/src/codegen/mir_llvm/mod.rs:53-61`
- It calls:
  - `sync_test_nominal_layouts`
  - `sync_test_backend_contract`
  - `compile_mir_program_bodies_unchecked`
- `sync_test_nominal_layouts` populates ID-keyed maps from old name-keyed maps.
  - `lib/src/codegen/mir_llvm/mod.rs:76-111`
- Many MIR LLVM tests construct `MirProgram` with default/partial backend contract and default metadata.

Why it matters:

- Tests do not always model production MIR.
- Old layout side-table assumptions remain alive in fixtures.
- Regressions in contract construction can be missed if tests bypass the contract.

Clean target:

- Add a proper `MirProgramFixtureBuilder` that requires a valid `MirBackendContract`.
- Delete unchecked codegen path where feasible.
- Keep only very narrow tests for internal codegen primitives if absolutely needed.
- Those tests should still explicitly state what contract facts are intentionally omitted.

### 2. Legacy Backend Contract Adapter Tests Preserve Old System

Current facts:

- `MirBackendContract::from_legacy_metadata*` tests occupy a large section.
  - `lib/src/mir/backend_contract.rs:823-1072`
- Tests verify adapter behavior such as:
  - declarations from metadata;
  - projection output copies;
  - drop glue linkage;
  - product link candidates;
  - runtime requirements empty state.

Why it matters:

- These tests will need deletion or conversion once the adapter is removed.
- Keeping them would imply continued support for the old system.

Clean target:

- Replace adapter tests with direct contract construction tests.
- Test direct builder output from mono/MIR fixture inputs.
- Add negative tests that stale legacy metadata cannot exist because the type is gone.

### 3. Tests With `backend_contract: Default::default()`

Current facts:

- Many codegen/MIR tests use default backend contracts.
- Many tests use `MirBackendMetadata` fixtures and then call `from_legacy_metadata`.

Why it matters:

- Default/empty backend contracts are often unrealistic for production MIR.
- The test suite can mask missing callable/layout/drop/projection contract facts.

Clean target:

- Require contract fixture helpers to declare all callables/layouts/projections used by a test.
- Tests that intentionally verify missing contract behavior should name that intention and expect validation errors.

## Acceptable / Not Old Systems / False Positives

The following are acceptable if kept within their intended ownership boundaries.

### 1. Resolver Name Maps At The Collection Boundary

Current facts:

- `ResolverTables` contains name and alias maps.
  - `lib/src/collect/resolver.rs:8-18`
- Fields include:
  - `module_paths`
  - `module_names_by_id`
  - `item_paths`
  - `item_names_by_id`
  - `import_aliases`
  - `export_aliases`
  - `scoped_module_aliases`
  - `module_aliases`

Why acceptable:

- Collection/resolution is exactly where names belong.
- Source code, imports, exports, and modules are name-based inputs.
- The issue is not that resolver has names; the issue is downstream phases using names to recover semantics after resolver should have produced IDs.

Boundary rule:

- Resolver names may answer source/import/export questions.
- Downstream semantic phases should consume resolved IDs and explicit target structs.

### 2. Source Loader Qualified Module Names

Current facts:

- Source loader tracks modules by qualified names and paths.
  - examples found in `lib/src/source_loader/mod.rs`.

Why acceptable:

- Module discovery is inherently name/path based.
- This is input loading, not backend semantic lookup.

Boundary rule:

- Qualified module names should not become backend symbols or semantic owner identities outside resolver/import/export boundaries.

### 3. `ProductDefId` Remapping And Artifact Identity Collision Handling

Current facts:

- Product/artifact code remaps IDs and reserves fallback product IDs when needed.
  - examples around `lib/src/products.rs:2300-2374`

Why acceptable:

- Serialized artifact identity needs stable IDs and collision handling.
- This is not the same as backend metadata duplication.

Boundary rule:

- Product ID remapping is acceptable for artifact identity.
- It should not carry backend symbol fallback data in identity tables.

### 4. `HirNameTables` As Display/Artifact Compatibility Data

Current facts:

- `HirNameTables` is documented as display/serialization aliases.
  - `lib/src/hir/mod.rs:80-91`

Why acceptable:

- Diagnostics and artifact display names need a source-facing naming model.
- Tests may inspect aliases.

Boundary rule:

- Keep it out of mono, MIR, codegen, backend contracts, and method/layout/projection/drop decisions.

### 5. Product Interface To Loaded Artifact Interface Conversion

Current facts:

- Product loading builds an `ArtifactCrateInterface` from serialized product data.
  - `lib/src/crate_artifact/load.rs:2984-3099`

Why acceptable:

- Serialized artifact representation and in-memory loaded dependency representation are different boundaries.
- Conversion itself is not a legacy system.

Concern to fix separately:

- During conversion, `function.qualified_name.get_or_insert(canonical_name)` still preserves the old hybrid `qualified_name` model.
  - `lib/src/crate_artifact/load.rs:3025-3028`

### 6. `rock-shared` Sysroot Fallback

Current facts:

- `rock-shared/src/sysroot.rs` has executable-relative fallback logic.

Why acceptable:

- This is deployment path fallback, not compiler semantic fallback.
- It does not affect semantic identity or backend contracts.

### 7. `Drop` Language Item Model Is Directionally Correct

Current facts:

- `Drop` trait/method IDs flow through `HirLanguageItems`, product language items, and extern crate metadata.

Why acceptable:

- This is the right model for compiler-recognized items.
- It should be generalized to `Sized` and any other compiler-recognized traits.

## Specific Clean-Slate Target Pipeline

This should become the architecture section of the future spec.

### Phase 1: Parse

Input:

- source files and loaded module graph.

Output:

- AST and parse diagnostics.

Allowed concepts:

- syntax names;
- raw AST receiver syntax;
- raw parse types;
- source spans.

Forbidden concepts:

- `DefId` semantics except possibly parser-independent IDs if already assigned externally;
- backend symbols;
- type inference facts;
- monomorphization instances.

### Phase 2: Collect / Resolve

Input:

- AST;
- explicit crate dependencies;
- explicit prelude imports when stdlib is passed.

Output:

- ID-keyed declarations;
- `ResolverTables`;
- module graph identities;
- import/export alias tables;
- language item IDs discovered from explicitly loaded crates.

Allowed concepts:

- names and aliases;
- module paths;
- source/export display names;
- ID allocation.

Forbidden concepts:

- backend symbols;
- generated impl method backend names;
- lower/infer lookup by strings after IDs exist.

Clean contract:

- This is the only phase that resolves names to IDs.
- Later phases can ask for diagnostic display names, but not for semantic recovery.

### Phase 3: Lower

Input:

- AST;
- ID-keyed declarations;
- resolver tables;
- language item IDs.

Output:

- HIR with semantic IDs;
- initial type variables/constraints;
- semantic receiver modes converted from AST receiver syntax;
- source spans and diagnostic labels.

Allowed concepts:

- `DefId`, `FieldId`, `VariantId`, `AssocTypeId`;
- semantic receiver modes;
- local IDs;
- source/display names for diagnostics.

Forbidden concepts:

- `HashMap<String, HirFunction>` as semantic storage;
- `def_id_for_name` production recovery;
- backend symbol construction;
- `qualified_name` as lookup/backend key;
- AST enum types in final HIR semantic fields.

Clean contract:

- HIR is ID-keyed.
- HIR names are diagnostics/display only.

### Phase 4: Infer / Select

Input:

- HIR;
- constraints;
- resolver/language item IDs;
- dependency interfaces.

Output:

- fully typed HIR;
- selected method/call authorities;
- associated type projection outputs;
- final language item use decisions.

Allowed concepts:

- type variables during solving;
- `TypeId`/`TypeContext` once interned;
- selected targets by IDs.

Forbidden concepts:

- identifying `Sized`, `Index`, etc. by string;
- method selection by display aliases;
- accepted `Type::Error` or unresolved `TypeVar` after finalization;
- mono/MIR re-running selection logic.

Clean contract:

- Selection is authoritative.
- If selection cannot resolve a call, compilation reports diagnostics before mono/MIR.

### Phase 5: Monomorphization

Input:

- resolved typed HIR;
- selected call authorities;
- dependency interfaces/bodies;
- `TypeContext`.

Output:

- concrete instance registry;
- pre-MIR instance bodies;
- instance IDs and backend symbols;
- object-provided instance records.

Allowed concepts:

- `InstanceKey`;
- `InstanceId`;
- `DefId`;
- `TypeId`;
- source/display names for debug only;
- backend symbols generated from canonical identity.

Forbidden concepts:

- `function_aliases` used for semantic lookup;
- `qualified_name` used to recover origins;
- type-name based receiver matching;
- duplicate method selection logic.

Clean contract:

- Mono specializes exactly the selected/required instances.
- It does not rediscover method targets by names.

### Phase 6: MIR Build

Input:

- monomorphized program;
- instance bodies;
- type context;
- selected authorities and projection outputs.

Output:

- `MirProgram` containing:
  - functions;
  - type context;
  - `MirBackendContract`.

Allowed concepts:

- `MirCallableKey` only for callable operands in final MIR;
- typed MIR locals/places/rvalues/terminators;
- `MirPassMode`, `MirParamAbi`, `MirReturnAbi`;
- `MirNominalLayout` by `DefId`;
- `MirProjectionKey`;
- `MirRuntimeHelper`.

Forbidden concepts:

- `MirBackendMetadata`;
- `from_legacy_metadata`;
- unresolved `MirCallable::Method` in final MIR;
- display alias layout injection.

Clean contract:

- MIR is directly consumable by borrowck/agreement/codegen.
- The backend contract is authoritative and complete.

### Phase 7: MIR Checks

Input:

- final MIR program;
- backend contract.

Output:

- borrow diagnostics;
- agreement diagnostics;
- internal compiler errors for violated invariants.

Allowed concepts:

- place paths;
- loan IDs;
- move paths;
- contract validation.

Forbidden concepts:

- legacy backend metadata validation;
- name-based layout/projection/callable recovery.

Clean contract:

- Accepted MIR has no unresolved callables, missing layouts, stale projection outputs, missing drop glue for required cleanup, or unsupported assertions/terminators.

### Phase 8: Codegen

Input:

- MIR;
- backend contract;
- dependency link inputs.

Output:

- LLVM IR/object/executable;
- codegen symbol override records if LLVM/linker required final emitted names.

Allowed concepts:

- LLVM symbol strings;
- linker symbol mangling;
- ABI lowering;
- runtime helper declarations.

Forbidden concepts:

- HIR executable types;
- `MirBackendMetadata`;
- display aliases for semantics;
- fallback from missing link symbols to qualified/source names.

Clean contract:

- Codegen is a consumer, not a resolver.
- If the contract lacks a fact, codegen errors as an invariant violation.

### Phase 9: Products / Artifacts

Input:

- resolved HIR interface/body data;
- backend contract artifact exports;
- final emitted backend symbols;
- dependency identity data.

Output:

- serialized product interface;
- product bodies;
- product link records;
- source fingerprint;
- language items.

Allowed concepts:

- product IDs;
- artifact display names;
- export aliases;
- link records.

Forbidden concepts:

- backend symbols in identity table;
- recovering missing object symbols from display names;
- old metadata rows preserved for compatibility.

Clean contract:

- Artifact format can be bumped.
- No backward compatibility shim is required during this prototyping phase unless explicitly requested.

## Recommended Refactor Order

This order is designed to reduce risk while moving toward the clean target.

### Step 1: Remove `MirBackendMetadata` And Adapter Layer

Status: Complete; final gates and direct-source audit refreshed on 2026-07-16.
`MirProgram` carries `MirBackendContract` as its only backend authority, built
directly from canonical HIR/program data. The eight staging types
(`MirDropGlue`, `MirProjectionResolution`, `MirInstanceDeclaration`,
`MirProjectionImplMetadata`, `MirExternDeclaration`, `MirProductLinkCandidate`,
`MirStructLayout`, and `MirEnumLayout`) and the struct/enum display-alias
traversal are deleted. Agreement has one complete contract-validation path;
there is no MIR-only mode or validation/layout opt-out. Builder tests use
complete contract-native fixtures, validate malformed projection outputs safely,
and prove deterministic projection `TypeId` traversal. Runtime requirements are
contract-owned and populated by the canonical shared MIR observer; agreement and
codegen preflight require exact agreement with observed MIR requirements, and
codegen declares runtime helpers only when the contract requires them.

Fresh validation evidence:

- `cargo test -p rock-lib --test integration`: `483` passed.
- `cargo test -p rock-lib`: `1,855` library tests, `483` integration tests,
  and one auxiliary test passed; one doc test was intentionally ignored.
- `cargo clippy -p rock-lib --all-targets`: completed with zero errors; it
  reported 130 library warnings, 3 integration-test warnings, and 193 lib-test
  warnings (129 duplicates). Three pre-existing-style Clippy warnings occur in
  changed `lower/paths.rs` test helpers (two `type_complexity`, one
  `too_many_arguments`); no new warning is in the Step 1 production contract
  construction or agreement code.
- `cargo fmt --all --check` and `git diff --check` passed.
- Direct residue scans found zero declarations of the eight staging types, zero
  `struct_display_aliases`/`enum_display_aliases`/`canonical: false` contract
  traversal markers, and zero production instances of
  `check_mir_runtime_agreement_for_mir_only`, `validate_backend_contract: bool`,
  `nominal_layouts_required`, or `layout_checks_enabled`. The Step 2-5
  forbidden-residue checks remained clear, and both product artifact constants
  remain `37`. Runtime checks found no `PanicBounds` residue or duplicate
  runtime-requirements scanners; the shared observer derives bounds-check and
  closure-allocation requirements, while codegen consumes the exact contract set
  conditionally.

Goal:

- Delete the most obvious old backend system.

Tasks:

- Remove `backend_metadata` from `MirProgram`.
- Replace `MirBuilder::backend_metadata_for_program` with direct `MirBackendContract` construction.
- Delete `MirBackendContract::from_legacy_metadata`.
- Delete `MirBackendContract::from_legacy_metadata_with_type_context`.
- Delete legacy helper functions in `backend_contract.rs`.
- Move or recreate required ABI/pass-mode logic as direct contract construction logic.
- Update agreement to validate only contract data.
- Update codegen to register layouts from contract only.
- Convert tests to contract-native fixtures.

Acceptance criteria:

- No production reference to `MirBackendMetadata`.
- No reference to `from_legacy_metadata` outside possibly deleted tests.
- `MirProgram` has only one backend authority.
- `cargo test -p rock-lib codegen -- --nocapture` passes.
- `cargo test -p rock-lib mir::agreement -- --nocapture` passes.
- Existing integration tests pass.

### Step 2: Remove Codegen Name-Keyed Layout Side Tables From Production

Status: Complete as of 2026-07-04. `CodeGen` no longer carries production
name-keyed struct layout or generic side tables (`struct_info`,
`struct_generic_owners`, `struct_generic_param_ids`) and the name-based
`struct_substitution` helper has been removed. Struct and enum generic layout
substitution now uses contract-populated `DefId` keyed tables. Existing enum
layout state was already ID-keyed; no production `enum_info` or
`enum_generic_owners` side table remains.

Validation evidence:

- `cargo test -p rock-lib codegen::types -- --nocapture`
- `cargo test -p rock-lib codegen -- --nocapture`
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`
- `cargo test -p rock-lib --test integration`
- `cargo fmt --all --check`
- `git diff --check`
- Source residue scan over `lib/src/codegen` found no matches for
  `struct_info`, `enum_info`, `struct_generic_owners`,
  `enum_generic_owners`, `struct_generic_param_ids`,
  `enum_generic_param_ids`, or `struct_substitution(`.

Goal:

- Make codegen layout/projection lowering ID-keyed and contract-owned.

Tasks:

- Remove production writes to `struct_info`, `enum_info`, `struct_generic_owners`, `enum_generic_owners`.
- Replace `struct_substitution(name, ...)` with ID-keyed variants.
- Keep display maps only for diagnostics if necessary.

Acceptance criteria:

- Codegen semantic layout lookup uses `DefId` only.
- Name-keyed layout maps are absent from production code or diagnostics-only.

### Step 3: Make Product Link Records The Only Backend Symbol Source

Status: Complete, with final evidence refreshed on 2026-07-15.
`ProductIdentityTable` no longer stores backend symbols. Product artifact
backend symbols are serialized only through `ProductLinkData.records` and loaded
into `ExternCrateLink.backend_symbols`. Link attachment uses the producer's
explicit `ProductIdRemap`; missing or ambiguous callable mappings fail current
compilation without display/export-name recovery. Artifact loading rejects every
link record with an empty backend symbol. Object-backed concrete callable
artifacts require explicit link records, and production dependency symbol lookup
does not fall back to `qualified_name`, display names, interface names, impl type
names, trait names, or method names. The product artifact format version was
bumped for the original schema change; the remap and validation hardening did
not change the schema.

Validation evidence:

- `cargo test -p rock-lib products -- --nocapture`
- `cargo test -p rock-lib crate_artifact -- --nocapture`
- `cargo test -p rock-lib crate_system -- --nocapture`
- `cargo test -p rock-lib mono::external -- --nocapture`
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`
- `cargo test -p rock-lib --test integration`
- `cargo fmt --all --check`
- `git diff --check`
- Source residue scans found no production matches for removed identity-table
  backend-symbol storage or source-name backend-symbol fallbacks.
- 2026-07-15 refresh: focused product-link tests passed (11), artifact tests
  passed (143), semantic identity audits passed (37), and the complete
  `rock-lib` suite passed (1,839 library, 483 integration, and one auxiliary
  test; one doc test intentionally ignored).

Goal:

- Eliminate duplicated backend symbol storage.

Tasks:

- Remove `ProductIdentityTable.backend_symbols`.
- Remove writes to identity backend symbols in `attach_product_link_records`.
- Remove `backend_symbols_from_link` duplication into identity table.
- Remove loader fallback from identity backend symbols.
- Update artifact validation to require link records for object-backed concrete bodies.
- Remove external-store fallback from link symbol to `qualified_name`/interface names.
- Bump artifact format.

Acceptance criteria:

- Backend symbols exist only in `ProductLinkData.records` and loaded `ExternCrateLink`.
- Missing link records produce explicit artifact errors.

### Step 4: Split `qualified_name` Into Explicit Concepts

Status: Complete as of 2026-07-08. `HirFunction` no longer owns
`qualified_name`, collect/lower no longer fabricate impl backend names, mono
generates local backend symbols from `InstanceOrigin` plus substitution, and
product/artifact function rows no longer serialize `qualified_name`. Static
associated function display names use canonical source paths such as
`Type::method`; object-backed external symbols still come only from product link
records. The product artifact format version was bumped for the schema change.

Validation evidence:

- `cargo test -p rock-lib products -- --nocapture`
- `cargo test -p rock-lib crate_artifact -- --nocapture`
- `cargo test -p rock-lib mono -- --nocapture`
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`
- `cargo test -p rock-lib --test integration > /tmp/rock-lib-integration-step4.log 2>&1`
- `cargo check -p rock-lib`
- `cargo fmt --all --check`
- `git diff --check`
- Source residue scans found no production matches for `HirFunction`
  `qualified_name`, impl backend-name construction, serialized
  `qualified_name` function rows, or stale MIR selected/projection backend
  symbol fields.

Goal:

- Remove HIR backend-name ownership and hybrid string semantics.

Tasks:

- Define replacement fields/types:
  - display/source path metadata;
  - method owner identity;
  - backend symbol source in mono/contract only.
- Remove `format_impl_backend_name` from collect/lower production paths.
- Stop writing backend-like names into `HirFunction`.
- Update mono instance origin to use IDs only.
- Update artifact interface/load to use canonical display paths without treating them as backend symbols.

Acceptance criteria:

- `HirFunction` no longer has `qualified_name` or the field is renamed/restricted to display metadata with no backend/semantic consumers.
- No backend symbol fallback from `qualified_name`.

### Step 5: Make Method Selection Authority Fully ID-Based

Status: Complete and final-gate verified as of 2026-07-15. Selection records
explicit impl-method, trait-method, and builtin-index authority variants with
canonical IDs and typed owner/method substitutions. Accepted HIR validates that
authority before mono, including a post-inference selection pass for calls whose
receiver type was not concrete during lowering. Typed impl receiver patterns and
effective trait-member mappings persist through product artifacts (format `37`).
Qualified static-method resolution now persists exact impl, method, optional
trait/member, receiver-pattern, and generic-parameter authority. Lowering
instantiates that authority by canonical `GenericParamId`, including structural
owner-to-method generic relations and selected trait arguments, without rescanning
impls or correlating display names. Mono materializes selected targets as
`InstanceId` call edges; final MIR has no unresolved method callable variant,
method-name dispatch metadata, or builtin-index guard metadata. Legacy type-name
matching is test-only, and production mono/MIR/DCE/codegen do not use it for
dispatch.

The final compliance pass introduced phase-parameterized executable HIR. Strict
inference recursively converts unresolved HIR into a structurally distinct
`AcceptedHirProgram`; required method and `Try` authorities are non-optional in
accepted nodes, and impl receiver authority is canonical on each accepted impl.
Conversion rejects raw method `DefId` callees without static authority,
incomplete `Try` receiver authority, unresolved/error types, and invalid typed
receiver-pattern bindings. Mono no longer repairs method identities or receiver
modes, rejects every overlapping typed impl match without implicit
specialization, and MIR's raw `DefId` callable map contains function origins
only.

The 2026-07-14 whole-suite pass closed six migration regressions: stale
receiver-authority fixtures, checked layout projection metadata, bare-slice impl
policy bypass, selfless default declaration-shape loss, generic impl body
attachment under rediscovered generic ordering, and Try/FromResidual diagnostic
wording. The 2026-07-15 follow-up closed F65 by deleting qualified-static
authority reconstruction and preserving resolver-owned typed authority through
lowering. The authoritative closure ledger records all F1-F65 repairs.

Validation evidence:

- `cargo test -p rock-lib` (`1,839` library tests, `483` integration tests, and
  the auxiliary test target passed; one doc test remains intentionally ignored)
- `cargo test -p rock-lib --test integration` (`483` passed)
- `cargo test -p rock-lib crate_artifact -- --nocapture` (`143` passed)
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture` (`37` passed)
- `cargo test -p rock-lib lower::paths -- --nocapture` (`48` passed)
- `cargo clippy --workspace --all-targets` (completed with no errors)
- `cargo fmt --all --check`
- `git diff --check`
- A fresh direct-source audit covered strict accepted-HIR creation, static
  resolution/lowering, all selection/mono materializers,
  products/artifacts/providers, DCE, MIR, and codegen.
- Production-source residue scans found no mono/MIR method lookup by receiver
  names, display aliases, receiver sidecars, specificity winners, or legacy
  lookup helpers; `selection/legacy_lookup.rs` and
  `static_method_target_for_callee` are deleted. The only remaining
  `lookup_self_receiver` helper is `#[cfg(test)]` and cannot repair production
  HIR.

Goal:

- Remove string/type-name method rediscovery after selection.

Tasks:

- Expand `SelectionAuthority` to include all data mono/MIR need.
- Ensure lower records selected target authority on HIR calls/method calls.
- Mono consumes selected authority for instance creation.
- MIR consumes selected authority for callable keys.
- Delete duplicate receiver/type-name matching in mono and MIR builder.
- Delete or quarantine `selection/legacy_lookup.rs`.

Acceptance criteria:

- Mono/MIR do not call `type_names_for_method_lookup`, `impl_matches_method_lookup_type`, or display alias helpers for method dispatch.
- Method calls lower to resolved IDs or fail before mono.

### Step 6: Migrate Collect/Lower To ID-Keyed Declarations

Status: Complete as of 2026-08-02. The collection, lowering, inference, final
HIR, artifact interface, and reusable-body boundaries are ID keyed.
`ArtifactCrossCrateHir` now stores accepted generic-function and trait-default
payloads in `BTreeMap<DefId, ...>` maps. Artifact loading remaps every ID-keyed
function, struct, enum, trait, and generic-function-body row without using
`product_display_name` as an admission gate. Body providers retain only
canonical ID-keyed payload maps; optional interface names remain display data
and cannot decide whether a semantic definition exists.

The previously completed portion is unchanged: collection now
performs one checked canonical collapse into private `DeclarationItems` maps
keyed by `DefId`; `Declarations`, `LowerItems`, and `PartialHir` preserve that
single payload authority through inference and direct ID-native `HirProgram`
construction. Functions, standalone signatures, structs, enums, traits, impls,
externs, and static impl methods no longer have synchronized name-keyed or
duplicated payload stores. Static impl methods remain owned only by their exact
`HirImpl`, while resolver metadata and product link records preserve source and
backend names without cloning semantic payloads.

Lowering dispatches root, inline, and source-backed module bodies through exact
`ModuleId` plus original AST ordinal records from `ItemIndex`. Function, trait,
trait-default, impl, and impl-method updates carry canonical owner/member IDs;
missing or mismatched payloads produce invariant diagnostics instead of probing
candidate names or silently skipping bodies. Production `def_id_for_name`,
rebuilt payload indexes, positional owner repair, inference-time alias payload
deduplication, and the product duplicate-method compatibility filter are
deleted. ID-sensitive body processing and multi-error diagnostics are ordered by
`DefId`.

Within collection, lowering, and final HIR, names remain intentionally confined to
collection-local source-resolution
builders before the one-way checked ID collapse, lexical scopes, owner-local
member syntax, resolver/import/export metadata, diagnostics/display tables, and
artifact export lookup that returns an exact ID. Final HIR name tables are
filtered from resolver metadata by IDs already present in each category and
cannot select semantic payload identity. Artifact/body transport now preserves
that same ownership rule through loaded dependency providers.

Historical validation evidence for the completed collection/lowering portion:

- Focused pre-closure gates passed: collect (`154` unit tests), lower (`358`
  unit and `4` matching integration tests), infer (`27` unit and `8` matching
  integration tests), crate artifact (`147` tests), and semantic identity audit
  (`40` tests).
- The fresh post-closure `cargo test -p rock-lib` run passed `1,903` unit tests,
  `486` integration tests, and `1` parser test with zero failures; one doc test
  remains intentionally ignored.
- `cargo clippy --workspace --all-targets` completed with zero errors. Its `215`
  emitted warning diagnostics are unchanged from the pre-closure run.
- `cargo fmt --all --check` and `git diff --check` completed successfully.
- A final full-codebase inventory covered all `534` tracked Rust files and every
  production definition, constructor, mutation, and consumer of
  `Declarations`, `LowerItems`, and `PartialHir`; it also classified all
  remaining string-keyed HIR maps, name-to-ID helpers, display tables, and
  embedded-ID member scans.
- The closure ledger is `/tmp/opencode/task6-closure-ledger.md`. Its historical verdict was
  `PASS: no production Step 6 debt remains; all name-keyed uses are classified
  allowed boundaries.` The initial 2026-08-02 swipe superseded that whole-step
  verdict for the artifact/body-provider boundary; the final remediation above
  closes the newly discovered exception.
- Final closure verification added a missing-display-metadata artifact
  regression, ID-keyed provider audits, and passed all `180` crate-artifact
  tests plus the complete `rock-lib` suite.

Goal:

- Retire legacy string-keyed declaration maps.

Tasks:

- Make `Declarations` expose ID-keyed item declarations as the primary API.
- Move string maps into resolver/display support or delete them.
- Convert `LowerItems` to ID-keyed storage.
- Replace `PartialHir` string-keyed maps with ID-keyed maps.
- Delete production `def_id_for_name`.

Acceptance criteria:

- Lowering can be understood as AST + resolved declarations -> HIR.
- No semantic ID lookup from candidate strings in lower.

### Step 7: Normalize Compiler-Recognized Traits Through Language Items

Status: Complete; definitive closure evidence refreshed on 2026-07-19.
`lang <role>` syntax binds compiler protocols to the complete typed
`LanguageItems<DefId>` registry: Sized has its trait `DefId`; Drop has trait and
method `DefId`s; Index has trait and method `DefId`s plus its output
`AssocTypeId`; and the Try protocol has Try, Output, Residual, branch,
FromResidual trait/method, ControlFlow enum, and Break/Continue variant IDs.
Collection binds markers through indexed declaration/member identities, and HIR,
products, artifacts, extern metadata, inference, selection, mono, and codegen
consume those IDs rather than provider names.

Provider selection is deterministic: the current source crate and explicitly
supplied artifact dependencies are sorted, exactly one may claim each protocol,
and every current-plus-dependency duplicate conflict is reported. The current
crate does not merge same-name dependency metadata. There is no ambient or
implicit crate-name provider: external providers require explicit artifacts, so
no-prelude compilation has no provider-name authority. Artifact format `38` emits current-crate definitions
only and validates current HIR IDs before serialization. Artifact loading
remaps language-item `DefId`s to the consumer's runtime crate identity, while
`AssocTypeId` and `VariantId` remain positional IDs under validated remapped
owners; load-time language-item preflight validates the resulting bundles.
Renamed providers, local shadowing, cross-crate providers, and
non-reexport of intermediate provider language items are covered by focused
collection, lowering, artifact, and integration tests.

Fresh validation evidence:

- Definitive focused-filter logs under `/tmp/opencode/task7-*-definitive.log`
  show these primary harness results: parser 433 library tests, collect 160
  library tests, lower 299 library tests, infer 31 library tests, selection 60
  library tests, products 87 library tests, crate_artifact 166 library tests,
  crate_system 24 library tests, mono 108 library tests plus one integration
  test, and semantic_identity_audit 40 library tests. The additional
  integration and auxiliary harnesses for the other filters ran zero tests and
  are excluded from those counts. The two added collection tests cover current
  source-provider and self-alias provenance behavior.
- Definitive `tree-sitter generate` and `tree-sitter test` logs are
  `/tmp/opencode/task7-tree-sitter-generate-definitive.log` and
  `/tmp/opencode/task7-tree-sitter-test-definitive.log`: generation reported
  only the unnecessary-conflicts warning, while the language-item corpus passed
  1/1. The dedicated valid indentation query, using the rebuilt Rock parser
  library, is retained at
  `/tmp/opencode/task7-tree-sitter-indent-query-definitive.log` and produced
  indentation captures for `stdlib/drop.rk` without warnings.
- Definitive saved full gates passed:
  `/tmp/opencode/task7-integration-definitive.log` records `cargo test -p
  rock-lib --test integration` at 493/493; and
  `/tmp/opencode/task7-rock-lib-definitive.log` records 2,054 library tests,
  493 integration tests, one auxiliary test, and one ignored doc test.
- `/tmp/opencode/task7-clippy-definitive.log` records `cargo clippy --workspace
  --all-targets` with zero errors but not warning-clean: three rock test
  warnings, one rockup warning, one rock-shared lib-test warning, 139 rock-lib
  library warnings, one rockc test warning, four integration-test warnings, and
  204 rock-lib lib-test warnings (139 duplicates). The four specific
  final-review diagnostics in `language_items.rs`, `infer/constraints.rs`,
  `selection/service.rs`, and `lower/control_flow/secondary.rs` were removed
  with behavior-preserving idiomatic edits and are absent from the definitive
  Clippy log; other production warnings remain as reflected by these counts.
  `cargo fmt --all --check` and `git diff --check` passed.
- The closure ledger is `/tmp/opencode/task7-language-items-closure-ledger.md`.
  Its fresh source inventory found no blocking executable semantic lookup by
  protocol/provider name. The requested legacy lookup tokens have zero
  production hits; protocol spellings are `lang <role>` syntax,
  source-boundary binding, display/diagnostic text, or tests.
- The exact remaining `BuiltinIndex` inventory is 28 production references
  across seven files: hir/mod 11, selection/types 6, selection/service 1,
  mono/process 2, products 2, products/type_table 5, and crate_artifact/load
  1. Six test-only matches are separately classified in the closure ledger.
  Those consumers do not identify an Index trait, member, or associated output
  by name; typed Index language-item IDs do. The explicit targetless fallback
  is handed off exclusively to Step 8 for its core-operation versus stdlib
  intrinsic-backed behavior decision.

Goal:

- Remove string-convention semantic traits.

Tasks:

- Add `stdlib_sized_trait` or a general language item representation.
- Load/store it in product interface and extern crate metadata.
- Replace `resolve_builtin_trait("Sized")` with language item ID access.
- Replace `"stdlib::sized::Sized"` checks with ID checks.
- Decide if `Index` is a language item, core operation, or ordinary stdlib trait.

Acceptance criteria:

- No production string checks for compiler-recognized traits.

### Step 8: Decide And Refactor Builtin Index Semantics

Status: Complete; definitive closure evidence refreshed on 2026-07-23.

Decision: indexing is stdlib trait semantics. The marked `Index` and `IndexMut`
language-item bundles carry typed trait IDs, method IDs, and associated-output
IDs. `IndexMut` must pair with `Index` from the same provider. Selection requires
those IDs and an explicit implementation; it has no compiler-owned fallback and
does not recover behavior from type names. Remaining `TypeFacts` checks only
classify/constrain receiver shapes and concreteness; they do not select an index
implementation.

The `[]` and mutable `[]` source syntax is unchanged. Lowering selects the
language-item `Index`/`IndexMut` method, constructs an ordinary method call, and
dereferences its shared or mutable reference result. Projection outputs remain
ordinary associated-type projections keyed by the marked trait and
`AssocTypeId`; the former builtin-index HIR target/origin/output/projection
variants and serialized forms are absent.

Stdlib ownership is explicit: `stdlib/index.rk` owns implementations for shared
slices, mutable slices, `*T`, and raw slice pointers `*[T]`; `stdlib/vec.rk` owns
`Vec T` read/write implementations. The ordinary Rock `index_bounds_check`
helper performs the normal `index < 0 || index >= len` check. It uses existing
Rock/LLVM facilities and adds no indexing intrinsic. Ordinary mono instance
calls materialize selected read/write impl methods, and MIR handles their
returned references through normal deref/place lowering. Borrow checking and
dataflow continue to treat MIR index projections as place paths for loans and
moves; artifact language-item validation/remapping and format `39` preserve the
same typed authority and selected behavior.

`Projection::Index` and `MirAssertKind::BoundsCheck` remain direct MIR-level
operations used by MIR iteration, cleanup, place lowering, and runtime bounds
checking; they do not restore source-level `HirExprKind::Index` semantics. This
closure does not claim that unrelated MIR assertion kinds are implemented.

Definitive validation evidence:

- Focused serial gates passed: parser 27, collect 161, HIR language items 51,
  lower 300, selection 58, mono 90, MIR builder 86, products 90, crate artifact
  176, and semantic identity audit 40 relevant tests; every command had zero
  failures and zero ignored relevant tests.
- Final-review serial gates also passed: `cargo test -p rock-lib lower` passed
  378 library tests plus 4 matching integration tests; `selection` passed 66
  library tests; `infer` passed 38 library tests plus 8 matching integration
  tests; `mir::borrowck` passed 67 library tests; integration filters `index`,
  `unsafe`, and `borrow` passed 79, 34, and 58 tests respectively. The six new
  exact integration regressions all passed, including distinct `Index`/
  `IndexMut` reference authority, safe custom pointer indexing, key-derived
  reference provenance, and receiver-derived/non-returned reference controls.
- `tree-sitter generate` passed with only the existing unnecessary-conflicts
  warning; `tree-sitter test` passed 1/1 parses.
- The one-shot integration run passed 533/533. The one-shot full
  `cargo test -p rock-lib` run passed 2,063 library tests, 533 integration
  tests, and 1 auxiliary parser test, with 1 ignored doc test and zero failures.
- A 2026-07-26 workspace follow-up fixed product validation for current-crate
  impls of dependency traits, including generated `Sized` impls, by validating
  against the loaded dependency trait/member authority instead of requiring the
  dependency trait in the current product interface. The obsolete, unused
  `receiver_supports_builtin_slice_impl` API and its test were deleted. The
  focused three-test artifact-consumer regression passed, and `cargo test
  --workspace` passed 36 `rock`, 2,062 `rock-lib`, 533 integration, 1 auxiliary
  parser, 14 `rock-shared`, 42 `rockc`, and 34 `rockup` tests; the one ignored
  `rock-lib` doc test and the `rock-shared` doc test retained their expected
  results.
- Clippy passed with zero errors but was not warning-clean: 364 emitted warning
  entries across targets, including 144 duplicate diagnostics in the
  `rock-lib` lib-test target. `cargo fmt --all --check` passed. The residue
  inventory was refreshed after the final line shifts: 46 total matches, exactly
  18 production MIR/direct-operation matches, 24 test fixtures, and 4 historical
  semantic-audit strings, with no unclassified entries. `git diff --check` passed.
- The refreshed tracked-Rust forbidden-token residue search found zero executable, type,
  or serialized matches for `SelectedOrigin::BuiltinIndex`,
  `HirSelectedMethodTarget::BuiltinIndex`,
  `HirMethodCallTarget::builtin_index`, `is_builtin_index`,
  `select_builtin_index_method`, `builtin_index_output`,
  `has_builtin_index_impl`, `builtin_index_output_id`,
  `builtin_index_trait_ids`, `builtin_index_projection`,
  `HirExprKind::Index`, and
  `SerializedHirSelectedMethodTarget::BuiltinIndex`. The only related broader
  matches are historical forbidden-token strings in semantic-identity test
  assertions. Explicit tracked-Rust checks for `HirExprKindFor::Index` and
  `SerializedHirExpr::Index` also returned zero matches. No new index intrinsic
  or `ptr[i]` recursion exists in stdlib index bodies. The complete ledger is
  `/tmp/opencode/task8-index-closure-ledger.md`.

Acceptance criteria:

- No implicit type-name fallback for `Array`/slice/pointer/Vec index trait
  behavior; all selected index semantics are owned by explicit typed stdlib
  trait implementations.

### Step 9: Replace AST Receiver Mode Past Lowering

Status: Complete as of 2026-07-26. Parser and AST syntax retain
`SelfReceiverMode`, while collection and lowering convert it into the semantic
`types::ReceiverMode` before constructing HIR. HIR, accepted HIR, language-item
validation, selection, inference, mono, DCE, MIR, products, and artifact loading
now use only the semantic mode. MIR ABI pass modes remain a separate backend
concept. Product function, body, signature, method-call, and Try rows serialize
the semantic mode, and the artifact format advanced from `39` to `40` in both
`rock-lib` and `rock-shared`.

Validation evidence:

- Focused product (`91`), artifact (`176`), selection-service (`46`), mono
  (`109` library plus `7` integration), MIR-builder (`86`), lower (`378`
  library plus `4` integration), HIR language-item (`51`), and semantic identity
  audit (`40`) tests passed.
- `cargo test -p rock-lib --test integration` passed `533/533`.
- `cargo test -p rock-lib` passed `2,063` library tests, `533` integration
  tests, and one auxiliary parser test; one doc test remained intentionally
  ignored.
- `cargo clippy --workspace --all-targets` completed with no errors and retained
  the repository's existing warnings. `cargo fmt --all --check` and
  `git diff --check` passed.
- The final source inventory found no `crate::ast::SelfReceiverMode` references
  in HIR, types, inference, selection, mono, DCE, MIR, codegen, products, or
  artifact code. Remaining references are confined to AST/parser/formatter input
  handling and explicit collection/lowering conversion boundaries and fixtures.

Goal:

- Clean AST/HIR/MIR layering.

Tasks:

- Introduce semantic receiver mode type.
- Convert AST `SelfReceiverMode` during lowering/collection.
- Update HIR, products, mono, selection, MIR to use semantic mode.
- Keep AST enum inside parser/AST/lowering only.

Acceptance criteria:

- No `crate::ast::SelfReceiverMode` references in mono, MIR, codegen, products, or artifact interfaces.

### Step 10: Remove Or Constrain Lenient Type Finalization

Status: Complete as of 2026-08-02. Lenient finalization APIs are deleted,
numeric literal evidence is the only inference defaulting authority,
`AcceptedHirProgram` validates the complete HIR type inventory, product encoding
rejects `TypeVar` and `Type::Error`, artifact loading rejects both sentinels, and
the artifact format is `41` in both `rock-lib` and `rock-shared`.

The final MIR barrier now rejects `Ty::TypeVar`, `Ty::Error`, and runtime uses of
`Ty::Generic`. Generic nominal-layout templates remain valid only when every
generic ID belongs to that layout's declared parameter list. Codegen no longer
maps unresolved/error/residual-projection types to integer placeholders, treats
unresolved/error drops as no-ops, uses `i8` as a `PtrOffset` element fallback,
or treats type variables as signed integers. Missing impl receiver authority is
a hard accepted-HIR invariant instead of producing `Type::Error`.

Goal:

- Ensure accepted programs have fully resolved types.

Tasks:

- Distinguish own vs external impls/methods by IDs, not by string maps.
- Validate loaded artifact types at load time.
- Remove global lenient finalization or restrict it to explicitly validated artifact boundaries.
- Add invariant check before mono: no `TypeVar`, no `Type::Error` in accepted HIR.

Acceptance criteria:

- Accepted compilation cannot silently default unresolved types to `I64` except for intentional language numeric defaulting with diagnostics-compatible rules.

### Step 11: Tighten MIR Codegen Completeness

Status: Complete as of 2026-08-01. Final MIR now represents only backend-supported
control flow and assertions. Rock has no exception or stack-unwind semantics, so
`Terminator::Call.cleanup` and `Terminator::Drop.unwind` are deleted rather than
retained as permanently invalid optional edges. Normal explicit destruction still
uses `Drop.target`, and statement cleanup marking remains separate. Bounds checks
are the only MIR assertion emitted by the builder, so the unused `NonNull` variant
and its late codegen fallback are deleted.

Callable constants remain values only in typed callable positions and use the
existing dedicated materialization path. `TypeId` constants are compile-time
intrinsic metadata: agreement validates every embedded ID and permits metadata
only for the complete typed forms accepted by `SizeOf` and array `ArrayLen`.
Misplaced callable or type metadata is rejected by the pre-codegen agreement
barrier. Scalar constant lowering now reports precise invariant violations if
either metadata form somehow bypasses that barrier; production codegen contains
no broad `not implemented yet` path.

Validation evidence:

- Focused suites passed: MIR agreement (`34`), MIR builder (`86`), MIR dataflow
  (`25`), MIR borrow checking (`67`), and codegen (`109` library tests plus the
  matching integration regression).
- `cargo test -p rock-lib --test integration` passed `537/537`.
- The saved full `cargo test -p rock-lib` gate passed `2,073` library tests,
  `537` integration tests, and one auxiliary parser test; one doc test remained
  intentionally ignored.
- `cargo clippy --workspace --all-targets` completed with zero errors and retained
  the repository's existing warnings; no warning referenced Task 11 code.
  `cargo fmt --all --check` and `git diff --check` passed.
- Residue checks found no call-cleanup or drop-unwind fields, no
  `MirAssertKind::NonNull`, no removed agreement counters, no combined
  callable/type-ID generic fallback, and no `not implemented yet` path under
  production codegen.

Goal:

- Replace late “not implemented yet” failures with supported codegen or earlier validation.

Tasks:

- Implement or forbid non-bounds-check assertions.
- Implement or forbid call cleanup/unwind terminators.
- Implement or assert-unreachable callable/type-id constants in generic constant codegen.

Acceptance criteria:

- Production codegen has no broad “not implemented yet” paths for representable MIR.

### Step 12: Replace Unchecked Test Scaffolds With Contract Fixtures

Status: Complete as of 2026-08-02. The test-only
`compile_mir_program_unchecked_for_test` entry point is deleted, and the private
post-declaration body compiler no longer carries unchecked naming. MIR LLVM
whole-program tests now call the production `CodeGen::compile_program_from_mir`
entry point. Successful fixtures are built through an explicit
`MirProgramFixtureBuilder` or checked by `compile_valid_mir_program`; both require
a clean MIR agreement report before LLVM lowering. Intentionally malformed
contracts remain only in named rejection tests and still run through production
codegen validation.

The stricter fixture barrier exposed two projection tests that supplied concrete
projection outputs but omitted their projection-base struct layouts; both now
provide the required `DefId`-keyed nominal layouts. MIR agreement now also counts
projection types that remain unresolved after contract normalization, including
projection `TypeId`s used only as cast targets or intrinsic metadata. This closes
the prior gap where an incomplete projection fixture could pass agreement and
panic during LLVM type lowering.

Validation evidence:

- `cargo test -p rock-lib mir::agreement -- --nocapture`: `36` passed.
- `cargo test -p rock-lib codegen::mir_llvm -- --nocapture`: `48` passed.
- `cargo test -p rock-lib codegen -- --nocapture`: `108` library tests and the
  matching integration regression passed.
- `cargo test -p rock-lib --test integration`: `537` passed.
- The saved full `cargo test -p rock-lib` gate passed `2,074` library tests,
  `537` integration tests, and one auxiliary parser test; one doc test remained
  intentionally ignored.
- `cargo clippy --workspace --all-targets` completed with zero errors and the
  repository's existing warnings. The only diagnostics in the changed MIR LLVM
  helper area are the two pre-existing `too_many_arguments` warnings.
- `cargo fmt --all --check` and `git diff --check` passed.
- Residue scans found no `compile_mir_program_unchecked_for_test`,
  `compile_mir_program_bodies_unchecked`, test contract sync/repair helpers,
  legacy metadata adapters, or default backend-contract field fixtures in MIR
  LLVM tests. Independent final review reported no remaining codegen-relevant
  `TypeId` or successful whole-program fixture bypass.

Goal:

- Make tests exercise production invariants.

Tasks:

- Add contract fixture builders.
- Convert MIR LLVM tests to construct complete contracts.
- Delete unchecked compile path where feasible.
- Delete legacy adapter tests after adapter removal.

Acceptance criteria:

- Tests fail if MIR lacks contract callables/layouts/projections/drop glue required by codegen.

### Step 13: Add Audit Tests To Prevent Regression

Status: Complete as of 2026-08-02. The semantic identity audit now applies six
additional production-source barriers at the ownership boundaries established by
the cleanup. Repository-wide checks reject the deleted MIR backend sidecar and
hardcoded `Sized` trait identity strings. Phase-scoped checks reject HIR display
alias consumption in mono/MIR/codegen, method rediscovery by type-name helpers in
mono/MIR, string-to-item-ID recovery after resolution, and AST receiver syntax in
HIR or later executable phases. Production scans strip `#[cfg(test)]` items and
exclude standalone test modules, so fixtures may describe rejected historical
forms without weakening executable-source coverage.

Existing audits remain the direct barriers for product link-record symbol
ownership and codegen/HIR separation. The final closure added a 47th audit
covering the newly discovered boundaries: artifact
body payload maps must be `DefId` keyed and display-name gates must remain absent;
final `MirCallable` must contain only a canonical key; backend-forbidden type
states must be rejected; and codegen repair fallbacks must remain absent. Together
the 47 focused audits cover every suggested Step 13 prohibition while preserving
names in collection/resolution and receiver syntax in syntax-facing phases.

Validation evidence:

- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`: `47` passed.
- The definitive saved `cargo test -p rock-lib` gate passed `2,082` library tests,
  `537` integration tests, and one auxiliary parser test; one doc test remained
  intentionally ignored.
- `cargo clippy --workspace --all-targets` completed with zero errors and the
  repository's existing warnings. No diagnostic referenced
  `semantic_identity_audit.rs`.
- `cargo fmt --all --check` and `git diff --check` passed.

Goal:

- Make the cleanup durable.

Suggested audit forbiddance checks:

- `MirBackendMetadata` absent from production source.
- `from_legacy_metadata` absent from all source.
- `backend_metadata` absent from `MirProgram` and codegen/agreement.
- `ProductIdentityTable.backend_symbols` absent.
- Codegen does not import HIR executable types.
- Codegen does not call HIR display alias helpers.
- Mono/MIR builder do not call method lookup by type names.
- Production code outside collect/resolution does not call `resolve_item_id` from strings for semantic recovery.
- No production string checks for `"stdlib::sized::Sized"` or `"Sized"` as compiler-recognized trait identity.
- No `crate::ast::SelfReceiverMode` outside AST/parser/lower boundary after receiver-mode cleanup.

## 2026-08-02 Whole-Codebase Final Closure Swipe

This pass treated all prior completion text as untrusted, found four remaining
boundaries, implemented them, and rescanned the current working tree across
production code, test fixtures, artifact boundaries, and the public CLI. It also
ran the 47 semantic identity audits and the complete `rock-lib` suite.

Task verdicts:

1. Step 1 passes its scoped criteria: there is one `MirBackendContract`, with no
   legacy metadata type or adapter residue.
2. Step 2 passes: production codegen layout and generic substitution state is
   `DefId` keyed.
3. Step 3 passes: serialized backend symbols are link-record owned and loaded
   symbols live in `ExternCrateLink`.
4. Step 4 passes: HIR/product function rows have no hybrid `qualified_name`
   backend identity.
5. Step 5 passes its method-selection criteria: accepted method authority is
   ID based and final production method calls are materialized instances.
6. Step 6 passes: artifact interfaces cannot be filtered by display-name
   availability, and reusable accepted payloads remain `DefId` keyed through
   `ArtifactCrossCrateHir` and loaded body providers.
7. Step 7 passes: compiler-recognized protocols use marker-bound language-item
   IDs; remaining protocol-name strings found by the swipe are test fixtures,
   syntax, or diagnostics.
8. Step 8 passes: source indexing is selected through ordinary marked `Index`
   and `IndexMut` impl authority, with no targetless builtin-index fallback.
9. Step 9 passes: AST `SelfReceiverMode` is confined to collection/lowering and
   syntax-facing code; executable downstream phases use semantic receiver mode.
10. Step 10 passes: strict HIR/artifact barriers are followed by MIR agreement
    rejection of backend-forbidden type states, with no codegen repair arms.
11. Step 11 passes its scoped criteria: representable final MIR has no unwind
    edges, unsupported assertion variant, or broad `not implemented yet`
    codegen path.
12. Step 12 passes: successful MIR LLVM fixtures use production validation and
    malformed fixtures use the production rejection path.
13. Step 13 passes and now protects the Step 6 artifact boundary, Step 10 backend
    type barrier, resolved-only callable model, and prior token prohibitions.

Original audit findings not assigned a numbered implementation step are closed:

- Final MIR has only `MirCallable::Resolved(MirCallableKey)`; unresolved variants
  and their downstream interpretation/error paths are deleted.
- The public unimplemented `rock test` command is removed and rejected by Clap.

Fresh verification evidence:

- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`: `47` passed.
- `cargo test -p rock-lib`: `2,082` library tests, `537` integration tests, and
  one auxiliary test passed; one doc test was intentionally ignored.
- Focused gates passed for MIR agreement (`37`), backend contract (`17`), borrow
  checking (`67`), mono methods (`21`), codegen (`109` library plus `1`
  integration), the removed CLI command (`1`), generic artifact compilation,
  and stdlib artifact compilation.
- `cargo clippy --workspace --all-targets` completed without errors and retained
  the repository's existing warnings. `cargo fmt --all --check` and
  `git diff --check` passed.
- Direct residue and ownership scans covered all tracked Rust source under
  `lib/src`, plus `rock`, `rock-shared`, `rockc`, and `rockup` where the audited
  contracts cross crate boundaries.

## Files Most Likely To Change

Backend/MIR cleanup:

- `lib/src/mir/mod.rs`
- `lib/src/mir/backend_contract.rs`
- `lib/src/mir/builder/mod.rs`
- `lib/src/mir/agreement.rs`
- `lib/src/mir/identity.rs`
- `lib/src/codegen/mod.rs`
- `lib/src/codegen/types.rs`
- `lib/src/codegen/mir_llvm/mod.rs`
- `lib/src/codegen/mir_llvm/operand.rs`
- `lib/src/codegen/mir_llvm/assert.rs`
- `lib/src/codegen/mir_llvm/terminator.rs`

Collect/lower/HIR identity cleanup:

- `lib/src/collect/mod.rs`
- `lib/src/collect/headers.rs`
- `lib/src/collect/context.rs`
- `lib/src/collect/resolver.rs`
- `lib/src/lower/items.rs`
- `lib/src/lower/mod.rs`
- `lib/src/lower/program.rs`
- `lib/src/lower/resolution.rs`
- `lib/src/lower/collect/traits.rs`
- `lib/src/lower/traits/conformance.rs`
- `lib/src/lower/bodies.rs`
- `lib/src/lower/function.rs`
- `lib/src/hir/mod.rs`

Selection/mono cleanup:

- `lib/src/selection/legacy_lookup.rs`
- `lib/src/selection/service.rs`
- `lib/src/selection/types.rs`
- `lib/src/selection/matching.rs`
- `lib/src/mono/mod.rs`
- `lib/src/mono/process.rs`
- `lib/src/mono/methods.rs`
- `lib/src/mono/external.rs`
- `lib/src/mono/registry.rs`

Products/artifacts cleanup:

- `lib/src/products.rs`
- `lib/src/products/type_table.rs`
- `lib/src/crate_artifact/load.rs`
- `lib/src/crate_artifact/types.rs`
- `lib/src/crate_system/extern_store.rs`
- `rock-shared/src/sysroot.rs` only if artifact format constants need updates.

Driver/CLI cleanup:

- `lib/src/lib.rs`
- `rock/src/cli.rs`

Audit tests:

- `lib/src/semantic_identity_audit.rs`

## Suggested Spec Structure For Next Session

A future spec should probably be split into sections like this:

1. Problem statement.
   - Current PR removed direct HIR codegen but left transitional backend/identity systems.

2. Non-goals.
   - Do not preserve artifact compatibility.
   - Do not introduce compiler-owned stdlib loading.
   - Do not move compiler logic into CLI crates.
   - Do not redesign parser syntax unless necessary.

3. Target pipeline.
   - Use the phase model above.

4. Data ownership contracts.
   - Resolver owns names.
   - HIR owns IDs and typed AST-lowered structure.
   - Selection owns call authority.
   - Mono owns instances and symbols.
   - MIR owns backend contract.
   - Products own serialized interface/body/link data.

5. Migration strategy.
   - Start with backend metadata removal.
   - Then artifact symbols.
   - Then qualified-name split.
   - Then selection/mono/lower identity cleanup.

6. Invariants and audit tests.
   - Define forbidden symbols/patterns.
   - Add source-level tests to enforce them.

7. Verification plan.
   - Focused backend tests.
   - Product/artifact tests.
   - Selection/mono tests.
   - Integration tests.
   - Full `cargo test -p rock-lib`.

## Suggested Initial Implementation Plan Seed

The most practical first implementation slice is not to attempt every cleanup at once. Start with the backend contract cleanup because it is the clearest and most isolated old system.

Initial slice:

1. Build `MirBackendContract` directly in `MirBuilder`.
2. Remove `MirBackendMetadata` from `MirProgram`.
3. Remove `from_legacy_metadata*`.
4. Make codegen consume contract-only layouts.
5. Make agreement consume contract-only data.
6. Convert tests to direct contract fixtures.
7. Add audit tests forbidding the removed legacy system.

Expected hard parts:

- Preserving method receiver ABI/pass modes without the legacy adapter.
- Preserving projection output validation without metadata projection impl sidecars.
- Preserving drop glue mapping and artifact exports.
- Updating many tests that used `backend_contract: Default::default()` or `MirBackendMetadata` fixtures.

The follow-up slices can then address product link symbols, `qualified_name`, selection authority, and lower/collect ID-keying.
