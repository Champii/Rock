# Artifact-Only Downstream Dependencies Design

**Date:** 2026-05-11
**Status:** Draft for review

## Purpose

Product artifacts are now the public dependency artifact format, but the compiler library still carries source-backed external dependency paths in `collect`, `lower`, and `mono`. This slice finishes the boundary from the earlier artifact-only designs: a compiler invocation may read source for exactly one crate, the current crate being compiled. Every other crate visible to that invocation must be represented by product artifact data.

The goal is not to remove source compilation. The goal is to remove dependency-source consumption from downstream compiler phases so dependency semantics always come from artifact metadata, resolver tables, cross-crate HIR bodies, and link data.

## Core Rule

For any `rockc` or `rock_lib::compile` invocation:

- The entry crate is the only source crate the invocation may parse, collect, lower, infer, monomorphize, or codegen from source.
- All dependencies, including `stdlib`, must enter through explicit product artifacts.
- `collect`, `lower`, `mono`, MIR, and codegen must not inspect dependency ASTs, dependency module file caches, or dependency source-backed module bodies.
- If a dependency body needed for generic/default-method compilation is absent from the artifact body section, that is an artifact-production error or unsupported artifact mode, not a reason to fall back to dependency source.

When `rock` builds a source path dependency, it should invoke `rockc` with that dependency as the current crate. In that separate invocation, the dependency source is current-crate source, and its own dependencies must be artifacts.

## Current Evidence

The public artifact boundary is already mostly in place:

- `rock_lib::Config` exposes `extern_artifacts`, and `compile_impl` loads them with `CrateContext::load_product_artifact_from_path_as`.
- `rockc` uses `--extern-artifact` for product artifacts and rejects the temporary product-artifact flag.
- `rock` and `rockup` shell out through `rockc` for package and sysroot artifact production.

The remaining downstream source-backed paths are internal compiler-library branches:

- `CrateContext::load_crate_from_dir` creates `LoadedCrate` values with `ArtifactMode::Source`.
- `collect` skips artifact-interface registration for source-backed loaded crates and recollects dependency ASTs.
- `lower` lowers source-backed dependency trait defaults and function bodies from ASTs.
- `mono` scans source-backed dependency ASTs for generic functions and impls.

These paths are useful only as transitional artifact-production helpers or legacy tests. They should not be reachable from normal downstream dependency consumption.

## Target Architecture

`LoadedCrate` should present dependency capabilities, not storage-mode-specific phase behavior. For downstream compilation, dependency data should be accessed through these artifact-backed capabilities:

- metadata: functions, externs, structs, enums, traits, impl headers, infix precedence, resolver aliases, and prelude exports
- body provider: generic function bodies, generic impl bodies, trait default bodies, and any future body categories required for specialization
- link provider: object path and backend symbol/link metadata for object-backed dependencies

The compiler phases should not ask whether a dependency is source-backed. They should ask for dependency metadata, body data, or link data. If those capabilities are missing for a dependency, compilation should fail with a clear diagnostic or error path.

`ArtifactMode::Source` can remain only if it is confined to artifact-production setup or tests that compile a crate as the current crate. It must not be a branch that causes dependencies to be consumed from source during another crate's compilation.

## Implementation Boundaries

This slice should remove or quarantine source-backed external dependency consumption from:

- `lib/src/collect/context.rs`: no dependency AST recollection in normal dependency registration
- `lib/src/lower/crates/registration.rs`: no legacy source-backed registration branch for downstream dependencies
- `lib/src/lower/crates/bodies.rs`: no source dependency trait-default or function-body lowering
- `lib/src/mono/external.rs`: no dependency AST/file-cache traversal for generic functions or impls
- tests that only prove source-backed external dependency consumption

This slice should preserve:

- current-crate source parsing and compilation
- `rock` support for local source path dependencies by building artifacts before compiling dependents
- product artifact loading through `--extern-artifact`
- artifact-carried generic functions, generic impls, trait defaults, prelude exports, resolver aliases, and object link data
- source loading that is strictly part of artifact production for the crate currently being compiled

## Error Handling

If a dependency is not provided as a product artifact in downstream compilation, the compiler should fail clearly. Preferred failure shape:

- direct `rockc`: report that external dependencies must be passed with `--extern-artifact name=path`
- library/internal misuse: return a structured error where the caller already handles dependency loading failures, or panic only for unreachable internal invariants in tests

Missing artifact body data should identify the dependency and missing body category where practical, for example generic function body, generic impl body, or trait default body.

## Testing

Required coverage:

- Direct `rockc` succeeds with current crate source plus explicit product dependency artifacts.
- Direct `rockc` has no supported source dependency input path.
- `rock` still builds a source path dependency graph by materializing artifacts and then compiling dependents with artifacts only.
- `collect`, `lower`, and `mono` product-artifact regressions cover imported/exported aliases, prelude exports, generic functions, generic impls, trait defaults, and object-backed linkage.
- Tests that only assert source-backed downstream dependency consumption are removed, converted to product-artifact tests, or moved under explicit artifact-production coverage.
- Grep-style checks confirm no downstream phase branches on `is_source_backed()` to read dependency ASTs/file caches.

Useful focused commands after implementation:

```bash
cargo test -p rock-lib crate_artifact
cargo test -p rock-lib products
cargo test -p rockc
cargo test -p rock
```

The final verification should include `cargo test -p rock-lib` plus the relevant `rock`, `rockc`, and `rockup` tests touched by the implementation plan.

## Non-Goals

- Do not remove source parsing for the current crate.
- Do not require users to prebuild artifacts manually when using `rock`; `rock` remains responsible for building source path dependencies into artifacts.
- Do not redesign the product artifact schema unless a missing capability blocks removal of a source fallback.
- Do not merge this cleanup with semantic `Type` to `Ty` migration, MIR-backed codegen, or monomorphization instance identity cleanup.
- Do not add product source bundles as a replacement for source-backed dependency consumption.

## Completion Criteria

This slice is complete when:

- normal downstream compilation cannot consume dependency source through `CrateContext`, `collect`, `lower`, or `mono`
- dependency data consumed by compiler phases comes from product artifact capabilities
- local source dependencies still work through `rock` because each dependency is compiled as the current crate before dependents consume its artifact
- artifact-backed tests become the authoritative external dependency coverage
- the audit checklist's crate/artifact interface split can mark downstream source-backed dependency consumption as removed while leaving broader provider-boundary cleanup in progress
