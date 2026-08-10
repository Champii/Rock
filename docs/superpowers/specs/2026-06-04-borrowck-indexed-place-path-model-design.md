# Borrowck Indexed Place Path Model Design

## Goal

Make `PlacePathId` and `MovePathId` the long-term borrowck path model instead of leaving them as unused typed IDs. Borrowck should continue to preserve existing diagnostics and behavior while moving loans, conflicts, provenance, and initialization/move checks toward indexed MIR place paths.

## Current State

- `MovePathId` and `PlacePathId` exist in `lib/src/ids.rs` but are not used outside the ID definitions.
- Active borrowck uses `LoanId`, `LoanTable`, `LoanState`, `LocalSet`, and typed `Location`.
- Loan facts and conflict checks still carry and compare structural `Place { local, projection }` values directly.
- Initialization and move checking currently use `InitMap(HashMap<Local, InitState>)`, so moves are tracked at local-root granularity.

## Decision

Use indexed path tables as the long-term model:

- `PlacePathTable` interns each MIR `Place` into a stable `PlacePathId` and records parent/child path relationships.
- `MovePathTable` maps tracked move paths to `PlacePathId`s, initially one-to-one for places observed in a MIR function.
- `LoanData` stores an indexed `PlacePathId` for semantic checks and keeps the original `Place` for diagnostics.
- Conflict and provenance checks resolve through indexed path tables rather than comparing ad hoc places at each call site.
- `InitializationAnalysis` migrates from `Local` state to `MovePathId` state so partial-move semantics can be represented without another model change.

## Data Flow

1. Build path tables from each `MirFunction` by scanning locals, statement destinations, operands, borrow places, closure captures, assertions, and terminator operands/destinations.
2. Build `LoanTable` from collected borrow facts and the function path tables.
3. Compute active loans with `LoanState` as today, but loan payloads point at indexed place paths.
4. Resolve reborrow provenance through active loan owners and `PlacePathTable` lookups.
5. Run initialization/move dataflow over `MovePathId` states.

## Error Handling And Diagnostics

Diagnostics should continue to use original `Place` and local span data. Indexed paths are semantic storage, not user-facing formatting.

## Testing

- Unit-test path interning, parent relationships, conflict behavior, and move-path state transitions.
- Keep existing borrowck diagnostics stable unless a test intentionally documents a corrected partial-move behavior.
- Verify focused suites first, then `cargo test -p rock-lib`, `cargo fmt --all --check`, and `git diff --check` before closing the bead.
