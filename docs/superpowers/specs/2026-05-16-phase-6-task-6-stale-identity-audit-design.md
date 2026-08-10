# Phase 6 Task 6 Stale Identity Audit Design

**Goal:** Complete parent Phase 6 Task 6 by proving every remaining stale-identity search hit is non-semantic or by removing the remaining string-based semantic type identity construction paths.

**Parent Plan:** `docs/superpowers/plans/2026-05-13-semantic-type-identity.md`

**Task:** `Task 6: Remove Remaining String-Based Type Identity Construction`

**Current Context:** The plan `docs/superpowers/plans/2026-05-15-trait-projection-identity-completion.md` and the follow-up inventory `docs/superpowers/plans/2026-05-16-task-5-default-method-substitution-fix-inventory.md` finished parent Phase 6 Task 5. This design is for the next parent task, not for the internal Task 6 inside the Task 5 inventory.

---

## Scope

Parent Phase 6 Task 6 is an audit and cleanup gate. It does not introduce a new type model, trait selection service, MIR backend boundary, parser loader, formatter, or borrow checker work. It validates that the semantic type identity migration already performed by Tasks 1-5 is complete enough that remaining string/name usage cannot affect type equality, trait identity, projection identity, monomorphization selection, artifact remapping, or codegen dispatch.

The exact parent-task search is:

```bash
rg "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src
```

Fresh evidence was saved to `/tmp/opencode/phase6-task6-stale-identity-search.txt` during analysis. The current by-file counts were:

```text
lib/src/lower/traits/conformance.rs:62
lib/src/lower/function.rs:6
lib/src/lower/bodies.rs:23
lib/src/lower/types.rs:1
lib/src/lower/types_helpers/helpers.rs:2
lib/src/lower/expression.rs:5
lib/src/lower/control_flow/pattern.rs:1
lib/src/lower/control_flow/secondary.rs:21
lib/src/lower/mod.rs:1
lib/src/lower/collect/traits.rs:4
lib/src/lower/collect/types.rs:2
lib/src/codegen/types.rs:2
lib/src/codegen/operators.rs:2
lib/src/codegen/mod.rs:2
lib/src/collect/headers.rs:12
lib/src/collect/context.rs:3
lib/src/collect/mod.rs:5
lib/src/infer/engine.rs:9
lib/src/infer/generalize.rs:5
lib/src/infer/solve.rs:4
lib/src/infer/type_vars.rs:2
lib/src/infer/mod.rs:9
lib/src/hir/mod.rs:7
lib/src/crate_artifact/tests.rs:1
lib/src/crate_artifact/load.rs:14
lib/src/mono/methods.rs:27
lib/src/mono/specialize.rs:9
lib/src/mono/process.rs:1
lib/src/mono/mod.rs:4
lib/src/mono/substitute.rs:1
lib/src/mono/external.rs:6
lib/src/types/mod.rs:27
lib/src/products.rs:26
lib/src/codegen/intrinsics.rs:2
```

The match count alone is not a failure. Many matches are now expected because `Type::Generic(GenericParamId)`, `Type::TypeVar(TypeVarId)`, `Type::Struct { id: DefId, .. }`, `Type::Enum { id: DefId, .. }`, and `Type::Projection { trait_id: DefId, assoc_type: AssociatedTypeKey, .. }` are the ID-backed forms required by Phase 6.

---

## Current Identity Model

`lib/src/types/mod.rs` is the source of truth for semantic type identity:

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
    Projection {
        ty: Box<Type>,
        trait_id: DefId,
        assoc_type: AssociatedTypeKey,
        trait_args: Vec<Type>,
    },
    // primitives and containers omitted
}
```

This means the parent Task 6 audit should not remove ordinary constructors for these variants. It should remove only sites where names are still used as semantic substitutes for those IDs.

`TraitBound` also needs to stay ID-backed. It should be keyed by `GenericParamId` or `TypeVarId` depending on phase and carry `trait_id: DefId` plus `type_args: Vec<Type>`. Any `HashMap<String, Vec<String>>` or trait-name-only semantic bound representation is disallowed.

---

## Classification Rules

### Allowed

These search hits can remain if the implementation note for each file explains why they cannot drive semantic identity:

- Construction, matching, substitution, or traversal of ID-backed `Type` variants.
- Tests constructing ID-backed `Type` values to prove equality, remapping, substitution, or same-name collision behavior.
- Source names used before lowering resolves them to IDs.
- Declaration maps, import/export maps, and resolver tables where strings are lookup keys rather than semantic identity.
- Diagnostics and display code.
- Backend symbol naming after HIR target identity or monomorphization identity has already selected the semantic item.
- Method or trait display names retained alongside canonical IDs for diagnostics, export metadata, artifact display names, or user-facing names.

### Suspicious

These hits require deeper inspection and usually need a focused test:

- `HashMap<String, Type>` or `HashMap<String, Vec<...>>` in lowering, monomorphization, or codegen. Some maps are variable scopes and are allowed, but their values must not become canonical item identity.
- Mono/codegen lookup maps keyed by type names, trait names, or method names. They are allowed only when a validated `DefId` target or instance identity already selected the semantic item and the string is just a backend-symbol lookup.
- Fallbacks from qualified names to short names. They are allowed only when ambiguity is impossible or explicitly checked by `DefId`.
- `trait_name: String` fields in HIR-ish data structures. They are allowed as display/source metadata only when a parallel `trait_id: DefId` or validated target identity drives dispatch and conformance.
- Artifact metadata strings that are read before ID remapping or validation. They are allowed for display/export lookup only, not for validating projections, impls, methods, or bounds.

### Disallowed

These must be removed or converted before Task 6 can be called complete:

- Any equality-bearing `Type` field storing a source/display name instead of `DefId`, `TypeVarId`, `GenericParamId`, or `AssociatedTypeKey`.
- Trait conformance, trait method lookup, projection resolution, operator lookup, monomorphization, or codegen selecting by `trait_name`, associated type name, method name, or nominal display name when an ID is available.
- Generic substitution keyed by generic parameter name instead of `GenericParamId`.
- Artifact-loaded `Type`, `TraitBound`, method-target, impl, trait, or associated-type identity that skips product-ID validation/remapping.
- Any compatibility helper that reconstructs old name-based semantic identity after lowering.

---

## Required Audit Inventory

The implementation plan should create a tracked audit table in a new plan file, not in code comments. The table must have one row per file from the stale-search count. Each row must include:

- file path
- match category: `Allowed`, `Suspicious`, `Disallowed`, or `Fixed`
- reason
- soundness argument
- focused test or verification command
- follow-up action if not fully allowed

The table should start with these known likely classifications, then verify each line before finalizing:

| File | Initial Classification | Required Soundness Check |
| --- | --- | --- |
| `lib/src/types/mod.rs` | Allowed | Confirm variants carry IDs, remap recurses through every embedded `DefId`, and `Display` is not used for equality. |
| `lib/src/hir/mod.rs` | Suspicious | Confirm `trait_name: String` hits are display/source metadata or enum fields paired with canonical method target IDs. Confirm `HirGenericBounds` is keyed by `GenericParamId`. |
| `lib/src/collect/**` | Mostly allowed | Confirm collection may use source names before lowering but emitted `Type`s and bounds carry IDs. |
| `lib/src/lower/types.rs` | Allowed if source-to-ID boundary | Confirm this is the only point source nominal/generic names become semantic IDs. |
| `lib/src/lower/**` | Mixed | Confirm remaining name maps are scopes/lookup only and all conformance/projection/operator decisions use IDs. |
| `lib/src/infer/**` | Allowed | Confirm `TypeVarId` and `GenericParamId` are used as keys, no raw `u32` or generic-name substitution maps remain. |
| `lib/src/mono/**` | Suspicious | Confirm type-name/trait-name maps are non-semantic or guarded by selected targets, instance origins, receiver IDs, or ambiguity checks. |
| `lib/src/codegen/**` | Suspicious | Confirm backend-name lookup happens after selected HIR targets or monomorphized instance identity, not as semantic trait/type selection. |
| `lib/src/crate_artifact/**` | Mostly allowed | Confirm every serialized `DefId` inside `Type`, `TraitBound`, impls, traits, method targets, generic owners, and projections is validated/remapped. |
| `lib/src/products.rs` | Suspicious | Confirm product export/display names cannot overwrite distinct IDs and are not used as semantic identity. |

---

## Soundness Questions To Answer Per File

Each file-level review must answer these questions. If the answer is unknown, the file stays `Suspicious` and needs a test or code change.

1. **Could two same-named types from different modules/crates compare equal?**
   - Required answer: no, because nominal `Type::Struct` and `Type::Enum` compare by `DefId` and recursive args.

2. **Could two same-named generic parameters from different owners compare equal or substitute into each other?**
   - Required answer: no, because `Type::Generic` carries `GenericParamId { owner, index }` and substitution maps use `GenericParamId`.

3. **Could two same-named traits or associated types collide in a projection?**
   - Required answer: no, because projections carry `trait_id` and `AssociatedTypeKey { owner, assoc_type_id }`, and resolution/artifact loading validate owner equality.

4. **Could lowering, mono, or codegen select a trait impl by display name after an ID-backed target exists?**
   - Required answer: no. If a fallback remains, it must be targetless/direct/builtin-only or guarded by receiver `DefId` and trait args.

5. **Could artifact loading accept a forged or stale type/projection/generic owner ID?**
   - Required answer: no, because load-time remap validates local and dependency IDs before downstream phases see them.

6. **Could export/import/display names overwrite or shadow distinct semantic IDs?**
   - Required answer: no, because product identity tables drop ambiguous export names and keep display names separate from canonical IDs.

7. **Could backend symbol construction feed back into semantic equality or dispatch?**
   - Required answer: no, because backend names are selected after semantic HIR method targets, impl IDs, instance origins, or product backend-symbol records are known.

---

## Expected Work Breakdown

### Workstream 1: Produce The Audit Inventory

Run the parent stale-identity search and group each result by subsystem. Create a new implementation plan document with a file-by-file table. Do not edit code during this pass except for the audit document.

The audit is incomplete if it says only “allowed by inspection.” Each allowed row must include a specific invariant, such as “`Type::Generic` is keyed by `GenericParamId`, and this match only constructs that ID-backed form in a test.”

### Workstream 2: Close Suspicious Mono And Codegen Paths

The likely highest-risk remaining area is mono/codegen because these modules still store several name-keyed maps for practical backend dispatch:

- `Monomorphizer::trait_impls: HashMap<String, Vec<HirImpl>>`
- `Monomorphizer::type_impls: HashMap<String, Vec<(String, Vec<Type>)>>`
- `Monomorphizer::generic_impls: HashMap<String, HirImpl>`
- `Codegen::trait_impls: HashMap<(String, Vec<Type>, DefId, Vec<Type>), HirImpl>`
- backend function maps keyed by strings

These are not automatically wrong. The implementation must prove each map is non-semantic or refactor it. A name-keyed map is acceptable when:

- it is used only after exact `DefId`/target/instance identity selection, or
- it is a cache/index whose entries are disambiguated by `DefId`, receiver args, and trait args, or
- it is strictly backend symbol generation after semantic selection.

If proof is difficult, prefer a small code change that adds the relevant ID to the key over documenting a fragile invariant.

### Workstream 3: Close Artifact And Product Identity Proofs

Artifact load already validates projections and remaps many ID-bearing fields. Task 6 must verify that this coverage is exhaustive for current serialized HIR shape:

- `Type::Struct.id`
- `Type::Enum.id`
- `Type::Generic.owner`
- `Type::Projection.trait_id`
- `Type::Projection.assoc_type.owner`
- `Type::Projection.trait_args`
- `TraitBound.trait_id`
- `TraitBound.type_args`
- `HirMethodCallTarget.impl_id`
- `HirMethodCallTarget.trait_id`
- `HirMethodCallTarget.trait_args`
- impl IDs, method IDs, trait IDs, associated type IDs, field IDs, and variant IDs where persisted

The audit must identify the remap/validation function for each item and cite a focused test. Missing validation gets a red test and a fix.

### Workstream 4: Update Master Audit Checklist

Only update `docs/superpowers/plans/master-audit-checklist.md` after the final stale-identity search is classified. The likely updates are under “Type Context And Semantic Types”:

- Mark “Replace remaining name-bearing generic and projection identity with canonical IDs” complete if the audit proves current `GenericParamId`, `AssociatedTypeKey`, and `TraitBound` coverage is sound.
- Mark “Remove remaining string-based trait/projection lookup identity from lower, mono, and codegen” complete only if mono/codegen suspicious paths are proven or fixed.
- Keep “Decide whether to intern types and make `TypeId` authoritative” unchecked because parent Phase 6 explicitly defers whole-type interning.

---

## Required Tests And Commands

The implementation checklist should include these exact commands. Some are audit commands and some are verification gates.

### Stale Identity Search

```bash
rg "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src
```

Expected final state: every remaining hit appears in the audit table and is classified as allowed by a concrete soundness argument.

### Existing Identity Regression Tests

Run focused tests that already exercise the key invariants:

```bash
cargo test -p rock-lib generic_param_identity_is_owner_and_index -- --nocapture
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids -- --nocapture
cargo test -p rock-lib type_def_id_remap_updates_projection_type_ids -- --nocapture
cargo test -p rock-lib product_artifact_remaps_projection_type_ids -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_projection_trait_id -- --nocapture
cargo test -p rock-lib product_artifact_rejects_unknown_projection_assoc_type_id -- --nocapture
cargo test -p rock-lib compiler_products_drop_ambiguous_export_name_collisions -- --nocapture
```

If any command has no matching tests in the current checkout, replace it with the actual nearest test and record that substitution in the implementation plan.

### Same-Name User-Visible Regressions

Run focused integration tests that prove same-name identity does not collapse:

```bash
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_trait_default_projection_uses_selected_trait_identity -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_generic_trait_default_methods_select_bound_trait_body -- --exact --nocapture
```

### Artifact Regressions

Run artifact tests that prove product identity and default/artifact remapping remain sound:

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --nocapture
cargo test -p rock-lib compiler_products_rejects_unsupported_format_before_full_deserialize -- --nocapture
cargo test -p rock-lib test_compile_generic_function_from_artifact_hir_bundle -- --nocapture
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle -- --nocapture
cargo test -p rock-lib test_compile_generic_impl_from_file_module_artifact_hir_bundle -- --nocapture
cargo test -p rock test_artifact_consumer_calls_dependency_qualified_impl_owner -- --nocapture
```

### Full Verification

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
cargo test -p rock
```

---

## Completion Checklist

Phase 6 Task 6 is complete only when all of these are true:

- [ ] The parent stale-identity search has been run fresh.
- [ ] Every file with search hits has a row in the audit table.
- [ ] Every remaining `Type::Struct`, `Type::Enum`, `Type::Generic`, and `Type::TypeVar` constructor/match is proven to use ID-backed identity or is test-only construction of ID-backed identity.
- [ ] Every remaining `trait_name: String` hit is display/source metadata or paired with `trait_id`/method target identity for semantics.
- [ ] Every remaining `HashMap<String, Type>` hit is a variable scope, source lookup, display/backend map, or otherwise non-semantic; disallowed maps are removed or keyed by IDs.
- [ ] Lowering does not create semantic nominal, generic, trait, or associated-type identity from strings after the source-to-HIR boundary.
- [ ] Inference uses `TypeVarId` and `GenericParamId`, not raw IDs or names, for type variables/generics.
- [ ] HIR generic bounds use `HirGenericBounds = HashMap<GenericParamId, Vec<TraitBound>>`, and inference-time unresolved trait bounds use `TypeVarId` keys; no trait-bound path uses generic names or raw integer type variables as semantic keys.
- [ ] Projection identity always uses `trait_id + assoc_type.owner + assoc_type_id`, and owner mismatches are rejected.
- [ ] Monomorphization same-name behavior is proven by tests or fixed so semantic lookup is by IDs, receiver args, trait args, method IDs, or instance origins.
- [ ] Codegen same-name behavior is proven by tests or fixed so backend names are never semantic selectors when a target/instance ID exists.
- [ ] Product artifact loading validates and remaps every serialized ID-bearing type field before downstream phases use it.
- [ ] Product export/display names cannot overwrite distinct IDs or become semantic identity.
- [ ] `master-audit-checklist.md` is updated only for items proven by the audit.
- [ ] `TypeId` interning remains explicitly deferred and unchecked unless a separate approved plan implements it.
- [ ] Focused identity, same-name, artifact, and export-collision tests pass.
- [ ] `cargo fmt --all --check`, `git diff --check`, `cargo test -p rock-lib`, and `cargo test -p rock` pass.
- [ ] A focused post-fix review reports no Critical or Important Phase 6 Task 6 findings.

---

## Known Risks

- The stale search is intentionally broad. It catches many safe `Type::Generic(GenericParamId)` and `Type::TypeVar(TypeVarId)` uses. The danger is not the match itself, but failing to distinguish safe ID-backed construction from old string identity.
- Mono and codegen still need careful review because backend symbol maps are necessarily string-keyed. The soundness requirement is that those strings are outputs of prior semantic selection, not inputs to semantic selection.
- The master audit checklist is stale relative to recent Task 5 work. Updating it too early would create another false completion claim. It should be changed only after the Task 6 audit table is complete.
- Whole-type `TypeId` interning is intentionally out of scope. Task 6 can complete without making `TypeId` authoritative.

---

## Deliverables

1. A Task 6 implementation plan with a file-by-file audit table and step-by-step checklist.
2. Any red-green fixes required by disallowed stale identity paths found during audit.
3. An updated `master-audit-checklist.md` reflecting only proven Phase 6 completion items.
4. Fresh verification output and focused review evidence.
