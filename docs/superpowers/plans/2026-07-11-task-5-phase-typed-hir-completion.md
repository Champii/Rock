# Task 5 Phase-Typed HIR Completion Implementation Plan

**Status:** Complete as of 2026-07-14. The authoritative closure record is
`docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make selected method authority mandatory at the accepted-HIR type boundary and remove every remaining method rediscovery, repair, and ambiguous impl-selection path before MIR.

**Architecture:** Parameterize body-bearing HIR by a sealed phase trait and provide `HirProgram`/`HirExpr` aliases for unresolved lowering plus `AcceptedHirProgram`/`AcceptedHirExpr` aliases for strict output. The phase chooses method and Try authority field types: unresolved HIR uses `Option`, accepted HIR uses required authority records. Strengthen conversion and artifact validation, then make mono and MIR consume accepted HIR before deleting compatibility paths.

**Tech Stack:** Rust 2021, serde/bincode product artifacts, existing HIR/mono/MIR pipeline, Cargo tests.

---

No VCS operations are part of this plan. The session instructions prohibit commits unless explicitly requested.

## File Structure

- Create `lib/src/hir/accepted.rs`: sealed phase markers, accepted authority records, recursive strict conversion, and accepted type aliases.
- Modify `lib/src/hir/mod.rs`: parameterize body-bearing HIR nodes by phase while preserving unresolved aliases for lowering.
- Modify `lib/src/infer/mod.rs`: return `AcceptedHirProgram` from strict finalization while lenient/debug finalization remains unresolved.
- Modify `lib/src/lib.rs`: pass accepted HIR into mono and unwrap it only at the post-mono boundary.
- Modify `lib/src/crate_artifact/load.rs`: validate exact impl receiver-pattern generic ownership and coverage.
- Modify `lib/src/mono/mod.rs`: accept only `AcceptedHirProgram` and remove accepted-program rediscovery helpers.
- Modify `lib/src/mono/process.rs`: remove method-ID repair and Try receiver reconstruction.
- Modify `lib/src/mono/methods.rs`: reject every multi-impl typed match before selection.
- Modify `lib/src/mir/builder/mod.rs`: index ordinary function origins only in the DefId-to-instance map.
- Modify `lib/src/semantic_identity_audit.rs`: assert the deleted repair symbols and patterns cannot return.
- Modify nearby unit tests in the files above and `lib/tests/integration.rs` only where user-visible behavior needs coverage.

### Task 1: Add the accepted-HIR boundary

**Files:**
- Create: `lib/src/hir/accepted.rs`
- Modify: `lib/src/hir/mod.rs`
- Test: `lib/src/hir/accepted.rs`

- [ ] **Step 1: Write failing accepted-boundary tests**

Add tests that construct minimal programs for these cases and assert `AcceptedHirProgram::try_from` rejects them:

```rust
#[test]
fn accepted_hir_rejects_raw_method_callee_without_call_target() {
    let program = program_with_raw_method_callee_and_no_target();
    let errors = AcceptedHirProgram::try_from(program).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("raw method DefId")));
}

#[test]
fn accepted_hir_rejects_try_receiver_mode_mismatch() {
    let program = program_with_try_branch_receiver(None);
    let errors = AcceptedHirProgram::try_from(program).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("Try branch receiver mode")));
}
```

- [ ] **Step 2: Run the tests and verify RED**

Run:

```text
cargo test -p rock-lib hir::accepted::tests -- --nocapture
```

Expected: compilation fails because `AcceptedHirProgram` does not exist.

- [ ] **Step 3: Implement phase-selected authority field types**

Define a sealed phase trait and parameterize the existing tree rather than maintaining duplicate executable bodies:

```rust
mod sealed {
    pub trait Sealed {}
}

pub trait HirPhase: sealed::Sealed {
    type MethodAuthority;
    type TryAuthority;
}

pub struct UnresolvedHir;
pub struct AcceptedHir;

pub struct AcceptedTryAuthority {
    pub branch_method: HirMethodCallTarget,
    pub self_receiver: Option<SelfReceiverMode>,
    pub from_residual: HirStaticMethodTarget,
}

impl HirPhase for UnresolvedHir {
    type MethodAuthority = Option<HirMethodCallTarget>;
    type TryAuthority = UnresolvedTryAuthority;
}

impl HirPhase for AcceptedHir {
    type MethodAuthority = HirMethodCallTarget;
    type TryAuthority = AcceptedTryAuthority;
}

pub type HirProgram = HirProgramFor<UnresolvedHir>;
pub type AcceptedHirProgram = HirProgramFor<AcceptedHir>;
pub type HirExpr = HirExprFor<UnresolvedHir>;
pub type AcceptedHirExpr = HirExprFor<AcceptedHir>;
```

Parameterize `HirProgramFor`, `HirFunctionFor`, `HirImplFor`, `HirTraitFor`, `HirBlockFor`, `HirStmtFor`, `HirExprFor`, and `HirExprKindFor` recursively. Keep declaration-only structs non-generic. `MethodCall` stores `P::MethodAuthority`; `Try` stores `P::TryAuthority`. `AcceptedHirProgram::try_from(HirProgram)` recursively moves nodes and rejects missing method targets, missing Try authorities, raw method `DefId` callees, receiver mismatches, and unresolved static owner types. Do not implement an unchecked unresolved-to-accepted conversion.

- [ ] **Step 4: Strengthen recursive validation**

In `validate_method_authorities_in_expr`:

```rust
HirExprKind::ResolvedVar(reference) => {
    if matches!(reference.target, HirVarTarget::Function(id)
        if program.indexes.methods_by_id.contains_key(&id))
    {
        errors.push(format!("accepted HIR callable retains raw method DefId {id:?}"));
    }
}
```

For `HirExprKind::Call`, reject the same raw method callee even when `target` is `None`. For `HirExprKind::Try`, compare `branch_self_receiver` with `method_target_self_receiver(program, target)` and report a mismatch before acceptance.

- [ ] **Step 5: Run accepted-boundary tests and verify GREEN**

Run the Step 2 command. Expected: all `hir::accepted::tests` pass.

### Task 2: Move strict inference output onto accepted HIR

**Files:**
- Modify: `lib/src/infer/mod.rs`
- Modify: `lib/src/lib.rs`
- Modify: direct callers identified by CodeGraph
- Test: existing inference and compile tests

- [ ] **Step 1: Write a failing type-level pipeline test**

Add a compile-time-shaped unit test that requires strict finalization to return accepted ownership:

```rust
fn assert_accepted(_: crate::hir::AcceptedHirProgram) {}

#[test]
fn strict_finalization_returns_accepted_hir() {
    let accepted = strict_finalize_fixture().expect("strict fixture");
    assert_accepted(accepted);
}
```

- [ ] **Step 2: Run the focused inference test and verify RED**

Run:

```text
cargo test -p rock-lib strict_finalization_returns_accepted_hir -- --exact
```

Expected: type mismatch while strict finalization still returns unresolved `HirProgram`/`ResolvedHirProgram` ownership.

- [ ] **Step 3: Change strict finalization output**

Make the strict resolved result own accepted HIR:

```rust
pub struct ResolvedHirProgram {
    pub program: AcceptedHirProgram,
    // existing resolver, ID, and type-context fields remain unchanged
}
```

Construct it only through `AcceptedHirProgram::try_from(program)` and convert validation strings to the existing structured inference diagnostics. Keep lenient debug-print paths returning unresolved HIR so diagnostics can display partial programs. Product emission serializes accepted bodies and therefore cannot encode absent selected authority.

- [ ] **Step 4: Update product emission and compile pipeline access**

Update strict consumers to use accepted aliases. Move accepted ownership into mono rather than converting back to unresolved HIR.

- [ ] **Step 5: Run focused inference and product tests**

Run:

```text
cargo test -p rock-lib strict_finalization_returns_accepted_hir -- --exact
cargo test -p rock-lib products::tests -- --nocapture
```

Expected: both commands pass.

### Task 3: Validate receiver-pattern generic authority

**Files:**
- Modify: `lib/src/hir/accepted.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Test: `lib/src/crate_artifact/load.rs`

- [ ] **Step 1: Write failing artifact tests**

Add three product-artifact tests:

```rust
#[test]
fn product_artifact_rejects_receiver_pattern_missing_impl_generic() { /* one declared T, no T binding */ }

#[test]
fn product_artifact_rejects_receiver_pattern_foreign_generic_owner() { /* binding owner != impl id */ }

#[test]
fn product_artifact_rejects_receiver_pattern_extra_impl_generic() { /* index == type_generics.len() */ }
```

Each test must assert an error containing `typed receiver pattern` and the impl product ID.

- [ ] **Step 2: Run the three tests and verify RED**

Run each fully qualified test with `cargo test -p rock-lib <path> -- --exact --nocapture`. Expected: artifact loading currently succeeds.

- [ ] **Step 3: Add reusable structural validation**

Implement a HIR helper that recursively collects `GenericParamId`s from `HirImplReceiverPattern` and compares impl-owned bindings to:

```rust
(0..imp.type_generics.len())
    .map(|index| GenericParamId { owner: imp.id, index: index as u32 })
    .collect::<BTreeSet<_>>()
```

Reject foreign generic owners in receiver patterns unless they are nested nominal type arguments that are valid in the impl declaration context. Reject missing and out-of-range impl-owned parameters. Call the same helper from accepted-HIR validation and from `validate_product_method_authority_maps` before remapping.

- [ ] **Step 4: Run artifact tests and verify GREEN**

Run the Step 2 commands. Expected: all three tests pass with the intended rejection.

### Task 4: Remove mono receiver and method-ID repair

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Test: `lib/src/mono/process.rs`

- [ ] **Step 1: Write failing mono boundary tests**

Add tests proving accepted mono input cannot contain missing Try receiver metadata and that an ordinary zero-substitution function still resolves without method fallback. Add a source audit assertion that `zero_substitution_method_instance` is absent from production code.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```text
cargo test -p rock-lib mono::process::tests -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected: the source audit fails while the compatibility helper remains.

- [ ] **Step 3: Require accepted mono input**

Change public mono entry points and `MonomorphizedProgram` to use `AcceptedHirProgram`. Update processing helpers to the accepted aliases. Method and Try branches destructure required authority directly, so there is no missing-target arm. Rewrites preserve accepted phase typing: ordinary materialized calls use `HirCallTarget::Instance`, and the only residual accepted `MethodCall` is the explicit `BuiltinIndex` marker permitted by the MIR boundary.

- [ ] **Step 4: Delete compatibility repair**

Delete `lookup_self_receiver`, `zero_substitution_method_instance`, and the method branch of `zero_substitution_callable_instance`. Preserve only exact ordinary-function lookup:

```rust
fn zero_substitution_callable_instance(&self, id: DefId) -> Option<InstanceId> {
    self.zero_substitution_function_instance(id)
}
```

Remove all writes that repair accepted `self_receiver` or `branch_self_receiver`. Treat those values as required invariants supplied by accepted HIR.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run the Step 2 commands. Expected: all pass.

### Task 5: Reject every overlapping typed impl

**Files:**
- Modify: `lib/src/mono/methods.rs`
- Test: `lib/src/mono/methods.rs`

- [ ] **Step 1: Write three failing ambiguity tests**

Build fixtures with one generic receiver pattern and one concrete receiver pattern that both match the same concrete type. Cover:

```rust
#[test]
fn drop_materialization_rejects_differently_specific_matching_impls() {}

#[test]
fn static_method_materialization_rejects_differently_specific_matching_impls() {}

#[test]
fn trait_method_materialization_rejects_differently_specific_matching_impls() {}
```

Assert a diagnostic containing `multiple matching typed implementations` and both impl IDs, and assert no call is rewritten to an instance.

- [ ] **Step 2: Run tests and verify RED**

Run each fully qualified test. Expected: the more-specific candidate currently wins.

- [ ] **Step 3: Move ambiguity checks before specificity**

For Drop, static, and trait method paths: sort and deduplicate by impl ID, diagnose when `len() > 1`, return without materialization, and remove `receiver_pattern_specificity` filtering. Remove `receiver_pattern_specificity` entirely if CodeGraph confirms no remaining caller.

- [ ] **Step 4: Run tests and verify GREEN**

Run all `mono::methods::tests`. Expected: all pass.

### Task 6: Remove stale MIR method mappings

**Files:**
- Modify: `lib/src/mir/builder/mod.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Write a failing mapping test**

Create a monomorphized fixture containing a zero-substitution function, impl method, and trait default. Assert the DefId map contains only the function origin.

- [ ] **Step 2: Run the test and verify RED**

Run the fully qualified MIR builder test. Expected: method IDs are currently present.

- [ ] **Step 3: Restrict the map to function origins**

Replace the origin match with:

```rust
if let InstanceOrigin::Function(def_id) = record.origin {
    instances.insert(def_id, record.id);
}
```

Keep the `method_def_ids` rejection guard as defense in depth.

- [ ] **Step 4: Run the test and verify GREEN**

Run the Step 2 command. Expected: pass.

### Task 7: Final residue audit and verification

**Files:**
- Modify: `lib/src/semantic_identity_audit.rs`
- Modify: `docs/superpowers/specs/2026-07-11-task-5-final-compliance-findings.md`
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md` only after every check passes

- [ ] **Step 1: Extend semantic source audits**

Assert production sources do not contain:

```text
zero_substitution_method_instance
lookup_self_receiver
receiver_pattern_specificity
InstanceOrigin::ImplMethod { method, .. } => method
InstanceOrigin::TraitDefault { method, .. } => method
```

Scope assertions carefully so tests and exact-authority `method_for_selected_target` remain allowed.

- [ ] **Step 2: Run focused quality gates**

Run serially:

```text
cargo fmt --all --check
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
```

Expected: every command exits zero with no failures.

- [ ] **Step 3: Run the full compiler suite**

Run:

```text
cargo test -p rock-lib
```

Expected: all unit, integration, auxiliary, and doc-test binaries complete with zero failures.

- [ ] **Step 4: Perform a final CodeGraph residue audit**

Query accepted-HIR creation, all mono method/static/Try/Drop paths, product/artifact authority maps, DCE, and MIR. Confirm no production method-name rediscovery, method-ID repair, optional accepted authority, or overlapping-impl winner remains.

- [ ] **Step 5: Update completion documentation**

Mark each finding in the reference document resolved with its regression test. Update Task 5 in `CLEAN_SLATE_COMPILER_AUDIT.md` only with commands and counts observed in Step 2 and Step 3; do not claim unrun checks.
