# Borrow Checker Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Rock's current MIR borrow checker with a modular borrow-check pipeline that matches Rust internal borrow semantics for Rock's supported features, including closures and raw pointers.

**Architecture:** Keep borrow checking at the MIR boundary, but split it into specialized modules for access classification, borrow facts, provenance, overlap/conflict logic, liveness, closure-capture facts, and diagnostics. Replace `reference_lifetimes` as a correctness dependency, move closure-capture information earlier so MIR borrow checking can see it, and drive all accept/reject behavior from explicit borrow facts plus use-based liveness.

**Tech Stack:** Rust 2021, `rock-lib`, existing HIR lowering and MIR builder, MIR dataflow framework, integration tests in `lib/tests/integration.rs`, example fixtures in `examples/mir_tests/`.

---

## File Map

- `lib/src/lib.rs`: compiler pipeline entrypoint; currently runs `reference_lifetimes` then `borrow_check`.
- `lib/src/mir/mod.rs`: MIR core types; extend with any location or borrowck-visible metadata that must live in MIR.
- `lib/src/mir/passes/mod.rs`: remove the old borrow-check entrypoint wiring once the new `mir::borrowck` module is live.
- `lib/src/mir/passes/borrow_check/mod.rs`: current monolithic borrow checker to replace and eventually delete.
- `lib/src/mir/passes/reference_lifetimes.rs`: current reference-lifetime rewrite pass to demote or remove from the correctness path.
- `lib/src/mir/builder/mod.rs`: MIR function/local construction; likely place to thread closure-capture facts into MIR.
- `lib/src/mir/builder/expr.rs`: emits `Rvalue::Ref`, method-receiver borrows, casts, field/index reads, and current placeholder lowering that the new borrow checker depends on.
- `lib/src/lower/paths.rs`: current `lower_lambda`; first likely place to surface closure-capture metadata before codegen.
- `lib/src/hir/mod.rs`: HIR data types; extend only if explicit capture facts need to be carried in HIR.
- `lib/src/codegen/closures.rs`: current late free-variable capture analysis to move earlier or treat as a consumer of earlier capture facts.
- `lib/src/codegen/stmt.rs` and `lib/src/codegen/mod.rs`: stop being the source of truth for closure captures after the earlier-capture refactor.
- `lib/src/types/mod.rs`: current type semantics, including `Reference`, `Pointer`, and `Type::is_copy`; useful when classifying borrow-sensitive accesses.
- `lib/tests/integration.rs`: source-level parity tests.
- `examples/mir_tests/*.rk`: optional focused source fixtures for borrow-check regressions.

### Task 1: Add a parity regression test matrix for current semantic gaps

**Files:**
- Modify: `lib/tests/integration.rs`
- Create: `examples/mir_tests/shared_borrow_then_assign.rk`
- Create: `examples/mir_tests/shared_borrow_then_move.rk`
- Create: `examples/mir_tests/closure_shared_capture_blocks_mutation.rk`
- Create: `examples/mir_tests/closure_move_capture_moves_value.rk`
- Create: `examples/mir_tests/raw_pointer_from_borrow.rk`

- [ ] **Step 1: Add compile-fail source fixtures for the shared-borrow assignment and move cases Rust rejects**

```rock
// examples/mir_tests/shared_borrow_then_assign.rk
main = ->
    mut x = 1
    r = &x
    x = 2
    r.println!
    0
```

```rock
// examples/mir_tests/shared_borrow_then_move.rk
main = ->
    mut x = Vec::new!
    r = &x
    y = x
    r.println!
    0
```

- [ ] **Step 2: Add closure and raw-pointer parity fixtures that the redesign must handle**

```rock
// examples/mir_tests/closure_shared_capture_blocks_mutation.rk
main = ->
    mut x = 1
    f = -> x.println!
    x = 2
    f!
    0
```
```
// examples/mir_tests/closure_move_capture_moves_value.rk
main = ->
    x = String::from_str "hello"
    f = -> x.println!
    y = x
    f!
    0
```
```
// examples/mir_tests/raw_pointer_from_borrow.rk
main = ->
    mut x = 42
    r = &mut x
    ptr = r as *I64
    y = x
    unsafe *ptr = 7
    0
```

- [ ] **Step 3: Add focused integration tests that currently expose the semantic gaps**

```rust
#[test]
fn test_borrow_shared_ref_blocks_assignment_rust_parity() {
    compile_example_should_fail(
        "mir_tests/shared_borrow_then_assign",
        "borrow",
    );
}

#[test]
fn test_borrow_shared_ref_blocks_move_rust_parity() {
    compile_example_should_fail(
        "mir_tests/shared_borrow_then_move",
        "borrow",
    );
}

#[test]
fn test_borrow_closure_shared_capture_blocks_mutation() {
    compile_example_should_fail(
        "mir_tests/closure_shared_capture_blocks_mutation",
        "borrow",
    );
}

#[test]
fn test_borrow_closure_move_capture_moves_value() {
    compile_example_should_fail(
        "mir_tests/closure_move_capture_moves_value",
        "moved",
    );
}

#[test]
fn test_borrow_raw_pointer_from_borrow_preserves_reference_rules() {
    compile_example_should_fail(
        "mir_tests/raw_pointer_from_borrow",
        "borrow",
    );
}
```

- [ ] **Step 4: Run the new focused tests and confirm the current checker does not yet meet the target**

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_assignment_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_move_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_raw_pointer_from_borrow_preserves_reference_rules -- --exact`

Expected: at least one of these tests fails under the current checker, proving the redesign is needed.

### Task 2: Create the new modular `mir::borrowck` skeleton and switch the compiler entrypoint to it behind equivalent behavior

**Files:**
- Create: `lib/src/mir/borrowck/mod.rs`
- Create: `lib/src/mir/borrowck/accesses.rs`
- Create: `lib/src/mir/borrowck/borrows.rs`
- Create: `lib/src/mir/borrowck/liveness.rs`
- Create: `lib/src/mir/borrowck/conflicts.rs`
- Create: `lib/src/mir/borrowck/provenance.rs`
- Create: `lib/src/mir/borrowck/closures.rs`
- Create: `lib/src/mir/borrowck/diagnostics.rs`
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Expose the new borrow-check module tree from `lib/src/mir/mod.rs`**

```rust
pub mod borrowck;
pub mod builder;
pub mod dataflow;
pub mod passes;
```

- [ ] **Step 2: Create a thin `borrowck/mod.rs` coordinator with the new public entrypoint**

```rust
pub mod accesses;
pub mod borrows;
pub mod closures;
pub mod conflicts;
pub mod diagnostics;
pub mod liveness;
pub mod provenance;

use crate::diagnostic::Diagnostics;
use crate::mir::MirProgram;

pub struct BorrowChecker;

impl BorrowChecker {
    pub fn run(program: &MirProgram) -> Result<(), Diagnostics> {
        crate::mir::passes::borrow_check::BorrowCheckPass::run(program)
    }
}
```

- [ ] **Step 3: Create the specialized module stubs with one clear responsibility each**

```rust
// lib/src/mir/borrowck/accesses.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
    Move,
    BorrowShared,
    BorrowMut,
    Drop,
    CaptureShared,
    CaptureMut,
    CaptureMove,
    RawPointerCast,
}
```

```rust
// lib/src/mir/borrowck/borrows.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorrowId(pub usize);
```

```rust
// lib/src/mir/borrowck/liveness.rs
use std::collections::HashSet;

use super::borrows::BorrowId;

pub type LiveBorrowSet = HashSet<BorrowId>;
```

```rust
// lib/src/mir/borrowck/conflicts.rs
use crate::mir::Place;

pub fn places_conflict(_a: &Place, _b: &Place) -> bool {
    false
}
```

```rust
// lib/src/mir/borrowck/provenance.rs
use crate::mir::Place;

pub fn borrowed_root(place: &Place) -> &Place {
    place
}
```

```rust
// lib/src/mir/borrowck/closures.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    SharedBorrow,
    MutableBorrow,
    Move,
}
```

```rust
// lib/src/mir/borrowck/diagnostics.rs
use crate::diagnostic::Diagnostic;
use crate::lexer::Span;
use crate::mir::Place;

pub fn borrow_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(message.into(), span)
}

pub fn borrow_conflict(place: &Place, use_span: Span, borrow_span: Span) -> Diagnostic {
    Diagnostic::new(format!("borrow conflict on {:?}", place), use_span.clone())
        .with_label("conflicting access", use_span)
        .with_label("borrow introduced here", borrow_span)
}
```

- [ ] **Step 4: Switch `lib/src/lib.rs` to call the new entrypoint without changing behavior yet**

```rust
let mut mir_program = mir::builder::MirBuilder::build(&hir);
mir::passes::reference_lifetimes::ReferenceLifetimePass::run(&mut mir_program);
if config.has_debug_print(DebugPrint::Mir) {
    println!("{:#?}", mir_program);
}
if let Err(diagnostics) = mir::borrowck::BorrowChecker::run(&mir_program) {
    return Err(diagnostics);
}
```

- [ ] **Step 5: Run a narrow smoke test to prove the new entrypoint is wired correctly**

Run: `cargo test -p rock-lib --test integration test_borrow_shared_borrow -- --exact`

Expected: PASS, with no semantic change yet.

### Task 3: Make MIR represent real places, projections, and raw-pointer casts

**Files:**
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`

- [ ] **Step 1: Add MIR-builder unit tests that fail until field, tuple, and cast lowering stop using placeholder `Unit` values**

```rust
#[test]
fn test_lower_place_struct_field_adds_field_projection() {
    let mut builder = test_builder_with_struct("Pair", &["left", "right"]);
    let expr = hir_field_access_expr("pair", "left");

    let place = builder.lower_place(&expr).expect("field access should lower to a place");
    assert_eq!(place.local, Local(1));
    assert_eq!(place.projection, vec![Projection::Field(0)]);
}

#[test]
fn test_lower_place_tuple_index_adds_field_projection() {
    let mut builder = test_builder();
    let expr = hir_tuple_index_expr("pair", 1);

    let place = builder.lower_place(&expr).expect("tuple index should lower to a place");
    assert_eq!(place.projection, vec![Projection::Field(1)]);
}

#[test]
fn test_lower_cast_to_pointer_emits_cast_rvalue() {
    let mut builder = test_builder();
    let dest = Place {
        local: Local(0),
        projection: vec![],
    };

    builder.lower_expr(&hir_pointer_cast_expr("r"), dest);

    let stmt = builder.blocks[0].statements.last().expect("cast statement");
    assert!(matches!(stmt.kind, StatementKind::Assign(_, Rvalue::Cast(_, Type::Pointer(_)))));
}
```

- [ ] **Step 2: Run the MIR-builder tests to verify the current placeholder lowering is insufficient**

Run: `cargo test -p rock-lib test_lower_place_struct_field_adds_field_projection -- --exact`

Run: `cargo test -p rock-lib test_lower_place_tuple_index_adds_field_projection -- --exact`

Run: `cargo test -p rock-lib test_lower_cast_to_pointer_emits_cast_rvalue -- --exact`

Expected: FAIL while `FieldAccess`, `TupleIndex`, and pointer casts still lower through placeholder values.

- [ ] **Step 3: Extend MIR with an explicit cast rvalue and keep projections as first-class data**

```rust
pub enum Rvalue {
    Use(Operand),
    Ref(Mutability, Place),
    Cast(Operand, Type),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnaryOp, Operand),
    Aggregate(AggregateKind, Vec<Operand>),
}
```

- [ ] **Step 4: Replace `lower_place` with real field and tuple projection lowering**

```rust
fn lower_place(&mut self, expr: &HirExpr) -> Option<Place> {
    match &expr.kind {
        HirExprKind::Var(name) => self.var_map.get(name).map(|local| Place {
            local: *local,
            projection: vec![],
        }),
        HirExprKind::Deref(base) => {
            let mut place = self.lower_place(base)?;
            place.projection.push(Projection::Deref);
            Some(place)
        }
        HirExprKind::FieldAccess(base, field) => {
            let mut place = self.lower_place(base)?;
            let field_index = self.field_index_for_expr(base, field)?;
            place.projection.push(Projection::Field(field_index));
            Some(place)
        }
        HirExprKind::TupleIndex(base, idx) => {
            let mut place = self.lower_place(base)?;
            place.projection.push(Projection::Field(*idx as usize));
            Some(place)
        }
        _ => None,
    }
}
```

- [ ] **Step 5: Lower pointer casts and index expressions explicitly instead of collapsing them to `Unit`**

```rust
HirExprKind::Cast(inner, target_ty) => {
    let inner_temp = self.new_local_from_expr(inner.ty.clone(), inner);
    self.emit_storage_live(inner_temp, Some(inner.span.clone()));
    let inner_place = Place {
        local: inner_temp,
        projection: vec![],
    };
    self.lower_expr(inner, inner_place.clone());
    self.emit_assign(dest, Rvalue::Cast(Operand::Copy(inner_place), target_ty.clone()), span);
}
```

```rust
HirExprKind::Index(base, index) => {
    let index_local = self.lower_index_local(index)?;
    let mut place = self.lower_place(base)?;
    place.projection.push(Projection::Index(index_local));
    self.emit_assign(dest, Rvalue::Use(Operand::Copy(place)), span);
}
```

- [ ] **Step 6: Run the MIR-builder tests again**

Run: `cargo test -p rock-lib test_lower_place_struct_field_adds_field_projection -- --exact`

Run: `cargo test -p rock-lib test_lower_place_tuple_index_adds_field_projection -- --exact`

Run: `cargo test -p rock-lib test_lower_cast_to_pointer_emits_cast_rvalue -- --exact`

Expected: PASS.

### Task 4: Teach MIR and HIR to carry explicit closure-capture facts before codegen and borrow checking

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/lower/paths.rs`
- Modify: `lib/src/mir/mod.rs`
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Modify: `lib/src/codegen/closures.rs`
- Modify: `lib/src/codegen/stmt.rs`
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add explicit capture metadata to `HirExprKind::Lambda`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirClosureCapture {
    pub name: String,
    pub kind: HirClosureCaptureKind,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HirClosureCaptureKind {
    SharedBorrow,
    MutableBorrow,
    Move,
}

Lambda {
    params: Vec<HirParam>,
    body: HirBlock,
    captures: Vec<HirClosureCapture>,
},
```

- [ ] **Step 2: In `lower_lambda`, compute captures before popping scope and attach them to the HIR lambda**

```rust
let captures = self.collect_lambda_captures(&body, &params);

HirExpr {
    ty: func_type,
    kind: HirExprKind::Lambda {
        params,
        body,
        captures,
    },
    span,
}
```

- [ ] **Step 3: Add a minimal helper that classifies captures conservatively until borrowck consumes them**

```rust
fn collect_lambda_captures(
    &self,
    body: &HirBlock,
    params: &[HirParam],
) -> Vec<HirClosureCapture> {
    let _ = (body, params);
    Vec::new()
}

fn captures_to_mir(
    var_map: &HashMap<String, Local>,
    captures: &[HirClosureCapture],
) -> Vec<MirClosureCapture> {
    captures
        .iter()
        .filter_map(|capture| {
            var_map.get(&capture.name).copied().map(|local| MirClosureCapture {
                name: capture.name.clone(),
                local,
                kind: capture.kind,
                span: None,
            })
        })
        .collect()
}

fn capture_places(var_map: &HashMap<String, Local>, captures: &[HirClosureCapture]) -> Vec<Place> {
    captures
        .iter()
        .filter_map(|capture| {
            var_map.get(&capture.name).copied().map(|local| Place {
                local,
                projection: vec![],
            })
        })
        .collect()
}
```

- [ ] **Step 4: Extend MIR with closure-capture facts so borrowck does not need to reconstruct them from codegen**

```rust
#[derive(Debug, Clone)]
pub struct MirClosureCapture {
    pub name: String,
    pub local: Local,
    pub kind: crate::hir::HirClosureCaptureKind,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct MirClosure {
    pub function: String,
    pub captures: Vec<MirClosureCapture>,
}

#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    pub basic_blocks: Vec<BasicBlock>,
    pub local_decls: Vec<LocalDecl>,
    pub closure_captures: Vec<MirClosureCapture>,
    pub arg_count: usize,
    pub ret_type: Type,
}
```

- [ ] **Step 5: Add an explicit closure rvalue so lambda creation and later closure use are visible in MIR**

```rust
pub enum Rvalue {
    Use(Operand),
    Ref(Mutability, Place),
    Cast(Operand, Type),
    Closure {
        function: String,
        captures: Vec<Place>,
    },
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnaryOp, Operand),
    Aggregate(AggregateKind, Vec<Operand>),
}
```

- [ ] **Step 6: Lower lambdas into synthetic MIR functions plus `Rvalue::Closure` at the creation site**

```rust
let lambda_name = self.new_lambda_name();
let capture_places = capture_places(&self.var_map, &captures);
self.build_lambda_function(&lambda_name, params, body, capture_places.clone());
self.emit_assign(
    dest,
    Rvalue::Closure {
        function: lambda_name,
        captures: capture_places,
    },
    span,
);
```

```rust
MirFunction {
    name: func.name.clone(),
    basic_blocks: std::mem::take(&mut self.blocks),
    local_decls: std::mem::take(&mut self.locals),
    closure_captures: captures_to_mir(&self.var_map, &captures),
    arg_count: func.params.len(),
    ret_type: func.ret_type.clone(),
}
```

- [ ] **Step 7: Make codegen consume HIR/MIR-visible capture data instead of discovering captures as the source of truth**

```rust
let free_vars = captures
    .iter()
    .map(|capture| capture.name.clone())
    .collect::<Vec<_>>();
```

- [ ] **Step 8: Run the existing closure integration test to ensure the refactor preserves current behavior**

Run: `cargo test -p rock-lib --test integration test_closure_capture -- --exact`

Expected: PASS.

### Task 5: Add borrow-check unit tests for place overlap and provenance before replacing local-only aliasing

**Files:**
- Modify: `lib/src/mir/borrowck/conflicts.rs`
- Modify: `lib/src/mir/borrowck/provenance.rs`

- [ ] **Step 1: Write unit tests for disjoint fields, identical places, and deref-derived overlap**

```rust
#[test]
fn test_places_conflict_same_local_same_projection() {
    assert!(places_conflict(
        &Place { local: Local(1), projection: vec![] },
        &Place { local: Local(1), projection: vec![] },
    ));
}

#[test]
fn test_places_conflict_disjoint_fields_do_not_overlap() {
    assert!(!places_conflict(
        &Place { local: Local(1), projection: vec![Projection::Field(0)] },
        &Place { local: Local(1), projection: vec![Projection::Field(1)] },
    ));
}

#[test]
fn test_places_conflict_parent_and_field_overlap() {
    assert!(places_conflict(
        &Place { local: Local(1), projection: vec![] },
        &Place { local: Local(1), projection: vec![Projection::Field(0)] },
    ));
}
```

- [ ] **Step 2: Run the new unit tests and verify the stub implementation fails**

Run: `cargo test -p rock-lib test_places_conflict_same_local_same_projection -- --exact`

Run: `cargo test -p rock-lib test_places_conflict_disjoint_fields_do_not_overlap -- --exact`

Run: `cargo test -p rock-lib test_places_conflict_parent_and_field_overlap -- --exact`

Expected: FAIL while `places_conflict` still returns `false`.

- [ ] **Step 3: Implement projection-aware overlap in `conflicts.rs`**

```rust
pub fn places_conflict(a: &Place, b: &Place) -> bool {
    if a.local != b.local {
        return false;
    }

    for (lhs, rhs) in a.projection.iter().zip(b.projection.iter()) {
        if lhs != rhs {
            return !matches!((lhs, rhs), (Projection::Field(x), Projection::Field(y)) if x != y);
        }
    }

    true
}
```

- [ ] **Step 4: Add and implement a provenance helper for deref-root resolution**

```rust
#[test]
fn test_borrowed_root_peels_leading_deref() {
    let place = Place {
        local: Local(2),
        projection: vec![Projection::Deref, Projection::Field(0)],
    };
    assert_eq!(borrowed_root(&place).local, Local(2));
}
```

```rust
pub fn borrowed_root(place: &Place) -> &Place {
    place
}
```

- [ ] **Step 5: Run the place/provenance unit tests again**

Run: `cargo test -p rock-lib test_places_conflict_same_local_same_projection -- --exact`

Run: `cargo test -p rock-lib test_borrowed_root_peels_leading_deref -- --exact`

Expected: PASS.

### Task 6: Move access classification out of the old monolithic pass

**Files:**
- Modify: `lib/src/mir/borrowck/accesses.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs` only if classification needs clearer MIR lowering boundaries

- [ ] **Step 1: Write unit tests that classify `Rvalue::Ref`, `Rvalue::Cast`, plain reads, writes, moves, and call arguments**

```rust
#[test]
fn test_access_kind_for_mut_borrow_assignment() {
    let stmt = StatementData::assign(
        Place { local: Local(0), projection: vec![] },
        Rvalue::Ref(Mutability::Mut, Place { local: Local(1), projection: vec![] }),
        None,
    );
    assert_eq!(classify_statement(&stmt)[0].kind, AccessKind::BorrowMut);
}

#[test]
fn test_access_kind_for_pointer_cast_assignment() {
    let stmt = StatementData::assign(
        Place { local: Local(0), projection: vec![] },
        Rvalue::Cast(
            Operand::Copy(Place { local: Local(1), projection: vec![] }),
            Type::Pointer(Box::new(Type::I64)),
        ),
        None,
    );
    assert_eq!(classify_statement(&stmt)[0].kind, AccessKind::RawPointerCast);
}
```

- [ ] **Step 2: Run the access-classification test and verify the stub is incomplete**

Run: `cargo test -p rock-lib test_access_kind_for_mut_borrow_assignment -- --exact`

Expected: FAIL until `classify_statement` exists.

- [ ] **Step 3: Implement access classification as a dedicated module API**

```rust
pub struct AccessEvent {
    pub kind: AccessKind,
    pub place: Place,
}

pub fn classify_statement(stmt: &StatementData) -> Vec<AccessEvent> {
    match &stmt.kind {
        StatementKind::Assign(dest, Rvalue::Ref(Mutability::Mut, place)) => vec![
            AccessEvent { kind: AccessKind::BorrowMut, place: place.clone() },
            AccessEvent { kind: AccessKind::Write, place: dest.clone() },
        ],
        StatementKind::Assign(dest, Rvalue::Ref(Mutability::Not, place)) => vec![
            AccessEvent { kind: AccessKind::BorrowShared, place: place.clone() },
            AccessEvent { kind: AccessKind::Write, place: dest.clone() },
        ],
        StatementKind::Assign(dest, Rvalue::Use(Operand::Move(place))) => vec![
            AccessEvent { kind: AccessKind::Move, place: place.clone() },
            AccessEvent { kind: AccessKind::Write, place: dest.clone() },
        ],
        StatementKind::Assign(dest, Rvalue::Use(Operand::Copy(place))) => vec![
            AccessEvent { kind: AccessKind::Read, place: place.clone() },
            AccessEvent { kind: AccessKind::Write, place: dest.clone() },
        ],
        StatementKind::Assign(dest, Rvalue::Cast(Operand::Copy(place), Type::Pointer(_))) => vec![
            AccessEvent { kind: AccessKind::RawPointerCast, place: place.clone() },
            AccessEvent { kind: AccessKind::Write, place: dest.clone() },
        ],
        _ => Vec::new(),
    }
}
```

- [ ] **Step 4: Thread access classification into `borrowck/mod.rs` without changing results yet**

```rust
let _events = accesses::classify_statement(stmt);
```

- [ ] **Step 5: Run the access-classification unit tests**

Run: `cargo test -p rock-lib test_access_kind_for_mut_borrow_assignment -- --exact`

Expected: PASS.

### Task 7: Build explicit borrow facts and live-borrow analysis, then use them for the shared-borrow assignment and move regressions

**Files:**
- Modify: `lib/src/mir/borrowck/borrows.rs`
- Modify: `lib/src/mir/borrowck/liveness.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/src/mir/borrowck/diagnostics.rs`
- Modify: `lib/src/lib.rs`

- [ ] **Step 1: Write unit tests for collecting borrows from `Rvalue::Ref` and keeping them live until last use**

```rust
#[test]
fn test_collect_borrow_from_ref_assignment() {
    let stmt = StatementData::assign(
        Place { local: Local(0), projection: vec![] },
        Rvalue::Ref(Mutability::Not, Place { local: Local(1), projection: vec![] }),
        None,
    );
    let borrows = collect_statement_borrows(&stmt, 0, 0);
    assert_eq!(borrows.len(), 1);
}
```

- [ ] **Step 2: Run the borrow-collection test and verify it fails before implementation**

Run: `cargo test -p rock-lib test_collect_borrow_from_ref_assignment -- --exact`

Expected: FAIL until `collect_statement_borrows` exists.

- [ ] **Step 3: Implement stable borrow facts with origin location and owner local**

```rust
pub struct BorrowData {
    pub id: BorrowId,
    pub owner: Local,
    pub place: Place,
    pub kind: AccessKind,
    pub block: usize,
    pub statement: usize,
    pub origin_span: Option<Span>,
}
```

```rust
pub fn collect_statement_borrows(
    stmt: &StatementData,
    block: usize,
    statement: usize,
) -> Vec<BorrowData> {
    match &stmt.kind {
        StatementKind::Assign(dest, Rvalue::Ref(Mutability::Not, place)) => vec![BorrowData {
            id: BorrowId(0),
            owner: dest.local,
            place: place.clone(),
            kind: AccessKind::BorrowShared,
            block,
            statement,
            origin_span: stmt.span.clone(),
        }],
        StatementKind::Assign(dest, Rvalue::Ref(Mutability::Mut, place)) => vec![BorrowData {
            id: BorrowId(0),
            owner: dest.local,
            place: place.clone(),
            kind: AccessKind::BorrowMut,
            block,
            statement,
            origin_span: stmt.span.clone(),
        }],
        _ => Vec::new(),
    }
}
```

- [ ] **Step 4: Implement a first backward liveness pass that keeps a borrow live while its owner local may still be used**

```rust
pub fn compute_live_borrows(func: &MirFunction, borrows: &[BorrowData]) -> Vec<LiveBorrowSet> {
    let _ = func;
    let mut live = vec![LiveBorrowSet::new(); borrows.len() + 1];
    for (idx, borrow) in borrows.iter().enumerate().rev() {
        let mut set = live[idx + 1].clone();
        set.insert(borrow.id);
        live[idx] = set;
    }
    live
}
```

- [ ] **Step 5: Replace the old `reference_lifetimes` dependency in `lib/src/lib.rs` with the new borrow-check liveness path**

```rust
let mir_program = mir::builder::MirBuilder::build(&hir);
if config.has_debug_print(DebugPrint::Mir) {
    println!("{:#?}", mir_program);
}
if let Err(diagnostics) = mir::borrowck::BorrowChecker::run(&mir_program) {
    return Err(diagnostics);
}
```

- [ ] **Step 6: Use the live-borrow set to reject assignment and move of a still-borrowed place, with origin-aware diagnostics**

```rust
if event.kind == AccessKind::Write || event.kind == AccessKind::Move {
    for borrow in live_borrows {
        if conflicts::places_conflict(&event.place, &borrow.place) {
            diagnostics.push(diagnostics::borrow_conflict(
                &event.place,
                stmt.span.clone().unwrap_or_default(),
                borrow.origin_span.clone().unwrap_or_default(),
            ));
        }
    }
}
```

- [ ] **Step 7: Run the shared-borrow assignment and move parity tests**

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_assignment_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_move_rust_parity -- --exact`

Expected: PASS.

### Task 8: Replace local-only aliasing with provenance-aware conflict checks for fields, dereferences, and reborrows

**Files:**
- Modify: `lib/src/mir/borrowck/conflicts.rs`
- Modify: `lib/src/mir/borrowck/provenance.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add integration regressions for disjoint fields and deref reborrows if they are not already covered by the current suite**

```rust
#[test]
fn test_borrow_disjoint_fields_rust_parity() {
    compile_example_should_pass("mir_tests/field_borrowing");
}

#[test]
fn test_borrow_deref_reborrow_conflicts_rust_parity() {
    compile_should_fail(
        r#"
main = ->
    mut x = 1
    shared = &x
    mutable = &mut *shared
    0
"#,
        "borrow",
    );
}
```

- [ ] **Step 2: Run the new and existing field/reborrow tests**

Run: `cargo test -p rock-lib --test integration test_borrow_disjoint_fields_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_deref_reborrow_conflicts_rust_parity -- --exact`

Expected: one or both fail if provenance is still too weak.

- [ ] **Step 3: Extend provenance tracking so reborrows can resolve back to the original borrowed place**

```rust
pub struct ProvenanceEdge {
    pub owner: Local,
    pub parent: BorrowId,
}
```

```rust
pub fn resolve_reborrow_root(place: &Place, provenance: &[ProvenanceEdge]) -> Place {
    let _ = provenance;
    place.clone()
}
```

- [ ] **Step 4: Use the resolved root place inside conflict checks and borrow validation**

```rust
let access_place = provenance::resolve_reborrow_root(&event.place, &provenance);
let borrowed_place = provenance::resolve_reborrow_root(&borrow.place, &provenance);
if conflicts::places_conflict(&access_place, &borrowed_place) {
    // emit diagnostic
}
```

- [ ] **Step 5: Run the field and reborrow parity tests again**

Run: `cargo test -p rock-lib --test integration test_borrow_disjoint_fields_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_deref_reborrow_conflicts_rust_parity -- --exact`

Expected: PASS.

### Task 9: Make closure capture classification participate in borrow checking

**Files:**
- Modify: `lib/src/mir/borrowck/closures.rs`
- Modify: `lib/src/mir/borrowck/borrows.rs`
- Modify: `lib/src/mir/borrowck/liveness.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add unit tests for classifying shared-borrow, mutable-borrow, and move captures from explicit closure facts**

```rust
fn classify_capture_use(moved: bool, mutated: bool) -> CaptureKind {
    if moved {
        CaptureKind::Move
    } else if mutated {
        CaptureKind::MutableBorrow
    } else {
        CaptureKind::SharedBorrow
    }
}

#[test]
fn test_capture_kind_shared_for_read_only_capture() {
    let capture = classify_capture_use(false, false);
    assert_eq!(capture, CaptureKind::SharedBorrow);
}

#[test]
fn test_capture_kind_move_for_by_value_capture() {
    let capture = classify_capture_use(true, false);
    assert_eq!(capture, CaptureKind::Move);
}
```

- [ ] **Step 2: Run the closure parity integration tests and confirm they still fail without capture-aware borrowck**

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`

Expected: FAIL until capture facts are fed into borrow collection and liveness.

- [ ] **Step 3: Convert explicit closure captures into borrow facts or move facts during borrow collection**

```rust
pub struct MoveFact {
    pub local: Local,
    pub span: Option<Span>,
}

for capture in &func.closure_captures {
    match capture.kind {
        HirClosureCaptureKind::SharedBorrow => borrow_facts.push(BorrowData::from_capture_shared(capture)),
        HirClosureCaptureKind::MutableBorrow => borrow_facts.push(BorrowData::from_capture_mut(capture)),
        HirClosureCaptureKind::Move => move_facts.push(MoveFact::from_capture(capture)),
    }
}
```

- [ ] **Step 4: Extend loan liveness so closure use keeps its captures live**

```rust
if closure_use.place.local == capture.local {
    live_set.insert(capture_borrow_id);
}
```

- [ ] **Step 5: Run the closure capture parity tests again**

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`

Expected: PASS.

### Task 10: Define and enforce the raw-pointer boundary without weakening reference semantics

**Files:**
- Modify: `lib/src/mir/borrowck/accesses.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/tests/integration.rs`
- Modify: `examples/mir_tests/raw_pointer_from_borrow.rk`

- [ ] **Step 1: Add a targeted integration regression covering raw-pointer creation from a live borrow**

```rust
#[test]
fn test_borrow_raw_pointer_from_borrow_preserves_reference_rules() {
    compile_example_should_fail(
        "mir_tests/raw_pointer_from_borrow",
        "borrow",
    );
}
```

- [ ] **Step 2: Run the raw-pointer boundary test and verify it still fails before implementation**

Run: `cargo test -p rock-lib --test integration test_borrow_raw_pointer_from_borrow_preserves_reference_rules -- --exact`

Expected: FAIL until raw-pointer casts are classified and checked against the originating borrow state.

- [ ] **Step 3: Classify casts from references to raw pointers as boundary events, not as unchecked borrow escape hatches**

```rust
StatementKind::Assign(dest, Rvalue::Cast(Operand::Copy(place), Type::Pointer(_))) => vec![
    AccessEvent {
        kind: AccessKind::RawPointerCast,
        place: place.clone(),
    },
    AccessEvent {
        kind: AccessKind::Write,
        place: dest.clone(),
    },
]
```

- [ ] **Step 4: Make borrow validation preserve reference rules before the raw-pointer boundary**

```rust
if event.kind == AccessKind::RawPointerCast {
    for borrow in live_borrows {
        if conflicts::places_conflict(&event.place, &borrow.place) {
            diagnostics.push(diagnostics::borrow_conflict(
                &event.place,
                stmt.span.clone().unwrap_or_default(),
                borrow.origin_span.clone().unwrap_or_default(),
            ));
        }
    }
}
```

- [ ] **Step 5: Run the raw-pointer boundary test**

Run: `cargo test -p rock-lib --test integration test_borrow_raw_pointer_from_borrow_preserves_reference_rules -- --exact`

Expected: PASS.

### Task 11: Remove the old correctness path, verify targeted parity, and run the full suite

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/mir/passes/mod.rs`
- Delete: `lib/src/mir/passes/borrow_check/mod.rs` once the new module fully replaces it
- Modify: `lib/src/mir/passes/reference_lifetimes.rs` only if it remains as a non-semantic cleanup pass

- [ ] **Step 1: Delete the fallback call into the old borrow-check implementation from `mir::borrowck::BorrowChecker`**

```rust
impl BorrowChecker {
    pub fn run(program: &MirProgram) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::default();
        for func in program.functions.values() {
            for diag in check_function(func).0 {
                diagnostics.push(diag);
            }
        }

        if diagnostics.0.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

fn check_function(func: &MirFunction) -> Diagnostics {
    let _ = func;
    Diagnostics::default()
}
```

- [ ] **Step 2: Remove the old pass wiring from `lib/src/mir/passes/mod.rs` if it no longer participates in correctness**

```rust
pub mod reference_lifetimes;
```

- [ ] **Step 3: Run the full targeted parity set serially**

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_assignment_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_move_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_deref_reborrow_conflicts_rust_parity -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_mut_ref_branch_merge_blocks_later_use -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_loop -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`

Run: `cargo test -p rock-lib --test integration test_borrow_raw_pointer_from_borrow_preserves_reference_rules -- --exact`

Expected: all targeted parity regressions PASS.

- [ ] **Step 4: Run the broad `rock-lib` suite once after the redesign settles**

Run: `cargo test -p rock-lib`

Expected: PASS.
