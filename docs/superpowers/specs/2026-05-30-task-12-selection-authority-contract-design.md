# Task 12 Selection Authority Contract Design

## Goal

Finish ordered roadmap Task 12 by making the shared selection service the explicit frontend authority for trait, method, operator, and index selection decisions, while keeping fallback deletion and backend metadata extraction assigned to Tasks 13 and 21.

## Scope

This work completes the Task 12 authority contract. It does not absorb later cleanup tasks.

In scope:
- Clarify the ordered roadmap so Task 2's remaining string-keyed staging/compatibility maps are tracked under Tasks 4-5, 18, and 21, not as unfinished Task 2 work.
- Define and test the selection authority contract in `lib/src/selection/`.
- Ensure lowering-time selection produces explicit selected identity/fact data that downstream phases can treat as authoritative when present.
- Add contract tests covering same-name, artifact-backed, generic trait-bound, stdlib operator/index, and associated-output/projection cases.
- Update audit and roadmap docs so Task 12 is marked complete once the authority contract is implemented and verified.

Out of scope:
- Removing all mono/codegen targetless fallback paths. That remains Task 13 work.
- Moving declaration, layout, product, symbol, and link metadata out of HIR/mono compatibility inputs. That remains Task 21 work.
- Reworking product artifact schemas beyond what is needed to preserve existing selected identity payloads.
- Redesigning trait solving or introducing a new solver.

## Current State

`lib/src/selection/` already centralizes lowering-time trait, method, operator, and index selection for the scoped slice completed in `docs/superpowers/plans/2026-05-19-selection-service.md`.

The roadmap still describes Task 12 as incomplete overall because the service is not documented or tested as the full frontend authority contract. Master audit evidence also notes that monomorphization and codegen still have fallback rediscovery paths. Those fallback paths are real, but they belong to Task 13 and Task 21 once the Task 12 selected contract is complete.

## Design

### Roadmap Clarification

Update `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` to clarify two boundaries:

- Task 2 is complete for finalized HIR ID-keyed ownership storage. Remaining string-keyed staging/compatibility maps are tracked by Tasks 4-5, 18, and 21.
- Task 12 completion means the shared selection service is the authoritative frontend selector and emits an explicit contract. Removing targetless fallback consumers remains Task 13/21 work.

This avoids treating later compatibility cleanup as unfinished Task 2 or Task 12 implementation.

### Selection Authority Contract

The selection service contract is the data and behavior downstream phases may trust when a selected target is present.

The contract should cover:
- selected impl identity when an impl method or trait impl is selected.
- selected method/function identity when dispatch resolves to a concrete callable.
- selected trait identity and trait arguments for trait-mediated dispatch.
- dispatch kind / receiver adjustment facts already modeled by the selection types.
- associated output or projection facts when the selection determines them.
- trait default origin when a default method body is selected or injected.
- structured diagnostics when selection fails.

The exact Rust structs may reuse existing selection types if they already carry these facts. The implementation should prefer extending existing `selection::types` shapes over inventing parallel records.

### Lowering Boundary

Lowering should continue to call the selection service for semantic decisions and should record selected target facts into HIR sidecars such as `HirMethodCallTarget` or existing selected-target records.

The selected facts are authoritative when present. Later mono/codegen fallbacks may still exist temporarily, but tests should prove they are not needed for the covered Task 12 authority cases.

### Tests

Add focused contract tests near `lib/src/selection/` when possible. Use integration or lowering/product tests only when the behavior requires the whole pipeline.

Required coverage:
- same-name traits or methods select by identity, not display name.
- artifact-backed methods preserve selected identity across product load.
- generic trait-bound dispatch records trait identity and trait args.
- stdlib operator/index selection goes through the shared service contract.
- associated type output/projection selection records enough facts for downstream consumers.
- diagnostics preserve selected-target context for failures.

Tests should be TDD-first. Each behavior change needs a failing test observed before implementation.

### Documentation Completion

After implementation and review:
- Mark ordered roadmap Task 12 as complete, with remaining fallback deletion explicitly assigned to Task 13 and backend metadata extraction to Task 21.
- Update the master audit `Trait And Method Selection Service` section so Task 12-specific items are complete and remaining work is only Task 13/21 cleanup.
- Keep Type Context And Semantic Types complete and do not reopen Task 11.

## Verification

Minimum verification:
- focused selection tests under `lib/src/selection/`.
- focused lowering/operator/index/trait default tests touched by the implementation.
- `cargo test -p rock-lib selection`.
- `cargo test -p rock-lib product_artifact` if artifact-backed selection payloads are touched.
- `cargo test -p rock-lib` before marking Task 12 complete.
- `cargo fmt --all --check`.
- `git diff --check`.

## Risks

- The boundary between Task 12 and Task 13 can blur. The plan must avoid deleting broad fallback paths unless needed for a focused Task 12 test.
- Existing selected-target structs may already carry some required facts under different names. The implementation should audit before adding new fields.
- Artifact-backed selected identity can cross product remapping boundaries. Tests must cover remapped product IDs when selection sidecars are serialized or loaded.
- Over-broad codegen cleanup could accidentally pull in Task 21. Keep backend metadata extraction out of this work.
