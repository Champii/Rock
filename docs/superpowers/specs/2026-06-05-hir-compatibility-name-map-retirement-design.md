# HIR Compatibility Name Map Retirement Design

## Context

The compiler has completed the main current-crate ID authority, ID-owned HIR storage, ID-backed alias, resolved-reference, monomorphization, MIR, and borrowck cleanup slices. `HirProgram` now owns finalized definitions by `DefId`, exposes ID-keyed iterators and accessors, and preserves name tables for compatibility and display.

The remaining Identity/Arenas gap is direct semantic use of string-keyed HIR ownership maps such as `program.names.functions_by_name`, `structs_by_name`, `enums_by_name`, and `traits_by_name`. Some consumers still use these maps to recover semantic owners or to enumerate alias names. That makes it harder to prove that canonical IDs, not source names, drive downstream behavior.

## Goal

Retire compatibility string HIR ownership maps as semantic inputs for migrated consumers. Keep names as display, diagnostics, artifact compatibility, alias metadata, and unresolved/error-recovery inputs where the compiler genuinely does not have a canonical ID yet.

## Non-Goals

- Do not delete all `HirNameTables` fields in this slice.
- Do not remove parser/lowerer lexical string maps that model source syntax, scopes, or local variables.
- Do not redesign product artifact schema, formatter trivia, source loading, or codegen metadata. This slice may only adjust metadata code when the adjustment replaces a direct HIR name-map semantic read with an ID-backed or display-alias helper.
- Do not remove compatibility lookup paths used only for unresolved HIR names or legacy artifact/display boundaries unless an ID-backed replacement is already available.

## Architecture

`HirProgram` ID-owned maps and `HirDefinitionIndexes` remain the semantic authority. A consumer that already has a `DefId`, `InstanceId`, `HirVarTarget`, `HirCallTarget`, `HirMethodCallTarget`, field sidecar, variant sidecar, or type ID should use that identity directly.

String name tables stay as compatibility metadata with explicit intent. If a consumer needs alias/display names for a canonical owner, it should call an explicit helper whose name makes that intent clear, rather than iterating `program.names.*_by_name` inline. If a consumer needs to resolve an unresolved source name, it may use a compatibility lookup path that is named as such and guarded by tests.

The intended direction is:

1. Add small `HirProgram` helper APIs for display/alias enumeration by ID when current consumers need all names for a canonical owner.
2. Replace direct semantic `program.names.*_by_name` reads in MIR builder, codegen, DCE, and monomorphization with ID-backed accessors or explicit display-alias helpers.
3. Keep existing `function_by_name`, `struct_by_name`, `enum_by_name`, and `trait_by_name` only for lexical compatibility paths, test setup, and unresolved-name recovery. Any call site left in production code must be classified in code by using an explicitly named helper or by remaining in a function whose name already says it is a compatibility lookup.
4. Add focused regressions for same-name and alias cases showing migrated consumers select by canonical IDs and only use strings for display aliases.

## Components

- `lib/src/hir/mod.rs`: add or refine helper APIs for alias/display names by canonical `DefId`, and document the boundary between ID-owned storage and name-table compatibility metadata.
- `lib/src/mono/process.rs`, `lib/src/mono/mod.rs`, and `lib/src/mono/methods.rs`: keep unresolved-name compatibility explicit while ensuring resolved function values/calls use ID-backed lookups when available.
- `lib/src/mir/builder/mod.rs`: replace direct name-map ownership lookups for callable and nominal metadata with ID-backed accessors or display-alias helpers.
- `lib/src/codegen/mod.rs` and related codegen metadata helpers: keep layout/display aliases where needed, but avoid treating name maps as semantic owner sources.
- `lib/src/dce.rs`: keep method lookup aliases explicit and sourced from canonical owner IDs, not direct inline map iteration.
- Documentation: update `master-audit-checklist.md` and the ordered roadmap after verification to describe which compatibility name-map consumers were retired and which remain as explicit non-semantic boundaries.

## Error Handling And Diagnostics

Diagnostics should continue to use source/display names. If a semantic ID lookup fails where an ID should be present, preserve the existing structured error or invariant behavior instead of falling back to a same-named item. Unresolved-name recovery paths may continue to report source names, but they must not silently select a different canonical owner when a resolved ID exists.

## Testing

Use TDD for each migrated consumer:

- Add focused tests that create same-name or alias-bearing HIR where a direct name-map lookup could pick the wrong owner.
- Prove the consumer uses the canonical ID or explicit target sidecar instead.
- Add helper-level tests for any new alias/display-name APIs.
- Run focused mono, MIR builder, codegen, DCE, and artifact tests for changed consumers, then `cargo fmt --all --check`, `git diff --check`, and `cargo test -p rock-lib` before closing the bead.

## Acceptance Criteria

- Direct `program.names.*_by_name` semantic reads are removed from the migrated consumer paths in this slice or replaced by explicit display/compat helper APIs.
- Any remaining name-table reads in `lib/src` are clearly lexical, diagnostic/display, artifact compatibility, unresolved-name recovery, or local-variable state.
- Same-name/alias regressions prove migrated consumers do not recover semantic owners from strings when canonical IDs are available.
- `master-audit-checklist.md` and the ordered roadmap accurately describe the retired compatibility name-map scope without claiming that all string metadata is gone.
