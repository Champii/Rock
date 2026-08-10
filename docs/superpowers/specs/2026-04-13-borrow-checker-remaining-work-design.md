# Borrow Checker Remaining Work

Date: 2026-04-13
Status: Draft

## Overview

The current branch has the major borrow-check semantics fixes in place and the full `rock-lib` test suite passes, but the original borrow-check redesign plan is not fully implemented as written. This document narrows the remaining work to the concrete gaps that still matter for architectural compliance and long-term correctness.

The goal of this follow-up is not to restart the redesign. The goal is to finish the remaining pieces with the smallest changes that make the implementation match the approved design closely enough to stop carrying split sources of truth and misplaced borrow-check responsibilities.

## Verified Current State

- `mir::borrowck` is the active compiler entrypoint.
- The compiler no longer runs `reference_lifetimes` as a correctness pass before borrow checking.
- Shared-borrow, closure-capture, branch-merge, deref-conflict, and raw-pointer boundary regressions are covered and currently pass.
- The implementation is still only partially aligned with the original redesign architecture in four areas:
  - provenance ownership
  - closure-capture ownership
  - MIR place fidelity
  - lambda MIR scope

## Remaining Problems

### 1. Provenance ownership is split

`mir::borrowck/provenance.rs` currently provides only a small `borrowed_root` helper. The effective reborrow/root resolution logic still lives in `mir/dataflow/analyses/loans.rs`.

This means the new borrow checker still depends on borrow-sensitive logic owned outside the `mir::borrowck` module tree, which conflicts with the redesign goal of keeping borrow semantics modular and centered in `mir::borrowck`.

### 2. Closure captures still have two authorities

HIR and MIR now carry explicit closure capture facts, but codegen still maintains and rebinds a separate `closure_captures` map while compiling lambdas.

This is an architectural problem even when tests pass: borrow checking and code generation can drift if capture ownership rules are interpreted in two places.

### 3. MIR lowering still has semantic fallbacks

Some field/index/tuple lowering paths still fall back to `Unit` instead of preserving place structure or an explicit non-place representation.

That is acceptable for irrelevant expressions, but not for borrow-sensitive expressions where place fidelity is required for overlap, provenance, or raw-pointer boundary behavior.

### 4. Lambda MIR scope is unresolved

The approved redesign plan expected lambda creation to lower into synthetic MIR functions plus explicit closure creation in MIR. The current implementation only lowers closure creation and capture metadata; it does not build synthetic MIR functions for lambda bodies.

This is the largest remaining design question. It may not be required for Rock's current supported borrow surface if explicit capture facts are already sufficient, but that needs to be decided explicitly rather than left as accidental partial implementation.

## Recommended Approach

Finish the remaining work in two phases.

### Phase A: Complete the architectural gaps that are clearly required

This phase should be implemented now.

1. Move provenance/reborrow resolution into `mir::borrowck/provenance.rs`
2. Make `mir::borrowck/mod.rs` consume provenance helpers directly
3. Reduce `mir/dataflow/analyses/loans.rs` to generic loan-state mechanics rather than borrow semantics ownership
4. Remove codegen's independent closure-capture authority and make it consume HIR/MIR-visible capture facts only
5. Remove remaining MIR `Unit` fallback paths where borrow-sensitive place structure should be preserved

### Phase B: Resolve lambda MIR scope explicitly

This phase starts only after Phase A is complete.

At that point, evaluate whether the first milestone truly requires synthetic MIR functions for lambda bodies.

Decision rule:

- If any supported borrow semantics still require borrowck to inspect lambda body MIR directly, implement synthetic MIR lambda functions.
- If current supported semantics are fully covered by explicit capture facts plus closure value liveness, document that synthetic lambda MIR is deferred as out of scope for the first milestone.

This keeps the branch honest: either we implement lambda MIR, or we intentionally narrow the first milestone and update the design/plan accordingly.

## Detailed Design

### A. Provenance becomes a borrowck-owned layer

`mir::borrowck/provenance.rs` should own:

- borrowed-root normalization
- reborrow-parent tracking
- resolution from derived reference places back to original borrowed places where current semantics need it

`borrowck/mod.rs` should call provenance helpers before conflict validation. Borrow-sensitive place reasoning should no longer be hidden inside `dataflow/analyses/loans.rs`.

Success criterion:

- no borrow-semantic root-resolution logic remains outside `mir::borrowck`
- existing deref/reborrow regressions stay green

### B. Closure captures become single-source-of-truth data

Closure capture classification is already computed during lowering and preserved into MIR. Codegen should consume those facts instead of discovering or rebinding captures through its own mutable bookkeeping.

This does not require changing closure runtime representation unless codegen currently depends on its side map for layout. If it does, that side map should be derived from the earlier capture facts, not treated as independent truth.

Success criterion:

- borrowck and codegen read the same capture facts
- removing codegen-side rediscovery does not change closure behavior tests

### C. MIR place fidelity is completed where semantics depend on it

Audit the remaining `Unit` fallback paths in MIR lowering and divide them into two buckets:

- acceptable non-place lowering where borrow semantics do not depend on structure
- unacceptable borrow-sensitive fallback where place or explicit cast/index semantics must survive into MIR

Only the second bucket must be fixed in this follow-up.

Success criterion:

- no borrow-sensitive field/index/tuple/cast path collapses into `Unit`
- existing MIR-builder tests remain green
- add focused tests if a missing path is fixed

### D. Lambda MIR scope gets an explicit milestone decision

After Phase A, reassess the remaining first-milestone requirements:

- closure captures by shared borrow, mutable borrow, and move
- closure use extending capture lifetime
- stored/passed/moved closures behaving correctly under current supported surface

If those are fully enforced from explicit capture facts and closure-value liveness, the first milestone can remain capture-fact-driven without synthetic MIR lambda bodies.

If not, add synthetic MIR lambda lowering as a dedicated follow-up implementation with its own tests and plan.

### Lambda MIR Scope Decision

For the first milestone, explicit closure capture facts plus closure-value liveness are sufficient to enforce the currently supported closure borrow semantics covered by the branch test suite. Synthetic MIR lambda-body lowering is therefore deferred to a later milestone unless a supported borrow rule is found that requires borrowck to inspect closure body MIR directly.

## Testing Strategy

### Provenance

- keep existing deref/reborrow conflict coverage green
- add unit coverage in `mir::borrowck/provenance.rs` for any new normalization or parent-resolution logic

### Closure ownership

- rerun `test_closure_capture`
- rerun closure borrow parity tests
- rerun any codegen-facing closure tests after removing codegen rediscovery

### MIR place fidelity

- rerun the existing MIR builder tests for field, tuple, and cast lowering
- add one focused test for any newly-fixed borrow-sensitive fallback path

### Broad verification

- rerun the targeted borrow regression set serially
- rerun `cargo test -p rock-lib`

## Non-Goals

- changing already-correct borrow semantics just to match the exact internal shape of the old plan
- refactoring unrelated codegen or MIR builder behavior
- adding support for unsupported Rust borrow-check features

## Success Criteria

This remaining work is complete when all of the following are true:

1. borrow-semantic provenance logic is owned by `mir::borrowck`
2. closure captures have one semantic source of truth from lowering through borrowck and codegen
3. borrow-sensitive MIR lowering no longer falls back to `Unit`
4. the lambda MIR question is explicitly resolved, either by implementation or by a deliberate milestone-bound deferral
5. targeted borrow regressions and `cargo test -p rock-lib` pass
