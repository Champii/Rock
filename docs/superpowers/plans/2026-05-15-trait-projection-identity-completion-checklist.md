# Trait Projection Identity Completion Checklist

This is the canonical remaining-work checklist for Phase 6 Task 5. The implementation plan in `2026-05-15-trait-projection-identity-completion.md` remains the high-level guide; this file records the stricter audit backlog required before Task 5 can be called complete.

## Status

- Current branch: `ohmyopenagent`
- Current baseline commit: `865702d hir: harden artifact identity validation`
- Do not touch untracked `.sisyphus/`.
- Product artifact format version is `17` after adding dependency source fingerprints, ambiguous export-name tombstones, and serialized method-call index-operator classification.

## Batch 1: Generic Bounds And Trait Args

- [x] Generic trait-method monomorphization matches trait args, not only trait ID (`lib/src/mono/methods.rs`).
- [x] Codegen selected-trait dispatch does not fall back to empty trait args (`lib/src/codegen/expr/mod.rs`).
- [x] Product artifact write/load remaps `HirFunction.generic_bounds` (`lib/src/products.rs`, `lib/src/crate_artifact/load.rs`).
- [x] Product artifact write/load remaps `HirFunctionSig.generic_bounds` (`lib/src/products.rs`, `lib/src/crate_artifact/load.rs`).
- [x] `generic_bounds` is keyed by `GenericParamId`, not `String` (`lib/src/hir/mod.rs`, `lib/src/lower/mod.rs`).
- [x] Unknown where-clause traits do not create sentinel `DefId` bounds (`lib/src/collect/headers.rs`, `lib/src/lower/function.rs`, `lib/src/lower/bodies.rs`, `lib/src/lower/collect/traits.rs`).
- [x] Regression: generic dispatch with same trait ID but different trait args.
- [x] Regression: selected-trait dispatch fails instead of using empty trait args.
- [x] Regression: artifact roundtrip/load for `HirFunction.generic_bounds` remap.
- [x] Regression: artifact roundtrip/load for `HirFunctionSig.generic_bounds` remap.
- [x] Regression: same-name generic-bound method lookup in `secondary.rs`.
- [x] Regression: unknown-trait impl is excluded from later dispatch/default processing.

## Batch 2: Method Target Identity

- [x] Inherent impl method calls carry `HirMethodCallTarget { impl_id: Some(...), trait_id: None, method_id }` (`lib/src/lower/control_flow/secondary.rs`, `lib/src/lower/expression.rs`).
- [x] Name-only `self.methods` dispatch is not semantic post-resolution dispatch (`lib/src/lower/control_flow/secondary.rs`).
- [x] Trait impl methods are not registered into name-only method maps as semantic lookup (`lib/src/lower/collect/traits.rs`).
- [x] Cross-crate impl method registration does not use name-only method maps for semantic lookup (`lib/src/lower/crates/registration.rs`, `lib/src/lower/crates/bodies.rs`).
- [x] Codegen targetless method fallback is removed, restricted, or proven safe (`lib/src/codegen/expr/mod.rs`).
- [x] Codegen consults exact `(impl_id, method_id)` backend symbols before name-derived lookup (`lib/src/codegen/expr/mod.rs`).
- [x] Artifact method-target validation rejects `trait_id: Some(...)` paired with an inherent impl (`lib/src/crate_artifact/load.rs`).
- [x] Replace `HirMethodCallTarget.method_id = trait_id` sentinel with real signature identity or a separate enum (`lib/src/lower/control_flow/secondary.rs`, `lib/src/lower/expression.rs`).
- [x] `HirFunctionSig` has stable identity for signature-only trait calls (`lib/src/hir/mod.rs`).
- [x] Mono targetless trait dispatch is impossible or proven safe (`lib/src/mono/methods.rs`).
- [x] Codegen `show` fallback returning `default_value` does not mask wrong or missing dispatch (`lib/src/codegen/expr/mod.rs`).
- [x] Regression: inherent impl method calls carry exact `HirMethodCallTarget`.
- [x] Regression: exact `(impl_id, method_id)` backend symbols win over name-derived lookup.
- [x] Regression: method-as-value target identity.
- [x] Regression: curried/partial method-call target identity.

## Batch 3: Artifact And Export Identity

- [x] Dependency projection/member validation during artifact load performs strong existence checks (`lib/src/crate_artifact/load.rs`).
- [x] Forged artifact references to dependency `trait_id`, `impl_id`, `method_id`, and associated IDs are rejected (`lib/src/crate_artifact/load.rs`).
- [x] Artifact dependency compatibility uses stronger identity/fingerprint (`lib/src/products.rs`, `lib/src/crate_system/context.rs`).
- [x] Product export-name recording does not silently overwrite different IDs (`lib/src/products.rs`).
- [x] Trait default method export names cannot collide by display name (`lib/src/products.rs`).
- [x] `PreferredTraitDefaultMethods` fallback by `(trait_def.name, method_name)` is removed or collision-safe (`lib/src/products.rs`).
- [x] Regression: full artifact-load rejection for invalid projection IDs.
- [x] Regression: full artifact-load rejection for forged dependency IDs.
- [x] Regression: artifact compatibility catches changed dependency contents with same identity fields.
- [x] Regression: integration artifact consumption for trait/projection identity.
- [x] Regression: direct impl method export collision with same display name and distinct IDs.

## Batch 4: Projection, Defaults, And Builtins

- [x] Projection resolution validates `assoc_type.owner == trait_id` (`lib/src/lower/types_helpers/helpers.rs`, `lib/src/codegen/types.rs`).
- [x] Default-method projection substitution checks `assoc_type.owner` (`lib/src/lower/traits/conformance.rs`).
- [x] Default-method substitution applies to lambda capture types (`lib/src/lower/traits/conformance.rs`).
- [x] Builtin operator trait lookup is consistent across binary, unary, and index (`lib/src/lower/expression.rs`, `lib/src/lower/control_flow/secondary.rs`).
- [x] Decide and encode whether local `Neg`/`Not` are valid operator traits until stdlib canonical `Neg`/`Not` exist.
- [x] Auto-`Sized` uses canonical builtin `Sized` identity (`lib/src/lower/traits/conformance.rs`).
- [x] `trait_ids_by_name["Index"]` in codegen is replaced with HIR-carried ID where possible (`lib/src/codegen/mod.rs`, `lib/src/codegen/expr/mod.rs`, `lib/src/codegen/types.rs`).
- [x] Regression: same-name associated type collision with distinct trait IDs.
- [x] Regression: same-name trait default-method collision selects correct default body.
- [x] Regression: default method returning `Self::Output` or another projection.
- [x] Regression: default method lambda captures with trait generics/projections.
- [x] Regression: shadowing or unrelated `Index` does not satisfy `[]`.
- [x] Regression: unary operator shadowing tests, or documented local `Neg`/`Not` fallback.
- [x] Regression: local user-defined `Sized` is not auto-implemented unless canonical builtin/std/prelude `Sized`.

## Batch 5: Mono And Backend Identity

- [x] Object-backed impl signature keys include concrete receiver and trait arg identity (`lib/src/mono/mod.rs`).
- [x] Object-backed impl method symbols use artifact-recorded backend symbols (`lib/src/mono/external.rs`).
- [x] Mono receiver-mode lookup does not choose ABI by `type_name + method_name` (`lib/src/mono/process.rs`).

## Closure Criteria

- [x] Every semantic method call has a validated target or explicit direct/builtin classification.
- [x] Trait identity after resolution is always `DefId + trait_args`.
- [x] Projection identity always validates `trait_id + assoc_type.owner + assoc_type_id`.
- [x] Every persisted ID-bearing field has product remap and artifact validation coverage.
- [x] Name-keyed maps are non-semantic only.
- [x] `cargo fmt --all --check` passes.
- [x] `git diff --check` passes.
- [x] `cargo test -p rock-lib` passes.
- [x] Semantic searches leave only documented non-semantic name-keyed hits.
- [x] Post-fix review finds no blocking same-name trait/projection identity issues.

Evidence from final verification on 2026-05-16:

- `cargo fmt --all --check` exited 0.
- `git diff --check` exited 0.
- `cargo test -p rock-lib` passed: 838 unit tests, 255 integration tests, 1 `test_parse_struct_with_fields` test, and 1 doc-test passed; expected ignored tests remained ignored.
- Semantic searches left only documented display/source/backend metadata, crate graph/DCE string maps, ID-backed mono receiver lookup, and canonical `builtin_trait_by_name("Index")` hits.
- Final post-fix review (`ses_1d0a43868ffeURFtUAyyqrjcih`) reported no Critical, Important, or Minor findings.
