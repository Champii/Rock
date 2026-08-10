# Extern Product Artifact Loading Design

**Date:** 2026-05-07
**Status:** Approved for planning
**Scope:** Add an explicit product-artifact dependency input path without changing current `--extern-artifact` behavior.

## Purpose

`rockc --emit-artifact` now writes product-backed artifacts from `CompilerProducts`, but `rockc --extern-artifact` still loads the existing `CrateArtifact` format. Reusing `--extern-artifact` for product artifacts immediately would silently change an established CLI boundary and risk breaking current crate artifact tests and cached artifacts.

The next migration slice should add a temporary, explicit product dependency input flag. This lets the compiler consume product artifacts in focused tests while keeping current artifact loading untouched until product-backed dependencies prove sufficient.

## Goals

- Add a separate `rockc --extern-product-artifact name=path` flag.
- Add `rock_lib::Config.extern_product_artifacts` beside existing `extern_artifacts`.
- Preserve existing `--extern-artifact` old-format loading with no format sniffing or fallback behavior.
- Load product artifacts only when the explicit product flag is used.
- Convert product artifact metadata, bodies, and link data into the current dependency-facing compiler structures needed by downstream compilation.
- Prove one crate can consume a direct dependency emitted by `rockc --emit-artifact` through the new flag.

## Non-Goals

- Do not make `--extern-artifact` accept product artifacts in this slice.
- Do not remove `CrateArtifact`, `CrateContext::build_artifact`, source bundles, or old artifact tests.
- Do not switch `rock` to product artifacts yet.
- Do not redesign all dependency-facing compiler phases around a new trait/interface in this slice.
- Do not add automatic stdlib discovery or unqualified stdlib injection.

## CLI Shape

The new flag mirrors `--extern-artifact` syntax but is intentionally separate:

```text
rockc \
  --entry-file app/main.rk \
  --extern-product-artifact dep=build/dep.rkca
```

`--extern-artifact dep=path` remains the old `CrateArtifact` path. `--extern-product-artifact dep=path` means `path` must contain a `ProductArtifact` produced by `rockc --emit-artifact`.

Both flags may exist in the same command only for different crate names. If the same crate name appears in both lists, `rockc` should reject the configuration with a clear diagnostic rather than choosing one silently.

## Compiler Boundary

Add product dependency loading to the current explicit dependency-loading phase in `rock_lib::compile_impl`:

1. Load old `extern_artifacts` exactly as today through `CrateContext::load_artifact_from_path`.
2. Load `extern_product_artifacts` through a new product loader.
3. Reject duplicate crate names across both input lists before loading.

The first implementation may adapt product artifacts into `LoadedCrate` because existing collect/lower/mono phases consume `CrateContext`. This adapter is not hidden compatibility behavior: it is an explicit product-artifact dependency bridge used only by `--extern-product-artifact`.

## Product Dependency Adapter

The adapter should derive current dependency-facing data from `CompilerProducts`:

- crate name and version from `ProductCrateIdentity`
- object path from `ProductLinkData.object_path`
- string-keyed interface maps from `ProductMetadata` and `ProductIdentityTable.display_names`
- generic/default downstream bodies from `ProductBodies`
- resolver tables from product display/export names and the `DefId`s already present inside cloned HIR values
- root export aliases from `ProductIdentityTable.export_names`

The adapter should be conservative. If a required fact cannot be derived for a product artifact used by a test case, the loader should fail with a clear error rather than falling back to source or old artifacts.

## Data Flow

The focused end-to-end flow is:

1. Compile dependency crate from source with `rockc --crate-name dep --emit-artifact dep.rkca --emit-object dep.o --no-link`.
2. Compile app crate from source with `rockc --extern-product-artifact dep=dep.rkca`.
3. The app compile loads dependency metadata/body/link data from the product artifact.
4. The app compile links against the dependency object path recorded in the product artifact.

This keeps one source crate per `rockc` invocation and avoids parsing dependency source in the app compile.

## Error Handling

Diagnostics should distinguish:

- old artifact load failures from `--extern-artifact`
- product artifact read/deserialization failures from `--extern-product-artifact`
- duplicate crate names across old and product artifact inputs
- product artifacts missing required object path or metadata for downstream compilation

## Testing

Testing should prove separation first, then behavior:

- CLI parse test for `--extern-product-artifact dep=path`.
- Config/compile test rejecting duplicate crate names across `extern_artifacts` and `extern_product_artifacts`.
- Product loader unit test converting a small `CompilerProducts` value into a loaded dependency interface.
- Runtime smoke test: emit a product artifact/object for a dependency with `rockc`, then compile and run an app with `--extern-product-artifact`.
- Existing `CrateArtifact` tests must keep passing, especially `cargo test -p rock-lib crate_artifact -- --nocapture`.

## Migration Boundary

This flag is temporary but useful. It gives product artifacts a real downstream compile path without destabilizing existing artifact users. Once product-backed dependencies cover the required cases, a later cleanup can decide whether to make `--extern-artifact` accept only the final artifact format or retire the temporary flag.
