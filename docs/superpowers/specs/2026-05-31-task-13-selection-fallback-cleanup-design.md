# Task 13 Selection Fallback Cleanup Design

## Goal

Complete roadmap Task 13 by deleting or hardening targetless mono/lowering selection fallback paths now that Task 12 defines the frontend selection authority contract.

Task 13 should prove that selected method/operator/index/trait facts produced by `lib/src/selection/` are consumed as authoritative call-edge data. It should not absorb broader backend metadata extraction or codegen trait/member registry cleanup assigned to Task 21.

## Scope

In scope:
- Identify remaining targetless method/operator/index/trait dispatch fallback paths in lowering and monomorphization.
- Remove or narrow semantic rediscovery by method name, receiver display name, generated backend symbol, or first matching impl where a `HirMethodCallTarget` should exist.
- Preserve explicitly supported non-dispatch call forms such as direct function calls, resolved static impl method functions, object-backed extern calls, and already materialized `InstanceId` calls.
- Add regressions that fail if mono/lowering silently reconstructs selected impls or methods after a selected target is missing.
- Update roadmap and audit docs only after verification.

Out of scope:
- Removing codegen trait/member metadata registration and direct codegen-side trait search. That remains Task 21 unless a narrow Task 13 regression requires a small guard.
- Moving declaration, layout, symbol, extern, product, or link metadata out of HIR/mono compatibility inputs.
- Redesigning the selection service, trait solver, instance registry, or product artifact schema.
- Reopening Task 12 authority-contract work or Task 11 type-context work.

## Current State

Task 12 made selected results expose an explicit authority view with impl, method, trait, trait-arg, receiver-adjustment, return/output, builtin-index, and diagnostic facts. Existing mono/codegen paths already treat present `HirMethodCallTarget` values as authoritative in several migrated paths.

Task 13 remains partial because targetless compatibility paths still exist. Examples include fallback logic in `lib/src/mono/methods.rs` and `lib/src/mono/process.rs` that can still recover method behavior from names or receiver facts when selected targets are absent. Some codegen-side trait registration and lookup also remains, but that is backend metadata cleanup and should stay assigned to Task 21.

## Design

### Boundary

Task 13 should establish a clear invariant:

When a HIR method call represents semantic dispatch selected by the frontend, its `HirMethodCallTarget` is the call-edge authority. Monomorphization must not silently replace a missing target by searching impls or traits by name.

The implementation should distinguish three cases:

- Targeted dispatch: use `HirMethodCallTarget` to select impl, method, trait identity, trait args, and index-operator shape.
- Non-dispatch callable edges: direct function IDs, static impl method function IDs, object-backed functions, and `InstanceId` references remain valid without method-call target fallback.
- Invalid targetless dispatch: method/operator/index calls that previously depended on semantic rediscovery should report a deterministic compiler/mono error or be rejected by a focused validation helper.

### Implementation Approach

Prefer small deletions and guard clauses over a broad new pass.

Likely changes:
- In `lib/src/mono/methods.rs`, remove or narrow branches that search impls by method name when a method call has no selected target but should be dispatch-selected.
- In `lib/src/mono/process.rs`, keep existing selected-target and `InstanceId` handling, but harden fallback paths so they do not rediscover missing selected method targets.
- In lowering files such as `lib/src/lower/expression.rs` and `lib/src/lower/control_flow/secondary.rs`, add focused checks only if tests show lowering can still emit targetless semantic dispatch for supported call edges.
- Keep codegen registration/search behavior intact unless a narrow Task 13 test exposes a direct dependency on targetless selected-call rediscovery.

The change should not invent a new selection record. It should consume existing `HirMethodCallTarget` and Task 12 `SelectionAuthority` facts.

### Diagnostics And Errors

If a targetless dispatch edge must now fail, prefer an existing structured diagnostic or phase error shape over `panic!` in user-facing paths. Test-only hard invariants may still use panics if they match nearby mono tests.

Error messages should state that a selected method target is missing or that targetless method dispatch is unsupported. They should avoid implying a source-level trait resolution failure when the real issue is an internal missing call target.

## Tests

Use TDD for each behavior change.

Required coverage:
- Same-name impl methods do not dispatch through targetless method-name lookup.
- Trait/default method calls use selected trait and method identity, not first matching name.
- Artifact-backed or qualified impl owners do not fall back to display-name matching when selected targets are absent.
- Index/operator calls preserve selected target identity and do not recover through builtin or trait-name fallback after a missing target.
- Missing selected target on a semantic method call fails deterministically at the mono/lowering boundary.
- Existing direct function, resolved static impl method, object-backed, and `InstanceId` call paths continue to pass.

Useful verification commands:
- Focused mono/lowering tests for each changed path.
- `cargo test -p rock-lib selection`
- `cargo test -p rock-lib product_artifact` if artifact-backed call behavior is touched.
- `cargo test -p rock-lib`
- `cargo fmt --all --check`
- `git diff --check`

## Documentation Completion

After implementation and verification:
- Mark ordered roadmap Task 13 complete only for mono/lowering targetless selection fallback cleanup.
- Keep Task 21 as the owner for codegen direct trait/member metadata cleanup and backend metadata extraction.
- Keep Task 12 marked complete and do not reopen Type Context And Semantic Types.

## Risks

- Some targetless paths may still be needed for valid static function-value or object-backed calls. Tests must separate those from semantic method dispatch.
- Deleting broad codegen lookup behavior would blur Task 13 with Task 21 and increase backend regression risk.
- Missing-target failures may expose legacy lowering gaps. Fix the lowering producer only when the edge is a supported selected dispatch form.
- Overly strict rejection could break artifact-backed or generated call edges that intentionally use direct function/instance identity rather than method-call targets.

## Spec Self-Review

- Placeholder scan: no placeholders or unresolved TODOs remain.
- Scope check: focused on Task 13 targetless fallback cleanup; Task 21 backend metadata cleanup remains explicit out of scope.
- Consistency check: design relies on existing Task 12 selection authority and `HirMethodCallTarget`, not a new selection mechanism.
- Ambiguity check: targetless semantic dispatch should fail or be produced with a selected target; non-dispatch direct callable edges remain valid.
