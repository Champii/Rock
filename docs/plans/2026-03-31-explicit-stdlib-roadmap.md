# Explicit Stdlib Roadmap

## Goal

Make the compiler treat `stdlib` like any other external crate.

- Remove deprecated paths, backward-compatibility shims, and transitional aliases.
- Stop compiler-owned stdlib discovery and implicit loading.
- Keep dependency source paths until a proper crate artifact interface exists.
- Keep automatic prelude injection as the only stdlib-specific compiler behavior, and only when `stdlib` was explicitly passed to `rockc`.

## Phase 1

Status: completed

Remove implicit stdlib loading and stdlib-only crate state.

- Remove `STDLIB_CACHE` and the stdlib-only fast path in `lib/src/lib.rs`.
- Remove `CrateContext::load_stdlib()`.
- Remove sysroot discovery and `ROCK_SYSROOT` handling from `lib/src/crate_system/mod.rs`.
- Remove `CrateContext::stdlib_loaded` and `mark_stdlib_loaded()`.

## Phase 2

Status: completed

Make `stdlib` explicit and remove `no_std` as a user-facing mode.

- Keep dependency source paths for now via the existing `--extern-crate name=path` flow.
- Remove `no_std` from `rockc` and `rock_lib::Config`.
- Treat absence of `stdlib` as simply not having the crate loaded.
- Keep `no_prelude`.

## Phase 3

Status: completed

Restrict stdlib-specific behavior to prelude injection only.

- Keep detection of a loaded crate named `stdlib` only for prelude handling.
- Keep capture of `stdlib::prelude` exports.
- Gate prelude injection on both `stdlib` being loaded and `!no_prelude`.
- Remove other stdlib convenience behavior, especially unqualified stdlib name registration outside the prelude path.

## Phase 4

Status: completed

Delete compiler-owned stdlib registration and fallback compatibility paths.

- Remove `register_stdlib()` and its callers.
- Remove the backward-compatibility fallback branch in lowering and collection.
- Let remaining compiler assumptions fail loudly so they can be cleaned up directly.

## Phase 5

Status: completed

Remove compatibility aliases and obsolete API shapes.

- Replace `CompileError` with `ResolveError`.
- Replace MIR `Statement` alias usage with `StatementData`.
- Remove stdlib-loading compatibility-shaped APIs.

## Phase 6

Status: completed

Audit hidden stdlib API assumptions in MIR, codegen, and closures.

- Review `lib/src/mir/builder.rs` builtin handling.
- Review `lib/src/codegen/mod.rs` runtime declarations.
- Review `lib/src/codegen/closures.rs` builtin exclusions.
- Separate true compiler intrinsics from stdlib APIs that are currently treated as compiler-owned.

## Phase 7

Status: completed

Update tests and docs for explicit stdlib dependency paths.

- Update tests that currently rely on implicit stdlib loading.
- Add the simplest shared helper for explicit stdlib setup where it removes repetition.
- Update `rockc` docs/examples to show explicit stdlib passing.

## Phase 8

Status: completed

Later, replace stdlib-only caching with generic crate artifacts.

- Generalize the current stdlib cache and per-crate parsed-module cache into a uniform crate caching story.
- Introduce a serialized crate artifact format instead of passing dependency source trees to `rockc`.
- Make `rock` responsible for producing and passing those artifacts in a later session.
- Keep `rockc` low-level: it should consume explicit dependency inputs, first as source paths, later as crate artifacts.
- Design captured in `docs/plans/2026-03-31-crate-artifact-design.md`.

## Artifact Direction

Do not serialize current compiler state wholesale.

Introduce a dedicated crate artifact carrying only downstream-needed data:

- crate identity and manifest data
- exported declarations and interface summary
- module/export lookup data
- prelude export metadata for `stdlib`
- optional object path for linking

This becomes the basis for crate-level incremental compilation.
