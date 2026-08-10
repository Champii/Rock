# Lower Bootstrap Helper Cleanup Design

**Date:** 2026-04-28
**Status:** Auto-approved for implementation
**Scope:** Remove the now-dead lower bootstrap bridge helpers left behind by the collect-owned bootstrap refactors, while preserving the still-active lower-owned dependency/body-lowering helpers used by the main compile path.

## Purpose

The active collect path and both artifact declaration/bootstrap entry points now construct collect-owned bootstrap state directly. That leaves two bridge helpers behind as dead transitional code:

- `lib/src/lower/mod.rs::Lowerer::bootstrap_for_collection(...)`
- `lib/src/collect/context.rs::CollectContext::from_bootstrap(...)`

They no longer participate in any production path and currently survive only through a bridge-oriented collect test. The next narrow step is to remove that dead bridge cleanly and verify that the warning surface disappears.

## Goals

- Remove `Lowerer::bootstrap_for_collection(...)`.
- Remove `CollectContext::from_bootstrap(...)`.
- Replace the bridge-specific test with collect-owned coverage that still protects bootstrap-state carriage where it matters.
- Prove the cleanup through a focused `-D warnings` verification gate and the normal `rock-lib` test suite.

## Non-Goals

- Do not remove lower-owned dependency registration helpers used by `lower_from_declarations(...)` and later body lowering.
- Do not touch `lib/src/lower/collect/declarations.rs` in this step.
- Do not start resolver tables or canonical ID work yet.
- Do not change artifact behavior.

## Why This Is The Next Step

After the artifact interface and artifact cross-crate HIR splits, the remaining architecture debt at this seam is no longer behavior in production code but dead bridge code and the warnings it produces. Removing those helpers is the smallest behavior-neutral step that closes the current bootstrap cleanup thread before moving into resolver-owned tables.

## Architecture

### Remove The Dead Lower Bootstrap Bridge

Delete the lower-owned bootstrap constructor and the collect-side import bridge. The collect phase already has its own bootstrap constructor and does not need to route state through `Lowerer` anymore.

### Keep Active Lower-Owned Runtime Helpers

Do not remove lower-owned crate registration or declaration helpers that are still used by the main compile/body-lowering path. This slice is only about the dead bootstrap bridge, not broader lower helper removal.

### Replace Bridge Coverage With Collect-Owned Coverage

The current bridge test should stop constructing a bootstrap `Lowerer`. Replace it with a test that seeds collect-owned state directly and asserts that the collect context/local collection still carries the relevant bootstrap maps:

- `import_aliases`
- `stdlib_prelude_exports`
- `artifact_module_index`
- `export_function_aliases`

That keeps meaningful coverage of collect-owned state carriage without preserving the dead bridge.

## Boundary Rules

- `collect` remains the only owner of collect-time bootstrap construction.
- `lower` remains the owner of body lowering and the still-active dependency/body-loading helpers used after `Lowerer::from_declarations(...)`.
- This slice must not change artifact or compile semantics.

## Testing

- First, run a focused warning gate with `RUSTFLAGS="-D warnings"` on a small `rock-lib` test target and observe the current failure from dead helpers.
- After cleanup, rerun that same command and require success.
- Run the focused collect tests around the updated coverage.
- Run full `cargo test -p rock-lib`.

## Risks

- Removing the bridge too aggressively could accidentally touch still-active lower-owned helpers.
- Replacing the test poorly could drop coverage of collect-owned bootstrap maps instead of only removing the obsolete bridge.

## Follow-Up Work

- Start the first resolver-owned canonical path/alias table slice.
- Continue shrinking lower-owned declaration helpers only when their remaining call sites are eliminated by those resolver steps.
