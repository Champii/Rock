# Borrowck Indexed Place Path Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Do not commit unless the user explicitly requests a commit.

**Goal:** Wire `PlacePathId` and `MovePathId` through borrowck as the long-term indexed place/move path model.

**Architecture:** Add focused borrowck path-table primitives first, then migrate loan storage/provenance/conflicts, then migrate initialization and move state. Keep original `Place` values for diagnostics while semantic checks use indexed path IDs.

**Tech Stack:** Rust 2021, `rock-lib`, MIR borrowck modules, `crate::ids::{PlacePathId, MovePathId, IdGen, Idx}`, existing `LoanId`/`LoanTable`/`LoanState`, focused `cargo test -p rock-lib mir::borrowck` and `mir::dataflow` tests.

---

## File Structure

- Create `lib/src/mir/borrowck/paths.rs`: `PlacePathData`, `PlacePathTable`, `MovePathData`, `MovePathTable`, and path conflict helpers.
- Modify `lib/src/mir/borrowck/mod.rs`: export `paths`, create path tables per function, pass them into loan and initialization analysis.
- Modify `lib/src/mir/dataflow/analyses/loans.rs`: add `place_path: PlacePathId` to `LoanData`, make `LoanTable` own the function `PlacePathTable`, and check aliasing by indexed paths.
- Modify `lib/src/mir/borrowck/provenance.rs`: resolve reborrow provenance through `PlacePathTable` while preserving structural `Place` diagnostics.
- Modify `lib/src/mir/dataflow/analyses/init.rs`: replace root-local-only initialization state with `MovePathId` state over a `MovePathTable`.
- Modify `lib/src/mir/dataflow/analyses/mod.rs`: re-export any new public analysis types needed by `BorrowChecker` tests.

## Task 1: Add Indexed Place Path Tables

**Files:**
- Create: `lib/src/mir/borrowck/paths.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/borrowck/paths.rs`

- [ ] **Step 1: Write failing path table tests**

Add `lib/src/mir/borrowck/paths.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Idx;
    use crate::mir::{Local, Place, Projection};

    fn local(local: usize) -> Place {
        Place { local: Local(local), projection: Vec::new() }
    }

    fn field(local: usize, index: usize) -> Place {
        Place {
            local: Local(local),
            projection: vec![Projection::Field { index, identity: None }],
        }
    }

    #[test]
    fn place_path_table_interns_places_once_and_records_parent() {
        let mut table = PlacePathTable::new();

        let root = table.intern(local(1));
        let field0 = table.intern(field(1, 0));
        let field0_again = table.intern(field(1, 0));

        assert_eq!(root.raw(), 0);
        assert_eq!(field0, field0_again);
        assert_eq!(table.get(field0).unwrap().parent, Some(root));
        assert_eq!(table.place(field0), Some(&field(1, 0)));
    }

    #[test]
    fn place_path_table_distinguishes_disjoint_fields() {
        let mut table = PlacePathTable::new();
        let field0 = table.intern(field(1, 0));
        let field1 = table.intern(field(1, 1));

        assert!(!table.paths_conflict(field0, field1));
        assert!(table.paths_conflict(table.intern(local(1)), field0));
    }
}
```

- [ ] **Step 2: Run the tests and verify RED**

Run: `cargo test -p rock-lib mir::borrowck::paths -- --nocapture`

Expected: compile failure because `paths` module and `PlacePathTable` do not exist.

- [ ] **Step 3: Implement path table primitives**

Replace the file body above the tests with:

```rust
use std::collections::HashMap;

use crate::ids::{IdGen, Idx, PlacePathId};
use crate::mir::{Place, Projection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacePathData {
    pub id: PlacePathId,
    pub place: Place,
    pub parent: Option<PlacePathId>,
}

#[derive(Debug, Default)]
pub struct PlacePathTable {
    paths: Vec<PlacePathData>,
    by_place: HashMap<Place, PlacePathId>,
    ids: IdGen<PlacePathId>,
}

impl PlacePathTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, place: Place) -> PlacePathId {
        if let Some(id) = self.by_place.get(&place).copied() {
            return id;
        }

        let parent = parent_place(&place).map(|parent| self.intern(parent));
        let id = self.ids.fresh();
        self.paths.push(PlacePathData { id, place: place.clone(), parent });
        self.by_place.insert(place, id);
        id
    }

    pub fn get(&self, id: PlacePathId) -> Option<&PlacePathData> {
        self.paths.get(id.index())
    }

    pub fn place(&self, id: PlacePathId) -> Option<&Place> {
        self.get(id).map(|data| &data.place)
    }

    pub fn path_id(&self, place: &Place) -> Option<PlacePathId> {
        self.by_place.get(place).copied()
    }

    pub fn paths_conflict(&self, a: PlacePathId, b: PlacePathId) -> bool {
        let Some(a) = self.place(a) else { return false; };
        let Some(b) = self.place(b) else { return false; };
        crate::mir::borrowck::conflicts::places_conflict(a, b)
    }
}

fn parent_place(place: &Place) -> Option<Place> {
    if place.projection.is_empty() {
        return None;
    }
    let mut parent = place.clone();
    parent.projection.pop();
    Some(parent)
}
```

- [ ] **Step 4: Register the module**

Add to `lib/src/mir/borrowck/mod.rs` with the other `pub mod` lines:

```rust
pub mod paths;
```

- [ ] **Step 5: Run GREEN tests**

Run: `cargo test -p rock-lib mir::borrowck::paths -- --nocapture`

Expected: `2 passed; 0 failed` for the new module tests.

## Task 2: Build Function-Level Path Tables

**Files:**
- Modify: `lib/src/mir/borrowck/paths.rs`
- Test: `lib/src/mir/borrowck/paths.rs`

- [ ] **Step 1: Write failing function scan test**

Append this test to `paths.rs`:

```rust
#[test]
fn place_path_table_collects_function_places() {
    use crate::mir::{BasicBlock, LocalDecl, MirFunction, MirFunctionId, Mutability, Operand, Rvalue, StatementData};

    let mut type_context = crate::type_context::TypeContext::new();
    let i64_id = type_context.intern_type(&crate::types::Type::I64);
    let function = MirFunction {
        id: MirFunctionId::Function(crate::ids::DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(0))),
        name: "scan".to_string(),
        basic_blocks: vec![BasicBlock {
            statements: vec![StatementData::assign(
                field(0, 1),
                Rvalue::Use(Operand::Copy(field(1, 0))),
                None,
            )],
            terminator: None,
        }],
        local_decls: vec![
            LocalDecl { ty: i64_id, mutability: Mutability::Mut, name: Some("a".to_string()), span: None },
            LocalDecl { ty: i64_id, mutability: Mutability::Mut, name: Some("b".to_string()), span: None },
        ],
        closure_captures: Vec::new(),
        arg_count: 0,
        ret_type: i64_id,
    };

    let table = PlacePathTable::for_function(&function);

    assert!(table.path_id(&local(0)).is_some());
    assert!(table.path_id(&field(0, 1)).is_some());
    assert!(table.path_id(&field(1, 0)).is_some());
}
```

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test -p rock-lib mir::borrowck::paths::tests::place_path_table_collects_function_places -- --exact --nocapture`

Expected: compile failure because `PlacePathTable::for_function` does not exist.

- [ ] **Step 3: Implement function scanning**

Add imports and methods in `paths.rs`:

```rust
use crate::mir::{MirFunction, Operand, Rvalue, StatementKind, Terminator};

impl PlacePathTable {
    pub fn for_function(function: &MirFunction) -> Self {
        let mut table = Self::new();
        for local in 0..function.local_decls.len() {
            table.intern(Place { local: crate::mir::Local(local), projection: Vec::new() });
        }
        for block in &function.basic_blocks {
            for statement in &block.statements {
                collect_statement_places(statement, &mut table);
            }
            if let Some(terminator) = &block.terminator {
                collect_terminator_places(terminator, &mut table);
            }
        }
        table
    }
}

fn collect_statement_places(statement: &crate::mir::StatementData, table: &mut PlacePathTable) {
    match &statement.kind {
        StatementKind::Assign(dest, rvalue) => {
            table.intern(dest.clone());
            collect_rvalue_places(rvalue, table);
        }
        StatementKind::Assert(assertion) => {
            for operand in &assertion.operands {
                collect_operand_place(operand, table);
            }
        }
        StatementKind::StorageLive(local) | StatementKind::StorageDead(local) => {
            table.intern(Place { local: *local, projection: Vec::new() });
        }
    }
}

fn collect_rvalue_places(rvalue: &Rvalue, table: &mut PlacePathTable) {
    match rvalue {
        Rvalue::Use(operand) | Rvalue::Cast(operand, _) | Rvalue::UnaryOp(_, operand) => {
            collect_operand_place(operand, table);
        }
        Rvalue::Ref(_, place) | Rvalue::Discriminant(place) => {
            table.intern(place.clone());
        }
        Rvalue::BinaryOp(_, left, right) => {
            collect_operand_place(left, table);
            collect_operand_place(right, table);
        }
        Rvalue::Aggregate(_, operands) => {
            for operand in operands {
                collect_operand_place(operand, table);
            }
        }
        Rvalue::Closure(closure) => {
            for capture in &closure.captures {
                table.intern(capture.place());
            }
        }
    }
}

fn collect_terminator_places(terminator: &Terminator, table: &mut PlacePathTable) {
    match terminator {
        Terminator::Call { func, args, destination, .. } => {
            collect_operand_place(func, table);
            for arg in args {
                collect_operand_place(arg, table);
            }
            table.intern(destination.clone());
        }
        Terminator::SwitchInt { discr, .. } => collect_operand_place(discr, table),
        Terminator::Drop { place, .. } => {
            table.intern(place.clone());
        }
        Terminator::Goto(_) | Terminator::Return => {}
    }
}

fn collect_operand_place(operand: &Operand, table: &mut PlacePathTable) {
    if let Operand::Copy(place) | Operand::Move(place) = operand {
        table.intern(place.clone());
    }
}
```

- [ ] **Step 4: Run module tests**

Run: `cargo test -p rock-lib mir::borrowck::paths -- --nocapture`

Expected: all `paths` tests pass.

## Task 3: Add Move Path Table And State

**Files:**
- Modify: `lib/src/mir/borrowck/paths.rs`
- Test: `lib/src/mir/borrowck/paths.rs`

- [ ] **Step 1: Write failing move path tests**

Append:

```rust
#[test]
fn move_path_table_maps_to_place_paths_and_tracks_children() {
    let mut places = PlacePathTable::new();
    let root_place = places.intern(local(2));
    let child_place = places.intern(field(2, 0));
    let moves = MovePathTable::from_place_paths(&places);

    let root_move = moves.move_path_for_place(root_place).unwrap();
    let child_move = moves.move_path_for_place(child_place).unwrap();

    assert_eq!(moves.place_path(root_move), Some(root_place));
    assert!(moves.is_ancestor(root_move, child_move));
    assert!(!moves.is_ancestor(child_move, root_move));
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p rock-lib mir::borrowck::paths::tests::move_path_table_maps_to_place_paths_and_tracks_children -- --exact --nocapture`

Expected: compile failure for missing `MovePathTable`.

- [ ] **Step 3: Implement move path table**

Add:

```rust
use crate::ids::MovePathId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePathData {
    pub id: MovePathId,
    pub place_path: PlacePathId,
    pub parent: Option<MovePathId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MovePathTable {
    paths: Vec<MovePathData>,
    by_place_path: HashMap<PlacePathId, MovePathId>,
}

impl MovePathTable {
    pub fn from_place_paths(places: &PlacePathTable) -> Self {
        let mut table = Self::default();
        for place_path in places.iter_ids() {
            table.intern_from_place_path(place_path, places);
        }
        table
    }

    pub fn move_path_for_place(&self, place_path: PlacePathId) -> Option<MovePathId> {
        self.by_place_path.get(&place_path).copied()
    }

    pub fn place_path(&self, id: MovePathId) -> Option<PlacePathId> {
        self.paths.get(id.index()).map(|data| data.place_path)
    }

    pub fn is_ancestor(&self, ancestor: MovePathId, child: MovePathId) -> bool {
        let mut current = Some(child);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.paths.get(id.index()).and_then(|data| data.parent);
        }
        false
    }

    fn intern_from_place_path(
        &mut self,
        place_path: PlacePathId,
        places: &PlacePathTable,
    ) -> MovePathId {
        if let Some(id) = self.by_place_path.get(&place_path).copied() {
            return id;
        }
        let parent = places
            .get(place_path)
            .and_then(|data| data.parent)
            .map(|parent| self.intern_from_place_path(parent, places));
        let id = MovePathId(self.paths.len() as u32);
        self.paths.push(MovePathData { id, place_path, parent });
        self.by_place_path.insert(place_path, id);
        id
    }
}
```

Also add this iterator to `PlacePathTable`:

```rust
pub fn iter_ids(&self) -> impl Iterator<Item = PlacePathId> + '_ {
    self.paths.iter().map(|data| data.id)
}
```

- [ ] **Step 4: Run path tests**

Run: `cargo test -p rock-lib mir::borrowck::paths -- --nocapture`

Expected: all path tests pass.

## Task 4: Store Loan Paths By `PlacePathId`

**Files:**
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/dataflow/analyses/loans.rs`

- [ ] **Step 1: Write failing loan table test**

Add to `loans.rs` tests:

```rust
#[test]
fn loan_table_records_indexed_place_paths() {
    use crate::mir::borrowck::borrows::{BorrowData, BorrowId};
    use crate::mir::borrowck::location::{Location, StatementIndex};
    use crate::mir::borrowck::paths::PlacePathTable;
    use crate::mir::{BasicBlockId, Local, Place};

    let place = Place { local: Local(1), projection: Vec::new() };
    let mut paths = PlacePathTable::new();
    let place_path = paths.intern(place.clone());
    let borrows = vec![BorrowData {
        id: BorrowId(0),
        owner: Local(2),
        place,
        kind: crate::mir::borrowck::accesses::AccessKind::BorrowShared,
        created_at: Location::new(BasicBlockId(0), StatementIndex(0)),
        origin_span: None,
    }];

    let table = LoanTable::from_borrows_with_paths(&borrows, paths);

    assert_eq!(table.get(crate::ids::LoanId(0)).unwrap().place_path, place_path);
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p rock-lib mir::dataflow::analyses::loans::tests::loan_table_records_indexed_place_paths -- --exact --nocapture`

Expected: compile failure for missing `from_borrows_with_paths` and `place_path`.

- [ ] **Step 3: Implement loan path storage**

Change `LoanData` and `LoanTable`:

```rust
pub struct LoanData {
    pub id: LoanId,
    pub place: Place,
    pub place_path: crate::ids::PlacePathId,
    pub initial_owner: Local,
    pub kind: LoanKind,
    pub origin_span: Option<crate::lexer::Span>,
    pub created_at: Location,
}
```

Add `place_paths` to `LoanTable` and add:

```rust
pub fn from_borrows_with_paths(
    borrows: &[crate::mir::borrowck::borrows::BorrowData],
    mut place_paths: crate::mir::borrowck::paths::PlacePathTable,
) -> Self {
    let mut table = Self { loans: Vec::new(), place_paths };
    for borrow in borrows {
        let kind = match borrow.kind {
            crate::mir::borrowck::accesses::AccessKind::BorrowShared => LoanKind::Shared,
            crate::mir::borrowck::accesses::AccessKind::BorrowMut => LoanKind::Mut,
            _ => continue,
        };
        let place_path = table.place_paths.intern(borrow.place.clone());
        table.push(LoanData {
            id: crate::ids::LoanId(0),
            place: borrow.place.clone(),
            place_path,
            initial_owner: borrow.owner,
            kind,
            origin_span: borrow.origin_span.clone(),
            created_at: borrow.created_at,
        });
    }
    table
}

pub fn place_paths(&self) -> &crate::mir::borrowck::paths::PlacePathTable {
    &self.place_paths
}
```

Keep `from_borrows` as a compatibility wrapper that builds a fresh table from borrow places only.

- [ ] **Step 4: Wire `BorrowChecker` to provide function paths**

In `BorrowChecker::check_function_with_context`, replace:

```rust
let loan_table = LoanTable::from_borrows(&borrows);
```

with:

```rust
let place_paths = crate::mir::borrowck::paths::PlacePathTable::for_function(func);
let loan_table = LoanTable::from_borrows_with_paths(&borrows, place_paths);
```

- [ ] **Step 5: Run loan tests**

Run: `cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture`

Expected: loan tests pass.

## Task 5: Use Indexed Paths For Loan Conflicts And Provenance

**Files:**
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Modify: `lib/src/mir/borrowck/provenance.rs`
- Test: `lib/src/mir/dataflow/analyses/loans.rs`
- Test: `lib/src/mir/borrowck/provenance.rs`

- [ ] **Step 1: Write failing conflict test**

Add to `loans.rs` tests:

```rust
#[test]
fn loan_aliasing_uses_indexed_disjoint_field_paths() {
    use crate::mir::borrowck::paths::PlacePathTable;
    use crate::mir::{Local, Place, Projection};

    let mut paths = PlacePathTable::new();
    let loan_place = Place {
        local: Local(1),
        projection: vec![Projection::Field { index: 0, identity: None }],
    };
    let access_place = Place {
        local: Local(1),
        projection: vec![Projection::Field { index: 1, identity: None }],
    };
    let loan_path = paths.intern(loan_place.clone());
    paths.intern(access_place.clone());
    let mut table = LoanTable { loans: Vec::new(), place_paths: paths };
    let loan_id = table.push(LoanData {
        id: crate::ids::LoanId(0),
        place: loan_place,
        place_path: loan_path,
        initial_owner: Local(2),
        kind: LoanKind::Mut,
        origin_span: None,
        created_at: crate::mir::borrowck::location::Location::new(
            crate::mir::BasicBlockId(0),
            crate::mir::borrowck::location::StatementIndex(0),
        ),
    });
    let mut state = LoanState::new(table.len());
    state.activate(loan_id, &table);

    assert!(LoanAnalysis::check_aliasing(&access_place, LoanKind::Mut, &table, &state).is_ok());
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p rock-lib mir::dataflow::analyses::loans::tests::loan_aliasing_uses_indexed_disjoint_field_paths -- --exact --nocapture`

Expected: compile errors until `LoanTable` exposes the path table and `LoanData::place_path` exists.

- [ ] **Step 3: Implement indexed conflict path lookup**

In `LoanAnalysis::check_aliasing`, intern or look up the access place in the table path map:

```rust
let access_path = table
    .place_paths()
    .path_id(&place)
    .unwrap_or_else(|| table.place_paths().path_id(&original_place).unwrap_or(loan.place_path));
```

Then compare with:

```rust
if table.place_paths().paths_conflict(access_path, loan.place_path) {
    match (kind, loan.kind) {
        (LoanKind::Mut, _) | (_, LoanKind::Mut) => return Err(loan_id),
        (LoanKind::Shared, LoanKind::Shared) => {}
    }
}
```

Keep `loan.place` for diagnostics and provenance fallbacks.

- [ ] **Step 4: Run conflict tests**

Run: `cargo test -p rock-lib mir::dataflow::analyses::loans -- --nocapture`

Expected: loan tests pass.

## Task 6: Convert Initialization To Move Paths

**Files:**
- Modify: `lib/src/mir/dataflow/analyses/init.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/dataflow/analyses/init.rs`

- [ ] **Step 1: Write failing move-path init test**

Add to `init.rs` tests:

```rust
#[test]
fn initialization_tracks_move_path_for_field_move() {
    use crate::mir::borrowck::paths::{MovePathTable, PlacePathTable};
    use crate::mir::{Local, Place, Projection};

    let mut places = PlacePathTable::new();
    let root_place = places.intern(Place { local: Local(1), projection: Vec::new() });
    let field_place = places.intern(Place {
        local: Local(1),
        projection: vec![Projection::Field { index: 0, identity: None }],
    });
    let moves = MovePathTable::from_place_paths(&places);
    let mut state = InitMap::new_for_move_paths(&moves, InitState::Init);

    state.set_moved(field_place, &moves, MoveInfo { span: crate::lexer::Span::default() });

    assert!(state.check_place_path(field_place, &moves).is_err());
    assert!(state.check_place_path(root_place, &moves).is_err());
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p rock-lib mir::dataflow::analyses::init::tests::initialization_tracks_move_path_for_field_move -- --exact --nocapture`

Expected: compile failure because `InitMap` is still local-keyed.

- [ ] **Step 3: Add move-path `InitMap` storage**

Change `InitMap` from `HashMap<Local, InitState>` to a struct that stores move path state and local roots:

```rust
#[derive(Debug, Clone)]
pub struct InitMap {
    states: Vec<InitState>,
    local_roots: HashMap<Local, crate::ids::MovePathId>,
}
```

Add methods:

```rust
impl InitMap {
    pub fn new_for_move_paths(
        moves: &crate::mir::borrowck::paths::MovePathTable,
        initial: InitState,
    ) -> Self {
        let mut states = Vec::new();
        for id in moves.iter_ids() {
            if states.len() <= id.index() {
                states.resize(id.index() + 1, initial.clone());
            }
            states[id.index()] = initial.clone();
        }
        Self { states, local_roots: moves.local_roots() }
    }

    pub fn set_moved(
        &mut self,
        place_path: crate::ids::PlacePathId,
        moves: &crate::mir::borrowck::paths::MovePathTable,
        info: MoveInfo,
    ) {
        if let Some(move_path) = moves.move_path_for_place(place_path) {
            for id in moves.descendants_inclusive(move_path) {
                self.states[id.index()] = InitState::Moved(info.clone());
            }
        }
    }

    pub fn check_place_path(
        &self,
        place_path: crate::ids::PlacePathId,
        moves: &crate::mir::borrowck::paths::MovePathTable,
    ) -> Result<(), InitError> {
        let Some(move_path) = moves.move_path_for_place(place_path) else { return Ok(()); };
        for id in moves.ancestors_inclusive(move_path) {
            match self.states.get(id.index()) {
                Some(InitState::Init) => {}
                Some(InitState::Uninit) => {
                    return Err(InitError { message: "use of uninitialized place".to_string(), move_span: None });
                }
                Some(InitState::Moved(info)) => {
                    return Err(InitError { message: "use of moved value".to_string(), move_span: Some(info.span.clone()) });
                }
                None => return Ok(()),
            }
        }
        for id in moves.descendants(move_path) {
            if let Some(InitState::Moved(info)) = self.states.get(id.index()) {
                return Err(InitError { message: "use of partially moved value".to_string(), move_span: Some(info.span.clone()) });
            }
        }
        Ok(())
    }
}
```

Add `iter_ids`, `local_roots`, `ancestors_inclusive`, `descendants`, and `descendants_inclusive` to `MovePathTable` in `paths.rs`.

- [ ] **Step 4: Run init tests**

Run: `cargo test -p rock-lib mir::dataflow::analyses::init -- --nocapture`

Expected: init tests pass.

## Task 7: Wire InitializationAnalysis To Function Move Paths

**Files:**
- Modify: `lib/src/mir/dataflow/analyses/init.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Test: `lib/src/mir/borrowck/mod.rs`

- [ ] **Step 1: Write failing borrowck regression**

Add to `mir::borrowck::tests` in `mod.rs`:

```rust
#[test]
fn borrowck_reports_parent_use_after_field_move() {
    let mut type_context = TypeContext::new();
    let i64_id = type_context.intern_type(&Type::I64);
    let function = MirFunction {
        id: MirFunctionId::Function(DefId::new(CrateId(0), LocalDefId(99))),
        name: "field_move".to_string(),
        basic_blocks: vec![BasicBlock {
            statements: vec![
                StatementData::assign(
                    Place { local: Local(2), projection: Vec::new() },
                    Rvalue::Use(Operand::Move(Place {
                        local: Local(1),
                        projection: vec![Projection::Field { index: 0, identity: None }],
                    })),
                    None,
                ),
                StatementData::assign(
                    Place { local: Local(0), projection: Vec::new() },
                    Rvalue::Use(Operand::Copy(Place { local: Local(1), projection: Vec::new() })),
                    None,
                ),
            ],
            terminator: Some(Terminator::Return),
        }],
        local_decls: vec![
            LocalDecl { ty: i64_id, mutability: Mutability::Mut, name: Some("ret".to_string()), span: None },
            LocalDecl { ty: i64_id, mutability: Mutability::Mut, name: Some("value".to_string()), span: None },
            LocalDecl { ty: i64_id, mutability: Mutability::Mut, name: Some("tmp".to_string()), span: None },
        ],
        closure_captures: Vec::new(),
        arg_count: 1,
        ret_type: i64_id,
    };

    let err = BorrowChecker::check_function(&function, &type_context)
        .expect_err("parent use after field move should be rejected");
    assert!(err.0.iter().any(|diag| diag.message.contains("moved")));
}
```

- [ ] **Step 2: Run and verify RED**

Run: `cargo test -p rock-lib mir::borrowck::tests::borrowck_reports_parent_use_after_field_move -- --exact --nocapture`

Expected: the test fails before path-aware initialization is wired.

- [ ] **Step 3: Construct move paths in `InitializationAnalysis::new`**

Change `InitializationAnalysis` to own a `MovePathTable`:

```rust
pub struct InitializationAnalysis {
    always_init: HashSet<Local>,
    place_paths: crate::mir::borrowck::paths::PlacePathTable,
    move_paths: crate::mir::borrowck::paths::MovePathTable,
}
```

Build it in `new`:

```rust
let place_paths = crate::mir::borrowck::paths::PlacePathTable::for_function(func);
let move_paths = crate::mir::borrowck::paths::MovePathTable::from_place_paths(&place_paths);
Self { always_init, place_paths, move_paths }
```

- [ ] **Step 4: Update transfer and check methods**

Use `self.place_paths.path_id(place)` to find `PlacePathId` and update `InitMap` by `MovePathId` for `StorageLive`, `StorageDead`, assignments, moves, drops, and closure move captures. Keep `check_operand` and `check_place` public wrappers by resolving through `InitMap` methods so existing call sites compile.

- [ ] **Step 5: Run borrowck regression**

Run: `cargo test -p rock-lib mir::borrowck::tests::borrowck_reports_parent_use_after_field_move -- --exact --nocapture`

Expected: test passes and reports a moved-value diagnostic.

## Task 8: Focused Verification And Review Prep

**Files:**
- Review: `lib/src/mir/borrowck/**`
- Review: `lib/src/mir/dataflow/analyses/{init,loans}.rs`
- Review: `lib/src/ids.rs`

- [ ] **Step 1: Run focused path/dataflow suites**

Run: `cargo test -p rock-lib mir::borrowck -- --nocapture`

Expected: borrowck unit tests pass.

- [ ] **Step 2: Run dataflow suites**

Run: `cargo test -p rock-lib mir::dataflow -- --nocapture`

Expected: dataflow unit tests pass.

- [ ] **Step 3: Run integration filters covering moves and borrows**

Run: `cargo test -p rock-lib borrow -- --nocapture`

Expected: borrow integration filters pass.

Run: `cargo test -p rock-lib move -- --nocapture`

Expected: move-related filters pass or no unrelated parser filters fail.

- [ ] **Step 4: Run formatting and whitespace checks**

Run: `cargo fmt --all --check`

Expected: no output.

Run: `git diff --check`

Expected: no output.

- [ ] **Step 5: Request code review**

Use the `requesting-code-review` skill. Ask the reviewer to focus on indexed path ownership, behavior preservation, partial-move diagnostics, and whether any active borrowck path still relies on unused `MovePathId`/`PlacePathId` placeholders.

- [ ] **Step 6: Run full verification before closing**

Run: `cargo test -p rock-lib`

Expected: all unit, integration, parser integration, and doctests pass with zero failures.

---

## Self-Review

- Spec coverage: Tasks 1-3 introduce `PlacePathId`/`MovePathId` tables; Tasks 4-5 wire loans/conflicts/provenance; Tasks 6-7 wire move/init state; Task 8 verifies and reviews.
- Placeholder scan: no incomplete sections or deferred implementation markers remain.
- Type consistency: plan consistently uses `PlacePathTable`, `MovePathTable`, `PlacePathId`, `MovePathId`, `LoanData::place_path`, and existing `LoanId`/`LoanState` names.
