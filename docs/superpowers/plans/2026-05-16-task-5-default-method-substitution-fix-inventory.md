# Task 5 Default Method Substitution Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish Task 5 by making injected trait default methods fully substitute trait generics, `Self`, associated projections, and all HIR metadata that can affect type checking, monomorphization, artifacts, or codegen.

**Architecture:** Keep substitution in `lib/src/lower/traits/conformance.rs`, because conformance owns default method injection. Add only small local traversal helpers for missing HIR carriers, reuse existing `Type` and `TraitBound` structures, and avoid name-based fallbacks. Validate with focused unit tests for HIR metadata and integration tests for user-visible default-method behavior.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `Type`, `GenericParamId`, `HirMethodCallTarget`, `TraitBound`, associated type projections, product artifacts, `cargo test -p rock-lib`.

---

## Current Status

- Scope: Task 5 from `docs/superpowers/plans/2026-05-15-trait-projection-identity-completion.md`.
- Task 5 spec line: recursively substitute `GenericParamId`s and projections in injected defaults, including params, return type, body type, and body expressions.
- Current branch at audit time: `ohmyopenagent`.
- Current tracked tree at audit time: clean except this document once added.
- Current ignored/untracked item: `.sisyphus/` remains untracked and must not be touched.
- Current Task 5 verdict before these fixes: partially implemented, not complete, not fully spec-compliant, not fully sound.

## Severity Legend

- Critical: can produce unsound or wrong semantic HIR after default injection.
- Important: can leave stale trait generics/projections in valid HIR paths or break cross-phase behavior.
- Minor: fragility, missing validation, or coverage gap that may hide future regressions.

---

## Exhaustive Fix Inventory

### TS5-001: Restrict Associated Projection Replacement To Impl `Self`

**Severity:** Critical

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:149-218`
- Test: `lib/src/lower/traits/conformance.rs`

**Current problem:**
`substitute_self` replaces a projection when this condition is true:

```rust
matches!(**ty, Type::Generic(_)) || substituted_ty == *impl_type
```

That treats any generic projection base as if it were the impl receiver. A default method containing a projection like `<T as Trait>::Assoc` can be replaced with the associated type from `impl Trait for Self`, which is wrong unless the projection base is actually trait `Self` after substitution.

**Required fix:**
Only replace a projection with an impl associated type when the substituted projection base equals the concrete impl receiver type.

**Minimal implementation shape:**

```rust
let projection_base_is_impl_self = substituted_ty == *impl_type;

if projection_base_is_impl_self
    && impl_trait_id == Some(*trait_id)
    && assoc_type.owner == *trait_id
    && trait_args_match
{
    if let Some(assoc) = impl_associated_types
        .iter()
        .find(|item| item.id == assoc_type.assoc_type_id)
    {
        return substitute_self(
            &assoc.ty,
            impl_type,
            impl_trait_id,
            impl_trait_arg_types,
            impl_associated_types,
            generic_subst,
        );
    }
}
```

**Required tests:**
- `conformance_does_not_substitute_projection_on_non_self_generic_base`
- The test should build a default method with `Type::Projection { ty: Box::new(Type::Generic(T)), trait_id, assoc_type, trait_args }` and assert the injected method still contains a projection, not the impl receiver associated type.

**Expected current red failure:**
The projection incorrectly becomes the impl associated type.

**Focused command:**

```bash
cargo test -p rock-lib conformance_does_not_substitute_projection_on_non_self_generic_base -- --nocapture
```

### TS5-002: Recurse Into `Type::Struct` Generic Arguments

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:67-220`
- Reference: `lib/src/types/mod.rs:73-77`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
`substitute_self` does not handle `Type::Struct { id, args }`. Default method types such as `Box<T>`, `Box<Self>`, or `Box<Self::Output>` can retain trait-owned generics or projections inside `args`.

**Required fix:**
Add a `Type::Struct` match arm that recursively substitutes every type argument.

**Minimal implementation shape:**

```rust
Type::Struct { id, args } => Type::Struct {
    id: *id,
    args: args
        .iter()
        .map(|arg| {
            substitute_self(
                arg,
                impl_type,
                impl_trait_id,
                impl_trait_arg_types,
                impl_associated_types,
                generic_subst,
            )
        })
        .collect(),
},
```

**Required tests:**
- `conformance_substitutes_default_method_nested_struct_trait_generic_types`
- `test_trait_default_method_substitutes_trait_generic_in_nested_struct_return`

**Expected current red failure:**
The injected default keeps `Type::Struct { args: [Type::Generic(GenericParamId { owner: trait_id, .. })] }` instead of `Type::Struct { args: [Type::I64] }`.

**Focused commands:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_nested_struct_trait_generic_types -- --nocapture
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_trait_generic_in_nested_struct_return -- --exact --nocapture
```

### TS5-003: Recurse Into `Type::Enum` Generic Arguments

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:67-220`
- Reference: `lib/src/types/mod.rs:78-82`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
`substitute_self` does not handle `Type::Enum { id, args }`. Default method types such as `Maybe<T>`, `Result<Self::Output>`, or `Option<Self>` can retain trait-owned generics or projections inside enum arguments.

**Required fix:**
Add a `Type::Enum` match arm that recursively substitutes every type argument.

**Minimal implementation shape:**

```rust
Type::Enum { id, args } => Type::Enum {
    id: *id,
    args: args
        .iter()
        .map(|arg| {
            substitute_self(
                arg,
                impl_type,
                impl_trait_id,
                impl_trait_arg_types,
                impl_associated_types,
                generic_subst,
            )
        })
        .collect(),
},
```

**Required tests:**
- `conformance_substitutes_default_method_nested_enum_trait_generic_types`
- `test_trait_default_method_substitutes_trait_generic_in_nested_enum_return`

**Expected current red failure:**
The injected default keeps `Type::Enum { args: [Type::Generic(GenericParamId { owner: trait_id, .. })] }` instead of substituting the impl trait argument.

**Focused commands:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_nested_enum_trait_generic_types -- --nocapture
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_trait_generic_in_nested_enum_return -- --exact --nocapture
```

### TS5-004: Compare Projection Trait Args Against Impl Trait Args Directly

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:149-218`
- Modify call sites in `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
Projection substitution infers the current trait argument count from `generic_subst` by relying on the synthetic `Self` parameter being inserted at `trait_def.generic_params.len()`. This is implicit and fragile. It also makes the projection matching rule difficult to audit.

**Required fix:**
Pass `impl_trait_arg_types` into `substitute_self` and compare the substituted projection `trait_args` directly to those impl trait args.

**Minimal implementation shape:**

```rust
fn substitute_self(
    ty: &Type,
    impl_type: &Type,
    impl_trait_id: Option<DefId>,
    impl_trait_arg_types: &[Type],
    impl_associated_types: &[HirAssociatedTypeDef],
    generic_subst: &HashMap<GenericParamId, Type>,
) -> Type {
    // ...
    let trait_args_match = substituted_trait_args == impl_trait_arg_types;
    // ...
}
```

Every recursive call to `substitute_self` must pass `impl_trait_arg_types` through unchanged.

**Required tests:**
- `conformance_substitutes_default_method_projection_with_matching_trait_args`
- `test_trait_default_method_substitutes_generic_trait_self_output_projection`
- Keep existing negative test `conformance_does_not_substitute_projection_with_different_trait_args` passing.

**Expected current red failure:**
A positive generic-trait projection can remain a projection or fail a type assertion if the implicit count/matching rule is wrong. The negative mismatched-trait-args test should continue to prove over-substitution is blocked.

**Focused commands:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_projection_with_matching_trait_args -- --nocapture
cargo test -p rock-lib conformance_does_not_substitute_projection_with_different_trait_args -- --nocapture
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_generic_trait_self_output_projection -- --exact --nocapture
```

### TS5-005: Substitute `HirMethodCallTarget.trait_args` During Default Injection

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:399-416`
- Reference: `lib/src/hir/mod.rs:612-619`
- Test: `lib/src/lower/traits/conformance.rs`

**Current problem:**
`substitute_trait_impl_types_in_expr` rewrites the receiver and arguments for `HirExprKind::MethodCall`, but it does not rewrite `target.trait_args`. Those trait args are semantic dispatch metadata and are later consumed by mono/codegen.

**Required fix:**
Split the current combined `Call | MethodCall` arm and substitute `target.trait_args` when the method target is present.

**Minimal implementation shape:**

```rust
HirExprKind::MethodCall(func, _, args, _, target) => {
    substitute_trait_impl_types_in_expr(
        func,
        impl_type,
        impl_trait_id,
        impl_trait_arg_types,
        impl_associated_types,
        generic_subst,
    );
    for arg in args {
        substitute_trait_impl_types_in_expr(
            arg,
            impl_type,
            impl_trait_id,
            impl_trait_arg_types,
            impl_associated_types,
            generic_subst,
        );
    }
    if let Some(target) = target {
        for trait_arg in &mut target.trait_args {
            *trait_arg = substitute_self(
                trait_arg,
                impl_type,
                impl_trait_id,
                impl_trait_arg_types,
                impl_associated_types,
                generic_subst,
            );
        }
    }
}
```

**Required tests:**
- `conformance_substitutes_default_method_call_target_trait_args`

**Expected current red failure:**
The injected default method expression has concrete `expr.ty`, but `target.trait_args` still contains `Type::Generic(GenericParamId { owner: trait_id, .. })`.

**Focused command:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_call_target_trait_args -- --nocapture
```

### TS5-006: Substitute `HirMethodCallTarget.trait_args` In HIR TypeVar Substitution

**Severity:** Important

**Files:**
- Modify: `lib/src/hir/mod.rs:1292-1307`
- Caller: `lib/src/lower/traits/conformance.rs:929-933`
- Test: `lib/src/hir/mod.rs` or `lib/src/lower/traits/conformance.rs`

**Current problem:**
Before trait generic/projection substitution, injected defaults call `crate::hir::substitute_typevars_in_function`. That helper also skips `HirMethodCallTarget.trait_args`. If the target trait args contain default-body TypeVars or source self generics, conformance substitution may not see the expected concrete shape later.

**Required fix:**
Update the `HirExprKind::MethodCall` handling inside HIR typevar substitution to recurse through `target.trait_args`.

**Minimal implementation shape:**

```rust
HirExprKind::MethodCall(recv, _, args, _, target) => {
    substitute_typevars_in_expr_with_targets(
        recv,
        target_ids,
        target_generic_ids,
        concrete_ty,
    );
    for a in args.iter_mut() {
        substitute_typevars_in_expr_with_targets(
            a,
            target_ids,
            target_generic_ids,
            concrete_ty,
        );
    }
    if let Some(target) = target {
        for trait_arg in &mut target.trait_args {
            substitute_typevar_in_type(
                trait_arg,
                target_ids,
                target_generic_ids,
                concrete_ty,
            );
            substitute_generic_param_in_type(
                trait_arg,
                target_generic_ids,
                concrete_ty,
            );
        }
    }
}
```

**Required tests:**
- `substitute_typevars_in_function_updates_method_target_trait_args`

**Expected current red failure:**
The target trait args retain the source self TypeVar or generic after `substitute_typevars_in_function`.

**Focused command:**

```bash
cargo test -p rock-lib substitute_typevars_in_function_updates_method_target_trait_args -- --nocapture
```

### TS5-007: Substitute Match Arm Guards

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:468-485`
- Reference: `lib/src/hir/mod.rs:881-885`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
Default method substitution traverses match scrutinees and arm bodies, but not `HirMatchArm.guard`. Trait generics, `Self`, projections, and method-call targets in guards can remain stale.

**Required fix:**
Inside the match arm loop, substitute the guard expression before substituting the body.

**Minimal implementation shape:**

```rust
for arm in arms {
    if let Some(guard) = &mut arm.guard {
        substitute_trait_impl_types_in_expr(
            guard,
            impl_type,
            impl_trait_id,
            impl_trait_arg_types,
            impl_associated_types,
            generic_subst,
        );
    }
    substitute_trait_impl_types_in_pattern(
        &mut arm.pattern,
        impl_type,
        impl_trait_id,
        impl_trait_arg_types,
        impl_associated_types,
        generic_subst,
    );
    substitute_trait_impl_types_in_block(
        &mut arm.body,
        impl_type,
        impl_trait_id,
        impl_trait_arg_types,
        impl_associated_types,
        generic_subst,
    );
}
```

**Required tests:**
- `conformance_substitutes_default_method_match_guard_types`

**Expected current red failure:**
The guard expression still contains a trait-owned generic or projection after default method injection.

**Focused command:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_match_guard_types -- --nocapture
```

### TS5-008: Substitute Struct Pattern Type Arguments

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:468-485`
- Reference: `lib/src/hir/mod.rs:888-904`
- Compare: `lib/src/hir/mod.rs:1027-1069` for existing pattern traversal style
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
`HirPattern::Struct(String, Vec<Type>, Vec<(String, HirPattern)>)` carries type arguments, but default method substitution never traverses match patterns. Default bodies matching `Box<T>` or `Box<Self::Output>` can retain stale trait-owned type args in the pattern.

**Required fix:**
Add `substitute_trait_impl_types_in_pattern` and call it for every match arm pattern.

**Minimal implementation shape:**

```rust
fn substitute_trait_impl_types_in_pattern(
    pattern: &mut HirPattern,
    impl_type: &Type,
    impl_trait_id: Option<DefId>,
    impl_trait_arg_types: &[Type],
    impl_associated_types: &[HirAssociatedTypeDef],
    generic_subst: &HashMap<GenericParamId, Type>,
) {
    match pattern {
        HirPattern::Struct(_, type_args, fields) => {
            for ty in type_args {
                *ty = substitute_self(
                    ty,
                    impl_type,
                    impl_trait_id,
                    impl_trait_arg_types,
                    impl_associated_types,
                    generic_subst,
                );
            }
            for (_, nested) in fields {
                substitute_trait_impl_types_in_pattern(
                    nested,
                    impl_type,
                    impl_trait_id,
                    impl_trait_arg_types,
                    impl_associated_types,
                    generic_subst,
                );
            }
        }
        HirPattern::Tuple(items) | HirPattern::Or(items) => {
            for nested in items {
                substitute_trait_impl_types_in_pattern(
                    nested,
                    impl_type,
                    impl_trait_id,
                    impl_trait_arg_types,
                    impl_associated_types,
                    generic_subst,
                );
            }
        }
        HirPattern::Enum(_, _, items) => {
            for nested in items {
                substitute_trait_impl_types_in_pattern(
                    nested,
                    impl_type,
                    impl_trait_id,
                    impl_trait_arg_types,
                    impl_associated_types,
                    generic_subst,
                );
            }
        }
        HirPattern::Wildcard | HirPattern::Binding(_, _) | HirPattern::Literal(_) => {}
    }
}
```

**Required tests:**
- `conformance_substitutes_default_method_struct_pattern_type_args`
- Strengthen `test_trait_default_method_substitutes_generic_struct_pattern_types` or add a new integration test that uses a trait generic in the matched struct type.

**Expected current red failure:**
The pattern type args retain a trait-owned `GenericParamId` after default method injection.

**Focused commands:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_struct_pattern_type_args -- --nocapture
cargo test -p rock-lib --test integration test_trait_default_method_substitutes_trait_generic_struct_pattern_types -- --exact --nocapture
```

### TS5-009: Substitute Default Method `generic_bounds` Trait Args

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs:223-253`
- Reference: `lib/src/hir/mod.rs:573-588`
- Reference: `lib/src/types/mod.rs` `TraitBound`
- Test: `lib/src/lower/traits/conformance.rs`

**Current problem:**
`substitute_trait_impl_types_in_function` substitutes params, return type, and body, but not `func.generic_bounds`. A default method with generic bounds that reference trait generics, `Self`, or projections can retain stale type arguments in bound metadata.

**Required fix:**
Traverse `func.generic_bounds.values_mut()` and substitute every `TraitBound.type_args`.

**Minimal implementation shape:**

```rust
for bounds in func.generic_bounds.values_mut() {
    for bound in bounds {
        for type_arg in &mut bound.type_args {
            *type_arg = substitute_self(
                type_arg,
                impl_type,
                impl_trait_id,
                impl_trait_arg_types,
                impl_associated_types,
                generic_subst,
            );
        }
    }
}
```

**Required tests:**
- `conformance_substitutes_default_method_generic_bound_type_args`

**Expected current red failure:**
`func.generic_bounds` still contains a trait-owned generic/projection after injection.

**Focused command:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_generic_bound_type_args -- --nocapture
```

### TS5-010: Align Trait Default Body Generic Context With Trait Header Context

**Severity:** Important

**Files:**
- Modify: `lib/src/lower/traits/defaults.rs:54-58`
- Reference: `lib/src/lower/collect/traits.rs:170-173`
- Reference: `lib/src/lower/function.rs` generic context handling
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
Trait headers are collected with generic context `[trait generics..., Self]`, but default bodies are lowered with `[Self, trait generics...]`. Explicit type references inside default bodies can get different `GenericParamId` indexes than the signature/header for the same trait.

**Required fix:**
Use the same generic parameter order for default body lowering as header collection: trait generics first, then `Self`.

**Minimal implementation shape:**

```rust
self.current_generic_owner = Some(trait_def.id);
self.current_generic_params = self.current_trait_generics.clone();
self.current_generic_params.push("Self".to_string());
```

Audit any code that assumed `Self` is index `0` during default body lowering. The expected canonical convention for traits in this codebase is trait generics at indexes `0..n`, synthetic `Self` at index `n`.

**Required tests:**
- `conformance_default_body_generic_ids_match_trait_header_order`
- `test_trait_default_method_explicit_body_type_uses_trait_generic_argument`

**Expected current red failure:**
Default body HIR can use a different `GenericParamId` for `T` or `Self` than the method signature, causing substitution to miss or substitute the wrong type.

**Focused commands:**

```bash
cargo test -p rock-lib conformance_default_body_generic_ids_match_trait_header_order -- --nocapture
cargo test -p rock-lib --test integration test_trait_default_method_explicit_body_type_uses_trait_generic_argument -- --exact --nocapture
```

### TS5-011: Substitute Projection Types Inside Lambda Captures, Params, Body, And Return Types

**Severity:** Important

**Files:**
- Modify indirectly through TS5-001, TS5-002, TS5-003, and TS5-004 in `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

**Current problem:**
Lambda capture traversal exists, but current tests only cover a direct trait generic capture. Projection and nominal-container capture cases remain uncovered and can still fail until `substitute_self` is fixed completely.

**Required fix:**
After fixing `substitute_self`, verify lambda capture, lambda param, lambda body type, and lambda expression type all receive recursive substitution.

**Required tests:**
- `conformance_substitutes_default_method_lambda_capture_projection_with_trait_args`
- `conformance_substitutes_default_method_lambda_param_projection_types`
- `test_trait_default_method_lambda_captures_self_output_projection`

**Expected current red failure:**
The captured or lambda-local type remains `Type::Projection` or retains a trait-owned generic.

**Focused commands:**

```bash
cargo test -p rock-lib conformance_substitutes_default_method_lambda_capture_projection_with_trait_args -- --nocapture
cargo test -p rock-lib conformance_substitutes_default_method_lambda_param_projection_types -- --nocapture
cargo test -p rock-lib --test integration test_trait_default_method_lambda_captures_self_output_projection -- --exact --nocapture
```

### TS5-012: Validate Same-Name Default Traits With Generic Args And Projections

**Severity:** Important

**Files:**
- Test: `lib/tests/integration.rs`
- Test: `lib/src/lower/traits/conformance.rs`

**Current problem:**
Existing same-name default method coverage proves body selection for non-generic defaults, but not same-name traits with trait arguments or associated projections. This leaves room for regressions where default body or projection substitution uses display names instead of `DefId + trait_args`.

**Required fix:**
Add tests that combine same method names, distinct trait IDs, generic trait args, and associated type projections.

**Required tests:**
- `test_same_name_generic_trait_default_methods_select_bound_trait_body`
- `test_same_name_trait_default_projection_uses_selected_trait_identity`

**Expected current red failure:**
If any name-based path remains, the call can select the wrong default body or wrong associated type output.

**Focused commands:**

```bash
cargo test -p rock-lib --test integration test_same_name_generic_trait_default_methods_select_bound_trait_body -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_trait_default_projection_uses_selected_trait_identity -- --exact --nocapture
```

### TS5-013: Validate Default Method Injection For Dependency-Qualified Impl Owners

**Severity:** Important

**Files:**
- Existing fix area: `lib/src/lower/traits/conformance.rs:682-742`
- Test: `lib/tests/integration.rs`
- Test: `rock/src/tests/artifact.rs` if cross-package artifact setup is easier there

**Current problem:**
The current canonical owner regression is unit-only. It proves the HIR-level fix for local struct/enum `Box` versus dependency struct `dep_b::Box`, but it does not prove the parser/lower/import/artifact pipeline preserves canonical owner identity through real code.

**Required fix:**
Add an integration or artifact test where a dependency-owned type with a same short name inherits a trait default method and the injected `Self` resolves to the dependency type.

**Required tests:**
- `test_cross_crate_default_method_uses_dependency_qualified_impl_owner`
- If implemented in `rock`: `test_artifact_default_method_uses_dependency_qualified_impl_owner`

**Expected current red failure before the existing owner fix:**
The injected default binds `Self` to a local same-name type or wrong nominal kind. Current HEAD may already pass; the test still locks the behavior through real pipeline coverage.

**Focused commands:**

```bash
cargo test -p rock-lib --test integration test_cross_crate_default_method_uses_dependency_qualified_impl_owner -- --exact --nocapture
cargo test -p rock test_artifact_default_method_uses_dependency_qualified_impl_owner -- --nocapture
```

### TS5-014: Validate Generic Impl Default Methods Across Artifacts

**Severity:** Important

**Files:**
- Reference: `lib/src/products.rs:386-443`
- Reference: `lib/src/crate_artifact/load.rs:1752-1757`
- Reference: `lib/src/mono/external.rs:287-297`
- Test: `lib/src/crate_artifact/tests.rs`
- Test: `rock/src/tests/artifact.rs`

**Current problem:**
Generic impls are exported for artifact consumption when they have impl generics or trait generics. Inherited default methods inside those generic impls must survive product writing, artifact loading, and external monomorphization.

**Required fix:**
Add an artifact test where crate A exports a generic impl that inherits a default method, and crate B calls the inherited default for a concrete instantiation.

**Required tests:**
- `test_product_artifact_preserves_generic_impl_inherited_default_method`
- `test_artifact_consumer_calls_generic_impl_inherited_default_method`

**Expected current red failure if broken:**
The consumer cannot find the default method body, monomorphization misses the generic impl method, or linking fails for the inherited default symbol.

**Focused commands:**

```bash
cargo test -p rock-lib test_product_artifact_preserves_generic_impl_inherited_default_method -- --nocapture
cargo test -p rock test_artifact_consumer_calls_generic_impl_inherited_default_method -- --nocapture
```

### TS5-015: Validate Concrete Object-Backed Default Methods Across Artifacts

**Severity:** Important

**Files:**
- Reference: `lib/src/products.rs:486-487`
- Reference: `lib/src/codegen/mod.rs:358-376`
- Reference: `lib/src/mono/external.rs:192-235`
- Test: `rock/src/tests/artifact.rs`

**Current problem:**
Concrete inherited defaults become impl methods and need backend symbols/link records like explicit methods. Object-backed dependency dispatch relies on those symbols.

**Required fix:**
Add a build/artifact test where crate A exports a concrete impl inheriting a default method, crate B consumes the object artifact, and crate B calls the inherited default.

**Required tests:**
- `test_artifact_consumer_calls_concrete_inherited_default_method`

**Expected current red failure if broken:**
Codegen/linking fails due to missing backend symbol, or runtime dispatch calls the wrong method.

**Focused command:**

```bash
cargo test -p rock test_artifact_consumer_calls_concrete_inherited_default_method -- --nocapture
```

### TS5-016: Validate Artifact Remapping Of Default-Body Method Target Trait Args

**Severity:** Important

**Files:**
- Reference: `lib/src/crate_artifact/load.rs:1294-1320`
- Test: `lib/src/crate_artifact/tests.rs`

**Current problem:**
Artifact loading remaps `HirMethodCallTarget.trait_args`, but default method bodies can travel through trait/default-body product paths rather than ordinary function body paths. That path needs explicit coverage.

**Required fix:**
Add a product artifact test with a trait default body containing a selected method call target whose `trait_args` reference a nominal type from a dependency.

**Required tests:**
- `test_product_artifact_remaps_trait_default_method_target_trait_args`

**Expected current red failure if broken:**
Artifact load rejects the method target as unknown, leaves dependency product IDs unmapped, or dispatch later compares product IDs against consumer IDs.

**Focused command:**

```bash
cargo test -p rock-lib test_product_artifact_remaps_trait_default_method_target_trait_args -- --nocapture
```

### TS5-017: Validate HIR Indexes And Impl Method IDs For Inherited Defaults

**Severity:** Minor

**Files:**
- Reference: `lib/src/lower/traits/conformance.rs:917-965`
- Reference: `lib/src/infer/mod.rs:56-128`
- Reference: `lib/src/hir/mod.rs:363-394`
- Test: `lib/src/hir/mod.rs` or `lib/src/infer/mod.rs`

**Current problem:**
When an impl inherits a default without an existing placeholder, conformance clones the trait default method. Later phases assign missing method IDs and build HIR indexes. This is cross-phase-sensitive, especially when multiple impls inherit the same default.

**Required fix:**
Add a test that multiple impls inheriting the same default get distinct method IDs and all method targets/indexes point to valid methods.

**Required tests:**
- `hir_indexes_distinguish_multiple_inherited_default_method_ids`

**Expected current red failure if broken:**
`methods_by_id` has a collision, a method target points at the trait default ID instead of impl method ID, or one impl default overwrites another.

**Focused command:**

```bash
cargo test -p rock-lib hir_indexes_distinguish_multiple_inherited_default_method_ids -- --nocapture
```

### TS5-018: Decide Whether Artifact Format Version Must Change

**Severity:** Minor

**Files:**
- Inspect: `lib/src/products.rs`
- Inspect: `rock-shared/src/sysroot.rs`
- Test: existing product artifact format tests

**Current problem:**
Most Task 5 fixes change semantics, not serialized field layout. If any fix changes persisted HIR shape, method-target semantics, or artifact compatibility expectations, the artifact format version must remain bumped or be bumped again.

**Required fix:**
After implementing code fixes, explicitly decide whether `PRODUCT_ARTIFACT_FORMAT_VERSION` changes. If it changes in `lib/src/products.rs`, mirror it in `rock-shared/src/sysroot.rs`.

**Required tests:**
- Existing `product_artifact_format_version_matches_shared_contract`
- Existing stale artifact tests in `rock`

**Focused commands:**

```bash
cargo test -p rock-lib product_artifact_format_version_matches_shared_contract -- --nocapture
cargo test -p rock bundled_sysroot -- --nocapture
```

---

## Required New Tests Summary

Add these tests before or during implementation. Each should be written first and observed failing before production code changes.

### Unit Tests In `lib/src/lower/traits/conformance.rs`

- [ ] `conformance_does_not_substitute_projection_on_non_self_generic_base`
- [ ] `conformance_substitutes_default_method_nested_struct_trait_generic_types`
- [ ] `conformance_substitutes_default_method_nested_enum_trait_generic_types`
- [ ] `conformance_substitutes_default_method_projection_with_matching_trait_args`
- [ ] `conformance_substitutes_default_method_call_target_trait_args`
- [ ] `conformance_substitutes_default_method_match_guard_types`
- [ ] `conformance_substitutes_default_method_struct_pattern_type_args`
- [ ] `conformance_substitutes_default_method_generic_bound_type_args`
- [ ] `conformance_default_body_generic_ids_match_trait_header_order`
- [ ] `conformance_substitutes_default_method_lambda_capture_projection_with_trait_args`
- [ ] `conformance_substitutes_default_method_lambda_param_projection_types`

### Unit Tests In `lib/src/hir/mod.rs`

- [ ] `substitute_typevars_in_function_updates_method_target_trait_args`
- [ ] `hir_indexes_distinguish_multiple_inherited_default_method_ids`

### Integration Tests In `lib/tests/integration.rs`

- [ ] `test_trait_default_method_substitutes_generic_trait_self_output_projection`
- [ ] `test_trait_default_method_substitutes_trait_generic_in_nested_struct_return`
- [ ] `test_trait_default_method_substitutes_trait_generic_in_nested_enum_return`
- [ ] `test_trait_default_method_substitutes_trait_generic_struct_pattern_types`
- [ ] `test_trait_default_method_explicit_body_type_uses_trait_generic_argument`
- [ ] `test_trait_default_method_lambda_captures_self_output_projection`
- [ ] `test_same_name_generic_trait_default_methods_select_bound_trait_body`
- [ ] `test_same_name_trait_default_projection_uses_selected_trait_identity`
- [ ] `test_cross_crate_default_method_uses_dependency_qualified_impl_owner`

### Artifact Tests In `lib/src/crate_artifact/tests.rs`

- [ ] `test_product_artifact_preserves_generic_impl_inherited_default_method`
- [ ] `test_product_artifact_remaps_trait_default_method_target_trait_args`

### Artifact/CLI Tests In `rock/src/tests/artifact.rs`

- [ ] `test_artifact_default_method_uses_dependency_qualified_impl_owner`
- [ ] `test_artifact_consumer_calls_generic_impl_inherited_default_method`
- [ ] `test_artifact_consumer_calls_concrete_inherited_default_method`

---

## Implementation Order

### Task 1: Lock Projection Soundness

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`

- [ ] **Step 1: Write `conformance_does_not_substitute_projection_on_non_self_generic_base`.**
- [ ] **Step 2: Run it and verify it fails because the projection is over-substituted.**
- [ ] **Step 3: Remove the raw `matches!(**ty, Type::Generic(_))` projection shortcut.**
- [ ] **Step 4: Compare projection trait args directly to impl trait args.**
- [ ] **Step 5: Run existing projection tests and the new projection test.**

### Task 2: Complete Recursive Type Substitution

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing struct-arg and enum-arg unit tests.**
- [ ] **Step 2: Add failing integration tests for nested struct and enum return types.**
- [ ] **Step 3: Add `Type::Struct` and `Type::Enum` arms to `substitute_self`.**
- [ ] **Step 4: Run the focused struct/enum tests.**

### Task 3: Complete HIR Expression Metadata Traversal

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/hir/mod.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/hir/mod.rs`

- [ ] **Step 1: Add failing method-target trait-arg tests for conformance substitution and HIR typevar substitution.**
- [ ] **Step 2: Substitute `HirMethodCallTarget.trait_args` in `conformance.rs`.**
- [ ] **Step 3: Substitute `HirMethodCallTarget.trait_args` in `hir::substitute_typevars_in_function`.**
- [ ] **Step 4: Run the focused method-target tests.**

### Task 4: Complete Match And Pattern Traversal

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing match guard test.**
- [ ] **Step 2: Add failing struct pattern type-args test.**
- [ ] **Step 3: Add `substitute_trait_impl_types_in_pattern`.**
- [ ] **Step 4: Call the pattern helper and guard traversal from the match arm loop.**
- [ ] **Step 5: Run the focused match and pattern tests.**

### Task 5: Substitute Default Method Generic Bounds

**Files:**
- Modify: `lib/src/lower/traits/conformance.rs`
- Test: `lib/src/lower/traits/conformance.rs`

- [ ] **Step 1: Add failing generic-bound type-arg substitution test.**
- [ ] **Step 2: Traverse `func.generic_bounds` in `substitute_trait_impl_types_in_function`.**
- [ ] **Step 3: Run the focused generic-bound test.**

### Task 6: Align Default Body Generic Context

**Files:**
- Modify: `lib/src/lower/traits/defaults.rs`
- Test: `lib/src/lower/traits/conformance.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add failing generic ID ordering test.**
- [ ] **Step 2: Change default body generic context to trait generics followed by `Self`.**
- [ ] **Step 3: Audit and adjust any code that assumed `Self` is index `0` in trait default bodies.**
- [ ] **Step 4: Run the focused default-body generic ID tests.**

### Task 7: Add Cross-Phase Artifact And Dispatch Coverage

**Files:**
- Test: `lib/src/crate_artifact/tests.rs`
- Test: `rock/src/tests/artifact.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add same-name generic default/projection integration tests.**
- [ ] **Step 2: Add dependency-qualified owner integration/artifact tests.**
- [ ] **Step 3: Add generic impl inherited default artifact tests.**
- [ ] **Step 4: Add concrete object-backed inherited default artifact test.**
- [ ] **Step 5: Add artifact remap test for default-body method target trait args.**
- [ ] **Step 6: Run focused artifact and integration tests.**

### Task 8: Final Verification

**Files:**
- No source edits expected in this task.

- [ ] **Step 1: Run formatting.**

```bash
cargo fmt --all
cargo fmt --all --check
```

- [ ] **Step 2: Run whitespace diff check.**

```bash
git diff --check
```

- [ ] **Step 3: Run focused Task 5 tests.**

```bash
cargo test -p rock-lib trait_default_method -- --nocapture
cargo test -p rock-lib conformance_substitutes_default -- --nocapture
```

- [ ] **Step 4: Run package suites.**

```bash
cargo test -p rock-lib
cargo test -p rock
```

- [ ] **Step 5: Request code review focused on Task 5 substitution soundness.**

The review prompt must include TS5-001 through TS5-018 and ask specifically for missing HIR carriers, projection over-substitution, and cross-crate default method artifact behavior.

---

## Completion Criteria

Task 5 can be called complete only after all of these are true:

- [ ] No projection substitution occurs for non-`Self` generic bases.
- [ ] `Type::Struct` args are recursively substituted.
- [ ] `Type::Enum` args are recursively substituted.
- [ ] Projection trait args are compared directly to impl trait args.
- [ ] `HirMethodCallTarget.trait_args` are substituted during conformance default injection.
- [ ] `HirMethodCallTarget.trait_args` are substituted during HIR typevar substitution.
- [ ] Match arm guards are substituted.
- [ ] Match arm patterns, including struct pattern type args, are substituted.
- [ ] Default method generic bounds are substituted.
- [ ] Default body generic context uses the same parameter order as trait header collection.
- [ ] Lambda captures, lambda params, lambda body types, and lambda return types are covered for projections and nested nominal args.
- [ ] Same-name trait default method tests cover trait IDs, trait args, and projections.
- [ ] Cross-crate dependency-qualified owner behavior is covered through real pipeline or artifact tests.
- [ ] Generic inherited defaults survive product artifacts and external monomorphization.
- [ ] Concrete inherited defaults expose usable backend symbols when consumed as artifacts.
- [ ] Artifact remapping covers default-body method target trait args.
- [ ] Artifact format version decision is documented and tested.
- [ ] `cargo fmt --all --check` passes.
- [ ] `git diff --check` passes.
- [ ] `cargo test -p rock-lib` passes.
- [ ] `cargo test -p rock` passes.
- [ ] A focused post-fix review reports no Critical or Important Task 5 findings.
