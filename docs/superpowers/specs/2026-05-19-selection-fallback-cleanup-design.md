# Mono And Codegen Selection Fallback Cleanup Design

## Goal

Roadmap Task 13 proves the shared selection service is authoritative by deleting or narrowing mono and codegen semantic rediscovery paths when a `HirMethodCallTarget` is present.

The task is intentionally narrow. Selected targets become hard identity contracts for downstream phases, but targetless compatibility paths remain for legacy calls and for later Roadmap Tasks 14-15, which will make the instance registry and call edges authoritative.

## Current State

Task 12 introduced `lib/src/selection/` and moved lowering-time method, trait-bound, operator, and index selection into the shared service. Lowering now records selected identity in `HirMethodCallTarget`, and mono/codegen prefer shared selected-target helpers before broad fallback lookup.

The remaining duplication lives mainly in:

- `lib/src/mono/methods.rs`, where method monomorphization still has receiver/name scans for targetless calls and constrained searches for selected trait targets.
- `lib/src/mono/process.rs`, where every method call still enters trait-method monomorphization and some calls also enter standalone method monomorphization.
- `lib/src/codegen/expr/mod.rs`, where method codegen resolves exact selected backend symbols first, then still has direct-method, trait-impl, and array fallback paths.
- `lib/src/codegen/mod.rs`, where codegen keeps helper logic for trait impl lookup, receiver argument matching, and builtin-index detection.

These paths are not all wrong. Some are compatibility scaffolding for calls that do not yet have selected targets, and some are needed to turn a selected trait-bound target into a concrete impl after monomorphization. Task 13 should remove only the parts that can override or hide an explicit selected identity.

## Scope

The cleanup covers calls with an explicit `HirMethodCallTarget`.

When a target exists, mono and codegen must treat it as authoritative:

- If `impl_id` and `method_id` are present, downstream phases may only use that impl/method pair.
- If `trait_id`, `method_id`, and `trait_args` are present without `impl_id`, downstream phases may only concretize against impls for that trait identity and selected method identity.
- If the selected identity cannot be resolved, mono/codegen must report an identity-specific error instead of trying a source-name, receiver-name, or array fallback.
- If a builtin index target is explicitly selected, codegen may use the builtin index path only when the target is marked as index-origin, has no user impl ID, and no matching user impl exists for the selected trait identity.

Targetless calls remain compatibility behavior:

- Direct method lookup by backend/display name can remain for unselected calls.
- Array/slice fallback lookup can remain for unselected calls.
- Static impl method lookup by source/backend name can remain until Task 14-15 replace backend-symbol call targets with instance/call edges.

## Mono Design

`lib/src/mono/process.rs` should make selected-target branching explicit before invoking specialization helpers.

For `HirExprKind::MethodCall`:

1. Substitute generic trait args on the selected target as today.
2. Process receiver and argument expressions as today.
3. If a selected target exists, call only the helper appropriate for that selected identity.
4. If no selected target exists, keep the existing compatibility sequence.

`lib/src/mono/methods.rs` should expose selected-target helpers with identity-shaped names, so the control flow communicates the contract:

- Selected inherent or selected trait-impl method calls use only `impl_id` plus `method_id`.
- Selected trait-bound or unresolved-generic calls use only `trait_id`, `method_id`, selected trait args, and receiver matching.
- No selected helper should fall back to method-name-only scans after it has observed a selected target.

Existing generic substitution and specialization machinery can stay. The task does not redesign `InstanceRegistry`, `InstanceKey`, backend symbol generation, or `HirExprKind::Call` rewriting.

## Codegen Design

`lib/src/codegen/expr/mod.rs` should separate targeted and targetless method codegen paths.

For targeted calls:

- Resolve exact instance/object backend symbols from `impl_method_backend_symbols` when available.
- If the selected impl method is present in registered HIR impls and is concrete enough for codegen, declare it and use that exact `(impl_id, method_id)` symbol.
- If the target is trait-only, search only impls that match the selected `trait_id`, selected `method_id`, selected trait args, and receiver type.
- If none of those paths resolves, return the existing selected-target diagnostic wording rather than trying direct method names or array fallback names.

For targetless calls:

- Keep direct method lookup and array fallback behavior as compatibility.
- Keep LLVM declaration maps, backend symbol maps, and `method_functions` receiver ABI metadata.

The design keeps codegen's backend responsibilities intact. It removes semantic fallback only where a selected target has already made a frontend dispatch decision.

## Tests

Use TDD for each behavior change.

Focused unit tests should cover:

- A selected inherent/generic impl method with a stale method name fails or leaves the call unresolved instead of specializing a same-name fallback.
- A selected trait impl with the wrong method ID does not fall back to another method under the selected impl.
- A selected trait-bound target does not dispatch through a same-name method from a different trait identity.
- A selected codegen target that cannot be resolved returns the identity-specific selected-target error and does not use a direct `Type_method` fallback.
- A selected index target with a user impl keeps user-impl precedence over builtin array/slice fallback.
- Targetless compatibility tests continue to pass for static impl methods and legacy direct method lookup.

Integration regressions should include:

- Same-name trait methods do not dispatch by method name only.
- Same-name trait default methods select the selected bound trait body.
- Generic trait-bound dispatch preserves selected trait arguments.
- Index dispatch with associated output uses the selected `Index` impl.
- Binary and unary operator dispatch still requires builtin trait identity.

Verification commands should include:

```bash
cargo fmt --all --check
cargo test -p rock-lib selected_method_target_does_not_fallback_to_direct_alias
cargo test -p rock-lib selected_method_target_does_not_use_impl_name_when_method_id_differs
cargo test -p rock-lib selected_index_target_ref_does_not_use_builtin_index_pointer
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact
cargo test -p rock-lib --test integration test_same_name_trait_default_methods_select_bound_trait_body -- --exact
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_uses_trait_arguments -- --exact
cargo test -p rock-lib --test integration test_index_dispatches_through_trait_with_associated_output -- --exact
cargo test -p rock-lib --test integration test_binary_operator_requires_builtin_trait_identity -- --exact
cargo test -p rock-lib --test integration test_unary_not_generic_dispatches_through_stdlib_trait_associated_output -- --exact
cargo test -p rock-lib
git diff --check
```

## Non-Goals

- Do not delete targetless compatibility fallback paths that are still needed before Tasks 14-15.
- Do not redesign `InstanceRegistry` or make it the sole callable universe; Task 14 owns that migration.
- Do not replace backend-symbol call targets with `InstanceId` edges; Task 15 owns that migration.
- Do not move codegen to consume MIR; Task 21 owns that migration.
- Do not persist `TypeId` in products or change the product artifact schema.
- Do not remove builtin index support for arrays, slices, borrowed slices, or strings where current behavior supports it.

## Risks

- Some selected trait-bound calls intentionally need constrained impl concretization after monomorphization. The cleanup must forbid broad fallback without preventing this selected-trait concretization.
- Some legacy paths may still emit targetless calls. Removing all fallback lookup now would cross roadmap boundaries and risk regressions unrelated to Task 13.
- Codegen still owns LLVM declaration and receiver ABI mechanics, so the cleanup must avoid moving backend concerns into the selection service.
- Error wording is covered by selected-target tests. Keep identity diagnostics stable unless tests are deliberately updated with clearer wording.
