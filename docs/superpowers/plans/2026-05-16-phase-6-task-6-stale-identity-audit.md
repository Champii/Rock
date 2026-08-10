# Phase 6 Task 6 Stale Identity Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete parent Phase 6 Task 6 by proving every remaining stale string/type-identity search hit is non-semantic or by converting the remaining disallowed paths to ID-backed identity.

**Architecture:** Treat this as an evidence-first audit. Start with the parent stale-identity search, classify every hit by file, prove each allowed path has a concrete ID-backed soundness invariant, and only edit production code where the audit finds a disallowed semantic name path. Keep backend/display/export names separate from semantic identity.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId`, `GenericParamId`, `TypeVarId`, `AssociatedTypeKey`, product artifact remapping, `rg`, focused Cargo tests, `cargo fmt --all --check`, `cargo test -p rock-lib`, `cargo test -p rock`.

---

## Source Documents

- Parent plan: `docs/superpowers/plans/2026-05-13-semantic-type-identity.md`
- Approved design: `docs/superpowers/specs/2026-05-16-phase-6-task-6-stale-identity-audit-design.md`
- Master audit checklist: `docs/superpowers/plans/master-audit-checklist.md`
- Previous parent Task 5 completion plan: `docs/superpowers/plans/2026-05-15-trait-projection-identity-completion.md`
- Previous parent Task 5 stricter inventory: `docs/superpowers/plans/2026-05-16-task-5-default-method-substitution-fix-inventory.md`

---

## Task 1: Capture Fresh Stale-Identity Evidence

**Files:**
- Read: `docs/superpowers/plans/2026-05-13-semantic-type-identity.md`
- Read: `docs/superpowers/specs/2026-05-16-phase-6-task-6-stale-identity-audit-design.md`
- Create: `/tmp/opencode/phase6-task6-stale-identity-search.txt`
- Create: `/tmp/opencode/phase6-task6-stale-identity-counts.txt`

- [ ] **Step 1: Confirm parent Task 6 scope**

Read `docs/superpowers/plans/2026-05-13-semantic-type-identity.md:376-415` and confirm the active task is `Task 6: Remove Remaining String-Based Type Identity Construction`.

Expected scope:
- run the stale identity search
- classify remaining string usage
- remove/update disallowed constructors or paths
- update `master-audit-checklist.md`
- rerun the stale identity search

- [ ] **Step 2: Run the parent stale-identity search**

Run:

```bash
rg "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src > /tmp/opencode/phase6-task6-stale-identity-search.txt
```

Expected: command exits `0` and writes the raw hit list.

- [ ] **Step 3: Count matches by file**

Run:

```bash
rg --count-matches "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src > /tmp/opencode/phase6-task6-stale-identity-counts.txt
```

Expected: command exits `0` and writes per-file counts.

- [ ] **Step 4: Preserve the exact audit command in notes**

When creating the tracked audit document in Task 2, copy the exact search command and timestamp-free evidence path references from Steps 2-3. Do not paste the entire raw search output into the tracked document; use grouped counts and file rows instead.

---

## Task 2: Create The File-By-File Audit Table

**Files:**
- Create: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`
- Read: `/tmp/opencode/phase6-task6-stale-identity-counts.txt`
- Read: `/tmp/opencode/phase6-task6-stale-identity-search.txt`

- [ ] **Step 1: Create the audit table document**

Create `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md` with this header and table schema:

````markdown
# Phase 6 Task 6 Stale Identity Audit Table

Parent plan: `docs/superpowers/plans/2026-05-13-semantic-type-identity.md`
Approved design: `docs/superpowers/specs/2026-05-16-phase-6-task-6-stale-identity-audit-design.md`

Search command:

```bash
rg "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src
```

Classification legend:
- `Allowed`: Search hit is ID-backed construction/traversal/test code or non-semantic display/source/backend metadata.
- `Suspicious`: Search hit may be safe, but needs a focused proof or nearby test.
- `Disallowed`: Search hit can affect semantic identity by name/string/raw ID and must be fixed before completion.
- `Fixed`: Search hit was disallowed or suspicious, then removed or guarded by implementation in this task.

| File | Matches | Classification | Soundness argument | Verification | Follow-up |
| --- | ---: | --- | --- | --- | --- |
```
````

- [ ] **Step 2: Add one row for every file with matches**

Use `/tmp/opencode/phase6-task6-stale-identity-counts.txt`. The row list must include every file from the count output. Do not merge multiple files into a single row.

- [ ] **Step 3: Seed initial classifications**

Use these initial classifications before detailed review:

```text
lib/src/types/mod.rs = Allowed
lib/src/hir/mod.rs = Suspicious
lib/src/collect/*.rs and lib/src/collect/**/*.rs = Mostly Allowed, record as Suspicious until reviewed
lib/src/lower/types.rs = Allowed if source-to-ID boundary is proven
lib/src/lower/**/*.rs = Suspicious until reviewed
lib/src/infer/**/*.rs = Allowed if TypeVarId/GenericParamId keys are proven
lib/src/crate_artifact/**/*.rs = Suspicious until exhaustive remap proof is recorded
lib/src/products.rs = Suspicious until export/display-name proof is recorded
lib/src/mono/**/*.rs = Suspicious until name-keyed maps are proven non-semantic or fixed
lib/src/codegen/**/*.rs = Suspicious until backend-name lookup is proven post-selection or fixed
```

- [ ] **Step 4: Add the seven soundness questions to the audit document**

Append the seven questions from the approved design under `## Soundness Questions`. They are:

```markdown
1. Could two same-named types from different modules/crates compare equal?
2. Could two same-named generic parameters from different owners compare equal or substitute into each other?
3. Could two same-named traits or associated types collide in a projection?
4. Could lowering, mono, or codegen select a trait impl by display name after an ID-backed target exists?
5. Could artifact loading accept a forged or stale type/projection/generic owner ID?
6. Could export/import/display names overwrite or shadow distinct semantic IDs?
7. Could backend symbol construction feed back into semantic equality or dispatch?
```

- [ ] **Step 5: Verify the audit table has no missing files**

Run:

```bash
rg --count-matches "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src
```

Compare every path in the output against the audit table. Expected: every path appears exactly once in the table.

---

## Task 3: Audit Core Type And HIR Identity

**Files:**
- Inspect: `lib/src/types/mod.rs`
- Inspect: `lib/src/hir/mod.rs`
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

- [ ] **Step 1: Audit `lib/src/types/mod.rs`**

Verify these invariants in `lib/src/types/mod.rs`:

```rust
pub struct GenericParamId {
    pub owner: DefId,
    pub index: u32,
}

pub struct AssociatedTypeKey {
    pub owner: DefId,
    pub assoc_type_id: AssocTypeId,
}

pub enum Type {
    Struct { id: DefId, args: Vec<Type> },
    Enum { id: DefId, args: Vec<Type> },
    TypeVar(TypeVarId),
    Generic(GenericParamId),
    Projection { trait_id: DefId, assoc_type: AssociatedTypeKey, trait_args: Vec<Type>, .. },
}
```

Expected classification: `Allowed` if every `Type` variant stores ID-backed identity and `Type::remap_def_ids` recurses through nominal IDs, generic owners, projection trait IDs, associated type owners, and trait args.

- [ ] **Step 2: Run core type identity tests**

Run:

```bash
cargo test -p rock-lib generic_param_identity_is_owner_and_index -- --nocapture
cargo test -p rock-lib generic_params_with_same_name_are_distinct_by_owner -- --nocapture
cargo test -p rock-lib same_named_nominal_types_are_distinct_by_def_id -- --nocapture
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids -- --nocapture
cargo test -p rock-lib type_def_id_remap_updates_projection_type_ids -- --nocapture
```

Expected: all pass.

- [ ] **Step 3: Audit `lib/src/hir/mod.rs` generic and method target identity**

Verify these HIR invariants:

```rust
pub type HirGenericBounds = HashMap<GenericParamId, Vec<TraitBound>>;

pub struct HirMethodCallTarget {
    pub impl_id: Option<DefId>,
    pub trait_id: Option<DefId>,
    pub trait_args: Vec<Type>,
    pub method_id: DefId,
    pub from_index_operator: bool,
}
```

Allowed names in HIR:
- `name: String` for source/display names.
- `qualified_name: Option<String>` for backend or lookup metadata.
- `trait_name: String` inside enum/debug/display variants only when a parallel `trait_id` or method target identity drives semantics.

Disallowed HIR state:
- generic bounds keyed by strings.
- method target identity represented only by method or trait names.

- [ ] **Step 4: Update audit table rows for `types` and `hir`**

Record the soundness argument for each row. If `lib/src/hir/mod.rs` still has suspicious `trait_name: String` hits, note the exact enum/struct and whether a focused test already proves it cannot drive dispatch.

---

## Task 4: Audit Collection And Lowering Source-To-ID Boundary

**Files:**
- Inspect: `lib/src/collect/headers.rs`
- Inspect: `lib/src/collect/context.rs`
- Inspect: `lib/src/collect/mod.rs`
- Inspect: `lib/src/lower/types.rs`
- Inspect: `lib/src/lower/collect/types.rs`
- Inspect: `lib/src/lower/collect/traits.rs`
- Inspect: `lib/src/lower/function.rs`
- Inspect: `lib/src/lower/bodies.rs`
- Inspect: `lib/src/lower/types_helpers/helpers.rs`
- Inspect: `lib/src/lower/traits/conformance.rs`
- Inspect: `lib/src/lower/control_flow/secondary.rs`
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

- [ ] **Step 1: Verify type lowering resolves names exactly once**

Inspect `lib/src/lower/types.rs` and `lib/src/lower/collect/types.rs`. Confirm parsed/source type names are converted into `Type::Struct { id, .. }`, `Type::Enum { id, .. }`, `Type::Generic(GenericParamId)`, and `Type::Projection { trait_id, assoc_type, .. }` before downstream semantic phases.

Expected allowed source-name uses:
- source paths before HIR type construction
- diagnostics for unresolved names
- resolver lookup keys

Disallowed outcome:
- storing source names in `Type` variants or `TraitBound` as semantic identity.

- [ ] **Step 2: Verify collection keeps declaration names non-semantic**

Inspect `lib/src/collect/headers.rs`, `lib/src/collect/context.rs`, and `lib/src/collect/mod.rs`. Confirm any `HashMap<String, ...>` hit is declaration lookup, resolver lookup, source names, or variable/scope metadata and not semantic type identity.

Specific invariant to record:
`collect_type_var_ids` visits `Type::Projection` base and trait args, so type-variable constraints are not lost inside ID-backed projections.

- [ ] **Step 3: Verify lowering generic substitution uses `GenericParamId`**

Inspect `lib/src/lower/function.rs`, `lib/src/lower/bodies.rs`, and `lib/src/lower/traits/conformance.rs`. Confirm generic substitution maps are keyed by `GenericParamId`, not generic parameter names.

Run:

```bash
cargo test -p rock-lib collect_type_var_ids_visits_projection_base_and_trait_args -- --nocapture
cargo test -p rock-lib conformance_does_not_substitute_projection_on_non_self_generic_base -- --nocapture
cargo test -p rock-lib conformance_substitutes_default_method_projection_with_matching_trait_args -- --nocapture
```

Expected: all pass.

- [ ] **Step 4: Verify lowerer trait/projection/operator decisions are ID-backed**

Inspect `lib/src/lower/types_helpers/helpers.rs`, `lib/src/lower/expression.rs`, and `lib/src/lower/control_flow/secondary.rs`. Confirm:
- projection resolution checks `assoc_type.owner == trait_id`
- trait impl lookup compares `trait_id` and `trait_args`
- builtin operator/index paths use canonical trait IDs where available
- same-name traits cannot satisfy each other by method or trait display name

Run:

```bash
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact --nocapture
cargo test -p rock-lib --test integration test_index_operator_rejects_local_shadow_index_trait -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact --nocapture
```

Expected: all pass.

- [ ] **Step 5: Update audit table for collection/lowering files**

For each collection/lowering file with search hits, set classification to `Allowed` only if the row includes a concrete invariant. Otherwise keep `Suspicious` and add the exact line or function that needs a focused test.

---

## Task 5: Audit Inference Identity

**Files:**
- Inspect: `lib/src/infer/engine.rs`
- Inspect: `lib/src/infer/generalize.rs`
- Inspect: `lib/src/infer/solve.rs`
- Inspect: `lib/src/infer/type_vars.rs`
- Inspect: `lib/src/infer/mod.rs`
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

- [ ] **Step 1: Verify inference type variables use `TypeVarId`**

Inspect inference files and confirm:
- `Type::TypeVar(TypeVarId)` is used, not raw `u32`.
- substitutions and trait-bound records are keyed by `TypeVarId` where they refer to inference variables.
- generic parameters remain `GenericParamId` when generalized or substituted.

- [ ] **Step 2: Run inference/type-var tests**

Run:

```bash
cargo test -p rock-lib substitute_uses_typed_type_var_id_keys -- --nocapture
cargo test -p rock-lib type_substitution_uses_generic_param_identity -- --nocapture
cargo test -p rock-lib resolve_all_types_in_expr_resolves_method_target_trait_args -- --nocapture
```

Expected: all pass.

- [ ] **Step 3: Update audit table for inference files**

Record why each `Type::TypeVar` and `Type::Generic` hit is an ID-backed operation. If any raw integer or generic-name key remains, classify that file `Disallowed` and proceed to Task 10.

---

## Task 6: Audit Artifact And Product Identity

**Files:**
- Inspect: `lib/src/crate_artifact/load.rs`
- Inspect: `lib/src/crate_artifact/tests.rs`
- Inspect: `lib/src/products.rs`
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

- [ ] **Step 1: Verify artifact `Type` remapping coverage**

Inspect `lib/src/crate_artifact/load.rs`. Confirm `remap_type_def_ids` validates/remaps:
- `Type::Struct.id`
- `Type::Enum.id`
- `Type::Generic.owner`
- `Type::Projection.trait_id`
- `Type::Projection.assoc_type.owner`
- `Type::Projection.trait_args`
- nested type containers

Confirm `ProductNominalTypeValidator::validate_projection` rejects:
- unknown projection trait IDs
- unknown associated type owners
- associated type IDs not declared by the owner trait
- `assoc_type.owner != trait_id`

- [ ] **Step 2: Verify artifact method target and trait-bound remapping coverage**

Inspect `lib/src/crate_artifact/load.rs` and confirm remapping covers:
- `TraitBound.trait_id`
- `TraitBound.type_args`
- `HirFunction.generic_bounds`
- `HirFunctionSig.generic_bounds`
- `HirMethodCallTarget.impl_id`
- `HirMethodCallTarget.trait_id`
- `HirMethodCallTarget.trait_args`
- impl IDs, method IDs, trait IDs, and associated type IDs persisted in products

- [ ] **Step 3: Run artifact identity tests**

Run:

```bash
cargo test -p rock-lib product_artifact_remaps_projection_type_ids -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_projection_trait_id -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_projection_assoc_type_id -- --nocapture
cargo test -p rock-lib product_artifact_rejects_projection_assoc_owner_mismatch_across_crates -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_dependency_projection_trait_id -- --nocapture
```

Expected: all pass.

- [ ] **Step 4: Verify product export/display names are non-semantic**

Inspect `lib/src/products.rs`. Confirm:
- `record_export_name` drops ambiguous collisions.
- display names can duplicate, but export names cannot silently overwrite distinct IDs.
- trait default method selection does not fall back by `(trait display name, method name)` when exact IDs exist.

Run:

```bash
cargo test -p rock-lib compiler_products_drop_ambiguous_export_name_collisions -- --nocapture
cargo test -p rock-lib compiler_products_keep_export_name_ambiguous_after_third_collision -- --nocapture
cargo test -p rock-lib compiler_products_do_not_export_colliding_impl_method_display_names -- --nocapture
cargo test -p rock-lib compiler_products_prefer_exact_trait_default_method_identity -- --nocapture
```

Expected: all pass.

- [ ] **Step 5: Update audit table for artifact/product files**

Record remap/validation functions by name. If a persisted ID-bearing field has no validation/remap path or test, classify it `Disallowed` and proceed to Task 10.

---

## Task 7: Audit Monomorphization Identity

**Files:**
- Inspect: `lib/src/mono/mod.rs`
- Inspect: `lib/src/mono/methods.rs`
- Inspect: `lib/src/mono/specialize.rs`
- Inspect: `lib/src/mono/process.rs`
- Inspect: `lib/src/mono/substitute.rs`
- Inspect: `lib/src/mono/external.rs`
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

- [ ] **Step 1: Identify every name-keyed mono map**

Inspect the `Monomorphizer` fields and record every string-keyed map in the audit row for `lib/src/mono/mod.rs`, including:

```rust
trait_impls: HashMap<String, Vec<HirImpl>>,
type_impls: HashMap<String, Vec<(String, Vec<Type>)>>,
generic_impls: HashMap<String, HirImpl>,
object_backed_impls: HashSet<String>,
var_types: HashMap<String, Type>,
```

Classify each as:
- `Allowed` if it is variable scope, backend/object symbol bookkeeping, or an index guarded by `DefId`/trait args/receiver args.
- `Suspicious` if a lookup can pick a semantic impl by string alone.
- `Disallowed` if same-name distinct IDs can collide and change monomorphized output.

- [ ] **Step 2: Verify instance and generic substitution identity**

Inspect `lib/src/mono/specialize.rs`, `lib/src/mono/substitute.rs`, and `lib/src/mono/methods.rs`. Confirm:
- substitutions use `GenericParamId`
- `InstanceOrigin` uses `DefId` where semantic identity matters
- method target matching uses `impl_id`, `trait_id`, `method_id`, and `trait_args`
- receiver matching for nominal types checks canonical IDs or proven unambiguous aliases

Run:

```bash
cargo test -p rock-lib receiver_type_match_ -- --nocapture
cargo test -p rock-lib method_generic_substitution_recurses_into_matching_projections -- --nocapture
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_matches_generic_impl_trait_arguments -- --exact --nocapture
cargo test -p rock-lib --test integration test_repeated_receiver_generic_impl_does_not_match_incompatible_receiver_args -- --exact --nocapture
```

Expected: all pass.

- [ ] **Step 3: Verify same-name monomorphization behavior**

Run:

```bash
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_generic_trait_default_methods_select_bound_trait_body -- --exact --nocapture
cargo test -p rock test_artifact_consumer_calls_dependency_qualified_impl_owner -- --nocapture
```

Expected: all pass.

- [ ] **Step 4: Update audit table for mono files**

For each mono file, record whether the string usage is a variable name, backend name, display name, or an index guarded by semantic IDs. If any lookup remains semantic-by-string, classify it `Disallowed` and proceed to Task 10.

---

## Task 8: Audit Codegen Identity

**Files:**
- Inspect: `lib/src/codegen/mod.rs`
- Inspect: `lib/src/codegen/types.rs`
- Inspect: `lib/src/codegen/operators.rs`
- Inspect: `lib/src/codegen/intrinsics.rs`
- Inspect: `lib/src/codegen/expr/mod.rs`
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

- [ ] **Step 1: Verify codegen type names are backend/display only**

Inspect `lib/src/codegen/types.rs` and `lib/src/codegen/mod.rs`. Confirm `get_type_name_for_method`, backend suffix helpers, and function maps are used for LLVM symbol lookup after semantic selection, not for type equality.

- [ ] **Step 2: Verify method-call codegen uses HIR targets first**

Inspect `lib/src/codegen/expr/mod.rs`. Confirm selected method dispatch resolves from:
- exact `(impl_id, method_id)` backend symbols
- registered impls matched by `DefId` and trait args
- selected trait target identity

Only targetless direct/builtin fallback paths may use legacy name forms.

- [ ] **Step 3: Verify codegen projection and operator identity**

Inspect `lib/src/codegen/types.rs` and `lib/src/codegen/operators.rs`. Confirm projection resolution and operator lowering do not accept same-name traits without canonical IDs.

Run:

```bash
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact --nocapture
cargo test -p rock-lib --test integration test_unary_neg_dispatches_through_stdlib_trait_with_associated_output -- --exact --nocapture
cargo test -p rock-lib --test integration test_unary_not_dispatches_through_stdlib_trait_with_associated_output -- --exact --nocapture
```

Expected: all pass.

- [ ] **Step 4: Update audit table for codegen files**

Record why every remaining codegen string hit is backend/display-only or guarded by selected IDs. If any string path can select a semantic impl when a target exists, classify it `Disallowed` and proceed to Task 10.

---

## Task 9: Soundness Synthesis And Checklist Update

**Files:**
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Answer the seven soundness questions globally**

Add a `## Soundness Synthesis` section to the audit table document. Each answer must name the concrete files/functions/tests that prove the claim:

```markdown
## Soundness Synthesis

1. Same-named nominal types cannot compare equal because `Type::Struct` and `Type::Enum` carry `DefId`; cite the audited files and focused test evidence.
2. Same-named generic parameters cannot collide because `Type::Generic` carries `GenericParamId { owner, index }`; cite substitution and bounds evidence.
3. Same-named traits/associated types cannot collide in projections because projections carry `trait_id` and `AssociatedTypeKey`; cite projection resolution and artifact validation evidence.
4. Lowering/mono/codegen cannot select trait impls by display name after IDs exist because selected targets, receiver IDs, trait IDs, trait args, method IDs, or instance origins drive semantic selection; cite every remaining fallback and its guard.
5. Artifact loading cannot accept forged/stale IDs because every serialized ID-bearing field is validated and remapped before downstream use; cite validation/remap functions and tests.
6. Export/import/display names cannot overwrite semantic IDs because ambiguous export names are dropped and display names stay separate from canonical IDs; cite product identity tests.
7. Backend symbols cannot feed back into semantic equality/dispatch because backend names are produced after semantic selection; cite mono/codegen paths and tests.
```

Each answer must cite at least one file/function and one focused test or search result.

- [ ] **Step 2: Check for unresolved suspicious/disallowed rows**

Search the audit table:

```bash
rg "\| .*\| .*Suspicious|\| .*\| .*Disallowed" docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md
```

Expected before completion: no matches. If matches remain, do not update `master-audit-checklist.md`; go to Task 10.

- [ ] **Step 3: Update `master-audit-checklist.md` Type Context section**

If Step 2 has no matches, update `docs/superpowers/plans/master-audit-checklist.md` under `## 3. Type Context And Semantic Types`:

Keep this unchecked:

```markdown
- [ ] Decide whether to intern types and make `TypeId` authoritative.
```

Mark these complete only if the audit proves them:

```markdown
- [x] Replace remaining name-bearing generic and projection identity with canonical IDs.
- [x] Remove remaining string-based trait/projection lookup identity from lower, mono, and codegen.
```

Update the evidence bullets to mention:
- `GenericParamId`
- `AssociatedTypeKey`
- `TraitBound { trait_id, type_args }`
- artifact remapping/validation
- stale-identity audit table path

- [ ] **Step 4: Run the stale identity search again**

Run:

```bash
rg "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src
```

Expected: remaining hits are exactly the hits classified as `Allowed` in the audit table.

---

## Task 10: Fix Any Disallowed Path Found During Audit

**Files:**
- Modify: file identified by a `Disallowed` audit row
- Test: nearest unit/integration/artifact test file for that subsystem
- Modify: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`

Use this task only if Tasks 3-8 identify a real disallowed path.

- [ ] **Step 1: Write a failing test for the disallowed path**

Choose the smallest test level that proves the bug:
- type/HIR identity: unit test near `lib/src/types/mod.rs` or `lib/src/hir/mod.rs`
- lowering/projection/operator identity: unit test near lower helper or integration test in `lib/tests/integration.rs`
- artifact remap/validation: unit test in `lib/src/crate_artifact/load.rs` or `lib/src/crate_artifact/tests.rs`
- product/export collision: unit test in `lib/src/products.rs`
- mono/codegen dispatch: integration test in `lib/tests/integration.rs` or `rock/src/tests/artifact.rs`

The test name must encode the exact soundness property. For a mono/codegen short-name bug, use a name like `same_named_dependency_nominals_do_not_dispatch_by_short_name` and make the test construct or compile two same-short-name nominal types with distinct `DefId`s, then assert the selected method/impl cannot come from the wrong owner.

- [ ] **Step 2: Run the focused test and confirm RED**

Run the exact focused test command for the new test.

Expected: FAIL for the disallowed semantic string path, not for syntax or setup errors.

- [ ] **Step 3: Implement the minimal ID-backed fix**

Allowed fix patterns:
- replace a string key with `DefId`, `GenericParamId`, `TypeVarId`, `AssociatedTypeKey`, or a tuple including those IDs
- add an exact ID check before a legacy backend-name fallback
- validate/remap a missing artifact ID-bearing field
- drop ambiguous export/display name collisions instead of overwriting IDs

Disallowed fix patterns:
- adding compatibility constructors that recreate old string identity
- accepting short-name fallback without an ambiguity or `DefId` check
- changing tests to accept name-based behavior

- [ ] **Step 4: Run the focused test and confirm GREEN**

Run the same focused test command from Step 2.

Expected: PASS.

- [ ] **Step 5: Update the audit row**

Change the row classification from `Disallowed` to `Fixed`, and add:
- failing test command
- fix summary
- green verification command

- [ ] **Step 6: Return to the originating audit task**

After one disallowed path is fixed, return to the task that found it and continue the file-by-file audit. Do not batch unrelated fixes.

---

## Task 11: Final Verification And Review

**Files:**
- Read: `docs/superpowers/plans/2026-05-16-phase-6-task-6-stale-identity-audit-table.md`
- Read/modify if needed: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Run focused identity tests**

Run:

```bash
cargo test -p rock-lib generic_param_identity_is_owner_and_index -- --nocapture
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids -- --nocapture
cargo test -p rock-lib type_def_id_remap_updates_projection_type_ids -- --nocapture
cargo test -p rock-lib product_artifact_remaps_projection_type_ids -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_projection_trait_id -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_projection_assoc_type_id -- --nocapture
cargo test -p rock-lib compiler_products_drop_ambiguous_export_name_collisions -- --nocapture
```

Expected: all pass.

- [ ] **Step 2: Run focused same-name and artifact tests**

Run:

```bash
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_trait_default_projection_uses_selected_trait_identity -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_generic_trait_default_methods_select_bound_trait_body -- --exact --nocapture
cargo test -p rock test_artifact_consumer_calls_dependency_qualified_impl_owner -- --nocapture
```

Expected: all pass.

- [ ] **Step 3: Run artifact compatibility tests**

Run:

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --nocapture
cargo test -p rock-lib compiler_products_rejects_unsupported_format_before_full_deserialize -- --nocapture
cargo test -p rock-lib test_compile_generic_function_from_artifact_hir_bundle -- --nocapture
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle -- --nocapture
cargo test -p rock-lib test_compile_generic_impl_from_file_module_artifact_hir_bundle -- --nocapture
```

Expected: all pass. If a filter has no matching test in the current checkout, replace it with the nearest actual artifact test and record the substitution in the audit table.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
cargo test -p rock
```

Expected: all commands exit `0`.

- [ ] **Step 5: Request focused post-fix review**

Request review with this scope:

```text
Review parent Phase 6 Task 6 stale identity audit. Check whether every remaining stale-search hit is either removed/fixed or correctly classified as non-semantic. Focus especially on mono/codegen name-keyed maps, artifact remap coverage, projection identity, generic substitution keys, and whether master-audit-checklist.md overclaims completion.
```

Expected: no Critical or Important findings.

- [ ] **Step 6: Commit tracked Task 6 audit/fixes**

Before committing, run:

```bash
git status --short
git diff --check
git diff
git log --oneline -10
```

Stage only intended tracked files. Do not stage `.sisyphus/`.

Commit message:

```bash
git commit -m "audit phase 6 stale type identity"
```

---

## Completion Criteria

Parent Phase 6 Task 6 can be called complete only after:

- [ ] Fresh stale-identity search evidence is captured.
- [ ] Every search-hit file has exactly one audit-table row.
- [ ] No audit row remains `Suspicious` or `Disallowed`.
- [ ] Every allowed row has a concrete soundness argument.
- [ ] Every fixed row has red-green test evidence.
- [ ] `master-audit-checklist.md` is updated only for proven items.
- [ ] `TypeId` interning remains explicitly deferred unless separately implemented.
- [ ] Focused identity/same-name/artifact tests pass.
- [ ] `cargo fmt --all --check`, `git diff --check`, `cargo test -p rock-lib`, and `cargo test -p rock` pass.
- [ ] Focused review reports no Critical or Important findings.
- [ ] Intended files are committed, and untracked `.sisyphus/` remains untouched.

---

## Notes For Executors

- This task is evidence-heavy. Do not mark a row `Allowed` without explaining why the string/name cannot affect semantic identity.
- Do not treat search hits as inherently bad. ID-backed constructors are expected to match this search.
- Do not update `master-audit-checklist.md` until the audit table has no `Suspicious` or `Disallowed` rows.
- If a real disallowed path is found, switch to TDD immediately using Task 10 before continuing the audit.
