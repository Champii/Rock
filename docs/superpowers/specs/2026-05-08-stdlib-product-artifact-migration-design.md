# Stdlib Product Artifact Migration Design

**Date:** 2026-05-08
**Status:** Approved for planning
**Scope:** Make product artifacts the normal artifact format, migrate sysroot stdlib packaging to `rockup`, remove the temporary product-artifact CLI split, and remove `rock_lib` as a dependency of `rock` and `rockup`.

## Purpose

`rock` now builds normal package dependencies through dev-target `rockc`, producing product artifacts and consuming them through the temporary `--extern-product-artifact` flag. The bundled stdlib is still the major exception: `rock` packages sysroot stdlib with the old `CrateArtifact` builder, records old artifact metadata, and passes stdlib to `rockc` with old `--extern-artifact` semantics.

The next migration should remove that exception and align the tool boundaries with Rust's model: `rockup` provisions the sysroot, `rockc` consumes it, and `rock` orchestrates package builds without linking the compiler library. Product artifacts should become the only public artifact format for `rockc` inputs, and stdlib should be packaged exactly like any other product artifact.

## Goals

- Make `rockc --extern-artifact name=path` load product artifacts.
- Remove the temporary `--extern-product-artifact` CLI surface.
- Keep `rockc --emit-artifact` producing product artifacts.
- Package bundled sysroot stdlib from `rockup` by invoking `rockc --emit-artifact --emit-object --no-link`.
- Treat sysroot `stdlib.rkca` as a product artifact while keeping its filename/path stable.
- Pass all `rock` package dependencies, including stdlib, through normal `--extern-artifact` product inputs.
- Preserve `no_std`/`no_prelude` behavior when stdlib is loaded only for dependency linkage.
- Remove `rock`'s remaining use of `CrateContext::build_artifact` and old `compile_package_object` paths for stdlib.
- Remove `rock_lib` from `rock` and `rockup` dependencies.
- Introduce a small shared contract crate for manifest and sysroot definitions used by `rock_lib`, `rock`, and `rockup`.

## Non-Goals

- Do not rename `stdlib.rkca`; only change its contents to product artifact bytes.
- Do not add format sniffing or fallback from product artifacts to old `CrateArtifact` behind `--extern-artifact`.
- Do not add compiler-owned stdlib discovery, implicit stdlib injection, or unqualified stdlib loading beyond the existing explicit sysroot/stdlib prelude rules.
- Do not delete every old `CrateArtifact` type and test in this slice if unrelated old-format behavior still needs a separate cleanup. This slice removes stdlib-facing and CLI-facing old artifact behavior.
- Do not preserve `--extern-product-artifact` as an alias.
- Do not put compiler pipeline code, diagnostics rendering, product artifact internals, or LLVM/inkwell-dependent code in the shared contract crate.

## Public CLI Boundary

After this slice:

```text
rockc --extern-artifact dep=build/dep.rkca
```

means `dep.rkca` is a product artifact readable by `CompilerProducts::read_artifact_from_path`.

`rockc --extern-product-artifact` should be removed from the CLI. Tests should assert the flag is rejected so downstream callers cannot rely on the temporary migration surface.

`rockc --emit-artifact path.rkca` keeps its current behavior: it compiles once through `compile_with_products` and writes product artifact bytes. No old `CrateArtifact` is emitted by `rockc`.

## Compiler Library Boundary

`rock_lib::Config` should expose one extern artifact list for product artifacts. The current split between `extern_artifacts` and `extern_product_artifacts` should collapse into the normal field name, with product loading behind it.

The current duplicate-name check should remain, but it no longer needs to compare old/product lists. It should reject duplicate names within the single product artifact list.

The compiler input path should call `CrateContext::load_product_artifact_from_path` for each configured external artifact. Old `load_artifact_from_path` should no longer be reachable through `rockc --extern-artifact`.

## Shared Contract Crate

Add a small crate such as `rock-shared` or `rock-common`. It should be dependency-light and should not depend on `rock-lib`.

The crate should own stable cross-tool contracts:

- `rock.toml` manifest structs and parser
- dependency path resolution primitives
- sysroot layout constants and `SysrootLayout`
- host target triple helper
- toolchain/sysroot metadata structs, if metadata becomes structured in this slice

Feature flags can keep consumers narrow:

- default or `manifest`: manifest data model and parser
- `sysroot`: sysroot constants, layout, host target triple, metadata structs
- `fs`: filesystem helpers such as source discovery, mtime checks, and source fingerprints
- `process`: optional dev-target binary resolution or command helper utilities for `rock`/`rockup`

`rock_lib` should use only the manifest/sysroot contracts it needs. It should not compile `rock`/`rockup` orchestration helpers. `rock` and `rockup` can enable the filesystem/process helpers needed for package orchestration and dev sysroot refresh.

This split preserves one manifest/sysroot definition across the workspace while removing the expensive dependency edge from `rock` and `rockup` to `rock_lib`.

## Sysroot Stdlib Packaging

`rockup` should own stdlib packaging. `rockup/src/dev.rs` should package stdlib by invoking `rockc`, not by loading crates in-process and calling `CrateContext::build_artifact`.

The sysroot packaging command should be equivalent to:

```text
rockc \
  --crate-name stdlib \
  --entry-file <workspace-stdlib-root>/<manifest lib path> \
  --output-dir <sysroot target-libdir> \
  --no-std \
  --no-prelude \
  --no-link \
  --emit-object <sysroot target-libdir>/stdlib.o \
  --emit-artifact <sysroot target-libdir>/stdlib.rkca
```

The object file and artifact should be emitted directly into the sysroot target libdir. That removes the need to build into `stdlib/build/objects` and copy the object afterward.

Freshness should validate:

- `layout.stdlib_artifact` exists
- `layout.stdlib_object` exists
- `manifest.json` exists
- `components.json` exists
- `layout.stdlib_artifact` is a product artifact
- product artifact crate name is `stdlib`
- product artifact link object path points at `layout.stdlib_object` or the canonical sysroot object path
- workspace stdlib source files and `rock.toml` are older than both sysroot outputs when workspace source is available

The sysroot metadata should record product artifact format version, not old `CRATE_ARTIFACT_FORMAT_VERSION`.

`rockup` should expose the dev command as the primary way to refresh stdlib during compiler development, for example:

```text
rockup dev stdlib package --path <workspace>/stdlib --sysroot <workspace>/target --copy-source
```

The exact CLI can stay close to the existing `rockup dev stdlib package` command, but the implementation must compile through `rockc` and product artifacts.

## Development-Only Auto Rebuild

`rock` should not be the long-term owner of sysroot packaging. Installed and explicit sysroots are read-only from `rock`'s perspective. If an explicit sysroot is missing, invalid, or stale, `rock` should report a toolchain/sysroot error and should not rebuild it.

For the implicit workspace sysroot used during compiler development, `rock` may auto-invoke dev-target `rockup` when the bundled workspace stdlib source exists and the sysroot stdlib product artifact is missing or stale. This preserves fast edit/build cycles while keeping production ownership in `rockup`.

The dev auto-rebuild should be constrained:

- only for non-explicit sysroot resolutions that point at the workspace `target/` tree
- only when workspace `stdlib/rock.toml` exists
- only by invoking dev-target `rockup`, not by linking `rock_lib`
- with clear subprocess context such as “dev sysroot stdlib packaging”
- with no fallback to source injection or compiler-owned stdlib registration

If dev-target `rockup` is missing, `rock` should tell the developer to run the equivalent `cargo build -p rockup` or `rockup dev stdlib package` command.

## Rock Package Builds

`rock/src/rockc.rs` command builders should use only `--extern-artifact` for dependency artifacts. The `ExternArtifact` type can remain as the command input model, but separate product-vs-old artifact vectors should be removed.

`rock/src/artifact.rs` should remove the stdlib old-artifact branch. `ensure_artifact` should build every package dependency, including an explicit stdlib package dependency, by invoking `rockc --emit-artifact --emit-object --no-link`. Product dependency graph inspection can still detect when a dependency needs stdlib for linkage, but the injected sysroot stdlib should be passed as a normal product `--extern-artifact`.

`rock/src/build.rs` should pass root dependencies and any required sysroot stdlib as normal product `--extern-artifact` inputs. `no_std` roots must still pass `--no-std --no-prelude`, even if stdlib is loaded only to satisfy dependency linkage.

`rock` package loading should use the shared contract crate for manifests, dependencies, sysroot layout, and freshness helpers. It should not import `rock_lib`.

## Rockup Boundary

`rockup` should use the shared contract crate for manifest parsing, sysroot layout, target triple, metadata, filesystem copying, and source freshness helpers. It should not import `rock_lib`.

`rockup` can find `rockc` using explicit toolchain paths, dev-target sibling resolution, or an argument/env override, depending on the command context. For the dev stdlib packaging path in this slice, resolving dev-target `rockc` from the active Cargo profile directory is acceptable and matches the current `rock` development workflow.

## Old Artifact Code

This slice should remove old artifact behavior from public CLI and `rock` stdlib packaging. Remaining `CrateArtifact` code in `rock-lib` can stay only if it is still covered by internal legacy tests or pending cleanup work. It should not be used by:

- `rockc --extern-artifact`
- `rockc --emit-artifact`
- `rock` dependency package builds
- `rock` root executable builds
- `rockup` sysroot stdlib packaging
- `rock` dev sysroot refresh

Any old stdlib tests that assert `CrateArtifact` behavior should be rewritten to product artifacts or removed if they test deprecated behavior.

## Error Handling

Subprocess failures should preserve compiler diagnostics printed by `rockc` and add orchestration context including:

- package/crate name
- whether the command was dev sysroot stdlib packaging, dependency artifact emission, or root executable build
- missing dev-target `rockc`
- missing dev-target `rockup`
- failed spawn
- non-zero exit status

Invalid sysroot stdlib product artifacts should make `rock` invoke dev-target `rockup` only for implicit workspace sysroots when workspace stdlib source is available. Explicit sysroots should still fail with a clear “does not contain a valid bundled stdlib” error.

## Testing

Required tests:

- `rockc` parse test proving `--extern-artifact` is accepted as the product artifact input.
- `rockc` parse/rejection test proving `--extern-product-artifact` is no longer accepted.
- `rockc` runtime test compiling an app with a dependency passed through `--extern-artifact`, where the dependency artifact is product bytes.
- `rockup` dev stdlib packaging test proving `stdlib.rkca` is product bytes and `stdlib.o` is emitted into the sysroot.
- `rock` sysroot test proving the dev auto-rebuild invokes `rockup` or produces the same product sysroot artifact in the workspace target context.
- `rock` build tests proving stdlib-backed root and dependency packages still run.
- `rock` no-std regression proving a no-std root with a std-using dependency still does not receive unqualified prelude items.
- Freshness test proving changed workspace stdlib source rebuilds the sysroot product artifact.
- Focused `rock-lib` tests proving old artifact paths are not required by current product-based stdlib behavior.
- Dependency checks proving `rock/Cargo.toml` and `rockup/Cargo.toml` no longer depend on `rock-lib`.

Verification commands:

```bash
cargo build -p rockc
cargo build -p rockup
cargo test -p rock
cargo test -p rockup
cargo test -p rockc
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
```

`cargo test -p rock-lib crate_artifact` may shrink in this slice if obsolete old stdlib tests are removed. If it remains, it should pass and only cover intentionally retained legacy artifact behavior.

## Migration Result

After this slice, product artifacts are the public artifact format. `rockup` provisions stdlib as a product artifact, `rockc` consumes one artifact flag, and `rock` orchestrates builds without linking `rock_lib`. `rock` keeps only a development-only auto-refresh path that shells out to `rockup` for the workspace sysroot. The old `CrateArtifact` path becomes internal legacy code awaiting a later deletion slice, not part of normal compiler or package-manager operation.
