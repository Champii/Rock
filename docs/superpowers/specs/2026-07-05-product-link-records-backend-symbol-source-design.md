# Product Link Records Backend Symbol Source Design

## Goal

Complete `CLEAN_SLATE_COMPILER_AUDIT.md` Step 3 in one implementation pass: make product link records the only product-artifact backend symbol source.

The completed step must remove duplicated identity-table backend symbol storage, require explicit link records for object-backed concrete callables, remove backend-symbol fallbacks from source or display names, bump the product artifact format, and mark the whole Step 3 complete in the clean-slate audit after verification.

## Current State

Product artifacts currently carry backend symbols in two places:

- `ProductLinkData.records`, where each `ProductLinkRecord` stores the object-file backend symbol for a product definition.
- `ProductIdentityTable.backend_symbols`, a duplicated compatibility map populated from link records.

Several paths still treat the identity-table map or source names as fallback backend-symbol authorities:

- `CompilerProducts::from_resolved_hir` remaps incoming link data, then copies backend symbols into `identity_table.backend_symbols`.
- `attach_product_link_records` writes each MIR artifact export to both `products.link.records` and `products.identity_table.backend_symbols`.
- Artifact loading merges `products.link.records` first, then fills missing symbols from `products.identity_table.backend_symbols`.
- `product_id_for_exported_backend_symbol` reconstructs exported symbols from `qualified_name`, display names, trait names, impl type names, and method names when attaching link records.
- `ExternCrateRef::imported_function_symbol` and `imported_impl_method_symbol` fall back from missing link symbols to `qualified_name`, interface names, or formatted impl/method names.

This leaves backend symbol identity split across link data, identity metadata, and backend-shaped source strings.

## Non-Goals

- Do not split or remove `HirFunction.qualified_name`; Step 4 owns that broader cleanup.
- Do not redesign product identity remapping beyond what is needed to preserve/remap `ProductLinkData.records`.
- Do not change backend symbol mangling rules except to stop deriving missing link records from source/display names.
- Do not preserve compatibility with older artifact format versions; this project is in clean-slate cleanup mode.
- Do not add compiler-owned stdlib loading or sysroot discovery behavior.

## Architecture

`ProductLinkData.records` becomes the only serialized product backend-symbol authority. `ProductIdentityTable` remains for local crate identity, dependency identity, display names, aliases, exports, prelude exports, and ambiguity markers only.

After artifact load, `ExternCrateLink.backend_symbols` is populated only from validated `ProductLinkData.records`. It remains the in-memory dependency-link lookup used by downstream codegen/linking, but it is no longer repaired from identity-table metadata or source names.

Missing link records for object-backed concrete bodies are artifact errors, not recoverable fallback cases. A product artifact with an object path claims that some concrete dependency bodies may be linkable from an object file; any function or impl method that is object-provided and concrete must have an explicit link record. Metadata-only bodies and generic bodies that require downstream specialization do not require link records.

## Components

### Product Data Model

Remove `ProductIdentityTable.backend_symbols` and all production reads/writes of that field.

Keep `ProductLinkData` and `ProductLinkRecord` as the serialized symbol container:

```rust
pub struct ProductLinkData {
    pub object_path: Option<PathBuf>,
    pub records: BTreeMap<ProductDefId, ProductLinkRecord>,
}

pub struct ProductLinkRecord {
    pub backend_symbol: String,
}
```

`remap_link_data` continues to remap link records and drop ambiguous original IDs. The helper `backend_symbols_from_link` is deleted because there is no identity-table mirror to populate.

### Link Record Attachment

`attach_product_link_records` should insert only into `products.link.records`. It should not mirror into identity metadata.

The product ID for an exported MIR artifact symbol should come from explicit identity mapping, not from reconstructing backend symbols from source names. The implementation can keep a product-id resolver for exported symbols, but it must not use `qualified_name`, display names, impl type names, trait names, or method names as backend-symbol fallbacks.

If a MIR artifact export cannot be matched to a product definition by explicit identity, the current non-fallible attach path should leave it unattached. It must not fabricate a product link record from a source/display string.

### Artifact Loading And Validation

Artifact load should build dependency backend-symbol maps only from `products.link.records`:

- Validate each link record ID with the existing callable-interface validation.
- Remap each valid product ID to a loaded `DefId`.
- Populate `ExternCrateLink::object(object_path, backend_symbols)` with this map.
- Do not inspect identity metadata for backend symbols.

Add validation for missing records in object-backed artifacts. For each object-provided concrete callable in the artifact interface, loading should require a corresponding `ProductLinkRecord`:

- Concrete exported functions that do not require downstream specialization require records.
- Concrete impl methods that are object-provided require records.
- Generic functions, generic impl bodies, and metadata-only artifacts do not require object link records.

The validation error should name that an object-backed artifact is missing a product link record/backend symbol for an interface callable. This gives users a direct artifact integrity failure instead of a later linker or codegen fallback failure.

### External Dependency Store

`ExternCrateRef::imported_function_symbol` and `imported_impl_method_symbol` should return a symbol only when `ExternCrateLink` has one for the callable `DefId`.

Remove these fallbacks:

- function `qualified_name`;
- interface name;
- method `qualified_name`;
- formatted `"{impl_type}_{method_name}"` names.

Provider predicates should stay aligned with link data. If an object-backed dependency has no link record for a concrete callable, it should not be treated as object-provided by downstream phases. The artifact loader rejects that state before the store is queried; the store methods still avoid fallback symbols if malformed in-memory data reaches them.

### Artifact Format Version

The serialized product artifact shape changes when `ProductIdentityTable.backend_symbols` is removed. Bump the shared product artifact format version in both places that assert equality:

- `lib/src/products.rs`
- `rock-shared/src/sysroot.rs`

Update tests that pin the numeric format version.

## Data Flow

The intended product/link flow after Step 3 is:

```text
MIR artifact exports
    -> attach explicit ProductLinkData.records
    -> serialize product artifact
    -> load artifact and validate records
    -> remap ProductDefId to DefId
    -> ExternCrateLink.backend_symbols
    -> downstream dependency symbol lookup
```

Identity/display metadata remains available for diagnostics, imports, exports, and user-facing names. It does not participate in backend symbol lookup or repair.

## Error Handling

Errors should be explicit and early:

- A link record whose product ID is not a callable interface declaration remains an artifact load error.
- An object-backed artifact missing a required link record for a concrete callable becomes an artifact load error.
- A downstream imported symbol lookup with no link record returns `None`; it does not synthesize a string fallback.
- Ambiguous product ID remaps continue to drop unsafe link records instead of guessing ownership.

Error messages should mention product link records or backend symbols and identify whether the problem is an undeclared callable or a missing required link record.

## Testing Strategy

Use test-driven implementation for the full step.

Focused tests should cover:

- `CompilerProducts::from_resolved_hir` preserves link records but does not mirror backend symbols into identity metadata.
- `attach_product_link_records` writes only `ProductLinkData.records`.
- Artifact load rejects identity-table-only backend symbols because the field no longer exists in the current format.
- Artifact load rejects object-backed concrete functions with missing link records.
- Artifact load rejects object-backed concrete impl methods with missing link records.
- Metadata-only artifacts and generic/downstream-specialized bodies do not require link records.
- `ExternCrateRef` imported symbol helpers do not fall back to `qualified_name` or formatted names.
- Product artifact format version is bumped and matches `rock_shared::sysroot::PRODUCT_ARTIFACT_FORMAT_VERSION`.

Regression verification should include focused product/artifact/store tests, semantic identity audit tests, the integration suite, formatting, whitespace checks, and a residue scan for removed backend-symbol fallback authority.

Suggested residue scan targets:

- `ProductIdentityTable.*backend_symbols`
- `identity_table.backend_symbols`
- `backend_symbols_from_link`
- external-store fallback patterns from `backend_symbol(...).or_else(|| ...qualified_name...)`

## Completion Criteria

Step 3 is complete only when all of the following are true:

- `ProductIdentityTable.backend_symbols` is removed from production and tests.
- Backend symbols exist only in `ProductLinkData.records` on disk and loaded `ExternCrateLink.backend_symbols` in memory.
- Object-backed concrete callables without link records fail during artifact load.
- No production backend-symbol lookup falls back to `qualified_name`, display names, interface names, impl type names, trait names, or method names.
- The product artifact format version is bumped in `lib` and `rock-shared`.
- Focused and integration verification passes.
- `CLEAN_SLATE_COMPILER_AUDIT.md` marks the entire Step 3 complete with validation evidence.

## Risks

- Some current tests intentionally exercise identity-table backend-symbol fallback. They should be rewritten to assert rejection or link-record-only behavior.
- Product ID matching for exported backend symbols may still rely on backend-shaped source names. If so, the implementation must replace that with explicit artifact export identity or fail loudly; it must not preserve a string fallback.
- Object-backed artifact validation needs to distinguish concrete object-provided bodies from generic bodies that require downstream specialization.
- Bumping the artifact format touches shared sysroot version checks; tests in `rock`, `rockup`, or integration code may need numeric updates.
