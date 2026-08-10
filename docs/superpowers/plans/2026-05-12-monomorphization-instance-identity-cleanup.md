# Monomorphization Instance Identity Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Phase 5 of the canonical identity roadmap by replacing string-based method identity in monomorphization instance keys with canonical method `DefId` identity while preserving owner context, backend symbol separation, object-backed dependency behavior, and generic artifact method specialization.

**Architecture:** Phase 4 made child fields, variants, and associated types first-class but deliberately left `InstanceOrigin::ImplMethod { method: String }` for this phase. HIR methods already carry `HirFunction.id: DefId`, and `HirProgram` indexes method IDs in `methods_by_id`. Phase 5 should use those existing method IDs in `InstanceOrigin::ImplMethod { owner: InstanceImplOwner, method: DefId }`. Keep `InstanceImplOwner` because builtin slice methods and same method IDs under different provider contexts still need explicit owner context in the current model. Keep backend names only in `InstanceRecord::backend_symbol`; they must not participate in `InstanceKey` equality.

**Tech Stack:** Rust 2021, `rock-lib`, `lib/src/mono/*`, HIR method `DefId`s, product artifact generic bodies, object-backed dependency instances, focused unit tests, artifact integration tests, `cargo fmt --all --check`, `cargo test -p rock-lib`.

---

## File Structure

- Modify `lib/src/mono/registry.rs`: change `InstanceOrigin::ImplMethod` from `{ owner: InstanceImplOwner, method: String }` to `{ owner: InstanceImplOwner, method: DefId }`; update registry tests to prove instance identity uses method IDs and not backend names.
- Modify `lib/src/mono/mod.rs`: change `Monomorphizer::method_instance_origin` to accept the selected `HirFunction` and use `method.id`; update `function_instance_origin` method branch and origin unit tests.
- Modify `lib/src/mono/methods.rs`: pass the selected method body into `method_instance_origin` for standalone impl methods and trait methods; preserve existing specialized-name/backend-symbol generation.
- Modify `lib/src/mono/external.rs`: pass object-backed and artifact-loaded method bodies into `method_instance_origin`; add tests around object-backed method records if a local unit seam exists.
- Modify only if needed `lib/src/mono/specialize.rs`: verify generic function origins are already canonical `Function(DefId)` and do not need method-origin changes.
- Modify only if needed `lib/src/codegen/mod.rs` and `lib/src/codegen/expr/call.rs`: only update pattern matches or tests if the changed `InstanceOrigin` shape requires it; do not make codegen use instance identity for backend naming.
- Modify `lib/src/crate_artifact/tests.rs`: add or extend artifact regressions covering generic impl methods and object-backed method declarations if unit coverage in `mono/external.rs` is insufficient.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: mark the Phase 5 monomorphization instance identity item complete or document any deliberately deferred boundary.

---

## Acceptance Criteria

- [ ] `InstanceOrigin::ImplMethod` no longer stores a method name string.
- [ ] Impl-method instance identity contains canonical method `DefId` plus the existing canonical/structural owner context.
- [ ] All current-crate standalone and trait method specialization paths build `InstanceKey { origin, substitution }` from the selected method body's `HirFunction.id`.
- [ ] Object-backed dependency method records and artifact-loaded generic impl methods use the same canonical origin shape as current-crate methods.
- [ ] Backend symbols remain in `InstanceRecord::backend_symbol` and changing a backend symbol cannot change `InstanceKey` equality.
- [ ] Builtin slice method origins remain structural by owner and canonical by method ID; no fake owner `DefId` or method-name fallback is reintroduced.
- [ ] Phase 6 semantic `Type`/`Ty` migration is not included.
- [ ] No trait-selection service redesign or MIR-backed codegen rewrite is included.
- [ ] Focused tests pass before the full `cargo test -p rock-lib` run.

---

### Task 1: Lock The New Registry Identity Shape With Tests

**Files:**
- Modify: `lib/src/mono/registry.rs`

- [ ] **Step 1: Write failing registry tests for canonical method identity**

Add tests proving:
- two impl method instances with the same owner and substitution but different method `DefId`s are distinct.
- two impl method instances with the same method `DefId` and substitution but different owners are distinct.
- interning the same `{ owner, method: DefId } + substitution` twice reuses the first instance even if the second prospective record would have a different `backend_symbol`.

Keep the existing function-origin reuse test intact.

- [ ] **Step 2: Run the focused registry tests and confirm RED**

Run the exact new test filters, for example:

```bash
cargo test -p rock-lib instance_registry_distinguishes_impl_methods_by_method_id
cargo test -p rock-lib instance_registry_distinguishes_impl_methods_by_owner_and_method_id
cargo test -p rock-lib instance_registry_ignores_backend_symbol_for_identity
```

Expected before implementation: compile failure because `InstanceOrigin::ImplMethod` still expects `method: String`, or assertion failure if a temporary compatibility adapter exists.

- [ ] **Step 3: Change `InstanceOrigin::ImplMethod` to use `DefId`**

In `lib/src/mono/registry.rs`, change the variant to:

```rust
ImplMethod {
    owner: InstanceImplOwner,
    method: DefId,
}
```

Do not remove `InstanceImplOwner`. Do not add backend symbol or source name fields to `InstanceKey` or `InstanceOrigin`.

- [ ] **Step 4: Update registry tests and constructors**

Update all local registry test constructors to use explicit method `DefId` values.

- [ ] **Step 5: Run focused registry tests and confirm GREEN**

Run the exact tests added or updated in this task:

```bash
cargo test -p rock-lib instance_registry_distinguishes_impl_methods_by_method_id
cargo test -p rock-lib instance_registry_distinguishes_impl_methods_by_owner_and_method_id
cargo test -p rock-lib instance_registry_ignores_backend_symbol_for_identity
```

---

### Task 2: Build Method Origins From Selected HIR Method IDs

**Files:**
- Modify: `lib/src/mono/mod.rs`

- [ ] **Step 1: Write failing origin-helper tests**

Add/update tests proving:
- named impl method origin is `ImplMethod { owner: InstanceImplOwner::Named(imp.id), method: method.id }`.
- builtin slice method origin is `ImplMethod { owner: InstanceImplOwner::BuiltinSlice, method: method.id }`.
- the helper no longer accepts only a method name string.

Use small `HirFunction` fixtures with explicit `DefId`s. Do not rely on backend symbol strings.

- [ ] **Step 2: Run the focused origin-helper tests and confirm RED**

Run exact test filters around `builtin_slice_owner_identity_is_structural` and `named_impl_method_origin_uses_impl_def_id_not_type_def_id` after updating their expected shape:

```bash
cargo test -p rock-lib builtin_slice_owner_identity_is_structural
cargo test -p rock-lib named_impl_method_origin_uses_impl_and_method_def_ids
```

- [ ] **Step 3: Change `method_instance_origin` signature**

Change `Monomorphizer::method_instance_origin` from accepting `method_name: &str` to accepting the selected method body, for example:

```rust
fn method_instance_origin(&self, imp: &HirImpl, method: &HirFunction) -> InstanceOrigin
```

Return `method: method.id` and keep `owner: self.impl_owner_identity(imp, ...)` or the existing owner helper shape as needed.

- [ ] **Step 4: Update `function_instance_origin` method branch**

When `function_instance_origin` detects that a generic function is actually an impl method, pass the matched method body into `method_instance_origin` rather than passing `func.name`.

- [ ] **Step 5: Run focused mono origin tests and confirm GREEN**

Run the exact tests updated in this task:

```bash
cargo test -p rock-lib builtin_slice_owner_identity_is_structural
cargo test -p rock-lib named_impl_method_origin_uses_impl_and_method_def_ids
```

---

### Task 3: Thread Canonical Method Origins Through Current-Crate Method Specialization

**Files:**
- Modify: `lib/src/mono/methods.rs`

- [ ] **Step 1: Write failing current-crate method specialization tests**

Add focused tests proving:
- two generic methods with the same method name and same substitution but different impl owners produce distinct instance records.
- two different method names on the same owner with different `HirFunction.id`s and same substitution produce distinct instance records even if their backend symbols would be similar.

Do not attempt a same-owner/same-method-name test: impl methods are stored in a `HashMap<String, HirFunction>`, so that case cannot exist in current HIR. Use direct `Monomorphizer` unit tests where possible to avoid full compiler setup.

- [ ] **Step 2: Run focused method specialization tests and confirm RED**

Run the new focused filters, for example:

```bash
cargo test -p rock-lib monomorphize_same_named_methods_on_different_impls_use_distinct_method_origins
cargo test -p rock-lib monomorphize_same_owner_distinct_methods_use_distinct_method_origins
```

Expected before implementation: compile errors from the changed `InstanceOrigin` shape or tests showing method-name identity is still used.

- [ ] **Step 3: Update standalone method monomorphization**

In `monomorphize_standalone_method_call`, replace:

```rust
self.method_instance_origin(&imp, None, method_name)
```

with the selected method body. Preserve existing `source_name` and `backend_symbol` generation.

- [ ] **Step 4: Update trait method monomorphization**

In `monomorphize_trait_method_call`, do the same for the selected trait impl method body. Preserve existing receiver handling and call argument construction.

- [ ] **Step 5: Run focused current-crate method tests and confirm GREEN**

Run the tests added in this task plus nearby existing method monomorphization tests:

```bash
cargo test -p rock-lib test_monomorphize_standalone_method_call_matches_short_impl_name_for_qualified_receiver
cargo test -p rock-lib test_monomorphize_trait_method_call_matches_short_impl_name_for_qualified_receiver
cargo test -p rock-lib test_monomorphize_trait_method_call_preserves_borrowed_slice_self_type
```

---

### Task 4: Update Object-Backed And Artifact Method Origins

**Files:**
- Modify: `lib/src/mono/external.rs`
- Modify if needed: `lib/src/crate_artifact/tests.rs`

- [ ] **Step 1: Write failing object-backed method origin tests**

Add tests proving object-backed impl method records intern with:
- `origin.owner` from the impl identity.
- `origin.method` from the declared method's `HirFunction.id`.
- `backend_symbol` only in `InstanceRecord`, not in `InstanceKey`.

If this is awkward in `mono/external.rs`, add an artifact-level regression in `lib/src/crate_artifact/tests.rs` that compiles through an object-backed artifact with same-named methods on different owners.

- [ ] **Step 2: Write failing generic artifact method tests**

Add or extend artifact regressions proving a generic impl method loaded from a product artifact specializes through the same `ImplMethod { owner, method: DefId }` origin shape as a current-crate method. A compile/run regression alone is not enough because it cannot prove the internal origin shape; include a unit-level assertion over `MonomorphizedProgram.instances` or a helper seam that exposes the recorded `InstanceRecord.origin`. Use compile/run only as additional user-visible coverage.

- [ ] **Step 3: Run focused external/artifact tests and confirm RED**

Run the new exact filters plus existing artifact generic impl tests:

```bash
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle
cargo test -p rock-lib test_compile_generic_impl_from_file_module_artifact_hir_bundle
```

- [ ] **Step 4: Update `record_object_backed_impl`**

In `lib/src/mono/external.rs`, pass `method` into `method_instance_origin` while preserving `backend_symbol` and `declared` handling.

- [ ] **Step 5: Verify artifact-loaded generic impl methods need no special fallback**

Confirm `load_external_generic_functions` pushes `HirImpl`s whose methods retain remapped `HirFunction.id` values. If a method ID is missing or producer-local, fix the artifact remap path rather than adding a string fallback in mono.

- [ ] **Step 6: Run focused external/artifact tests and confirm GREEN**

Run all tests from Steps 1-3 again.

---

### Task 5: Remove Remaining String-Based Method Origin Construction

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/external.rs`
- Modify if needed: `lib/src/mono/specialize.rs`

- [ ] **Step 1: Search for remaining string method origin construction**

Run:

```bash
rg "InstanceOrigin::ImplMethod|method_instance_origin|method: .*to_string|method_name" lib/src/mono
```

- [ ] **Step 2: Remove or update all stale constructors**

Every `InstanceOrigin::ImplMethod` constructor should use a method `DefId`. Method names may remain in source lookup, diagnostics, `source_name`, and backend symbol generation only.

- [ ] **Step 3: Confirm function origins remain canonical**

Review `lib/src/mono/specialize.rs` and ensure generic function specializations still use `InstanceOrigin::Function(DefId)` through `function_instance_origin`.

- [ ] **Step 4: Run string-origin search again**

The remaining string references should be limited to method lookup, display/source names, or backend symbols; none should feed `InstanceKey` identity.

---

### Task 6: Product, Codegen, And Audit Verification

**Files:**
- Modify if needed: `lib/src/products.rs`
- Modify if needed: `lib/src/codegen/mod.rs`
- Modify if needed: `lib/src/codegen/expr/call.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`

- [ ] **Step 1: Check whether product emission needs instance-origin changes**

`CompilerProducts` stores HIR bodies and link records, not `InstanceRecord`s. Confirm no serialized product shape needs a version bump for Phase 5 unless a serialized type changes. If no serialized product type changes, do not bump `PRODUCT_ARTIFACT_FORMAT_VERSION`.

- [ ] **Step 2: Check codegen instance consumers**

`codegen` should continue consuming `InstanceRecord::backend_symbol`, `declared`, and `body`. It should not inspect method names for semantic identity. Update only compile errors or stale pattern matches caused by the new origin shape.

- [ ] **Step 3: Update the master audit checklist**

In `docs/superpowers/plans/master-audit-checklist.md`, mark:
- `Remove string-based method identity from InstanceOrigin::ImplMethod` complete.
- `Ensure all specialization entry points are keyed only by canonical (DefId, substitution)` complete if all function and method entry points now satisfy it.
- Leave Phase 6 semantic type identity gaps untouched.

- [ ] **Step 4: Run LSP diagnostics on changed Rust files**

Run `lsp_diagnostics` on each changed Rust file and fix new errors.

- [ ] **Step 5: Run formatting check**

Run:

```bash
cargo fmt --all --check
```

- [ ] **Step 6: Run focused regression tests**

Run all exact tests added or changed in Tasks 1-5. At minimum, include the focused filters named in those tasks plus:

```bash
cargo test -p rock-lib test_monomorphize_standalone_method_call_matches_short_impl_name_for_qualified_receiver
cargo test -p rock-lib test_monomorphize_trait_method_call_matches_short_impl_name_for_qualified_receiver
cargo test -p rock-lib test_monomorphize_trait_method_call_preserves_borrowed_slice_self_type
cargo test -p rock-lib test_compile_generic_impl_from_artifact_hir_bundle
cargo test -p rock-lib test_compile_generic_impl_from_file_module_artifact_hir_bundle
```

- [ ] **Step 7: Run full library test suite**

Run:

```bash
cargo test -p rock-lib
```

- [ ] **Step 8: Manual QA through compiler/artifact surface**

Run at least one compiler-surface command that exercises generic impl method specialization. Prefer a temporary Rock program under `/tmp/opencode` that defines or imports a generic impl method and compiles through `rockc`; artifact unit tests can supplement this, but manual QA should go through the compiler CLI surface.

---

## Out Of Scope

- No semantic `Type` to canonical `Ty` migration.
- No trait-selection service redesign.
- No MIR-backed codegen rewrite.
- No parser, macro, formatter, or module-loader cleanup.
- No compiler-owned stdlib discovery or implicit stdlib loading change.
- No removal of string maps used for source lookup, diagnostics, import/export metadata, `source_name`, or backend symbol generation.
- No artifact format version bump unless a serialized product type actually changes.

---

## Review Notes

- Oracle recommendation for this plan: use `ImplMethod { owner: InstanceImplOwner, method: DefId }`, not method-only `ImplMethod(DefId)`, because the owner preserves builtin slice and current model context while the method field becomes canonical.
- Every inspected method specialization path already has access to the selected `HirFunction` at `InstanceKey` construction time.
- If any path lacks a canonical method `DefId`, fix that upstream in HIR/artifact remapping. Do not add a method-name fallback in monomorphization.
