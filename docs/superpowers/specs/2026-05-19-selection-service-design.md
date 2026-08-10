# Shared Trait And Method Selection Service Design

## Goal

Roadmap Task 12 builds one semantic selection boundary for trait and method dispatch. The selector centralizes impl lookup, receiver adjustment, generic substitution, associated projection normalization, trait default resolution, selected-target recording, and selection diagnostics.

The first implementation should cover the full Task 12 role while preserving compatibility fallbacks that Task 13 will delete or narrow. Lowering becomes the authoritative selection point. Mono and codegen consume explicit selection facts first, but keep existing name and trait lookup fallbacks as guardrails until the next roadmap task proves they are no longer needed.

## Current State

Selection logic is spread across several phases:

- `lib/src/lower/control_flow/secondary.rs` selects concrete methods, trait-bound methods, method values, receiver adjustments, and index calls.
- `lib/src/lower/expression.rs` selects trait-backed operators and custom operator fallbacks.
- `lib/src/lower/types_helpers/helpers.rs` owns reusable but `Lowerer`-coupled helpers for type names, impl matching, receiver generic substitution, and projection normalization.
- `lib/src/mono/methods.rs` reinterprets `HirMethodCallTarget` and repeats method/impl matching to monomorphize calls.
- `lib/src/codegen/expr/mod.rs` re-resolves selected and unselected method calls, then falls back to direct names, trait impl scans, and array-specific paths.

The codebase already records `HirMethodCallTarget` for many calls. Task 12 makes that record more authoritative by moving selection into a shared service and giving downstream phases enough explicit facts to prefer selected targets over name-based rediscovery.

## Architecture

Create a new focused module under `lib/src/selection/`.

The module owns selection data structures and pure matching helpers. It should not own AST parsing, HIR construction, monomorphization, or LLVM codegen. It receives read-only views of the data needed to answer selection queries and returns typed selection outcomes.

Core types:

- `SelectionRequest`: describes the dispatch being selected. It includes the receiver type, method/operator/index name, argument types, optional required trait ID, whether builtin index dispatch is allowed, whether method value lowering is requested, and the source span or diagnostic context string.
- `SelectionContext`: read-only access to traits, impls, resolver type names, current trait context, type-var bounds, generic bounds, projection normalization, and builtin type facts.
- `SelectionOutcome`: the selected result. It includes adjusted receiver information, selected origin, substituted params, substituted return type, receiver mode, normalized projection facts, optional builtin index output, and diagnostics.
- `SelectedOrigin`: distinguishes inherent impl methods, trait impl methods, trait-bound methods/signatures, current-trait `Self` methods/signatures, builtin index dispatch, and unresolved generic/type-var targets.
- `ReceiverAdjustment`: records whether the original receiver, borrow, mutable borrow, deref, or deref-chain candidate was selected.
- `SelectionDiagnostic`: structured lowering-time diagnostics for no matching impl, ambiguous candidates, receiver mismatch, trait/member mismatch, and selected-target inconsistency.

`HirMethodCallTarget` remains the compatibility representation in HIR and products. Task 12 may extend it only if needed to record selection facts that are already serializable as structural HIR data. It must not introduce context-owned `TypeId` persistence or remove structural `Type` compatibility.

## Data Flow

Lowering is the first authoritative consumer.

1. `Lowerer` lowers receiver and argument expressions as it does today.
2. `Lowerer` creates a `SelectionContext` view over its resolver, HIR maps, inference engine bounds, current trait/impl context, and projection service.
3. `Lowerer` calls the selector for method calls, method values, trait-backed operators, and index dispatch.
4. The selector returns a `SelectionOutcome` or structured diagnostics.
5. `Lowerer` converts a successful outcome into `HirExprKind::MethodCall` with substituted parameter and return types, receiver mode, and `HirMethodCallTarget`.
6. `Lowerer` converts diagnostics into its existing error channel with spans, preserving current user-facing wording where practical.

Mono and codegen are staged consumers.

- `lib/src/mono/methods.rs` should prefer selected impl IDs, trait IDs, trait args, method IDs, and substituted call facts when monomorphizing generic or trait-backed method calls.
- `lib/src/codegen/expr/mod.rs` should prefer selected impl IDs and method IDs when resolving backend symbols, and report selected-target failures using clearer diagnostics.
- Existing fallback scans remain temporarily so behavior stays compatible while Task 12 is introduced. Task 13 is responsible for deleting or narrowing these duplicate fallback paths after selection records are proven authoritative.

## Selection Semantics

The service should preserve existing dispatch behavior while making selection explicit.

Concrete method selection:

- Try receiver adjustment candidates in the same order as today.
- Match impl owner/type names through resolver-backed canonical names and existing slice/array/reference special cases.
- Seed receiver generic substitutions from receiver type arguments.
- Substitute method params and return type before lowering unifies argument and result types.
- Record exact `impl_id` and `method_id` for inherent and trait impl methods when known.

Trait-bound and generic selection:

- For type variables, inspect inference-engine trait bounds and select methods or signatures by trait ID and member ID.
- For generic params, inspect current impl/function bounds and select methods or signatures by trait ID and member ID.
- Preserve trait args from the selected bound, including associated output trait args.
- Prefer selected trait/member identity over name-only method maps.

Current-trait default body selection:

- Continue supporting `Self` method/signature lookup while lowering trait default bodies.
- Prefer real canonical trait/member IDs over temporary placeholder IDs.
- Preserve selected trait IDs so same-name traits and same-name default methods dispatch by identity.

Operator selection:

- Builtin trait-backed operators keep requiring builtin trait identity, not local shadow traits.
- Concrete receivers select the matching impl method and record target identity.
- Type-var and generic receivers record trait/member targets for later monomorphization.
- Custom operators that are not builtin trait-backed keep function-call fallback behavior.

Index selection:

- User `Index` impls continue to take precedence over builtin array/slice index behavior when both match.
- Builtin slice/array/str index output remains provided by `TypeFacts`.
- User `Index` impl selection records trait args and associated `Output` projection facts.
- The lowered expression keeps the current method-call-plus-deref shape.

## Diagnostics

Selection diagnostics should be structured in the service and rendered by lowering.

Required diagnostics:

- No matching method/operator/index implementation for a concrete receiver.
- Trait member exists by name but not by selected trait identity.
- Receiver adjustment failed to find a candidate matching the required trait/member.
- Selected target references an impl or method that cannot be found by identity.
- Ambiguous candidates when multiple identity-distinct selections remain valid.

Diagnostics should include the receiver type, method/operator/index name, required trait identity when available, and source span from the request. Existing message text should be preserved where tests or users likely depend on it, but new selected-target diagnostics should fail during lowering where possible rather than surfacing first as codegen `Unknown selected method` errors.

## Testing

Use TDD for the implementation plan.

Unit tests should cover:

- Concrete method selection returns exact `impl_id` and `method_id`.
- Same-name methods do not dispatch through name-only maps.
- Type-var and generic-bound selection prefer trait/member identity.
- Current-trait `Self` default body calls select canonical signature/default method IDs.
- Receiver generic substitution fills method params and return type.
- Trait args are preserved for generic trait-bound dispatch.
- User `Index` impls beat builtin array/slice index output.
- Builtin index output still works for arrays, slices, and supported references.
- Operator selection uses builtin trait identity and preserves associated output projections.
- Selected-target diagnostics are produced before codegen for identity mismatches.

Integration and regression tests should include existing coverage for same-name traits/methods, generic trait bounds, trait defaults, artifact-backed methods, stdlib operator traits, index associated output, and selected-target diagnostics. Prefer extending focused existing tests before adding broad duplicates.

Verification commands for the implementation plan should include:

```bash
cargo fmt --all --check
cargo test -p rock-lib selection_
cargo test -p rock-lib type_var_method_call_uses_trait_bound_target_before_name_candidates
cargo test -p rock-lib inherent_impl_method_call_carries_exact_target
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_uses_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_unary_not_generic_dispatches_through_stdlib_trait_associated_output -- --exact
cargo test -p rock-lib
git diff --check
```

## Non-Goals

- Do not delete mono/codegen fallback lookup paths in Task 12 unless a narrow fallback is made unreachable by explicit selection and has direct tests. Broad deletion belongs to Task 13.
- Do not migrate product artifacts away from structural `Type` serialization.
- Do not persist context-owned `TypeId` values in product artifacts.
- Do not move parser, resolver, mono registry, or LLVM declaration ownership into the selection module.
- Do not redesign `InstanceRegistry`; Task 14 owns the sole callable-universe migration.

## Open Risks

- The selector will initially need a context view over `Lowerer` internals. Keep this as a narrow adapter to avoid moving `Lowerer` state wholesale into the service.
- Existing tests rely on some exact error text. Preserve message wording at rendering boundaries where practical.
- `HirMethodCallTarget` is serialized in products. Any extension must be deliberate and covered by artifact compatibility tests.
- Receiver adjustment and projection normalization are subtle for borrowed slices, arrays, `Str`, and deref chains. Implementation tasks should migrate these paths in small red-green steps.
