# Task 5 Authoritative Closure Ledger

## Status

Task 5 is complete as of 2026-07-15. F1-F65 are closed and the final gates were
rerun after F65. This file is the single authoritative closure ledger. Future
regressions must be added here before they are fixed.

This ledger supersedes completion claims in
`CLEAN_SLATE_COMPILER_AUDIT.md` and the narrower findings list in
`2026-07-11-task-5-final-compliance-findings.md`.

## Implementation Checkpoint

The following table records each finding's status and supporting evidence.
F1-F64 `closed` entries preserve historical 2026-07-14 closure evidence. F65
records the 2026-07-15 follow-up closure. A `closed` entry means the finding's
implementation, focused regression, and applicable whole-suite coverage were
complete.

| Finding | Status | Per-finding evidence |
| --- | --- | --- |
| F1 | closed | Frontend ordinary/static/trait/index selection no longer ranks typed overlaps by specificity; differently-specific trait overlap and direct ambiguity regressions pass. |
| F2 | closed | `HirPhase` makes method, `Try`, and impl-receiver authority phase-selected; recursive conversion produces `HirProgramFor<AcceptedHir>` whose executable nodes cannot represent missing accepted authority. |
| F3 | closed | `AcceptedHirProgram::try_from` now recursively rejects `TypeVar`/`Type::Error` in functions, bodies, patterns, declarations, impl metadata, bounds, receiver patterns, and extern signatures. |
| F4 | closed | Lenient finalization returns raw debug/recovery `HirProgram`, never `ResolvedHirProgram` or accepted HIR. |
| F5 | closed | External provider maps expose only `AcceptedHirFunction`, `AcceptedHirTrait`, and `AcceptedHirImpl`; source/artifact reconstruction validates before provider registration. |
| F6 | closed | Exact standalone/static/trait targets validate canonical receiver applicability and trait references before instance creation; failures return source-spanned `MonoError` values, including object-backed paths. |
| F7 | closed | Static mono materialization returns source-spanned `Result` with distinct missing impl/method, receiver, trait-argument, binding, ambiguity, no-match, and missing-instance causes; callers record the typed failure and never silently retain the target. |
| F8 | closed | HIR and artifact receiver-pattern validation now require exact generic-binding set equality; repeated occurrences remain set-equal. |
| F9 | closed | `HirImpl.receiver_pattern` is the sole canonical impl receiver authority across lowering, accepted HIR, products, artifacts, selection, projection, mono, and MIR; all receiver sidecars are deleted. |
| F10 | closed | Production `SelectionService::new` starts without synthesized receiver patterns; callers must supply canonical authority. Test-only isolated selection fixtures retain local setup pending their migration under F13 coverage. |
| F11 | closed | Accepted conversion rejects every ordinary `FieldAccess(_, _, None)`. |
| F12 | closed | Both mono entry points require `AcceptedHirProgram` and return the complete `MonomorphizedProgram` materialization contract. |
| F13 | closed | `new_unchecked_for_test`, accepted `DerefMut`, and product `Exact(Unit)` fallback are deleted; product fixtures mutate clones and re-enter production conversion. |
| F14 | closed | Frontend, static mono, trait mono, Drop, and materialization regressions reject differently-specific overlaps with deterministic candidate IDs and no emitted instance. |
| F15 | closed | Production selection with an effective-method map requires the exact `(impl, member) -> body` row and never substitutes the member ID. |
| F16 | closed | Method-parameter receiver-pattern reconstruction was removed from lowering. |
| F17 | closed | Accepted and artifact boundaries validate target kind, trait/member/impl membership, trait arity, selected-vs-impl trait arguments, receiver applicability, substitutions, and builtin-index shape with malformed-row regressions. |
| F18 | closed | Test and production DCE now share the same function-only instance reachability indexes; test rediscovery helpers are deleted. |
| F19 | closed | Mono records exact generated Drop instances by concrete `TypeId` with trait/member/instance identity and origin span; MIR roots, direct-drop sets, cleanup, backend drop glue, and link classification consume that map without impl/projection/parameter rediscovery. Focused mono Drop and all 89 MIR builder tests pass. |
| F20 | closed | MIR targetless string-based builtin-index recognition is deleted. |
| F21 | closed | Trait default bodies are traversed by accepted authority/type validation with a focused targetless-call regression. |
| F22 | closed | Index rebuild no longer creates effective-method rows. |
| F23 | closed | Index rebuild no longer creates receiver authority or resolves nominal owners from display aliases. |
| F24 | closed | Targeted calls are never reselected. Calls that cannot be selected while lowering remain explicitly unresolved and are selected once by the inference-owned authority materializer: a permissive pre-generalization pass feeds unique selections into inference, and a strict post-finalization pass rejects all residue while skipping already-authoritative calls. |
| F25 | closed | Required selection APIs return `Result<_, SelectionDiagnostic>` with deterministic candidate IDs; `Option`/`Vec` remain only for explicitly named probe/candidate enumeration APIs. |
| F26 | closed | Required static, standalone, and trait materializers return source-spanned `Result<_, MonoError>` with structured missing impl/method, receiver, trait-argument, binding, no-match, ambiguity, effective-method, and missing-instance payloads; diagnostics are created at processing boundaries. |
| F27 | closed | Accepted validation rejects ambiguous concrete projections and MIR projection metadata resolves only one unique impl ID; no projection consumer selects the first matching impl. |
| F28 | closed | Task 5 DCE/method-repair absence-string assertions were removed after production-edge DCE, accepted-boundary, and mono behavioral coverage replaced them; serialized/public contract audits remain. |
| F29 | closed | Accepted conversion, product remapping, artifact loading, and accepted provider types reject `TypeVar`/`Type::Error` throughout executable bodies and impl receiver metadata. |
| F30 | closed | Accepted and artifact validators reject static/receiver shape swaps, cross-impl/cross-trait method IDs, missing selected-trait identity, malformed substitutions/trait arguments, and builtin-index authority mutations. |
| F31 | closed | Accepted conversion and artifact remapping reject local instance targets, `Try.branch_target` is never encoded, and product call/variable schemas no longer contain an `InstanceId` variant. |
| F32 | closed | Drop trait/member language items must be a complete, declared pair during current-product construction and artifact loading. |
| F33 | closed | Static call-target construction requires the canonical impl receiver pattern and complete substitutions; nominal display-name reconstruction and `Unit` fallback are deleted. Generic-bound targets retain exact generic/trait/member IDs. |
| F34 | closed | Static bound lookup gathers and deduplicates exact trait/member targets and succeeds only for a unique candidate; bound order cannot select a winner. |
| F35 | closed | Unresolved binary/unary inference returns deterministic ID-bearing ambiguity diagnostics; concrete operator filtering is unique and service-owned, with no first-compatible winner. |
| F36 | closed | Qualified static lookup resolves the owner nominal `DefId`, matches impls by canonical receiver-pattern root, and requires one exact impl/method; alias/name method scans are deleted. |
| F37 | closed | Generated autoderef resolves the zero-argument `*` operator protocol through normal selection, persists exact impl/trait/member IDs, and contains no compiler lookup for a trait named `Deref`. |
| F38 | closed | Current-trait selection carries and resolves the exact current trait `DefId`. |
| F39 | closed | Production session preparation imports dependency effective-method authority while receiver authority travels on structural dependency impls; cross-crate method tests pass without index reconstruction. |
| F40 | closed | Exact-applicability hardening exposed five stale mono fixtures without canonical receiver patterns; fixtures now supply exact nominal/slice patterns and all 17 mono method tests pass. |
| F41 | closed | Five additional external/process fixtures now supply canonical receiver authority; all 102 mono-filtered tests pass. |
| F42 | closed | The apparent receiver mismatch was exact trait-argument validation incorrectly treating legacy inherent receiver arguments as trait arguments. Inherent targets now skip trait-reference checks, static failures remain typed, bare-generic receiver validation is consistent across HIR/artifacts, and the triggering integration test passes. |
| F43 | closed | Empty-bang partial method application previously returned an unresolved `FieldAccess`; it now builds an authority-bearing method-value lambda, and all 20 secondary-lowering tests pass. |
| F44 | closed | Four qualified-static path fixtures now register exact impl IDs and canonical receiver patterns; all 33 lower-path tests pass and stale name-map collisions do not affect selection. |
| F45 | closed | Artifact callable validation now checks exact target kind/`DefId` only; display names and ambiguity are non-semantic, related name indexes were removed from the validator, and all 131 artifact-filtered tests pass. |
| F46 | closed | Removing first-match behavior exposed 35 regressions. Underconstrained operators now choose a unique typed candidate or the explicit `I64` numeric default, literal defaulting is receiver-local, selected index outputs apply current substitutions, and all 482 integration tests pass after the related fixes. |
| F47 | closed | Type-variable field access no longer selects the first display-name match. Lowering accepts only one current-crate nominal owner, records its exact `DefId`/`FieldId`, and strict materialization resolves only concrete nominal IDs. |
| F48 | closed | Call-site method generics are instantiated with fresh inference variables before receiver/argument unification; rigid `Type::Generic` identities no longer leak into concrete method-section chains. |
| F49 | closed | Closure environments now preserve `Move`/`SharedBorrow`/`MutableBorrow`: borrowed captures store addresses and use reference-typed capture locals, while escaping synthetic curried/method-value closures move their captures. |
| F50 | closed | Intrinsic range HIR no longer stores `Type::Error`; its non-first-class expression result is `Unit`, while loop element typing remains explicit `I64`. |
| F51 | closed | Dependency trait-default provider bodies are instantiated with the selected impl's exact `Self` and trait arguments before replacing downstream stubs; source-free artifact coverage exercises a default-to-default call. |
| F52 | closed | Unchecked accepted constructors/mutators are deleted; the sole test rebuild helper first converts to unresolved HIR and re-enters production `AcceptedHirProgram::try_from`. |
| F53 | closed | Accepted and artifact validation now bind impl generics across the canonical receiver and selected trait arguments while still rejecting missing or foreign generic IDs. |
| F54 | closed | Trait-backed candidate construction now requires the exact declared trait-member ID; missing members yield no selection, and all selection fixtures carry distinct canonical member IDs. |
| F55 | closed | Two mono receiver-identity regressions mutated unrelated monomorphizer state while passing an impl with a bare-generic receiver; fixtures now put the exact nominal receiver pattern on the impl under test. |
| F56 | closed | Projection substitution is checked for receiver applicability, exact trait-argument arity, and argument compatibility; malformed provider metadata leaves the projection unresolved. |
| F57 | closed | Concrete and generic-bound static method values lower to synthetic closures whose body carries exact `HirStaticMethodTarget` authority; name-based call-target recovery is deleted. |
| F58 | closed | Concrete static lookup now requires `!method.is_method`; receiver methods cannot enter static path or target construction based on missing receiver-mode metadata. |
| F59 | closed | All stale subsystem fixtures now store and assert exact canonical receiver patterns; collection, HIR, inference, body-lowering, trait-collection, and conformance suites pass. |
| F60 | closed | The layout provider now returns receiver and trait-argument patterns matching the requested projection; layout, projection, and complete suites pass. |
| F61 | closed | Impl owner collection routes bare slices through the shared type-lowering policy; the rejection regression and complete integration suite pass. |
| F62 | closed | Generated defaults preserve the trait member's static/receiver declaration shape; focused conformance and selfless-associated-default integration coverage pass. |
| F63 | closed | Body attachment lowers AST identity in each candidate impl's canonical generic context, so the blanket `Into` body is retained and invokes the concrete `From<Small> for Big`; focused and complete suites pass. |
| F64 | closed | Try lowering maps structured no-implementation failures to stable Try/FromResidual diagnostics without masking ambiguity or malformed-authority diagnostics; both regressions and complete suites pass. |
| F65 | closed | Qualified static-method resolution returns exact impl, method, optional trait/member, receiver-pattern, and generic-parameter authority. Lowering instantiates that authority by canonical `GenericParamId`, including structural owner-to-method relations and selected trait arguments, without an impl rescan or display-name correlation; `static_method_target_for_callee` is deleted. |

## Required Architecture

- Lowering and inference may use unresolved HIR with optional recovery fields.
- Strict inference must convert unresolved HIR into structurally distinct
  accepted executable HIR.
- Accepted method calls, static calls, method values, and `Try` protocol calls
  must own required authority fields. Their invalid optional states must be
  unrepresentable.
- Products and dependency body providers must contain validated accepted bodies,
  not unchecked unresolved HIR.
- Mono must consume only accepted authority, validate exact typed applicability,
  diagnose every failed required materialization, and produce `InstanceId`
  executable edges.
- MIR and production DCE must consume only materialized instance edges and must
  not reconstruct method selection.
- Distinct overlapping typed impl matches are ambiguous. Task 5 introduces no
  implicit specialization or specificity preference.

## Open Findings

### F1. Frontend selection performs implicit specialization

`SelectionService::select_concrete_method` selects the first sorted candidate.
`impl_selection_priority` prefers inherent and structurally specific impls over
generic impls. The same priority is used by static trait selection,
`unique_most_specific_impl`, and index selection. Overlapping generic and
concrete impls can therefore be selected instead of diagnosed as ambiguous
before mono.

Relevant code:

- `lib/src/selection/service.rs`, `select_concrete_method`
- `lib/src/selection/service.rs`, `select_concrete_method_candidates`
- `lib/src/selection/service.rs`, `impl_selection_priority`
- `lib/src/selection/service.rs`, `select_static_trait_method`
- `lib/src/selection/service.rs`, `unique_most_specific_impl`
- `lib/src/selection/service.rs`, `select_index_method`

Closure evidence:

- Selection gathers all distinct typed matches for the winning receiver
  adjustment and diagnoses more than one match.
- No specificity score resolves generic/concrete overlap.
- Projection and bound-method selection use the same ambiguity rule.
- Direct selection and integration regressions cover differently specific
  overlaps.

### F2. Accepted HIR is only a wrapper around unresolved HIR

`AcceptedHirProgram` owns the same `HirProgram` whose method, call, field, and
`Try` authority fields remain optional. `into_program` returns that unresolved
shape to mono. This does not implement the approved phase-typed executable HIR
and does not make invalid accepted states unrepresentable.

Relevant code:

- `lib/src/hir/accepted.rs`
- `lib/src/hir/mod.rs`, `HirExprKind`
- `lib/src/mono/mod.rs`, `monomorphize_with_crates`

Closure evidence:

- Accepted expression/function/block/program types are structurally distinct or
  phase-parameterized.
- Required method and `Try` authorities are non-optional in accepted nodes.
- Mono cannot receive unresolved executable nodes through its type signature.
- No unchecked accepted constructor exists in production or general test
  fixtures.

### F3. Accepted conversion does not validate all HIR types

`AcceptedHirProgram::try_from` runs only method-authority validation. It does not
reject `Type::Error` or disallowed `TypeVar` values across expression types,
function signatures, locals, blocks, fields, patterns, impl arguments, bounds,
or projections.

Relevant code:

- `lib/src/hir/accepted.rs`, `TryFrom<HirProgram>`
- `lib/src/hir/mod.rs`, `validate_method_authorities`
- `lib/src/hir/type_ids.rs`

Closure evidence:

- Strict conversion recursively validates every stored type location.
- Numeric defaulting remains intentional and phase-owned.
- Focused tests reject unresolved/error types in signatures, bodies, patterns,
  impl metadata, and method authority.

### F4. Lenient finalization produces accepted executable HIR

`finalize_lenient` defaults unresolved type variables and returns
`ResolvedHirProgram` containing `AcceptedHirProgram`. Error-recovery HIR must not
enter products, mono, or MIR.

Relevant code:

- `lib/src/infer/mod.rs`, `finalize_lenient`
- `lib/src/infer/finalize.rs`, lenient finalization helpers

Closure evidence:

- Lenient finalization returns an explicitly unresolved/debug-only type or is
  removed.
- Only strict finalization can construct accepted executable HIR.

### F5. Artifact bodies bypass accepted-HIR validation

The artifact loader validates product rows, relation maps, and remapped child
IDs, then stores raw `HirFunction`, `HirTrait`, and `HirImpl` bodies in
`ExternBodyProviders`. Mono loads those bodies directly without accepted-HIR
conversion. Missing method targets, malformed `Try` authority, and unresolved
types can bypass the current-crate accepted boundary.

Relevant code:

- `lib/src/crate_artifact/load.rs`, `cross_crate_hir_from_products`
- `lib/src/crate_system/extern_store.rs`, `ExternBodyProviders`
- `lib/src/mono/external.rs`, `load_external_generic_functions`

Closure evidence:

- Loaded executable bodies are converted to accepted body types before storage.
- Artifact load fails before registration for every malformed authority/type
  case.
- Mono external providers expose only accepted bodies.

### F6. Exact selected impl targets skip receiver applicability validation

Standalone impl-method materialization loads an exact impl/method by ID but does
not validate the call receiver against the impl receiver pattern. Static and
trait materialization explicitly skip receiver-pattern and trait-argument checks
when the target already names an impl.

Relevant code:

- `lib/src/mono/methods.rs`, `monomorphize_standalone_method_call`
- `lib/src/mono/methods.rs`, `monomorphize_static_method_call`
- `lib/src/mono/methods.rs`, `monomorphize_trait_method_call`

Closure evidence:

- Every exact impl target validates structural receiver applicability and trait
  arguments before instance creation or reuse.
- Mismatches produce structured mono diagnostics at the source span.
- Tests cover standalone, static, trait, and object-backed exact targets.

### F7. Static materialization loses the specific failure cause

`monomorphize_static_method_call` returns `None` for zero candidates, missing
methods, incomplete substitutions, and missing bodies without recording the
specific failure. `process_expr` retains the unresolved static target. The final
`validate_materialized_body_edges` check prevents successful mono for registered
executable bodies, but emits only a generic post-monomorphization error and does
not identify the missing candidate, method, substitution, or body.

Relevant code:

- `lib/src/mono/methods.rs`, `monomorphize_static_method_call`
- `lib/src/mono/process.rs`, static call processing

Closure evidence:

- Required static materialization returns `Result`, not a reason-erasing
  `Option`.
- Every failure adds a structured, source-spanned diagnostic and prevents
  `MonomorphizedProgram` construction.
- Post-mono validation rejects every remaining semantic static/method target.

### F8. Receiver-pattern generic validation is not exact

HIR and product validation collect generic parameters into `HashSet`s and accept
`expected.is_subset(actual)`, where `expected` is itself derived from the legacy
`receiver_arg_types`. Extra owner-valid receiver bindings can therefore be
accepted. Repeated occurrences of the same generic in a structural pattern are
valid and enforce equality; duplicate substitution rows, not repeated
type-pattern occurrences, are invalid. Generics used only by the trait reference
are not required in the receiver pattern.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_impl_receiver_pattern_bindings`
- `lib/src/crate_artifact/load.rs`, `validate_product_method_authority_maps`

Closure evidence:

- Validation checks exact owner-qualified declared receiver-binding set coverage.
- Missing, extra, foreign-owner, and out-of-range cases have direct HIR and
  artifact regressions; repeated-generic patterns remain accepted.

### F9. `receiver_arg_types` remains a second semantic receiver authority

Validation derives expected receiver bindings from `HirImpl.receiver_arg_types`,
selection seeds substitutions from it, body lowering uses it to identify impl
slots, conformance rebuilds `Self` from it and display names, and
`ProjectionImpl::projection_substitution` zips it against receiver arguments.
It can drift from the canonical typed receiver pattern throughout the pipeline.

Relevant code:

- `lib/src/hir/mod.rs`, `HirImpl`
- `lib/src/lower/bodies.rs`, `impl_slot_matches_ast_identity`
- `lib/src/lower/traits/conformance.rs`, generated-default `Self` construction
- `lib/src/selection/matching.rs`, `seed_receiver_substitution_from_impl`
- `lib/src/type_services/projection.rs`, `ProjectionImpl`
- product interface and type-table serialization

Closure evidence:

- `receiver_arg_types` is deleted or mechanically derived from the canonical
  receiver pattern.
- No selection, artifact, or mono path treats it as independent authority.
- Body attachment, conformance, projection normalization, MIR metadata, and
  products consume the canonical receiver pattern instead.

### F10. Selection reconstructs receiver patterns from names

`SelectionService::new` resolves `HirImplOwner::Named` through resolver names,
dependency names, and suffix matching, then fabricates typed receiver patterns.
The constructor therefore retains a production name-based receiver-authority
path.

Relevant code:

- `lib/src/selection/service.rs`, `SelectionService::new`

Closure evidence:

- `SelectionService` requires the canonical receiver-pattern map from its
  caller.
- Missing patterns are diagnostics/invariant failures, not reconstructed values.
- No suffix/display-name receiver lookup remains in production selection.

### F11. Accepted validation permits unresolved ordinary field access

Targetless `FieldAccess` is rejected only when used as a call callee. A plain
`FieldAccess(_, _, None)` can cross accepted validation even though accepted
field access must be ID-backed.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_method_authorities_in_expr`

Closure evidence:

- Accepted conversion requires `HirFieldLocation` for every field access.
- Error-recovery field accesses remain only in unresolved/debug HIR.

### F12. Standalone mono bypasses accepted HIR

The public `mono::monomorphize` entry point accepts raw `HirProgram` and calls
`Monomorphizer::process` directly.

Relevant code:

- `lib/src/mono/mod.rs`, `monomorphize`

Closure evidence:

- The entry point accepts accepted HIR and returns the same fallible
  `MonomorphizedProgram` contract, or it is removed.

### F13. Tests bypass accepted construction

`new_unchecked_for_test`, test-only `DerefMut`, and the test branch in
`ResolvedHirProgram::new` let broad product/downstream fixtures skip production
accepted conversion. Product creation also substitutes `Exact(Unit)` for a
missing receiver pattern under `cfg(test)`, so tests do not exercise the same
invariant as production.

Relevant code:

- `lib/src/hir/accepted.rs`
- `lib/src/infer/mod.rs`, `ResolvedHirProgram::new`
- `lib/src/products.rs`, test-only receiver-pattern fallback

Closure evidence:

- Contract fixtures construct valid accepted HIR through the production
  conversion.
- Deliberately malformed tests target the unresolved-to-accepted conversion or
  artifact decoder directly.

### F14. Ambiguity regression coverage is incomplete

There is a differently-specific overlap regression for generated Drop, but no
equivalent direct coverage for frontend selection, static materialization, or
trait materialization.

Relevant code:

- `lib/src/mono/methods.rs`, Drop tests
- `lib/src/selection/service.rs`, selection tests

Closure evidence:

- All four paths have red-green overlap tests and deterministic diagnostics.

### F15. Selection falls back when effective trait-method authority is missing

`SelectionService::impl_method` substitutes the trait member ID when the
`effective_trait_methods` relation has no `(impl, member)` row. Selection also
uses missing-relation checks to reinterpret a trait impl method as an inherited
trait default. The effective-method relation is required authority and must be
complete after conformance; selection must not invent or reinterpret it.

Relevant code:

- `lib/src/selection/service.rs`, `impl_method`
- `lib/src/selection/service.rs`, `instantiate_impl_method_with_subst`

Closure evidence:

- Trait impl selection requires an exact `(impl, member) -> body` row.
- Missing, foreign, and inconsistent rows produce deterministic diagnostics or
  fail accepted conversion.
- No `unwrap_or(member_id)` or missing-map default-body inference remains.

### F16. Lowering reconstructs receiver patterns from method signatures

`Lowerer::selection_service` first builds receiver patterns from owner metadata,
then falls back to the first method's first parameter and self-receiver mode.
This makes a method signature an independent receiver-authority source and can
silently choose a pattern based on hash-map iteration order.

Relevant code:

- `lib/src/lower/control_flow/secondary.rs`, `selection_service`

Closure evidence:

- Canonical receiver patterns are created once from impl owner syntax and IDs.
- Selection receives that complete map and fails if a required entry is absent.
- No method-parameter-derived receiver pattern remains.

### F17. Method authority does not validate trait-argument shape

`validate_method_target_authority` checks trait IDs and member IDs but does not
require `trait_args.len()` to match the selected trait's generic parameter count.
It also does not validate that selected impl trait arguments agree with the
impl's declared trait reference. Malformed authority can therefore carry missing,
extra, or inconsistent trait arguments into mono.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_method_target_authority`
- `lib/src/hir/mod.rs`, `HirSelectedTraitMember`

Closure evidence:

- Trait targets validate exact argument arity and accepted type contents.
- Impl-backed selected-trait authority equals the impl's declared trait
  reference after applying owner substitution.
- Missing, extra, and inconsistent trait arguments have focused regressions.

### F18. Test builds retain a legacy DCE method reachability algorithm

`InstanceReachabilityIndexes` indexes method and trait-default IDs as ordinary
zero-substitution functions under `cfg(test)`, and includes receiver/name-based
method rediscovery helpers only in test builds. Most unit tests therefore run a
different reachability algorithm from production and can hide raw method-ID
edges that production intentionally rejects.

Relevant code:

- `lib/src/dce.rs`, `InstanceReachabilityIndexes::new`
- `lib/src/dce.rs`, `method_target_instances`
- `lib/src/dce.rs`, `field_method_instances`
- `lib/src/dce.rs`, `find_trait_impl_for_method_target`

Closure evidence:

- Test and production DCE build identical function-only DefId indexes.
- Legacy HIR method rediscovery helpers and their fixtures are deleted.
- DCE tests construct realistic `InstanceId` MIR edges.

### F19. Generated Drop authority is not carried from mono to MIR

Mono directly probes types and creates method instances but records no canonical
`receiver type -> InstanceId` generated obligation with an origin span. MIR then
rediscovers Drop by scanning impls, method bodies, projection metadata, receiver
parameter types, and effective-method maps. Several paths treat any method in a
Drop impl as Drop rather than requiring the language-item member's effective
body. `has_direct_drop_impl` can also fall back to matching method parameter
types when no direct-drop set was built.

Relevant code:

- `lib/src/mono/methods.rs`, `monomorphize_drop_for_type`
- `lib/src/mono/process.rs`, Drop probes without origin spans
- `lib/src/mir/builder/mod.rs`, `drop_glue_instance_roots`
- `lib/src/mir/builder/mod.rs`, `instance_is_stdlib_drop_method`
- `lib/src/mir/builder/mod.rs`, `direct_drop_types_for_program`
- `lib/src/mir/builder/mod.rs`, `mir_drop_glue`
- `lib/src/mir/builder/mod.rs`, `drop_glue_callable_key_for_type`
- `lib/src/mir/builder/blocks.rs`, `has_direct_drop_impl`

Closure evidence:

- Mono records explicit generated Drop obligations/results keyed by canonical
  `TypeId`, trait ID, member ID, instance ID, and optional source span.
- MIR roots and backend `drop_glue` are copied from that mono result without impl
  or method lookup.
- Only `effective_trait_methods[(impl, stdlib_drop_method)]` identifies the Drop
  body.
- Source-tied probes retain `Some(span)`; recursive/global probes use `None`.

### F20. MIR still recognizes targetless builtin indexing by name

Reference lowering special-cases a `MethodCall` when `target.is_none()`, the
method-name string is `"index"`, and `TypeFacts` says builtin indexing applies.
The approved representation requires explicit `BuiltinIndex` authority and
forbids absent-target plus name recognition.

Relevant code:

- `lib/src/mir/builder/expr.rs`, reference lowering

Closure evidence:

- MIR recognizes builtin indexing only from required accepted
  `HirSelectedMethodTarget::BuiltinIndex` authority.
- Read, write, borrow, slice, string, pointer, and bounds-check regressions retain
  behavior without the string fallback.

### F21. Accepted validation omits trait default bodies

`HirProgram::validate_method_authorities` visits top-level functions and impl
methods but never visits bodies in `HirTrait.methods`. Trait defaults can carry
missing authority, raw method IDs, targetless field calls, and unresolved types
through the accepted and product boundaries.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_method_authorities`

Closure evidence:

- Strict accepted conversion recursively converts and validates every trait
  default body.
- A malformed trait-default body fails before product creation and mono.

### F22. HIR index rebuild reconstructs effective method authority by name

`HirDefinitionIndexes::from_parts` iterates trait methods/signatures, looks up an
impl method with the same string key, and inserts `effective_trait_methods` rows.
Every index rebuild can therefore recreate semantic authority from names instead
of preserving the relation produced by conformance or artifact loading.

Relevant code:

- `lib/src/hir/mod.rs`, `HirDefinitionIndexes::from_parts`
- `lib/src/hir/mod.rs`, `rebuild_indexes_with_canonical_names`

Closure evidence:

- Index rebuilding never creates effective-method rows.
- Conformance and artifact loading are the only producers of the complete
  relation.
- Rebuild preserves existing rows and validation rejects missing rows.

### F23. HIR index rebuild reconstructs receiver authority from names

Index construction resolves nominal impl owners through exact or unqualified
name suffixes, reparses reference/pointer/array/slice/primitive display strings,
and derives generic receiver patterns from `type_name` and
`receiver_arg_types`. This is a downstream name-based authority constructor.

Relevant code:

- `lib/src/hir/mod.rs`, `nominal_owner_id_from_names`
- `lib/src/hir/mod.rs`, `impl_receiver_pattern`
- `lib/src/hir/mod.rs`, `impl_source_owner_type`
- `lib/src/hir/mod.rs`, `HirDefinitionIndexes::from_parts`

Closure evidence:

- Collection/lowering creates the canonical typed receiver pattern once from
  syntax and canonical IDs.
- Index rebuild preserves supplied patterns and never parses display names.
- Missing receiver patterns fail validation rather than being synthesized.

### F24. Lowering and inference reselect deferred method calls

Call-shaped dot expressions can be lowered to targetless
`Call(FieldAccess(..., None))` even after a method candidate was found.
`resolve_all_types_in_expr` later runs selection again and rewrites it to
`MethodCall`; `infer::materialize_deferred_method_calls` contains a second
reselection pass. This is the hidden field-name method channel the design
explicitly prohibits, and it allows candidate ordering to change between
phases.

Relevant code:

- `lib/src/lower/control_flow/secondary.rs`, dot/method-value lowering
- `lib/src/lower/types_helpers/type_vars.rs`, `resolve_all_types_in_expr`
- `lib/src/infer/mod.rs`, `materialize_deferred_method_calls`

Closure evidence:

- Successful dot-method selection emits a selected `MethodCall` immediately.
- Genuine method values emit a lambda whose inner call has authority.
- `Call(FieldAccess(...))` is accepted only for an ID-backed field.
- Both deferred reselection passes are deleted.

### F25. Selection failures are collapsed into `Option`

`SelectionDiagnostic` defines structured no-implementation, receiver-mismatch,
ambiguity, and missing-target cases, but `SelectionService` does not return or
record it. Candidate APIs return `Option`; ambiguity often becomes `None` and is
later reported as an unknown field or generic missing authority.

Relevant code:

- `lib/src/selection/types.rs`, `SelectionDiagnostic`
- `lib/src/selection/service.rs`, public selection methods
- `lib/src/lower/control_flow/secondary.rs`, selection consumers

Closure evidence:

- Required selection APIs distinguish no match from ambiguity and malformed
  authority.
- Lowering reports the structured selection diagnostic at the call span.
- Projection and deferred-generic paths do not erase ambiguity.

### F26. Mono diagnostics do not have a structured error payload

Mono pushes free-form `Diagnostic` strings for missing IDs, invalid bindings,
zero or ambiguous matches, missing effective methods, and missing bodies. The
approved design requires `MonoErrorKind` or an equivalent structured payload so
required materialization failures cannot be conflated or reason-erased.

Relevant code:

- `lib/src/mono/methods.rs`
- `lib/src/mono/mod.rs`, `Monomorphizer::diagnostics`

Closure evidence:

- Mono materialization returns/records a typed error kind with IDs and candidate
  sets.
- Conversion to user-facing `Diagnostic` preserves the owning expression or
  generated-obligation span.

### F27. Projection selection retains secondary authority and first-match lookup

`ProjectionImpl` stores `receiver_arg_types` instead of the canonical receiver
pattern and computes substitutions by zipping decomposed receiver arguments.
MIR projection resolution and Drop lookup use `find_map`, selecting the first
matching projection impl without detecting overlap. This violates the typed
projection key and no-first-match requirements retained by the Task 5 design.

Relevant code:

- `lib/src/type_services/projection.rs`, `ProjectionImpl`
- `lib/src/lower/types_helpers/helpers.rs`, `find_projection_impl`
- `lib/src/mir/builder/mod.rs`, `resolve_projection_from_impl_metadata`
- `lib/src/mir/builder/mod.rs`, `drop_glue_callable_key_for_type`

Closure evidence:

- `ProjectionImpl` owns the canonical typed receiver pattern.
- Projection matching uses `(base type, trait ID, associated-type ID, trait
  arguments)` and exact structural substitution.
- Multiple matching projection impls are diagnosed before accepted HIR/MIR.

### F28. Task 5 source-string audit tests contradict the approved test strategy

The design explicitly says not to add permanent tests that merely assert deleted
symbol strings are absent, but `semantic_identity_audit.rs` contains Task 5
source-text assertions. These are brittle and coexist with test-only legacy
algorithms that behavioral tests should replace.

Relevant code:

- `lib/src/semantic_identity_audit.rs`
- design testing strategy, lines 691-711

Closure evidence:

- Task 5 behavior is protected by typed boundary and behavioral tests.
- Obsolete Task 5 source-string assertions are removed after their replacement
  tests are present.

### F29. Codegen-concrete predicates accept `Type::Error`

HIR, artifact, and external-link concreteness helpers reject generics,
type variables, and projections but classify `Type::Error` as concrete. A
malformed body or impl can therefore be registered as a concrete/object-backed
provider instead of being rejected at the accepted or artifact boundary.

Relevant code:

- `lib/src/hir/mod.rs`, `hir_type_is_codegen_concrete`
- `lib/src/crate_artifact/load.rs`, `product_type_is_codegen_concrete`
- `lib/src/crate_system/extern_store.rs`, `type_is_codegen_concrete`

Closure evidence:

- Every concreteness predicate rejects `Type::Error` recursively.
- Accepted conversion and artifact loading fail before provider/link
  classification when any stored type contains `Type::Error`.
- Focused tests cover function bodies, impl metadata, and object-backed method
  interfaces.

### F30. Accepted authority does not validate call-shape invariants

The validator checks target IDs but permits semantic mismatches between the HIR
call shape and selected authority: a `MethodCall` may target a static method, a
`StaticMethod` call may target a method with a self receiver, a trait impl may be
encoded as `ImplMethod` without `selected_trait`, and `BuiltinIndex` may carry
generic bindings or appear in a static-call target. These malformed states are
then deferred to mono or MIR.

Relevant code:

- `lib/src/hir/mod.rs`, `validate_method_target_authority`
- `lib/src/hir/mod.rs`, `validate_method_authorities_in_expr`

Closure evidence:

- Accepted method-call authority requires a method with a self receiver.
- Accepted static authority requires a method without a self receiver.
- Trait impl targets require exact selected-trait identity and effective mapping.
- `BuiltinIndex` is legal only for method-shaped builtin index operations and
  carries no substitutions.
- Each malformed combination has a focused conversion and artifact regression.

### F31. Pre-mono accepted HIR and products permit session-local `InstanceId`s

`HirCallTarget::Instance`, `HirVarTarget::Instance`, and `Try.branch_target` can
enter the current accepted wrapper. Product serialization includes the instance
variant and artifact validation ignores it. Products are defined as resolved
pre-mono HIR and must never persist session-local `InstanceId` edges.

Relevant code:

- `lib/src/hir/mod.rs`, accepted authority validation
- `lib/src/products/type_table.rs`, serialized call targets
- `lib/src/crate_artifact/load.rs`, call-target validation/remapping

Closure evidence:

- Pre-mono accepted types cannot represent materialized instance edges, or
  strict conversion rejects every such edge.
- Accepted `Try` stores selected branch/from-residual authority without a
  materialized `branch_target` slot.
- Product schema has no serialized `InstanceId` executable target.
- Post-mono HIR uses a distinct materialized call-target shape consumed by MIR.

### F32. Incomplete Drop language-item pairs are accepted

Artifact validation permits `stdlib_drop_trait = Some(...)` with
`stdlib_drop_method = None`, product construction can emit that state, and mono
silently returns from every Drop probe when the method ID is absent. The design
requires an invalid language-item relation to be a compiler error.

Relevant code:

- `lib/src/products.rs`, `hir_language_items_from_resolved_hir`
- `lib/src/crate_artifact/load.rs`, `validate_product_language_items`
- `lib/src/mono/methods.rs`, `monomorphize_drop_for_type`

Closure evidence:

- Drop trait and member language items are either both absent or both present.
- The member ID is declared by the selected trait.
- Current-crate injection, artifact loading, and mono reject incomplete or
  inconsistent pairs with structured diagnostics.

### F33. Static method authority is reconstructed with names and a `Unit` fallback

`static_method_target_for_callee` finds the trait member by the method-name key
instead of the effective-method relation, repairs owner generic bindings by
matching generic display names, reconstructs owner types from nominal names, and
falls back to `Type::Unit` when owner resolution fails. This fabricates malformed
authority after an exact method `DefId` was already known.

Relevant code:

- `lib/src/lower/control_flow/secondary.rs`, `static_method_target_for_callee`

Closure evidence:

- Static authority is produced by the normal selection result with exact impl,
  member, method, receiver pattern, and `GenericParamId` bindings.
- Trait member identity comes from the effective-method relation, not a name
  lookup.
- Generic bindings never match parameter display names.
- Missing owner authority is a lowering diagnostic; `Type::Unit` is never used
  as a fallback owner.

### F34. Static trait-bound path selection returns the first matching bound

`resolve_static_bound_method_path` iterates a generic parameter's trait bounds
and returns the first trait containing the requested static member name. If two
bounds define that member, iteration order selects authority instead of
reporting ambiguity. The path also bypasses `SelectionService` diagnostics.

Relevant code:

- `lib/src/lower/paths.rs`, `resolve_static_bound_method_path`

Closure evidence:

- Static bound selection gathers all matching `(trait_id, member_id)` candidates.
- Zero and multiple matches return structured selection diagnostics.
- Bound ordering cannot change selected authority.

### F35. Unresolved operator inference chooses the first named impl method

Binary and unary operator inference scan impls, reconstruct receiver types from
`imp.type_name`, look up the operator's method-name string, and stop at the first
candidate. This constrains unresolved operands from hash/vector iteration order
before the real selection service runs, so overlapping candidates can silently
change inferred types and selected authority.

Relevant code:

- `lib/src/lower/expression.rs`, `infer_unresolved_binary_operator_receiver`
- `lib/src/lower/expression.rs`, `infer_unresolved_unary_operator_receiver`

Closure evidence:

- Operator inference records trait/selection constraints without selecting an
  impl by display name.
- Candidate discovery uses canonical receiver patterns and reports ambiguity.
- Reordering impl declarations cannot change inferred operator receiver types.
- `SelectionService::select_concrete_method_matching` owns compatibility
  filtering, target deduplication, receiver-adjustment preference, and
  deterministic ambiguity IDs.
- `cargo test -p rock-lib lower::expression::tests -- --nocapture` passes all 13
  focused regressions, including unresolved binary/unary and concrete overlap.

### F36. Qualified static method resolution is name/alias based and first-match

`resolve_static_method_path` builds display-name aliases for a resolved struct,
queries the legacy `(type_name, method_name)` method map, then scans impl display
names and returns the first matching method. The result is a raw method function
value whose static authority is reconstructed later by
`static_method_target_for_callee`. A qualified static method used as a value has
no accepted authority representation at all.

Relevant code:

- `lib/src/lower/resolution.rs`, `resolve_static_method_path`
- `lib/src/lower/resolution.rs`, `static_method_lookup_names`
- `lib/src/lower/control_flow/secondary.rs`, `call_target_for_callee`
- `lib/src/lower/control_flow/secondary.rs`, `static_method_target_for_callee`

Closure evidence:

- Qualified static lookup starts from canonical owner `DefId`/typed receiver
  pattern and gathers all applicable impls.
- Zero and multiple matches are diagnosed; aliases and impl order cannot select
  authority.
- Static method calls and values persist exact selected authority immediately.
- The legacy `(String, String) -> HirFunction` map is not semantic authority.

### F37. Compiler-generated Deref selection starts from a hardcoded trait name

`apply_trait_deref` resolves the literal trait name `"Deref"`, and deferred
receiver construction repeats the same lookup before selecting a method. Task 5
requires compiler-generated Deref calls to carry exact selected IDs and the
repository architecture forbids compiler-owned stdlib trait-name semantics.

Relevant code:

- `lib/src/lower/types_helpers/helpers.rs`, `apply_trait_deref`
- `lib/src/infer/mod.rs`, `deferred_receiver_candidates`

Closure evidence:

- The deferred inference path is removed with F24.
- Trait-driven dereference receives canonical trait/member authority from the
  language's operator/protocol resolution rather than a compiler string.
- Generated Deref method calls persist that exact authority before inference.
- The stdlib protocol declares `@*`; its trait display name is non-semantic.
- `generated_operator_deref_persists_exact_selected_authority` proves the
  generated call contains the exact impl, trait, and effective method IDs.
- `cargo test -p rock-lib --test integration deref -- --nocapture` passes all
  seven dereference integration regressions, including a locally named
  `PointerLike` protocol.

### F38. Current-trait method selection resolves the current trait by name

`Lowerer` stores `current_trait` as `Option<String>`, passes it into
`SelectionService`, and `select_current_trait_method` indexes the trait map by
that string before deriving the ID. The current declaration already has a
canonical owner `DefId`; aliases or same-named dependency traits must not
participate in current-trait authority.

Relevant code:

- `lib/src/lower/mod.rs`, `Lowerer::current_trait`
- `lib/src/selection/service.rs`, `SelectionService::current_trait`
- `lib/src/selection/service.rs`, `select_current_trait_method`

Closure evidence:

- Body lowering carries the current trait `DefId`.
- Current-trait selection loads the trait directly by ID and emits its exact
  member ID.
- Same-named local/dependency trait regressions cannot alter the target.

### F39. Production lowering missed dependency effective-method authority

`LowerCrateRegistration::register_extern_crate` imports dependency effective
trait-method rows, but production lowering did not invoke that service. The
dependency declarations arrived through collection while the semantic relation
remained empty, and HIR index reconstruction had been masking the missing
wiring.

Relevant code:

- `lib/src/lower/crates/registration.rs`, `LowerCrateRegistration`
- `lib/src/lower/session.rs`, `LoweringSessionServices::prepare_lowerer`
- `lib/src/lower/pipeline.rs`

Closure evidence:

- Production session preparation imports the dependency effective-method
  relation exactly once without re-registering declarations.
- Canonical receiver authority travels on each structural dependency `HirImpl`;
  no receiver sidecar remains.
- A cross-crate test observes structural receiver authority and the non-empty
  effective relation before conformance and after strict finalization.
- Removing downstream reconstruction does not break dependency method calls.

### F40. Mono fixtures bypass canonical receiver authority

Five mono method regressions constructed generic impls directly without entries
in `impl_receiver_patterns`. They failed after exact selected-target
applicability correctly stopped consulting legacy receiver/name metadata. Two
test names also described short-name matching even though that behavior is
forbidden by Task 5.

Relevant code:

- `lib/src/mono/methods.rs`, mono method fixtures

Closure evidence:

- Generic nominal fixtures install owner-qualified exact receiver patterns.
- Borrowed-slice fixtures install an owner-qualified `SliceFamily` pattern.
- Qualified receiver regressions exercise canonical typed matching, not
  short-name fallback.
- `cargo test -p rock-lib mono::methods::tests -- --nocapture` passes all 17
  tests.

### F41. External/process mono fixtures bypass canonical receiver authority

Running the complete mono suite after F40 exposed five more direct
`Monomorphizer` fixtures whose selected methods have no canonical receiver
pattern. Their expected instance materialization predates the strict authority
boundary.

Relevant code:

- `lib/src/mono/external.rs`, `generic_artifact_method_specialization_records_method_def_id_origin`
- `lib/src/mono/process.rs`, selected generic/inherent/object-backed method tests

Closure evidence:

- Every direct fixture supplies canonical receiver authority using the same
  setup as production loading.
- `cargo test -p rock-lib mono -- --nocapture` passes all 102 selected tests.

### F42. Exact stdlib static authority has a receiver/owner mismatch

After static materialization became a typed `Result`, compiling the stdlib for
the integration suite exposed inherent impl rows whose legacy
`trait_arg_types` contain receiver arguments even though the impl has no trait
reference. Mono incorrectly compared those values to the selected target's
empty trait arguments, then mislabeled the failure as a receiver mismatch. The
old `Option` path hid this discrepancy and left method calls for `ptr`/`push`
unmaterialized until post-mono validation.

Relevant code:

- `lib/src/lower/`, static and method target construction
- `lib/src/mono/methods.rs`, `monomorphize_static_method_call`
- `stdlib/vec.rk`, `stdlib/string.rk`, `stdlib/string_type.rk`, `stdlib/net.rk`

Closure evidence:

- Trait-argument applicability is checked only when the impl/target names a
  trait; inherent applicability remains governed by the canonical receiver
  pattern and exact method identity.
- Bare-generic receiver patterns validate consistently before and after product
  serialization.
- Exact applicability remains mandatory; no fallback or diagnostic suppression
  is introduced.
- `test_generic_function_argument_in_monomorphized_method_call` passes.

### F43. Empty-bang partial method application loses selected authority

When `receiver.method!` left explicit parameters unapplied, lowering returned
the original unresolved `FieldAccess` rather than an executable method value.
The next application therefore inferred a fresh function type and could bypass
the selected method target.

Relevant code:

- `lib/src/lower/control_flow/secondary.rs`, argument secondary lowering

Closure evidence:

- Partial application re-enters the authority-bearing method-value path and
  produces a lambda whose body owns the exact impl/method target.
- Applying the remaining argument calls that lambda without rediscovery.
- `cargo test -p rock-lib lower::control_flow::secondary -- --nocapture` passes
  all 20 tests.

### F44. Qualified-static fixtures register only name-keyed methods

The first whole-suite run after removing alias/name static lookup exposed four
tests that populated `LowerItems.methods` without an owning `HirImpl` or
canonical receiver pattern. Those fixtures asserted the deleted name-priority
behavior rather than the production semantic contract.

Relevant code:

- `lib/src/lower/paths.rs`, qualified static path tests

Closure evidence:

- Fixtures register method-owning impl IDs and exact nominal receiver patterns.
- Module/import aliases choose the nominal owner by resolver `DefId`; stale
  short/canonical method-map entries cannot choose the semantic method.
- `cargo test -p rock-lib lower::paths::tests -- --nocapture` passes all 33
  tests.

### F45. Artifact callable validation treats display names as semantic

The whole-suite artifact tests showed a valid `ResolvedVar` targeting the exact
`Global::dealloc` `DefId` being rejected because its canonical diagnostic name
was absent from an alias-derived name set. This made artifact acceptance depend
on display alias spelling even though executable identity was complete.

Relevant code:

- `lib/src/crate_artifact/load.rs`, `ProductNominalTypeValidator`

Closure evidence:

- Function/extern/static-method validation checks target kind and exact product
  identity only.
- Callable display-name and ambiguous-name maps are no longer part of the
  semantic validator.
- Tests cover mismatched and ambiguous display names with exact local and
  dependency targets.
- `cargo test -p rock-lib crate_artifact -- --nocapture` passes all 131 selected
  tests.

### F46. Removing first-match selection exposed latent inference ordering

The clean selection rule exposed calls whose receiver became concrete only after
constraint solving, plus index outputs and integer literal variables whose
substitutions had not yet been applied. These were previously masked by first
impl iteration and downstream rediscovery.

Closure evidence:

- `lib/src/infer/authority.rs` selects only previously unresolved calls and
  rejects zero/multiple finalized matches.
- Pre-generalization selection feeds unique return and argument constraints into
  inference; strict post-finalization selection skips already-selected calls.
- Operator fallback is unique candidate or explicit `I64`, never collection
  order.
- Literal defaulting touches only variables reachable from the active receiver.
- All 482 integration tests pass.

### F47. Type-variable field access selected the first name match

The old lowering branch sorted all structs with a matching field spelling and
selected the first. It now resolves only a unique current-crate nominal owner,
then records exact owner and field IDs. The strict authority pass resolves fields
only from a concrete `Struct DefId`; ambiguity and missing fields remain errors.

### F48. Method generics were not instantiated at call sites

`infer_method_substitution` unified arguments against rigid generic identities,
which could freeze contextual lambdas and method sections as self-referential
`Type::Generic` values. Every callable generic now receives a fresh inference
variable before receiver and argument unification, and authority stores only the
resolved substitutions.

### F49. Closure codegen ignored borrow capture kinds

MIR carried `ByRef` and `ByMutRef`, but LLVM closure environments loaded and
stored every capture by value. Borrowed captures now store addresses, closure
body locals have reference types and dereference as places, and escaping
synthetic currying/method-value closures move their captures. Focused mutable
method-section, borrow-loan, curried function, and TCP method-chain regressions
pass.

### F50. Range HIR used `Type::Error` as executable metadata

Range syntax is a compiler-owned, non-first-class iterator form. Its HIR result
now uses `Unit` rather than an unresolved/error type; `for` lowering still gives
the loop variable explicit `I64` semantics. All range integration regressions
pass.

### F51. Dependency trait defaults are not instantiated for downstream impls

Mono replaces an empty, already-conformed downstream impl method stub with the
accepted dependency trait-default provider body, but only preserves the local
method ID. Types and selected authority inside the provider body still refer to
the trait's generic `Self` and trait parameters. A default that calls another
trait member therefore fails exact receiver matching for a downstream concrete
impl.

Relevant code:

- `lib/src/mono/external.rs`, dependency trait-default body merge
- `lib/src/mono/substitute.rs`, accepted-body generic substitution

Closure evidence:

- Provider bodies are instantiated with the impl's exact trait arguments and
  conformed receiver type before they replace downstream stubs.
- The downstream stub's semantic method identity and conformed signature remain
  authoritative.
- A source-free stdlib artifact regression exercises `Show.println` calling
  `Show.show` on a downstream `Named` impl.

### F52. Test fixtures can construct or mutate accepted HIR without validation

The phase-typed migration added `AcceptedHirProgram::from_program` and
`AcceptedHirProgram::program_mut` under `cfg(test)`. Product and mono fixtures use
those methods to create or mutate `HirProgramFor<AcceptedHir>` directly. This
reintroduces the same invariant bypass F13 was intended to remove and allows the
test build to exercise accepted states that production cannot construct.

Relevant code:

- `lib/src/hir/accepted.rs`, test-only accepted constructors/mutators
- `lib/src/products.rs`, accepted-program rebuild/mutation helpers
- `lib/src/mono/mod.rs`, direct accepted-program fixture construction

Closure evidence:

- No constructor accepts `HirProgramFor<AcceptedHir>` without validating an
  unresolved `HirProgram` first.
- Tests that need a valid accepted program construct unresolved HIR and call the
  production `AcceptedHirProgram::try_from` boundary.
- Deliberately malformed accepted-artifact tests mutate serialized product rows,
  not an in-memory accepted program.

### F53. Receiver-pattern validation rejects trait-argument generic bindings

Accepted-HIR and artifact validation compare the generic IDs occurring in an
impl receiver pattern against every generic declared by the impl. Valid impls
can bind some generics through the selected trait arguments instead, such as a
conversion impl whose source type appears in the trait reference but not its
receiver. The receiver-only equality check rejects those canonical impls and
prevents the stdlib artifact from crossing the accepted boundary.

Relevant code:

- `lib/src/hir/mod.rs`, accepted impl receiver-pattern validation
- `lib/src/crate_artifact/load.rs`, product impl receiver-pattern validation

Closure evidence:

- Validation requires each declared impl generic to occur in the canonical
  receiver pattern or selected trait arguments.
- Unknown generic owners/indices and genuinely unconstrained impl generics are
  still rejected.
- Accepted-HIR and artifact regressions cover a generic bound only by the trait
  argument, and the stdlib product artifact crosses both boundaries.
- `cargo test -p rock-lib impl_generic -- --nocapture` passes all four focused
  accepted/artifact positive and negative regressions.

### F54. Trait selection fabricates a missing member identity

`SelectionService::instantiate_impl_method_with_subst` uses the selected impl
method ID when `trait_member_id` cannot find the named member in the exact trait.
The same fallback participates in inherited-default detection. A malformed or
stale trait impl can therefore emit a target that claims its impl method is a
trait member instead of refusing selection.

Relevant code:

- `lib/src/selection/service.rs`, `instantiate_impl_method_with_subst`

Closure evidence:

- Trait-backed candidate construction requires the exact member ID from the
  selected trait and cannot substitute an impl method ID.
- Missing trait-member identity yields no selected candidate and no authority-
  bearing HIR call.
- Required-trait selection returns `TraitMemberMissing`; diagnostic construction
  does not substitute the trait ID as a fake member ID.
- A focused selection regression supplies an impl method absent from its trait
  and proves selection refuses it.
- `cargo test -p rock-lib selection::service -- --nocapture` passes all 34
  selection-service regressions after canonicalizing the affected fixtures.

### F55. Mono receiver-identity fixtures do not exercise canonical authority

Two mono tests called `set_impl_receiver_pattern_for_test` on an empty
monomorphizer, then passed a separate `HirImpl` with a bare generic receiver to
`impl_receiver_pattern_matches`. The helper mutation could not affect the impl
under test, so the tests asserted nominal identity while actually supplying a
pattern intended to match every receiver.

Relevant code:

- `lib/src/mono/mod.rs`, receiver-pattern identity regressions

Closure evidence:

- Each fixture stores its exact nominal `DefId` and generic argument directly in
  `HirImpl.receiver_pattern` before invoking the matcher.
- `cargo test -p rock-lib mono -- --nocapture` passes 104 library mono-filtered
  tests plus the matching integration anchor.

### F56. Projection substitution silently accepts malformed provider metadata

`ProjectionImpl::projection_substitution` uses `unwrap_or_default` when its
canonical receiver pattern does not match the projected base type. It then zips
selected and expected trait arguments without requiring equal arity or checking
pattern compatibility. A provider bug or malformed metadata row can therefore
substitute only part of an impl and normalize a projection through authority
that does not apply.

Relevant code:

- `lib/src/type_services/projection.rs`, `ProjectionImpl::projection_substitution`
- `lib/src/type_services/projection.rs`, `ProjectionNormalizer::normalize`

Closure evidence:

- Projection substitution returns no result unless the canonical receiver
  pattern matches and every selected trait argument matches at equal arity.
- The normalizer leaves the original projection unresolved when checked
  substitution fails.
- Focused regressions cover receiver mismatch and trait-argument mismatch from a
  deliberately malformed projection provider.
- `cargo test -p rock-lib type_services::projection -- --nocapture` passes all
  six projection-service regressions.

### F57. Static method values do not carry selected authority

`lower_path` represents a concrete `Type::method` value as
`ResolvedVar(Function(method_id))` and a generic-bound static method value as an
unresolved `Var`. `call_target_for_callee` can reconstruct static authority for
an immediate call, but storing or passing the value leaves no exact impl/trait,
member, owner type, or substitutions on the executable HIR node. Accepted HIR
rejects the concrete raw method ID, while the unresolved bound spelling can
survive without authority.

Relevant code:

- `lib/src/lower/paths.rs`, qualified static path lowering
- `lib/src/lower/control_flow/secondary.rs`, `call_target_for_callee`
- `lib/src/lower/control_flow/secondary.rs`, `static_method_target_for_callee`

Closure evidence:

- Concrete and generic-bound static method values lower to closure values whose
  body calls the method with an exact `HirStaticMethodTarget`.
- No accepted expression stores a raw static method `DefId` or relies on a
  qualified display spelling to recover static authority.
- Focused lowering and end-to-end regressions cover storing then invoking a
  static method value and assert the exact target IDs in the generated body.
- Artifact remapping recognizes an exact authorized static callee, remaps it
  from the enclosing target, and does not misclassify a static trait signature
  as an ordinary function.
- `cargo test -p rock-lib lower::paths -- --nocapture` passes all 34 path tests;
  `test_static_method_value_preserves_selected_authority` passes end to end.

### F58. Static lookup uses receiver-mode metadata as declaration shape

`resolve_static_method_path` and `static_method_target_for_callee` treat a method
as static when `self_receiver` is `None`. `HirFunction::is_method` is the
canonical declaration-shape field; `self_receiver` describes receiver mode and
can be absent in implicit/default receiver fixtures. The mismatch lets a
receiver method be wrapped as a static value and later causes artifact shape
validation to reject its raw function target.

Relevant code:

- `lib/src/lower/resolution.rs`, `resolve_static_method_path`
- `lib/src/lower/control_flow/secondary.rs`, `static_method_target_for_callee`

Closure evidence:

- Concrete static lookup requires `!method.is_method` before constructing any
  resolved value or authority.
- Receiver methods cannot be selected through `Type::method` static syntax even
  when their receiver-mode metadata is absent.
- Focused static-resolution and artifact-backed integration regressions pass.
- `resolution_context_rejects_receiver_method_without_explicit_receiver_mode_as_static`
  passes, and the source-free stdlib artifact built by the F57 integration test
  validates successfully.

### F59. Receiver-authority migration left stale subsystem fixtures

The final unfiltered library suite exposes eleven tests across collection, HIR,
inference, body lowering, and trait conformance that still construct or assert
the pre-F9 model. The stale cases use bare generic receiver patterns, mutate a
detached helper/side table, omit canonical nominal IDs, or expect receiver
authority to be absent/reconstructed from display names.

Relevant code:

- `lib/src/collect/headers.rs`, impl-header fixture
- `lib/src/hir/mod.rs`, impl receiver identity fixture
- `lib/src/infer/solve.rs`, trait-bound impl fixture
- `lib/src/lower/bodies.rs`, impl body context fixtures
- `lib/src/lower/collect/traits.rs`, borrowed-slice receiver fixture
- `lib/src/lower/traits/conformance.rs`, conformance receiver fixtures

Closure evidence:

- Every fixture supplies the exact canonical receiver pattern on the `HirImpl`
  it passes to production code.
- Assertions inspect structural receiver authority rather than expecting
  display-name reconstruction or detached side effects.
- `cargo test -p rock-lib collect::headers::tests -- --nocapture` passes all 29
  tests; `hir::tests` passes 51, `infer::solve::tests` passes three,
  `lower::bodies::tests` passes ten, `lower::collect::traits::tests` passes five,
  and `lower::traits::conformance::tests` passes 32.
- The final complete `rock-lib` suite passes.

### F60. Layout projection fixture violates checked provider applicability

The layout test provider returns a projection impl for any base type. Its
receiver pattern does not match the projection used by
`type_layout_identifies_slice_and_fat_pointer_shapes`; the old empty-substitution
fallback masked that malformed provider row.

Relevant code:

- `lib/src/type_services/layout.rs`, projection layout fixtures

Closure evidence:

- The fixture's canonical receiver pattern and trait arguments match the
  projection it returns.
- `cargo test -p rock-lib type_services::layout::tests -- --nocapture` passes;
  the six projection-service regressions and complete suite also pass.

### F61. Impl-header collection bypasses bare-slice type policy

`CollectContext::impl_type_info` and the lower-stage mirror special-case
`ParseType::Slice` into `HirImplReceiverPattern::SliceFamily` without invoking
the shared `TypeLowerer` policy on the slice node. As a result,
`impl Trait for [T]` is accepted even though bare slices are forbidden outside a
reference or pointer.

Relevant code:

- `lib/src/collect/context.rs`, `CollectContext::impl_type_info`
- `lib/src/lower/collect/traits.rs`, `Lowerer::impl_type_info`
- `lib/src/type_lowering.rs`, bare-slice policy

Closure evidence:

- Impl owner collection applies the same bare-slice legality policy as every
  other type position while preserving canonical patterns for legal borrowed or
  pointer slice owners.
- `test_bare_slice_impl_target_is_rejected`, all 29 impl-header tests, and the
  483-test integration suite pass.

### F62. Default injection erases selfless associated-function shape

`TraitDefaultMethodInjector::prepare_missing_default_method` sets
`func.is_method = true` for every generated default. A trait default with no
receiver is therefore injected as a receiver method even though static path
selection and accepted call-shape validation rely on `is_method` as the
canonical declaration-shape bit.

Relevant code:

- `lib/src/lower/traits/conformance.rs`,
  `TraitDefaultMethodInjector::prepare_missing_default_method`
- static path selection and mono materialization

Closure evidence:

- Generated defaults preserve the trait member's receiver/static declaration
  shape.
- All 32 conformance tests and
  `test_selfless_trait_default_stays_associated_function_when_injected` pass.

### F63. Blanket Into body attachment uses rediscovered generic order

Body attachment rediscovered impl generic order from AST as `[T, U]`, while the
collected canonical impl owns `[U, T]`. Its lookup receiver and trait arguments
therefore became `Type::Error`, the `Into::into` body was never attached, and
the artifact faithfully carried an empty generic body that returned a zeroed
`Big`.

Relevant code:

- `stdlib/convert.rk`, blanket `Into` implementation
- `lib/src/lower/bodies.rs`, impl body slot matching
- generic-bound static selection and mono materialization

Closure evidence:

- Body slot matching lowers AST identity under the candidate impl's canonical
  owner and generic parameter order rather than independently rediscovering it.
- The generated static target retains exact trait/member authority and concrete
  `Small -> Big` substitutions through mono.
- All ten body-lowering tests, all 17 external-mono tests,
  `test_stdlib_into_uses_from_blanket_impl`, and the complete suite pass.

### F64. Try protocol failures lose language-level diagnostics

Try lowering now forwards generic `SelectionDiagnostic::NoImplementation`
messages for a non-carrier operand and a missing `FromResidual` conversion.
These are semantically expected protocol failures and must retain the stable,
source-facing `?` diagnostics rather than exposing internal operator wording and
raw trait IDs.

Relevant code:

- `lib/src/lower/control_flow/secondary.rs`, Try lowering
- `lib/src/selection/types.rs`, `SelectionDiagnostic::message`

Closure evidence:

- Try lowering maps no-carrier and no-residual-conversion failures to precise
  language diagnostics while retaining structured ambiguity/malformed-authority
  details.
- `test_try_non_carrier_reports_diagnostic`,
  `test_try_option_to_result_without_from_residual_reports_diagnostic`, the
  483-test integration suite, and the complete suite pass.

### F65. Qualified static-method authority is reconstructed from names

`resolve_static_method_path` selects the canonical owner and impl but returns a
`LowerResolvedValue` with `HirVarTarget::Function(method_id)`. Later,
`static_method_target_for_callee` rescans impls, resolves the selected trait
member by method name, and correlates impl-owned and method-owned generics by
display name. This contradicts F36 and F57: the selected static method's exact
authority must persist from resolution through lowering.

For this overlooked qualified-static path, F65 supersedes and narrows the
overly broad closure claims in F36 and F57; their rows and detailed sections
remain historical records.

Relevant code:

- `lib/src/lower/resolution.rs`, `resolve_static_method_path`
- `lib/src/lower/paths.rs`, qualified static-method value lowering
- `lib/src/lower/control_flow/secondary.rs`, `static_method_target_for_callee`

Closure requirements:

- Resolution returns the exact impl, method, optional trait/member,
  receiver-pattern, and generic-parameter authority.
- Lowering instantiates that authority without an impl rescan, trait-member name
  lookup, or generic display-name matching.
- `static_method_target_for_callee` is deleted.
- Post-fix focused lower and audit regressions are added and pass, and all final
  gates are rerun and pass.

Closure evidence:

- `resolve_static_method_path` returns `LowerResolvedStaticMethod` with exact
  impl, method, optional trait/member, receiver-pattern, owner-generic, and
  method-generic authority.
- `instantiate_static_method_value` consumes that record directly. It derives
  owner-to-method generic relations structurally, requires all matching
  occurrences to agree, composes owner and method substitutions into selected
  trait arguments, and reports missing or conflicting authority.
- Resolver-produced trait-backed coverage uses distinct impl-owned and
  method-owned `GenericParamId` values and asserts exact impl, method, trait,
  member, owner-substitution, method-substitution, and trait-argument authority.
- Local and artifact-backed generic static constructors pass, including nested
  structural generic relations. `static_method_target_for_callee` is absent
  from production code and guarded by `semantic_identity_audit`.

## Confirmed Clean Areas To Preserve

- Production DCE follows MIR `InstanceId` edges and indexes only ordinary
  function origins; test-only divergence is tracked by F18.
- MIR's raw `DefId` callable map indexes only `InstanceOrigin::Function`.
- Production method-ID zero-substitution repair is removed.
- Production receiver-mode repair is removed.
- Mono no longer filters typed matches by receiver-pattern specificity before
  its ambiguity checks.
- Product format `37` consistently carries receiver-pattern and effective
  trait-method relations.
- `effective_trait_methods` remains ID-keyed and is not reconstructed by method
  name.
- Production MIR/codegen has no `MirCallable::Method` dispatch path.

## Iteration Protocol

For each finding:

1. Add the smallest failing regression that proves the gap.
2. Run it and confirm the expected failure.
3. Implement the architectural fix, not a compatibility shim.
4. Run focused tests and the nearest subsystem suite.
5. Mark the finding closed here with exact tests and source evidence.
6. Append any newly discovered Task 5 gap to this ledger before continuing.

## Final Closure Gates

Task 5 may be marked complete only when all of the following are true:

- Every finding above is marked closed with evidence.
- `cargo fmt --all --check` passes.
- `git diff --check` passes.
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture` passes.
- `cargo test -p rock-lib crate_artifact -- --nocapture` passes.
- `cargo test -p rock-lib` passes.
- `cargo clippy -p rock-lib --all-targets` completes without errors.
- A fresh whole-codebase source audit finds no optional accepted authority,
  authority reconstruction, implicit specialization, silent mono failure, raw
  method `DefId` executable edge, or downstream method rediscovery path.
- `CLEAN_SLATE_COMPILER_AUDIT.md` is corrected only after all previous gates are
  satisfied.

## Final F1-F65 Closure Evidence

Observed on 2026-07-15 after F65 was closed:

- `cargo fmt --all --check`: passed.
- `git diff --check`: passed.
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`: 37 passed.
- `cargo test -p rock-lib crate_artifact -- --nocapture`: 143 passed.
- `cargo test -p rock-lib lower::paths -- --nocapture`: 48 passed.
- `cargo test -p rock-lib`: 1,839 library tests, 483 integration tests, and one
  auxiliary test passed; one doc test was intentionally ignored.
- `cargo clippy --workspace --all-targets`: completed successfully with no
  errors. Existing advisory warnings remain outside Task 5's error gate.
- A fresh direct-source audit covered accepted-HIR conversion, lower/static
  resolution, selection and mono materializers, products/artifacts/providers,
  DCE, MIR, and codegen. It found no optional accepted method authority,
  downstream authority reconstruction, specificity winner, silent required
  materialization failure, raw method-`DefId` executable edge, or downstream
  name-based method rediscovery.
- Residue searches found no production `static_method_target_for_callee`,
  `zero_substitution_method_instance`, `receiver_pattern_specificity`,
  `imported_impl_receiver_patterns`, or `receiver_arg_types`.

## Historical F1-F64 Final Closure Evidence

Observed on 2026-07-14 after F1-F64 were closed; this historical evidence does
not close F65:

- `cargo fmt --all --check`: passed.
- `git diff --check`: passed.
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`: 36 passed.
- `cargo test -p rock-lib crate_artifact -- --nocapture`: 141 passed.
- `cargo test -p rock-lib --test integration`: 483 passed.
- `cargo test -p rock-lib`: 1,817 library tests, 483 integration tests, and one
  auxiliary test passed; one doc test was intentionally ignored.
- `cargo clippy -p rock-lib --all-targets`: completed successfully with no
  errors. Existing advisory warnings remain outside Task 5's error gate.
- Three fresh CodeGraph audits covered accepted-HIR conversion, frontend
  selection and every mono materializer, products/artifacts/providers, DCE, and
  MIR. They found no production optional accepted method authority, authority
  reconstruction, implicit-specialization winner, silent materialization
  failure, raw method-`DefId` executable edge, or downstream method
  rediscovery.
- Residue scans found no production `zero_substitution_method_instance`,
  `receiver_pattern_specificity`, `imported_impl_receiver_patterns`, or
  `receiver_arg_types`. The remaining `lookup_self_receiver` helper is
  `#[cfg(test)]` and is not a production repair path.
