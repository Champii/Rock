# Sysroot And Rockup Roadmap

## Goal

Move default stdlib provisioning from `rock`'s temporary manifest injection into a real toolchain/sysroot model.

- `rockc` should automatically load bundled stdlib artifacts from the selected toolchain unless stdlib is explicitly disabled.
- `rock` should behave like a lightweight cargo-style wrapper, not like the owner of stdlib semantics.
- `rockup` should install, select, and expose Rock toolchains the same way `rustup` does for Rust.
- Explicit non-stdlib dependencies should remain explicit.
- The current artifact/object pipeline should be reused rather than replaced.

## Why Change Direction

The current state is useful but intentionally temporary:

- `rock` auto-injects `stdlib` into package manifests in memory.
- `rock.toml` now supports `no_std = true` as an opt-out.
- artifact/object support is strong enough to package stdlib as a prebuilt dependency.

This is convenient, but it is still the wrong ownership boundary.

In a Rust-like model:

- stdlib is toolchain-owned, not project-owned
- `cargo` does not inject `std` as a normal package dependency
- `rustc` loads standard libraries from its sysroot
- `rustup` selects which toolchain and sysroot are active

Rock should move to the same shape.

## Desired End State

Normal user flow:

- `rock build` works in a crate with no explicit stdlib dependency.
- `rock run` works the same way.
- `rockc` also works directly with no explicit stdlib argument when a sysroot is available.
- `rock.toml` can disable the bundled stdlib with `no_std = true`.

Compiler/toolchain flow:

- `rockc` loads stdlib from a sysroot as a prebuilt artifact/object pair.
- explicit `--extern-artifact stdlib=...` or `--extern-crate stdlib=...` still override the sysroot for compiler development and testing.
- the selected toolchain determines the stdlib version and target libraries.

Tooling flow:

- `rockup` installs toolchains under a stable home directory.
- `rockup` chooses the active toolchain.
- shims or launcher binaries make `rock` and `rockc` resolve to the selected toolchain automatically.

## Design Principles

1. Keep `rockc` low-level for normal dependencies.
2. Make stdlib a toolchain-owned exception, not a general dependency-discovery system.
3. Use prebuilt stdlib artifacts/objects in the default path.
4. Keep explicit overrides for compiler developers.
5. Prefer simple, monotonic versioning over compatibility shims.
6. Reuse the current artifact interface and object reuse work.

## Non-Goals

- Do not introduce automatic discovery for arbitrary third-party crates.
- Do not make the default sysroot point at stdlib source trees.
- Do not block custom stdlib experiments; explicit overrides should still work.
- Do not split `core`, `alloc`, and `std` immediately.
- Do not make `rockup` a package registry client in the initial version.

Note:

- Initial `no_std` only means "disable bundled stdlib loading".
- It does not yet imply a Rust-like `core`/`alloc` split.

## Proposed Toolchain Layout

```text
$ROCKUP_HOME/
  toolchains/
    stable-x86_64-unknown-linux-gnu/
      bin/
        rock
        rockc
      lib/
        rocklib/
          x86_64-unknown-linux-gnu/
            stdlib.rkca
            stdlib.o
            manifest.json
            components.json
      share/
        rock/
          version.txt
```

Optional future dev components:

```text
      src/
        stdlib/
```

The important rule is that the default sysroot path should be derivable from the running `rockc` binary.

## Sysroot Resolution Rules

Compiler-side resolution order:

1. `rockc --sysroot <path>`
2. `ROCK_SYSROOT`
3. executable-relative sysroot layout

Toolchain selection should mostly live outside the compiler:

- `rockup` decides which toolchain binary is being executed
- once `rockc` is launched, executable-relative sysroot discovery should usually be enough

Useful introspection commands:

- `rockc --print sysroot`
- `rockc --print target-libdir`
- `rockc --print target-triple`

## Stdlib Resolution Rules

Stdlib precedence should be:

1. if stdlib was explicitly disabled, do not load it
2. if user explicitly passed `stdlib` via `--extern-artifact` or `--extern-crate`, use that
3. otherwise load bundled stdlib from sysroot

This keeps the default ergonomic path while preserving explicit dev overrides.

## `no_std` Behavior

Project wrapper behavior:

- `rock.toml` keeps `[crate] no_std = true`
- `rock` reads it and passes the equivalent compiler setting through

Compiler behavior:

- `rockc` needs a direct `--no-std` mode for manifest-less usage
- `no_std` disables automatic sysroot stdlib loading
- `no_prelude` remains separate and only controls prelude injection when stdlib is loaded

Recommended rule set:

- `no_std`: do not auto-load stdlib
- `no_prelude`: stdlib may still be loaded, but prelude names are not injected

## Rockup Responsibilities

Initial responsibilities:

- install toolchains
- list installed toolchains
- set default toolchain
- run commands with a selected toolchain
- expose shims for `rock` and `rockc`

Useful environment variables:

- `ROCKUP_HOME`
- `ROCKUP_TOOLCHAIN`
- `ROCK_SYSROOT`
- `ROCK_TARGET`

Recommended separation:

- `ROCKUP_TOOLCHAIN` selects which toolchain binary to run
- `ROCK_SYSROOT` is a lower-level compiler override for debugging/dev use

## Phased Roadmap

### Phase 1: define the sysroot contract

Status: implemented

Write down the exact compiler-visible sysroot layout and lookup rules.

- finalize directory names under `lib/rocklib/<target>/`
- finalize the names of stdlib artifact/object outputs
- define a tiny metadata file for toolchain version, target, and component presence
- define precedence between `--sysroot`, env vars, explicit stdlib overrides, and `no_std`

Validation:

- a developer can assemble a valid sysroot directory manually
- `rockc --print sysroot` and `rockc --print target-libdir` have a stable contract

### Phase 2: add sysroot support to `rockc`

Status: implemented

Teach the compiler CLI to understand toolchain-owned stdlib inputs.

- add `--sysroot`
- add `--no-std`
- add `--print sysroot`
- add `--print target-libdir`
- add `--print target-triple`
- extend `rock_lib::Config` with the minimal state needed for sysroot stdlib loading

Validation:

- `rockc --print ...` returns the expected paths
- `rockc --entry-file examples/hello.rk` can compile from an assembled sysroot without explicit stdlib flags
- explicit `--extern-artifact stdlib=...` still overrides sysroot stdlib

### Phase 3: package stdlib as a toolchain component

Status: implemented

Build the current `stdlib/` into the shape Phase 2 expects.

- produce `stdlib.rkca`
- produce `stdlib.o`
- record stdlib/toolchain/target metadata together
- ensure artifact version mismatches force rebuild rather than partial reuse

Implementation note:

- this should reuse the current source-free artifact path rather than inventing a second packaging mechanism

Validation:

- a packaged stdlib sysroot can compile and link a hello-world crate
- the packaged stdlib works with prelude injection and object-backed linking

### Phase 4: move implicit stdlib loading from `rock` to `rockc`

Status: implemented

Replace the current wrapper-only manifest injection with compiler-owned sysroot loading.

- remove default stdlib injection from `rock` package loading
- keep `rock.toml no_std = true`
- make `rock` pass `no_std` through to `rockc`
- allow explicit manifest dependency `stdlib = { ... }` to remain as a deliberate override path for development if needed

Validation:

- root crates build without explicit stdlib manifests
- transitive crates build without explicit stdlib manifests
- `no_std` still disables stdlib loading
- direct `rockc` usage works without explicit stdlib flags when a sysroot is present

### Phase 5: simplify `rock` around the sysroot model

Status: implemented

After Phase 4, make the wrapper reflect the new ownership model cleanly.

- keep artifact building/caching behavior
- stop treating stdlib like an ordinary dependency in the default path
- prefer sysroot stdlib when building dependency graphs
- keep explicit stdlib override workflows for compiler development isolated and intentional

Validation:

- `rock build`, `rock run`, and `rock artifact` all work with the sysroot stdlib
- dependency artifact caches remain valid without source-based stdlib injection

### Phase 6: implement minimal `rockup`

Status: implemented

Add a first toolchain manager that can install and select Rock toolchains.

- `rockup toolchain install <name> --path <dir>`
- `rockup toolchain list`
- `rockup default <name>`
- `rockup run <name> -- rock build`
- install shims for `rock` and `rockc`

Recommended initial scope:

- support local file/dir installs first
- support one host target first
- defer remote distribution polish until the toolchain layout is stable

Validation:

- a fresh machine can install a toolchain and build a stdlib-using crate without extra path configuration

### Phase 7: add pinned toolchains and target components

Status: implemented

Bring the workflow closer to `rustup` once the base model is working.

Implemented so far:

- per-project toolchain pinning via `rock-toolchain.toml`
- `rockup` shims resolve toolchains with precedence: `ROCKUP_TOOLCHAIN`, then nearest `rock-toolchain.toml`, then the global default toolchain
- `rockup target add <triple> --path <dir>` installs a target-specific stdlib component bundle into the selected toolchain

Current `rock-toolchain.toml` format:

```toml
[toolchain]
channel = "stable"
```

- target-specific stdlib bundles can be installed separately from host binaries

Validation:

- projects can pin a toolchain
- cross-target stdlib components can be installed independently

### Phase 8: add developer-facing stdlib override workflows

Status: implemented

Keep compiler development flexible without polluting the default user path.

Implemented so far:

- `rockup dev stdlib package --path <stdlib-dir> --sysroot <path>` packages a local stdlib source tree into a standalone sysroot
- `rockup dev stdlib package --copy-source` also installs `src/stdlib/` into that dev sysroot as an optional source component
- the intended dev override flow is now explicit: use `ROCK_SYSROOT` or `rockc --sysroot` for a dev sysroot, or use explicit `--extern-artifact stdlib=...` / `--extern-crate stdlib=...` for narrower experiments
- override precedence is documented and matches the compiler implementation

- support a local rebuilt stdlib sysroot for compiler developers
- optionally ship stdlib source as a separate component
- document explicit override precedence for custom stdlib experiments

Current override precedence:

1. `--no-std` disables bundled stdlib loading entirely
2. explicit `stdlib` via `--extern-artifact stdlib=...` or `--extern-crate stdlib=...` wins
3. otherwise stdlib loads from sysroot resolution order: `--sysroot`, then `ROCK_SYSROOT`, then executable-relative sysroot discovery

Preferred dev path:

- explicit `--extern-artifact stdlib=...`
- explicit `--extern-crate stdlib=...`
- or `ROCK_SYSROOT` pointing at a dev sysroot

Example dev-sysroot workflow:

```bash
rockup dev stdlib package --path ./stdlib --sysroot /tmp/rock-dev-sysroot --copy-source
ROCK_SYSROOT=/tmp/rock-dev-sysroot rock run
```

Validation:

- compiler developers can rebuild stdlib and test changes without changing normal-user toolchain semantics

## Migration Notes

The current wrapper-injected stdlib behavior should be treated as transitional.

Migration sequence:

1. keep the current `rock` injection behavior until `rockc` sysroot loading exists
2. add `rockc` sysroot loading and direct `--no-std`
3. package stdlib into the toolchain layout
4. switch `rock` to passing through `no_std` instead of injecting stdlib
5. remove the default injection path once sysroot builds are stable

This avoids a user-facing regression while the toolchain model is still incomplete.

## Open Questions

1. Should explicit manifest dependency `stdlib = { ... }` remain supported long-term as an override, or only as a temporary compiler-dev escape hatch?
2. Should `rockc` get only `--no-std`, or should the language later gain a source-level crate attribute too?
3. Do we want `stdlib.o` directly, or a target library archive like `libstdlib.a`?
4. Should target metadata live in JSON, TOML, or be folded into the artifact metadata only?
5. When we eventually split `core`/`alloc`/`std`, do we keep the same sysroot layout with additional bundled crates?

## Recommended Next Step

Implement Phase 8 next.

That means:

- make developer stdlib override workflows explicit and documented around sysroots/toolchains

Once that exists, Rock will have both a stable user-facing toolchain flow and a clean compiler-developer override path.
