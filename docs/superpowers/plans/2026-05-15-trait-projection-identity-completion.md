# Trait Projection Identity Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish Phase 6 Task 5 by removing the remaining semantic name-based trait/projection paths and proving same-name traits, associated types, default methods, and artifacts use canonical IDs.

**Architecture:** Keep the existing HIR and `Type` model. Add the smallest missing ID-backed seams: HIR method calls carry selected trait/impl identity, function generic bounds carry `TraitBound`s, builtin trait lookups use alias-aware canonical resolution, and artifact loading validates remapped projection identities. Names remain for source lookup, diagnostics, export metadata, and backend symbol construction only.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `DefId`s, `TraitBound`, `AssociatedTypeKey`, product artifact remapping, focused unit/integration tests, `cargo fmt --all --check`, `cargo test -p rock-lib`.

---

## File Structure

- Modify `lib/src/hir/mod.rs`: add ID-bearing generic-bound storage and method-call identity sidecars while preserving display names.
- Modify `lib/src/collect/headers.rs` and `lib/src/lower/function.rs`: resolve where-clause trait bounds to `TraitBound` during header/signature lowering.
- Modify `lib/src/lower/mod.rs`, `lib/src/lower/bodies.rs`, and `lib/src/lower/control_flow/secondary.rs`: carry ID-backed generic bounds during body lowering and reject or disambiguate name-only trait method lookup.
- Modify `lib/src/lower/expression.rs`: make builtin unary/binary operator trait lookup use canonical trait IDs and require matching impl IDs.
- Modify `lib/src/lower/traits/conformance.rs`: require trait impls to have resolved `trait_id`, compare associated type declarations by ID, and substitute trait generics/default projections into injected default methods.
- Modify `lib/src/mono/mod.rs` and `lib/src/mono/methods.rs`: stop grouping trait impls by trait name for semantic dispatch; use `DefId` where selected trait identity is available.
- Modify `lib/src/codegen/mod.rs`, `lib/src/codegen/expr/mod.rs`, and `lib/src/codegen/types.rs`: use method-call identity sidecars for trait impl backend selection instead of `Type_method` aliases.
- Modify `lib/src/crate_artifact/load.rs` and `lib/src/crate_artifact/tests.rs`: validate projection trait IDs and associated type IDs during artifact remap.
- Modify `lib/src/products.rs`: make impl method export identity collision-safe and keep exact-ID default-body selection.
- Modify `lib/tests/integration.rs`: add user-visible regressions for same-name traits, operator trait identity, trait default generic substitution, and artifact consumption.

---

### Task 1: Record Selected Method Identity In HIR

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/codegen/expr/mod.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing tests for ambiguous same-name trait methods**

Add `test_same_name_trait_methods_do_not_dispatch_by_method_name_only` in `lib/tests/integration.rs`. The Rock program should define two traits with the same receiver method name on one concrete type, call the method through a context that requires the second trait, and assert the second impl result. It fails before the fix because lowering/codegen can select by method name or `Type_method` alias.

Run:

```bash
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact --nocapture
```

Expected before implementation: FAIL, either wrong exit/output or ambiguous/wrong method dispatch.

- [ ] **Step 2: Add method identity sidecar**

Add a small serializable sidecar in `lib/src/hir/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HirMethodCallTarget {
    pub impl_id: Option<DefId>,
    pub trait_id: Option<DefId>,
    pub method_id: DefId,
}
```

Change `HirExprKind::MethodCall` to include `Option<HirMethodCallTarget>` after the receiver mode.

- [ ] **Step 3: Populate method targets during lowering**

When `concrete_method_candidate` or operator lowering selects a concrete impl method, populate `impl_id`, `trait_id`, and `method_id`. Inherent method calls should set `trait_id: None` and `impl_id: Some(imp.id)` when the method came from an impl, or `None` for old direct function-map fallback.

- [ ] **Step 4: Use method targets in codegen**

In method-call codegen, if a target has `impl_id` or `trait_id`, resolve the backend name from the selected impl/method instead of checking `Type_method` aliases first. Keep the old alias fallback only when the target is `None`.

- [ ] **Step 5: Verify focused test passes**

Run the test from Step 1. Expected: PASS.

---

### Task 2: Make Generic Function Bounds ID-Backed

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/bodies.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Test: `lib/src/lower/control_flow/secondary.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing same-name generic-bound test**

Add `same_named_traits_do_not_satisfy_each_other_by_name` as a lower-level test that constructs two `HirTrait`s with the same display name and distinct `DefId`s, records a bound to one, and verifies generic method lookup does not select the other.

Run:

```bash
cargo test -p rock-lib same_named_traits_do_not_satisfy_each_other_by_name
```

Expected before implementation: FAIL because generic-bound lookup still consumes `HashMap<String, Vec<String>>` names.

- [ ] **Step 2: Replace semantic bound storage**

Introduce `HirGenericBounds = HashMap<GenericParamId, Vec<TraitBound>>` in `lib/src/hir/mod.rs`, and use it for `HirFunction.generic_bounds`, `HirFunctionSig.generic_bounds`, and `Lowerer.current_impl_bounds`.

- [ ] **Step 3: Resolve where clauses once**

In collect/lower header code, resolve each where-clause trait name with `trait_by_name`, map the bounded generic name to its `GenericParamId`, and store `TraitBound { trait_id, type_args }`. Unknown trait names must push a structured lowering error and not create a name-only fallback.

- [ ] **Step 4: Update generic method lookup**

In `secondary.rs`, when the receiver is `Type::Generic(param)`, read `current_impl_bounds[&param]`, find traits by `trait_def.id == bound.trait_id`, and select methods/signatures from that trait only.

- [ ] **Step 5: Verify focused tests pass**

Run the tests from Step 1 plus existing generic-bound tests. Expected: PASS.

---

### Task 3: Require Trait Impl IDs And ID-Backed Associated Type Conformance

**Files:**
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing unknown-trait impl test**

Add `test_trait_impl_rejects_unknown_trait_name` in `lib/tests/integration.rs`. The source should declare `struct Box` and `impl MissingTrait for Box` with a method body. Expected compile status is failure with an unknown trait diagnostic.

Run:

```bash
cargo test -p rock-lib --test integration test_trait_impl_rejects_unknown_trait_name -- --exact --nocapture
```

Expected before implementation: FAIL if the impl is silently retained with `trait_id: None`.

- [ ] **Step 2: Make named trait impl resolution mandatory**

When `imp.for_.is_some()` and `trait_by_name` returns `None`, push a resolve error and keep the impl from being semantically processed as a trait impl. In conformance, if `trait_name.is_some()` and `trait_id.is_none()`, emit a structured error instead of using name fallback.

- [ ] **Step 3: Compare associated type declarations by ID**

In conformance, required/unknown associated type checks should compare `AssocTypeId` under the selected trait ID. Use associated type names only for diagnostic text.

- [ ] **Step 4: Verify focused tests pass**

Run the test from Step 1 plus `cargo test -p rock-lib --test integration test_trait_impl_rejects_unknown_associated_type_definition -- --exact --nocapture`.

---

### Task 4: Canonicalize Builtin Operator Trait Lookup

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing unrelated-operator-trait test**

Add `test_binary_operator_requires_builtin_trait_identity` in `lib/tests/integration.rs`. The Rock program should define an unrelated trait with method `+`, implement it for a type, omit the required `Num` impl, and assert compilation fails for `value + value`.

Run:

```bash
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact --nocapture
```

Expected before implementation: FAIL because the unrelated `+` method may satisfy the operator.

- [ ] **Step 2: Add canonical builtin trait resolver**

Add a `builtin_trait_by_name(&self, name: &str) -> Option<&HirTrait>` helper that prefers explicit prelude/export IDs when present, then `trait_by_name`, and never silently accepts an unrelated same-named trait when a stdlib/prelude ID is known.

- [ ] **Step 3: Require matching trait IDs for builtin operators and indexing**

Update binary, unary, and index lowering to resolve the builtin trait ID and require selected impls to match it. Store `TraitBound { trait_id, type_args }` using the resolved ID.

- [ ] **Step 4: Verify focused operator tests pass**

Run the test from Step 1 plus unary/index associated-output regressions.

---

### Task 5: Substitute Trait Generics In Injected Default Methods

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing generic default method test**

Add `test_trait_default_method_substitutes_trait_generic_output` in `lib/tests/integration.rs`. The Rock program should define a generic trait with a default method returning a trait generic or associated output, implement the trait without overriding the method, and call it on a concrete receiver.

Run:

```bash
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_trait_generic_output -- --exact --nocapture
```

Expected before implementation: FAIL if trait-owned generics/projections remain in the injected default method.

- [ ] **Step 2: Apply full substitution to defaults**

When injecting a default method, recursively substitute `GenericParamId`s and projections using the same `trait_generic_subst`, `impl_trait_id`, and associated-type definitions used for required signatures. Apply the substitution to params, return type, body type, and body expressions.

- [ ] **Step 3: Verify focused default tests pass**

Run the test from Step 1 plus existing trait default integration tests.

---

### Task 6: Validate Artifact Projection Identity And Export Collisions

**Files:**
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_artifact/tests.rs`
- Modify: `lib/src/products.rs`
- Test: `lib/src/crate_artifact/tests.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Write failing artifact projection remap tests**

Add tests named `product_artifact_remaps_projection_type_ids`, `product_artifact_rejects_unknown_projection_trait_id`, and `product_artifact_rejects_unknown_projection_assoc_type_id` near existing artifact tests.

Run:

```bash
cargo test -p rock-lib product_artifact_remaps_projection_type_ids
cargo test -p rock-lib product_artifact_rejects_unknown_projection_trait_id
cargo test -p rock-lib product_artifact_rejects_unknown_projection_assoc_type_id
```

Expected before implementation: first test may pass only for raw `Type::remap_def_ids`; validator tests fail because artifact load accepts invalid projection IDs.

- [ ] **Step 2: Add product trait validator**

Add a validator that records product trait IDs and declared associated type IDs from `CompilerProducts.metadata.traits`. During `remap_type_def_ids`, validate local projection `trait_id`, `assoc_type.owner`, and `assoc_type.assoc_type_id` before remapping.

- [ ] **Step 3: Make impl method export names collision-safe**

When recording impl method export names, include a stable product-ID disambiguator or skip ambiguous export names. Do not allow a later same-display method to overwrite a different method ID silently.

- [ ] **Step 4: Verify artifact/product tests pass**

Run the tests from Step 1 plus product default identity tests.

---

### Task 7: Verification And Final Review

**Files:**
- Read/review all changed files

- [ ] **Step 1: Run focused tests**

Run every focused test added in Tasks 1-6 and the existing associated-output regressions.

- [ ] **Step 2: Run formatting and full test suite**

Run:

```bash
cargo fmt --all --check
cargo test -p rock-lib
```

Expected: all pass with `0` failures.

- [ ] **Step 3: Run semantic identity searches**

Run targeted searches for stale semantic strings:

```bash
rg "trait_name: String|assoc_name: String|HashMap<String, Vec<String>>|self\.traits\.get\(\"(Num|Ord|Eq|Bitwise|Neg|Not|Index)\"\)" lib/src
```

Expected: remaining hits are either display/source metadata or intentionally documented lookup helpers, not semantic comparison/dispatch paths.

- [ ] **Step 4: Request post-fix review**

Run a final review focused on same-name trait soundness, artifact remapping, default methods, and codegen dispatch.
