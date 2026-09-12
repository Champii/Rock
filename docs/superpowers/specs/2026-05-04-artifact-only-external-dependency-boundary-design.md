# Artifact-Only External Dependency Boundary Design

**Date:** 2026-05-04
**Status:** Draft for review
**Scope:** Define the supported external dependency boundary between `rock`, `rockc`, and crate artifacts, and stop treating source-backed external crates as a supported compiler input model.

## Purpose

The compiler currently carries two external dependency models at once:

- artifact-backed dependencies through `--extern-artifact`
- source-backed external crates through `--extern-crate` and `CrateContext::load_crate_from_dir(...)`

The artifact path is already the intended direction. `rock build` recursively materializes dependency artifacts and compiles the root crate with artifact inputs, while `rockc` and parts of `lib/` still preserve the older source-backed external dependency path.

That split is now actively costing time in `collect`, `lower`, and `mono`. The goal of this design is to make the supported boundary explicit:

- `rockc` compiles one current crate from source
- external dependencies are consumed only as artifacts
- `rock` owns dependency resolution, recursive dependency builds, and artifact provisioning

## Goals

- Make `rockc` artifact-only for external dependencies.
- Remove `--extern-crate` from the supported compiler interface.
- Keep direct `rockc` usage explicit: no implicit dependency discovery and no implicit stdlib artifact lookup.
- Treat `stdlib` like any other external crate artifact for downstream compilation.
- Keep source path dependencies as a `rock` concern only, by building them into artifacts before downstream compilation.
- Remove ongoing maintenance pressure from source-backed external dependency consumption inside `lib/`.
- Preserve the current crate as a source input to `rockc`.
- Leave a clean follow-up path for `rock` to shell out to `rockc` once the artifact-only boundary is stable.

## Non-Goals

- Do not remove source parsing for the crate currently being compiled.
- Do not remove source-backed crate loading from artifact-production workflows.
- Do not make `rockc` discover dependencies automatically.
- Do not make `rockc` locate bundled stdlib artifacts implicitly.
- Do not combine this boundary cleanup with the later in-process to subprocess `rock -> rockc` integration step.
- Do not preserve long-term compatibility shims for source-backed external dependency consumption.
- Do not redesign the artifact schema unless a removal step proves the current interface or cross-crate HIR bundle is insufficient.

## Current State

The repository already contains most of the desired structure.

- `rock/src/build.rs` builds the current package with `extern_artifacts` only.
- `rock/src/artifact.rs` recursively builds path dependency artifacts and reuses cached outputs.
- `docs/plans/2026-03-31-crate-artifact-design.md` already describes artifact consumption as the future compiler boundary.
- `rockc/src/main.rs` still accepts both `--extern-crate` and `--extern-artifact`.
- `lib/src/lib.rs` still loads both source-backed external crates and artifact-backed external crates into `CrateContext`.
- `lib/src/lower/crates/registration.rs`, `lib/src/lower/crates/bodies.rs`, and `lib/src/mono/external.rs` still preserve source-backed external dependency consumption paths.

This means the build tool is already moving toward a Cargo/rustc split, but the compiler boundary is not yet strict enough to remove the old model.

## Target Boundary

### `rockc`

`rockc` is the low-level compiler.

It should:

- compile exactly one current crate from source
- accept external dependencies only through explicit artifact inputs
- never resolve or parse dependency source trees as part of downstream compilation
- never auto-discover stdlib or other dependency artifacts

The supported direct interface becomes:

- current crate source
- explicit `--extern-artifact name=path` inputs
- normal compiler flags for output, optimization, debug printing, and sysroot selection

### `rock`

`rock` is the package manager and build orchestrator.

It should:

- resolve the package graph
- support local source path dependencies
- recursively build or reuse dependency artifacts
- ensure stdlib artifacts exist when the package graph requires stdlib
- invoke the compiler for the root crate using explicit artifact inputs only

This keeps local development ergonomic without preserving dependency-source loading inside the compiler.

### Source-Backed Crates

Source-backed crates remain valid only on the artifact-production side.

The rule is:

- source-backed crates are for building artifacts
- artifacts are for consuming dependencies

That rule applies equally to normal dependencies and `stdlib`.

## Stdlib

`stdlib` should follow the same downstream boundary as every other external crate.

- `rockc` should consume `stdlib` only through an explicit `--extern-artifact stdlib=...` input.
- `rockc` should not load `stdlib` source as an external dependency.
- `rock` should ensure the bundled stdlib artifact and object file exist when needed, then pass that artifact explicitly.
- `--no-std` should mean that `rock` does not provide the stdlib artifact.

The only compiler-side stdlib-specific behavior that should remain is conditional prelude injection, and only when an explicitly loaded dependency named `stdlib` is present.

## Path Dependency Policy

Two end states were considered:

- require prebuilt artifacts everywhere, even in `rock`
- let `rock` accept source path dependencies and build them into artifacts before compiling dependents

The recommended model is the second one.

It preserves a normal local development workflow while keeping `rockc` honest about its boundary. Path dependencies remain a package-manager concern, not a compiler concern.

## Compiler Contract Changes

### CLI

`rockc` should move to this external dependency contract:

- remove `--extern-crate`
- keep `--extern-artifact`
- require explicit `--extern-artifact stdlib=...` for stdlib-backed direct compiler use

`--crate-path` is not part of the final external dependency model. Its removal or later redefinition as an artifact-search convenience is a separate follow-up decision and is out of scope for this design.

### `rock_lib::compile`

The supported compile path should become:

- parse current crate source
- load external dependency artifacts into `CrateContext`
- collect/lower/infer/mono/codegen against artifact-backed dependency data only

Normal downstream compilation should no longer load external dependency source directories.

## Internal Cleanup Targets

The following source-backed external dependency paths should become removal targets.

### `rockc/src/main.rs`

- remove CLI support for `--extern-crate`

### `lib/src/lib.rs`

- remove external dependency loading through `config.extern_crates`
- keep artifact loading through `config.extern_artifacts`
- keep source-backed crate loading only where it is needed to build an artifact for the current crate

### `lib/src/lower/crates/registration.rs`

- remove the legacy source-backed dependency registration branch
- rely on artifact interface data, resolver tables, module indexes, and object-backed metadata only

### `lib/src/lower/crates/bodies.rs`

- remove external dependency source-module and source-body lowering paths
- rely on artifact-carried cross-crate generic bodies and trait default bodies only

### `lib/src/mono/external.rs`

- remove dependency AST and module-cache traversal for external crates
- rely on artifact-backed resolver tables, interface data, and cross-crate HIR only

### `lib/src/crate_system/context.rs`

- keep `load_crate_from_dir(...)` only for artifact-production workflows and related tooling
- stop treating it as part of the supported external input model for downstream compilation

## Migration Plan

### Step 1: Freeze the legacy boundary

Stop investing in source-backed external dependency consumption as an actively supported downstream compiler path.

Bug fixes in that area should only be taken if they are required for artifact production itself, or if they unblock removal work. They should not be treated as long-term compiler maintenance.

### Step 2: Make `rockc` artifact-only for externals

- remove `--extern-crate`
- keep `--extern-artifact`
- require explicit stdlib artifacts for direct `rockc` use
- fail clearly when an external dependency is only available as source

This is the near-term hard cutoff for the compiler boundary.

### Step 3: Narrow `lib/` to the supported model

Remove source-backed external dependency consumption from collect/lower/mono integration paths so the supported dependency model becomes unambiguous.

At the end of this step, downstream compilation should operate only on:

- artifact interface data
- dependency resolver tables
- module export indexes
- cross-crate HIR bundles
- dependency object outputs

### Step 4: Keep `rock` as the source-facing tool

`rock` continues to accept source path dependencies, but only as inputs to artifact production.

The root crate build becomes conceptually equivalent to:

- build or reuse dependency artifacts recursively
- invoke compiler for the current crate with explicit artifact inputs only

### Step 5: Split process boundaries later

Once `rock` already behaves like a driver over the artifact-only compiler contract, switching from in-process `rock_lib` calls to subprocess `rockc` invocations becomes mostly an operational change.

That later step should not be coupled to the current boundary cleanup.

## Why The Process Split Comes Later

Two changes are easy to conflate but should stay separate:

- removing source-backed external dependency consumption from the compiler
- changing `rock` from calling `rock_lib` directly to spawning `rockc`

The first is a compiler architecture simplification. The second is a driver/process integration change.

Doing both at once would mix compiler cleanup with:

- CLI contract transitions
- diagnostics/process propagation
- invocation plumbing
- cache and sysroot orchestration details

The recommended order is to make the compiler contract strict first, then move the driver boundary afterward.

## Risks

- Some existing tests likely still assume source-backed external dependency consumption and will need to be rewritten or removed.
- Artifact-carried cross-crate generics and trait default bodies must remain sufficient before removing legacy source-backed fallback paths.
- Direct `rockc` users will need to pass explicit stdlib and dependency artifacts, which is stricter than current behavior.
- There may be a transition period where `rock` is the only ergonomic entrypoint for multi-crate builds.

## Validation

The migration is successful when the following are true:

- `rock build` still works for source path dependency graphs by recursively building artifacts first.
- direct `rockc` compilation works with only current crate source plus explicit `--extern-artifact` inputs.
- direct `rockc` compilation fails clearly when a dependency is provided only as source.
- stdlib-backed builds work only when `stdlib` is passed explicitly as an artifact.
- artifact-backed compile/runtime/generic tests become the authoritative external dependency coverage.
- tests that only prove source-backed external dependency consumption are removed, rewritten, or moved under artifact-production coverage.

## Rejected Alternatives

### Keep long-term coexistence

Keeping both source-backed and artifact-backed external dependency models inside the compiler would preserve the exact maintenance burden this design is trying to remove.

### Require prebuilt artifacts everywhere

Forcing even `rock` to accept only prebuilt artifacts would make local development worse without improving the compiler boundary. The useful simplification is to remove source-backed external dependencies from `rockc`, not from the package manager.

### Do the subprocess split immediately

Making `rock` shell out to `rockc` in the same change would expand the migration surface without making the dependency boundary any clearer. The process split should happen after the compiler contract is already artifact-only.

## Resulting Rule

The supported architecture after this migration is:

- `rockc` is a compiler, not a dependency loader
- `rock` is the dependency loader, artifact builder, and build orchestrator

That boundary matches the repository's existing direction and gives a clean basis for the later Cargo/rustc-style split.
