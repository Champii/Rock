# Borrowck Indexed Dataflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert MIR borrow checking from map-heavy loan state to typed locations, indexed loan metadata tables, and dense dataflow state while preserving current borrowck behavior.

**Architecture:** Add dense set and location primitives first, then introduce indexed loan tables and owner state. Convert borrow collection, liveness, provenance, conflict checking, and `BorrowChecker` in a coordinated sequence so the final active-loan path no longer uses `HashMap<LoanId, Loan>` or `HashSet<Local>` owner sets.

**Tech Stack:** Rust 2021, `rock-lib`, MIR borrowck/dataflow modules, existing `crate::ids::{Idx, IdGen, LoanId}`, existing borrowck unit and integration tests.

---

## Approved Spec

- `docs/superpowers/specs/2026-05-23-borrowck-indexed-dataflow-design.md`

## File Structure

- Create `lib/src/mir/dataflow/bitset.rs`: dense `BitSet<I>` for `Idx` IDs and `LocalSet` for MIR locals.
- Create `lib/src/mir/borrowck/location.rs`: typed `StatementIndex` and `Location` for statement-level MIR locations.
- Modify `lib/src/mir/dataflow/mod.rs`: register and re-export bitset helpers.
- Modify `lib/src/mir/borrowck/mod.rs`: register `location`, wire indexed loan tables/states into checking.
- Modify `lib/src/mir/borrowck/borrows.rs`: store typed creation locations in `BorrowData`.
- Modify `lib/src/mir/dataflow/analyses/loans.rs`: replace local `LoanId`, add `LoanData`, `LoanTable`, and indexed `LoanState`.
- Modify `lib/src/mir/borrowck/liveness.rs`: convert reference liveness and active loan propagation to indexed sets/states.
- Modify `lib/src/mir/borrowck/provenance.rs`: resolve deref provenance through indexed loan tables/states.
- Modify nearby borrowck tests and `lib/tests/integration.rs` only if a diagnostic intentionally changes.

## Task 1: Add Dense Indexed Sets And Typed Locations

**Files:**
- Create: `lib/src/mir/dataflow/bitset.rs`
- Create: `lib/src/mir/borrowck/location.rs`
- Modify: `lib/src/mir/dataflow/mod.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/dataflow/bitset.rs`
- Test: `lib/src/mir/borrowck/location.rs`

- [ ] **Step 1: Write failing dense set tests**

Create `lib/src/mir/dataflow/bitset.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ids::LoanId;
    use crate::mir::Local;

    #[test]
    fn bitset_tracks_typed_ids_and_joins() {
        let mut left = BitSet::<LoanId>::new();
        let mut right = BitSet::<LoanId>::new();

        assert!(left.insert(LoanId(1)));
        assert!(!left.insert(LoanId(1)));
        assert!(right.insert(LoanId(2)));

        assert!(left.contains(LoanId(1)));
        assert!(!left.contains(LoanId(2)));
        assert!(left.join(&right));
        assert!(left.contains(LoanId(2)));
        assert_eq!(left.iter().collect::<Vec<_>>(), vec![LoanId(1), LoanId(2)]);
    }

    #[test]
    fn local_set_tracks_mir_locals_and_removes() {
        let mut set = LocalSet::new();

        assert!(set.insert(Local(3)));
        assert!(set.contains(Local(3)));
        assert!(set.remove(Local(3)));
        assert!(!set.contains(Local(3)));
        assert!(set.is_empty());
    }
}
```

- [ ] **Step 2: Write failing location tests**

Create `lib/src/mir/borrowck/location.rs` with the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::mir::BasicBlockId;

    #[test]
    fn location_is_typed_and_hashable() {
        let loc = Location::new(BasicBlockId(2), StatementIndex(5));
        let mut map = std::collections::HashMap::new();

        map.insert(loc, "loan");

        assert_eq!(map.get(&loc), Some(&"loan"));
        assert_eq!(loc.block, BasicBlockId(2));
        assert_eq!(loc.statement, StatementIndex(5));
    }
}
```

- [ ] **Step 3: Register modules and verify tests fail**

In `lib/src/mir/dataflow/mod.rs`, add:

```rust
pub mod bitset;
```

In `lib/src/mir/dataflow/mod.rs`, add this re-export:

```rust
pub use bitset::{BitSet, LocalSet};
```

In `lib/src/mir/borrowck/mod.rs`, add:

```rust
pub mod location;
```

Run: `cargo test -p rock-lib mir::dataflow::bitset -- --nocapture && cargo test -p rock-lib mir::borrowck::location -- --nocapture`

Expected: FAIL because `BitSet`, `LocalSet`, `StatementIndex`, and `Location` are missing.

- [ ] **Step 4: Implement dense set primitives**

Insert this implementation above the tests in `lib/src/mir/dataflow/bitset.rs`:

```rust
use std::marker::PhantomData;

use crate::ids::Idx;
use crate::mir::Local;

use super::Lattice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitSet<I> {
    bits: Vec<bool>,
    _marker: PhantomData<fn() -> I>,
}

impl<I> Default for BitSet<I> {
    fn default() -> Self {
        Self {
            bits: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<I: Idx> BitSet<I> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bits: vec![false; capacity],
            _marker: PhantomData,
        }
    }

    pub fn insert(&mut self, id: I) -> bool {
        let index = id.index();
        if self.bits.len() <= index {
            self.bits.resize(index + 1, false);
        }
        let changed = !self.bits[index];
        self.bits[index] = true;
        changed
    }

    pub fn remove(&mut self, id: I) -> bool {
        let index = id.index();
        if index >= self.bits.len() || !self.bits[index] {
            return false;
        }
        self.bits[index] = false;
        true
    }

    pub fn contains(&self, id: I) -> bool {
        self.bits.get(id.index()).copied().unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        !self.bits.iter().any(|bit| *bit)
    }

    pub fn iter(&self) -> BitSetIter<'_, I> {
        BitSetIter { set: self, next: 0 }
    }
}

impl<I: Idx> Lattice for BitSet<I> {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        if self.bits.len() < other.bits.len() {
            self.bits.resize(other.bits.len(), false);
        }
        for (index, bit) in other.bits.iter().enumerate() {
            if *bit && !self.bits[index] {
                self.bits[index] = true;
                changed = true;
            }
        }
        changed
    }
}

pub struct BitSetIter<'a, I> {
    set: &'a BitSet<I>,
    next: usize,
}

impl<I: Idx> Iterator for BitSetIter<'_, I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.set.bits.len() {
            let index = self.next;
            self.next += 1;
            if self.set.bits[index] {
                return Some(I::from_raw(index as u32));
            }
        }
        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalSet {
    bits: Vec<bool>,
}

impl LocalSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bits: vec![false; capacity],
        }
    }

    pub fn insert(&mut self, local: Local) -> bool {
        let index = local.0;
        if self.bits.len() <= index {
            self.bits.resize(index + 1, false);
        }
        let changed = !self.bits[index];
        self.bits[index] = true;
        changed
    }

    pub fn remove(&mut self, local: Local) -> bool {
        let index = local.0;
        if index >= self.bits.len() || !self.bits[index] {
            return false;
        }
        self.bits[index] = false;
        true
    }

    pub fn contains(&self, local: Local) -> bool {
        self.bits.get(local.0).copied().unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        !self.bits.iter().any(|bit| *bit)
    }

    pub fn iter(&self) -> LocalSetIter<'_> {
        LocalSetIter { set: self, next: 0 }
    }
}

impl Lattice for LocalSet {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        if self.bits.len() < other.bits.len() {
            self.bits.resize(other.bits.len(), false);
        }
        for (index, bit) in other.bits.iter().enumerate() {
            if *bit && !self.bits[index] {
                self.bits[index] = true;
                changed = true;
            }
        }
        changed
    }
}

pub struct LocalSetIter<'a> {
    set: &'a LocalSet,
    next: usize,
}

impl Iterator for LocalSetIter<'_> {
    type Item = Local;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.set.bits.len() {
            let index = self.next;
            self.next += 1;
            if self.set.bits[index] {
                return Some(Local(index));
            }
        }
        None
    }
}
```

- [ ] **Step 5: Implement typed locations**

Insert this implementation above the tests in `lib/src/mir/borrowck/location.rs`:

```rust
use crate::mir::BasicBlockId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatementIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Location {
    pub block: BasicBlockId,
    pub statement: StatementIndex,
}

impl Location {
    pub fn new(block: BasicBlockId, statement: StatementIndex) -> Self {
        Self { block, statement }
    }
}
```

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib mir::dataflow::bitset -- --nocapture && cargo test -p rock-lib mir::borrowck::location -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::dataflow::bitset -- --nocapture && cargo test -p rock-lib mir::borrowck::location -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/dataflow/bitset.rs lib/src/mir/dataflow/mod.rs lib/src/mir/borrowck/location.rs lib/src/mir/borrowck/mod.rs
git commit -m "add indexed borrowck dataflow primitives"
```

## Task 2: Add Indexed Loan Tables And State

**Files:**
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Modify: `lib/src/mir/dataflow/analyses/mod.rs`
- Test: `lib/src/mir/dataflow/analyses/loans.rs`

- [ ] **Step 1: Write failing loan table/state tests**

Add these tests to `lib/src/mir/dataflow/analyses/loans.rs`:

```rust
#[cfg(test)]
mod indexed_tests {
    use super::*;

    use crate::ids::LoanId;
    use crate::mir::borrowck::location::{Location, StatementIndex};
    use crate::mir::{BasicBlockId, Local, Place};

    fn loan_data(id: LoanId, owner: Local) -> LoanData {
        LoanData {
            id,
            place: Place {
                local: Local(1),
                projection: vec![],
            },
            initial_owner: owner,
            kind: LoanKind::Shared,
            origin_span: None,
            created_at: Location::new(BasicBlockId(0), StatementIndex(0)),
        }
    }

    #[test]
    fn loan_table_allocates_crate_loan_ids() {
        let mut table = LoanTable::new();

        let id = table.push(loan_data(LoanId(999), Local(2)));

        assert_eq!(id, LoanId(0));
        assert_eq!(table.get(id).expect("loan data").id, id);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn loan_state_activates_transfers_releases_and_joins_owners() {
        let mut table = LoanTable::new();
        let first = table.push(loan_data(LoanId(99), Local(2)));
        let second = table.push(loan_data(LoanId(99), Local(4)));
        let mut left = LoanState::new(table.len());
        let mut right = LoanState::new(table.len());

        left.activate(first, &table);
        right.activate(first, &table);
        right.activate(second, &table);
        right.transfer_owner(Local(2), Local(3));

        assert!(left.join(&right));
        assert!(left.is_active(first));
        assert!(left.is_active(second));
        assert!(left.owners(first).contains(Local(2)));
        assert!(left.owners(first).contains(Local(3)));
        assert!(left.release_owner(Local(2)));
        assert!(!left.owners(first).contains(Local(2)));
    }
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib loan_table_allocates_crate_loan_ids -- --nocapture && cargo test -p rock-lib loan_state_activates_transfers_releases_and_joins_owners -- --nocapture`

Expected: FAIL because `LoanData`, `LoanTable`, and indexed `LoanState` are missing.

- [ ] **Step 3: Replace local LoanId and add indexed loan data**

In `lib/src/mir/dataflow/analyses/loans.rs`, remove the local `LoanId(pub usize)` definition and import the canonical ID:

```rust
use crate::ids::{Idx, LoanId};
use crate::mir::borrowck::location::Location;
use crate::mir::dataflow::{BitSet, LocalSet, Lattice};
```

Add these types above `LoanKind`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanData {
    pub id: LoanId,
    pub place: Place,
    pub initial_owner: Local,
    pub kind: LoanKind,
    pub origin_span: Option<crate::lexer::Span>,
    pub created_at: Location,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoanTable {
    loans: Vec<LoanData>,
}

impl LoanTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut loan: LoanData) -> LoanId {
        let id = LoanId(self.loans.len() as u32);
        loan.id = id;
        self.loans.push(loan);
        id
    }

    pub fn get(&self, id: LoanId) -> Option<&LoanData> {
        self.loans.get(id.index())
    }

    pub fn iter(&self) -> impl Iterator<Item = &LoanData> {
        self.loans.iter()
    }

    pub fn len(&self) -> usize {
        self.loans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loans.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanState {
    active: BitSet<LoanId>,
    owners: Vec<LocalSet>,
}

impl LoanState {
    pub fn new(loan_count: usize) -> Self {
        Self {
            active: BitSet::with_capacity(loan_count),
            owners: (0..loan_count).map(|_| LocalSet::new()).collect(),
        }
    }

    pub fn is_active(&self, id: LoanId) -> bool {
        self.active.contains(id)
            && self
                .owners
                .get(id.index())
                .is_some_and(|owners| !owners.is_empty())
    }

    pub fn activate(&mut self, id: LoanId, table: &LoanTable) -> bool {
        let Some(loan) = table.get(id) else {
            return false;
        };
        self.ensure_len(id.index() + 1);
        let active_changed = self.active.insert(id);
        let owner_changed = self.owners[id.index()].insert(loan.initial_owner);
        active_changed || owner_changed
    }

    pub fn owners(&self, id: LoanId) -> &LocalSet {
        &self.owners[id.index()]
    }

    pub fn active_ids(&self) -> impl Iterator<Item = LoanId> + '_ {
        self.active.iter().filter(|id| self.is_active(*id))
    }

    pub fn owner_contains(&self, id: LoanId, local: Local) -> bool {
        self.owners
            .get(id.index())
            .is_some_and(|owners| owners.contains(local))
    }

    pub fn transfer_owner(&mut self, from: Local, to: Local) -> bool {
        let mut changed = false;
        for owners in &mut self.owners {
            if owners.remove(from) {
                changed = true;
                changed |= owners.insert(to);
            }
        }
        changed
    }

    pub fn copy_owner(&mut self, from: Local, to: Local) -> bool {
        let mut changed = false;
        for owners in &mut self.owners {
            if owners.contains(from) {
                changed |= owners.insert(to);
            }
        }
        changed
    }

    pub fn release_owner(&mut self, local: Local) -> bool {
        let mut changed = false;
        for (index, owners) in self.owners.iter_mut().enumerate() {
            if owners.remove(local) {
                changed = true;
            }
            if owners.is_empty() {
                changed |= self.active.remove(LoanId(index as u32));
            }
        }
        changed
    }

    pub fn retain_owners<F>(&mut self, mut keep: F) -> bool
    where
        F: FnMut(Local) -> bool,
    {
        let mut changed = false;
        for (index, owners) in self.owners.iter_mut().enumerate() {
            let current: Vec<_> = owners.iter().collect();
            for owner in current {
                if !keep(owner) {
                    changed |= owners.remove(owner);
                }
            }
            if owners.is_empty() {
                changed |= self.active.remove(LoanId(index as u32));
            }
        }
        changed
    }

    pub fn join(&mut self, other: &Self) -> bool {
        let mut changed = self.active.join(&other.active);
        if self.owners.len() < other.owners.len() {
            self.owners.resize_with(other.owners.len(), LocalSet::new);
            changed = true;
        }
        for (index, owners) in other.owners.iter().enumerate() {
            changed |= self.owners[index].join(owners);
        }
        changed
    }

    fn ensure_len(&mut self, len: usize) {
        if self.owners.len() < len {
            self.owners.resize_with(len, LocalSet::new);
        }
    }
}
```

- [ ] **Step 4: Keep temporary map alias compiling**

During this task only, rename the old map alias so existing callers keep compiling until Task 7 wires the indexed state:

```rust
pub type LegacyLoanState = HashMap<LoanId, Loan>;
```

Update the old `LoanAnalysis::check_aliasing` signature to use `&LegacyLoanState` temporarily. Task 7 removes this alias and converts the checker to `LoanState`.

- [ ] **Step 5: Update re-exports**

In `lib/src/mir/dataflow/analyses/mod.rs`, re-export the new types:

```rust
pub use loans::{LegacyLoanState, Loan, LoanAnalysis, LoanData, LoanKind, LoanState, LoanTable};
```

Do not re-export a local `LoanId`; callers should import `crate::ids::LoanId` after this task.

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib loan_table_allocates_crate_loan_ids -- --nocapture && cargo test -p rock-lib loan_state_activates_transfers_releases_and_joins_owners -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/dataflow/analyses/loans.rs lib/src/mir/dataflow/analyses/mod.rs
git commit -m "add indexed mir loan tables"
```

## Task 3: Convert Borrow Collection To Typed Locations And Loan Tables

**Files:**
- Modify: `lib/src/mir/borrowck/borrows.rs`
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Test: `lib/src/mir/borrowck/borrows.rs`
- Test: `lib/src/mir/dataflow/analyses/loans.rs`

- [ ] **Step 1: Write failing borrow location tests**

Update the first borrow collection test in `lib/src/mir/borrowck/borrows.rs` to assert typed locations:

```rust
use crate::mir::borrowck::location::{Location, StatementIndex};
use crate::mir::BasicBlockId;

assert_eq!(borrows[0].created_at, Location::new(BasicBlockId(0), StatementIndex(0)));
```

Add this loan table conversion test to `lib/src/mir/dataflow/analyses/loans.rs`:

```rust
#[test]
fn loan_table_builds_from_collected_borrows_by_location() {
    let stmt = crate::mir::StatementData::assign(
        crate::mir::Place {
            local: crate::mir::Local(2),
            projection: vec![],
        },
        crate::mir::Rvalue::Ref(
            crate::mir::Mutability::Not,
            crate::mir::Place {
                local: crate::mir::Local(1),
                projection: vec![],
            },
        ),
        None,
    );
    let borrow = crate::mir::borrowck::borrows::collect_statement_borrows(&stmt, 0, 0)
        .pop()
        .expect("borrow");

    let borrows = vec![borrow];
    let table = LoanTable::from_borrows(&borrows);

    let loan = table.get(crate::ids::LoanId(0)).expect("loan data");
    assert_eq!(loan.initial_owner, crate::mir::Local(2));
    assert_eq!(loan.place.local, crate::mir::Local(1));
    assert_eq!(loan.created_at.block, crate::mir::BasicBlockId(0));
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib test_collect_borrow_from_ref_assignment -- --nocapture && cargo test -p rock-lib loan_table_builds_from_collected_borrows_by_location -- --nocapture`

Expected: FAIL because `BorrowData::created_at` and `LoanTable::from_borrows` are missing.

- [ ] **Step 3: Update BorrowData**

In `lib/src/mir/borrowck/borrows.rs`, import typed locations:

```rust
use crate::mir::borrowck::location::{Location, StatementIndex};
use crate::mir::BasicBlockId;
```

Replace `BorrowData` location fields:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowData {
    pub id: BorrowId,
    pub owner: Local,
    pub place: Place,
    pub kind: AccessKind,
    pub created_at: Location,
    pub origin_span: Option<Span>,
}
```

In `collect_statement_borrows`, create the location once:

```rust
let created_at = Location::new(BasicBlockId(block), StatementIndex(statement));
```

Use `created_at` for both reference and closure-capture borrow records. Remove the old `block` and `statement` field assignments.

- [ ] **Step 4: Add loan table conversion**

In `impl LoanTable` in `lib/src/mir/dataflow/analyses/loans.rs`, add:

```rust
pub fn from_borrows(borrows: &[crate::mir::borrowck::borrows::BorrowData]) -> Self {
    let mut table = Self::new();
    for borrow in borrows {
        let kind = match borrow.kind {
            crate::mir::borrowck::accesses::AccessKind::BorrowShared => LoanKind::Shared,
            crate::mir::borrowck::accesses::AccessKind::BorrowMut => LoanKind::Mut,
            _ => continue,
        };
        table.push(LoanData {
            id: crate::ids::LoanId(0),
            place: borrow.place.clone(),
            initial_owner: borrow.owner,
            kind,
            origin_span: borrow.origin_span.clone(),
            created_at: borrow.created_at,
        });
    }
    table
}
```

Replace `LoanAnalysis::collect_loans` with this required table-returning signature:

```rust
pub fn collect_loans(func: &MirFunction) -> LoanTable {
    let borrows = collect_function_borrows(func);
    LoanTable::from_borrows(&borrows)
}
```

Do not keep the old tuple-returning helper. Current active borrowck callers may continue using their existing manual map construction until Task 7, but new code should use `LoanTable::from_borrows` or `LoanAnalysis::collect_loans`.

- [ ] **Step 5: Update direct field uses**

Replace uses of `borrow.block` and `borrow.statement` in borrowck code with:

```rust
borrow.created_at.block.0
borrow.created_at.statement.0
```

This is a temporary bridge until Task 6 groups loans by `Location` directly.

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib test_collect_borrow_from_ref_assignment -- --nocapture && cargo test -p rock-lib loan_table_builds_from_collected_borrows_by_location -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::borrowck::borrows -- --nocapture && cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/borrowck/borrows.rs lib/src/mir/dataflow/analyses/loans.rs lib/src/mir/borrowck/mod.rs
git commit -m "collect mir borrows with typed locations"
```

## Task 4: Convert Provenance And Aliasing To Indexed State

**Files:**
- Modify: `lib/src/mir/borrowck/provenance.rs`
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Test: `lib/src/mir/borrowck/provenance.rs`
- Test: `lib/src/mir/dataflow/analyses/loans.rs`

- [ ] **Step 1: Write failing indexed provenance tests**

Replace the `resolve_place` test setup in `lib/src/mir/borrowck/provenance.rs` with indexed table/state construction:

```rust
use crate::ids::LoanId;
use crate::mir::borrowck::location::{Location, StatementIndex};
use crate::mir::dataflow::analyses::{LoanData, LoanKind, LoanState, LoanTable};
use crate::mir::BasicBlockId;

let mut table = LoanTable::new();
let id = table.push(LoanData {
    id: LoanId(99),
    place: Place {
        local: Local(1),
        projection: vec![],
    },
    initial_owner: Local(2),
    kind: LoanKind::Shared,
    origin_span: None,
    created_at: Location::new(BasicBlockId(0), StatementIndex(0)),
});
let mut active = LoanState::new(table.len());
active.activate(id, &table);

let place = Place {
    local: Local(2),
    projection: vec![Projection::Deref],
};

assert_eq!(resolve_place(&place, &table, &active), Place { local: Local(1), projection: vec![] });
```

Add an aliasing test in `loans.rs` that calls the indexed `LoanAnalysis::check_aliasing` with `LoanTable` and `LoanState` instead of a map.

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib test_resolve_place_reborrow_uses_owner_loan -- --nocapture && cargo test -p rock-lib check_aliasing -- --nocapture`

Expected: FAIL because `resolve_place` and `check_aliasing` still accept legacy map state.

- [ ] **Step 3: Update provenance API**

In `lib/src/mir/borrowck/provenance.rs`, replace the old import and signature with:

```rust
use crate::mir::dataflow::analyses::{LoanState, LoanTable};

pub fn resolve_place(place: &Place, table: &LoanTable, active_loans: &LoanState) -> Place {
    let mut resolved = place.clone();
    let mut seen = std::collections::HashSet::new();

    loop {
        let Some(Projection::Deref) = resolved.projection.first() else {
            break;
        };

        if !seen.insert(resolved.local) {
            break;
        }

        let Some(loan_id) = active_loans
            .active_ids()
            .find(|loan_id| active_loans.owner_contains(*loan_id, resolved.local))
        else {
            break;
        };

        let Some(loan) = table.get(loan_id) else {
            break;
        };

        let mut next = loan.place.clone();
        next.projection
            .extend(resolved.projection.iter().skip(1).cloned());
        resolved = next;
    }

    resolved
}
```

- [ ] **Step 4: Update aliasing API**

In `LoanAnalysis::check_aliasing`, replace the signature with:

```rust
pub fn check_aliasing(
    place: &Place,
    kind: LoanKind,
    table: &LoanTable,
    active_loans: &LoanState,
) -> Result<(), LoanId>
```

Inside the function:

```rust
let original_place = place.clone();
let place = crate::mir::borrowck::provenance::resolve_place(place, table, active_loans);

for loan_id in active_loans.active_ids() {
    let Some(loan) = table.get(loan_id) else {
        continue;
    };
    if Self::same_mutable_deref_access(&original_place, loan_id, loan, kind, active_loans) {
        continue;
    }
    let loan_place = crate::mir::borrowck::provenance::resolve_place(
        &loan.place,
        table,
        active_loans,
    );
    if crate::mir::borrowck::conflicts::places_conflict(&place, &loan_place) {
        match (kind, loan.kind) {
            (LoanKind::Mut, _) | (_, LoanKind::Mut) => return Err(loan_id),
            (LoanKind::Shared, LoanKind::Shared) => {}
        }
    }
}
Ok(())
```

Replace `same_mutable_deref_access` with:

```rust
fn same_mutable_deref_access(
    place: &Place,
    loan_id: LoanId,
    loan: &LoanData,
    kind: LoanKind,
    active_loans: &LoanState,
) -> bool {
    kind == LoanKind::Mut
        && loan.kind == LoanKind::Mut
        && active_loans.owner_contains(loan_id, place.local)
        && matches!(place.projection.first(), Some(Projection::Deref))
}
```

- [ ] **Step 5: Bridge current BorrowChecker callers**

If `BorrowChecker` still uses legacy state at this point, keep a compile bridge in Task 4 by converting legacy map state into a temporary `LoanTable`/`LoanState` only at the call site. This bridge must be removed in Task 7.

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib mir::borrowck::provenance -- --nocapture && cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::borrowck::provenance -- --nocapture && cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/borrowck/provenance.rs lib/src/mir/dataflow/analyses/loans.rs lib/src/mir/borrowck/mod.rs
git commit -m "check mir loans through indexed state"
```

## Task 5: Convert Reference Liveness To LocalSet

**Files:**
- Modify: `lib/src/mir/borrowck/liveness.rs`
- Test: `lib/src/mir/borrowck/liveness.rs`

- [ ] **Step 1: Write failing LocalSet liveness test**

Update `test_reference_liveness_keeps_pointer_owner_live_for_projected_write` in `liveness.rs` to assert `LocalSet` behavior directly:

```rust
assert!(liveness.before_statement_sets[0][1].contains(Local(1)));
assert!(liveness.after_statement_sets[0][1].contains(Local(1)));
```

Add this test if no existing test asserts join behavior at branch merges:

```rust
#[test]
fn reference_liveness_uses_local_set_at_statement_boundaries() {
    let func = test_function_with_reference_copy();
    let liveness = compute_reference_liveness(&func);

    assert!(liveness.entry_sets[0].is_empty());
    assert!(liveness.before_statement_sets[0]
        .iter()
        .any(|set| set.iter().any(|local| local == Local(1))));
}
```

If `test_function_with_reference_copy` does not exist, add a local helper in the test module that builds a `MirFunction` with one reference local copied into another and later read.

- [ ] **Step 2: Run tests and verify they fail**

Run: `cargo test -p rock-lib reference_liveness_uses_local_set_at_statement_boundaries -- --nocapture`

Expected: FAIL to compile or fail assertions because liveness still uses `HashSet<Local>`.

- [ ] **Step 3: Change ReferenceLiveness fields**

In `liveness.rs`, replace `HashSet<Local>` liveness fields with `LocalSet`:

```rust
use crate::mir::dataflow::{LocalSet, Lattice};

#[derive(Clone, Debug)]
pub struct ReferenceLiveness {
    pub entry_sets: Vec<LocalSet>,
    pub exit_sets: Vec<LocalSet>,
    pub before_statement_sets: Vec<Vec<LocalSet>>,
    pub after_statement_sets: Vec<Vec<LocalSet>>,
}
```

Change `reference_locals`, `borrow_locals`, and helper parameters that represent dense local sets to `LocalSet`. Keep temporary `Vec<Local>` snapshots where reverse iteration needs stable iteration order.

- [ ] **Step 4: Replace set operations**

Use the new API:

```rust
let mut live_out = LocalSet::new();
live_out.join(&entry_sets[succ]);
live.insert(local);
live.remove(local);
live.contains(local);
live.iter().collect::<Vec<_>>()
```

Do not keep `HashSet<Local>` in `ReferenceLiveness`, `reference_locals`, or `borrow_locals` after this task.

- [ ] **Step 5: Update loan release helpers**

Change `release_dead_reference_loans` to accept `&LocalSet`. For each owner set in active state or temporary legacy state, check liveness with `live_locals.contains(owner)`.

- [ ] **Step 6: Run focused verification**

Run: `cargo test -p rock-lib mir::borrowck::liveness -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::borrowck::liveness -- --nocapture && cargo test -p rock-lib mir::borrowck -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/borrowck/liveness.rs lib/src/mir/dataflow/bitset.rs
git commit -m "use indexed local sets for borrow liveness"
```

## Task 6: Convert Active Loan Propagation To LoanState

**Files:**
- Modify: `lib/src/mir/borrowck/liveness.rs`
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Test: `lib/src/mir/borrowck/liveness.rs`
- Test: `lib/src/mir/dataflow/analyses/loans.rs`

- [ ] **Step 1: Write failing active loan state tests**

Add this test to `liveness.rs`:

```rust
#[test]
fn active_loan_entries_use_indexed_state_and_merge_owners() {
    let (func, merge_block, owner_a, owner_b) = branch_merge_borrow_function();
    let borrows = crate::mir::borrowck::borrows::collect_function_borrows(&func);
    let table = crate::mir::dataflow::analyses::LoanTable::from_borrows(&borrows);
    let by_location = group_loans_by_location(&table);
    let reference_liveness = compute_reference_liveness(&func);

    let entries = compute_active_loan_entries(&func, &table, &by_location, &reference_liveness);
    let loan_id = table.iter().next().expect("loan").id;
    let merged_owners = entries[merge_block.0].owners(loan_id);

    assert!(merged_owners.contains(owner_a));
    assert!(merged_owners.contains(owner_b));
}
```

If `branch_merge_borrow_function` does not exist, create a small test helper in the same module that returns `(MirFunction, BasicBlockId, Local, Local)`. It should build two predecessors that copy the same reference owner into different locals before the returned merge block, so the assertion proves the merged entry state contains both owners for one loan.

- [ ] **Step 2: Run test and verify it fails**

Run: `cargo test -p rock-lib active_loan_entries_use_indexed_state_and_merge_owners -- --nocapture`

Expected: FAIL because `group_loans_by_location` and indexed `compute_active_loan_entries` do not exist.

- [ ] **Step 3: Add location grouping helper**

In `liveness.rs`, add:

```rust
pub type LoansByLocation = std::collections::HashMap<crate::mir::borrowck::location::Location, Vec<crate::ids::LoanId>>;

pub fn group_loans_by_location(table: &crate::mir::dataflow::analyses::LoanTable) -> LoansByLocation {
    let mut grouped = LoansByLocation::new();
    for loan in table.iter() {
        grouped.entry(loan.created_at).or_default().push(loan.id);
    }
    grouped
}
```

- [ ] **Step 4: Rewrite active loan propagation signature**

Replace `compute_active_loan_entries` with:

```rust
pub fn compute_active_loan_entries(
    func: &MirFunction,
    table: &crate::mir::dataflow::analyses::LoanTable,
    loan_at_location: &LoansByLocation,
    reference_liveness: &ReferenceLiveness,
) -> Vec<crate::mir::dataflow::analyses::LoanState>
```

Initialize entry sets with `LoanState::new(table.len())`. Use `state.join(&exit_sets[block_idx])` instead of `join_loan_state`.

- [ ] **Step 5: Rewrite statement transfer**

Inside active loan propagation, use typed locations:

```rust
let location = crate::mir::borrowck::location::Location::new(
    crate::mir::BasicBlockId(block_idx),
    crate::mir::borrowck::location::StatementIndex(stmt_idx),
);
if let Some(loan_ids) = loan_at_location.get(&location) {
    for loan_id in loan_ids {
        state.activate(*loan_id, table);
    }
}
```

Replace legacy release/transfer calls with `LoanState` methods:

```rust
state.release_owner(*local);
state.copy_owner(src.local, dest.local);
state.transfer_owner(src.local, dest.local);
state.retain_owners(|owner| live_locals.contains(owner));
```

Preserve the same statement-order effects described in the spec.

- [ ] **Step 6: Remove old join_loan_state helper**

Delete the `join_loan_state` function after all active propagation uses `LoanState::join`.

- [ ] **Step 7: Run focused verification**

Run: `cargo test -p rock-lib active_loan_entries_use_indexed_state_and_merge_owners -- --nocapture && cargo test -p rock-lib mir::borrowck::liveness -- --nocapture`

Expected: PASS.

- [ ] **Step 8: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::borrowck::liveness -- --nocapture && cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/borrowck/liveness.rs lib/src/mir/dataflow/analyses/loans.rs
git commit -m "propagate mir loans with indexed state"
```

## Task 7: Wire BorrowChecker To Indexed LoanState

**Files:**
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Modify: `lib/src/mir/borrowck/provenance.rs`
- Modify: `lib/src/mir/borrowck/liveness.rs`
- Test: `lib/src/mir/borrowck/mod.rs`

- [ ] **Step 1: Write failing no-legacy-map test**

Add this test to the borrowck test module in `lib/src/mir/borrowck/mod.rs`:

```rust
#[test]
fn borrowck_uses_indexed_loan_state_for_active_conflicts() {
    let func = mut_borrow_then_shared_read_function();
    let err = BorrowChecker::check_function(&func).expect_err("borrow conflict");

    assert!(err.0.iter().any(|diag| {
        diag.message.contains("Cannot borrow") || diag.message.contains("borrow")
    }));
}
```

If the test helper does not exist, add a private helper in the same test module that builds a `MirFunction` with:

- `x: I64`
- `r: &mut I64 = &mut x`
- a later read or shared borrow of `x`

The failure should currently pass through legacy map state; after the implementation, it proves the indexed path still catches conflicts.

- [ ] **Step 2: Run test and verify existing state still passes**

Run: `cargo test -p rock-lib borrowck_uses_indexed_loan_state_for_active_conflicts -- --nocapture`

Expected before wiring: PASS or compile with legacy state. This is a characterization test; keep it to guard behavior while replacing internals.

- [ ] **Step 3: Replace loan setup in check_function**

In `BorrowChecker::check_function`, replace manual `all_loans` and `loan_at_location` maps with:

```rust
let borrows = collect_function_borrows(func);
let loan_table = crate::mir::dataflow::analyses::LoanTable::from_borrows(&borrows);
let loans_by_location = crate::mir::borrowck::liveness::group_loans_by_location(&loan_table);
let reference_liveness = compute_reference_liveness(func);
let active_loan_entries = compute_active_loan_entries(
    func,
    &loan_table,
    &loans_by_location,
    &reference_liveness,
);
```

Keep `borrows` for diagnostics until diagnostics are moved fully onto `LoanData`.

- [ ] **Step 4: Update statement loop active state**

Change `active_loans` variables from maps to `LoanState`. When filtering live loans before a statement, clone the current indexed state and retain owners through the current live-local set:

```rust
let mut live_loans = active_loans.clone();
if let Some(live_locals) = reference_liveness
    .before_statement_sets
    .get(block_idx)
    .and_then(|stmt_sets| stmt_sets.get(stmt_idx))
{
    live_loans.retain_owners(|owner| live_locals.contains(owner));
}
```

After the existing checks for the statement, update the mutable `active_loans` in this order:

```rust
init_analysis.apply_statement(&mut state, stmt);

if let StatementKind::Assign(dest, Rvalue::Use(Operand::Move(src))) = &stmt.kind {
    active_loans.transfer_owner(src.local, dest.local);
}

if let StatementKind::Assign(dest, Rvalue::Use(Operand::Copy(src))) = &stmt.kind {
    active_loans.copy_owner(src.local, dest.local);
}

if let StatementKind::Assign(dest, Rvalue::Cast(op, Type::Pointer(_))) = &stmt.kind {
    if let Operand::Copy(place) | Operand::Move(place) = op {
        let should_transfer = func
            .local_decls
            .get(place.local.0)
            .map(|decl| matches!(&decl.ty, Type::Reference { mutable: true, .. }))
            .unwrap_or(false);
        if should_transfer {
            active_loans.transfer_owner(place.local, dest.local);
        }
    }
}

let location = crate::mir::borrowck::location::Location::new(
    crate::mir::BasicBlockId(block_idx),
    crate::mir::borrowck::location::StatementIndex(stmt_idx),
);
if let Some(loan_ids) = loans_by_location.get(&location) {
    for loan_id in loan_ids {
        active_loans.activate(*loan_id, &loan_table);
    }
}

if let Some(live_locals) = reference_liveness
    .after_statement_sets
    .get(block_idx)
    .and_then(|stmt_sets| stmt_sets.get(stmt_idx))
{
    active_loans.retain_owners(|owner| live_locals.contains(owner));
}

if let StatementKind::StorageDead(local) = &stmt.kind {
    active_loans.release_owner(*local);
}
```

Do not leave the old `HashMap<LoanId, Loan>` transfer or release calls in the statement loop.

- [ ] **Step 5: Update loan check function signatures**

Keep `check_statement` focused on initialization checking. It should accept the indexed state only if it still needs it for debug tracing or future assertions; loan conflict checking stays in `check_statement_loans` so all calls to `LoanAnalysis::check_aliasing` have both `LoanTable` and `LoanState` available.

```rust
fn check_statement(
    stmt: &StatementData,
    state: &InitMap,
    active_loans: &LoanState,
    func: &MirFunction,
    diagnostics: &mut Diagnostics,
)

fn check_statement_loans(
    stmt: &StatementData,
    table: &LoanTable,
    active_loans: &LoanState,
    func: &MirFunction,
    borrows: &[crate::mir::borrowck::borrows::BorrowData],
    diagnostics: &mut Diagnostics,
)

fn check_terminator(
    term: &Terminator,
    state: &InitMap,
    table: &LoanTable,
    active_loans: &LoanState,
    func: &MirFunction,
    borrows: &[crate::mir::borrowck::borrows::BorrowData],
    diagnostics: &mut Diagnostics,
)
```

Update calls to `LoanAnalysis::check_aliasing` inside `check_statement_loans` and `check_terminator` to pass `table` and `active_loans`. Do not call `LoanAnalysis::check_aliasing` from `check_statement` unless its signature is expanded to include `&LoanTable` and the diagnostics borrow lookup.

- [ ] **Step 6: Update conflict diagnostic lookup**

When `LoanAnalysis::check_aliasing` returns `LoanId`, find the original span from `LoanTable` first:

```rust
let origin_span = table
    .get(conflicting_loan)
    .and_then(|loan| loan.origin_span.clone());
```

If existing `borrow_error` requires `BorrowData`, keep a small lookup from `LoanId` raw index to `borrows[index]` while preserving spans. Remove this lookup in a follow-up only if diagnostics can be emitted directly from `LoanData` without changing messages.

- [ ] **Step 7: Remove legacy active loan helpers**

Delete or stop exporting:

- `LegacyLoanState`
- old map-based `LoanAnalysis::check_aliasing`
- map-based `release_loans_owned_by`
- map-based `transfer_reference_loans_owned_by`
- map-based `resolve_place`

Search with:

```bash
rg "HashMap<LoanId, Loan>|LegacyLoanState|pub struct LoanId|type LoanState = HashMap|owners: HashSet<Local>" lib/src/mir --glob '*.rs'
```

Expected remaining hits: none in production borrowck/dataflow active loan state. `HashSet<Local>` may still appear in initialization analysis or non-loan helper tests.

- [ ] **Step 8: Run focused verification**

Run: `cargo test -p rock-lib borrowck_uses_indexed_loan_state_for_active_conflicts -- --nocapture && cargo test -p rock-lib mir::borrowck -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Run behavior verification**

Run these commands serially:

```bash
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
```

Expected: PASS.

- [ ] **Step 10: Run task verification and commit**

Run: `cargo fmt --all --check && cargo test -p rock-lib mir::borrowck -- --nocapture && cargo test -p rock-lib --test integration test_borrow_ -- --nocapture && git diff --check`

Expected: PASS.

Commit:

```bash
git add lib/src/mir/borrowck lib/src/mir/dataflow/analyses/loans.rs lib/src/mir/dataflow/analyses/mod.rs
git commit -m "wire borrowck to indexed loan state"
```

## Task 8: Final Task 20 Audit And Verification

**Files:**
- Modify if needed: `lib/src/mir/borrowck/**`
- Modify if needed: `lib/src/mir/dataflow/**`

- [ ] **Step 1: Audit for legacy active loan state**

Run:

```bash
rg "HashMap<LoanId, Loan>|LegacyLoanState|pub struct LoanId|type LoanState = HashMap|owners: HashSet<Local>|HashMap<\(usize, usize\), Vec<LoanId>>" lib/src/mir --glob '*.rs'
```

Expected: no production hits for active loan propagation. Any remaining hit must be in unrelated initialization analysis, non-loan tests, or a helper that does not participate in active loan state.

- [ ] **Step 2: Audit typed location use**

Run:

```bash
rg "block: usize|statement: usize|\(usize, usize\)" lib/src/mir/borrowck lib/src/mir/dataflow --glob '*.rs'
```

Expected: no borrow creation or active loan location tracking uses raw `(usize, usize)`. Raw indexes in local test helpers are acceptable only when immediately wrapped as `BasicBlockId`, `StatementIndex`, or `Location`.

- [ ] **Step 3: Run focused verification**

Run these commands serially:

```bash
cargo test -p rock-lib mir::dataflow -- --nocapture
cargo test -p rock-lib mir::borrowck -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Run behavior filters**

Run these commands serially:

```bash
cargo test -p rock-lib --test integration test_borrow_ -- --nocapture
cargo test -p rock-lib --test integration test_closure -- --nocapture
cargo test -p rock-lib --test integration test_array -- --nocapture
cargo test -p rock-lib --test integration test_vec_index -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run full verification**

Run these commands serially:

```bash
cargo fmt --all --check
cargo test -p rock-lib
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Request final code review**

Use the requesting-code-review skill with this context:

```text
Description: Finished roadmap Task 20 by converting MIR borrowck active loan state from HashMap/HashSet data structures to typed locations, indexed loan tables, and dense local/loan sets while preserving diagnostics and behavior.

Spec: docs/superpowers/specs/2026-05-23-borrowck-indexed-dataflow-design.md
Plan: docs/superpowers/plans/2026-05-23-borrowck-indexed-dataflow.md

Review focus:
- Borrow creation locations use typed Location/StatementIndex values.
- Active loan propagation no longer uses HashMap<LoanId, Loan> or HashSet<Local> owner sets.
- crate::ids::LoanId is the canonical loan ID.
- LoanTable stores immutable facts and LoanState stores path-sensitive active/owner data.
- Reference liveness keeps before/after statement precision.
- Provenance and conflict checks preserve current behavior and diagnostics.
- HIR codegen remains unchanged.
```

Expected: reviewer returns pass or findings. Fix Critical and Important findings before continuing.

- [ ] **Step 7: Commit final cleanup if needed**

If audit or review fixes changed files, commit them:

```bash
git add lib/src/mir/borrowck lib/src/mir/dataflow lib/tests/integration.rs
git commit -m "finish borrowck indexed dataflow conversion"
```

If there are no uncommitted changes after prior task commits, do not create an empty commit.

## Completion Check

Task 20 is complete only when:

- `cargo test -p rock-lib mir::borrowck -- --nocapture` passes.
- `cargo test -p rock-lib mir::dataflow -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_borrow_ -- --nocapture` passes.
- `cargo test -p rock-lib --test integration test_closure -- --nocapture` passes.
- `cargo test -p rock-lib` passes.
- `cargo fmt --all --check` passes.
- `git diff --check` passes.
- Final code review has no Critical or Important findings.
