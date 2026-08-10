# Borrow Checker Redesign

Date: 2026-04-12
Status: Approved

## Overview

Redesign Rock's MIR borrow checker to match Rust internal borrow semantics for the language features Rock supports today. The new design replaces the current ad hoc active-loan tracking and `reference_lifetimes`-driven correctness path with a borrow-checker-owned model based on explicit borrow facts, place/projection overlap, provenance-aware reborrows, and use-based loan liveness.

The goal is not just to reject the same source programs as Rust in common cases. The goal is to align Rock's internal borrow semantics with Rust's model where those semantics surface in currently supported Rock features, including closures and raw pointers.

## Goals

- Match Rust internal borrow semantics for Rock's currently supported feature set.
- Replace local-only alias checks with place/projection-sensitive conflict checking.
- Replace the current `reference_lifetimes` correctness dependency with borrow-check-owned loan liveness.
- Make closure captures first-class borrow-check inputs.
- Preserve Rust's borrow-check boundary around raw pointers and `unsafe`.
- Keep the redesign scoped to features Rock already implements.

## Non-Goals

- Matching Rust diagnostic wording exactly.
- Designing for unsupported Rust features that Rock does not yet implement.
- Reproducing rustc's internal architecture one-for-one.
- Keeping the current permissive behavior where it disagrees with Rust semantics.

## Current Gaps

The existing checker already uses MIR, but its core semantics differ from Rust in several important ways:

- Shared borrows are currently too permissive in some cases, including programs Rust would reject after assignment to or move from a shared-borrowed place.
- Place conflict checking is largely local-based rather than projection-aware.
- Borrow lifetime shortening depends on the separate `reference_lifetimes` MIR rewrite pass instead of borrow-checker-owned liveness.
- Borrow checks are centralized in a single `mod.rs` with ad hoc logic rather than split into specialized components.
- Closure capture behavior is not currently modeled as borrow-check-visible facts.
- Raw pointers exist in the language today, but the checker does not explicitly define the Rust-like boundary between reference semantics and raw-pointer semantics.

## Recommended Approach

Implement a Rust-style MIR borrow checker within Rock's existing MIR pipeline rather than patching the current checker incrementally or attempting a wholesale clone of rustc.

This approach keeps Rock's current architectural boundary, avoids overbuilding for unsupported language surface, and still allows the checker to align with Rust semantics for places, loans, reborrows, closure captures, control-flow merges, and the reference/raw-pointer boundary.

## Architecture

The redesign remains MIR-based and is organized around five borrow-checker-owned layers:

1. MIR access classification
2. Borrow set and provenance construction
3. Use-based loan liveness
4. Forward access validation
5. Unsafe/raw-pointer semantic boundary

The checker should consume MIR plus explicit closure-capture facts and produce diagnostics through the existing diagnostics system.

### 1. MIR Access Classification

Walk each MIR statement and terminator and classify every place interaction into a small set of access kinds:

- shared read
- mutable write
- move
- borrow creation
- drop
- closure capture
- raw-pointer creation from a place or reference

This classification becomes the canonical input for later analyses and replaces scattered direct checks in the current borrow-check pass.

### 2. Borrow Set And Provenance Construction

Build a stable borrow table for every source of reference semantics in MIR:

- each `Rvalue::Ref`
- each closure capture whose semantics are shared borrow, mutable borrow, or move

Each borrow record should carry enough information to support Rust-like overlap and reborrow reasoning:

- `BorrowId`
- borrow kind
- borrowed root place
- full borrowed place with projections
- origin MIR location
- storage owner for the resulting reference value or capture slot
- provenance parent for reborrows
- any reserve/activation metadata required by the currently supported receiver/method surface

This layer is also responsible for resolving provenance through deref chains so accesses like `&mut *r` and later operations on `*r` are understood in terms of the original place being borrowed.

### 3. Use-Based Loan Liveness

Replace `reference_lifetimes` as a correctness mechanism with a backward liveness analysis computing where each borrow must remain live.

A borrow stays live while:

- the reference value may still be used later
- a reborrow derived from it may still be used later
- a closure containing the capture may still be used later

This makes the checker's notion of loan lifetime derive from actual use semantics rather than inserted `StorageDead` statements for reference locals.

### 4. Forward Access Validation

Once borrow facts and liveness are known, validate each MIR access against the set of live overlapping borrows at that program point.

The access rules should follow Rust semantics for Rock's supported features:

- reads require shared compatibility
- writes require mutable exclusivity
- moves require mutable exclusivity where Rust would require it
- drops are checked as exclusive accesses
- creating `&mut` requires exclusive access
- creating `&` is rejected in the presence of overlapping live mutable borrows
- closure capture is checked according to whether the capture is shared-borrow, mutable-borrow, or move

This is the layer that should reject currently-permitted Rock programs such as assignment to or move from places that Rust would consider actively borrowed.

### 5. Unsafe And Raw Pointer Boundary

Borrow checking should follow Rust's semantic boundary:

- references are borrow-checked
- raw pointers are not made safe by borrow checking
- `unsafe` remains the syntactic and lowering gate for raw-pointer dereference, indexing, and arithmetic

Borrow check is still responsible for correct behavior before the raw-pointer boundary. For example, creating a raw pointer from a reference or from a borrowed place must interact correctly with the originating borrow semantics where Rust's model does. Later raw-pointer dereference is not rejected by borrowck merely because aliasing would be dangerous; that remains an `unsafe` responsibility.

## Supported Semantic Surface

The first milestone targets Rust-parity borrow behavior for the Rock features that exist today:

- locals
- field projections
- dereference and reborrow chains
- branches
- loops
- control-flow merge points
- function calls
- method receivers
- closures and closure captures
- raw pointers and `unsafe` blocks, at Rust's borrow-check boundary

The design should not take on unsupported Rust-only features unless Rock already implements them.

## Semantic Model

### Place-Based Borrowing

Borrowing is tracked on MIR places rather than only on locals.

- `pair.left` and `pair.right` can be non-overlapping.
- `x`, `*r`, and a reborrow derived from `r` may alias depending on provenance.
- conflict checking must compare projection paths and overlap, not merely local identity.

### Reborrows And Provenance

Reborrows must be represented explicitly so the checker knows when a later access is derived from an earlier borrow.

This is required for Rust-like handling of:

- `&*r`
- `&mut *r`
- dereference chains used in reads or writes
- method receivers that implicitly borrow a place

### Closure Captures

Closures are in scope for the first milestone. The checker must see capture semantics directly rather than reconstruct them from codegen-only information.

Closure capture analysis should classify each capture as one of:

- shared borrow capture
- mutable borrow capture
- by-value move capture

The checker must then treat the closure value as carrying those capture obligations for as long as the closure may still be used. This includes cases where the closure is stored, passed, moved, or called multiple times.

### Raw Pointers

Raw pointers are also in scope for the first milestone.

The checker must preserve Rust's split between reference semantics and raw-pointer semantics:

- reference creation and movement are borrow-checked
- casting or converting to raw pointers must respect the originating borrow semantics where applicable
- raw-pointer dereference, indexing, and arithmetic remain `unsafe` operations and are not made alias-safe by borrow checking

## Module Layout

Do not concentrate the redesign in a single `mod.rs`. Split the code into specialized modules and keep `mod.rs` as a thin coordinator.

Recommended structure:

- `lib/src/mir/borrowck/mod.rs`
- `lib/src/mir/borrowck/accesses.rs`
- `lib/src/mir/borrowck/borrows.rs`
- `lib/src/mir/borrowck/liveness.rs`
- `lib/src/mir/borrowck/conflicts.rs`
- `lib/src/mir/borrowck/provenance.rs`
- `lib/src/mir/borrowck/closures.rs`
- `lib/src/mir/borrowck/diagnostics.rs`

Possible support modules:

- `lib/src/mir/borrowck/place.rs`
- `lib/src/mir/borrowck/location.rs`

Responsibilities:

- `accesses.rs`: classify MIR operations into borrow-relevant access events
- `borrows.rs`: construct borrow facts and stable borrow identifiers
- `liveness.rs`: compute use-based loan liveness
- `conflicts.rs`: determine projection overlap and compatibility
- `provenance.rs`: resolve deref/reborrow provenance
- `closures.rs`: expose closure-capture facts to MIR borrow checking
- `diagnostics.rs`: build structured error reporting from conflicts and uses
- `mod.rs`: sequence the analyses and expose the pass entrypoint

## Data Flow

The intended data flow is:

1. Lower source/HIR into MIR while preserving enough structure for place-based checking.
2. Materialize closure-capture facts before MIR borrow checking runs.
3. Build access facts from MIR.
4. Build borrow facts and provenance relationships.
5. Run backward loan liveness.
6. Run forward access validation using the live-borrow state at each point.
7. Emit diagnostics with spans for both use/access sites and conflicting borrow origins where available.

## Diagnostics

Keep the existing `Diagnostics` system, but improve borrow-related reporting to better reflect Rust-like semantics.

The checker should report at least these categories:

- use after move
- move while borrowed
- write or assignment while borrowed
- conflicting shared/mutable borrows
- invalid mutable borrow from immutable binding
- closure capture conflicts

Where possible, diagnostics should point to both:

- the invalid access or use site
- the conflicting borrow, move, or capture origin

The goal is semantic accuracy, not exact rustc wording.

## Migration Strategy

Implement the redesign as a replacement path, not as a layer of exceptions on top of the existing checker.

Recommended sequence:

1. Introduce the new `mir::borrowck` module tree.
2. Move closure-capture facts into lowering or MIR construction so borrowck consumes explicit capture semantics.
3. Implement access classification, borrow facts, provenance, and overlap logic.
4. Implement use-based liveness.
5. Implement forward validation and diagnostics.
6. Switch the compiler to the new borrow-check path.
7. Remove or sharply reduce `reference_lifetimes` from the correctness path.
8. Delete superseded ad hoc aliasing logic once parity tests cover the targeted surface.

The old permissive behavior should not be preserved when it conflicts with Rust semantics.

## Testing Strategy

Testing should prove semantic parity for Rock's supported surface rather than merely guarding individual regressions.

### Unit Tests

Add focused tests in the new borrow-check modules for subtle logic:

- place overlap and disjointness
- provenance and reborrow resolution
- liveness propagation across branches and loops
- merge-point loan behavior
- closure capture classification
- raw-pointer boundary behavior

### Source-Level Integration Tests

Extend `lib/tests/integration.rs` with parity-oriented source tests for cases such as:

- shared borrow then assignment
- shared borrow then move
- mutable borrow then read/write
- disjoint field borrows
- reborrow chains through dereference
- method receiver borrow edge cases
- closure captures by shared borrow, mutable borrow, and move
- closure moves or stores extending capture lifetime
- raw pointer creation from references
- `unsafe` raw-pointer operations remaining syntactically allowed without weakening reference borrow semantics

### Success Criterion

For the first milestone, accepted versus rejected behavior should match Rust borrow semantics for the Rock features currently implemented, including closures and raw pointers.

## Risks And Constraints

- Closure capture facts currently appear to be derived late, near codegen, so earlier materialization may require coordinated lowering and MIR-builder changes.
- Precise projection overlap and provenance are necessary to avoid both false positives and false negatives.
- Raw-pointer support must not tempt the checker into pretending unsafe operations are borrow-checked for safety.
- The design should stay bounded to Rock's real feature set and avoid importing unsupported rustc complexity.

## Summary

The redesign replaces Rock's current borrow checker with a specialized, modular MIR borrow-check pipeline that:

- uses Rust-like place/provenance semantics
- owns loan liveness directly
- treats closure captures as first-class borrow facts
- respects Rust's raw-pointer boundary
- avoids centralizing all logic in one `mod.rs`

This is the smallest design that can realistically target Rust internal borrow semantics for Rock's currently supported language surface.
