# Rock Product Artifact Subprocess Driver Design

**Date:** 2026-05-07
**Status:** Approved for planning
**Scope:** Move `rock` package builds to dev-target `rockc` subprocess invocations that produce and consume product artifacts for package dependencies.

## Purpose

`rock` still compiles packages in-process. It calls `rock_lib::compile` for root builds, calls `rock_lib::compile` again for dependency object files, and uses `CrateContext::build_artifact` to build old-format dependency artifacts. That keeps the package manager coupled to compiler internals and preserves the artifact-specific re-lowering path.

`rockc` can now emit product artifacts with `--emit-artifact` and consume them explicitly with `--extern-product-artifact`. The next slice should make `rock` use that boundary for normal package dependencies while keeping old stdlib artifact handling intact until stdlib product migration is designed separately.

## Goals

- Add a focused `rockc` subprocess boundary inside `rock`.
- Resolve `rockc` from the active Cargo target profile directory during development, not from the system `PATH`.
- Build stale package dependencies by invoking `rockc --emit-artifact --emit-object --no-link`.
- Pass direct package dependencies to `rockc` with `--extern-product-artifact name=path`.
- Build the root executable by invoking `rockc` as a subprocess instead of `rock_lib::compile`.
- Keep sysroot stdlib as an old-format `--extern-artifact stdlib=...` input for this slice.
- Keep `rock` responsible for dependency traversal, freshness decisions, output paths, and subprocess error context.

## Non-Goals

- Do not migrate stdlib artifacts to product artifacts in this slice.
- Do not remove `CrateContext::build_artifact` from `rock_lib`; only stop using it from `rock` package dependency builds.
- Do not make `--extern-artifact` load product artifacts.
- Do not remove source bundles or old `CrateArtifact` tests.
- Do not redesign package graph resolution beyond what is needed for product-artifact subprocess builds.
- Do not add compiler-owned stdlib discovery or implicit stdlib injection.

## Subprocess Boundary

Add a small helper module such as `rock/src/rockc.rs`. It should own:

- resolving the `rockc` executable path
- constructing root-build command arguments
- constructing dependency artifact-build command arguments
- running commands and reporting non-zero exit status with crate/package context

Default `rockc` resolution must use the current Cargo target profile. Test binaries run from `target/<profile>/deps`, so the resolver should derive the profile directory from `std::env::current_exe()` and join the sibling executable name `rockc` or `rockc.exe`. A `ROCKC` environment override is acceptable for explicit workflows, but default development behavior must never choose an installed system `rockc` from `PATH`.

## Dependency Artifact Build Command

For each stale package dependency, `rock` should invoke:

```text
rockc \
  --crate-name <crate-name> \
  --entry-file <crate-root>/<manifest lib path> \
  --output-dir <crate-root>/build/objects \
  --no-link \
  --emit-object <crate-root>/build/objects/<entry-stem>.o \
  --emit-artifact <crate-root>/build/artifacts/<crate-name>-<version>.rkca \
  --extern-product-artifact <direct-package-dep>=<dep-product-artifact> ... \
  --extern-artifact stdlib=<stdlib-old-artifact> # only when this package needs stdlib
```

Package dependencies use the product flag. The stdlib input remains old-format through `--extern-artifact` because the bundled sysroot still produces and serves old `CrateArtifact` files.

## Root Build Command

For root executable builds, `rock` should invoke:

```text
rockc \
  --entry-file <root-entry> \
  --output-dir <root-build-dir> \
  --extern-product-artifact <direct-package-dep>=<dep-product-artifact> ... \
  --extern-artifact stdlib=<stdlib-old-artifact> # only when root needs stdlib
```

`rock` should keep returning the expected executable path based on the root output directory and entry file. `rock run` should continue running that executable and forwarding args.

## Freshness

Current freshness reads old `CrateArtifact` fields. Product package artifacts should not require `rock` to inspect product internals. For product-built package dependencies, freshness should be based on:

- artifact path exists
- object path exists
- manifest and source files under the package root are older than both artifact and object outputs
- direct dependency artifacts are older than both artifact and object outputs
- implicit stdlib artifact input, if any, is older than both artifact and object outputs

Source-file tracking can start with deterministic package source discovery under the package root for `.rk` files plus `rock.toml`. It should ignore the package `build/` directory to avoid build output feeding freshness. This is less exact than old artifact-loaded file caches for now, but it is conservative enough for the migration slice and does not tie `rock` to product artifact internals.

## Data Flow

1. `rock` loads the root package manifest.
2. Existing recursive dependency traversal calls `ensure_artifact` for dependencies before dependents.
3. `ensure_artifact` returns product artifact paths for package dependencies.
4. If a package dependency is stale, `rock` invokes `rockc` once for that package with direct package dependencies as product artifacts and stdlib, if needed, as an old artifact.
5. `build_project` invokes `rockc` once for the root package with direct package dependencies as product artifacts and stdlib, if needed, as an old artifact.
6. `rockc` remains the only compiler process for package builds.

## Error Handling

`rock` should preserve compiler diagnostics printed by `rockc` and add orchestration context:

- missing dev-target `rockc` executable
- failed subprocess spawn
- non-zero `rockc` exit status
- crate/package name being built
- whether the command was dependency artifact emission or root executable build

Freshness and path errors should include the package root or affected output path.

## Testing

Testing should cover the subprocess boundary and package behavior:

- unit tests for resolving dev-target `rockc` from a test binary path
- unit tests for dependency artifact command arguments, including `--extern-product-artifact`, `--emit-artifact`, and `--emit-object`
- unit tests for root build command arguments
- freshness tests proving unchanged product artifacts are reused and source changes rebuild them
- runtime `rock` build test proving transitive package dependencies compile, product artifacts exist, product objects exist, and the executable runs
- sysroot regression tests proving stdlib is still passed as old `--extern-artifact`
- `cargo test -p rock`, `cargo test -p rockc`, and focused `rock-lib` product/artifact tests

## Migration Boundary

After this slice, normal `rock` package dependencies should be product artifacts built by `rockc`. The bundled stdlib remains old-format and explicit. Later slices can migrate stdlib product artifacts, remove old artifact builder re-lowering paths, and decide when the temporary `--extern-product-artifact` flag should converge with the final artifact CLI.
