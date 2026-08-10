# Legacy DCE Helper Cleanup Design

## Goal

Retire the legacy non-pipeline HIR/name-based DCE helpers now that the compile pipeline uses instance reachability over `MonomorphizedProgram.instances`.

This is the next focused Monomorphization Instances cleanup after ID-keyed mono semantic lookup. It removes dead compatibility code without changing the active compiler DCE authority.

## Current State

`lib/src/dce.rs` has two DCE paths:

- Active path: `prune_unreachable_instances`, which prunes monomorphized instance records and is called by the compiler pipeline.
- Legacy path: `prune_dead_functions`, `count_dead_functions`, and the `collect_calls_*` helper chain, which operate over HIR function names and are retained only for local compatibility tests.

Searches show the legacy helpers are only referenced in `lib/src/dce.rs` tests. There are no production callers outside the file.

## Scope

In scope:

- Delete `prune_dead_functions`.
- Delete `count_dead_functions`.
- Delete the legacy `collect_calls_block`, `collect_calls_stmt`, and `collect_calls_expr` helper chain if no active code uses it.
- Delete tests that exist only to exercise those legacy helpers.
- Update comments in `lib/src/dce.rs` so the module no longer says older HIR/name helpers remain.
- Update `master-audit-checklist.md` and the ordered roadmap after verification.

Out of scope:

- Changing `prune_unreachable_instances` behavior.
- Reworking instance reachability indexes.
- Changing the compiler pipeline DCE call site.
- Changing instance body representation away from `HirFunction`.
- Claiming the whole Monomorphization Instances track complete.

## Design

Remove the legacy name-based DCE implementation entirely instead of hiding it behind `#[cfg(test)]`. The compatibility tests that depended on `count_dead_functions` and `prune_dead_functions` should be removed unless they prove behavior not already covered by instance reachability tests.

The active instance-DCE tests should remain. In particular, tests proving that instance reachability does not root by backend symbol, source name, callable variable name, or wrong trait/member names should stay in place because they cover the actual pipeline authority.

## Error Handling

This cleanup removes unused code and should not introduce new user-facing error paths. If deleting helper code exposes an accidental production dependency, the implementation should stop and preserve behavior by routing that dependency through `prune_unreachable_instances` rather than restoring name-based DCE.

## Testing

Focused verification should include:

- A search proving `prune_dead_functions` and `count_dead_functions` have no remaining references.
- Focused `dce::tests::prune_unreachable_instances...` regressions that cover current DCE behavior.
- `cargo test -p rock-lib dce::tests -- --nocapture`.
- `cargo fmt --all --check`.
- `git diff --check`.
- Full `cargo test -p rock-lib` before final completion.

## Success Criteria

- Legacy HIR/name DCE helper APIs and helper traversal functions are removed.
- Only instance reachability remains as the DCE implementation in `lib/src/dce.rs`.
- Audit and roadmap docs mark only the legacy DCE helper cleanup complete.
- Instance body representation cleanup remains open.
