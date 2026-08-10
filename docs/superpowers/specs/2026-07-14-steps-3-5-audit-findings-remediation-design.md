# Steps 3 And 5 Audit Findings Remediation Design

## Goal

Close the three gaps found by the post-completion sweep of clean-slate audit
Steps 3 and 5:

- link-record attachment must not recover product identity from source, export,
  or display names;
- product artifacts must reject empty backend symbols during loading;
- qualified static-method values must carry canonical selected authority without
  reconstructing trait members or generic bindings from names.

The changes must preserve the existing phase boundaries: products own artifact
identity, lowering owns source-path resolution, selection authority is ID-based,
and backend symbols remain link-record data.

## Product Identity Remap

`CompilerProducts::from_resolved_hir` already constructs the authoritative
`id_remap` from each original `ProductDefId` to the product IDs reserved for it.
It applies that map to product interfaces, bodies, aliases, language items, and
initial link data, but currently discards it when construction returns. Later,
`attach_product_link_records` therefore falls back to matching
`MirArtifactExport.source_name` against export and display names.

No second remap will be introduced. Product construction will expose the
existing remap as producer-side build state for the compiler driver. It will
not be added to `CompilerProducts`, serialized into artifacts, or reconstructed
from names.

When attaching a MIR artifact export:

1. Convert its original `DefId` to the requested `ProductDefId`.
2. Read the existing remap entry for that requested ID.
3. Retain only mapped IDs that identify callable product-interface rows.
4. Require exactly one callable product ID.
5. Return a compiler error for a missing or ambiguous mapping.
6. Insert the backend symbol only into `ProductLinkData.records`.

The direct-ID, export-name, and display-name branches in
`product_link_id_for_candidate` will be replaced by this single explicit path.
Link attachment will return `Result` and compilation will propagate failures as
diagnostics rather than emitting an incomplete object-backed artifact.

## Backend Symbol Validation

Artifact loading will validate every product link record before constructing
`ExternCrateLink`. An empty `ProductLinkRecord.backend_symbol` is an artifact
integrity error, even when the record key is present. This check applies to all
link records, not only records currently required by concrete object-backed
functions or methods.

The existing checks for undeclared callable IDs and missing required records
remain unchanged. No source or display name will be used to repair an invalid
record.

## Static Method Authority

`resolve_static_method_path` currently selects a unique impl and static method
using the canonical nominal owner, but returns only a generic resolved value
containing the method `DefId`. `static_method_target_for_callee` subsequently
rescans impls, looks up the selected trait member by method name, and correlates
owner and method generics by their display names.

Qualified static-method resolution will instead preserve a typed authority
descriptor containing:

- the exact impl ID;
- the exact method ID;
- the canonical receiver pattern;
- the exact trait and effective trait-member IDs when trait-backed;
- canonical owner and method generic parameter IDs.

Lowering will instantiate the resolved function type and this authority from
the same ID-keyed generic substitution. It will then construct
`HirStaticMethodTarget` directly. It will not scan impls by method name, look up
trait members by method name, or correlate generic parameters by string.

`static_method_target_for_callee` will be deleted. Failure to construct complete
authority will produce a lowering diagnostic instead of leaving a raw method
`DefId` for accepted-HIR validation to reject later.

## Error Handling

- Missing or ambiguous product callable remaps fail current compilation.
- Empty artifact backend symbols fail artifact loading with the product ID in
  the error.
- Incomplete static-method authority fails lowering at the qualified path's
  source span.
- No compatibility fallback or artifact format shim will be added.

The product artifact schema does not change, so the artifact format version
will remain unchanged.

## Regression Coverage

Product/link tests will prove that:

- an existing remapped callable ID attaches the correct link record without
  consulting display or export names;
- duplicate or misleading display names cannot affect attachment;
- a missing or ambiguous callable remap fails compilation;
- an empty link-record backend symbol is rejected during artifact loading.

Static-method tests will prove that:

- qualified static method values preserve exact impl, method, trait, and member
  IDs;
- owner and method substitutions use canonical `GenericParamId` values even
  when display names collide or differ;
- no raw method target survives when authority construction fails;
- the prior static-method integration behavior remains valid.

The Task 5 authoritative closure ledger will record this regression as F65
before the implementation is marked complete. The clean-slate audit Step 3 and
Step 5 evidence will be refreshed after focused and full-suite verification.

## Verification

Run, in order:

- focused product link-record tests;
- focused artifact-loading tests;
- focused lower path/static-method tests;
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`;
- `cargo test -p rock-lib`;
- `cargo clippy -p rock-lib --all-targets`;
- `cargo fmt --all --check`;
- `git diff --check`.
