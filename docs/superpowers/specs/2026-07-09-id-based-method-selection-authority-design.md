# ID-Based Method Selection Authority Design

## Goal

Complete `CLEAN_SLATE_COMPILER_AUDIT.md` Step 5 in one bounded architecture
pass: selection resolves source-level method syntax once, lowering persists a
complete typed authority, monomorphization materializes the selected callable as
a canonical instance edge, and final MIR contains only backend-contract callable
keys.

The completed step must remove method dispatch rediscovery by method name,
receiver display name, type display alias, formatted type string, or first
matching impl from production mono and MIR. Exact method identities use `DefId`;
concrete callable identities use `InstanceId`; final MIR calls use
`MirCallableKey` registered in `MirBackendContract`.

## Current State

Earlier selection work established useful pieces of this contract but did not
complete the phase handoff:

- `selection::SelectedMethod` contains the selected origin, receiver
  adjustment, return type, associated outputs, owner substitution, and an
  optional `HirMethodCallTarget`.
- `selection::SelectionAuthority` exposes several of those facts for tests, but
  it is an ephemeral view. Lowering does not persist it as a complete HIR
  contract.
- `HirMethodCallTarget` stores optional impl and trait IDs, trait arguments, a
  method ID, and an index-origin flag. It does not distinguish exact impl
  methods, trait members awaiting monomorphization, and builtin indexing as
  validated states.
- Builtin indexing is represented by `SelectedOrigin::BuiltinIndex` with no
  `HirMethodCallTarget`. Lowering then emits a targetless method call named
  `index`, and MIR recognizes that shape by string and type facts.
- `HirImpl` still exposes `HirImplOwner::Named(String)`, `type_name`, and
  `receiver_arg_types`. Selection and later phases can reconstruct nominal or
  builtin receiver identity from those strings.
- Trait impl methods are stored by method name without an explicit
  trait-member-ID to impl-method-ID relation. Trait-bound concretization can
  therefore recover an override by method name after the trait member was
  already selected.
- Mono honors exact targets in several paths, but trait-bound and static method
  paths still use method-name tables or receiver matching to find the concrete
  body.
- Compiler-generated `Drop` specialization starts with the canonical Drop trait
  ID but still derives a receiver name, scans impls, and selects a method from
  the matched impl instead of carrying an exact trait-member/body relation.
- Selected method values normally lower to lambdas with selected inner calls,
  but compatibility paths still reinterpret accepted
  `Call(FieldAccess(receiver, method_name, None), ...)` as method dispatch in
  mono and MIR. Real function-valued fields use the same broad expression shape
  and must remain distinct through `HirFieldLocation`.
- MIR builder can turn a selected method ID back into a method name, search impls
  by receiver aliases, and emit `MirCallable::Method` when it cannot produce an
  instance key directly.
- MIR projection impl lookup can still recover a missing owner through receiver
  type-name helpers instead of treating the typed projection provider as the
  only projection-selection authority.
- Production DCE understands unresolved `MirCallable::Method` and follows its
  optional instance or method-ID fallback. The larger DCE display-alias lookup
  helpers are test-only, but the production unresolved-callable branch remains.
- Products serialize the partial HIR method target and the string-shaped impl
  receiver metadata, so artifact-backed generic bodies preserve the same
  incomplete contract. Artifact validation can also verify a selected method by
  looking it up again through the expression's method-name string.

These paths do not normally override a valid exact target, which is what the
older Task 12 and Task 13 work proved. The remaining problem is stronger: the
selected contract is not sufficient by itself, so downstream phases still own
parts of method selection.

## Design Principles

1. Source method names are resolved once at the frontend boundary.
2. Selected dispatch is represented by a validated enum, not a set of
   independently optional fields.
3. Impl receiver applicability is structural and ID-based.
4. Trait member identity and concrete impl method identity are distinct facts.
5. Mono may materialize a trait-only authority after substituting enclosing
   generics, but it may not rerun source-level method lookup.
6. Every executable user method call becomes an `InstanceId` edge before MIR.
7. Builtin indexing is explicit and does not borrow a fake method or trait ID.
8. Display names remain available for diagnostics and debug output but never
   affect dispatch, instance identity, or backend symbol lookup.

## Non-Goals

- Do not migrate all `Declarations`, `LowerItems`, `PartialHir`, resolver, or
  source-scope maps to ID-keyed storage. Audit Step 6 owns that broader phase
  boundary.
- Do not redesign language-item representation. Preserve the existing Drop
  trait and Drop method IDs; audit Step 7 owns normalization of remaining
  compiler-recognized traits such as `Sized`.
- Do not decide whether builtin indexing should remain a core operation, become
  an ordinary stdlib impl, or require a language item. Audit Step 8 owns that
  policy. This step preserves current behavior and user-impl precedence through
  an explicit authority/core-operation representation.
- Do not replace `ast::SelfReceiverMode` throughout HIR. Audit Step 9 owns that
  migration. This step only prevents receiver selection from being rerun.
- Do not redesign trait solving, coherence, operator syntax, or method-call
  syntax.
- Do not collapse every transitional `MirCallable` variant unless required by
  the method migration. This step must remove `MirCallable::Method` from the
  production contract; unrelated callable cleanup remains separate.
- Do not convert all unchecked MIR test fixtures. Update fixtures touched by this
  work; audit Step 12 owns the complete fixture cleanup.
- Do not change the Step 3 link-record rule or the Step 4 backend-symbol rule.
  Object-backed symbols still come only from link records, and source/display
  names never become symbol fallbacks.
- Do not add compiler-owned stdlib loading or unqualified stdlib injection.
- Do not preserve the old artifact schema or add compatibility decoding.

## Architecture

The phase boundary after this work is:

```text
source method name + typed receiver
    -> selection service
    -> validated HIR method authority
    -> lower applies receiver adjustment and finalizes substitutions
    -> mono substitutes enclosing generics and materializes InstanceId
    -> pre-MIR body contains direct call edges or explicit builtin indexing
    -> MIR contains Resolved(MirCallableKey)
    -> backend contract provides the callable declaration and symbol
```

Names may participate in the first arrow because that is source resolution.
They may not participate in any later arrow.

### Canonical Impl Receiver Pattern

`HirImpl` needs one authoritative typed receiver pattern and an explicit
applicability class. Add fields with semantics equivalent to:

```rust
pub struct HirImplReceiverPattern {
    pub ty: Type,
    pub applicability: HirImplApplicability,
}

pub enum HirImplApplicability {
    Exact,
    SliceFamily,
}
```

The pattern represents the complete declared impl receiver:

- nominal receivers carry their struct or enum `DefId` and generic arguments;
- primitive receivers use the corresponding `Type` variant;
- slices, fixed arrays, references, and pointers use their structural `Type`
  shape;
- generic positions carry owner-qualified `GenericParamId`s.

`Exact` uses structural unification. Nominal heads compare by `DefId`; primitive
and structural heads compare by typed variants; fixed-array length remains part
of the type; generic bindings are keyed by `GenericParamId`.

`SliceFamily` preserves the existing builtin-slice applicability to slices,
borrowed slices, and fixed arrays without spelling those receivers as `Array`,
`&[T]`, or formatted type strings. The matcher operates on typed receiver
candidates and the candidate's explicit array-to-slice or reference adjustment.
It does not treat an arbitrary `Type::Array` as structurally equal to
`Type::Slice`.

Selection first builds the ordered receiver-adjustment candidates, then matches
each candidate against the typed pattern/applicability, and finally records the
winning adjustment. Lowering applies that winning adjustment exactly once. This
preserves current autoref, mutability, deref, and array-to-slice ordering.

This field must not become a second receiver authority. During the migration:

- `HirImpl.type_name` may remain only as display metadata;
- `HirImplOwner::Named(String)` may remain only if a non-semantic provider or
  display use is still required, and must be renamed or documented accordingly;
- `receiver_arg_types` must be deleted, derived from `receiver_pattern.ty`, or
  otherwise prevented from acting as an independent semantic matcher;
- `HirProgram::impl_owner_id` and resolver alias helpers must not be used to
  repair a missing receiver identity in production selection or mono.

Products and loaded dependency interfaces persist and remap the same typed
receiver pattern. They must not reconstruct it from `type_name` during loading.

### Explicit Trait Member Implementation Mapping

A trait member `DefId` and an overriding impl method `DefId` are not the same
identity. Persist their relation on the impl through a deterministic ID-keyed
map or an equivalent sorted typed record:

```rust
pub effective_trait_methods: BTreeMap<DefId, DefId>
// trait member DefId -> effective impl-local method body DefId
```

Lowering builds this relation while checking trait conformance, where both IDs
are already known. Inherent methods do not enter the map. An explicit override
maps to the override method ID. An injected trait default maps to the fresh
impl-local method ID produced after default-body generic-owner remapping and
call retargeting. This preserves the current generated-default body model rather
than bypassing it through the original trait method body.

The mapping is serialized in product interfaces and generic bodies and is
validated on load:

- the key belongs to the impl's `trait_id`;
- the value belongs to that impl;
- duplicate keys are rejected;
- every callable required trait member has an effective body mapping;
- a missing mapping is malformed HIR/artifact data, not a request to search by
  name or silently use the original trait body.

### Persisted HIR Selection Authority

Separate the complete frontend selection result from the smaller authority that
must cross the HIR-to-mono boundary.

The selection service still returns the winning receiver adjustment, selected
return type, associated-output facts, diagnostics, and candidate function data.
Refactor that result around the validated target enum below so origin, target,
and builtin output cannot contradict one another. Those lowering inputs remain
owned by selection and are not all copied into serialized HIR.

Persist only dispatch identity and generic bindings that mono cannot recover
without repeating semantic work. The exact exported Rust names may reuse
`HirMethodCallTarget`, but the stored data must have semantics equivalent to:

```rust
pub struct HirMethodSelection {
    pub target: HirSelectedMethodTarget,
    pub owner_substitution: Vec<HirTypeBinding>,
    pub method_substitution: Vec<HirTypeBinding>,
}

pub enum HirSelectedMethodTarget {
    ImplMethod {
        impl_id: DefId,
        method_id: DefId,
        selected_trait: Option<HirSelectedTraitMember>,
    },
    TraitMethod {
        trait_id: DefId,
        member_id: DefId,
        trait_args: Vec<Type>,
        dispatch: HirTraitDispatchKind,
    },
    BuiltinIndex,
}

pub struct HirSelectedTraitMember {
    pub trait_id: DefId,
    pub member_id: DefId,
    pub trait_args: Vec<Type>,
}

pub struct HirTypeBinding {
    pub param: GenericParamId,
    pub ty: Type,
}
```

`HirTraitDispatchKind` distinguishes the existing trait-bound, current-trait,
and unresolved-generic cases. It is not a second identity source; it controls
which invariant mono must satisfy when materializing the already selected trait
member.

The authority has no optional method ID and no boolean that changes the meaning
of an otherwise identical target. Invalid combinations are unrepresentable:

- an exact inherent call has an impl and concrete method ID;
- an exact trait impl call has impl and concrete method IDs plus the selected
  trait/member identity;
- a trait-bound call has the selected trait/member IDs and awaits only concrete
  impl materialization;
- builtin indexing has an explicit core-operation authority and no fabricated
  `DefId`.

Substitutions are keyed by `GenericParamId`, not positional strings. They are
stored as deterministic vectors so serialization is stable. Before creating an
`InstanceKey`, mono orders them by the declaration's canonical generic-ID order
and rejects missing, duplicate, extra, or unresolved bindings.

Lowering applies the selected receiver adjustment exactly once and stores the
adjusted receiver expression. `HirExpr.ty` is the single finalized result-type
authority. Canonical `Type::Projection` values already carry base type, trait ID,
associated-type ID, and trait arguments; they remain the projection authority
when an output cannot be concrete until mono. Do not serialize duplicate
receiver-adjustment, return-type, or associated-output fields that could drift
from the adjusted HIR expression and its type.

`HirStaticMethodTarget` remains a wrapper with the semantics
`{ owner_ty: Type, selection: HirMethodSelection }`, because a static method has
no receiver expression from which mono can recover its concrete owner type.
Instance method names and static method display paths may remain on expressions
for diagnostics, but downstream phases ignore them for dispatch.

### Selection Service

The selection service may accept a source method name in `SelectionRequest` and
may use frontend declaration/resolver name maps to find initial candidates. Once
a candidate is selected, its output is the complete frontend result; lowering
then persists the target and bindings subset defined above.

Receiver applicability must use `receiver_pattern`, trait IDs, trait arguments,
and typed unification. Delete the semantic roles of:

- `type_names_for_method_lookup`;
- `type_name_for_method_lookup`;
- `impl_matches_method_lookup_type`;
- primitive-name and `Array` string tables;
- suffix or display-alias recovery used to decide receiver ownership.

`selection/legacy_lookup.rs` should be deleted once all production consumers are
migrated. Source-facing diagnostics may still render types and method names.

Selection failures remain structured diagnostics. A successful selection cannot
return `target: None`.

### Lowering Boundary

Lowering consumes `SelectedMethod` only long enough to:

1. apply the chosen receiver adjustment exactly once;
2. coerce arguments against the selected signature;
3. finalize owner, trait, and method generic substitutions;
4. normalize selected associated outputs into `HirExpr.ty` or canonical
   `Type::Projection` values;
5. attach `HirMethodSelection` to the HIR call.

Lowering must attach authority for:

- inherent method calls;
- concrete trait impl calls;
- trait-bound and unresolved-generic method calls;
- current-trait and default-method calls;
- binary and unary operators lowered through methods;
- user-defined index implementations;
- static associated functions;
- trait-driven dereference calls;
- method calls generated inside `Try` lowering;
- calls inside generated method-value lambdas.

Builtin indexing must no longer be encoded as a targetless `MethodCall` named
`index`. Preserve the current method-shaped HIR if useful, but attach the
explicit `BuiltinIndex` selection and make the method-name string display-only.
MIR branches on that authority rather than `None + "index"`. User-defined
`Index` impls continue to use method authority and keep precedence over builtin
behavior. This representation records the current selection result without
deciding audit Step 8's core-operation-versus-stdlib policy.

Selected method values must lower to a lambda or another explicit bound-method
form whose inner call carries authority. `Call(FieldAccess(...))` is valid only
when the field access has an actual `HirFieldLocation`; it must never be a hidden
method lookup channel.

After inference finalization, validate accepted HIR before product creation or
mono:

- every semantic method call has an authority;
- every referenced impl, trait, member, and method ID exists in the relevant
  program or dependency interface;
- stored substitutions and expression/projection types contain no `Type::Error`
  and obey the phase's allowed generic/type-variable rules;
- the adjusted receiver expression and `SelfReceiverMode` agree with the
  selected method's self parameter;
- field access is either ID-backed field access or diagnosed error recovery.

Error-recovery HIR may temporarily omit authority while diagnostics are being
collected, but it must not enter products, mono, or MIR.

### Generated Method Obligations

Automatic Drop is not a lowering-time HIR call. Preserve its current producer
phase and represent it as a canonical generated obligation owned by mono/MIR,
with semantics equivalent to:

```rust
pub struct GeneratedMethodObligation {
    pub receiver_ty: Type,
    pub trait_id: DefId,
    pub member_id: DefId,
    pub trait_args: Vec<Type>,
    pub origin_span: Option<Span>,
}
```

Drop obligations use the existing `stdlib_drop_trait` and
`stdlib_drop_method` language-item IDs. They query the same typed impl receiver
index and `effective_trait_methods` mapping as source-selected trait calls. They
must not derive a receiver name, choose the only method in an impl, or invent a
lowered Drop call. The resulting drop callable is an `InstanceId` recorded in
the MIR backend contract.

Automatic Drop discovery is a typed probe before it is a required obligation.
Mono may probe parameter, local, expression, nominal-field, and globally interned
types. Zero matching Drop impls means the type needs no user Drop call and is not
an error. Exactly one matching impl creates the required obligation above.
Multiple matches, a missing effective Drop method for a matched impl, or an
invalid language-item relation is an error. `origin_span` is `Some` for a probe
tied to source HIR and `None` for recursive or global type-context probes.

### Monomorphization

Mono consumes authority; it does not call the frontend selection service and
does not search by method or receiver name.

For `ImplMethod`, mono:

- loads the exact impl and method by `DefId`;
- applies the recorded substitutions after substituting the enclosing instance;
- validates the receiver against the impl's typed receiver pattern;
- registers or reuses the exact `InstanceKey`;
- rewrites the executable edge to `HirCallTarget::Instance(instance_id)`.

For `TraitMethod`, mono:

- substitutes the enclosing instance into receiver, trait arguments, and stored
  bindings;
- queries an ID/typed impl index by trait ID, structural receiver pattern, and
  trait arguments;
- requires exactly one coherence-valid impl;
- uses `effective_trait_methods[member_id]` to select the explicit override or
  impl-local generated default body;
- registers or reuses that concrete instance;
- rewrites the edge to `HirCallTarget::Instance(instance_id)`.

This is materialization of an already selected trait obligation, not a second
method selection pass. Mono must not reconsider method names, receiver
adjustments, operator meaning, builtin precedence, or associated-output choice.

Object-backed dependency methods follow the same `InstanceKey` path. Their
`InstanceRecord` is provided by the loaded artifact/link data, and their backend
symbol still comes only from `ExternCrateLink.backend_symbols`.

All executable user-defined methods, including zero-substitution methods, use
an `InstanceId` call edge before MIR. For a required selected call, a missing
instance, zero matching impls, or multiple matching impls is a structured
mono/invariant error. None of those cases may fall back to a same-named method.

Mono becomes an explicitly fallible compiler phase. Introduce a `MonoErrorKind`
or equivalent structured diagnostic payload for missing IDs, invalid bindings,
zero or ambiguous typed impl matches for required selected calls, missing
effective trait methods, and missing object-backed instances. Each
source-selected obligation carries the owning `HirExpr.span`; generated
obligations carry their recorded origin span.

Change the production entry point to:

```rust
pub fn monomorphize_with_crates(
    program: ResolvedHirProgram,
    crate_ctx: &CrateContext,
) -> Result<MonomorphizedProgram, Diagnostics>
```

`Monomorphizer` accumulates phase diagnostics while walking bodies and returns no
partial `MonomorphizedProgram` when any executable authority cannot be
materialized. `compile_impl` propagates the result with `?` before MIR body
construction. The simpler `monomorphize` entry point and tests must adopt the
same fallible contract rather than hiding errors with `Option`, `panic!`, or a
compatibility wrapper.

### MIR Boundary

Production MIR builder receives post-mono bodies. It must not receive a semantic
method call that still needs impl selection.

For calls, MIR builder consumes direct `HirCallTarget` values and emits:

```text
Constant::Callable(MirCallable::Resolved(MirCallableKey::...))
```

Method migration must remove production dependencies on:

- `method_name_for_target`;
- `callable_for_field_method`;
- `instance_for_method_target` receiver/name fallback;
- `find_trait_impl_for_method_target`;
- MIR-local `type_names_for_method_lookup`;
- HIR display aliases for receiver matching;
- projection impl owner repair through type or display names;
- `MirCallable::Method`;
- `MirSelectedMethodMetadata` used to carry unresolved dispatch into MIR.

If a builder-local candidate type is useful during migration, keep it private to
the builder and require conversion to `MirCallableKey` before insertion into a
`MirFunction`. Final `MirProgram` operands must be contract keys, and every key
must have a `MirCallableDecl` in `MirBackendContract`.

MIR recognizes `HirSelectedMethodTarget::BuiltinIndex` and applies the existing
builtin index place/projection behavior. It must not recognize the operation from
an absent target plus the string `index`, and this step does not otherwise change
indexing policy.

Borrow checking, agreement, and codegen then consume the same resolved callable
shape. They must not retain special handling for `MirCallable::Method { instance:
Some(...) }`.

The existing typed projection provider remains the global authority for
projections, including projections not produced by method calls. Step 5 only
removes its name-based impl-owner repair: projection selection uses the full
typed key `(base type, trait ID, associated-type ID, trait arguments)` plus typed
impl receiver patterns. A missing impl owner or projection binding is an
invariant error, not an opportunity to call method-receiver name helpers.

### DCE Boundary

Production DCE runs on resolved MIR/instance edges. It follows
`MirCallableKey::Instance` directly and must not recover reachability from a
method ID, zero-substitution method table, receiver name, or display alias.

Remove the production `MirCallable::Method` branch together with that MIR
variant. Test-only helpers that construct HIR and MIR through legacy name lookup
must either use realistic selected authority and registered instances or remain
clearly outside the production reachability algorithm until audit Step 12
replaces the remaining fixtures.

### Products And Artifacts

Products are created from resolved pre-mono HIR, so they serialize the typed HIR
authority, not local-session `InstanceId` edges.

Update product and artifact data for:

- the canonical impl receiver pattern and applicability;
- the effective trait-member to impl-local-body ID mapping;
- every `HirMethodSelection` target and type binding;
- static method owner types plus selections;
- generated `Try` method selections.

Encoding uses `ProductTypeId` for every stored type. Remapping must visit impl,
trait, member, method, associated-type, and generic-parameter owners as well as
all nested types. Artifact validation rejects authorities that point outside the
loaded interface/body identity set. It validates impl/member/body relationships
through the explicit ID mapping and must not consult the method-name string on
the containing expression.

Because this changes serialized HIR and impl interfaces, bump
`PRODUCT_ARTIFACT_FORMAT_VERSION` to `37` in both `rock-lib` and
`rock-shared`. Older artifacts are rejected; no compatibility shim or defaulted
authority field is allowed.

## Error Handling

Use structured phase diagnostics or existing phase error containers:

- Selection reports no candidate, ambiguity, receiver mismatch, or invalid
  associated output before emitting authority.
- HIR validation reports a missing or malformed selected authority before mono.
- Product loading reports an invalid/remap-missing authority or impl receiver
  pattern as an artifact error.
- Mono reports missing exact IDs, non-concrete stored substitutions, no matching
  impl, ambiguous matching impls, missing effective-method mapping, or missing
  object-backed instance records.
- MIR reports an internal phase-boundary violation if any unresolved semantic
  method call reaches the builder.
- Codegen reports a missing backend-contract callable key, never a source-name
  lookup failure.

Do not repair malformed authority with source names, display aliases, short
paths, backend symbols, or first-match iteration order.

## Migration Strategy

1. Add behavioral RED tests for stale display names, same-named cross-crate
   receivers, exact trait-member override identity, explicit builtin indexing,
   method values, and resolved final MIR callables.
2. Introduce the typed impl receiver pattern, explicit trait-member mapping, and
   validated HIR authority. Update product encoding/remapping in the same change
   so no half-serializable HIR state exists.
3. Migrate selection producers and lower consumers. Add the accepted-HIR
   validation gate and eliminate successful targetless dispatch.
4. Make mono fallible, migrate it to typed/ID-only materialization, and rewrite
   every executable method edge to `InstanceId`.
5. Remove MIR method and projection rediscovery plus unresolved method callable
   data. Update DCE, borrowck, agreement, codegen, and touched fixtures to consume
   resolved keys.
6. Delete `selection/legacy_lookup.rs` and manually scan production sources for
   the removed rediscovery helpers.
7. Run focused and full verification, then mark audit Step 5 complete with exact
   evidence and any explicitly deferred non-Step-5 work.

Do not mark intermediate compatibility bridges as completion. This repository is
in a prototyping phase; temporary adapters should be removed in the same
implementation series.

## Testing Strategy

Use TDD for behavioral changes. Each migration slice should observe the focused
test fail before implementation and pass afterward.

### Selection And Lowering

Cover:

- exact inherent and trait impl selection with stale or changed display names;
- same-named nominal receivers with distinct `DefId`s;
- primitive, slice, array, pointer, reference, and generic receiver patterns;
- trait-bound, current-trait, unresolved-generic, operator, and index authority;
- receiver adjustments preserved and applied exactly once;
- owner and method substitutions keyed by `GenericParamId`;
- selected associated outputs becoming concrete types or canonical
  `Type::Projection` identities without duplicate serialized sidecars;
- builtin indexing represented explicitly without a fake or missing target;
- method values lowering to explicit selected calls rather than field fallback;
- deterministic diagnostics for missing authority and ambiguity.

### Mono And Instance Registry

Cover:

- exact impl authority never selecting a same-named method from another impl;
- trait-only authority materializing through trait/member IDs and typed receiver
  matching;
- trait default selection by trait/member IDs;
- compiler-generated Drop and trait-driven Deref selection by exact IDs;
- a type with no Drop impl producing no generated call and no diagnostic;
- ambiguous or malformed matched Drop impls producing structured diagnostics;
- generic receiver, method, and trait-argument substitutions;
- static associated methods and generated `Try` calls;
- object-backed dependency methods using registered instances and link records;
- zero-substitution and specialized methods both rewritten to `InstanceId`;
- zero or multiple typed impl matches failing without name fallback;
- method display-name mutation leaving `InstanceKey` and selected body unchanged.

### MIR And Backend Contract

Cover:

- post-mono traversal contains no executable unresolved method call;
- method calls lower to `Resolved(MirCallableKey::Instance(...))`;
- every emitted callable key is present in `MirBackendContract`;
- final MIR contains no `MirCallable::Method` or method selection metadata;
- projection impl selection does not reconstruct impl ownership from names;
- builtin index expressions preserve place, borrow, bounds, and user-impl
  precedence behavior;
- DCE follows only resolved instance call edges;
- borrowck and agreement derive callable identity from the resolved key only;
- codegen fails on a missing key without consulting any name map.

### Products And Cross-Crate Behavior

Cover:

- product roundtrip of impl receiver patterns/applicability, effective trait
  method mappings, selected targets, bindings, and static owner types;
- producer `ProductDefId` to consumer `DefId` remapping for every authority ID;
- two dependencies with the same display paths do not collide;
- artifact-backed generic bodies materialize the selected dependency method by
  IDs and typed receiver pattern;
- object-backed methods still require link records;
- artifact format `37` matches `rock-shared` and old versions are rejected.

Existing integration anchors include:

- `test_index_operator_prefers_selected_user_impl_over_builtin_array_index`
- `trait_bound_mut_receiver_call_autorefs_mut_reference`
- `trait_bound_shared_receiver_on_ref_does_not_double_borrow`
- `test_same_name_trait_methods_do_not_dispatch_by_method_name_only`
- `test_generic_trait_bound_dispatch_uses_trait_arguments`
- `test_generic_trait_bound_dispatch_matches_generic_impl_trait_arguments`
- `test_same_name_trait_default_methods_select_bound_trait_body`
- `test_same_name_generic_trait_default_methods_select_bound_trait_body`
- `test_same_name_trait_default_projection_uses_selected_trait_identity`

Minimum verification after focused tests:

```bash
cargo test -p rock-lib selection -- --nocapture
cargo test -p rock-lib lower -- --nocapture
cargo test -p rock-lib mono -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib mir::agreement -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib --test integration test_index_operator_prefers_selected_user_impl_over_builtin_array_index -- --exact --nocapture
cargo test -p rock-lib --test integration trait_bound_mut_receiver_call_autorefs_mut_reference -- --exact --nocapture
cargo test -p rock-lib --test integration test_same_name_trait_methods_do_not_dispatch_by_method_name_only -- --exact --nocapture
cargo test -p rock-lib --test integration test_generic_trait_bound_dispatch_uses_trait_arguments -- --exact --nocapture
cargo test -p rock-lib --test integration
cargo test -p rock-lib
cargo fmt --all --check
git diff --check
```

Run test suites serially. Save the full-suite output once when a persistent log
is useful.

Use a manual production-source residue scan for:

```text
type_names_for_method_lookup
type_name_for_method_lookup
impl_matches_method_lookup_type
callable_for_field_method
method_name_for_target
find_trait_impl_for_method_target
get_type_name_for_method
receiver_type_matches
trait_method_names_by_id
register_trait_member_names
semantic uses of HirProgram::impl_owner_id
MirCallable::Method
MirSelectedMethodMetadata
```

Do not add a permanent test that merely asserts deleted source symbol strings are
absent. Behavioral tests and typed phase validators are the durable regression
protection.

## Completion Criteria

Step 5 is complete only when all of the following are true:

- successful frontend selection always produces one validated target, and every
  accepted semantic HIR method call persists its `HirMethodSelection`;
- impl receiver applicability is represented and matched structurally by typed
  identity;
- trait-member to effective impl-local method identity is explicit, deterministic,
  and ID-keyed for overrides and injected defaults;
- lower applies receiver/result/projection facts in HIR and persists only the
  selected target plus owner/method bindings needed by mono;
- builtin indexing carries explicit `BuiltinIndex` authority and is never
  identified by target absence plus method name;
- method values cannot reach mono or MIR as field-name method lookup;
- mono uses exact IDs or typed trait-obligation materialization and never method
  or receiver display names;
- mono returns structured diagnostics through a fallible production API instead
  of dropping materialization failures through `Option` or `panic!`;
- every executable user method edge is rewritten to `InstanceId` before MIR;
- generated Drop obligations use the existing language-item IDs and typed impl
  index; Deref, operator, index, and Try calls obey the selected-call rules;
- final MIR method calls contain only resolved backend-contract callable keys;
- `MirCallable::Method` and its downstream special cases are removed;
- DCE follows resolved callable keys without method-ID fallback;
- product/artifact roundtrip and ID remapping preserve the persisted selection;
- artifact format version is `37` in `rock-lib` and `rock-shared`;
- object-backed method symbols still come only from link records;
- focused, integration, full, formatting, and diff verification passes;
- `CLEAN_SLATE_COMPILER_AUDIT.md` Step 5 is marked complete with validation
  evidence and without overclaiming Step 6, 7, 8, 9, or 12 work.

## Risks

- Receiver-pattern migration touches source collection, HIR, products, artifact
  loading, selection, and mono. Updating serialization with the HIR model avoids
  a dual-authority intermediate state.
- Trait member IDs and impl method IDs currently coincide in some tests and
  differ in real default/override paths. Regressions must deliberately use
  distinct IDs.
- Injected defaults currently receive fresh impl-local method IDs after body
  remapping. Building `effective_trait_methods` too early would capture the
  original trait member instead of the executable generated body.
- Generic trait-bound calls cannot choose a concrete impl until enclosing
  substitution. Mono must materialize the stored obligation without importing
  frontend candidate ranking or receiver adjustment logic.
- Method-value lowering captures receivers and can expose ownership or lifetime
  bugs. Preserve existing lambda capture behavior while replacing only the
  hidden name-based call edge.
- Builtin indexing currently relies on targetless method shape recognition in
  MIR place lowering. Replacing that recognition with explicit authority
  requires parity tests for reads, writes, references, borrowed slices, strings,
  pointers, and bounds checks without pre-deciding Step 8 policy.
- Making mono fallible changes its public and production call sites. Diagnostic
  accumulation must not leave a partially materialized program available to
  DCE or MIR.
- Removing `MirCallable::Method` affects borrowck and agreement helpers that
  currently extract `instance: Some(id)`. Those consumers should simplify to
  resolved keys rather than receive a replacement sidecar.
- Product schema changes are intentionally incompatible. A format bump is
  required, and no defaults should make incomplete old authorities appear valid.
- Existing audit/checklist wording may claim downstream selection authority is
  already complete. Update only the clean-slate Step 5 evidence after the actual
  implementation and preserve any broader remaining debt accurately.
