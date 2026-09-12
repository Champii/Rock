# Release Maintainer Guide

This guide defines the Linux release contract for `Champii/Rock`. Releases use tags named `vVERSION`. Only `x86_64-unknown-linux-gnu` is supported, built on Ubuntu 24.04 (glibc 2.39 baseline) with statically linked LLVM 18. End users do not need to install LLVM, but still need a C linker, curl, and CA certificates (`build-essential curl ca-certificates` on Ubuntu 24.04), plus GNU tar, gzip, and `sha256sum`. These are not fully static executables: system-library dependencies remain, and archives do not bundle the operating-system runtime.

The rockup asset format starts with `v0.5.0`. Earlier releases provide historical standalone assets and are not supported by the bootstrap. This guide uses `v0.5.1` as its example; choose a new, unused tag when preparing subsequent releases.

## Asset Contract

For `VERSION=0.5.1` and `TARGET=x86_64-unknown-linux-gnu`, attach:

| Asset | Contents or purpose |
| --- | --- |
| `rock-v0.5.1-x86_64-unknown-linux-gnu.tar.gz` | Complete toolchain, extracted directly into its toolchain root |
| `rockup-x86_64-unknown-linux-gnu` | Standalone executable, not an archive |
| `stdlib-v0.5.1-x86_64-unknown-linux-gnu.tar.gz` | Matching standard-library component archive |
| Each binary/archive name followed by `.sha256` | SHA-256 sidecar for that exact asset |
| `install.sh` | POSIX bootstrap from `scripts/install.sh` |

The complete toolchain contains `bin/rock`, `bin/rockc`, `bin/rock-lsp`, the matching standard-library artifacts and component metadata under `lib/rocklib/TARGET/`, standard-library sources under `src/stdlib/`, and LLVM license notices under `share/licenses/llvm/`. Keep the compiler, object files, serialized artifacts, and metadata from the same build. Do not add an enclosing version directory inside the archive.

Generate sidecars from the asset directory using `sha256sum ASSET > ASSET.sha256`. Each sidecar must contain exactly one checksum line naming the asset's basename, not a local path. The bootstrap accepts the standard text or binary sha256sum separator; unrelated filenames, extra lines, and mismatched hashes are rejected. These are integrity checks, not signed provenance.

The public bootstrap URL is:

```text
https://github.com/Champii/Rock/releases/latest/download/install.sh
```

Stable standalone-manager downloads use `releases/latest/download/rockup-TARGET` and its `.sha256` sidecar. Pinned downloads use `releases/download/vVERSION/rockup-TARGET` and its sidecar. The bootstrap verifies the standalone manager and runs `rockup self install`, which copies only the manager and command shims and adds shell setup. It does not download a toolchain; an optional `vVERSION` script argument pins only the manager. After restarting the shell, users run `rockup install` separately to resolve stable to the latest non-prerelease tag and download the matching `rock-vVERSION-TARGET.tar.gz`, or `rockup install vVERSION` to pin the toolchain. Keep all required assets together before publishing.

## Local Packaging

The maintainer entry point is:

```sh
bash scripts/release.sh v0.5.1
```

Its contract is to build, package, and smoke-test locally, writing output to `dist/v0.5.1`. This default mode must not create tags, upload assets, or publish releases. Use Ubuntu 24.04 x86_64 with Rust/Cargo, LLVM 18 development files and static archives, a C toolchain, GNU tar, gzip, curl, CA certificates, and sha256sum. A build on a newer distribution can accidentally raise the glibc baseline; use the baseline environment for distributable artifacts.

Install the source-build dependencies and select LLVM 18 before packaging:

```sh
sudo apt install llvm-18-dev libpolly-18-dev libzstd-dev libxml2-dev zlib1g-dev libffi-dev libedit-dev libncurses-dev build-essential
export LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
```

Static LLVM archives are mandatory; there is no dynamic-linking fallback. The new packaging contract requires rejecting shared LLVM dependencies via `ldd` and including LLVM license notices under `share/licenses/llvm/`.

Before tagging, set the package versions in `rock/Cargo.toml`, `rockc/Cargo.toml`, `rockup/Cargo.toml`, and `rock-lsp/Cargo.toml` to the release version and refresh `Cargo.lock`; the script rejects mismatched versions. Existing output directories are never overwritten. The separate stdlib archive extracts directly to a target component directory and includes its manifests, artifact, and object file.

Before accepting the output, inspect archive layouts, verify every sidecar from the output directory, and smoke-test with a disposable `HOME` and `ROCKUP_HOME`. Test that the bootstrap installs only the manager, shims, and shell setup, with no toolchain download. Remove temporary downloads afterward: `rockup self install` must persist the downloaded manager at `ROCKUP_HOME/bin/rockup`. Activate the shell and run `rockup install` separately, then check `rock --version`, `rock-lsp --help`, and compilation/execution of a small application with the packaged stdlib. All three shims must work after toolchain installation even though the bootstrap staging directory is gone. A shim pointing into the build or download staging directory is a release blocker.

Run the bootstrap checks without network access:

```sh
sh -n scripts/install.sh
sh scripts/tests/install.sh
bash scripts/tests/release.sh dist/v0.5.1
```

The final check uses the real packaged binaries and archives with an offline HTTP fixture to exercise bootstrap cleanup, stable updates, pinned installation, self-update, shims, and compilation. The new CI contract requires running package smoke tests before draft creation in a clean Ubuntu 24.04 container with the runtime packages listed above and no LLVM installation. A successful build on an LLVM-equipped runner alone does not validate the runtime contract.

## Draft Creation

Uploading is explicitly opt-in:

```sh
bash scripts/release.sh v0.5.1 --publish
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
