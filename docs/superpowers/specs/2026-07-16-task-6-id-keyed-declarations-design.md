# Task 6 ID-Keyed Declarations Design

**Goal:** Complete Step 6 of `CLEAN_SLATE_COMPILER_AUDIT.md` by making collected and partially lowered declaration payloads ID-keyed, eliminating lower-phase semantic ID recovery from candidate strings, and proving closure with a fresh whole-codebase analysis before updating the audit.

**Scope:** This is a hard-cut migration across collection, lowering, and partial HIR. It does not retain synchronized name-keyed payload maps or compatibility adapters.

## Problem

The final `HirProgram` is already ID-keyed, but its inputs are not:

- `collect::Declarations` stores named declaration payloads in string-keyed maps while carrying `ItemIndex` as a parallel ID system.
- `lower::items::LowerItems` stores the same payloads by name, clones them into secondary ID indexes, and repairs those indexes with `rebuild_id_indexes`.
- Lowering mutates bodies and recovers declaration IDs through `def_id_for_name`, including candidate-name fallback lists.
- `infer::PartialHir` returns to string-keyed semantic maps before finalization converts the values back to ID-keyed final HIR.
- Resolver-owned names, payload ownership, display aliases, and semantic identity are therefore mixed across the collect/lower boundary.

Step 6 is complete only when the semantic payload path is ID-keyed from collection through final HIR and names remain confined to source resolution, lexical scopes, diagnostics, display metadata, and import/export boundaries.

## Selected Approach

Use a dedicated ID-keyed declaration payload model while preserving `ItemIndex` and `ResolverTables` in their existing identity and name-resolution roles.

Rejected alternatives:

- Do not add HIR payloads to `ItemIndex`. Source indexing and partially lowered semantic payloads have different responsibilities.
- Do not construct `PartialHir` directly in collection. Collection owns headers and resolution; lowering still owns body conversion and constraints.
- Do not retain duplicate string-keyed payload maps during migration. This repository does not require compatibility shims, and dual storage would preserve the authority problem Step 6 removes.

## Ownership Model

### Collection

`Declarations` owns one canonical copy of every collected semantic payload. Named top-level payloads are keyed by their canonical `DefId`:

- functions;
- standalone function signatures;
- structs;
- enums;
- traits;
- extern declarations.

Impls and any other declaration category with a `DefId` also use ID-keyed primary storage. If source order is behaviorally required, collection records that order explicitly rather than relying on string-map insertion or payload duplication.

`ItemIndex` continues to own indexed source/module identity. `ResolverTables` continues to own canonical paths, source names, import aliases, export aliases, and module-local aliases. Neither structure owns duplicate HIR payloads.

Collection also emits exact body-owner records for functions, trait defaults, and impl methods. These records associate each body-bearing AST position with the canonical declaration or method `DefId`. The key must be structural source identity, such as module identity plus source position/order, rather than a list of candidate names.

### Lowering

`LowerItems` consumes the collected ID-keyed maps directly and remains the mutable owner while bodies are lowered.

It has no parallel name-keyed payload maps, fallback scans, cloned `*_by_id` indexes, or `rebuild_id_indexes` operation. Reads and mutations of semantic declarations use `DefId`.

The standalone `(owner_name, method_name)` method payload map is removed. Methods remain canonically attached to their trait or impl and are addressable through their collected IDs. Source method names can remain inside owner-local declaration metadata where selection of source syntax requires them, but they cannot be used to reconstruct a global semantic owner or target.

Body lowering receives the exact owner ID produced by collection. It does not call `def_id_for_name`, probe short and qualified alternatives, or recover an ID from display aliases. Local variable scopes remain string-keyed because lexical source names are legitimate lowering input.

### Partial And Final HIR

`PartialHir` stores functions, structs, enums, traits, impls, and externs by `DefId`. Inference and finalization preserve those keys while transforming payload values.

Final `HirProgram` construction accepts ID-keyed payloads directly. It does not temporarily rebuild identity by iterating string-keyed maps. `HirNameTables` and canonical display indexes are populated from resolver/display metadata.

## Data Flow

```text
AST + source modules
  -> ItemIndex + ResolverTables
  -> ID-keyed Declarations + exact body-owner records
  -> ID-keyed LowerItems
  -> ID-keyed PartialHir
  -> ID-keyed HirProgram
```

Collection is the only phase that assigns or resolves global declaration identity. Lowering may read source names and resolver display data for syntax and diagnostics, but semantic payload access after collection uses IDs.

Function signature metadata, including unsafe-signature state, is keyed by the function or signature `DefId`. Imported aliases resolve to IDs through resolver tables before payload access. Dependency declarations follow the same ID-keyed payload contract as current-crate declarations.

## Errors And Invariants

Collection reports structured diagnostics for:

- duplicate payload insertion under one `DefId`;
- missing indexed owners;
- disagreement between a map key and a payload's embedded ID;
- missing body-owner identity for a body-bearing declaration.

Lowering reports an invariant diagnostic, rather than panicking or trying alternate names, when:

- a body-owner record is missing;
- an owner ID has no matching declaration payload;
- a body-owner record refers to the wrong declaration category;
- a key and embedded payload ID disagree.

`PartialHir` validates key/embedded-ID agreement before inference. Existing provisional and duplicate method-ID checks remain and operate on ID-keyed storage.

Diagnostics use source spans and resolver-owned canonical names. Normal user-facing errors do not expose raw IDs.

## Testing Strategy

Implementation proceeds in focused TDD slices:

1. Collection tests prove payloads are keyed by canonical IDs and that aliases or duplicate source names cannot change payload identity.
2. Lower item-store tests prove semantic lookup and body mutation use only `DefId` and that key/payload mismatches are rejected.
3. Body-lowering tests cover root functions, inline modules, source-backed modules, trait defaults, impl methods, imported functions, and imported externs.
4. Inference tests prove `PartialHir` and final HIR preserve IDs while canonical and display names still come from resolver metadata.
5. Negative tests cover missing body-owner records, absent ID-keyed payloads, duplicate IDs, and key/embedded-ID mismatches.
6. Existing collection, lowering, inference, artifact, semantic-identity, integration, and full library suites remain passing.

Source residue searches are validation evidence, not permanent tests that merely assert removed symbols are absent. Durable tests must verify semantic behavior or phase-boundary invariants.

## Mandatory Closure Analysis

Passing tests is necessary but not sufficient. Before Step 6 is marked complete in `CLEAN_SLATE_COMPILER_AUDIT.md`, perform a fresh full-codebase analysis from the final source state.

The closure analysis must:

1. Trace the complete production flow from source collection through `Declarations`, `LowerItems`, `PartialHir`, inference finalization, and final `HirProgram` construction.
2. Inspect every production definition, construction, mutation, and consumer of those phase-boundary types.
3. Inventory every remaining `HashMap<String, Hir...>`, `(String, String)` declaration map, name-to-`DefId` helper, candidate-name lookup, and payload scan across the repository.
4. Classify every remaining string-keyed use as source resolution, lexical scope, import/export metadata, diagnostics/display, test-only support, or a prohibited semantic payload/identity path.
5. Inspect current-crate, loaded-module, inline-module, dependency-artifact, prelude, trait-default, impl-method, extern, and standalone-signature paths.
6. Confirm there is no replacement compatibility adapter, synchronized duplicate store, fallback scan, or semantic recovery hidden behind a renamed helper.
7. Confirm final HIR names and aliases remain display/resolver data and cannot select semantic payloads.
8. Compare the final implementation directly against every Step 6 task and acceptance criterion in the audit.
9. Record all findings. Any unresolved or ambiguous production finding blocks completion and must be fixed and revalidated before the audit status changes.
10. Re-run affected focused tests and the required full quality gates after the last closure fix.

The audit document may be updated only when this analysis finds no remaining Step 6 production debt. Its status entry must include exact test counts, quality-gate results, residue-analysis scope, and any intentionally allowed name-keyed boundaries. It must not claim completion of later clean-slate steps.

## Verification Gates

Run focused tests first, followed serially by broader gates:

```bash
cargo test -p rock-lib collect -- --nocapture
cargo test -p rock-lib lower -- --nocapture
cargo test -p rock-lib infer -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib --test integration
cargo test -p rock-lib
cargo clippy --workspace --all-targets
cargo fmt --all --check
git diff --check
```

The full-codebase closure analysis follows implementation and normal verification. If it discovers a defect, fix it, rerun the smallest relevant RED/GREEN test, rerun every invalidated broader gate, and repeat the closure analysis over the final source.

## Acceptance Criteria

- `Declarations` exposes semantic declaration payloads through canonical IDs, not string keys.
- `LowerItems` has one ID-keyed payload authority and no rebuilt or synchronized secondary indexes.
- `PartialHir` semantic item maps are ID-keyed.
- Production `def_id_for_name` and candidate-string semantic recovery are deleted.
- Lowering body ownership is supplied by collection-produced canonical identity.
- Resolver and display names cannot select or mutate semantic declaration payloads after collection.
- Root, module, dependency, trait, impl, extern, signature, and artifact paths satisfy the same contract.
- Focused and full verification gates pass.
- The mandatory final whole-codebase analysis reports no unresolved Step 6 finding.
- `CLEAN_SLATE_COMPILER_AUDIT.md` is updated only after all preceding criteria are proven.

## Non-Goals

- Normalizing compiler-recognized traits through language items.
- Deciding builtin indexing semantics.
- Replacing AST receiver modes past lowering.
- Removing lenient type finalization except where a direct Step 6 change requires adapting ID-keyed traversal.
- Redesigning parser syntax, local lexical scopes, resolver name tables, or diagnostic display naming.
- Completing later clean-slate audit steps under the Task 6 status.
