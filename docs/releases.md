# Release Maintainer Guide

This guide defines the Linux release contract for `Champii/Rock`. Releases use tags named `vVERSION`. Only `x86_64-unknown-linux-gnu` is supported, built on Ubuntu 24.04 (glibc 2.39 baseline) with dynamically linked LLVM 18. Users need LLVM 18 shared libraries and a C linker; archives do not bundle the operating-system runtime.

Historical GitHub releases exist, but the new rockup asset format has not yet been published. A read-only GitHub API check on 2026-09-12 found historical releases with a standalone `rock` asset, not the assets below. Do not advertise the new bootstrap as usable until a compatible release is publicly available. `v0.1.0` below is illustrative; choose an unused tag appropriate for the project.

## Asset Contract

For `VERSION=0.1.0` and `TARGET=x86_64-unknown-linux-gnu`, attach:

| Asset | Contents or purpose |
| --- | --- |
| `rock-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` | Complete toolchain, extracted directly into its toolchain root |
| `rockup-x86_64-unknown-linux-gnu` | Standalone executable, not an archive |
| `stdlib-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` | Matching standard-library component archive |
| Each binary/archive name followed by `.sha256` | SHA-256 sidecar for that exact asset |
| `install.sh` | POSIX bootstrap from `scripts/install.sh` |

The complete toolchain contains `bin/rock`, `bin/rockc`, `bin/rock-lsp`, the matching standard-library artifacts and component metadata under `lib/rocklib/TARGET/`, and standard-library sources under `src/stdlib/`. Keep the compiler, object files, serialized artifacts, and metadata from the same build. Do not add an enclosing version directory inside the archive.

Generate sidecars from the asset directory using `sha256sum ASSET > ASSET.sha256`. Each sidecar must contain exactly one checksum line naming the asset's basename, not a local path. The bootstrap accepts the standard text or binary sha256sum separator; unrelated filenames, extra lines, and mismatched hashes are rejected. These are integrity checks, not signed provenance.

The public bootstrap URL is:

```text
https://github.com/Champii/Rock/releases/latest/download/install.sh
```

Stable standalone-manager downloads use `releases/latest/download/rockup-TARGET` and its `.sha256` sidecar. Pinned downloads use `releases/download/vVERSION/rockup-TARGET` and its sidecar. Rockup resolves stable to the latest non-prerelease tag and downloads the matching `rock-vVERSION-TARGET.tar.gz`. Keep all required assets together before publishing.

## Local Packaging

The maintainer entry point is:

```sh
bash scripts/release.sh v0.1.0
```

Its contract is to build, package, and smoke-test locally, writing output to `dist/v0.1.0`. This default mode must not create tags, upload assets, or publish releases. Use Ubuntu 24.04 x86_64 with Rust/Cargo, LLVM 18 development tools and shared libraries, a C toolchain, GNU tar, gzip, curl, and sha256sum. A build on a newer distribution can accidentally raise the glibc baseline; use the baseline environment for distributable artifacts.

Before tagging, set the package versions in `rock/Cargo.toml`, `rockc/Cargo.toml`, `rockup/Cargo.toml`, and `rock-lsp/Cargo.toml` to the release version and refresh `Cargo.lock`; the script rejects mismatched versions. Existing output directories are never overwritten. The separate stdlib archive extracts directly to a target component directory and includes its manifests, artifact, and object file.

Before accepting the output, inspect archive layouts, verify every sidecar from the output directory, and smoke-test with a disposable `HOME` and `ROCKUP_HOME`. Check `rock --version`, `rock-lsp --help`, and compilation/execution of a small application with the packaged stdlib. Test the bootstrap with temporary downloads removed afterward: `ensure_shims` must persist the downloaded manager at `ROCKUP_HOME/bin/rockup`, and all three shims must continue to work after the bootstrap staging directory disappears. A shim pointing into the build or download staging directory is a release blocker.

Run the bootstrap checks without network access:

```sh
sh -n scripts/install.sh
sh scripts/tests/install.sh
bash scripts/tests/release.sh dist/v0.1.0
```

The final check uses the real packaged binaries and archives with an offline HTTP fixture to exercise bootstrap cleanup, stable updates, pinned installation, self-update, shims, and compilation. The release workflow runs it before creating a draft.

## Draft Creation

Uploading is explicitly opt-in:

```sh
bash scripts/release.sh v0.1.0 --publish
```

Despite the option name, this creates a **draft only**, using authenticated `gh`. It requires the existing tag to point at local `HEAD` and to exist on GitHub; it must not create or move a tag. Arrange the reviewed commit and remote tag separately through the project's normal maintainer process. Do not use this option merely to test packaging.

The tag workflow for `v*` runs on Ubuntu 24.04, tests and packages the toolchain, and creates a draft release. It must not automatically make a public release. Coordinate the workflow and local `--publish` path rather than trying to create the same draft twice. Verify the implemented script/workflow and their logs before relying on this contract; documentation alone is not evidence of a successful packaging run.

## Review and Publish

1. Confirm the draft tag identifies the reviewed commit and the build used the supported baseline.
2. Review test and smoke-test logs, release notes, platform requirements, and known limitations.
3. Check that every required asset and exact checksum sidecar is attached and that no private files or build paths leaked into archives.
4. Download draft assets with authenticated maintainer access and verify them; anonymous bootstrap URLs cannot install a draft.
5. Manually publish the reviewed draft in GitHub. Mark actual prereleases as prereleases; stable must mean the latest non-prerelease release.
6. For the first new-format stable release, ensure GitHub marks it as Latest rather than leaving a historical release selected, then test the anonymous `latest/download/install.sh` flow and a pinned install in disposable homes.

No local packaging command or tag workflow should silently publish publicly. Do not claim Windows, macOS, nightly, automatic cross-target support, or Rustup feature parity. If the public smoke test fails, communicate the problem and correct the release deliberately rather than telling users to skip checksum checks.
