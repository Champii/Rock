# Borrowck Indexed Dataflow Design

## Goal

Finish roadmap Task 20 by converting MIR borrow checking from ad hoc `HashMap`/`HashSet` loan state to typed locations, indexed metadata tables, and dense dataflow state while preserving current borrow diagnostics and behavior.

## Current State

Task 19 made MIR identities canonical and runtime-complete enough for borrowck to operate over stable MIR semantics. Borrowck still uses several map-heavy structures that make future MIR codegen and deeper borrow analysis harder to reason about:

- `BorrowChecker::check_function` builds `HashMap<LoanId, Loan>` state directly from collected borrows.
- `LoanId` is defined locally in `mir::dataflow::analyses::loans`, even though `crate::ids::LoanId` already exists.
- Borrow creation locations are raw `(usize, usize)` pairs for block and statement indexes.
- Active loan propagation clones full `Loan` values and merges `HashSet<Local>` owner sets.
- Reference liveness uses `HashSet<Local>` at block and statement boundaries.
- Provenance resolution scans active loan maps to find which loan owns a reference local.
- The generic dataflow engine is forward-only and returns only block entry/exit facts, while borrowck needs statement-level facts.

The current behavior is well covered by borrowck unit tests and integration tests. Task 20 must keep those semantics stable while changing the internal representation.

## Non-Goals

- Do not redesign borrow checking rules or intentionally change user-facing diagnostics.
- Do not move codegen to MIR; that remains Task 21.
- Do not implement full Rust-style non-lexical lifetimes beyond the behavior already modeled.
- Do not add broad compatibility shims for the old borrowck loan map representation once the new indexed representation owns the path.
- Do not introduce external bitset crates unless the standard-library-backed representation proves insufficient.

## Architecture

Task 20 should be implemented as one feature boundary, but with tightly ordered internal slices. The end state is an indexed borrowck pipeline:

```text
MIR function
    -> borrow collection with typed locations
    -> loan table with crate::ids::LoanId keys
    -> dense local/loan owner sets
    -> reference-liveness and active-loan dataflow
    -> statement/terminator checking against indexed loan metadata
```

The central principle is to separate immutable loan facts from path-sensitive dataflow state. Loan identity, borrowed place, kind, origin span, owner at creation, and creation location live in indexed tables. Dynamic flow state carries dense sets of active loan IDs and dense owner sets keyed by loan ID.

## Data Model

### Typed MIR Locations

Introduce typed location data under borrowck or MIR dataflow:

- `StatementIndex(pub usize)` identifies a statement within a basic block.
- `Location { block: BasicBlockId, statement: StatementIndex }` identifies statement locations that can create loans or emit diagnostics.

Task 20 does not need terminator locations for existing loan creation, because current borrow collection only creates loans from statements. If future work needs terminator-originated loans, add a separate `LocationKind` then rather than over-generalizing now.

### Dense Indexed Sets

Add a small dense set abstraction for IDs and MIR locals. It should be simple and repository-local:

- `BitSet<I>` stores bits for types implementing `Idx`.
- `LocalSet` stores bits for `Local`, which currently does not implement `Idx`.
- Both sets support insert, remove, contains, union/join, iteration, equality, and empty construction.

The first implementation can use `Vec<bool>` or a compact `Vec<u64>` internally. The API should hide representation so it can be optimized later.

### Loan Tables

Replace the local `mir::dataflow::analyses::loans::LoanId(pub usize)` with `crate::ids::LoanId`.

Introduce:

- `LoanData { id, place, kind, origin_span, created_at, initial_owner }` for immutable loan facts.
- `LoanTable { loans: Vec<LoanData> }` for indexed lookup by `LoanId`.
- `LoanState { active: BitSet<LoanId>, owners: Vec<LocalSet> }` for path-sensitive active loan state.

`LoanState` replaces `HashMap<LoanId, Loan>` in active-loan propagation and checking. A loan is active when its ID is in `active` and its owner set is non-empty. Joining states unions active loans and owner sets per active loan.

### Borrow Records

`BorrowData` should retain its role as the statement-level collection record, but use typed IDs and locations:

- `BorrowId` can remain borrowck-local unless it causes duplication with `LoanId`.
- `block: usize` and `statement: usize` should become `created_at: Location`.
- The borrow-to-loan conversion should allocate `crate::ids::LoanId` through `IdGen<LoanId>` or equivalent monotonic table insertion.

Borrow IDs and loan IDs do not have to be the same type. Borrow IDs represent collection-order source events; loan IDs represent indexed runtime borrow facts used by dataflow.

## Dataflow

### Reference Liveness

Convert `ReferenceLiveness` from `HashSet<Local>` sets to `LocalSet` while preserving the same before/after statement precision:

- `entry_sets: Vec<LocalSet>`
- `exit_sets: Vec<LocalSet>`
- `before_statement_sets: Vec<Vec<LocalSet>>`
- `after_statement_sets: Vec<Vec<LocalSet>>`

The transfer behavior must remain unchanged. Reference-like locals are still identified by `TypeFacts::contains_reference(ty)` or function/pointer types.

### Active Loan Propagation

Replace `compute_active_loan_entries` with an indexed version that takes:

- `LoanTable`
- loans grouped by `Location`
- `ReferenceLiveness`

It should return block entry `LoanState` values. The statement loop in `BorrowChecker::check_function` should keep a mutable `LoanState` and update it after each statement in the same order as today:

1. Check active loan conflicts for the statement.
2. Check initialization for the statement.
3. Apply initialization effects.
4. Transfer owner sets for moves/copies/casts that preserve reference ownership.
5. Activate loans created at the current location.
6. Release dead reference-carried loans using after-statement liveness.
7. Release loans owned by locals that go storage-dead.

This ordering preserves current diagnostics and aliasing behavior.

### Provenance And Conflict Checks

`resolve_place` should stop scanning a `HashMap<LoanId, Loan>` and instead query `LoanState` plus `LoanTable`:

- Find active loans whose owner set contains the current local.
- Use the corresponding `LoanData.place` as the dereference source.
- Preserve the current cycle guard.

`LoanAnalysis::check_aliasing` should iterate active loan IDs, fetch immutable facts from the table, and use owner sets only for ownership-sensitive exceptions such as the existing same mutable dereference access case.

## Error Handling And Diagnostics

- Borrow diagnostics should continue to use `origin_span` from the loan/borrow that caused the conflict.
- Missing table entries for valid IDs are internal compiler invariants and may panic in tests; production borrowck should construct tables consistently so this path is unreachable.
- The conversion must not hide or downgrade current borrow errors.
- Debug-only tracing through `ROCK_TRACE_BORROWS` should remain useful, but it may print indexed states instead of full maps.

## Testing Strategy

Task 20 needs both representation tests and behavior tests.

Representation tests should cover:

- `BitSet<LoanId>` insert/remove/contains/join/iteration.
- `LocalSet` insert/remove/contains/join/iteration.
- `Location` equality, hashing, and grouping of loans by location.
- `LoanTable` allocation and lookup by `crate::ids::LoanId`.
- `LoanState` join, owner transfer, owner release, active loan activation, and empty-owner cleanup.

Behavior tests should cover existing borrowck behavior:

- Reborrow provenance resolution.
- Shared and mutable borrow conflicts.
- Reference copy alias preservation.
- Owner release at `StorageDead`.
- Branch merges that union owner sets.
- Loop-carried borrow state.
- Closure shared/mutable captures.
- Assert operands as read-only loan accesses.

The main verification commands are:

```bash
cargo test -p rock-lib mir::borrowck -- --nocapture
cargo test -p rock-lib mir::dataflow -- --nocapture
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo fmt --all --check
cargo test -p rock-lib
git diff --check
```

## Completion Criteria

Task 20 is complete when:

- Borrowck uses typed `Location` values instead of raw `(usize, usize)` loan locations.
- Active loan state no longer uses `HashMap<LoanId, Loan>` for dataflow propagation.
- Loan owner sets no longer use `HashSet<Local>` in active flow state.
- `mir::dataflow::analyses::loans` uses `crate::ids::LoanId` instead of a local `LoanId` newtype.
- Reference liveness and active loan propagation preserve before/after statement precision.
- Provenance resolution and conflict checks operate through `LoanTable` plus indexed `LoanState`.
- Current borrowck diagnostics and spans remain stable unless a change is intentional and covered by tests.
- Focused borrowck/dataflow tests, relevant integration filters, full `cargo test -p rock-lib`, formatting, and `git diff --check` pass.

## Risks

- Owner sets are path-sensitive; replacing only active loan membership with a bitset is insufficient. The indexed design must keep per-loan owner sets in flow state.
- Current liveness has statement-level precision while the generic dataflow engine only exposes block entry/exit results. Task 20 should not force this into the existing engine if doing so loses precision.
- Provenance resolution currently picks the first active loan owning a local. Indexed lookup must preserve deterministic behavior or make ambiguity explicit in tests.
- Borrowck tests include `#[should_panic]` MIR-construction invariants and diagnostics-sensitive integration cases; verification output must be read carefully rather than assuming any panic text is a failure.
- A one-pass feature boundary increases risk. The implementation plan should still use small TDD commits and review gates to avoid an unreviewable rewrite.
