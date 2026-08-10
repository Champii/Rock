# Step 1 Backend Contract Finalization Design

## Goal

Finish Clean-Slate Compiler Audit Step 1 by constructing
`MirBackendContract` directly from authoritative mono, HIR, and MIR inputs and
by requiring complete backend-contract validation in every MIR builder test.

The change must preserve all completed Step 2-5 architecture. It must not
restore name-keyed layout lookup, duplicate product symbol authority, HIR
backend-name ownership, or downstream method-selection reconstruction.

## Current Gaps

`MirProgram` already stores only `MirBackendContract`, but
`MirBuilder::backend_contract_for_program` still creates parallel staging rows
before translating them into the contract. These include extern, instance,
nominal-layout, projection, drop-glue, and product-link records. The nominal
layout staging also collects display aliases that are discarded after the
canonical row is inserted.

MIR builder tests use `check_mir_runtime_agreement_for_mir_only`, which disables
full backend-contract validation. Helpers can therefore combine real MIR with
an empty or incomplete contract and still report clean agreement when the
missing facts are nominal layouts or other disabled contract requirements.

## Direct Contract Construction

`backend_contract_for_program` will create one `MirBackendContract` at the start
and populate it directly while traversing authoritative inputs.

The construction flow will be:

1. Insert extern callable declarations directly from accepted HIR externs.
2. Insert instance callable declarations and local function-body mappings
   directly from `MonomorphizedProgram.instances` and `MirInstanceBodies`.
3. Insert one canonical nominal layout per struct or enum `DefId` directly from
   accepted HIR declarations. Display aliases will not be traversed.
4. Derive projection traits and projection outputs from canonical impl
   authority and the concrete `TypeId` uses visible in MIR and callable/layout
   signatures.
5. Insert generated drop glue directly from the generated method-instance map.
6. Insert artifact exports directly from instance origins, substitutions,
   object/body ownership, and backend symbols already owned by mono instance
   records.
7. Add nested/local function declarations through the existing function-body
   completion path, then return the completed contract.

Temporary local variables such as a callable key, signature, or projection
candidate are allowed. Parallel collections or reusable row structs that mirror
contract sections are not.

The following staging types will be deleted when no longer used:

- `MirDropGlue`
- `MirProjectionResolution`
- `MirInstanceDeclaration`
- `MirProjectionImplMetadata`
- `MirExternDeclaration`
- `MirProductLinkCandidate`
- `MirStructLayout`
- `MirEnumLayout`

`MirEnumVariantLayout` and `MirVariantLayoutFields` remain because they are the
canonical enum-layout payload used by `MirNominalLayout`, not a parallel
authority.

## Projection Construction

Projection discovery still needs to inspect every concrete type used by MIR,
callable signatures, and nominal layouts. Its helpers will receive canonical
inputs rather than staging slices:

- callable declarations from the contract;
- canonical nominal layouts from the contract;
- accepted HIR impl receiver patterns and associated types;
- the current type context;
- builtin index trait IDs from language-item/selection authority.

Projection matching remains ID-based and must retain the existing unique-match
rule. The refactor will not add a name lookup, specificity preference, or
fallback output.

## Full Agreement Validation

`check_mir_runtime_agreement` will be the only whole-program agreement entry
point. The boolean validation switch and
`check_mir_runtime_agreement_for_mir_only` will be deleted.

Builder test helpers will construct minimal complete contracts before invoking
agreement. A complete fixture contains all facts used by its MIR:

- local callable declarations and function-body mappings;
- external or instance callable declarations used by call operands;
- canonical nominal layouts used by aggregates and projections;
- projection outputs and projection-trait IDs when required;
- drop glue and runtime requirements when exercised by the test.

Tests that intentionally target one agreement diagnostic will provide a valid
contract for unrelated facts so the expected diagnostic is not masked by
additional contract errors.

No default empty contract may be treated as valid for MIR that requires backend
facts.

## Step 2-5 Preservation Invariants

### Step 2: ID-Keyed Codegen Layouts

- Nominal layouts remain keyed only by `DefId`.
- Direct construction must not visit `struct_display_aliases` or
  `enum_display_aliases`.
- No `struct_info`, `enum_info`, generic-owner name table, or name-based
  substitution helper may be added.
- Codegen continues consuming only `MirBackendContract`.

### Step 3: Link Records Own Dependency Symbols

- Artifact exports remain output candidates; they do not become a second symbol
  lookup table.
- Product link attachment continues using the producer's explicit
  `ProductIdRemap` and remains fallible for missing or ambiguous callable IDs.
- Artifact loading continues rejecting missing or empty required backend
  symbols.
- No source, display, export, interface, trait, impl, or method name may recover
  a backend symbol.

### Step 4: No HIR Backend-Name Ownership

- Contract symbols continue coming from mono `InstanceRecord` symbol metadata
  selected by `InstanceOrigin` and substitutions.
- `HirFunction` and product function rows remain free of `qualified_name` or
  backend-symbol authority.
- The refactor will not recreate impl backend-name formatting or change the
  artifact schema.

### Step 5: ID-Based Method Authority

- Contract construction consumes already-materialized instance origins and MIR
  callable keys; it does not select methods.
- It must not inspect receiver display names, method names, or aliases to recover
  impl, trait, member, or callable identity.
- Static authority remains resolver-owned and typed through lowering.
- No raw method `DefId`, optional accepted authority, specificity winner, or
  downstream method rediscovery path may be introduced.

## Error Handling

Contract construction remains deterministic and does not invent fallback facts.
Conflicting rows for one canonical key are programming errors and must be
detected by existing contract validation or an explicit invariant check rather
than silently overwritten when the values differ.

Agreement reports remain structured through `MirAgreementReport` and
`MirBackendContractError`. Removing the MIR-only mode may expose incomplete test
fixtures; those fixtures must be corrected instead of suppressing validation.

## Regression Coverage

Add or strengthen tests proving:

- canonical struct and enum declarations create exactly one `DefId`-keyed
  layout regardless of display aliases;
- direct construction preserves extern, local instance, object-provided,
  projection, drop-glue, runtime-helper, and artifact-export contract sections;
- missing nominal layouts and callable declarations make agreement non-clean;
- every builder agreement helper uses full contract validation;
- production source contains none of the deleted staging types or the MIR-only
  agreement entry point;
- Step 3 explicit product remap and empty-symbol regressions remain green;
- Step 5 static-method authority and semantic identity audits remain green.

## Verification

Run focused suites in this order:

- `cargo test -p rock-lib mir::builder -- --nocapture`
- `cargo test -p rock-lib mir::agreement -- --nocapture`
- `cargo test -p rock-lib mir::backend_contract -- --nocapture`
- `cargo test -p rock-lib codegen -- --nocapture`
- `cargo test -p rock-lib codegen::mir_llvm -- --nocapture`
- `cargo test -p rock-lib products -- --nocapture`
- `cargo test -p rock-lib crate_artifact -- --nocapture`
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`
- `cargo test -p rock-lib --test integration`
- `cargo test -p rock-lib`
- `cargo clippy -p rock-lib --all-targets`
- `cargo fmt --all --check`
- `git diff --check`

After verification, update `CLEAN_SLATE_COMPILER_AUDIT.md` Step 1 evidence and
close `new_lang2-cnb`. No artifact format change is expected.
