# NLL Borrow Checker Design

Date: 2026-02-20
Status: Approved

## Overview

Implement a full Non-Lexical Lifetimes (NLL) borrow checker for Rock, using Rust's dataflow approach. The borrow checker will operate on MIR and detect use-after-move, aliasing violations, and lifetime errors.

## Goals

- Detect use-after-move and double-move errors
- Track loans (borrows) through the CFG
- Enforce aliasing rules (no mutable + shared borrows overlapping)
- Implement NLL-style liveness (borrows end at last use, not scope end)
- Keep HIR codegen unchanged during implementation

## Non-Goals

- MIR → LLVM codegen (keep using HIR codegen)
- Full Rust compatibility (simpler rules sufficient for Rock)
- Lifetime annotations/syntax in source code

## Architecture

### Dataflow Analysis Framework

Location: `lib/src/mir/dataflow/`

```
dataflow/
├── mod.rs          // Framework traits and types
├── lattice.rs      // Lattice traits (Join, Meet, Top, Bottom)
├── engine.rs       // Fixpoint iteration engine
└── analyses/       // Individual analyses
    ├── mod.rs
    ├── move_semantics.rs
    ├── initialization.rs
    ├── loans.rs
    └── liveness.rs
```

**Core abstractions:**

```rust
/// A dataflow analysis over MIR
pub trait Analysis {
    type Domain: Lattice;

    fn bottom() -> Self::Domain;
    fn apply_statement(&self, state: &mut Self::Domain, stmt: &Statement);
    fn apply_terminator(&self, state: &mut Self::Domain, term: &Terminator);
}

/// Lattice trait for join operations
pub trait Lattice: Clone {
    fn join(&mut self, other: &Self) -> bool; // Returns true if changed
}
```

**Fixpoint engine:**

```rust
pub struct Results<D> {
    pub entry_sets: Vec<D>,  // State at entry of each block
    pub exit_sets: Vec<D>,   // State at exit of each block
}

pub fn run_fixpoint<A: Analysis>(func: &MirFunction) -> Results<A::Domain>;
```

### Analysis Implementations

#### 1. Move Semantics Analysis

- **Domain:** `HashSet<Local>` - initialized locals
- **Transfer:**
  - `StorageLive(l)` → no change (uninitialized)
  - `Assign(dest, Rvalue::Use(Operand::Move(src)))` → remove `src.local`, add `dest.local`
  - `Assign(dest, _)` → add `dest.local`
- **Errors:** Using `Move` or `Copy` operand when local not in set

#### 2. Initialization Analysis (extends Move)

- **Domain:** `HashMap<Local, InitState>` where `InitState = Uninit | Init | Moved`
- **Transfer:**
  - `StorageLive(l)` → set to `Uninit`
  - `Assign(dest, ...)` → set `dest.local` to `Init`
  - `Move(src)` → set `src.local` to `Moved`
- **Errors:** Reading from `Moved` or `Uninit` state

#### 3. Loan Analysis (Borrow Tracking)

- **Domain:** `HashMap<LoanId, Loan>`
- **Loan struct:**
  ```rust
  struct Loan {
      place: Place,
      kind: LoanKind,  // Shared or Mut
      origin_block: BasicBlockId,
  }
  ```
- **Transfer:**
  - `Rvalue::Ref(mutability, place)` → create new loan
- **Errors:** Creating `Mut` loan while any loan on same place exists

#### 4. Borrow Liveness Analysis (NLL Core)

- **Domain:** `HashSet<LoanId>` - loans that are "live" (may be used later)
- **Transfer:**
  - At dereference of reference → mark corresponding loan as potentially used
  - Compute backwards from uses to determine liveness
- **Errors:** Mutating a place while a loan on it is live

### Integration

**Pipeline:**
```
HIR → MIR Builder → Dataflow Analyses → Error Collection → HIR Codegen
```

**BorrowCheckPass:**
```rust
impl BorrowCheckPass {
    pub fn run(program: &MirProgram) -> Result<(), Diagnostics> {
        for func in program.functions.values() {
            let init_results = InitializationAnalysis::run(func);
            let loan_results = LoanAnalysis::run(func);
            let liveness_results = BorrowLivenessAnalysis::run(func, &loan_results);

            Self::check_errors(func, &init_results, &loan_results, &liveness_results)?;
        }
        Ok(())
    }
}
```

### Span Recovery

Store source spans in MIR for error reporting:

```rust
pub struct LocalDecl {
    pub ty: Type,
    pub mutability: Mutability,
    pub name: Option<String>,
    pub span: Option<Span>,  // NEW: source location
}
```

## Testing Strategy

### Test Files in `examples/mir_tests/`

**Move semantics:**
- `move_basic.rk` - valid move
- `use_after_move.rk` - error expected
- `double_move.rk` - error expected

**Borrow tests:**
- `shared_borrow.rk` - multiple shared refs OK
- `mut_while_shared.rk` - error: mutate while borrowed
- `mut_and_shared.rk` - error: overlapping borrows

**NLL tests:**
- `nll_early_end.rk` - borrow ends before lexical scope
- `nll_conditional.rk` - conditional borrows handled correctly

### Test Format

```rock
// RUN: rockc %s 2>&1 | grep "expected error message"
```

### Integration Tests

Add to `lib/tests/integration.rs`:
- `test_borrow_check_passes()` - valid programs compile
- `test_borrow_check_fails()` - invalid programs produce expected errors

## Implementation Order

1. **Dataflow framework** - lattice traits, fixpoint engine
2. **Move semantics** - basic initialization tracking
3. **Initialization analysis** - full InitState tracking
4. **Loan analysis** - borrow creation and tracking
5. **Borrow liveness** - NLL-style live loan detection
6. **Error reporting** - diagnostics with spans
7. **Testing** - comprehensive test suite

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Performance on large functions | Start simple, optimize later if needed |
| Complex control flow | Test with nested loops, early returns |
| False positives | Conservative error reporting, allow suppression |
