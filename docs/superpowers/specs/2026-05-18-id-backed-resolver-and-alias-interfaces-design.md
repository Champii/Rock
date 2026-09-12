# ID-Backed Resolver And Alias Interfaces Design

## Goal

Make resolver alias tables the authoritative interface between collection, lowering, products, and artifact loading, so body/type lowering asks for canonical IDs instead of translating source names through persistent string-to-string maps.

## Context

The completed HIR storage slice made current-crate IDs and core HIR storage ID-first. The next blockers are alias and resolution boundaries:

- `ResolverTables` already stores `import_aliases` and `export_aliases` as `HashMap<String, DefId>`, but `build_resolver_tables` still receives `HashMap<String, String>` alias inputs.
- `Declarations`, `CollectContext`, and `Lowerer` still carry compatibility maps such as `import_aliases`, `export_aliases`, `export_function_aliases`, `stdlib_prelude_exports`, and `artifact_root_exports`.
- `Lowerer::from_declarations`, expression path lowering, type lowering, trait lookup, owner lookup, prelude injection, and artifact glob handling still reconstruct or consult strings even when canonical IDs are available.
- Product artifacts already persist root exports as `ProductDefId` in `ProductIdentityTable::export_names`, but prelude exports are still stored as `CompilerProducts::prelude_exports: BTreeMap<String, String>` and validated back into IDs at load time.

## Design

### Resolver API Boundary

Add narrow resolver APIs on `ResolverTables` and use them before direct map access:

- `resolve_item_or_alias(&self, name: &str) -> Option<DefId>` checks canonical item paths, import aliases, then export aliases.
- `canonical_name(&self, id: DefId) -> Option<&str>` returns display/canonical source names.
- `insert_import_alias_with_name(&mut self, alias: String, source: String, id: DefId)` records both the alias ID and the reverse name if absent.
- `insert_export_alias_with_name(&mut self, alias: String, source: String, id: DefId)` records both the alias ID and the reverse name if absent.

These helpers do not remove existing fields immediately. They make the ID-backed path explicit and reduce repeated lookup chains in lower/collect/product code.

### Collection Handoff

Keep legacy string maps during this slice only as compatibility/display data, but make resolver aliases the semantic handoff:

- Continue collecting import/export source strings while parsing/export expansion still produces source names.
- Build `ResolverTables` from those strings and existing canonical ID data.
- In `Declarations`, treat `resolver.import_aliases` and `resolver.export_aliases` as authoritative for alias targets.
- Keep `Declarations.import_aliases`, `export_aliases`, and `export_function_aliases` until all lowering consumers move to ID-backed APIs. They must not override resolver ID entries.

This avoids widening the slice into a full AST export representation rewrite.

### Lowerer Resolution Boundary

Add a small lowerer-side resolution facade that asks resolver tables for IDs first and reconstructs names only for existing HIR maps and diagnostics:

- `Lowerer::resolve_item_def_id(&self, name: &str) -> Option<DefId>` checks current resolver and dependency resolvers through `ResolverTables::resolve_item_or_alias`.
- `Lowerer::canonical_name_for_def_id(&self, id: DefId) -> Option<&str>` checks current and dependency resolver reverse maps.
- `Lowerer::canonical_name_for_alias_or_item(&self, name: &str) -> Option<String>` resolves a source name or alias to an ID, then returns the canonical source name.
- `trait_by_name`, `nominal_def_id_for_name`, `try_canonical_owner_path`, and `resolve_owner_def_id_inner` should use the facade before falling back to legacy strings.
- Expression path lowering can still emit `HirExprKind::Var(String)` for locals and backend-symbol compatibility, but top-level aliases should resolve via IDs before looking in string maps.
- Type lowering should prefer resolver IDs for structs/enums and only use suffix matching as a fallback for legacy/error-recovery behavior.

The `Scope` alias marker remains as-is. It protects shadowing and local binding behavior, and replacing it belongs to the later full source/path resolution split.

### Product And Artifact Aliases

Root exports are already ID-backed in `ProductIdentityTable::export_names`. This slice should close the equivalent prelude gap:

- Add `ProductIdentityTable::prelude_export_names: BTreeMap<String, ProductDefId>` with `#[serde(default)]`.
- Populate it when product emission knows a prelude export source and canonical product ID.
- Make product loading normalize prelude exports through `prelude_export_names` first, remapping `ProductDefId` to `DefId` and deriving the source name from `display_names`.
- Keep `CompilerProducts::prelude_exports` temporarily as compatibility string data and fallback validation.
- Reject stale prelude IDs if the product ID cannot be remapped or has no display name, matching root export validation behavior.

This keeps artifact format evolution small and avoids changing external artifact name semantics. `--extern-artifact name=path` remains a crate identity check, not a dependency alias mechanism.

## Non-Goals

- Do not remove every string-keyed HIR compatibility view in this slice.
- Do not replace local variable references or all HIR call edges with IDs here.
- Do not introduce a full source/module loader boundary.
- Do not start the `Ty`/type-context migration.
- Do not change stdlib/sysroot discovery policy.

## Testing Strategy

- Add resolver unit tests for helper lookup order and alias name recording.
- Add collection tests proving stale compatibility string aliases cannot override resolver ID targets.
- Add lowerer tests for function/extern aliases, trait aliases, and nominal type aliases resolving by canonical ID.
- Add artifact/product tests for ID-backed prelude export roundtrip, remapping, and stale-ID rejection.
- Run focused tests first, then `cargo fmt --all --check`, `git diff --check`, and `cargo test -p rock-lib` before claiming the slice is complete.

## Risks

- Some tests intentionally preserve string-only aliases for legacy paths; those should be retained as compatibility behavior until their consumers are migrated.
- `export_function_aliases` still mirrors lowered function bodies by name. It can be derived from resolver IDs in this slice, but the full removal of backend/name call targets belongs to later instance and MIR tasks.
- Type lowering still has suffix fallback behavior. The resolver path should be preferred and tested, but removing all suffix fallback requires a broader diagnostic and ambiguity plan.
- Prelude exports can be derived from `crate::prelude::*` items during load. ID-backed persisted prelude names must not break that derivation for artifacts emitted before the new field exists.
