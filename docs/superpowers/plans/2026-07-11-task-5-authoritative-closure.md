# Task 5 Authoritative Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every finding in `2026-07-11-task-5-authoritative-closure-ledger.md` and make method-selection authority structurally mandatory from strict inference through products, mono, MIR, and DCE.

**Architecture:** Canonical impl receiver patterns and effective trait-method rows are created once during lowering/conformance and remain ID-keyed. Body-bearing HIR is phase-typed so unresolved recovery nodes cannot enter products or mono, while mono produces a distinct materialized program with `InstanceId` call edges and explicit generated Drop results. Selection and mono return typed failure reasons; no downstream phase reconstructs authority from names, specificity, or first-match iteration.

**Tech Stack:** Rust 2021, serde/bincode product artifacts, existing Rock HIR/type/mono/MIR pipeline, Cargo tests.

---

No VCS operations are part of this plan. Current repository instructions prohibit commits unless the user explicitly requests them.

## File Structure

- Modify `lib/src/hir/mod.rs`: phase-parameterized body nodes, canonical impl receiver field, accepted/materialized call shapes, and non-reconstructing indexes.
- Rewrite `lib/src/hir/accepted.rs`: strict recursive unresolved-to-accepted conversion and typed validation errors.
- Modify `lib/src/lower/collect/traits.rs`: construct canonical receiver patterns with impl-owned generic IDs.
- Modify `lib/src/lower/traits/conformance.rs`: produce complete effective trait-member rows explicitly.
- Modify `lib/src/selection/{types,service,matching}.rs`: typed outcomes, no specificity, and canonical receiver/projection matching.
- Modify `lib/src/lower/{mod,expression,paths,resolution,bodies}.rs` and `lib/src/lower/control_flow/secondary.rs`: persist authority immediately and delete name/deferred repair.
- Modify `lib/src/infer/mod.rs`: strict accepted output only; lenient output remains unresolved.
- Modify `lib/src/type_services/projection.rs`: canonical receiver-pattern projection keys and ambiguity results.
- Modify `lib/src/products.rs` and `lib/src/products/type_table.rs`: accepted pre-mono bodies only; no serialized `InstanceId`.
- Modify `lib/src/crate_artifact/{types,load}.rs` and `lib/src/crate_system/extern_store.rs`: exact validation and accepted external providers.
- Modify `lib/src/mono/{mod,process,methods,external,registry}.rs`: accepted input, typed errors, exact applicability, and explicit Drop results.
- Modify `lib/src/mir/builder/{mod,expr,blocks}.rs`: consume materialized calls and Drop results without rediscovery.
- Modify `lib/src/dce.rs`: identical production/test function-only reachability.
- Modify focused unit/integration tests and remove obsolete Task 5 source-string audits.

## Finding Coverage

| Tasks | Findings |
| --- | --- |
| 1 | F8, F9, F10, F16, F22, F23, F39 |
| 2 | F1, F15, F17, F25, F27 |
| 3 | F24, F33, F34, F35, F36, F37, F38 |
| 4 | F2, F3, F4, F11, F21, F29, F30, F31 |
| 5 | F5, F8, F13, F17, F29, F31, F32 |
| 6 | F6, F7, F12, F14, F26 |
| 7 | F19, F20, F27, F32 |
| 8 | F13, F14, F18, F28 and all final gates |

### Task 1: Establish canonical impl authority

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/lower/bodies.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Test: the modules above

- [ ] **Step 1: Add failing canonical-authority tests**

Add tests proving that index rebuild does not create receiver/effective rows,
receiver patterns reject missing/extra/foreign bindings while accepting repeated
uses of one generic, and body attachment identifies impls by `DefId` rather than
`receiver_arg_types` or display names.

```rust
#[test]
fn rebuilding_indexes_does_not_reconstruct_impl_authority() {
    let mut program = program_with_trait_impl_but_no_authority_rows();
    program.rebuild_indexes();
    assert!(program.indexes.impl_receiver_patterns.is_empty());
    assert!(program.indexes.effective_trait_methods.is_empty());
}

#[test]
fn receiver_pattern_requires_every_declared_receiver_generic() {
    let (imp, missing_pattern) = missing_receiver_generic_fixture();
    assert!(validate_impl_receiver_pattern_bindings(&imp, &missing_pattern).is_err());
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run `cargo test -p rock-lib hir::tests::rebuilding_indexes_does_not_reconstruct_impl_authority -- --exact --nocapture` and the new lowering tests. Expected: reconstructed maps remain populated or duplicate coverage is accepted.

- [ ] **Step 3: Make receiver authority canonical**

Store the typed pattern directly with each impl and remove `receiver_arg_types` as semantic state:

```rust
pub struct HirImpl {
    pub id: DefId,
    pub owner: HirImplOwner,
    pub receiver_pattern: HirImplReceiverPattern,
    // existing trait args, bounds, associated types, and methods
}
```

Construct it once from impl syntax after allocating `imp.id`. Remove `nominal_owner_id_from_names`, `impl_source_owner_type`, method-parameter fallbacks, and receiver-argument matching. Projection/product views derive from `receiver_pattern`.

- [ ] **Step 4: Make effective method rows conformance output**

Give lowering a single map containing current and imported rows:

```rust
pub(crate) effective_trait_methods: HashMap<(DefId, DefId), DefId>,
```

When conformance accepts an override or injects a default body, insert exactly one `(impl.id, trait_member.id) -> impl_local_body.id` row. Reject conflicting inserts. Delete row creation from `HirDefinitionIndexes::from_parts`; index rebuild only preserves supplied rows.

- [ ] **Step 5: Run canonical-authority tests and verify GREEN**

Run `cargo test -p rock-lib hir -- --nocapture`, `cargo test -p rock-lib lower::traits -- --nocapture`, and `cargo test -p rock-lib lower::bodies -- --nocapture`. Expected: all pass.

### Task 2: Make selection coherent and diagnostic

**Files:**
- Modify: `lib/src/selection/types.rs`
- Modify: `lib/src/selection/service.rs`
- Modify: `lib/src/selection/matching.rs`
- Modify: `lib/src/type_services/projection.rs`
- Test: the modules above

- [ ] **Step 1: Add failing overlap and malformed-authority tests**

Cover concrete/generic overlap for ordinary, static, index, bound, and projection selection; missing effective rows; extra trait arguments; and ordering independence.

```rust
#[test]
fn differently_specific_impls_are_ambiguous() {
    let outcome = service.select_concrete_method(&[receiver_i64()], "value", Type::clone);
    assert!(matches!(outcome, SelectionOutcome::Ambiguous { candidates } if candidates.len() == 2));
}

#[test]
fn projection_overlap_is_ambiguous() {
    assert!(matches!(provider.resolve_projection(key()), Err(SelectionDiagnostic::AmbiguousCandidates { .. })));
}
```

- [ ] **Step 2: Run selection tests and verify RED**

Run `cargo test -p rock-lib selection -- --nocapture`. Expected: specificity chooses a winner or ambiguity is erased as `None`.

- [ ] **Step 3: Introduce a typed selection outcome**

Use one result shape for required selection:

```rust
pub enum SelectionOutcome<T> {
    Selected(T),
    NoMatch,
    Ambiguous { candidates: Vec<HirMethodCallTarget> },
    Invalid(SelectionDiagnostic),
}
```

Delete `impl_selection_priority` and `unique_most_specific_impl`. For each receiver-adjustment level, sort/deduplicate exact candidate IDs and return ambiguity when more than one remains. Require complete effective rows and exact trait-argument arity.

- [ ] **Step 4: Move projections to canonical patterns**

Replace `ProjectionImpl.receiver_arg_types` with `receiver_pattern`. Resolve the full typed key and return an ambiguity error rather than `find_map`:

```rust
pub struct ProjectionImpl {
    pub impl_id: DefId,
    pub receiver_pattern: HirImplReceiverPattern,
    pub trait_arg_types: Vec<Type>,
    pub associated_types: Vec<ProjectionAssociatedType>,
}
```

- [ ] **Step 5: Run selection/projection tests and verify GREEN**

Run `cargo test -p rock-lib selection -- --nocapture` and `cargo test -p rock-lib type_services::projection -- --nocapture`. Expected: all pass.

### Task 3: Persist authority during lowering

**Files:**
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/expression.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/lower/resolution.rs`
- Modify: `lib/src/lower/types_helpers/helpers.rs`
- Modify: `lib/src/lower/types_helpers/type_vars.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/infer/mod.rs`
- Test: nearby lowering tests and `lib/tests/integration.rs`

- [ ] **Step 1: Add failing lowering tests**

Add tests for direct dot calls, method values, qualified static calls/values, two matching trait bounds, operator candidate reorder, generated Deref authority, and same-named current traits from two crates. Assert exact IDs are present immediately after lowering.

- [ ] **Step 2: Run focused tests and verify RED**

Run each new fully-qualified test. Expected failures include targetless `FieldAccess`, first-bound selection, or display-name-selected methods.

- [ ] **Step 3: Delete deferred method reselection**

Make dot lowering emit either an ID-backed field or a selected call/lambda immediately. Delete `materialize_deferred_method_calls`, `deferred_receiver_candidates`, and the field-call rewrite in `resolve_all_types_in_expr`.

```rust
match selection {
    SelectionOutcome::Selected(selected) => lower_selected_method_call(selected, args),
    SelectionOutcome::NoMatch => lower_resolved_field_call(field_location, receiver, args),
    SelectionOutcome::Ambiguous { candidates } => error_expr(report_ambiguity(candidates)),
    SelectionOutcome::Invalid(error) => error_expr(report_selection(error)),
}
```

`lower_resolved_field_call` requires `Some(HirFieldLocation)` and reports an
unknown-field diagnostic when `field_location` is `None`; it never creates a
targetless method channel.

- [ ] **Step 4: Replace static/name/operator shortcuts**

Resolve qualified owners to canonical IDs, invoke selection, and preserve its target. Delete static alias scanning, `Type::Unit` owner fallback, generic-name binding repair, first-bound return, and first-impl operator inference. Carry `current_trait: Option<DefId>`. Feed generated Deref the canonical protocol authority supplied by operator/protocol resolution, not `"Deref"`.

- [ ] **Step 5: Run lowering and integration tests**

Run `cargo test -p rock-lib lower -- --nocapture` followed by the method/operator/index/Try integration anchors from the design. Expected: all pass.

### Task 4: Implement structural accepted HIR

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Rewrite: `lib/src/hir/accepted.rs`
- Modify: `lib/src/infer/mod.rs`
- Modify: `lib/src/lib.rs`
- Test: `lib/src/hir/accepted.rs`

- [ ] **Step 1: Add failing accepted-phase tests**

Test every invalid state from F2, F3, F4, F11, F21, F29, F30, and F31, including trait-default bodies and local `InstanceId` edges.

- [ ] **Step 2: Run accepted tests and verify RED**

Run `cargo test -p rock-lib hir::accepted -- --nocapture`. Expected: invalid programs currently construct the wrapper.

- [ ] **Step 3: Parameterize body-bearing HIR by phase**

Use sealed phase-associated authority types:

```rust
pub trait HirPhase: sealed::Sealed {
    type MethodTarget;
    type StaticTarget;
    type TryAuthority;
    type ExecutableCallTarget;
}

pub enum UnresolvedHir {}
pub enum AcceptedHir {}
pub enum MaterializedHir {}

pub type HirProgram = HirProgramFor<UnresolvedHir>;
pub type AcceptedHirProgram = HirProgramFor<AcceptedHir>;
pub type MaterializedHirProgram = HirProgramFor<MaterializedHir>;
```

Accepted method/field/Try authority is non-optional. Accepted executable targets contain ordinary function/extern/local/intrinsic/static selections but no `InstanceId`; materialized targets contain direct instances and explicit builtin indexing.

- [ ] **Step 4: Implement exhaustive strict conversion**

Recursively move every function, impl method, trait default, block, statement, pattern, and expression. Validate every stored type with owner-aware generic scopes, exact target/call shape, field locations, trait args, effective rows, receiver modes, and absence of `TypeVar`, `Type::Error`, and pre-mono instances. Return typed acceptance errors converted to `ResolveError`.

- [ ] **Step 5: Separate lenient output**

Change `finalize_lenient` to return an explicitly unresolved debug/recovery result. Only `finalize` constructs `AcceptedHirProgram`. Delete unchecked accepted constructors and `DerefMut`.

- [ ] **Step 6: Run accepted and inference tests**

Run `cargo test -p rock-lib hir::accepted -- --nocapture` and `cargo test -p rock-lib infer -- --nocapture`. Expected: all pass.

### Task 5: Make products and artifacts accepted-only

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/products/type_table.rs`
- Modify: `lib/src/crate_artifact/types.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: `lib/src/crate_system/extern_store.rs`
- Modify: `lib/src/mono/external.rs`
- Test: product/artifact modules

- [ ] **Step 1: Add failing malformed-artifact tests**

Cover missing/extra/duplicate receiver bindings, malformed trait args/call shapes, trait-default bodies, `Type::Error`, local instances, incomplete Drop language items, and bodies that bypass accepted conversion.

- [ ] **Step 2: Run artifact tests and verify RED**

Run the new fully-qualified tests. Expected: malformed rows currently load or are classified concrete.

- [ ] **Step 3: Serialize accepted pre-mono shapes**

Remove serialized `Instance` targets and optional accepted method/Try authorities. Encode canonical receiver patterns and effective rows exactly once. Keep artifact format `37` as required by the approved design.

- [ ] **Step 4: Validate before provider registration**

Decode dependency bodies into unresolved transport values, run the same strict accepted conversion with dependency interfaces in scope, and store accepted function/trait/impl body providers keyed by `DefId`. Reject `Type::Error` in every concreteness helper and require complete Drop language-item pairs.

- [ ] **Step 5: Remove test-only product fallbacks**

Delete `new_unchecked_for_test`, `Exact(Unit)` receiver fallback, and fixtures that mutate accepted bodies. Build fixtures through production conversion.

- [ ] **Step 6: Run product and artifact suites**

Run `cargo test -p rock-lib products -- --nocapture` and `cargo test -p rock-lib crate_artifact -- --nocapture`. Expected: all pass.

### Task 6: Make mono exact and structurally fallible

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mono/registry.rs`
- Test: mono modules

- [ ] **Step 1: Add failing mono tests**

Cover exact receiver mismatch, exact trait-arg mismatch, static zero-match/missing-body errors, standalone accepted input, object-backed missing records, and static/trait/Drop overlap.

- [ ] **Step 2: Run mono tests and verify RED**

Run `cargo test -p rock-lib mono -- --nocapture`. Expected: exact selected targets skip applicability or failures collapse into `Option`.

- [ ] **Step 3: Add typed mono errors**

```rust
pub enum MonoErrorKind {
    MissingImpl { impl_id: DefId },
    MissingMethod { impl_id: DefId, method_id: DefId },
    ReceiverMismatch { impl_id: DefId, receiver: Type },
    InvalidBindings { target: HirSelectedMethodTarget },
    NoMatchingImpl { trait_id: DefId, member_id: DefId },
    AmbiguousImpls { trait_id: DefId, member_id: DefId, impls: Vec<DefId> },
    MissingEffectiveMethod { impl_id: DefId, member_id: DefId },
    MissingInstance { origin: InstanceOrigin },
}
```

Each required materializer returns `Result<MaterializedCall, MonoError>` with
the source/generated span. Convert errors to diagnostics only at the public
boundary.

- [ ] **Step 4: Consume accepted authority and produce materialized HIR**

Both public mono entry points accept accepted HIR. Exact impl paths validate canonical receiver substitution and selected trait args before registry lookup. Trait/static paths gather all distinct matches and reject zero/multiple. Successful calls become `MaterializedHirCallTarget::Instance`; no semantic method target remains except explicit builtin indexing.

- [ ] **Step 5: Run mono tests and post-materialization validator**

Run `cargo test -p rock-lib mono -- --nocapture`. Expected: all pass and every produced body validates as materialized.

### Task 7: Carry generated Drop results into MIR

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/blocks.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: mono and MIR builder modules

- [ ] **Step 1: Add failing Drop/MIR and builtin-index tests**

Test exact effective Drop member identity, origin spans, ambiguous Drop, no-impl probes, explicit type-to-instance mapping, and rejection of targetless name-based index nodes.

- [ ] **Step 2: Run focused tests and verify RED**

Run new Drop tests and `cargo test -p rock-lib mir::builder -- --nocapture`. Expected: MIR reconstructs Drop or accepts targetless `"index"`.

- [ ] **Step 3: Record generated Drop results in mono**

```rust
pub struct GeneratedMethodInstance {
    pub receiver_ty: TypeId,
    pub trait_id: DefId,
    pub member_id: DefId,
    pub instance_id: InstanceId,
    pub origin_span: Option<Span>,
}
```

Deduplicate by the full canonical key. Source parameter/local/expression probes pass `Some(span)`; recursive/global probes pass `None`. Require valid paired Drop language items and exact effective rows.

- [ ] **Step 4: Delete MIR Drop and builtin rediscovery**

Build runtime roots, direct-drop sets, and backend `drop_glue` directly from mono's generated instances. Delete `drop_glue_callable_key_for_type`, Drop impl scans, method-parameter matching, and targetless `method_name == "index"` recognition.

- [ ] **Step 5: Run mono/MIR/borrowck/codegen tests**

Run `cargo test -p rock-lib mono -- --nocapture`, `cargo test -p rock-lib mir::builder -- --nocapture`, `cargo test -p rock-lib mir::borrowck -- --nocapture`, and `cargo test -p rock-lib codegen -- --nocapture`. Expected: all pass.

### Task 8: Align DCE/tests and close the ledger

**Files:**
- Modify: `lib/src/dce.rs`
- Modify: `lib/src/semantic_identity_audit.rs`
- Modify: `lib/tests/integration.rs`
- Modify: `docs/superpowers/specs/2026-07-11-task-5-authoritative-closure-ledger.md`
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md` only after every gate passes

- [ ] **Step 1: Replace legacy DCE fixtures**

Delete test-only method-ID/name reachability indexes and helpers. Construct tests with realistic `MirCallableKey::Instance` edges. Production and test builds use the same function-only DefId map.

- [ ] **Step 2: Replace source-string assertions**

Remove Task 5 absence-string tests after equivalent typed and behavioral tests exist. Keep only audits that validate serialized/public contracts not expressible in Rust types.

- [ ] **Step 3: Run all focused suites serially**

Run the selection, lower, mono, MIR, agreement, products, artifact, DCE, semantic audit, and integration commands listed in the design. Expected: every command exits zero.

- [ ] **Step 4: Run full quality gates**

Run serially:

```text
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
cargo clippy -p rock-lib --all-targets
```

Expected: formatting/diff/tests exit zero; clippy completes without errors, with any repository-preexisting warnings reported accurately.

- [ ] **Step 5: Perform final whole-codebase CodeGraph audit**

Re-run the full requirements-to-code trace across collection, conformance, selection, lowering, inference, accepted HIR, products, artifacts, mono, MIR, DCE, and codegen. Search for optional accepted authority, `receiver_arg_types`, specificity, deferred selection, name/alias method lookup, serialized instances, Drop rediscovery, targetless index recognition, and method-ID reachability.

- [ ] **Step 6: Close findings with evidence**

For each F1-F39, record exact source symbols and passing regression commands in the ledger. Append any newly discovered Task 5 issue before fixing it. Only after every row and gate is closed may `CLEAN_SLATE_COMPILER_AUDIT.md` mark Task 5 complete.
