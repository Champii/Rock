# Old CrateArtifact Deletion Design

**Date:** 2026-05-08
**Status:** Draft for review
**Scope:** Remove the old `CrateArtifact` system from `rock-lib` after first adapting useful legacy coverage to product artifacts.

## Purpose

The stdlib product artifact migration made product artifacts the public and current compiler artifact format. `rockc`, `rock`, and `rockup` now build and consume product artifacts, and the old `CrateArtifact` path is no longer part of normal compiler or package-manager operation.

The remaining old artifact implementation now mostly supports legacy tests. Keeping it creates misleading coverage and preserves unused builder/loader paths. This slice deletes the old system, but first ports tests that still describe real product-artifact behavior so coverage is not lost during deletion.

## Goals

- Add product-artifact tests for adaptable legacy behavior before deleting any old artifact code.
- Remove old `CrateArtifact` data structures, serialization, builder, loader, helpers, and old-only tests.
- Keep shared interface data structures that product artifact loading still uses, such as `ArtifactCrateInterface`, `ArtifactModuleSummary`, and `ArtifactCrossCrateHir`. A small rename is allowed in this slice only if it directly clarifies that these are product-loading interfaces rather than old artifact containers.
- Keep `rockc --extern-artifact name=path` as the only public dependency artifact input, backed by product artifacts.
- Keep `CompilerProducts` and `ProductArtifact` as the only artifact file format emitted or consumed by current tooling.
- Preserve useful test coverage for exports, prelude behavior, associated types, cross-crate generics, trait defaults, object-backed linking, and product relocation where product artifacts support those behaviors.

## Non-Goals

- Do not preserve old `CrateArtifact` compatibility, format sniffing, migration readers, or aliases.
- Do not add source-bundle product artifacts. Source bundles are part of the deleted old artifact system, not the product artifact model.
- Do not reintroduce `rock_lib` dependencies into `rock` or `rockup`.
- Do not broaden this slice into unrelated product artifact redesigns or schema changes beyond what deletion requires.

## Two-Phase Structure

### Phase 1: Adapt Coverage First

Before deleting old code, classify legacy tests into three buckets:

- **Adapt:** tests that assert behavior product artifacts should preserve.
- **Delete:** tests that assert old-only mechanisms.
- **Already Covered:** tests that already have product equivalents from the previous slice.

Adaptable coverage should be rewritten as product artifact tests while the old tests still exist. This keeps failures attributable to missing product behavior instead of deletion fallout.

Product equivalents should use `compile_with_products`, `CompilerProducts::write_artifact_to_path`, `CrateContext::load_product_artifact_from_path`, or `rockc --emit-artifact --emit-object --no-link` depending on the behavior under test. Prefer actual compile/run tests when the old test was proving user-visible linking behavior.

### Phase 2: Delete Old System

After product coverage is green, delete old-only code and tests in a separate commit. The deletion commit should remove dead implementation rather than hiding it behind `#[cfg(test)]` or crate-private visibility.

Deletion targets include:

- `CrateArtifact`
- old artifact identity/fingerprint/source-bundle/object-output structs that are not shared with product loading
- `CRATE_ARTIFACT_FORMAT_VERSION`
- `CrateContext::build_artifact` and `build_artifacts`
- old `ArtifactBuilder` and helper functions used only by the old builder
- `CrateContext::load_artifact_from_path` and `load_artifact_from_path_with_root`
- tests named `legacy_old_artifact_*`
- tests whose only assertion is old source-bundle, old interface-only, old source-backed, or old serialization behavior

## Test Adaptation Policy

Adapt when possible and sensible.

Adapt tests that validate current compiler behavior through artifacts:

- stdlib exports and prelude exports
- string/deref ABI and method behavior through artifact-backed stdlib
- associated types surviving artifact boundaries
- extern declarations using resolved IDs
- generic functions and generic impls across artifact boundaries
- trait default methods needed by downstream generic impls
- object-backed linking and source-free dependency usage
- dependency identity when product artifacts expose the equivalent information

Delete tests that only validate removed implementation details:

- old artifact serialization round trips
- old source bundles and file caches
- old artifact modes: source-backed, object-backed, interface-only
- old `CrateContext::build_artifact` output shape
- root-dir override behavior for old artifacts
- manually mutating old artifact internals to simulate missing bodies or stale objects

If a legacy test mixes useful behavior with old-only setup, split the useful assertion into a product test and delete the old setup.

## Expected Architecture After Deletion

`lib/src/crate_artifact/` should no longer contain an old artifact format. It may either be reduced to product-loading adapters and shared interface structs, or product loading can move closer to `lib/src/products.rs` if that keeps boundaries clearer.

`CrateContext` should only expose product artifact loading for artifact-backed dependencies. Source-backed crate loading remains through normal manifest/source loading, not artifact source bundles.

Product artifact loading should keep converting `CompilerProducts` into the existing `LoadedCrate` shape until a separate compiler-internal representation cleanup is designed. This slice removes the old file format, not every internal compatibility name.

## Error Handling

Removal should not degrade current product errors. Product artifact loading should continue to report:

- invalid product artifact bytes
- format version mismatch
- missing object path
- missing object file
- `--extern-artifact` name mismatch

Old artifact files should fail as invalid product artifacts if passed to `rockc --extern-artifact`. No old-format fallback should remain.

## Testing

Required focused tests before deletion:

- Product artifact test for associated types across an artifact boundary. If existing product coverage already proves this, explicitly identify that test during implementation and do not add a duplicate.
- Product artifact test for cross-crate generic function use.
- Product artifact test for cross-crate generic impl use.
- Product artifact test for trait default behavior needed by downstream generics.
- Product-backed stdlib/string/deref behavior where the old tests covered ABI regressions.
- Product artifact failure test proving an old artifact file is not accepted, if an old fixture can be produced before deletion.

Required deletion checks:

- Grep for `CrateArtifact`, `CRATE_ARTIFACT_FORMAT_VERSION`, `load_artifact_from_path`, `build_artifact`, `legacy_old_artifact`, `ArtifactSourceBundle`, and `ArtifactObjectOutput`.
- Remaining matches must be limited to shared product-loading names such as `ArtifactCrateInterface`, `ArtifactModuleSummary`, or `ArtifactCrossCrateHir`; all old format/container names must be gone.
- `rock`, `rockup`, and `rockc` must still avoid old artifact APIs.

Verification commands:

```bash
cargo fmt --all --check
cargo test -p rockc -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock -- --nocapture
cargo build -p rockc && cargo test -p rockup -- --nocapture
cargo tree -p rock --edges normal
cargo tree -p rockup --edges normal
```

## Migration Result

After this slice, the compiler workspace has one artifact file format: product artifacts. Old `CrateArtifact` files are not emitted, loaded, tested, or represented as a compatibility path. Product artifacts retain the behavior coverage that still matters, and obsolete source-bundle/interface-only/old-serialization tests are removed with the implementation they described.
