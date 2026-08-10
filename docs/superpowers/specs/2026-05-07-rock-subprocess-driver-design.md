# Rock Subprocess Driver Design

**Date:** 2026-05-07
**Status:** Approved for planning
**Scope:** Move `rock` build and artifact production paths from in-process compiler calls to `rockc` subprocess invocations while preserving current artifact loading behavior.

## Purpose

`rock` currently owns part of the compiler pipeline directly. It calls `rock_lib::compile` for root builds, calls `rock_lib::compile` again to produce dependency objects, and uses `CrateContext::build_artifact` to create dependency artifacts. This keeps `rock` coupled to compiler internals and preserves the old artifact-production path that re-runs semantic collection/lowering.

The next migration slice should make `rock` act like a package/build orchestrator: resolve packages, decide freshness, compute build order through the existing recursive dependency traversal, and invoke `rockc` for each crate that must be compiled.

## Goals

- Replace `rock_lib::compile` calls in `rock` build paths with `rockc` subprocess invocations.
- Produce dependency objects and `.rkca` artifacts through `rockc --emit-object` and `rockc --emit-artifact`.
- Preserve existing dependency graph traversal, freshness checks, sysroot stdlib behavior, and current `--extern-artifact` downstream loading.
- Keep `rock` responsible for paths, crate graph order, cache decisions, and subprocess error reporting.
- Keep compiler semantics in `rockc`/`rock_lib`, not in `rock`.

## Non-Goals

- Do not implement product-artifact downstream loading in this slice.
- Do not remove `CrateContext::build_artifact` from `rock_lib`; only stop using it from `rock` where this slice touches build paths.
- Do not remove source bundles or current `CrateArtifact` fields globally.
- Do not redesign package graph resolution or artifact freshness from scratch.
- Do not make `rock` inspect product-artifact internals.

## Approach

Add a small subprocess boundary inside `rock`, preferably a dedicated helper module such as `rock/src/rockc.rs`. The helper builds and runs `std::process::Command` values that invoke the sibling `rockc` executable. In tests, command construction should be testable without spawning a compiler process.

During development, `rock` must not resolve `rockc` from the system `PATH`. The subprocess boundary should resolve the workspace-built compiler from the active Cargo target directory, such as `target/debug/rockc` for debug builds and `target/release/rockc` for release builds. Test binaries run from `target/<profile>/deps`, so the resolver should derive the target profile directory from `std::env::current_exe()` and then join the sibling `rockc` binary name. A `ROCKC` environment override may be useful for tests or explicit developer workflows, but the default development behavior must use the dev target folder, not an installed compiler.

Dependency crate artifact production should call `rockc` with:

```text
rockc \
  --crate-name <crate-name> \
  --entry-file <crate-root>/<manifest lib path> \
  --output-dir <crate-root>/build/objects \
  --no-link \
  --emit-object <crate-root>/build/objects/<entry-stem>.o \
  --emit-artifact <crate-root>/build/artifacts/<crate-name>-<version>.rkca \
  --extern-artifact <direct-dep>=<path> ...
```

Root executable builds should call `rockc` with:

```text
rockc \
  --entry-file <root-entry> \
  --output-dir <root-build-dir> \
  --extern-artifact <direct-dep>=<path> ...
```

Both command forms should pass `--no-std` when the package manifest disables stdlib or the package is the stdlib crate itself. They should not pass `--no-prelude` unless an existing caller already needs that behavior.

## Data Flow

1. `rock` loads the root package manifest.
2. Existing recursive dependency traversal calls `ensure_artifact` for dependencies before dependents.
3. `ensure_artifact` keeps its freshness check and build cache state.
4. When a dependency artifact is stale, `rock` invokes `rockc` once for that dependency with direct dependency artifacts, explicit object output, and explicit artifact output.
5. `build_project` invokes `rockc` once for the root executable with direct dependency artifacts.
6. Current downstream artifact loading remains inside `rockc` and `rock_lib` through the existing `--extern-artifact` path.

## Error Handling

The subprocess helper should report:

- failure to locate or execute `rockc`
- failure to locate the workspace-built `rockc` in the active target profile directory
- non-zero `rockc` exit status
- the package/crate name being compiled
- the command role, such as dependency artifact build or root executable build

`rockc` should keep printing compiler diagnostics. `rock` should add orchestration context without duplicating compiler diagnostics.

## Testing

Testing should start at the subprocess boundary and then exercise the existing package tests:

- Unit tests for command argument construction for dependency artifact emission.
- Unit tests for command argument construction for root executable builds.
- Unit tests for resolving `rockc` from a target profile directory instead of the system `PATH`.
- A runtime `rock` build test proving transitive dependency artifacts and objects still appear and the executable runs.
- Existing `cargo test -p rock` should remain the primary verification for this slice.

## Migration Boundary

This phase moves ownership of compilation invocation, not artifact interpretation. Product-backed artifacts may be emitted by `rockc`, but current dependency consumption still goes through existing `--extern-artifact` loading. Later phases can replace downstream artifact loading and remove old artifact builders once product artifacts can satisfy dependencies end-to-end.
