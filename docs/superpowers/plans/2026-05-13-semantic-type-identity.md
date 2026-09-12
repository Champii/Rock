# Semantic Type Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Phase 6 of the canonical identity roadmap by replacing string-bearing semantic type identity with canonical ID-backed identity in the existing `Type` representation, while keeping source names only for lookup, diagnostics, display, and backend symbol generation.

**Architecture:** Phase 6 should mutate the existing `Type` model in place rather than introducing a parallel long-lived `Ty` model. `Type` is already stored in HIR, serialized into product artifacts, consumed by inference, monomorphization, MIR, and codegen, so a second representation would create a conversion boundary and two semantic truth sources. Do not make `TypeId` interning authoritative in this phase. First make the identity fields inside `Type` canonical IDs; whole-type interning can be a later optimization once semantic identity is sound.

**Tech Stack:** Rust 2021, `rock-lib`, `lib/src/types/mod.rs`, HIR `DefId`s, `TypeVarId`, artifact DefId remapping, inference constraints, monomorphization substitution, codegen type/projection resolution, focused unit tests, artifact integration tests, `cargo fmt --all --check`, `cargo test -p rock-lib`.

---

## File Structure

- Modify `lib/src/types/mod.rs`: replace string-bearing identity fields in `Type` and `TraitBound`; introduce small serializable identity keys for generic params and associated types; update substitution, traversal, display, equality/hash tests, and helper methods.
- Modify `lib/src/ids.rs` only if a new typed ID is genuinely needed. Prefer composite serializable keys such as `{ owner: DefId, index: u32 }` for generic params because generic params are owner-scoped and must remap with artifacts.
- Modify `lib/src/lower/types.rs`: resolve parsed type names once into canonical IDs when lowering to `Type`; stop producing nominal `Type`s keyed by source strings.
- Modify `lib/src/lower/**`: update trait-bound and associated-type projection lowering so trait and associated-type identity is ID-backed.
- Modify `lib/src/hir/mod.rs`: update HIR tests/constructors and any helper logic that currently assumes type generics are semantic strings. Keep HIR declaration names available for display and lookup.
- Modify `lib/src/infer/**`: replace raw `u32` type-variable keys with `TypeVarId`; update constraint solving and trait-bound checks to use ID-backed trait/type identity.
- Modify `lib/src/mono/**`: update generic substitution maps and type matching/suffix helpers so semantic equality uses IDs while backend names remain derived display strings.
- Modify `lib/src/mir/**` only where pattern matches on `Type::Struct`, `Type::Enum`, `Type::Generic`, `Type::TypeVar`, or `Type::Projection` require shape updates.
- Modify `lib/src/codegen/**`: update type lowering, projection resolution, struct/enum lookup, and backend suffix generation to use canonical IDs for semantic lookup and names only for LLVM symbol/display construction.
- Modify `lib/src/crate_artifact/load.rs` and nearby artifact tests: recursively remap every `DefId` embedded in serialized `Type`s, including nominal type IDs, generic-param owners, trait IDs, and associated-type owners.
- Modify `lib/src/products.rs` and `rock-shared/src/sysroot.rs` only if serialized artifact layout changes. If `Type` layout changes in serialized HIR, bump the artifact format version and add the focused version/remap tests in the same task.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: mark only the semantic type identity items completed by this plan; keep `TypeId` interning unchecked if deliberately deferred.

---

## Out Of Scope

Do not do these in Phase 6:

- Do not introduce a parallel long-lived `Ty` model alongside `Type`.
- Do not make whole-type `TypeId` interning authoritative unless every ID-backed identity field is already migrated and tests prove a clear need.
- Do not redesign trait selection into a new global service beyond the minimal ID-backed lookup seams needed here.
- Do not migrate codegen to a MIR-only backend boundary.
- Do not change parser IO, module loading, formatter trivia, borrowck dataflow, stdlib discovery, or implicit stdlib loading.
- Do not preserve string-identity compatibility shims for unreleased internal shapes. Source/display names may remain only for lookup, diagnostics, export metadata, and backend symbol generation.

---

## Task 1: Convert Inference Type Variables To `TypeVarId`

**Files:**
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/infer/engine.rs`
- Modify: `lib/src/infer/constraints.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/src/infer/finalize.rs`
- Modify: `lib/src/infer/generalize.rs`
- Modify nearby tests in `lib/src/infer/**` and `lib/src/types/mod.rs`

- [ ] **Step 1: Write failing `TypeVarId` tests**

Add focused tests proving:
- `InferenceEngine::fresh_type_var` and `fresh_type_var_at` produce `Type::TypeVar(TypeVarId(...))`, not raw `u32`.
- `InferenceEngine::resolve` and `Type::substitute` use `HashMap<TypeVarId, Type>`.
- `TraitBound` collection for unresolved type variables records bounds under `TypeVarId` keys.

Suggested filters:

```bash
cargo test -p rock-lib type_var_uses_typed_id_in_inference
cargo test -p rock-lib type_substitution_uses_type_var_id_keys
cargo test -p rock-lib solve_constraints_reports_generic_bounds_by_type_var_id
```

- [ ] **Step 2: Run focused tests and confirm RED**

Expected before implementation: compile failures or assertions showing `Type::TypeVar(u32)` and `HashMap<u32, Type>` are still used.

- [ ] **Step 3: Change `Type::TypeVar` payload**

Change `Type::TypeVar(u32)` to `Type::TypeVar(TypeVarId)` in `lib/src/types/mod.rs` and update imports.

- [ ] **Step 4: Update inference storage and APIs**

Update `InferenceEngine` fields and methods:
- `next_var` should use `IdGen<TypeVarId>` or a typed counter that returns `TypeVarId`.
- `substitutions`, `trait_bounds`, and `var_spans` should key by `TypeVarId`.
- `fresh_type_var`, `fresh_type_var_at`, `resolve`, `unify`, occurs-check, and diagnostics should use typed IDs.

- [ ] **Step 5: Update solver/finalization/generalization callers**

Replace all raw type-var IDs in infer modules with `TypeVarId`. Do not convert typed IDs back to raw `u32` except in display/debug output.

- [ ] **Step 6: Run focused tests and confirm GREEN**

Run the tests from Steps 1-2 again.

---

## Task 2: Add Canonical Type Identity Keys And Traversal Helpers

**Files:**
- Modify: `lib/src/types/mod.rs`
- Modify if needed: `lib/src/ids.rs`
- Modify tests in `lib/src/types/mod.rs`

- [ ] **Step 1: Write failing identity-key tests**

Add tests for the new key types:
- `GenericParamId { owner: DefId, index: u32 }` equality/hash ignores display names because it stores no display name.
- `AssociatedTypeKey { owner: DefId, assoc_type_id: AssocTypeId }` distinguishes same-named associated types on different traits/impls.
- Key owner remapping updates `GenericParamId` and `AssociatedTypeKey` owners.
- Current `Type::remap_def_ids` recursively walks nested type containers and remains a no-op for today's string-bearing `Type` shape until later tasks introduce `DefId`-bearing type variants.

Suggested filters:

```bash
cargo test -p rock-lib generic_param_identity_is_owner_and_index
cargo test -p rock-lib associated_type_key_is_owner_scoped
cargo test -p rock-lib identity_key_remap_updates_owner_fields
cargo test -p rock-lib type_def_id_remap_is_noop_for_current_string_bearing_type_shape
```

- [ ] **Step 2: Run focused tests and confirm RED**

Expected before implementation: missing key types or missing traversal helpers.

- [ ] **Step 3: Introduce key structs**

In `lib/src/types/mod.rs`, add serializable ID-backed structs:

```rust
pub struct GenericParamId {
    pub owner: DefId,
    pub index: u32,
}

pub struct AssociatedTypeKey {
    pub owner: DefId,
    pub assoc_type_id: AssocTypeId,
}
```

Only add a new `ids.rs` typed ID if implementation evidence shows owner/index is insufficient. Do not use `TypeId` as a whole-type key yet.

- [ ] **Step 4: Add recursive remap/traversal helpers**

Add key remap helpers and a helper on `Type` for recursively remapping embedded `DefId`s. The `Type` helper is intentionally a no-op for current string-bearing identity fields, but it should recurse through nested containers now so later tasks can add ID-bearing fields locally. It should eventually cover:
- nominal struct/enum IDs.
- generic-param owner IDs.
- trait IDs in trait bounds and projections.
- associated-type owner IDs.
- nested type arguments.

It may be partially unused until later tasks, but tests must prove each migrated identity field is visited when introduced.

- [ ] **Step 5: Run focused key/traversal tests and confirm GREEN**

Run all tests from Steps 1-2.

---

## Task 3: Migrate Nominal Struct And Enum Types To `DefId`

**Files:**
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/infer/**` where nominal type matching occurs
- Modify: `lib/src/mono/**` where nominal type matching/suffixing occurs
- Modify: `lib/src/mir/**` where nominal type pattern matches occur
- Modify: `lib/src/codegen/**` where struct/enum layout lookup occurs
- Modify: `lib/src/crate_artifact/load.rs`
- Modify artifact/integration tests as needed

- [x] **Step 1: Write failing nominal identity tests**

Add tests proving:
- Lowering `ParseTypeInner { name: ... }` for known structs/enums returns `Type::Struct { id, args }` or `Type::Enum { id, args }` with the declaration `DefId`.
- Two structs/enums with the same display name but different `DefId`s are not equal.
- A product artifact dependency remaps nominal type IDs in serialized HIR types before downstream use.

Suggested filters:

```bash
cargo test -p rock-lib lower_nominal_type_uses_decl_def_id
cargo test -p rock-lib same_named_nominal_types_are_distinct_by_def_id
cargo test -p rock-lib product_artifact_remaps_nominal_type_ids_in_hir_types
```

- [x] **Step 2: Run focused tests and confirm RED**

Expected before implementation: `Type::Struct(String, ...)` / `Type::Enum(String, ...)` remain string-keyed.

- [x] **Step 3: Change nominal `Type` variants**

Replace:

```rust
Struct(String, Vec<Type>)
Enum(String, Vec<Type>)
```

with ID-backed variants such as:

```rust
Struct { id: DefId, args: Vec<Type> }
Enum { id: DefId, args: Vec<Type> }
```

Do not store names in the equality-bearing variant. Display names should be recovered from HIR/resolver context where available, or fall back to a stable debug representation for context-free `Display`.

- [x] **Step 4: Update type lowering**

Update `lower/types.rs` so source names resolve once:
- structs use `HirStruct.id` / resolver `DefId`.
- enums use `HirEnum.id` / resolver `DefId`.
- unknown names remain generic parameters until Task 4.

- [x] **Step 5: Update all nominal pattern matches**

Update inference, monomorphization, MIR, and codegen matches. Where code needs a display or backend name, look it up from `HirProgram` indexes, resolver tables, declaration maps, or explicit backend-name helpers. Do not use names as semantic keys.

- [x] **Step 6: Update artifact type remapping**

Recursively remap nominal `DefId`s in every serialized `Type` inside artifact interfaces and cross-crate HIR. If serialized HIR layout changes, bump `PRODUCT_ARTIFACT_FORMAT_VERSION` in both `lib/src/products.rs` and `rock-shared/src/sysroot.rs` and add/update the version-contract tests.

- [x] **Step 7: Run nominal identity tests and confirm GREEN**

Run all tests from Steps 1-2 plus nearby type display/codegen tests:

```bash
cargo test -p rock-lib test_display_formats_slice_and_fixed_array_differently
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle
```

---

## Task 4: Migrate Generic Parameter Identity

**Files:**
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/**`
- Modify: `lib/src/infer/generalize.rs`
- Modify: `lib/src/mono/substitute.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/specialize.rs`
- Modify artifact remap helpers if generic-param owners are serialized

- [ ] **Step 1: Write failing generic-param identity tests**

Add tests proving:
- Two generic params named `T` under different owners are distinct semantic types.
- Substitution maps key by `GenericParamId`, not by `String`.
- Artifact loading remaps generic-param owner IDs inside serialized `Type::Generic` values.

Suggested filters:

```bash
cargo test -p rock-lib generic_params_with_same_name_are_distinct_by_owner
cargo test -p rock-lib substitute_generics_uses_generic_param_identity
cargo test -p rock-lib product_artifact_remaps_generic_param_owners_in_hir_types
```

- [ ] **Step 2: Run focused tests and confirm RED**

Expected before implementation: generic params are still `Type::Generic(String)` and substitutions use `HashMap<String, Type>`.

- [ ] **Step 3: Change `Type::Generic`**

Replace `Type::Generic(String)` with `Type::Generic(GenericParamId)`.

- [ ] **Step 4: Add owner/index construction helpers**

Add helpers that derive `GenericParamId` from the declaration owner and parameter position. Use owner `DefId` from:
- `HirFunction.id` for function generics.
- `HirStruct.id` / `HirEnum.id` for nominal type generics.
- `HirTrait.id` for trait generics.
- `HirImpl.id` for impl generics.

Keep generic parameter names in declaration `generic_params: Vec<String>` for source syntax, diagnostics, and display.

- [ ] **Step 5: Update substitution maps**

Convert substitution maps from `HashMap<String, Type>` to `HashMap<GenericParamId, Type>` in:
- `Type::substitute_generics`.
- HIR substitution helpers.
- monomorphization specialization/substitution.
- associated-type projection substitution.

- [ ] **Step 6: Update inference/generalization**

When generalization turns unresolved type vars into generic params, assign deterministic `GenericParamId`s tied to the owning function or impl and parameter index. Preserve display names separately.

- [ ] **Step 7: Run generic-param tests and confirm GREEN**

Run all tests from Steps 1-2 plus existing monomorphization generic tests:

```bash
cargo test -p rock-lib test_compile_generic_function_from_artifact_hir_bundle
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle
```

---

## Task 5: Migrate Trait Bounds And Associated-Type Projections

**Files:**
- Modify: `lib/src/types/mod.rs`
- Modify: `lib/src/lower/types.rs`
- Modify: `lib/src/lower/types_helpers/**`
- Modify: `lib/src/infer/constraints.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/src/mono/**`
- Modify: `lib/src/codegen/types.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify tests for trait bounds/projections/artifacts

- [ ] **Step 1: Write failing trait/projection identity tests**

Add tests proving:
- `TraitBound` stores canonical trait ID plus ID-backed type args, not a trait name string.
- `Type::Projection` stores `trait_id: DefId` and `assoc_type: AssociatedTypeKey`, not `trait_name`/`assoc_name` strings.
- Two traits with the same display name but different `DefId`s do not satisfy each other's bounds or projections.
- Artifact loading remaps trait/projection IDs embedded in serialized HIR types.

Suggested filters:

```bash
cargo test -p rock-lib trait_bound_identity_uses_trait_def_id
cargo test -p rock-lib projection_identity_uses_trait_and_assoc_type_ids
cargo test -p rock-lib same_named_traits_do_not_satisfy_each_other_by_name
cargo test -p rock-lib product_artifact_remaps_projection_type_ids
```

- [ ] **Step 2: Run focused tests and confirm RED**

Expected before implementation: `TraitBound { trait_name: String }` and `Type::Projection { trait_name, assoc_name, ... }` still exist.

- [ ] **Step 3: Change `TraitBound`**

Replace `trait_name: String` with `trait_id: DefId`. Keep type args as `Vec<Type>`.

- [ ] **Step 4: Change projection `Type` variant**

Replace string projection fields with ID-backed fields, for example:

```rust
Projection {
    ty: Box<Type>,
    trait_id: DefId,
    assoc_type: AssociatedTypeKey,
    trait_args: Vec<Type>,
}
```

- [ ] **Step 5: Update lower and infer trait lookup**

Resolve trait and associated type source names once during lowering using resolver/HIR declaration IDs. Missing canonical trait or associated-type IDs should be structured compiler errors, not fallback strings.

- [ ] **Step 6: Update mono/codegen projection resolution**

Change projection resolution and trait impl matching to use IDs. Backend/display names can still be looked up for symbols and diagnostics, but semantic comparisons must be ID-based.

- [ ] **Step 7: Update artifact remapping**

Recursively remap trait IDs and associated-type owner IDs inside `TraitBound` and `Type::Projection` stored in artifact HIR.

- [ ] **Step 8: Run trait/projection tests and confirm GREEN**

Run all tests from Steps 1-2 plus existing associated type regressions:

```bash
cargo test -p rock-lib test_trait_impl_requires_associated_type_definition
cargo test -p rock-lib test_trait_impl_body_must_match_associated_type_signature
cargo test -p rock-lib test_unary_neg_dispatches_through_trait_with_associated_output
```

---

## Task 6: Remove Remaining String-Based Type Identity Construction

**Files:**
- Modify: `lib/src/types/mod.rs`
- Modify all files found by the searches below
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Search for stale identity constructors**

Run:

```bash
rg "Type::Struct\(|Type::Enum\(|Type::Generic\(|Type::TypeVar\([0-9a-zA-Z_]+\)|trait_name: String|assoc_name: String|HashMap<String, Type>" lib/src
```

- [ ] **Step 2: Classify remaining string usage**

Allowed remaining uses:
- source parse/type names before lowering.
- declaration maps and export/import lookup strings.
- diagnostics and display helpers.
- backend symbol generation after semantic identity has already selected the entity.

Disallowed remaining uses:
- equality/hash-bearing `Type` identity fields.
- trait/projection semantic comparisons.
- substitution keys for generic params.
- artifact-exposed type identity fields that skip DefId remapping.

- [ ] **Step 3: Remove or update stale constructors**

Update all remaining disallowed sites. Do not add compatibility constructors that rebuild string identity.

- [ ] **Step 4: Update audit checklist**

Mark completed Type Context / Semantic Types items only after the search proves no string-bearing semantic identity remains. Keep `TypeId` interning unchecked if it remains deferred.

- [ ] **Step 5: Run stale identity search again**

The remaining matches should be explainable by the allowed-use list.

---

## Task 7: Full Product, Artifact, And Compiler Verification

**Files:**
- Modify if needed: `lib/src/products.rs`
- Modify if needed: `rock-shared/src/sysroot.rs`
- Modify tests adjacent to any artifact format/version changes

- [ ] **Step 1: Run all focused tests from Tasks 1-6**

Run each exact focused test filter added in this plan.

- [ ] **Step 2: Run artifact compatibility tests**

Run at least:

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract
cargo test -p rock-lib compiler_products_rejects_unsupported_format_before_full_deserialize
cargo test -p rock-lib test_compile_generic_function_from_artifact_hir_bundle
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle
cargo test -p rock-lib test_compile_generic_impl_from_file_module_artifact_hir_bundle
```

- [ ] **Step 3: Run related integration tests**

Run integration filters covering nominal types, traits, associated types, projections, and generic monomorphization:

```bash
cargo test -p rock-lib --test integration test_trait_impl_requires_associated_type_definition -- --exact
cargo test -p rock-lib --test integration test_unary_neg_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_vec_get_option -- --exact
```

If an exact filter does not exist in `tests/integration.rs`, replace it with the actual nearest filter and record the substitution in the implementation notes. Artifact-specific regressions are covered in Step 2.

- [ ] **Step 4: Run diagnostics on changed Rust files**

Run `lsp_diagnostics` on every changed Rust file before broad tests.

- [ ] **Step 5: Run formatting and full library tests**

```bash
cargo fmt --all --check
cargo test -p rock-lib
```

- [ ] **Step 6: Manual QA through `rockc`**

Create a temporary Rock program under `/tmp/opencode` that exercises:
- a generic nominal type.
- a trait bound or associated-type projection.
- a method call that monomorphizes through the ID-backed type identity path.

Then run:

```bash
cargo run -p rockc -- --entry-file /tmp/opencode/phase6_semantic_type_identity.rk --no-link
```

- [ ] **Step 7: Post-implementation review**

Request a review focused on:
- no remaining semantic string identity in `Type`.
- artifact remapping covers every new `DefId` embedded in serialized types.
- backend names remain separate from semantic type equality.
- tests prove same-name/different-ID type and trait cases.

---

## Success Criteria

Phase 6 is complete only when:

- `Type::Struct`, `Type::Enum`, `Type::Generic`, `Type::TypeVar`, `TraitBound`, and `Type::Projection` no longer carry string/raw-ID semantic identity.
- Type lowering resolves nominal, generic, trait, and associated-type source names into canonical semantic IDs exactly once.
- Inference, trait constraints, monomorphization, MIR, and codegen compare semantic types by IDs, not names.
- Artifact loading remaps every `DefId` embedded in serialized HIR `Type`s before downstream phases see them.
- Same-named types and traits from different owners/crates cannot collide in type equality, trait bounds, projections, monomorphization, or codegen lookup.
- Names remain available only for source lookup, diagnostics, import/export metadata, and backend symbol generation.
- `cargo fmt --all --check`, `cargo test -p rock-lib`, and the manual `rockc` QA command pass.

---

## Notes For Implementers

- Expect a large compile-fail wave after each `Type` variant shape change. Keep each task small enough that the compiler errors point to one identity dimension at a time.
- Do not start with whole-type interning. `TypeId` remains scaffolding until identity fields are canonical and there is a proven need for interned structural types.
- Do not store display names inside equality-bearing type variants. If context-free `Display` becomes less pretty temporarily, prefer a deterministic debug representation over reintroducing names as semantic fields.
- Artifact remapping is not optional. Any `DefId` added to serialized `Type` must be included in the product artifact remap path in the same task.
- If the implementation discovers that a proposed exact test name is impossible because the language surface cannot express the collision, replace it with a lower-level unit test over HIR/lowering/artifact structures and keep one user-visible regression for the nearest supported behavior.
