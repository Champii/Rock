# Task 8 Trait-Owned Index Semantics Design

## Status

Approved design for `CLEAN_SLATE_COMPILER_AUDIT.md` Step 8,
"Decide And Refactor Builtin Index Semantics."

Task 7 already decided that `[]` is a language-item-backed stdlib trait
protocol, not a compiler-owned structural operation. This specification defines
the complete behavioral migration, including read and mutable indexing, stdlib
implementations, lowering context, canonical selection authority, artifact
persistence, fallback deletion, diagnostics, and closure criteria.

## Problem Statement

Task 7 made Index identity canonical but intentionally retained a bounded
transition:

- `Index` trait, method, and associated-output identities come from the marked
  `IndexLanguageItems` bundle.
- Selection still fabricates `SelectedOrigin::BuiltinIndex` when no real impl
  matches arrays, slices, or pointers.
- HIR persists `HirSelectedMethodTarget::BuiltinIndex`, which has no trait,
  method, impl, or instance identity.
- Projection normalization uses structural `TypeFacts::builtin_index_output*`
  rules.
- Mono leaves builtin index method calls unmaterialized.
- MIR recognizes the resulting `Deref(MethodCall(...BuiltinIndex...))` shape,
  converts it directly to `Projection::Index`, and synthesizes bounds checks.
- Products and artifacts serialize the targetless authority.
- MIR tracks builtin Index trait IDs to synthesize projection outputs.

This is a hybrid language model. User-defined indexing is an ordinary trait
call, while compiler-owned types receive behavior from a targetless structural
fallback. It also hides a second semantic dependency: indexed assignment works
only because MIR bypasses the shared-reference `Index` return and treats builtin
indexing as a directly mutable place.

Task 8 removes both authorities. Reads use `Index`; writes use `IndexMut`. Both
are ordinary selected trait impl calls backed by stdlib source or user source.

## Goals

1. Preserve `arr[idx]` syntax through the marked `Index` protocol.
2. Preserve `arr[idx] = value` syntax through a marked `IndexMut` protocol.
3. Represent every accepted read or write target with canonical trait, impl,
   method, substitution, and instance identity.
4. Put array, slice, pointer, raw-slice-pointer, and `Vec` behavior in stdlib
   impl bodies.
5. Preserve fixed-array-to-slice coercion without making fixed-array indexing a
   compiler fallback.
6. Preserve safe bounds checks for fixed arrays, slices, raw slice pointers,
   and `Vec`.
7. Preserve unsafe requirements for raw pointer reads and writes.
8. Remove all targetless `BuiltinIndex` selection, serialization, projection,
   mono, and MIR paths.
9. Remove structural builtin index type facts from semantic selection and
   projection normalization.
10. Make missing providers or impls fail before mono and MIR without fallback.
11. Preserve user-defined index types, associated outputs, and exact-impl
    precedence.
12. Keep strings and declaration spellings out of Index and IndexMut identity.

## Non-Goals

- Do not make `Str` integer-indexable. UTF-8 text continues to require explicit
  string or byte APIs.
- Do not change `[U8]` indexing from `U8` to `Char`.
- Do not add implicit coercion from arbitrary integer key types to `I64`.
- Do not add a new indexing or bounds-check intrinsic.
- Do not preserve artifact format 38 compatibility.
- Do not add compiler-owned stdlib discovery or implicit stdlib loading.
- Do not optimize or inline the resulting trait calls as part of this task.
- Do not redesign unrelated operator traits, general method selection, or MIR
  assertion completeness.
- Do not retain a temporary mutable indexed-place bypass after the read fallback
  is removed.

## Surface Semantics

The source syntax remains unchanged:

```rock
value = arr[idx]
arr[idx] = value
```

The two forms have different semantic authorities:

- value use selects `Index::index` and dereferences its `&Output` result;
- assignment-place use selects `IndexMut::index_mut` and dereferences its
  `&mut Output` result.

Implementing `Index` does not imply mutable indexing. A read-only container may
implement only `Index`; reads compile and indexed assignments produce a
selection diagnostic. Implementing `IndexMut` requires the effective language
environment to provide the paired `Index` protocol from the same provider. Each
`IndexMut` impl also requires an `Index` impl for the same receiver and key type,
and their associated output types must be equal.

Nested assignment places propagate mutable-index context along the complete
place spine. Examples include:

```rock
matrix[row][column] = value
records[idx].field = value
(*owner).items[idx] = value
```

Every index operation needed to reach the assignment destination selects
`IndexMut`. Index operations in the right-hand side or in unrelated value
expressions continue to select `Index`.

## Language-Item Model

### Source Protocols

The existing read protocol remains:

```rock
lang index
< trait Index Idx
    lang output
    type Output
    lang method
    @index: Idx -> &Self::Output
```

Task 8 adds the mutable protocol:

```rock
lang index_mut
< trait IndexMut Idx
    lang output
    type Output
    lang method
    ^@index_mut: Idx -> &mut Self::Output
```

The source names `Index`, `IndexMut`, `Idx`, `Output`, `index`, and `index_mut`
are display text only. Renaming them while preserving markers does not change
semantics.

### Typed Registry

Add `LanguageItemRole::IndexMut` and a complete bundle:

```rust
pub struct IndexMutLanguageItems<D> {
    pub trait_id: D,
    pub output_id: AssocTypeId,
    pub method_id: D,
}

pub struct LanguageItems<D> {
    // existing bundles
    pub index: Option<IndexLanguageItems<D>>,
    pub index_mut: Option<IndexMutLanguageItems<D>>,
}
```

Final bundles are complete or absent. Only collection-local builder state may
be partial.

### Provider Policy

`Index` may exist without `IndexMut`. If `IndexMut` exists:

- `Index` must also exist;
- both roots and all marked children must come from one provider crate;
- current and dependency providers do not override each other;
- duplicate or split providers produce deterministic collection diagnostics.

This prevents one crate from defining read `[]` authority while another crate
silently defines assignment `[]` authority.

### Shape Validation

IndexMut validation requires:

- an exported trait root;
- exactly one trait generic for the index type;
- exactly one marked associated output;
- exactly one marked method signature or default method;
- mutable receiver mode;
- one explicit parameter equal to the trait generic;
- return type `&mut` of the marked associated-output projection.

Impl conformance additionally requires every IndexMut receiver/key pair to have
a matching Index impl with the same associated output. This models IndexMut as
the mutable capability layered on read indexing without requiring general trait
inheritance support.

The existing Index validation continues to require a shared receiver and shared
reference result. Source and artifact validation use exact IDs and type shape,
not names.

## Lowering And Selection

### Assignment Context

The current expression lowerer lowers binary operands before it discovers that
the operator is assignment. That ordering cannot correctly select IndexMut,
especially for fixed arrays whose read path may already have coerced the
receiver to a shared slice.

Refactor assignment lowering so place context is known before lowering its
left-hand expression. Use a lower-private mode such as:

```rust
enum PlaceUse {
    Value,
    Assignment,
}
```

The exact name is implementation-local. It must not enter AST, final HIR,
products, or MIR.

Assignment mode propagates only through the assignable place spine. Calls,
indices, and other expressions outside that spine remain value-mode. Lowering
must not select Index first and then rewrite its IDs to IndexMut after the fact.

### Read Selection

For `receiver[index]` in value mode:

1. Require the effective `IndexLanguageItems` bundle.
2. Build the normal receiver-adjustment candidates.
3. Select the exact marked trait and member IDs with the concrete key type.
4. Resolve the marked associated output from the selected impl.
5. Emit `Deref(MethodCall(...selected Index authority...))`.

### Mutable Selection

For `receiver[index]` on an assignment place:

1. Require the effective `IndexMutLanguageItems` bundle.
2. Build mutable-capable receiver candidates.
3. Select the exact marked mutable trait and member IDs.
4. Resolve the marked mutable associated output.
5. Unify the assignment RHS with that output.
6. Emit `Deref(MethodCall(...selected IndexMut authority...))`, where the call
   returns `&mut Output`.

Normal borrow checking then decides whether the receiver can be mutably
borrowed and whether the returned mutable reference conflicts with active loans.

### Fixed-Array Coercion

Stdlib provides generic slice impls rather than one impl for every fixed length.
Selection therefore uses:

- shared `[T; N]` to shared `[T]` receiver coercion for Index;
- mutable `[T; N]` to mutable `[T]` receiver coercion for IndexMut.

The existing `ArrayRefToSlice` primitive already preserves reference
mutability. Task 8 adds the missing mutable candidate construction and an
explicit receiver-adjustment identity if needed; it does not add an
Index-specific coercion primitive.

Exact receiver impls are considered before coerced slice impls. A user impl for
`Index Key for [T; N]` or `IndexMut Key for [T; N]` therefore remains
authoritative over the stdlib slice impl.

### Selection Output

Selection produces only normal canonical authority:

- `HirSelectedMethodTarget::ImplMethod` for concrete stdlib/user impls;
- `HirSelectedMethodTarget::TraitMethod` for unresolved generic or trait-bound
  dispatch before mono;
- owner and method substitutions keyed by `GenericParamId`;
- selected trait args containing the exact key type.

There is no `SelectedOrigin::BuiltinIndex`, builtin output side field,
targetless selected target, or structural retry after ordinary selection fails.

## Stdlib Implementations

### Shared Bounds Helper

`stdlib/index.rk` owns a private ordinary Rock helper that receives `index` and
`length`, rejects negative and upper-bound indices, prints the standard
`index out of bounds` message, and exits with failure.

The helper uses ordinary Rock comparisons/control flow and existing libc output
and exit declarations. It is not a compiler-recognized function and does not
use a new intrinsic.

### Existing Primitives

Stdlib impl bodies use only existing explicit low-level primitives:

- `~ArrayLen` to obtain slice length;
- `~ArrPtr` to obtain the element pointer;
- `~PtrOffset` to compute an element-stride address;
- raw-pointer dereference plus `&` or `&mut` to construct the result.

These primitives expose representation operations. The impl bodies, not the
compiler, define which traits use them and what indexing means.

### Slice Implementations

Stdlib implements both protocols for `[T]` with `I64` keys:

- Index checks bounds and returns `&T`;
- IndexMut checks bounds and returns `&mut T` from a mutable receiver.

Fixed arrays reach these impls through receiver coercion. Borrowed `[U8]`
therefore returns `U8`, preserving current behavior.

### Sized Raw Pointers

Stdlib implements both protocols for `*T where T: Sized` with `I64` keys:

- both methods are unsafe;
- neither method performs a bounds check because no length exists;
- Index returns `&T`;
- IndexMut returns `&mut T`.

The `Sized` bound prevents these impls from claiming fat raw slice pointers.

### Raw Slice Pointers

Stdlib separately implements both protocols for `*[T]`:

- both methods are unsafe;
- both recover the slice length and check bounds;
- Index returns `&T`;
- IndexMut returns `&mut T`.

This preserves the current `*[T]` flattening behavior rather than treating one
index step as producing another `[T]` value.

### Vec

`Vec<T>` keeps a direct `Index I64` impl and gains `IndexMut I64`. Both use the
same stdlib bounds helper and the vector's raw storage. Direct Vec impls avoid an
unrelated Deref redesign and preserve current public behavior.

## HIR, Mono, MIR, And Codegen Flow

### Final HIR

Accepted HIR contains selected method calls only. Remove
`HirSelectedMethodTarget::BuiltinIndex` and its helper methods. Remove the
executable structural `HirExprKind::Index` representation if no non-recovery
producer remains; failed lowering should produce the normal error expression
rather than an index node that later phases could execute.

Accepted-HIR validation treats Index and IndexMut like every other selected
method. It verifies IDs, substitutions, receiver mode, and finalized types with
no builtin exemption.

### Products And Artifacts

Product HIR serializes normal selected method authorities and the current-owned
IndexMut language-item bundle. Remove the serialized BuiltinIndex target
variant.

Increment both artifact format constants from 38 to 39. There is no format-38
decoder or compatibility shim.

Artifact loading validates the complete IndexMut bundle before remapping IDs.
It rejects partial bundles, wrong declaration kinds, foreign members,
wrong-owner associated types, invalid signature shapes, and split Index/IndexMut
providers.

### Monomorphization

Mono handles selected stdlib and user Index/IndexMut methods through the normal
trait-method path:

1. consume exact selected authority;
2. materialize the selected impl method;
3. specialize generic receiver/output types;
4. intern an `InstanceId`;
5. rewrite the call to that instance.

Delete all branches that leave builtin index method calls unmaterialized or use
BuiltinIndex as an invalid-binding sentinel.

### MIR

MIR receives an ordinary call returning a shared or mutable reference followed
by an ordinary dereference place. Indexed assignment writes through the selected
mutable reference. There is no recognition of method names, receiver shapes,
language-item IDs, or targetless index guards in MIR.

Delete:

- builtin indexed-method shape recognition;
- direct conversion of that shape to `Projection::Index`;
- builtin index trait-ID collection in `MirInstanceBodies`;
- builtin projection output synthesis;
- builder state that records the marked builtin Index projection;
- builtin-specific tests and fixtures.

`Projection::Index` itself remains valid MIR for compiler-generated loops and
other explicit MIR place operations. Its existence does not grant source `[]`
semantics.

### Bounds Codegen

Stdlib indexing bounds checks compile as ordinary Rock control flow and libc
calls. They do not synthesize `MirAssertKind::BoundsCheck` and do not require a
new runtime-helper contract entry.

Existing MIR bounds assertions remain supported for other MIR producers. Task 8
does not remove or redesign that independent MIR capability.

## Projection Model

Associated output resolution uses the selected impl's ordinary associated-type
definition for both Index and IndexMut.

Delete:

- `TypeFacts::builtin_index_output`;
- `TypeFacts::has_builtin_index_impl`;
- `TypeFacts::builtin_index_output_id`;
- projection-provider builtin Index predicates;
- special contract output rows for structurally indexable types;
- `builtin_index_output` fields in selected authority.

Codegen consumes projection outputs already present in the MIR backend contract.
It does not infer array, slice, or pointer outputs from type shape.

## Error Model

### Missing Protocols

- no Index provider: value indexing reports that the Index language-item
  protocol is unavailable;
- no IndexMut provider: indexed assignment reports that mutable indexing is
  unavailable;
- an unmarked trait named `Index` or `IndexMut` receives no `[]` semantics.

### Missing Implementations

- read with no matching impl reports no implementation for operator `[]` on the
  receiver/key types;
- write with no matching mutable impl reports no mutable implementation for
  indexed assignment on the receiver/key types;
- an IndexMut impl without a matching Index impl, or with a different associated
  output, is rejected during impl conformance;
- a failed exact impl selection does not retry structural builtin behavior;
- non-I64 builtin-container keys fail unless an explicit matching impl exists.

### Invalid Providers And Artifacts

Diagnostics cover duplicate roots, missing children, wrong receiver mode, wrong
parameter relation, wrong return mutability, wrong associated output, split
provider ownership, and malformed persisted IDs.

### Safety And Borrowing

- raw pointer Index and IndexMut calls outside unsafe context are rejected;
- mutable selection from an immutable or actively borrowed receiver is rejected
  by normal borrow rules;
- a shared Index reference cannot be used as an assignment destination;
- references returned from Index/IndexMut retain the receiver-derived origin
  required for escape and alias checks.

## Migration Strategy

### Slice 1: Extend The Protocol Contract

- add `index_mut` syntax role and typed bundle;
- bind and validate current-source providers;
- merge dependency providers with the paired-provider rule;
- carry the bundle through declarations, HIR, products, and artifacts;
- bump artifact format to 39;
- mark the stdlib IndexMut root and children.

The old fallback may remain temporarily in this compiling slice, but no new
consumer may use IndexMut by name.

### Slice 2: Add Context-Sensitive Mutable Selection

- detect assignment place context before lowering the LHS;
- propagate mutable-place context through nested place expressions;
- add exact ID-based IndexMut selection;
- add mutable array-to-slice receiver candidates;
- emit ordinary selected method authority;
- add read-only and mutable custom-container tests.

### Slice 3: Move Builtin-Type Behavior Into Stdlib

- add the private Rock bounds helper;
- implement Index and IndexMut for slices;
- implement unsafe Index and IndexMut for sized pointers;
- implement unsafe checked variants for raw slice pointers;
- add Vec IndexMut and share bounds behavior;
- prove fixed arrays use slice coercion and exact custom impls still win.

### Slice 4: Remove Targetless Authority End To End

- remove BuiltinIndex selection and authority fields;
- remove HIR and serialized target variants;
- remove product/artifact encode/decode/remap branches;
- remove mono bypasses;
- remove MIR method-shape recognition and direct place lowering;
- remove structural projection output synthesis and builtin trait-ID tracking;
- remove builtin index TypeFacts;
- convert or delete old fixtures.

### Slice 5: Close The Audit Step

- run focused behavior and malformed-contract tests;
- run artifact-backed stdlib tests;
- run complete integration and workspace gates serially;
- perform a complete production-source residue audit;
- update only Step 8 status and evidence in the clean-slate audit after every
  criterion passes.

## Test Strategy

### Language Items And Providers

- parse `lang index_mut` with marked output and method children;
- reject wrong root/member contexts and incomplete bundles;
- validate mutable receiver and `&mut Output` shape;
- rename every source declaration while preserving marker roles;
- prove same-spelled unmarked traits receive no semantics;
- allow Index without IndexMut;
- reject IndexMut without Index;
- reject an IndexMut impl without a matching receiver/key Index impl;
- reject paired impls with different associated outputs;
- reject split providers and duplicate providers deterministically;
- preserve exact IDs through product serialization and artifact remapping;
- reject malformed product-local IndexMut IDs and signatures;
- assert both artifact constants are 39.

### Selection And Lowering

- value index selects the marked Index member;
- assignment index selects the marked IndexMut member;
- read-only custom type reads but cannot be assigned through;
- custom key types select exact impls without I64 coercion;
- exact fixed-array impl wins over slice coercion;
- exact fixed-array IndexMut impl wins on assignment;
- nested `matrix[i][j] = value` selects mutable authority along the place spine;
- `records[i].field = value` selects mutable authority for the base index;
- RHS index expressions remain shared selections;
- no-provider array/slice/pointer reads and writes fail before mono.

### Runtime Behavior

- fixed-array read and assignment;
- slice read and assignment;
- borrowed `[U8]` returns `U8`;
- negative and upper-bound fixed-array/slice failures print the standard message;
- sized raw-pointer read and assignment work in unsafe context;
- sized raw-pointer use fails outside unsafe context;
- raw-slice-pointer read and assignment are checked and unsafe;
- Vec read and assignment use direct impls;
- Vec out-of-bounds behavior matches slice behavior;
- `Str` integer indexing remains rejected.

### Borrowing And References

- `&arr[i]` preserves a receiver-derived shared origin;
- mutable indexed references conflict with active shared loans;
- shared indexed references conflict with mutable assignment;
- returning a reference derived from a local indexed container is rejected;
- returning an indexed reference derived from an input borrow is accepted;
- mutable custom IndexMut results do not bypass receiver mutability.

### Regression Coverage

Preserve existing custom Index associated-output, ambiguity, generic impl,
artifact, unsafe-method, fixed-array precedence, slice, pointer, Vec, projection,
method-authority, and semantic-identity tests. Rewrite tests whose expected
authority is BuiltinIndex to assert exact stdlib impl/method/instance identity.

## Residue Audit

The final tracked production-source audit must find no executable or serialized
use of:

- `SelectedOrigin::BuiltinIndex`;
- `HirSelectedMethodTarget::BuiltinIndex`;
- `HirMethodCallTarget::builtin_index`;
- `is_builtin_index`;
- `select_builtin_index_method`;
- `builtin_index_output`;
- `has_builtin_index_impl`;
- `builtin_index_output_id`;
- `builtin_index_trait_ids`;
- `builtin_index_projection`;
- builtin index method-shape recognition in MIR;
- structural fallback after ordinary Index/IndexMut selection failure.

Remaining `Projection::Index` and `MirAssertKind::BoundsCheck` uses must be
classified as MIR-level place/assertion operations unrelated to source operator
authority. Resolver names, source declarations, diagnostics, and test fixture
text remain allowed boundary data.

Do not add permanent source-text tests that merely assert deleted tokens are
absent. Record residue evidence in the Task 8 closure audit and use semantic
tests for durable prevention.

## Verification Gates

Run test suites serially. The implementation plan must name the smallest exact
RED and GREEN commands for each behavior before broad gates.

Focused gates include:

```bash
cargo test -p rock-lib parser::items::tests::items -- --nocapture
cargo test -p rock-lib collect:: -- --nocapture
cargo test -p rock-lib hir::language_items -- --nocapture
cargo test -p rock-lib lower:: -- --nocapture
cargo test -p rock-lib selection:: -- --nocapture
cargo test -p rock-lib mono:: -- --nocapture
cargo test -p rock-lib mir::builder -- --nocapture
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

User-visible and complete gates:

```bash
cargo test -p rock-lib --test integration
cargo test -p rock-lib
cargo clippy --workspace --all-targets
cargo fmt --all --check
git diff --check
```

If adding `index_mut` changes tree-sitter marker corpus expectations, run from
`tree-sitter-rock/`:

```bash
tree-sitter generate
tree-sitter test
```

## Likely File Impact

Language-item model and binding:

- `lib/src/language_items.rs`
- `lib/src/collect/language_items.rs`
- `lib/src/collect/mod.rs`
- `lib/src/hir/language_items.rs`
- `lib/src/hir/mod.rs`
- `lib/src/hir/accepted.rs`

Lowering and selection:

- `lib/src/lower/expression.rs`
- `lib/src/lower/control_flow/secondary.rs`
- `lib/src/lower/types_helpers/helpers.rs`
- `lib/src/lower/types_helpers/projection.rs` or the current projection provider
  implementation after inspection
- `lib/src/infer/authority.rs` if post-inference index authority is materialized
  there
- `lib/src/selection/types.rs`
- `lib/src/selection/service.rs`
- `lib/src/type_services/facts.rs`

Mono, MIR, and backend contract:

- `lib/src/mono/process.rs`
- `lib/src/mono/methods.rs`
- `lib/src/mono/substitute.rs` if the structural HIR index variant is removed
- `lib/src/mir/mod.rs`
- `lib/src/mir/builder/mod.rs`
- `lib/src/mir/builder/expr.rs`
- `lib/src/mir/backend_contract.rs`
- `lib/src/codegen/types.rs`

Products and artifacts:

- `lib/src/products.rs`
- `lib/src/products/type_table.rs`
- `lib/src/crate_artifact/language_items.rs`
- `lib/src/crate_artifact/load.rs`
- `lib/src/crate_artifact/tests.rs`
- `lib/src/crate_system/extern_store.rs` if registry fixtures require updates
- `rock-shared/src/sysroot.rs`

Stdlib, tests, and audit:

- `stdlib/index.rk`
- `stdlib/vec.rk`
- `lib/tests/integration.rs`
- co-located unit tests in every changed subsystem
- `lib/src/semantic_identity_audit.rs` only for semantic behavior, not deleted
  spelling assertions
- `tree-sitter-rock/test/corpus/language_items.txt` if needed
- `CLEAN_SLATE_COMPILER_AUDIT.md` only after closure

The implementation plan must inspect every production BuiltinIndex occurrence
identified by the Task 7 closure ledger and name its deletion or replacement. It
must not assume this likely-file list is exhaustive.

## Acceptance Criteria

Task 8 is complete only when all of the following are true:

1. `arr[idx]` selects marked Index authority.
2. `arr[idx] = value` selects marked IndexMut authority.
3. Nested assignment places propagate mutable selection correctly.
4. Read-only Index implementations remain valid and reject mutable use.
5. Every IndexMut impl has a matching Index impl with the same receiver, key,
   and associated output.
6. Fixed arrays use stdlib slice impls through shared/mutable coercion.
7. Exact custom fixed-array impls take precedence over coerced slice impls.
8. Slice, sized pointer, raw slice pointer, and Vec behavior lives in stdlib
   impl bodies.
9. Bounds policy is ordinary stdlib Rock code using existing primitives; no new
   indexing intrinsic exists.
10. Sized raw pointers remain unchecked and unsafe.
11. Raw slice pointers remain checked and unsafe.
12. No provider or missing impl produces a diagnostic before mono without
    fallback.
13. Selection output always has canonical target identity.
14. Mono materializes every accepted Index/IndexMut call as an instance call.
15. MIR does not recognize source Index method shapes or language-item IDs.
16. Products and artifacts contain no BuiltinIndex target variant.
17. Artifact format is 39 in both `rock-lib` and `rock-shared`.
18. Projection outputs come only from ordinary selected impl/contract facts.
19. Structural builtin index TypeFacts and MIR projection synthesis are gone.
20. No backward-compatibility, display-name, or type-name fallback exists.
21. Borrow and escape checks remain sound for shared and mutable indexed
    references.
22. All focused, integration, full-suite, Clippy, rustfmt, diff, and applicable
    tree-sitter gates pass.
23. The complete production residue audit has no unclassified Task 8 fallback.
24. `CLEAN_SLATE_COMPILER_AUDIT.md` records completion only after closure
    evidence exists.

## Risks And Mitigations

### Assignment Is Currently Lowered Too Late

Risk: selecting Index before recognizing assignment loses the original mutable
receiver path and can silently preserve the old MIR bypass.

Mitigation: make assignment place context an input to expression/secondary
lowering and add nested-place tests before deleting fallback code.

### Mutable Fixed-Array Coercion

Risk: existing candidate generation primarily constructs shared slice borrows,
so IndexMut may fail or accidentally call Index.

Mitigation: add an explicit mutable array-to-slice candidate and test exact
authority plus borrow behavior.

### Generic Stdlib Impl Overlap

Risk: `*T` also structurally matches `*[U]`, while raw slice pointers require
different output and bounds behavior.

Mitigation: constrain the sized pointer impl with marked Sized and provide a
separate `*[T]` impl. Test that selection finds exactly one applicable impl.

### Returned Reference Origins

Risk: replacing direct MIR places with method calls may expose missing call
return-origin propagation, especially for mutable references.

Mitigation: add escape and alias tests for local, parameter-derived, shared, and
mutable indexed references. Fix reference-origin propagation in the ordinary
call path rather than restoring index-specific MIR logic.

### Stdlib Bounds Message

Risk: moving bounds checks out of compiler assertions could change observable
failure output.

Mitigation: centralize one private stdlib helper and retain integration tests for
the exact standard message and failing exit status.

### Artifact Schema Expansion

Risk: partial IndexMut bundles or split provider ownership could survive
serialization.

Mitigation: use complete typed bundles, current-owner filtering, load-time shape
validation, explicit ID remapping, and a format-39 hard break.

### Partial Fallback Deletion

Risk: removing the selection variant while leaving TypeFacts, MIR projection
synthesis, or fixture repair would preserve semantic debt under another path.

Mitigation: use the enumerated residue audit across all tracked production Rust
files and classify every remaining MIR index operation.

## Final Architectural Invariant

`[]` has no compiler-owned meaning. Collection binds Index and IndexMut protocol
roles to canonical IDs. Lowering chooses the protocol from value versus mutable
place context. Selection chooses an explicit stdlib or user impl. Mono produces
a concrete instance. MIR and codegen execute an ordinary call returning a shared
or mutable reference. Arrays, slices, pointers, raw slice pointers, and Vec are
indexable only because explicitly loaded stdlib source implements those traits
with existing low-level primitives. If any protocol, impl, projection, symbol,
or body fact is absent, compilation fails at its owning phase instead of
recovering behavior from type shape, names, or targetless fallback authority.
