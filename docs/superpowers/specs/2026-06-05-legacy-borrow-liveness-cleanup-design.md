# Legacy Borrow Liveness Cleanup Design

## Context

Roadmap Task 20 converted active borrow checking to typed MIR locations, `LoanId`, `LoanTable`, `LoanState`, and dense local/loan sets. The remaining legacy island is `compute_live_borrows` in `lib/src/mir/borrowck/liveness.rs`, which still uses `BorrowId`, `LiveBorrowSet`, `HashMap`, and `HashSet`. The active compiler path no longer consumes that helper.

## Goal

Make `LoanId` the only borrow/loan identity used by borrow checking. Remove the legacy borrow-liveness helper and delete `BorrowId` instead of preserving a transitional compatibility bridge.

## Non-Goals

- Do not move active loan propagation into the generic MIR dataflow engine in this slice.
- Do not change user-facing borrow diagnostics except for removing the unused fallback from `BorrowId`-indexed `BorrowData`.
- Do not expand `MovePathId` / `PlacePathId` modeling beyond the state already landed in the indexed place-path slice.

## Architecture

`BorrowData` remains a temporary collection record for constructing `LoanTable` entries. It should contain semantic facts needed to create `LoanData`: owner, borrowed place, access kind, creation location, and origin span. It should not carry its own ID.

`LoanTable::from_borrows` remains the ID allocation boundary. It assigns canonical `LoanId`s in table order and preserves origin span, place, place path, owner, kind, and creation location in `LoanData`.

The active liveness path remains:

1. Collect `BorrowData` facts from MIR statements and terminators.
2. Build a `LoanTable` from collected facts.
3. Group loans by typed `Location` with `group_loans_by_location`.
4. Compute reference owner liveness with `ReferenceLiveness` / `LocalSet`.
5. Propagate active loans with `LoanState` and report conflicts from `LoanData`.

## Components

- `lib/src/mir/borrowck/borrows.rs`: remove `BorrowId`, remove `BorrowData.id`, and remove post-collection ID rewriting.
- `lib/src/mir/borrowck/liveness.rs`: delete `LiveBorrowSet`, `compute_live_borrows`, and the private helper functions that exist only for that legacy helper.
- `lib/src/mir/dataflow/analyses/loans.rs`: keep `LoanTable` as the canonical ID allocator and update tests that build `BorrowData`.
- `lib/src/mir/borrowck/mod.rs`: remove the diagnostic fallback that looks up `borrows[LoanId.index()]` after checking `LoanData.origin_span`.
- Documentation: update `master-audit-checklist.md` and `2026-05-17-compiler-architecture-ordered-roadmap.md` after verification to mark the legacy helper removed and leave only real follow-up work.

## Error Handling And Diagnostics

Borrow conflict diagnostics should source the borrow span from `LoanData.origin_span`. If a loan has no origin span, diagnostics keep the current default-span fallback. No diagnostics path should depend on a positional relationship between `LoanId` and an external `BorrowData` slice.

## Testing

Use TDD for the cleanup:

- First update or replace the legacy `compute_live_borrows` unit test with active-path tests that prove `LoanTable` assigns canonical `LoanId`s and preserves origin spans/creation locations.
- Run focused tests for `mir::borrowck`, `mir::dataflow`, and relevant borrow filters.
- Run formatting and the smallest broad verification needed before closing the bead.

## Acceptance Criteria

- `rg "BorrowId|compute_live_borrows|LiveBorrowSet" lib/src/mir --glob '*.rs'` has no production or test hits.
- Active borrow checking still builds loans through `LoanTable` and propagates `LoanState` by `LoanId`.
- Borrow conflict diagnostics use `LoanData.origin_span` without the legacy `BorrowData` fallback.
- `master-audit-checklist.md` and the ordered roadmap accurately state that the legacy `BorrowId` / `compute_live_borrows` helper is removed.
