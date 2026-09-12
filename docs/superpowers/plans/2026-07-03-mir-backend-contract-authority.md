# MIR Backend Contract Authority Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete the legacy `MirBackendMetadata` sidecar and make `MirBackendContract` the only backend authority carried by final MIR.

**Architecture:** `MirBuilder` constructs a complete `MirBackendContract` directly from `MonomorphizedProgram`, `MirInstanceBodies`, final MIR functions, and `TypeContext`. Borrow checking, MIR agreement, codegen, product link attachment, and tests consume the contract only. No adapter, fallback reconstruction, metadata sync helper, or name/display side channel may remain as a backend-contract authority after this slice.

**Tech Stack:** Rust 2021, `rock-lib`, MIR builder/agreement/borrowck, LLVM codegen, `MirBackendContract`, `DefId`, `InstanceId`, `TypeId`, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Source Audit Item

This plan implements `CLEAN_SLATE_COMPILER_AUDIT.md` recommended Step 1, `Remove MirBackendMetadata And Adapter Layer`.

Do not mark that audit item complete until every validation rule in this plan passes. When it does pass, update `CLEAN_SLATE_COMPILER_AUDIT.md` in the Step 1 section with a short `Status: Complete` note and the exact validation evidence. Keep later audit steps pending unless they were separately completed and verified.

## Problem Statement

The HIR-to-LLVM backend has been removed, but final MIR still carries two backend authorities:

- `MirProgram.backend_metadata`, a broad sidecar with externs, instance declarations, layouts, trait/projection data, drop glue, builtin index trait IDs, and product link candidates.
- `MirProgram.backend_contract`, the intended clean backend contract.

The current production path builds metadata first, converts it through `MirBackendContract::from_legacy_metadata_with_type_context`, stores both objects on `MirProgram`, then lets agreement, borrowck, and codegen read both. That is not a clean-slate compiler boundary. It allows stale metadata to disagree with the contract, lets tests pass by syncing fixture state after construction, and preserves the old mental model that backend facts live in a loose sidecar.

## Current Residue Inventory

The implementation must remove these production-active residues:

- `lib/src/mir/mod.rs`: `MirProgram.backend_metadata` and public `MirBackendMetadata`.
- `lib/src/mir/backend_contract.rs`: `from_legacy_metadata`, `from_legacy_metadata_with_type_context`, `from_legacy_metadata_inner`, `legacy_generic_param_ids`, `legacy_instance_signature`, and `legacy_receiver_pass_mode`.
- `lib/src/mir/builder/mod.rs`: `backend_metadata_for_program` as the normal path and any flow that constructs contract data by first building metadata.
- `lib/src/mir/agreement.rs`: `validate_backend_metadata_type_ids`, `validate_backend_metadata_contract`, and `BackendContract::new` reads of `program.backend_metadata`.
- `lib/src/mir/borrowck/mod.rs`: `MoveValidationContext.metadata` and any borrow-check lookup through metadata.
- `lib/src/codegen/mod.rs`: `CodeGen::register_mir_nominal_layouts` requiring metadata, `register_mir_nominal_layout_names`, and production layout registration from metadata.
- `lib/src/codegen/mir_llvm/mod.rs`: test helpers that synthesize or repair backend contract state from old side tables, especially `sync_test_backend_contract`.
- Tests in `lib/src/**` that instantiate `MirBackendMetadata`, pass `backend_metadata: Default::default()`, or call `MirBackendContract::from_legacy_metadata*`.

## Non-Goals

- Do not redesign method selection authority in this slice.
- Do not split `HirFunction.qualified_name` in this slice.
- Do not remove product identity-table backend symbols in this slice, except where product link attachment already reads contract artifact exports.
- Do not rewrite collect/lower declaration ownership in this slice.
- Do not change source-language semantics, stdlib semantics, or parser behavior.
- Do not add backward-compatibility shims for the deleted metadata model.

## Target Architecture

### Final MIR Shape

`MirProgram` must contain exactly one backend authority:

```rust
pub struct MirProgram {
    pub functions: BTreeMap<MirFunctionId, MirFunction>,
    pub type_context: TypeContext,
    pub backend_contract: MirBackendContract,
}
```

There must be no `backend_metadata` field, no equivalent renamed sidecar, and no optional compatibility field that can be used by agreement, borrowck, codegen, or tests.

### Direct Contract Construction

`MirBuilder::build_monomorphized_with_instance_bodies` must build the contract directly. A small private builder helper is acceptable if it writes into `MirBackendContract` and does not escape final MIR, for example:

```rust
let mut backend_contract = MirBackendContract::default();
Self::populate_backend_contract_externs(&mut backend_contract, program, &mut type_context);
Self::populate_backend_contract_instances(&mut backend_contract, program, bodies, &functions, &mut type_context);
Self::populate_backend_contract_nominal_layouts(&mut backend_contract, program, &mut type_context);
Self::populate_backend_contract_projection_outputs(&mut backend_contract, program, bodies, &functions, &mut type_context);
Self::populate_backend_contract_drop_glue(&mut backend_contract, program, bodies, &functions, &mut type_context);
Self::populate_backend_contract_artifact_exports(&mut backend_contract, program, bodies);
Self::populate_backend_contract_function_bodies(&mut backend_contract, &functions, &type_context);
```

The exact helper names can differ, but the data flow cannot. Contract facts must be inserted into `MirBackendContract` directly, not through an intermediate public metadata aggregate.

### Contract-Owned Facts

After this slice, `MirBackendContract` owns all backend facts needed by consumers:

- `callables`: declarations for functions, externs, instances, closures, intrinsics, and runtime helpers.
- `function_bodies`: body-to-callable mapping for local MIR bodies.
- `nominal_layouts`: struct/enum layouts keyed by canonical `DefId`.
- `projection_outputs`: concrete associated-type projection outputs keyed by `MirProjectionKey`.
- `drop_glue`: cleanup/drop entry points keyed by concrete `TypeId`.
- `runtime_requirements`: runtime helper requirements.
- `artifact_exports`: concrete body/link export candidates.

If agreement still needs trait-level projection knowledge that cannot be derived from `projection_outputs`, add an explicit contract field such as `projection_traits: BTreeSet<DefId>`. Do not keep or reintroduce `trait_members`, `projection_impls`, or `builtin_index_trait_ids` as a sidecar outside the contract.

### ABI And Receiver Pass Modes

The receiver pass-mode logic currently hidden behind `legacy_instance_signature` must be renamed and owned by direct contract construction. The rule remains semantic:

- Non-reference receiver: `MirPassMode::Direct`.
- Thin reference receiver: `MirPassMode::Pointer`.
- Slice/fat-pointer-shaped receiver: `MirPassMode::FatDirect`.

This logic must not be called `legacy_*`, must not depend on metadata, and must use `TypeContext` plus `TypeLayout` directly.

### Codegen Contract Boundary

`CodeGen::compile_program_from_mir` and declaration preparation must take layout, callable, projection, drop, runtime, and artifact facts from `mir.backend_contract` only.

Production codegen must not:

- Accept `MirBackendMetadata`.
- Register nominal layouts from metadata names.
- Fill name-keyed layout maps from backend metadata.
- Repair a missing contract by inspecting old side tables.

Name-keyed layout tables are Step 2 cleanup if they remain for currently unmigrated expression helpers, but they cannot be populated from `MirBackendMetadata` or used as the production MIR contract source in this slice.

### MIR Agreement Contract Boundary

MIR agreement must validate only:

- `MirProgram.functions`.
- `MirProgram.type_context`.
- `MirProgram.backend_contract`.

All metadata-specific validation must be deleted or rewritten as contract validation. Projection-trait checks must derive their allowed traits from contract-owned data.

### Borrow Checker Contract Boundary

Borrow checking must remove `MirBackendMetadata` from imports, context structs, constructor parameters, and test fixtures. Any type cleanup, projection normalization, drop-obligation, or callee summary logic must use `MirBackendContract` and `TypeContext`.

### Test Contract Boundary

Tests must construct realistic `MirBackendContract` fixtures. Empty/default contracts are allowed only in tests whose name and assertion are explicitly about missing or empty contract behavior.

These are forbidden in tests after this slice:

- `MirBackendMetadata` construction.
- `backend_metadata: Default::default()`.
- `MirBackendContract::from_legacy_metadata*`.
- `sync_test_backend_contract` or any helper that silently fills missing contract facts.
- Metadata adapter tests. Replace them with direct contract builder/validation tests.

## Hard Validation Gate: Zero Legacy Residue

This slice is not complete if any legacy backend metadata residue remains. No partial compatibility is allowed.

The forbidden strings must be absent from all Rust source under `lib/src`. Documentation may mention the old system while the cleanup is being planned or while the completion evidence is recorded.

Forbidden Rust-source patterns:

```text
MirBackendMetadata
backend_metadata
from_legacy_metadata
legacy_generic_param_ids
legacy_instance_signature
legacy_receiver_pass_mode
backend_metadata_for_program
validate_backend_metadata_type_ids
validate_backend_metadata_contract
register_mir_nominal_layout_names
sync_test_backend_contract
```

Manual source check after implementation:

```bash
rg -n "MirBackendMetadata|backend_metadata|from_legacy_metadata|legacy_generic_param_ids|legacy_instance_signature|legacy_receiver_pass_mode|backend_metadata_for_program|validate_backend_metadata_type_ids|validate_backend_metadata_contract|register_mir_nominal_layout_names|sync_test_backend_contract" lib/src
```

Expected result: no matches.

Do not add a permanent source-content audit test for this rule. The residue check is a manual implementation validation step: run the search after deleting the legacy items, inspect any matches, and do not mark the audit item complete while matches remain.

## File Structure

- Modify `lib/src/mir/mod.rs`: remove `MirBackendMetadata` from `MirProgram`; delete the public metadata aggregate and any no-longer-used metadata row structs.
- Modify `lib/src/mir/backend_contract.rs`: delete adapter constructors and legacy helpers; keep validation and projection normalization contract-native.
- Modify `lib/src/mir/builder/mod.rs`: replace metadata construction with direct `MirBackendContract` construction.
- Modify `lib/src/mir/agreement.rs`: remove metadata validation and projection-trait recovery from metadata.
- Modify `lib/src/mir/borrowck/mod.rs`: remove metadata from borrow-check contexts and tests.
- Modify `lib/src/codegen/mod.rs`: make MIR declaration preparation and layout registration contract-only.
- Modify `lib/src/codegen/types.rs` and `lib/src/codegen/mir_llvm/mod.rs` as needed for contract-native test fixtures and layout lookup.
- Modify `CLEAN_SLATE_COMPILER_AUDIT.md`: only after all validation passes, mark Step 1 complete with evidence.

## Task 1: Establish The Manual Zero-Residue Baseline

**Files:**
- Inspect: `lib/src/**`

- [ ] **Step 1: Run the baseline residue search**

Run:

```bash
rg -n "MirBackendMetadata|backend_metadata|from_legacy_metadata|legacy_generic_param_ids|legacy_instance_signature|legacy_receiver_pass_mode|backend_metadata_for_program|validate_backend_metadata_type_ids|validate_backend_metadata_contract|register_mir_nominal_layout_names|sync_test_backend_contract" lib/src
```

Expected before implementation: matches exist. Use this output as the deletion inventory.

- [ ] **Step 2: Do not create a permanent source-scanning test**

This cleanup is validated by manual source search plus compile/test gates. Do not add a `semantic_identity_audit` test that only asserts these strings are absent from current source files.

- [ ] **Step 3: Keep the forbidden pattern list unchanged**

Every later manual residue check must use the same pattern list:

```bash
MirBackendMetadata|backend_metadata|from_legacy_metadata|legacy_generic_param_ids|legacy_instance_signature|legacy_receiver_pass_mode|backend_metadata_for_program|validate_backend_metadata_type_ids|validate_backend_metadata_contract|register_mir_nominal_layout_names|sync_test_backend_contract
```

## Task 2: Remove Metadata From `MirProgram`

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: every `MirProgram { ... }` fixture under `lib/src/**`

- [ ] **Step 1: Delete the final-MIR sidecar field**

Change `MirProgram` to store only `functions`, `type_context`, and `backend_contract`.

- [ ] **Step 2: Remove `backend_metadata` from all `MirProgram` construction sites**

For each fixture, either provide a realistic `backend_contract` or keep `MirBackendContract::default()` only when the test explicitly exercises empty-contract behavior.

- [ ] **Step 3: Delete `MirBackendMetadata` if no longer referenced**

Do not keep a deprecated alias, hidden compatibility struct, or renamed equivalent. Temporary construction records may be private to `mir::builder` only if they cannot escape final MIR and are not consumed by agreement, borrowck, or codegen.

## Task 3: Build `MirBackendContract` Directly

**Files:**
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/backend_contract.rs`

- [ ] **Step 1: Replace `backend_metadata_for_program` with direct contract construction**

`build_monomorphized_with_instance_bodies` must not call a metadata builder or adapter. It must initialize `MirBackendContract::default()` and populate each contract field directly.

- [ ] **Step 2: Move adapter logic into direct, non-legacy helpers**

Preserve required behavior for extern declarations, instance callables, local body mappings, nominal layouts, projection outputs, drop glue, artifact exports, and runtime helper requirements. Helper names and comments must describe current contract construction, not legacy migration.

- [ ] **Step 3: Preserve ABI/pass-mode behavior without legacy naming**

Recreate the receiver pass-mode rule as current contract ABI logic. Delete `legacy_instance_signature` and `legacy_receiver_pass_mode`.

- [ ] **Step 4: Delete adapter functions and tests**

Remove `from_legacy_metadata*` and the adapter test block. Replace useful coverage with tests that assert direct contract construction contains the required callables, layouts, projections, drop glue, and artifact exports.

## Task 4: Move Agreement, Borrowck, And Codegen To Contract-Only Reads

**Files:**
- Modify: `lib/src/mir/agreement.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/types.rs` if layout/projection consumers require adjustment

- [ ] **Step 1: Rewrite MIR agreement**

Delete metadata validators. `BackendContract::new` must derive all layout and projection knowledge from `program.backend_contract`.

- [ ] **Step 2: Rewrite borrowck contexts**

Remove metadata from `MoveValidationContext` and any helper signatures. Drop/projection/cleanup behavior must use `backend_contract` plus `type_context`.

- [ ] **Step 3: Rewrite codegen layout registration**

`register_mir_nominal_layouts` must take only `&MirBackendContract`. Delete `register_mir_nominal_layout_names` and all production calls that populate layout data from metadata.

- [ ] **Step 4: Remove metadata imports**

Clean imports in touched modules so `MirBackendMetadata` is not imported anywhere in Rust source.

## Task 5: Convert Tests To Contract-Native Fixtures

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/mir_llvm/mod.rs`
- Modify: `lib/src/mir/backend_contract.rs`
- Modify: `lib/src/mir/agreement.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: any other failing `lib/src/**` tests

- [ ] **Step 1: Replace metadata fixtures with contract fixtures**

Tests that used metadata rows must construct `MirBackendContract` directly. Each callable used by a MIR body must have a matching `MirCallableDecl`, and each local body must have a `function_bodies` entry.

- [ ] **Step 2: Delete metadata sync helpers**

Delete `sync_test_backend_contract`. If `compile_mir_program_unchecked_for_test` remains temporarily, it must not create, repair, or sync missing contract facts. It may only compile a `MirProgram` whose contract was explicitly provided by the test.

- [ ] **Step 3: Make default contracts intentional**

Any remaining `backend_contract: Default::default()` fixture must be in a test whose name and assertions show that an empty contract is the behavior under test.

## Task 6: Mark The Audit Item Complete Only After Validation

**Files:**
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md`

- [ ] **Step 1: Run all validation commands**

Do not edit the audit status until every command in `Final Validation` passes.

- [ ] **Step 2: Mark Step 1 complete in the audit document**

In the `Recommended Refactor Order` section, update the Step 1 entry with a concise completion note, for example:

```markdown
Status: Complete as of 2026-07-03. Validation: zero Rust-source matches for the legacy metadata forbidden patterns under `lib/src`; `cargo test -p rock-lib codegen -- --nocapture`; `cargo test -p rock-lib codegen::mir_llvm -- --nocapture`; `cargo test -p rock-lib mir::agreement -- --nocapture`; `cargo test -p rock-lib mir::builder -- --nocapture`; `cargo test -p rock-lib semantic_identity_audit -- --nocapture`; `cargo test -p rock-lib --test integration`.
```

Keep the original explanation of why the item mattered. Do not mark Step 2 or later complete in the same edit unless they have their own passing validation evidence.

## Final Validation

Run these in order, stopping on the first failure. The first command wraps `rg` because `rg` returns exit status 1 when no matches are found, and no matches is the expected result here.

```bash
bash -lc 'rg -n "MirBackendMetadata|backend_metadata|from_legacy_metadata|legacy_generic_param_ids|legacy_instance_signature|legacy_receiver_pass_mode|backend_metadata_for_program|validate_backend_metadata_type_ids|validate_backend_metadata_contract|register_mir_nominal_layout_names|sync_test_backend_contract" lib/src; status=$?; test "$status" -eq 1'
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib codegen -- --nocapture
cargo test -p rock-lib codegen::mir_llvm -- --nocapture
cargo test -p rock-lib --test integration
cargo fmt --all --check
git diff --check
```

Expected result for the `rg` wrapper: no output and wrapper exit status 0, proving the underlying `rg` returned 1 because there were no matches. All Cargo commands must pass. `git diff --check` must report no whitespace errors.

## Completion Criteria

This item is complete only when all criteria are true:

- `MirProgram` has no metadata sidecar.
- `MirBackendMetadata` and the adapter functions are deleted from Rust implementation code.
- MIR builder constructs `MirBackendContract` directly.
- Agreement, borrowck, and codegen consume contract data only.
- Tests no longer use metadata fixtures or metadata-to-contract conversion.
- No fallback helper can recreate or repair contract facts from old side tables.
- The manual zero-residue source check returns no matches.
- `CLEAN_SLATE_COMPILER_AUDIT.md` marks Step 1 complete with validation evidence.

## Self-Review Notes

- Scope is intentionally limited to MIR backend metadata authority. Product link symbol ownership, `qualified_name`, method selection authority, collect/lower ID-keying, and compiler-recognized trait language items remain separate audit items.
- The hard validation gate forbids transitional names and compatibility helpers, including test-only sync helpers that would hide incomplete contract fixtures.
- The plan does not require commits. Follow the current repo instruction: do not commit, stage, push, or mutate VCS state unless the user explicitly asks.
