# First-Class Child Definition Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement Phase 4 of the canonical identity roadmap by making HIR child definitions carry first-class canonical identity after their owner has been selected.

**Architecture:** Phase 3 already made top-level HIR consumers authoritative through ID-keyed indexes and already assigns `DefId`s to trait default methods and impl methods. This phase should build on that state, not replace it. Add owner-scoped IDs for fields, enum variants, and associated types; add derived child identity indexes to HIR; and carry selected child identity through lowered HIR expressions where field and variant names are currently the only post-lookup handles.

**Tech Stack:** Rust 2021, `rock-lib`, existing `DefId`, `FieldId`, `VariantId`, `AssocTypeId`, collect -> lower_from_declarations -> infer -> mono pipeline, `cargo test -p rock-lib`.

---

## File Structure

- Modify `lib/src/hir/mod.rs`: add child identity structs/maps to `HirDefinitionIndexes`; add `id` fields to `HirField`, `HirVariant`, `HirAssociatedTypeDecl`, and `HirAssociatedTypeDef`; update `HirExprKind` field/struct/variant shapes only as far as needed to carry resolved child identity; add HIR unit tests.
- Modify `lib/src/collect/headers.rs`: assign owner-scoped child IDs in collected HIR headers for structs, enums, traits, and impls.
- Modify `lib/src/lower/collect/types.rs` and `lib/src/lower/collect/traits.rs`: mirror child ID assignment in legacy builder paths still used by tests or direct lowering.
- Modify `lib/src/lower/mod.rs`, `lib/src/lower/paths.rs`, and `lib/src/lower/control_flow/secondary.rs`: resolve field and variant names to owner-scoped IDs after owner selection and preserve those IDs in HIR expressions.
- Modify expression walkers that pattern-match changed `HirExprKind` variants: likely `lib/src/infer/finalize.rs`, `lib/src/mono/substitute.rs`, `lib/src/mono/process.rs`, `lib/src/mono/methods.rs`, `lib/src/dce.rs`, `lib/src/mir/builder/mod.rs`, `lib/src/mir/builder/expr.rs`, and `lib/src/lower/traits/conformance.rs`.
- Modify tests in the touched modules and, if necessary, add a focused integration regression in `lib/tests/integration.rs`.
- Modify `docs/superpowers/plans/master-audit-checklist.md` after implementation verification to mark the child identity decision/work as complete or record any deliberately deferred boundary.

---

## Acceptance Criteria

- [x] `HirField`, `HirVariant`, `HirAssociatedTypeDecl`, and `HirAssociatedTypeDef` carry owner-scoped IDs.
- [x] `HirProgram` derives child identity tables keyed by owner `DefId` plus child ID, with names retained only for source lookup and diagnostics.
- [x] Two structs with the same field name produce distinct `(owner DefId, FieldId)` identities and do not collide in HIR indexes.
- [x] Two enums with the same variant name produce distinct `(owner DefId, VariantId)` identities and do not collide in HIR indexes.
- [x] Trait associated type declarations and impl associated type definitions produce owner-scoped `AssocTypeId` values.
- [x] Existing trait default and impl method `DefId` behavior remains intact.
- [x] `InstanceOrigin::ImplMethod { method: String }` is left for Phase 5 unless a minimal adapter is needed; no Phase 6 semantic type migration is included.
- [x] Focused tests pass before the full `cargo test -p rock-lib` run.

---

### Task 1: Add HIR Child Identity Data And Pure Index Tests

**Files:**
- Modify: `lib/src/hir/mod.rs`

- [x] **Step 1: Write failing HIR tests for child identity indexes**

Add tests proving:
- Two structs can both have a field named `value`, but their HIR child identity entries are keyed by different owner `DefId`s.
- Two enums can both have a variant named `Some`, but their variant identity entries are keyed by different owner `DefId`s.
- A trait associated type declaration and an impl associated type definition both get owner-scoped associated type identity.

- [x] **Step 2: Run the focused HIR tests and confirm RED**

Run the smallest exact test commands for the new tests. Expected before implementation: compile failure or assertion failure because the child IDs/indexes do not exist.

- [x] **Step 3: Add child ID fields and index types**

Add owner-scoped child IDs:
- `HirField { id: FieldId, name, ty, public }`
- `HirVariant { id: VariantId, name, fields }`
- `HirAssociatedTypeDecl { id: AssocTypeId, name }`
- `HirAssociatedTypeDef { id: AssocTypeId, name, ty }`

Add compact index records, for example:
- `HirFieldLocation { owner: DefId, field_id: FieldId, name: String }`
- `HirVariantLocation { owner: DefId, variant_id: VariantId, name: String }`
- `HirAssociatedTypeLocation { owner: DefId, assoc_type_id: AssocTypeId, name: String }`

Keep names as metadata. Do not remove existing string-keyed ownership maps in this task.

- [x] **Step 4: Build child indexes in `HirDefinitionIndexes::from_parts`**

Derive field entries from structs and named enum variant fields, variant entries from enums, associated type declaration entries from traits, and associated type definition entries from impls.

- [x] **Step 5: Run focused HIR tests and confirm GREEN**

Run the exact HIR tests added in Step 1.

---

### Task 2: Thread Child IDs Through Collection And Lowering Builders

**Files:**
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/collect/types.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: constructor sites in tests as needed

- [x] **Step 1: Write failing collect/lower tests**

Add tests proving collection/lowering assigns stable owner-scoped child IDs for:
- struct fields
- enum variants and named variant fields
- trait associated type declarations
- impl associated type definitions

- [x] **Step 2: Run focused tests and confirm RED**

Run exact tests before production changes. Expected: compile failure or assertion failure until constructors assign real child IDs.

- [x] **Step 3: Assign IDs in collection headers**

Use owner-local enumeration order for child IDs:
- struct fields: `FieldId(0..)` per struct
- named enum variant fields: `FieldId(0..)` unique within the enum owner while field locations are keyed by enum `DefId`
- variants: `VariantId(0..)` per enum
- associated types: `AssocTypeId(0..)` per trait or impl owner

Use existing typed ID constructors directly; do not add new global allocators for owner-scoped IDs.

- [x] **Step 4: Mirror IDs in legacy lower builders**

Update `lower/collect/types.rs` and `lower/collect/traits.rs` to match the same owner-local assignment policy.

- [x] **Step 5: Run focused collect/lower tests and confirm GREEN**

Run the exact tests added in Step 1.

---

### Task 3: Carry Field And Variant Identity In Lowered HIR Expressions

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: expression walkers that pattern-match changed variants

- [x] **Step 1: Write failing expression identity tests**

Add tests proving:
- field access on a resolved struct carries that struct owner `DefId` and selected `FieldId`.
- struct literals preserve field IDs for their named entries.
- enum variant construction carries enum owner `DefId` and selected `VariantId`.

- [x] **Step 2: Run focused tests and confirm RED**

Run exact tests before production changes.

- [x] **Step 3: Add expression identity payloads**

Use optional resolved sidecars on the existing expression variants:
- `FieldAccess(Box<HirExpr>, String, Option<HirFieldLocation>)`
- `StructLiteral(String, Vec<HirStructLiteralField>)`, where each field keeps `name`, `value`, and `Option<HirFieldLocation>`
- `EnumVariant(String, String, Vec<HirExpr>, Option<HirVariantLocation>)`

Keep display/source names available for diagnostics and backend naming.

- [x] **Step 4: Resolve names to IDs after owner selection**

Update field and variant lookup helpers so the result includes both the selected owner `DefId` and owner-scoped child ID. Do not add fallback IDs for missing names.

- [x] **Step 5: Update expression walkers mechanically**

Adjust finalize, substitution, DCE, MIR builder, lower trait conformance walking, and mono walking to preserve the new payloads without changing unrelated behavior.

- [x] **Step 6: Run focused expression tests and confirm GREEN**

Run exact tests added in Step 1.

---

### Task 4: Audit Method And Associated Item Boundaries

**Files:**
- Modify only if needed: `lib/src/hir/mod.rs`, `lib/src/collect/mod.rs`, `lib/src/mono/*`, `lib/src/products.rs`

- [x] **Step 1: Add or update tests that lock existing method ID behavior**

Ensure existing tests still prove trait default methods and impl methods have distinct canonical `DefId`s, especially same-named methods on different owners.

- [x] **Step 2: Preserve Phase 5 boundary**

Do not replace `InstanceOrigin::ImplMethod { method: String }` in this phase unless the field/variant work requires a minimal adapter. If touched, document why and keep the change minimal.

- [x] **Step 3: Run focused method identity tests**

Run exact tests around `assign_canonical_method_ids`, `methods_by_id`, and current monomorphization method origin behavior.

---

### Task 5: Final Verification And Audit Checklist Update

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [x] **Step 1: Run diagnostics on changed Rust files**

Run `lsp_diagnostics` for each changed Rust file and fix new errors.

- [x] **Step 2: Run formatting check**

Run:

```bash
cargo fmt --all --check
```

- [x] **Step 3: Run focused regression tests**

Run all exact tests added or changed in Tasks 1-4.

- [x] **Step 4: Run full library test suite**

Run:

```bash
cargo test -p rock-lib
```

- [x] **Step 5: Manual QA through compiler surface**

Run a small `rockc` compile command for a program that uses struct fields and enum variants. Prefer an existing example if it covers both; otherwise add only a temporary file under `/tmp/opencode`.

- [x] **Step 6: Update audit checklist**

Mark the Phase 4 child identity item in `docs/superpowers/plans/master-audit-checklist.md` as complete or explicitly document any deferred boundary.

---

## Out Of Scope

- No semantic `Type` to `Ty` migration.
- No codegen-to-MIR rewrite.
- No trait-selection service redesign.
- No compiler-owned stdlib discovery or implicit stdlib loading changes.
- No removal of `InstanceOrigin::ImplMethod { method: String }` unless strictly required as a minimal bridge; that cleanup belongs to Phase 5.
