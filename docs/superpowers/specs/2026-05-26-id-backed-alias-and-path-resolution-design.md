# ID-Backed Alias And Path Resolution Design

**Date:** 2026-05-26
**Status:** Approved for implementation planning
**Roadmap tasks:** Task 4, `Replace Persistent String Alias Interfaces With ID-Backed Aliases`; strict Task 5 subset, `Split Source/Path Name Resolution Out Of Lowerer`

## Purpose

Complete roadmap Task 4 and the alias/path-resolution-owned subset of Task 5 by removing string-to-string alias maps as semantic compiler interfaces. Import aliases, export aliases, prelude exports, module-local aliases, and artifact root exports must persist and resolve through canonical IDs. Source strings remain only as diagnostics, display, source reconstruction, or compatibility metadata where they do not define semantic ownership.

This design intentionally takes the strict route: product artifact format is bumped, old string-only alias artifacts are rejected, and `Lowerer` stops owning semantic alias maps. The implementation should still land in separable commits so the Task 4 product/resolver/artifact boundary is completed before the Task 5 lowerer extraction slice.

## Source Requirements

- `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` marks Task 4 as partial and calls out persistent string alias maps as remaining work.
- The same roadmap marks Task 5 as partial because `Lowerer` still owns scope, module caches, alias maps, resolver tables, dependency resolvers, prelude data, and artifact exports.
- `docs/superpowers/plans/master-audit-checklist.md` still lists compatibility string ownership maps and lowering-time string alias compatibility maps under Real Collection And Name Resolution.
- `docs/superpowers/plans/2026-05-18-id-backed-resolver-and-alias-interfaces.md` completed the prior helper/facade slice but explicitly preserved string maps as compatibility views.

## Completion Definition

This work is complete when all of these are true:

- Product artifact format is bumped from `21` to `22` in both compiler and shared sysroot constants.
- Product artifacts no longer serialize string-to-string alias contracts such as string prelude exports as authoritative data.
- ID-backed product metadata is the required persistent source for root exports, prelude exports, import aliases, export aliases, and resolver-visible alias data.
- Artifact loading rejects format-21/string-only alias products through the existing unsupported-format path or a specific structured validation error.
- Artifact loading remaps and validates alias `ProductDefId`s into consumer `DefId`s before exposing dependency metadata or resolver tables.
- `ResolverTables` is the canonical alias lookup authority for collected current-crate and dependency aliases.
- `Lowerer` no longer stores semantic `HashMap<String, String>` alias maps for imports, module-local aliases, artifact root exports, or export-function aliases.
- Lowering asks resolver/module-context APIs for item IDs and canonical display names instead of translating source names through lowerer-owned string alias maps.
- Missing aliases produce diagnostics and existing error-typed recovery, not guessed names, fabricated IDs, or string fallback resolution.
- Documentation marks Task 4 complete and Task 5 complete only for the strict alias/path-resolution subset, leaving broader Task 5 lowerer decomposition work open.

## Non-Goals

- Do not finish all of Task 5. Broad source module ownership, scope management, dependency resolver storage, body lowering decomposition, and type-name resolution extraction remain later work unless directly needed to remove alias maps.
- Do not migrate structural `Type` to `TypeId`; that remains Task 11.
- Do not make the selection service fully ID-only; that remains Tasks 13 and 21.
- Do not change instance registry, MIR, codegen metadata extraction, or formatter architecture.
- Do not preserve backward compatibility for old string-only product artifacts after the format bump.

## Architecture

### Resolver Tables

`ResolverTables` remains the canonical ID-backed lookup structure. It should hold alias-to-`DefId` entries and reverse canonical names for display. Resolver APIs should answer questions such as:

- resolve this source path or alias to a `DefId`
- resolve this dependency-qualified alias to a dependency `DefId`
- return a canonical display/source name for an ID
- report whether a short name is ambiguous or unavailable

Resolver aliases are semantic references. Any string retained next to them is display metadata and must not be the only target representation.

### Product Artifacts

`CompilerProducts` should expose alias persistence through ID-backed identity data. Product identity tables already contain root export IDs and prelude export IDs; the strict Task 4 completion extends this so no persistent alias class depends on a string target.

The artifact format changes to `22`. Product load rejects older artifacts instead of trying to migrate string-only maps. This keeps the compiler prototype simple and avoids compatibility shims for data shapes that are being removed.

### Artifact Loading

Artifact loading remaps `ProductDefId` alias targets to consumer `DefId`s before constructing `ExternCrateMetadata`, dependency resolver tables, prelude exports, root exports, or cross-crate HIR access. It validates that each alias target exists and has a compatible kind before exposing it.

String display names may be reconstructed from product identity display metadata or canonical interface names after IDs are validated. Missing display names are errors only when a diagnostic/display string is required for the public metadata view.

### Lowering

`Lowerer` should stop carrying semantic alias maps such as:

- `import_aliases: HashMap<String, String>`
- `module_local_aliases: HashMap<String, String>`
- `artifact_root_exports: HashMap<String, HashMap<String, String>>`
- `export_function_aliases: HashMap<String, String>`

Lowering may keep source strings for diagnostics, active module path display, and syntax-derived names. Semantic resolution goes through resolver/module-context helpers that return IDs or structured miss information. If lowering needs a display name after resolving an ID, it asks the resolver for the canonical name.

The Task 5 subset is intentionally narrow: remove lowerer-owned alias semantic state and route alias/path lookups through explicit APIs. It does not require moving every path/type/module responsibility out of `Lowerer` in this slice.

## Data Flow

1. Collection builds canonical module/item paths, import aliases, export aliases, and module-local aliases as ID-backed resolver entries.
2. Product emission serializes resolver-visible alias data and public export/prelude data by product IDs.
3. Product artifact load checks format `22`, validates/remaps alias product IDs, and constructs dependency resolver and metadata views.
4. Lowering receives resolver/module-context state and asks it for `DefId` targets when source syntax names an alias or path.
5. Lowering emits ID-backed HIR references where resolution succeeds and diagnostics/error recovery where it fails.

## Error Handling

- Old product artifacts with format `21` or earlier are rejected by format validation.
- A product alias that references an unknown, ambiguous, or kind-incompatible ID is rejected during artifact load.
- Missing lowerer alias resolution is a normal diagnostic path; lowering must not invent `DefId`s or recover through string-to-string alias maps.
- Ambiguous export names remain explicit ambiguous metadata and must not silently pick an arbitrary ID.
- Generated/current-crate IDs remain governed by Task 1 rules; this work must not reintroduce current-crate sentinel or repair IDs.

## Testing Strategy

Use TDD for each implementation slice. Required coverage:

- Resolver tests for ID-backed import/export/module-local alias insertion and lookup.
- Product tests proving artifact format `22` is required and string-only alias/prelude data is not accepted as authoritative.
- Product round-trip tests for ID-backed prelude exports, root exports, import aliases, and export aliases.
- Artifact loader tests for remapping and rejecting invalid alias target IDs.
- Lowering tests for import aliases, module-local aliases, prelude aliases, artifact root exports, and custom operator aliases resolving through IDs after lowerer string maps are removed.
- Regression tests for same-name aliases across modules/dependencies and ambiguous exports.
- Final verification with focused alias filters, `cargo test -p rock-lib product_artifact`, full `cargo test -p rock-lib`, `cargo fmt --all --check`, and `git diff --check`.

## Documentation Updates At Completion

At implementation completion, update:

- `docs/superpowers/plans/master-audit-checklist.md`: mark Task 4 alias persistence complete and keep broader Task 5/lowerer decomposition follow-ups open.
- `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`: mark Task 4 complete and Task 5 complete only for the strict alias/path-resolution subset if the implementation reaches that boundary.
- Any implementation plan created from this spec with exact verification evidence and zero-test filter replacements.
