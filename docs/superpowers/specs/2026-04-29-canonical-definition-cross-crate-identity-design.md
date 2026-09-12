# Canonical Definition And Cross-Crate Identity Design

**Date:** 2026-04-29
**Status:** Draft for review
**Scope:** Make the collect-owned canonical resolver tables authoritative for the current-crate definitions we have already started carrying, then immediately extend the same identity model across dependency crates, stdlib/prelude, and artifact-backed paths.

## Purpose

The previous slice introduced collect-owned canonical path and alias tables, but lowering still rebuilds most identity from legacy string maps. That leaves the compiler with two name systems: the new canonical tables and the old string-keyed bootstrap paths.

This slice removes that split for the migrated current-crate identity paths and then immediately extends canonical identity to the remaining crate sources. The goal is to stop treating canonical identity as advisory.

## Goals

- Make the existing collect-owned resolver tables the source of truth for current-crate canonical definition identity.
- Remove any "canonical lookup, then string fallback" behavior for the migrated identity paths.
- Extend canonical identity to dependency crates, stdlib/prelude, and artifact-backed paths in the immediately following step.
- Preserve current compilation behavior while the legacy string maps are still used for untouched paths.

## Non-Goals

- Do not change parser, syntax, or lowering semantics unrelated to identity lookup.
- Do not remove the legacy string maps until the cross-crate identity expansion is complete.
- Do not add a generic fallback layer between canonical tables and string maps.
- Do not normalize path spelling beyond the compiler's current behavior in this slice.

## Why This Comes Next

The collect-side resolver tables already exist. The next meaningful step is to make them authoritative for the current-crate names they already cover, rather than carrying them as extra metadata. Once that is done, the same machinery should immediately absorb dependency and artifact identity so the remaining string maps can disappear instead of becoming a long-lived compatibility layer.

## Architecture

### Phase 1: Authoritative Current-Crate Canonical Identity

The `ResolverTables` carried by `Declarations` should be the primary identity source for the current crate:

- `module_paths: HashMap<String, ModuleId>`
- `item_paths: HashMap<String, DefId>`
- `import_aliases: HashMap<String, DefId>`
- `export_aliases: HashMap<String, DefId>`

Lowering should consume those tables directly for the migrated names instead of trying canonical resolution first and string resolution second. The string maps may still exist for legacy-only paths, but they are not a fallback for names that are already represented canonically.

### Phase 2: Immediate Cross-Crate Canonical Identity

The same canonical tables should then be populated for the remaining sources of identity:

- dependency crates
- stdlib/prelude exports
- artifact-backed modules and items

This step should extend the canonical tables rather than bolt on another translation layer. The intent is to let lower and artifact paths use the same identity model everywhere, instead of preserving separate canonical and string lookup flows.

### No Fallback Rule

This slice should not add a helper that does:

1. try canonical lookup
2. if missing, consult the string map

That shape makes canonical identity optional and creates the exact long-lived split we are trying to eliminate.

Instead, each consumer should be migrated in one of two ways:

- use canonical tables for the names it now owns
- continue using legacy string maps only for name domains not yet migrated in this branch

## Boundary Rules

- `collect` remains the owner of canonical table construction.
- `lower` becomes the consumer of canonical identity for migrated names.
- dependency/prelude/artifact identity work must follow immediately after the current-crate canonical switch.
- legacy string maps stay only as temporary coverage for non-migrated domains.

## Testing

Add or preserve coverage for:

- canonical module/item paths for current-crate root, inline, and source-backed modules
- current-crate import aliases resolving to canonical `DefId`s
- current-crate export aliases resolving to canonical `DefId`s
- lower behavior for migrated names without relying on legacy string fallback
- dependency and stdlib/prelude resolution once the cross-crate identity phase lands

Verification should include:

- focused collect/lower tests for the migrated identity paths
- `cargo fmt --all`
- `cargo test -p rock-lib`

## Risks

- If collect and lower disagree on canonical path spelling, removing fallback will surface missing identity sooner.
- If cross-crate identity does not land immediately, the remaining legacy string paths can become a second de facto identity system.
- If dependency and artifact identities are only partially canonicalized, lower will become harder to reason about instead of simpler.

## Follow-Up Work

- complete the immediate cross-crate identity expansion
- delete legacy string-path resolution for migrated identity domains
- continue shrinking collect/lower coupling around identity bootstrap
