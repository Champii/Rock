# Compiler Ownership Architecture Rewrite Design

## Goal

Establish Rock's production ownership core around explicit MIR-level invariants. The RAII, implicit-receiver, artifact, and stdlib ownership changes exposed gaps in receiver adjustment, borrow validation, temporary lifetime handling, drop obligations, artifact specialization, allocation safety, and test infrastructure. This rewrite should fix the current branch findings and create architecture that prevents the same classes of bugs from recurring.

## Scope

This design covers a staged architecture rewrite inside the existing compiler pipeline. It does not redesign the whole compiler, parser, HIR model, or code generator. It does introduce a stronger MIR ownership-analysis layer that becomes the central authority for initialization, moves, loans, temporary escapes, and drop obligations.

In scope:

- Receiver adjustment and method selection semantics.
- A MIR ownership-analysis layer for initialization, moves, loans, borrow conflicts, temporary escapes, and drop obligations.
- Borrow-origin and temporary-scope validation for references.
- MIR drop obligation representation and direct-drop glue requirements.
- Product artifact body-retention rules for downstream monomorphization.
- Stdlib allocation/container safety primitives.
- Integration-test stdlib artifact cache infrastructure.

Out of scope:

- Full Rust-compatible region inference in this branch.
- General region-polymorphic reference types in this branch.
- A wholesale parser, HIR, MIR, or codegen replacement.
- Compiler-owned stdlib discovery or implicit stdlib loading.

## Architecture Principles

- Selection chooses legal candidates, not merely plausible names.
- Lowering applies explicit adjustments; it should not infer hidden mutability after selection.
- Ownership analysis over MIR validates initialization, moves, loans, reference escapes, and drop obligations before codegen.
- MIR represents enough ownership structure that codegen only lowers explicit MIR semantics to LLVM.
- Artifact serialization and monomorphization share one predicate for downstream-specialization needs.
- Stdlib safe APIs centralize allocation and arithmetic safety instead of duplicating checks in containers.
- Test cache reuse must be deterministic, validated, and safe across concurrent test processes.

## MIR Ownership Core

The compiler should have one ownership-analysis layer over MIR. This layer should absorb the currently scattered responsibilities for move validation, initialization state, borrow conflicts, temporary escape checks, and cleanup/drop obligation validation.

Responsibilities:

- Track initialization and move state for locals and projections.
- Track active loans for shared and mutable borrows.
- Reject mutable/shared borrow conflicts using MIR places and projections.
- Track temporary scopes created by lowering and MIR construction.
- Validate that returned or stored references do not outlive their referents.
- Validate drop obligations before codegen, including required direct-drop glue.

Inputs:

- MIR locals, places, projections, statements, and terminators.
- Type information from `TypeContext`.
- Receiver adjustment metadata from lowering/selection.
- Temporary scope metadata from lowering/MIR construction.
- Direct-drop and structural-drop metadata from MIR construction.

Outputs:

- Structured diagnostics for user-visible ownership errors.
- Internal compiler errors for violated MIR/codegen invariants.
- A validated MIR program that codegen can lower without reinterpreting ownership policy.

Implementation stance:

- Prefer extending the existing MIR where the model remains clear.
- If an invariant requires awkward side channels or repeated inference, introduce explicit MIR metadata or new MIR statement/terminator fields.
- Do not attempt full Rust-style region inference in this branch; implement conservative escape validation that rejects ambiguous cases.

## Receiver Adjustment Model

Receiver selection should return a concrete adjustment plan with the selected method. The adjustment plan is part of method-call semantics and must be applied by lowering before MIR.

Adjustment kinds:

- `None`: receiver type already matches the method receiver.
- `AutorefShared`: borrow a receiver as `&T`.
- `AutorefMut`: borrow a mutable lvalue receiver as `&mut T`.
- `MutToSharedRef`: coerce `&mut T` to `&T`.
- `BuiltinDeref`: dereference a built-in reference.
- `TraitDeref`: call a `Deref` implementation.
- `ArrayRefToSliceRef`: coerce `&[T; N]` to `&[T]`.
- `ArrayValueToSliceRef`: borrow an array value as a slice reference when legal.
- `ArrayValueToSliceValue`: produce slice value semantics where the language already supports it.

Rules:

- `&mut Self` may only match an existing mutable reference or a mutable lvalue that can be adjusted with `AutorefMut`.
- An immutable local, temporary, shared reference, or non-lvalue expression must not select a mutable receiver method.
- `&Self` may match shared references, mutable references via `MutToSharedRef`, or values through `AutorefShared` when borrowing is legal.
- `Self` by-value methods require move/copy semantics, not reference adjustment.
- Selection must carry the adjustment plan to lowering; lowering must emit the corresponding HIR shape and preserve source spans for diagnostics.

Expected diagnostics:

- Calling a mutable receiver method on an immutable binding should report that the receiver is not mutable.
- Calling a mutable receiver method through `&T` should report that a mutable reference is required.
- Calling a by-value receiver method on a borrowed value should report that the value cannot be moved out of the reference.

## Borrow-Origin And Temporary Escape Model

The ownership core should track enough reference provenance to reject references that outlive their referent. This is intentionally smaller than full Rust-compatible region inference, but it should be a first-class MIR analysis rather than a one-off lowering check.

Reference origins:

- `Param`: a function or method parameter.
- `Local`: a named local binding.
- `Field`: a projection from a longer-lived owner.
- `Deref`: a projection through a pointer/reference whose safety is governed by the base.
- `Temporary`: a compiler-created expression temporary or call result.
- `Static`: string literals and other statically valid references.
- `UnknownExternal`: external values where the compiler cannot inspect internals but can still enforce local escape rules.

Escape rules:

- Returning `&Temporary` is rejected.
- Returning a reference to a local that dies at function exit is rejected unless it is tied to an input reference that remains valid.
- Storing `&Temporary` into a local is allowed only within the temporary's statement lifetime and cannot escape that statement or block.
- Direct patterns like `&self.as_slice!` are rejected when the call result is a temporary slice value.
- Deref implementations must not return references to temporaries created in their body.

Implementation boundary:

- Lowering should annotate reference expressions and compiler-created temporaries with origin/scope metadata where practical.
- MIR construction should preserve temporary scopes and reference origins in a form the ownership analysis can inspect.
- The MIR ownership analysis is the final authority for escape validation because it sees moves, locals, projections, and returns.
- The first implementation should be conservative and reject ambiguous escapes rather than accepting unsound code.

## Drop Obligation Model

MIR should represent what kind of cleanup is required, and the ownership analysis should validate those obligations before codegen. Codegen must not infer drop policy from missing metadata.

Drop obligation kinds:

- `DirectDrop`: invoke user `Drop` glue for a type with a direct `Drop` impl.
- `StructuralDrop`: recursively drop fields, tuple elements, array elements, or enum payloads that need cleanup.
- `NoDrop`: no cleanup needed.

Rules:

- A `DirectDrop` terminator must always resolve to drop glue metadata. Missing glue is a compiler error.
- `StructuralDrop` should be elaborated into child obligations before codegen or represented explicitly enough that codegen never treats missing direct glue as no-op.
- Partial moves from direct-`Drop` owners remain rejected.
- Structural cleanup must still occur for fields of non-direct-drop owners.
- Return-place exceptions must be explicit and documented as a MIR invariant.
- Drop obligation validation belongs to the MIR ownership core; codegen only enforces that already-validated required symbols exist.

Expected diagnostics:

- Missing required drop glue should produce an internal compiler/codegen error that names the MIR function and type where possible.
- User-facing move errors should remain span-aware and explain the direct-`Drop` owner rule.

## Artifact Specialization Predicate

Product artifact writing, artifact loading, crate context capability checks, and monomorphization should share one definition of when a body must be available to downstream consumers.

Shared predicate:

- A function body must be serialized when the function has generic parameters or unresolved type variables that require downstream specialization.
- An impl body must be serialized when the impl has type generics, trait generics, or any method with method-level generic parameters or generic parameter IDs.
- Concrete object-provided impls with no downstream specialization needs can remain object-backed only.

Rules:

- The product writer must not drop a non-generic impl that contains generic methods.
- The artifact loader must expose the same body-provider set that the monomorphizer expects.
- The monomorphizer should not eagerly emit dependency generic impl methods unless they are actually referenced.

## Stdlib Ownership Primitives

Safe stdlib containers should call shared primitives for allocation and capacity arithmetic.

Required primitives:

- `require_non_zero_size`: preserve the branch's conservative ZST rejection policy.
- `checked_alloc`: allocate non-zero layouts and abort on null.
- `checked_mul_i64`: abort on overflow or negative operands where sizes/capacities require non-negative values.
- `checked_add_i64`: abort on overflow for capacity and length arithmetic.
- `checked_grow_capacity`: compute container growth without overflow.

Rules:

- `RawBuffer::with_capacity` must validate `elem_size * cap` before allocation.
- `Vec::push` must validate length increment and growth capacity.
- `HashMap::grow` must validate doubled capacity and all buffer byte sizes.
- `Box::new`, `Vec`, `RawBuffer`, and `HashMap` continue rejecting ZST values in safe APIs for this branch.

## Vec Deref Policy

The current `Vec` deref implementation must not return `&self.as_slice!`, because that borrows a temporary slice value.

Acceptable resolutions:

- Remove `impl Deref for Vec T` and require explicit `as_slice!` until a stable slice-reference representation exists.
- Or change the Deref target/model so the returned reference points at stable state, not a temporary.

Preferred resolution for this branch:

- Remove or avoid the unsafe `Vec` Deref implementation.
- Keep explicit `as_slice!` APIs and tests.
- Add compiler tests that reject returning references to call-result temporaries, including a deref-like method body.

## Test Artifact Cache Infrastructure

The integration test stdlib artifact cache should remain persistent across test invocations but must be safe for concurrent processes.

Rules:

- Cache key includes artifact format version, compiler/test binary stamp, checkout path, and stdlib source contents.
- Population uses a lock file per cache key.
- Build output goes to a unique temporary directory.
- Publish is atomic: rename the completed directory or write a final marker after both `.rkca` and `.o` exist.
- Reuse validates the final marker and that the artifact can be deserialized.
- Failed or interrupted population must not leave a reusable partial cache.

## Testing Strategy

Add tests at the lowest phase that owns the invariant, plus integration coverage for user-visible behavior.

Receiver tests:

- Immutable binding cannot call a `^@` method.
- Shared reference cannot call a `^@` method.
- Mutable binding can call a `^@` method through explicit mutable adjustment.
- `&mut T` can call `@` shared methods through mut-to-shared coercion.

Borrow-origin tests:

- Returning `&make_value!` fails.
- Returning `&self.as_slice!` fails.
- Returning an input reference succeeds.
- Returning a string literal/static reference succeeds.

Drop tests:

- Missing direct drop glue fails loudly in codegen/unit tests.
- Generic wrapper fields containing `Box<T>` get cleanup.
- Partial moves from direct-`Drop` owners remain rejected.

Artifact tests:

- Product artifacts preserve non-generic impls with generic methods.
- Consumers specialize such dependency methods only when called.
- Dependency generic impls are not eagerly emitted.

Stdlib tests:

- Overflowing RawBuffer/Vec/HashMap capacities fail safely where expressible.
- ZST rejection behavior remains covered.
- `HashMap::get` references block later mutation while live.

Cache tests:

- Cache key changes when stdlib source changes.
- Reusing an existing valid cache does not rebuild.
- Partial cache without final marker is ignored.

## Migration Plan

This should be implemented in stages so each invariant becomes testable before the next stage depends on it.

Stage 1: Receiver adjustment architecture.

Stage 2: MIR ownership-analysis skeleton for initialization, moves, loans, and temporary scopes.

Stage 3: Borrow-origin and temporary escape checks on top of the ownership-analysis layer.

Stage 4: Drop obligation representation and codegen enforcement.

Stage 5: Shared artifact specialization predicate.

Stage 6: Stdlib allocation primitives and `Vec` deref removal/fix.

Stage 7: Concurrent-safe stdlib artifact cache.

## Acceptance Criteria

- All seven review findings are fixed by architectural invariants, not one-off patches.
- Existing RAII, implicit receiver, borrowed Eq/Ord, stdlib, artifact, and integration tests continue passing.
- New negative tests fail before the relevant implementation and pass afterward.
- Codegen no longer silently ignores missing direct drop glue.
- The compiler rejects returning references to temporaries such as `&self.as_slice!`.
- Product artifacts preserve every body needed for downstream specialization.
- Safe stdlib containers cannot allocate undersized buffers through integer overflow.
- Repeated integration test invocations reuse the stdlib artifact cache without rebuilding, and concurrent processes cannot consume partial cache entries.

## Risks And Trade-Offs

- Conservative temporary escape checks may reject programs that a future full lifetime system could accept. This is acceptable for production safety.
- Centralizing ownership validation may require moving existing checks out of lowering/MIR construction over time. The first pass should avoid broad churn where current phase boundaries remain sound.
- Receiver adjustment changes may expose existing tests that relied on permissive selection. Those tests should be updated only if the old behavior was unsound.
- Removing `Vec` deref may require minor test/example updates to call `as_slice!` explicitly.
- Cache locking should avoid platform-specific APIs beyond standard file creation/rename patterns where possible.

## Future Roadmap

These items are good long-term compiler directions, but they are not required to land this branch safely.

- Region-polymorphic references: add explicit lifetime/region parameters when Rock needs APIs that express which input reference a returned reference is tied to.
- MIR v2: introduce a revised MIR only if incremental extensions make ownership invariants harder to express or verify. The goal is stronger invariants, not format churn.
- Full Rust-compatible lifetime inference: consider this only after the conservative ownership core proves too imprecise for real Rock programs.
- More precise temporary promotion/static analysis: allow more safe reference patterns once the ownership core can prove them.

## Review Decisions

These decisions should be settled before the implementation plan is finalized.

- Whether `Vec` should regain `Deref` in this branch through a stable slice-reference representation, or whether explicit `as_slice!` is the intended API until slice references are modeled more deeply.
- Whether borrow-origin metadata should live directly in MIR locals/places/statements or in a side table keyed by MIR local/place IDs.
- Whether drop obligation kinds should be represented as a new MIR terminator field, separate MIR metadata, or an explicit pre-codegen drop-elaboration product.
