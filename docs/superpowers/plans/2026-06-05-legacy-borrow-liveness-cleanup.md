# Legacy Borrow Liveness Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the legacy `BorrowId` / `compute_live_borrows` borrow-liveness island so `LoanId` / `LoanTable` is the only borrow identity model in MIR borrow checking.

**Architecture:** Keep `BorrowData` as an ID-less temporary collection record and keep `LoanTable` as the canonical `LoanId` allocator. Delete the old `HashSet<BorrowId>` liveness helper instead of translating it to `LoanId`, because active borrow checking already uses `LoanState` and statement-precise reference liveness.

**Tech Stack:** Rust 2021, `rock-lib`, MIR borrowck modules, `crate::ids::LoanId`, `LoanTable`, `LoanState`, `ReferenceLiveness`, `LocalSet`, `bd` issue tracking.

**Session Constraint:** Do not commit, amend, push, stage, or otherwise mutate VCS state unless the user explicitly asks. Update docs and bead status as working-tree changes only.

---

## File Structure

- Modify `lib/src/mir/borrowck/borrows.rs`: remove `BorrowId`, remove `BorrowData.id`, and stop assigning synthetic IDs during borrow collection.
- Modify `lib/src/mir/borrowck/liveness.rs`: remove `LiveBorrowSet`, `compute_live_borrows`, and helper functions that only serve the legacy `HashSet<BorrowId>` analysis; keep active reference liveness and active loan propagation.
- Modify `lib/src/mir/dataflow/analyses/loans.rs`: update tests and any `BorrowData` construction to the ID-less temporary record model.
- Modify `lib/src/mir/borrowck/mod.rs`: remove the legacy `BorrowData` diagnostic fallback and remove now-unused `borrows` parameters from loan-checking helpers.
- Modify `docs/superpowers/plans/master-audit-checklist.md`: mark the legacy `BorrowId` / `compute_live_borrows` item complete after verification.
- Modify `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`: update Task 20 summary, verified-status table, and Task 20 details to reflect removal of the legacy helper.

---

### Task 1: Add Red Active-Path Tests

**Files:**
- Modify: `lib/src/mir/dataflow/analyses/loans.rs:329-424`
- Modify: `lib/src/mir/borrowck/liveness.rs:877-1290`

- [ ] **Step 1: Update `LoanTable` borrow-construction test to require ID-less `BorrowData`**

In `lib/src/mir/dataflow/analyses/loans.rs`, replace the `loan_table_records_indexed_place_paths` test with this version:

```rust
    #[test]
    fn loan_table_records_indexed_place_paths() {
        use crate::mir::borrowck::borrows::BorrowData;
        use crate::mir::borrowck::paths::PlacePathTable;

        let place = Place {
            local: Local(1),
            projection: Vec::new(),
        };
        let mut paths = PlacePathTable::new();
        let place_path = paths.intern(place.clone());
        let borrows = vec![BorrowData {
            owner: Local(2),
            place,
            kind: crate::mir::borrowck::accesses::AccessKind::BorrowShared,
            created_at: Location::new(BasicBlockId(0), StatementIndex(0)),
            origin_span: None,
        }];

        let table = LoanTable::from_borrows_with_paths(&borrows, paths);

        assert_eq!(
            table.get(crate::ids::LoanId(0)).unwrap().place_path,
            place_path
        );
    }
```

- [ ] **Step 2: Replace the legacy live-borrow test with an active `LoanState` test**

In `lib/src/mir/borrowck/liveness.rs`, update the test module imports:

```rust
    use super::{
        apply_active_loan_statement_transfer, compute_active_loan_entries,
        compute_reference_liveness, group_loans_by_location, type_id_contains_reference,
    };
    use crate::ids::{AssocTypeId, CrateId, DefId, LoanId, LocalDefId};
    use crate::mir::borrowck::location::{Location, StatementIndex};
    use crate::mir::dataflow::analyses::{LoanState, LoanTable};
    use crate::mir::{
        BasicBlock, BasicBlockId, Local, LocalDecl, MirFunction, MirFunctionId, Mutability,
        Operand, Place, Rvalue, StatementData, StatementKind, Terminator,
    };
```

Also replace:

```rust
    use super::super::borrows::{BorrowData, BorrowId};
```

with:

```rust
    use super::super::borrows::BorrowData;
```

Then replace `test_compute_live_borrows_tracks_owner_storage_dead` with:

```rust
    #[test]
    fn active_loan_entries_release_owner_after_storage_dead() {
        let mut type_context = TypeContext::new();
        let func = test_function(&mut type_context);
        let reference_liveness = compute_reference_liveness(&func, &type_context);
        let borrows = vec![BorrowData {
            owner: Local(2),
            place: Place {
                local: Local(1),
                projection: vec![],
            },
            kind: AccessKind::BorrowShared,
            created_at: Location::new(BasicBlockId(0), StatementIndex(1)),
            origin_span: None,
        }];
        let table = LoanTable::from_borrows(&borrows);
        let grouped = group_loans_by_location(&table);
        let entries = compute_active_loan_entries(
            &func,
            &type_context,
            &table,
            &grouped,
            &reference_liveness,
        );

        let block = &func.basic_blocks[0];
        let mut state = entries[0].clone();
        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            apply_active_loan_statement_transfer(&mut state, &func, &type_context, stmt);

            let location = Location::new(BasicBlockId(0), StatementIndex(stmt_idx));
            if let Some(loan_ids) = grouped.get(&location) {
                for loan_id in loan_ids {
                    state.activate(*loan_id, &table);
                }
            }

            if let Some(live_locals) = reference_liveness
                .after_statement_sets
                .get(0)
                .and_then(|stmt_sets| stmt_sets.get(stmt_idx))
            {
                state.retain_owners(|owner| live_locals.contains(owner));
            }

            if let StatementKind::StorageDead(local) = &stmt.kind {
                state.release_owner(*local);
            }

            if stmt_idx == 1 {
                assert!(state.is_active(LoanId(0)));
                assert!(state.owner_contains(LoanId(0), Local(2)));
            }
            if stmt_idx == 2 {
                assert!(!state.is_active(LoanId(0)));
            }
        }
    }
```

- [ ] **Step 3: Run focused tests and confirm the red state**

Run:

```bash
cargo test -p rock-lib mir::dataflow::analyses::loans::tests::loan_table_records_indexed_place_paths -- --exact
```

Expected: fails to compile because `BorrowData` still requires `id: BorrowId`.

Run:

```bash
cargo test -p rock-lib mir::borrowck::liveness::tests::active_loan_entries_release_owner_after_storage_dead -- --exact
```

Expected: fails to compile because `compute_live_borrows` / `BorrowId` imports still exist and `BorrowData` still requires `id`.

---

### Task 2: Remove `BorrowId` From Borrow Collection

**Files:**
- Modify: `lib/src/mir/borrowck/borrows.rs:1-122`

- [ ] **Step 1: Remove the `BorrowId` type and `BorrowData.id` field**

Change the top of `lib/src/mir/borrowck/borrows.rs` to this structure:

```rust
use crate::lexer::Span;
use crate::mir::borrowck::location::{Location, StatementIndex};
use crate::mir::BasicBlockId;
use crate::mir::{
    Constant, Local, MirCallable, MirFunction, Operand, Place, Rvalue, StatementData,
    StatementKind, Terminator,
};

use super::accesses::AccessKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowData {
    pub owner: Local,
    pub place: Place,
    pub kind: AccessKind,
    pub created_at: Location,
    pub origin_span: Option<Span>,
}
```

- [ ] **Step 2: Remove synthetic ID assignment from statement borrow collection**

In `collect_statement_borrows`, remove every `id` field that initializes a `BorrowId` from constructed `BorrowData` values. The ref assignment case should look like:

```rust
        StatementKind::Assign(dest, Rvalue::Ref(mutability, place)) => vec![BorrowData {
            owner: dest.local,
            place: place.clone(),
            kind: AccessKind::for_borrow(*mutability),
            created_at,
            origin_span: stmt.span.clone(),
        }],
```

The closure capture case should look like:

```rust
                Some(BorrowData {
                    owner: dest.local,
                    place: capture.place(),
                    kind,
                    created_at,
                    origin_span: stmt.span.clone(),
                })
```

- [ ] **Step 3: Remove synthetic ID assignment from terminator borrow collection**

In `collect_terminator_borrows`, the returned `BorrowData` should look like:

```rust
    vec![BorrowData {
        owner: destination.local,
        place: root,
        kind: AccessKind::BorrowShared,
        created_at,
        origin_span: None,
    }]
```

- [ ] **Step 4: Remove post-collection ID rewriting**

In `collect_function_borrows`, delete this loop entirely:

```rust
    for (idx, borrow) in borrows.iter_mut().enumerate() {
        borrow.id = BorrowId(idx);
    }
```

The function should end with:

```rust
    borrows
}
```

- [ ] **Step 5: Run borrow collection tests**

Run:

```bash
cargo test -p rock-lib mir::borrowck::borrows -- --nocapture
```

Expected: borrow collection tests pass or reveal remaining `BorrowId` references in tests.

---

### Task 3: Delete Legacy `compute_live_borrows`

**Files:**
- Modify: `lib/src/mir/borrowck/liveness.rs:1-558`
- Modify: `lib/src/mir/borrowck/liveness.rs:877-1290`

- [ ] **Step 1: Remove unused `HashSet` and `BorrowId` imports**

At the top of `lib/src/mir/borrowck/liveness.rs`, replace:

```rust
use std::collections::{HashMap, HashSet};
```

with:

```rust
use std::collections::HashMap;
```

Remove this import:

```rust
use super::borrows::{BorrowData, BorrowId};
```

There should be no replacement import for `BorrowData` in production code.

- [ ] **Step 2: Remove the legacy public type alias and function**

Delete `pub type LiveBorrowSet = HashSet<BorrowId>;` and delete the full `compute_live_borrows` function body, including its `HashMap<Location, LiveBorrowSet>` return type. The next public item after this deletion should be `pub fn group_loans_by_location(table: &LoanTable) -> LoansByLocation`.

- [ ] **Step 3: Remove private helpers used only by the legacy helper**

Delete these private functions entirely: `borrow_locals`, `transfer_borrow_block`, `apply_statement_borrow_liveness`, `apply_terminator_borrow_liveness`, `kill_borrows_owned_by_local`, `replace_borrow_alias`, `rvalue_borrow_uses`, `operand_borrow_uses`, and `place_borrow_uses`.

Keep these active-path functions: `compute_reference_liveness`, `group_loans_by_location`, `compute_active_loan_entries`, `apply_active_loan_statement_transfer`, `apply_active_loan_terminator_transfer`, `compute_successors`, `reference_locals`, and `compute_reference_liveness_for`.

- [ ] **Step 4: Update liveness test imports and remove legacy test references**

Use the imports and replacement test from Task 1. After this step, `liveness.rs` should have no `BorrowId`, `LiveBorrowSet`, or `compute_live_borrows` references.

- [ ] **Step 5: Run liveness focused tests**

Run:

```bash
cargo test -p rock-lib mir::borrowck::liveness -- --nocapture
```

Expected: liveness tests pass or report remaining compile errors from removed helper names.

---

### Task 4: Remove Diagnostic Fallback And Update `BorrowData` Test Fixtures

**Files:**
- Modify: `lib/src/mir/borrowck/mod.rs:100-178`
- Modify: `lib/src/mir/borrowck/mod.rs:280-546`
- Modify: `lib/src/mir/dataflow/analyses/loans.rs:398-424`

- [ ] **Step 1: Stop passing `borrows` into loan-checking helpers**

In `BorrowChecker::check_function`, change the `check_statement_loans` call from:

```rust
                Self::check_statement_loans(
                    stmt,
                    type_context,
                    &loan_table,
                    &live_loans,
                    func,
                    &borrows,
                    &mut diagnostics,
                );
```

to:

```rust
                Self::check_statement_loans(
                    stmt,
                    type_context,
                    &loan_table,
                    &live_loans,
                    func,
                    &mut diagnostics,
                );
```

Change the `check_terminator` call from:

```rust
                Self::check_terminator(
                    term,
                    &init_analysis,
                    &state,
                    &loan_table,
                    &active_loans,
                    func,
                    &borrows,
                    &mut diagnostics,
                );
```

to:

```rust
                Self::check_terminator(
                    term,
                    &init_analysis,
                    &state,
                    &loan_table,
                    &active_loans,
                    func,
                    &mut diagnostics,
                );
```

- [ ] **Step 2: Remove `borrows` parameters from helper signatures**

Change `check_statement_loans` signature to:

```rust
    fn check_statement_loans(
        stmt: &StatementData,
        type_context: &crate::type_context::TypeContext,
        table: &LoanTable,
        active_loans: &LoanState,
        func: &MirFunction,
        diagnostics: &mut Diagnostics,
    ) {
```

Change `check_terminator` signature to:

```rust
    fn check_terminator(
        term: &Terminator,
        init_analysis: &InitializationAnalysis,
        state: &InitMap,
        table: &LoanTable,
        active_loans: &LoanState,
        func: &MirFunction,
        diagnostics: &mut Diagnostics,
    ) {
```

Change `check_place_loan_access` signature to:

```rust
    fn check_place_loan_access(
        place: &crate::mir::Place,
        kind: LoanKind,
        table: &LoanTable,
        active_loans: &LoanState,
        func: &MirFunction,
        diagnostics: &mut Diagnostics,
        stmt_span: Option<Span>,
    ) {
```

Update every `Self::check_place_loan_access` call in `check_statement_loans` and `check_terminator` by removing the `borrows` argument.

- [ ] **Step 3: Remove the legacy borrow-span fallback**

Inside `check_place_loan_access`, replace:

```rust
            let borrow_span = table
                .get(conflicting_loan_id)
                .and_then(|loan| loan.origin_span.clone())
                .or_else(|| {
                    borrows
                        .get(conflicting_loan_id.index())
                        .and_then(|borrow| borrow.origin_span.clone())
                })
                .unwrap_or_default();
```

with:

```rust
            let borrow_span = table
                .get(conflicting_loan_id)
                .and_then(|loan| loan.origin_span.clone())
                .unwrap_or_default();
```

- [ ] **Step 4: Update remaining `BorrowData` fixtures**

Run:

```bash
rg "BorrowData \{" lib/src/mir --glob '*.rs'
```

For every result, remove the `id` field that initializes a `BorrowId`. Keep owner, place, kind, created_at, and origin_span.

- [ ] **Step 5: Run focused active-path tests**

Run:

```bash
cargo test -p rock-lib mir::dataflow::analyses::loans::tests::loan_table_records_indexed_place_paths -- --exact
```

Expected: passes.

Run:

```bash
cargo test -p rock-lib mir::borrowck::liveness::tests::active_loan_entries_release_owner_after_storage_dead -- --exact
```

Expected: passes.

Run:

```bash
rg "BorrowId|compute_live_borrows|LiveBorrowSet" lib/src/mir --glob '*.rs'
```

Expected: no matches.

---

### Task 5: Documentation, Bead, And Verification

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md:39-41`
- Modify: `docs/superpowers/plans/master-audit-checklist.md:411-435`
- Modify: `docs/superpowers/plans/master-audit-checklist.md:447-449`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md:147-180`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md:505-521`

- [ ] **Step 1: Update master checklist evidence and remaining work**

In `master-audit-checklist.md`, update the Borrowck Indexed Dataflow row summary to remove the legacy helper from remaining work and mention the completed cleanup. Keep generic active-loan dataflow and any true remaining work as future items.

In section `## 11. Borrowck Indexed Dataflow`, replace the legacy-helper evidence line with:

```markdown
- The legacy `compute_live_borrows` / `BorrowId` helper has been removed; active borrow checking now uses `LoanId` / `LoanTable` / `LoanState` as the sole borrow identity path.
```

Move the still-to-do legacy-helper checkbox into `Done` as:

```markdown
- [x] Deleted the legacy `compute_live_borrows` / `BorrowId` helper so `LoanId` is the only borrow identity model in MIR borrow checking.
```

Leave these remaining items unchecked:

```markdown
- [ ] Consider moving the statement-precise active-loan fixpoint into the generic MIR dataflow framework if it can preserve current before/after-statement precision.
```

Do not re-add `MovePathId` / `PlacePathId` as remaining Task 20 work if the current checklist already reflects the indexed place-path model as completed elsewhere.

- [ ] **Step 2: Update ordered roadmap Task 20 status**

In `2026-05-17-compiler-architecture-ordered-roadmap.md`, update the top status note and Task 20 table row so they state that `BorrowId` and `compute_live_borrows` are removed. The remaining-work cell for Task 20 should retain only active-loan generic dataflow integration.

In the Task 20 detail section, update the `Status` and `Work` paragraphs to say the strict cleanup removed the legacy helper after the scoped indexed dataflow slice.

Add a verification note after the existing 2026-05-25 note:

```markdown
**2026-06-05 verification note:** Removed the legacy `compute_live_borrows` / `BorrowId` helper and made `LoanTable` the only loan ID allocation boundary. Focused `mir::borrowck` and `mir::dataflow` verification passed, the borrow filter passed, the `BorrowId|compute_live_borrows|LiveBorrowSet` audit returned no `lib/src/mir` hits, and formatting passed.
```

- [ ] **Step 3: Run focused verification serially**

Run:

```bash
cargo test -p rock-lib mir::borrowck -- --nocapture
```

Expected: all matching borrowck tests pass.

Run:

```bash
cargo test -p rock-lib mir::dataflow -- --nocapture
```

Expected: all matching dataflow tests pass.

Run:

```bash
cargo test -p rock-lib borrow -- --nocapture
```

Expected: unit and integration tests matching `borrow` pass.

- [ ] **Step 4: Run formatting and audit checks**

Run:

```bash
cargo fmt --all --check
```

Expected: exits successfully.

Run:

```bash
rg "BorrowId|compute_live_borrows|LiveBorrowSet" lib/src/mir --glob '*.rs'
```

Expected: no matches.

Run:

```bash
git diff --check
```

Expected: exits successfully.

- [ ] **Step 5: Run broad verification before closing the bead**

Run:

```bash
cargo test -p rock-lib > /tmp/rock-lib-eha-tests.log 2>&1
```

Expected: exits successfully. Inspect the captured output if it fails; do not immediately rerun the full suite.

- [ ] **Step 6: Close the bead after verification succeeds**

Run:

```bash
bd close new_lang2-eha --reason "Completed legacy BorrowId live-borrow cleanup" --json
```

Expected: bead `new_lang2-eha` is closed. Do not commit or push unless explicitly requested.

---

## Self-Review Notes

- Spec coverage: the plan removes `BorrowId`, `LiveBorrowSet`, `compute_live_borrows`, `BorrowData.id`, the diagnostic fallback, and updates both required docs after verification.
- Placeholder scan: the plan avoids placeholder steps and names exact commands, files, and code blocks for each code change.
- Type consistency: the plan consistently uses `BorrowData` as an ID-less collection fact and `LoanId` / `LoanTable` / `LoanState` as the canonical active borrow model.
