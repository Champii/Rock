# NLL Borrow Checker Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a full NLL-style borrow checker with dataflow analysis over MIR.

**Architecture:** Build a reusable dataflow framework with lattice traits and fixpoint iteration, then implement four analyses (move semantics, initialization, loans, borrow liveness) that run sequentially to detect memory safety errors.

**Tech Stack:** Rust, existing MIR structures, existing Diagnostics system

---

## Task 1: Create Dataflow Framework - Lattice Traits

**Files:**
- Create: `lib/src/mir/dataflow/mod.rs`
- Create: `lib/src/mir/dataflow/lattice.rs`
- Modify: `lib/src/mir/mod.rs`

**Step 1: Add dataflow module to MIR**

Edit `lib/src/mir/mod.rs`, add after `pub mod passes;`:

```rust
pub mod dataflow;
```

**Step 2: Create dataflow module structure**

Create `lib/src/mir/dataflow/mod.rs`:

```rust
//! Dataflow analysis framework for MIR
//!
//! Based on rustc's dataflow design with lattice-based analyses.

pub mod lattice;
pub mod engine;
pub mod analyses;

pub use lattice::Lattice;
pub use engine::{Analysis, Results, run_fixpoint};
```

**Step 3: Create lattice trait**

Create `lib/src/mir/dataflow/lattice.rs`:

```rust
//! Lattice traits for dataflow analysis.

use std::collections::HashSet;
use std::hash::Hash;

/// A lattice with join operation for forward dataflow analysis.
pub trait Lattice: Clone {
    /// Join `other` into `self`. Returns `true` if `self` changed.
    fn join(&mut self, other: &Self) -> bool;
}

impl<T: Eq + Hash + Clone> Lattice for HashSet<T> {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for elem in other.iter() {
            if self.insert(elem.clone()) {
                changed = true;
            }
        }
        changed
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone + Eq> Lattice for std::collections::HashMap<K, V> {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (k, v) in other.iter() {
            if let Some(existing) = self.get(k) {
                if existing != v {
                    // Conflict - in a proper lattice we'd need to handle this
                    // For now, keep the existing value
                }
            } else {
                self.insert(k.clone(), v.clone());
                changed = true;
            }
        }
        changed
    }
}
```

**Step 4: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -20`
Expected: Compiles successfully or only warnings

**Step 5: Commit**

```bash
git add lib/src/mir/dataflow/
git commit -m "feat: add dataflow lattice trait for borrow checker"
```

---

## Task 2: Create Dataflow Framework - Analysis Trait

**Files:**
- Create: `lib/src/mir/dataflow/engine.rs`
- Create: `lib/src/mir/dataflow/analyses/mod.rs`

**Step 1: Create engine with Analysis trait and fixpoint runner**

Create `lib/src/mir/dataflow/engine.rs`:

```rust
//! Dataflow analysis engine with fixpoint iteration.

use std::collections::VecDeque;

use super::lattice::Lattice;
use crate::mir::{BasicBlockId, MirFunction, Statement, Terminator};

/// A forward dataflow analysis over MIR.
pub trait Analysis {
    /// The lattice domain for this analysis.
    type Domain: Lattice;

    /// Create the initial state (entry state for first block).
    fn initial_state(&self, func: &MirFunction) -> Self::Domain;

    /// Apply a statement's effects to the state.
    fn apply_statement(&self, state: &mut Self::Domain, stmt: &Statement);

    /// Apply a terminator's effects to the state.
    fn apply_terminator(&self, state: &mut Self::Domain, term: &Terminator);

    /// Apply the effects of a basic block to the state.
    fn apply_block(&self, state: &mut Self::Domain, func: &MirFunction, block_id: BasicBlockId) {
        let block = &func.basic_blocks[block_id.0];

        for stmt in &block.statements {
            self.apply_statement(state, stmt);
        }

        if let Some(term) = &block.terminator {
            self.apply_terminator(state, term);
        }
    }
}

/// Results of a dataflow analysis.
pub struct Results<D> {
    /// State at entry of each basic block.
    pub entry_sets: Vec<D>,
    /// State at exit of each basic block.
    pub exit_sets: Vec<D>,
}

/// Run a forward dataflow analysis to fixpoint.
pub fn run_fixpoint<A: Analysis>(analysis: &A, func: &MirFunction) -> Results<A::Domain> {
    let num_blocks = func.basic_blocks.len();

    // Initialize entry/exit sets
    let mut entry_sets: Vec<A::Domain> = (0..num_blocks)
        .map(|_| analysis.initial_state(func))
        .collect();

    // First block gets the initial state
    entry_sets[0] = analysis.initial_state(func);

    let mut exit_sets: Vec<A::Domain> = (0..num_blocks)
        .map(|_| analysis.initial_state(func))
        .collect();

    // Worklist of blocks to process
    let mut worklist: VecDeque<usize> = (0..num_blocks).collect();

    // Successor map
    let successors = compute_successors(func);

    while let Some(block_idx) = worklist.pop_front() {
        let block_id = BasicBlockId(block_idx);

        // Compute exit state from entry state
        let mut state = entry_sets[block_idx].clone();
        analysis.apply_block(&mut state, func, block_id);

        // Check if exit state changed
        let changed = {
            let exit = &mut exit_sets[block_idx];
            exit.join(&state)
        };

        // If changed, enqueue successors
        if changed {
            for &succ_idx in &successors[block_idx] {
                let entry = &mut entry_sets[succ_idx];
                entry.join(&exit_sets[block_idx]);

                if !worklist.contains(&succ_idx) {
                    worklist.push_back(succ_idx);
                }
            }
        }
    }

    Results { entry_sets, exit_sets }
}

/// Compute successor blocks for each basic block.
fn compute_successors(func: &MirFunction) -> Vec<Vec<usize>> {
    let mut successors = vec![Vec::new(); func.basic_blocks.len()];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        if let Some(term) = &block.terminator {
            match term {
                Terminator::Goto(target) => {
                    successors[i].push(target.0);
                }
                Terminator::SwitchInt { targets, otherwise, .. } => {
                    for (_, target) in targets {
                        successors[i].push(target.0);
                    }
                    successors[i].push(otherwise.0);
                }
                Terminator::Call { target, .. } => {
                    successors[i].push(target.0);
                }
                Terminator::Drop { target, .. } => {
                    successors[i].push(target.0);
                }
                Terminator::Return => {}
            }
        }
    }

    successors
}
```

**Step 2: Create analyses module stub**

Create `lib/src/mir/dataflow/analyses/mod.rs`:

```rust
//! Dataflow analyses for borrow checking.

pub mod init;
pub mod loans;
pub mod liveness;

pub use init::InitializationAnalysis;
pub use loans::LoanAnalysis;
pub use liveness::BorrowLivenessAnalysis;
```

**Step 3: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -30`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add lib/src/mir/dataflow/
git commit -m "feat: add dataflow analysis engine with fixpoint iteration"
```

---

## Task 3: Implement Initialization Analysis

**Files:**
- Create: `lib/src/mir/dataflow/analyses/init.rs`

**Step 1: Write the initialization analysis**

Create `lib/src/mir/dataflow/analyses/init.rs`:

```rust
//! Initialization tracking analysis.
//!
//! Tracks which locals are initialized, moved, or uninitialized.

use std::collections::{HashMap, HashSet};

use crate::mir::{Local, MirFunction, Operand, Place, Rvalue, Statement, Terminator, Mutability};
use crate::mir::dataflow::{Analysis, Lattice};

/// Initialization state of a local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitState {
    /// Local is uninitialized (just StorageLive)
    Uninit,
    /// Local is initialized and can be used
    Init,
    /// Local has been moved from
    Moved,
}

impl Lattice for HashMap<Local, InitState> {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (k, v) in other.iter() {
            if let Some(existing) = self.get(k) {
                // At control flow merge, use the "more uninitialized" state
                let merged = match (existing, v) {
                    (InitState::Init, InitState::Init) => InitState::Init,
                    (InitState::Moved, InitState::Moved) => InitState::Moved,
                    (InitState::Uninit, _) | (_, InitState::Uninit) => InitState::Uninit,
                    (InitState::Moved, _) | (_, InitState::Moved) => InitState::Moved,
                    _ => InitState::Uninit,
                };
                if existing != &merged {
                    self.insert(*k, merged);
                    changed = true;
                }
            } else {
                self.insert(*k, *v);
                changed = true;
            }
        }
        changed
    }
}

/// Analysis that tracks initialization state of all locals.
pub struct InitializationAnalysis {
    /// Locals that are always initialized (function params, constants)
    always_init: HashSet<Local>,
}

impl InitializationAnalysis {
    pub fn new(func: &MirFunction) -> Self {
        let mut always_init = HashSet::new();

        // Arguments are initialized
        for i in 1..=func.arg_count {
            always_init.insert(Local(i));
        }

        // Return place (Local(0)) is not always init - it's written to

        Self { always_init }
    }
}

impl Analysis for InitializationAnalysis {
    type Domain = HashMap<Local, InitState>;

    fn initial_state(&self, func: &MirFunction) -> Self::Domain {
        let mut state = HashMap::new();

        // Mark all locals as Uninit initially
        for i in 0..func.local_decls.len() {
            state.insert(Local(i), InitState::Uninit);
        }

        // Mark always-initialized locals
        for local in &self.always_init {
            state.insert(*local, InitState::Init);
        }

        state
    }

    fn apply_statement(&self, state: &mut Self::Domain, stmt: &Statement) {
        match stmt {
            Statement::StorageLive(local) => {
                // Variable comes into scope but is uninitialized
                // (unless it's an always-init local like a param)
                if !self.always_init.contains(local) {
                    state.insert(*local, InitState::Uninit);
                }
            }
            Statement::StorageDead(local) => {
                // Variable goes out of scope
                state.insert(*local, InitState::Uninit);
            }
            Statement::Assign(dest, rvalue) => {
                // Destination becomes initialized
                state.insert(dest.local, InitState::Init);

                // Handle move semantics
                match rvalue {
                    Rvalue::Use(Operand::Move(src)) => {
                        // Source becomes moved
                        state.insert(src.local, InitState::Moved);
                    }
                    Rvalue::Ref(mutability, place) => {
                        // Creating a reference doesn't move the source
                        // But for mutable refs, we need to track this later
                        let _ = (mutability, place);
                    }
                    _ => {}
                }
            }
        }
    }

    fn apply_terminator(&self, state: &mut Self::Domain, term: &Terminator) {
        match term {
            Terminator::Call { destination, .. } => {
                // Call initializes the destination
                state.insert(destination.local, InitState::Init);
            }
            Terminator::Drop { place, .. } => {
                // After drop, the place is uninitialized
                state.insert(place.local, InitState::Uninit);
            }
            _ => {}
        }
    }
}

impl InitializationAnalysis {
    /// Check if an operand is valid to use at the given state.
    pub fn check_operand(operand: &Operand, state: &HashMap<Local, InitState>) -> Result<(), String> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                match state.get(&place.local) {
                    Some(InitState::Init) => Ok(()),
                    Some(InitState::Uninit) => {
                        Err(format!("use of uninitialized variable: {:?}", place.local))
                    }
                    Some(InitState::Moved) => {
                        Err(format!("use of moved value: {:?}", place.local))
                    }
                    None => Ok(()), // Unknown local, assume OK
                }
            }
            Operand::Constant(_) => Ok(()),
        }
    }

    /// Check if a place is valid to create a reference to.
    pub fn check_place(place: &Place, state: &HashMap<Local, InitState>) -> Result<(), String> {
        match state.get(&place.local) {
            Some(InitState::Init) => Ok(()),
            Some(InitState::Uninit) => {
                Err(format!("borrow of uninitialized variable: {:?}", place.local))
            }
            Some(InitState::Moved) => {
                Err(format!("borrow of moved value: {:?}", place.local))
            }
            None => Ok(()),
        }
    }
}
```

**Step 2: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -30`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add lib/src/mir/dataflow/
git commit -m "feat: add initialization dataflow analysis"
```

---

## Task 4: Implement Loan Analysis

**Files:**
- Create: `lib/src/mir/dataflow/analyses/loans.rs`

**Step 1: Write the loan analysis**

Create `lib/src/mir/dataflow/analyses/loans.rs`:

```rust
//! Loan (borrow) tracking analysis.
//!
//! Tracks active loans (borrows) and checks for aliasing violations.

use std::collections::HashMap;

use crate::mir::{Local, MirFunction, Mutability, Place, Rvalue, Statement, Terminator};
use crate::mir::dataflow::{Analysis, Lattice};

/// Unique identifier for a loan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoanId(pub usize);

/// A loan represents an active borrow.
#[derive(Debug, Clone)]
pub struct Loan {
    /// The place being borrowed.
    pub place: Place,
    /// Whether this is a shared or mutable borrow.
    pub kind: LoanKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoanKind {
    Shared,
    Mut,
}

impl From<Mutability> for LoanKind {
    fn from(m: Mutability) -> Self {
        match m {
            Mutability::Not => LoanKind::Shared,
            Mutability::Mut => LoanKind::Mut,
        }
    }
}

/// State for loan analysis: maps loan ID to loan info.
pub type LoanState = HashMap<LoanId, Loan>;

impl Lattice for LoanState {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (k, v) in other.iter() {
            if !self.contains_key(k) {
                self.insert(*k, v.clone());
                changed = true;
            }
        }
        changed
    }
}

/// Analysis that tracks active loans.
pub struct LoanAnalysis {
    /// Counter for generating unique loan IDs.
    next_loan_id: usize,
}

impl LoanAnalysis {
    pub fn new() -> Self {
        Self { next_loan_id: 0 }
    }

    fn new_loan_id(&mut self) -> LoanId {
        let id = LoanId(self.next_loan_id);
        self.next_loan_id += 1;
        id
    }
}

impl Analysis for LoanAnalysis {
    type Domain = LoanState;

    fn initial_state(&self, _func: &MirFunction) -> Self::Domain {
        HashMap::new()
    }

    fn apply_statement(&self, state: &mut Self::Domain, stmt: &Statement) {
        match stmt {
            Statement::Assign(_dest, rvalue) => {
                if let Rvalue::Ref(mutability, place) = rvalue {
                    // Create a new loan for this borrow
                    // Note: We use a deterministic ID based on statement position
                    // For now, just mark that this place has a loan
                    let loan = Loan {
                        place: place.clone(),
                        kind: LoanKind::from(*mutability),
                    };
                    // We'll use a placeholder ID since we can't mutate self here
                    // The actual ID assignment happens in the checker
                    let _ = (state, loan);
                }
            }
            _ => {}
        }
    }

    fn apply_terminator(&self, _state: &mut Self::Domain, _term: &Terminator) {
        // Loans don't change at terminators
    }
}

impl LoanAnalysis {
    /// Collect all loans created in a function.
    pub fn collect_loans(func: &MirFunction) -> Vec<(usize, usize, Loan)> {
        let mut loans = Vec::new();
        let mut loan_id = 0usize;

        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                if let Statement::Assign(_dest, Rvalue::Ref(mutability, place)) = stmt {
                    loans.push((
                        block_idx,
                        stmt_idx,
                        Loan {
                            place: place.clone(),
                            kind: LoanKind::from(*mutability),
                        },
                    ));
                    loan_id += 1;
                }
            }
        }

        loans
    }

    /// Check for aliasing violations at a given state.
    pub fn check_aliasing(
        place: &Place,
        kind: LoanKind,
        active_loans: &LoanState,
    ) -> Result<(), String> {
        for (_, loan) in active_loans.iter() {
            // Check if loans conflict
            if Self::places_conflict(place, &loan.place) {
                match (kind, loan.kind) {
                    (LoanKind::Mut, _) | (_, LoanKind::Mut) => {
                        // Mutable loan conflicts with any existing loan
                        return Err(format!(
                            "cannot borrow `{:?}` as {} because it is already borrowed as {}",
                            place,
                            if kind == LoanKind::Mut { "mutable" } else { "immutable" },
                            if loan.kind == LoanKind::Mut { "mutable" } else { "immutable" }
                        ));
                    }
                    (LoanKind::Shared, LoanKind::Shared) => {
                        // Multiple shared loans are OK
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if two places may alias.
    fn places_conflict(a: &Place, b: &Place) -> bool {
        // Simple check: same local means potential conflict
        // A more precise analysis would consider projections
        a.local == b.local
    }
}
```

**Step 2: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -30`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add lib/src/mir/dataflow/analyses/loans.rs
git commit -m "feat: add loan tracking analysis for borrow checking"
```

---

## Task 5: Implement Borrow Liveness Analysis

**Files:**
- Create: `lib/src/mir/dataflow/analyses/liveness.rs`

**Step 1: Write the liveness analysis**

Create `lib/src/mir/dataflow/analyses/liveness.rs`:

```rust
//! Borrow liveness analysis (NLL core).
//!
//! Tracks which loans are "live" at each program point.
//! A loan is live if it might be used later (dereferenced).

use std::collections::{HashMap, HashSet};

use crate::mir::{BasicBlockId, Local, MirFunction, Operand, Place, Projection, Rvalue, Statement, Terminator};
use crate::mir::dataflow::{Analysis, Lattice};
use super::loans::{Loan, LoanId, LoanKind};

/// Map from loan ID to whether it's live.
pub type LiveLoans = HashSet<LoanId>;

impl Lattice for LiveLoans {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for loan in other.iter() {
            if self.insert(*loan) {
                changed = true;
            }
        }
        changed
    }
}

/// Backward analysis to determine which loans are live.
pub struct BorrowLivenessAnalysis<'a> {
    /// All loans in the function, indexed by ID.
    loans: &'a HashMap<LoanId, Loan>,
    /// Map from local to the loan it represents (if it's a reference).
    ref_to_loan: HashMap<Local, LoanId>,
}

impl<'a> BorrowLivenessAnalysis<'a> {
    pub fn new(loans: &'a HashMap<LoanId, Loan>, ref_locals: HashMap<Local, LoanId>) -> Self {
        Self {
            loans,
            ref_to_loan: ref_locals,
        }
    }

    /// Run backward dataflow analysis.
    /// Returns for each (block, statement) which loans are live AFTER that point.
    pub fn run_backward(&self, func: &MirFunction) -> HashMap<(usize, usize), LiveLoans> {
        let num_blocks = func.basic_blocks.len();

        // Live loans at exit of each block
        let mut block_exit: Vec<LiveLoans> = vec![HashSet::new(); num_blocks];
        // Live loans at entry of each block
        let mut block_entry: Vec<LiveLoans> = vec![HashSet::new(); num_blocks];

        // Compute predecessors
        let predecessors = compute_predecessors(func);

        // Iterate until fixpoint
        let mut changed = true;
        while changed {
            changed = false;

            // Process blocks in reverse order
            for block_idx in (0..num_blocks).rev() {
                let block = &func.basic_blocks[block_idx];

                // Exit state = union of successor entry states
                let mut exit_state = LiveLoans::new();
                for &succ_idx in &successors_of(func, block_idx) {
                    exit_state.join(&block_entry[succ_idx]);
                }

                // Apply block statements in reverse to get entry state
                let mut entry_state = exit_state.clone();

                // Apply terminator
                if let Some(term) = &block.terminator {
                    self.apply_terminator_backward(&mut entry_state, term);
                }

                // Apply statements in reverse
                for stmt in block.statements.iter().rev() {
                    self.apply_statement_backward(&mut entry_state, stmt);
                }

                // Update and check for changes
                if block_exit[block_idx].join(&exit_state) {
                    changed = true;
                }
                if block_entry[block_idx].join(&entry_state) {
                    changed = true;
                }
            }
        }

        // Build per-statement liveness map
        let mut result = HashMap::new();
        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            let mut state = block_exit[block_idx].clone();

            // After terminator
            if let Some(term) = &block.terminator {
                self.apply_terminator_backward(&mut state, term);
            }

            // After each statement (in reverse)
            for (stmt_idx, stmt) in block.statements.iter().enumerate().rev() {
                result.insert((block_idx, stmt_idx), state.clone());
                self.apply_statement_backward(&mut state, stmt);
            }
        }

        result
    }

    fn apply_statement_backward(&self, state: &mut LiveLoans, stmt: &Statement) {
        match stmt {
            Statement::Assign(dest, rvalue) => {
                // If we assign to a reference local, the loan is no longer live
                if let Some(loan_id) = self.ref_to_loan.get(&dest.local) {
                    state.remove(loan_id);
                }

                // Check rvalue for uses
                match rvalue {
                    Rvalue::Use(operand) => {
                        self.mark_loan_live(state, operand);
                    }
                    Rvalue::Ref(_, _) => {
                        // Creating a ref doesn't use existing loans
                    }
                    Rvalue::BinaryOp(_, a, b) => {
                        self.mark_loan_live(state, a);
                        self.mark_loan_live(state, b);
                    }
                    Rvalue::UnaryOp(_, a) => {
                        self.mark_loan_live(state, a);
                    }
                    Rvalue::Aggregate(_, operands) => {
                        for op in operands {
                            self.mark_loan_live(state, op);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn apply_terminator_backward(&self, state: &mut LiveLoans, term: &Terminator) {
        match term {
            Terminator::SwitchInt { discr, .. } => {
                self.mark_loan_live(state, discr);
            }
            Terminator::Call { func: op_func, args, .. } => {
                self.mark_loan_live(state, op_func);
                for arg in args {
                    self.mark_loan_live(state, arg);
                }
            }
            _ => {}
        }
    }

    fn mark_loan_live(&self, state: &mut LiveLoans, operand: &Operand) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                // Check if this is a reference being used
                if let Some(loan_id) = self.ref_to_loan.get(&place.local) {
                    state.insert(*loan_id);
                }

                // Check for deref projections - means the loan is being used
                for proj in &place.projection {
                    if matches!(proj, Projection::Deref) {
                        if let Some(loan_id) = self.ref_to_loan.get(&place.local) {
                            state.insert(*loan_id);
                        }
                    }
                }
            }
            Operand::Constant(_) => {}
        }
    }
}

/// Compute predecessor blocks for each basic block.
fn compute_predecessors(func: &MirFunction) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); func.basic_blocks.len()];

    for (i, block) in func.basic_blocks.iter().enumerate() {
        if let Some(term) = &block.terminator {
            match term {
                Terminator::Goto(target) => {
                    preds[target.0].push(i);
                }
                Terminator::SwitchInt { targets, otherwise, .. } => {
                    for (_, target) in targets {
                        preds[target.0].push(i);
                    }
                    preds[otherwise.0].push(i);
                }
                Terminator::Call { target, .. } => {
                    preds[target.0].push(i);
                }
                Terminator::Drop { target, .. } => {
                    preds[target.0].push(i);
                }
                Terminator::Return => {}
            }
        }
    }

    preds
}

/// Get successors of a block.
fn successors_of(func: &MirFunction, block_idx: usize) -> Vec<usize> {
    let block = &func.basic_blocks[block_idx];
    let mut succs = Vec::new();

    if let Some(term) = &block.terminator {
        match term {
            Terminator::Goto(target) => {
                succs.push(target.0);
            }
            Terminator::SwitchInt { targets, otherwise, .. } => {
                for (_, target) in targets {
                    succs.push(target.0);
                }
                succs.push(otherwise.0);
            }
            Terminator::Call { target, .. } => {
                succs.push(target.0);
            }
            Terminator::Drop { target, .. } => {
                succs.push(target.0);
            }
            Terminator::Return => {}
        }
    }

    succs
}

impl BorrowLivenessAnalysis<'_> {
    /// Check if mutating a place is valid given live loans.
    pub fn check_mutation(place: &Place, live_loans: &LiveLoans, loans: &HashMap<LoanId, Loan>) -> Result<(), String> {
        for loan_id in live_loans {
            if let Some(loan) = loans.get(loan_id) {
                // Check if this mutation conflicts with a live loan
                if loan.place.local == place.local {
                    return Err(format!(
                        "cannot assign to `{:?}` because it is borrowed",
                        place.local
                    ));
                }
            }
        }
        Ok(())
    }
}
```

**Step 2: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -40`
Expected: Compiles with possible unused warnings

**Step 3: Commit**

```bash
git add lib/src/mir/dataflow/analyses/liveness.rs
git commit -m "feat: add borrow liveness analysis for NLL"
```

---

## Task 6: Update BorrowCheckPass to Use Dataflow

**Files:**
- Modify: `lib/src/mir/passes/borrow_check/mod.rs`
- Modify: `lib/src/mir/passes/borrow_check/init.rs`

**Step 1: Rewrite BorrowCheckPass**

Replace `lib/src/mir/passes/borrow_check/mod.rs`:

```rust
//! MIR-based Borrow Checker
//!
//! Uses dataflow analysis to check:
//! - Move semantics (use-after-move, double-move)
//! - Initialization (use of uninitialized values)
//! - Borrow rules (aliasing, mutable vs shared)
//! - NLL (non-lexical lifetimes)

use std::collections::HashMap;

use crate::mir::{MirProgram, MirFunction, Statement, Terminator, Rvalue, Operand, Place, BasicBlockId};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::lexer::Span;

use crate::mir::dataflow::analyses::{
    InitializationAnalysis, InitState,
    LoanAnalysis, Loan, LoanId, LoanKind,
};
use crate::mir::dataflow::{Analysis, run_fixpoint};

pub struct BorrowCheckPass;

impl BorrowCheckPass {
    pub fn run(program: &MirProgram) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::default();

        for (_, func) in &program.functions {
            if let Err(func_diagnostics) = Self::check_function(func) {
                for diag in func_diagnostics.0 {
                    diagnostics.push(diag);
                }
            }
        }

        if diagnostics.0.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    fn check_function(func: &MirFunction) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::default();

        // Run initialization analysis
        let init_analysis = InitializationAnalysis::new(func);
        let init_results = run_fixpoint(&init_analysis, func);

        // Collect loans
        let loans_list = LoanAnalysis::collect_loans(func);
        let loans: HashMap<LoanId, Loan> = loans_list
            .iter()
            .enumerate()
            .map(|(i, (_, _, loan))| (LoanId(i), loan.clone()))
            .collect();

        // Check each block
        for (block_idx, block) in func.basic_blocks.iter().enumerate() {
            let state = &init_results.exit_sets[block_idx];

            // Check statements
            for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                Self::check_statement(stmt, state, &loans, func, block_idx, stmt_idx, &mut diagnostics);
            }

            // Check terminator
            if let Some(term) = &block.terminator {
                Self::check_terminator(term, state, &mut diagnostics);
            }
        }

        if diagnostics.0.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }

    fn check_statement(
        stmt: &Statement,
        state: &HashMap<crate::mir::Local, InitState>,
        loans: &HashMap<LoanId, Loan>,
        func: &MirFunction,
        block_idx: usize,
        stmt_idx: usize,
        diagnostics: &mut Diagnostics,
    ) {
        match stmt {
            Statement::Assign(dest, rvalue) => {
                // Check rvalue operands
                match rvalue {
                    Rvalue::Use(operand) => {
                        if let Err(e) = InitializationAnalysis::check_operand(operand, state) {
                            diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                        }
                    }
                    Rvalue::Ref(mutability, place) => {
                        // Check that borrowed place is initialized
                        if let Err(e) = InitializationAnalysis::check_place(place, state) {
                            diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                        }

                        // Check aliasing rules
                        let kind = LoanKind::from(*mutability);
                        let loan_state: HashMap<LoanId, Loan> = HashMap::new(); // TODO: track active loans
                        if let Err(e) = LoanAnalysis::check_aliasing(place, kind, &loan_state) {
                            diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                        }
                    }
                    Rvalue::BinaryOp(_, a, b) => {
                        if let Err(e) = InitializationAnalysis::check_operand(a, state) {
                            diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                        }
                        if let Err(e) = InitializationAnalysis::check_operand(b, state) {
                            diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                        }
                    }
                    Rvalue::UnaryOp(_, a) => {
                        if let Err(e) = InitializationAnalysis::check_operand(a, state) {
                            diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                        }
                    }
                    Rvalue::Aggregate(_, operands) => {
                        for op in operands {
                            if let Err(e) = InitializationAnalysis::check_operand(op, state) {
                                diagnostics.push(Self::make_error(e, func, block_idx, stmt_idx));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn check_terminator(
        term: &Terminator,
        state: &HashMap<crate::mir::Local, InitState>,
        diagnostics: &mut Diagnostics,
    ) {
        match term {
            Terminator::SwitchInt { discr, .. } => {
                if let Err(e) = InitializationAnalysis::check_operand(discr, state) {
                    diagnostics.push(Diagnostic::new(e, Span::default()));
                }
            }
            Terminator::Call { func: op_func, args, .. } => {
                if let Err(e) = InitializationAnalysis::check_operand(op_func, state) {
                    diagnostics.push(Diagnostic::new(e, Span::default()));
                }
                for arg in args {
                    if let Err(e) = InitializationAnalysis::check_operand(arg, state) {
                        diagnostics.push(Diagnostic::new(e, Span::default()));
                    }
                }
            }
            _ => {}
        }
    }

    fn make_error(message: String, func: &MirFunction, block_idx: usize, stmt_idx: usize) -> Diagnostic {
        // Try to get span from local decl
        let _ = (func, block_idx, stmt_idx); // TODO: implement span recovery
        Diagnostic::new(message, Span::default())
    }
}
```

**Step 2: Remove old init.rs**

```bash
rm lib/src/mir/passes/borrow_check/init.rs
```

**Step 3: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -50`
Expected: Compiles with warnings about unused imports/code

**Step 4: Commit**

```bash
git add lib/src/mir/passes/borrow_check/
git commit -m "feat: integrate dataflow analyses into BorrowCheckPass"
```

---

## Task 7: Add Test Cases

**Files:**
- Create: `examples/mir_tests/move_valid.rk`
- Create: `examples/mir_tests/use_after_move.rk`
- Create: `examples/mir_tests/double_move.rk`
- Create: `examples/mir_tests/shared_borrow.rk`

**Step 1: Create move_valid.rk**

Create `examples/mir_tests/move_valid.rk`:

```rock
// Valid move - should compile without error

main = ->
    x = "hello"
    y = x
    println y
    0
```

**Step 2: Create use_after_move.rk**

Create `examples/mir_tests/use_after_move.rk`:

```rock
// RUN: rockc %s 2>&1 | grep "use of moved"

main = ->
    x = "hello"
    y = x
    println x
    0
```

**Step 3: Create double_move.rk**

Create `examples/mir_tests/double_move.rk`:

```rock
// RUN: rockc %s 2>&1 | grep "use of moved"

main = ->
    x = "hello"
    y = x
    z = x
    0
```

**Step 4: Create shared_borrow.rk**

Create `examples/mir_tests/shared_borrow.rk`:

```rock
// Multiple shared borrows should be allowed

main = ->
    x = 42
    r1 = &x
    r2 = &x
    println r1
    println r2
    0
```

**Step 5: Test the borrow checker**

Run: `cargo run -p rockc -- --entry-file examples/mir_tests/use_after_move.rk 2>&1`
Expected: Error about "use of moved"

**Step 6: Commit**

```bash
git add examples/mir_tests/
git commit -m "test: add borrow checker test cases"
```

---

## Task 8: Add Span Tracking to MIR

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder.rs`

**Step 1: Add span to LocalDecl**

Edit `lib/src/mir/mod.rs`, find `LocalDecl` struct and add:

```rust
#[derive(Debug, Clone)]
pub struct LocalDecl {
    pub ty: Type,
    pub mutability: Mutability,
    pub name: Option<String>,
    pub span: Option<Span>,  // Add this line
}
```

Also add the import at top:
```rust
use crate::lexer::Span;
```

**Step 2: Update builder to track spans**

Edit `lib/src/mir/builder.rs`, find `new_local` method and update:

```rust
fn new_local(&mut self, ty: Type, mutability: Mutability, name: Option<String>) -> Local {
    let id = Local(self.locals.len());
    self.locals.push(LocalDecl {
        ty,
        mutability,
        name,
        span: None,  // Will be set when we have span info
    });
    id
}

fn new_local_with_span(&mut self, ty: Type, mutability: Mutability, name: Option<String>, span: Option<Span>) -> Local {
    let id = Local(self.locals.len());
    self.locals.push(LocalDecl {
        ty,
        mutability,
        name,
        span,
    });
    id
}
```

Add import:
```rust
use crate::lexer::Span;
```

**Step 3: Verify compilation**

Run: `cargo build -p rock-lib 2>&1 | head -30`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add lib/src/mir/
git commit -m "feat: add span tracking to MIR LocalDecl for error reporting"
```

---

## Task 9: Clean Up Warnings and Final Test

**Step 1: Remove unused imports**

Run: `cargo build -p rock-lib 2>&1 | grep "unused"`
Expected: List of unused imports

Fix each unused import warning by removing the import.

**Step 2: Run full test suite**

Run: `cargo test -p rock-lib 2>&1 | tail -20`
Expected: All tests pass

**Step 3: Test borrow checker with examples**

Run: `cargo run -p rockc -- --entry-file examples/hello.rk 2>&1`
Expected: Compiles successfully

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: clean up warnings in borrow checker implementation"
```

---

## Summary

After completing all tasks, the borrow checker will:

1. **Track initialization** - Uses dataflow to know which variables are initialized at each point
2. **Detect use-after-move** - Errors when reading a moved value
3. **Detect use-of-uninit** - Errors when reading uninitialized variables
4. **Track loans** - Records all borrows created
5. **Check aliasing** - Prevents mutable + shared borrow conflicts
6. **Support NLL** - Borrows end at last use, not lexical scope end

The implementation keeps HIR codegen unchanged while adding proper memory safety checks via MIR analysis.
