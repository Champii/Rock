# Canonical Dependency Prelude Artifact Resolution Design

## Purpose

Implement Phase 2 of the canonical identity completion roadmap: make dependency imports, stdlib prelude aliases, and artifact root exports resolve through canonical `DefId`s instead of treating string paths as semantic identity.

This follows `docs/superpowers/specs/2026-05-11-canonical-identity-completion-design.md`, Phase 2. Phase 1 already remaps product-artifact-local crate IDs into consumer-session `CrateId`s before dependency data reaches compiler phases. Phase 2 uses those remapped IDs as the authoritative identity for dependency-facing lookup.

## Scope

In scope:

- Make artifact root exports carry alias-to-`DefId` metadata, with string names retained as display and import spelling metadata.
- Make stdlib prelude exports carry alias-to-`DefId` metadata, derived from resolver/provider data for the exported item.
- Make explicit dependency imports resolve to canonical dependency resolver IDs before collection/lowering insert semantic HIR declarations.
- Make glob imports from artifact root exports preserve the exported definition's canonical ID.
- Reject migrated dependency/prelude/artifact aliases that cannot resolve to a canonical `DefId`.
- Keep stdlib prelude injection limited to an explicitly loaded `stdlib` artifact for downstream crates, or the current stdlib source while producing stdlib products.

Out of scope:

- Replacing all `functions`, `structs`, `enums`, `traits`, `methods`, and `scope` storage maps with authoritative ID-keyed consumers. That is Phase 3.
- Redesigning child definition identity for methods, associated items, fields, or variants. That is Phase 4.
- Replacing monomorphization method-name instance identity. That is Phase 5.
- Replacing semantic type names with canonical type IDs. That is Phase 6.
- Adding compiler-owned stdlib discovery, implicit stdlib loading, or unqualified stdlib injection beyond the explicit-artifact prelude path.

## Current State

`ResolverTables` already stores canonical `DefId` mappings for item paths, import aliases, and export aliases. Product artifact loading also remaps artifact-local `ProductDefId`s to consumer-session `DefId`s before exposing loaded interface data.

Some active dependency paths still use string-to-string maps as semantic lookup authority:

- `LoadedCrate::prelude_exports` is `alias -> source path`.
- `ArtifactCrateInterface::root_exports` is `alias -> source path`.
- `CollectContext::artifact_root_exports` and `Lowerer::artifact_root_exports` mirror artifact root export strings.
- `CollectContext::stdlib_prelude_exports` and `Lowerer::stdlib_prelude_exports` mirror stdlib prelude export strings.
- Collection and lowering can insert dependency/prelude declaration views after checking string-keyed declaration maps, rather than requiring the selected item to have a resolver-backed `DefId`.

Those string maps are acceptable as syntax, display, and temporary storage views, but they must not be the authority for migrated dependency/prelude/artifact semantic identity.

## Design

### Canonical Export Metadata

Introduce a small ID-backed export metadata shape for dependency-facing aliases. The exact Rust type can be local to the crate system or artifact module, but it must represent:

- alias spelling, such as `answer` or `Deref`
- source/display path, such as `dep::answer` or `stdlib::prelude::Deref`
- canonical `DefId` of the exported item

Artifact root exports and stdlib prelude exports must expose this metadata through provider/resolver paths. Existing string maps may remain during the transition, but only as compatibility views derived from the canonical export metadata.

### Artifact Root Exports

Product artifact loading must validate every root export against the loaded artifact resolver. If an exported source path cannot resolve through `resolver.item_paths`, `resolver.import_aliases`, or `resolver.export_aliases`, loading fails with a clear artifact metadata error.

After validation, artifact root export metadata records the exported item's canonical `DefId`. Glob import from an artifact root uses that ID to populate import alias resolver metadata and declaration views. The source/display path remains available for string-keyed storage maps and diagnostics.

### Stdlib Prelude Exports

Stdlib prelude exports must be normalized to alias-to-canonical-`DefId` metadata after artifact registration, or current-stdlib source collection, has resolver data for the exported item.

Prelude injection during collection and lowering must use the canonical prelude export metadata. If an alias points at a missing item, collection reports a diagnostic instead of silently skipping or falling back to string lookup. Lowering can keep panic-on-missing-canonical-ID for internal invariants, but user-facing malformed dependency/prelude metadata must be caught before that point.

The prelude rule remains explicit: only a loaded dependency named `stdlib` can provide automatic prelude aliases, and only when prelude injection is enabled by config.

### Dependency Imports

Explicit imports from dependencies must resolve through dependency resolver tables. The selected item ID is copied into the current crate's import alias resolver data, or represented in equivalent dependency alias metadata that lowering can consume.

The declaration views inserted for imported dependency functions, structs, enums, traits, and externs must preserve the HIR item's existing canonical `id`. If a dependency import is found only by string-keyed maps but not by resolver/provider identity metadata, collection rejects it for migrated item kinds.

### String-Keyed Storage Boundary

The existing string-keyed maps for functions, structs, enums, traits, methods, and scope may remain in Phase 2 because many consumers still use source spelling or canonical display paths to retrieve declarations. Phase 2 changes the authority of dependency/prelude/artifact resolution, not every downstream storage key.

The boundary is:

- Resolver/provider metadata is authoritative for semantic identity.
- String maps are storage, display, or compatibility views.
- Items inserted through dependency/prelude/artifact paths must already carry canonical IDs.
- Missing canonical IDs in migrated paths are errors, not reasons to allocate new IDs or synthesize identity from names.

## Data Flow

1. Artifact loading builds `ArtifactCrateInterface` and `ResolverTables` with remapped consumer-session `DefId`s.
2. Artifact loading validates root exports and prelude exports against the resolver and records alias-to-`DefId` metadata.
3. Collection registers dependency interfaces, resolver tables, root exports, and prelude aliases through provider metadata.
4. Collection handles explicit imports and glob imports by resolving aliases to canonical IDs before inserting import alias data.
5. Lowering rebuilds scope and declaration views from collected data, using resolver-backed IDs to preserve semantic identity.
6. Existing HIR indexes continue to be rebuilt from canonical names and IDs, while later phases migrate more consumers to direct ID-keyed lookup.

## Error Handling

Malformed product artifacts fail at artifact load when root/prelude export metadata references a source item that cannot resolve to a `DefId`.

Collection reports diagnostics when explicit dependency imports, artifact glob imports, or stdlib prelude aliases lack canonical IDs. This is preferable to lowering panics for user-controlled dependency metadata.

Lowering may still panic for internal invariants such as `def_id_for_name` on already-migrated current-crate paths, because those indicate compiler bugs after collection accepted the program.

## Tests

Focused tests must cover these invariants:

- Artifact root exports resolve to the same `DefId` as their exported definitions.
- Glob imports from artifact root exports preserve the exported definition's `DefId`.
- Stdlib prelude aliases resolve to the canonical `DefId` of the exported item.
- Lowered HIR for dependency imports and prelude aliases uses IDs matching dependency resolver IDs.
- Malformed artifact root export metadata fails when it references a missing source item.
- Malformed stdlib prelude export metadata fails when it references a missing source item.
- Existing artifact-backed dependency, stdlib prelude, cross-crate generic body, trait default, and object-linking regressions still pass.

Final verification for the implementation slice should run:

```bash
cargo fmt --all --check
cargo test -p rock-lib
```

## Success Criteria

- Dependency imports, artifact root exports, and stdlib prelude aliases use resolver/provider `DefId`s as semantic identity.
- String-to-string export/prelude maps are no longer authoritative for migrated dependency-facing lookup.
- No new fallback `DefId` allocation is added for dependency/prelude/artifact resolution.
- Existing supported programs continue compiling through canonical ID-backed paths.
- Later canonical identity phases remain explicitly deferred.
