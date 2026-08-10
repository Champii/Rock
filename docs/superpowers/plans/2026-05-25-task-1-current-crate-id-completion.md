# Task 1 Current-Crate ID Completion Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `test-driven-development` for every behavior change, `systematic-debugging` for failures/regressions, `requesting-code-review` before commit, and `verification-before-completion` before any completion claim. Use `executing-plans` or `subagent-driven-development` to execute this document task-by-task.

**Goal:** Complete roadmap Task 1, `Make Current-Crate ID Allocation Single-Source`, in full. After this plan is complete, the master audit checklist and ordered roadmap should no longer list collect/header provisional fallback paths, lowering placeholder recovery paths, or current-crate generated/sentinel identity repair as Task 1 remaining work.

**Source Requirements:**
- `docs/superpowers/specs/2026-04-24-compiler-architecture-audit-design.md` requires stable semantic identity through `DefId { crate_id, local }`, with names and backend symbols kept as display/output data only.
- `docs/superpowers/plans/master-audit-checklist.md` still lists Task 1 gaps: remaining generated/sentinel IDs that affect user-visible compilation paths, including collect/header provisional fallback paths and lowering placeholder recovery paths.
- `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` marks Task 1 complete only for scoped slices and still records generated/non-indexed collect/header fallback paths plus lowering placeholder recovery paths as remaining work.

**Architecture:** Collection and item/member indexing are the only sources of current-crate semantic declaration identity. Header builders consume explicit IDs; they never fabricate current-crate or sentinel IDs. Generated compiler definitions are explicit generated definitions with provenance and are recorded in current-definition sets. Missing identity is a collection/lowering error or invariant failure, never a hidden repair pass. Error recovery may use `Type::Error` and diagnostic placeholders, but not bogus semantic `DefId`s or valid-looking `Unit` expressions that can reach later phases as real code.

**Non-Goals:**
- Do not complete all master audit tracks in this plan. Tasks 2-23 remain separate unless they are directly required to close Task 1.
- Do not migrate all HIR string maps, all `TypeId` phase boundaries, or formatter trivia work.
- Do not remove product artifact collision/fallback remapping that is explicitly product-schema repair, unless it accepts compiler sentinel IDs from current-crate HIR as normal input.
- Do not add stdlib/sysroot discovery or compiler-owned stdlib injection.

---

## Completion Definition

Task 1 is complete only when all of these are true:

- No production collection or header-building path calls `fresh_provisional_def_id` or constructs `CrateId(u32::MAX)` to stand in for a current-crate declaration.
- `CollectContext` no longer owns a provisional current-crate ID counter for production collection.
- Top-level items, externs, impls, trait methods, trait signatures, impl methods, fields, variants, associated types, function signatures, artifact declarations, and generated definitions get IDs from an explicit collection/index/generated-ID authority before their HIR headers are built.
- Missing collection identity becomes a structured collection error or a hard invariant failure before body lowering; it is not repaired after headers are built.
- Lowering does not recover from missing semantic identity by fabricating placeholder `DefId`s, accepting sentinel owners, or emitting valid-looking `Unit` HIR nodes for failed semantic resolution.
- Any remaining `CrateId(u32::MAX)`, `LocalDefId(u32::MAX)`, `DefId(0, 0)`, `placeholder`, `fallback`, or `sentinel` hits are audited and documented as tests, product-schema collision remapping, diagnostic display, glob-export sentinel strings, or non-Task-1 cleanup.
- `master-audit-checklist.md` marks Task 1 as done for current-crate ID allocation and removes Task 1 generated/sentinel ID gaps.
- `2026-05-17-compiler-architecture-ordered-roadmap.md` marks Task 1 complete without the current generated/non-indexed caveat.

---

## Current Evidence To Retire Or Classify

### Must Retire From Production Task 1 Paths

- `lib/src/collect/context.rs`: `CollectContext::fresh_provisional_def_id` and `provisional_def_id` exist as a sentinel allocator using `CrateId(u32::MAX)`.
- `lib/src/collect/collector.rs`: `item_id_for_name` and `next_impl_id` still fall back to `fresh_provisional_def_id` after pushing missing canonical identity errors.
- `lib/src/collect/headers.rs`: `build_function_sig` fabricates a provisional ID; trait method and impl method header loops call `fresh_provisional_def_id`.
- `lib/src/lower/expression.rs`: missing operator implementation returns `HirExprKind::Unit` placeholders after reporting errors.
- `lib/src/lower/expression.rs`: unknown operator precedence uses arbitrary precedence `5` to continue tree construction.
- `lib/src/lower/traits/conformance.rs`: trait default handling still refers to empty placeholder methods left by collection and remaps generic owners from placeholder method IDs.
- `lib/src/lower/mod.rs`: `resolve_item_def_id_or` accepts a fallback `DefId`; verify whether it is still production reachable and remove or constrain it.
- `lib/src/lower/mod.rs`: `nominal_def_id_for_name` and generated impl paths still need audit for fallback/sentinel identity behavior.

### Must Keep But Classify As Non-Task-1 Or Test-Only

- Glob export sentinels such as `module::*` are string markers for export expansion, not semantic `DefId` sentinels.
- `DefId(CrateId(0), LocalDefId(0))` is a valid current-crate ID and may appear in tests asserting local ID zero preservation.
- Product artifact fallback remapping for `ProductCrateId(u32::MAX)` and duplicate product IDs is product-schema repair. It must reject or remap already-invalid compiler HIR IDs before artifact exposure, but it is not itself current-crate ID allocation.
- Tests may construct invalid `DefId`s to prove validation rejects them, but production code must not create those IDs as normal control flow.

---

## Ordered Implementation Tasks

### Task 0: Rebaseline Task 1 Evidence

- [x] Run focused greps and save results in the implementation notes:

```text
fresh_provisional_def_id
CrateId(u32::MAX)
LocalDefId(u32::MAX)
DefId::new(CrateId(0), LocalDefId(0))
placeholder
fallback
sentinel
missing canonical
provisional
```

- [x] Classify every hit as one of: `must remove`, `test-only`, `product-schema fallback`, `glob-export sentinel`, `diagnostic/error recovery`, or `other audit track`.
- [x] Add at least one RED test for every production `must remove` category before changing production code.
- [x] Keep this plan, the master checklist, and the ordered roadmap updated after each slice.

**Implementation notes, 2026-05-26:** Baseline grep audit found these production Task 1 cleanup buckets:

- `must remove`: collection provisional allocator in `collect/context.rs`, `collect/collector.rs`, and `collect/headers.rs`; post-header method ID assignment in `collect/mod.rs`; inference method ID repair in `infer/mod.rs`; product export accepting invalid current-crate sentinel IDs in `products.rs`; range lowering fallback to `DefId(0, 0)` in `lower/control_flow/secondary.rs`.
- `allowed`: glob export string sentinels in collect/lower export maps; tests that construct invalid IDs to validate rejection; product-local duplicate/collision fallback that does not accept invalid compiler HIR IDs as normal input; diagnostic/error guards for missing canonical IDs.
- `other audit track`: display/name fallback, parser fallback, codegen array projection fallback, MIR placeholder-agreement tests, and non-Task-1 `DefId(0, 0)` fixtures.

Baseline verification before edits: `cargo test -p rock-lib` passed with 1271 unit tests, 277 integration tests, and doc tests passing.

Final grep audit after implementation found no `fresh_provisional_def_id` or `provisional_def_id` hits. Remaining `CrateId(u32::MAX)` / `LocalDefId(u32::MAX)` hits are validation guards, rejection tests, or product-schema fallback handling. Remaining `DefId(0, 0)` hits are real local-ID-zero fixtures or unrelated tests. Remaining `placeholder` hits are MIR agreement checks, one crate-artifact regression, and `Type::Error` documentation. Remaining `fallback` hits are product-schema collision repair, parser/display/codegen compatibility behavior, or non-Task-1 tests. Remaining `sentinel` hits are glob-export string markers or artifact/test validation.

Follow-up compliance audit found two additional Task 1 gaps: unary operator semantic errors still recovered as executable `Unit` HIR, and public lower wrappers still exposed the legacy lowerer-owned declaration pass. Both were covered with RED tests before implementation and then fixed by routing unary failures through `error_expression()` and routing public lower entry points through indexed collection plus `lower_from_declarations`.

### Task 1: Allocate Trait And Impl Member IDs Before Header Building

- [x] Design a collection-owned member ID environment for trait methods, trait signatures, and impl methods.
- [x] Build that environment from the current source module tree, artifact source module tree, and loaded source-backed modules before header builders run.
- [x] Use deterministic keys that cannot collide across same-name modules or impls. Acceptable keys include canonical module path, owner `DefId`, member kind, member name, and impl ordinal.
- [x] Allocate member IDs from the same `IndexingIds` authority used for the current crate, or from a generated-definition allocator explicitly owned by collection and included in `current_def_ids`.
- [x] Pass explicit member IDs into trait and impl header builders.
- [x] Preserve existing method ABI flags, receiver metadata, generic owners, and qualified backend names.
- [x] Add tests proving trait methods, trait signatures, impl methods, and default methods have stable non-provisional IDs before body lowering.

### Task 2: Remove Provisional Header Builders

- [x] Remove or make test-only `headers::build_function_sig` if it fabricates an ID.
- [x] Replace all production callers with explicit-ID variants.
- [x] Remove `fresh_provisional_def_id` calls from `build_trait_with_id` and `build_impl_with_id`.
- [x] Ensure signature-owned generic params use the explicit signature/member ID as owner.
- [x] Add tests for generic trait signatures, generic trait methods, generic impl methods, and where-clause generic owners.

### Task 3: Make `LocalCollector` Missing IDs Fail Without Fabricating IDs

- [x] Change `LocalCollector::item_id_for_name` and `LocalCollector::next_impl_id` so they do not allocate provisional IDs on missing canonical identity.
- [x] Choose one explicit failure mode and apply it consistently:
  - return `Result<DefId, ResolveError>` and skip the invalid declaration after recording an error, or
  - panic on violated internal identity invariants after indexing is complete.
- [x] Prefer structured collection errors for user-source misses and invariant panics for impossible post-indexing states.
- [x] Add tests that intentionally omit an ID environment entry and prove collection fails before HIR body lowering with no sentinel IDs in declarations.
- [x] Remove `CollectContext::fresh_provisional_def_id` and the `provisional_def_id` field once all callers are gone.

### Task 4: Remove Current-Crate ID Repair From Post-Header Passes

- [x] Audit `apply_canonical_named_item_ids`, `apply_canonical_impl_item_ids`, `assign_canonical_method_ids`, and related remap functions.
- [x] Keep canonical remapping only where it maps already-explicit generated IDs to indexed IDs as a temporary transition within collection.
- [x] Remove any path that treats missing IDs as something to repair after declarations are built.
- [x] Ensure `current_def_ids` contains item-index IDs plus explicit generated definition IDs, never IDs allocated as fallback repairs.
- [x] Add tests proving missing IDs produce errors and valid generated IDs are present in `current_def_ids`.

### Task 5: Replace Trait Default Placeholder Method Semantics

- [x] Audit why collection leaves empty placeholder methods for trait defaults.
- [x] Replace placeholder detection with an explicit representation:
  - either impls record `DefaultMethodSelection { trait_id, default_method_id, impl_id }`, or
  - collection creates explicit generated impl method definitions with provenance pointing to the trait default.
- [x] Do not infer default-method replacement from an empty function body.
- [x] Do not remap default-method generic owners from provisional placeholder IDs.
- [x] Preserve behavior for impl overrides, trait generic substitution, associated type substitution, and backend symbol naming.
- [x] Add tests for default method injection, generic default method owners, overridden defaults, and same-name trait methods.

### Task 6: Replace Lowering Placeholder Recovery With Error-Typed Recovery

- [x] Replace arbitrary operator precedence fallback with a deterministic error path that does not silently choose precedence for semantic lowering.
- [x] Replace missing operator implementation `HirExprKind::Unit` recovery with error-typed HIR recovery such as `Type::Error` plus an expression form that later phases cannot treat as valid executable unit code.
- [x] Audit other lowering placeholder/fallback comments and classify each as diagnostic recovery or Task 1 identity fallback.
- [x] Ensure any diagnostic recovery carries source span and cannot introduce a semantic declaration or callable ID.
- [x] Add tests proving missing operator impls report errors and do not lower to valid `Unit` expressions that can pass later phases as real code.

### Task 7: Remove Or Constrain Fallback `DefId` APIs In Lowering

- [x] Audit `Lowerer::resolve_item_def_id_or` and all callers.
- [x] Remove it if all callers can use `resolve_item_def_id` plus explicit diagnostics.
- [x] If one narrow generated-definition call site needs a fallback, replace the generic helper with a named generated-ID API that records provenance and is included in `current_def_ids` or generated-definition metadata.
- [x] Audit `nominal_def_id_for_name` and generated impl paths for missing owner fallback behavior.
- [x] Add tests proving unsupported missing IDs are errors or invariant failures rather than repaired with fallback IDs.

### Task 8: Product And Artifact Boundary Guardrails

- [x] Ensure product emission validates that current-crate HIR does not contain `CrateId(u32::MAX)` before product ID remapping.
- [x] Keep product duplicate-ID fallback behavior only for product-local collision repair and producer/consumer schema translation.
- [x] Rename tests or comments that call real `DefId(0, 0)` a placeholder when the behavior now preserves local ID zero.
- [x] Add or update tests proving real local ID zero is valid, while sentinel current-crate IDs are rejected before artifact exposure.

### Task 9: Final Task 1 Sentinel Audit

- [x] Re-run the Task 0 grep set.
- [x] For each remaining hit, document why it is allowed and which future audit task owns it if not Task 1.
- [x] Expected final collect result: no production `fresh_provisional_def_id`, no production `CollectContext` provisional allocator, and no production collect/header `CrateId(u32::MAX)` declaration fallback.
- [x] Expected final lower result: no missing semantic identity fallback `DefId`, no sentinel generic owner repair, no valid-looking `Unit` placeholders for failed semantic operator resolution.
- [x] Expected final product result: product fallback remains only product-schema repair; current-crate compiler sentinel IDs are not accepted as normal input.

### Task 10: Documentation And Status Update

- [x] Update `master-audit-checklist.md`:
  - mark Identity And Arenas Task 1 current-crate ID allocation as done,
  - remove Task 1 collect/header and lowering placeholder recovery from “Still to do,”
  - keep non-Task-1 gaps under their correct sections.
- [x] Update `2026-05-17-compiler-architecture-ordered-roadmap.md`:
  - mark Task 1 complete without the generated/non-indexed caveat,
  - move any remaining non-Task-1 work to Tasks 3-5, 11, 13, 18, 21, or 23 as appropriate.
- [x] Add a final verification note with exact commands and results.

---

## Required Verification

Run these before claiming Task 1 complete:

```bash
cargo test -p rock-lib collect
cargo test -p rock-lib lower_function_sig
cargo test -p rock-lib conformance
cargo test -p rock-lib resolver_tables
cargo test -p rock-lib collect_artifact_declarations
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Also run the grep audit from Task 0 and include the classified results in the final update.

If a focused test name is too broad or slow in practice, use the smallest exact test filters that cover the changed code, then still run full `cargo test -p rock-lib` before marking Task 1 complete.

**Final verification, 2026-05-26:**

- `cargo test -p rock-lib collect`: passed; 129 unit tests matched, plus filtered integration/parser test binaries with 0 failures.
- `cargo test -p rock-lib lower_function_sig`: passed; 3 matched unit tests, 0 failures.
- `cargo test -p rock-lib conformance`: passed; 23 matched unit tests, 0 failures.
- `cargo test -p rock-lib resolver_tables`: passed; 3 matched unit tests, 0 failures.
- `cargo test -p rock-lib collect_artifact_declarations`: passed; 7 matched unit tests, 0 failures.
- `cargo test -p rock-lib missing_concrete_unary_operator_impl_lowers_to_error_typed_expression`: initially failed with `left: Unit` / `right: Error`, then passed after unary error recovery was changed.
- `cargo test -p rock-lib public_lower_entrypoint_uses_indexed_current_def_ids`: initially failed by panicking on `missing canonical DefId for lowered item: ["Show"]`, then passed after public lower wrappers were routed through indexed collection.
- `cargo test -p rock-lib`: passed; 1274 unit tests passed, 1 ignored; 277 integration tests passed; parser integration test passed; doc tests passed with 1 passed and 1 ignored.
- `cargo fmt --all --check`: passed with no output.
- `git diff --check`: passed with no output.
- Final grep audit: no `fresh_provisional_def_id` or `provisional_def_id` hits. Remaining sentinel/fallback/placeholder hits are validation guards, rejection tests, product-schema collision fallback, glob-export string sentinels, diagnostic/error recovery, `DefId(0, 0)` fixtures proving real local ID zero, dead private legacy lower collection helpers, or non-Task-1 compatibility tracks.
- Focused code review found inline-module member-ID lookup and nested product sentinel validation gaps; both were fixed and reverified. Follow-up compliance review found unary error recovery and public lower-wrapper gaps; both were fixed, reverified, and re-reviewed with no Critical or Important findings.

---

## Code Review Checklist

Request focused code review before committing the implementation. The reviewer must check:

- No production path fabricates current-crate declaration IDs after indexing.
- Header builders require explicit IDs for all semantic declarations and members.
- Generated definitions are explicit, provenance-carrying, and included in current definition metadata.
- Error recovery uses diagnostics and `Type::Error`-style recovery, not semantic placeholder IDs or real `Unit` expressions.
- Product/artifact fallback remapping is not mistaken for compiler current-crate ID allocation.
- Documentation does not overstate unrelated audit tracks as complete.

---

## Done Criteria

This plan is done when:

- All implementation tasks above are checked off.
- Required verification passes with fresh output.
- Final code review reports no Critical or Important findings.
- `master-audit-checklist.md` and the ordered roadmap both show Task 1 fully complete.
- The working tree is clean after a commit.
